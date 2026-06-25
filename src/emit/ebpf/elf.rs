use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::ast::{MapDecl, MapType, Type};
use crate::ir::ProgramIr;

use super::tracepoint::CompiledProgram;

const ET_REL: u16 = 1;
const EM_BPF: u16 = 247;
const SHT_PROGBITS: u32 = 1;
const SHT_SYMTAB: u32 = 2;
const SHT_STRTAB: u32 = 3;
const SHT_REL: u32 = 9;
const SHF_WRITE: u64 = 1;
const SHF_ALLOC: u64 = 2;
const SHF_EXECINSTR: u64 = 4;
const STB_LOCAL: u8 = 0;
const STB_GLOBAL: u8 = 1;
const STT_OBJECT: u8 = 1;
const STT_FUNC: u8 = 2;
const R_BPF_64_64: u32 = 1;
const BTF_KIND_INT: u32 = 1;
const BTF_KIND_PTR: u32 = 2;
const BTF_KIND_ARRAY: u32 = 3;
const BTF_KIND_STRUCT: u32 = 4;
const BTF_KIND_VAR: u32 = 14;
const BTF_KIND_DATASEC: u32 = 15;
const BTF_VAR_GLOBAL_ALLOCATED: u32 = 1;
const BTF_INT_SIGNED: u32 = 1;

#[derive(Debug, Clone)]
pub struct ProgramReloc {
    pub offset: u64,
    pub symbol: String,
}

pub struct BpfObject {
    sections: Vec<Section>,
    symbols: Vec<Symbol>,
    map_symbols: HashMap<String, usize>,
    btf_maps: Vec<BtfMap>,
}

#[derive(Clone)]
struct Section {
    name: String,
    sh_type: u32,
    flags: u64,
    align: u64,
    entsize: u64,
    link: u32,
    info: u32,
    data: Vec<u8>,
}

struct Symbol {
    name: String,
    bind: u8,
    typ: u8,
    section: u16,
    value: u64,
    size: u64,
}

impl BpfObject {
    pub fn new() -> Self {
        Self {
            sections: vec![Section::null()],
            symbols: vec![Symbol::null()],
            map_symbols: HashMap::new(),
            btf_maps: Vec::new(),
        }
    }

    pub fn add_license(&mut self, program: &ProgramIr) -> Result<(), String> {
        let license = program
            .units
            .first()
            .map(|u| u.license.as_str())
            .unwrap_or("GPL");
        let mut data = license.as_bytes().to_vec();
        data.push(0);
        let section = self.add_section("license", SHT_PROGBITS, SHF_ALLOC | SHF_WRITE, 1, data);
        self.symbols.push(Symbol {
            name: "LICENSE".to_string(),
            bind: STB_GLOBAL,
            typ: STT_OBJECT,
            section,
            value: 0,
            size: license.len() as u64 + 1,
        });
        Ok(())
    }

    pub fn add_maps(&mut self, maps: &[MapDecl]) -> Result<(), String> {
        if maps.is_empty() {
            return Ok(());
        }

        let maps_section = self.add_empty_section(".maps", SHT_PROGBITS, SHF_ALLOC | SHF_WRITE, 8);
        for map in maps {
            let offset = align_to(self.sections[maps_section as usize].data.len() as u64, 8);
            while self.sections[maps_section as usize].data.len() < offset as usize {
                self.sections[maps_section as usize].data.push(0);
            }
            let map_size = btf_map_size(map);
            let bytes = vec![0; map_size as usize];
            self.sections[maps_section as usize]
                .data
                .extend_from_slice(&bytes);

            let sym_idx = self.symbols.len();
            self.symbols.push(Symbol {
                name: sanitize_ident(&map.name),
                bind: STB_GLOBAL,
                typ: STT_OBJECT,
                section: maps_section,
                value: offset,
                size: bytes.len() as u64,
            });
            self.map_symbols.insert(map.name.clone(), sym_idx);
            self.btf_maps.push(BtfMap {
                name: sanitize_ident(&map.name),
                map_type: map.map_type,
                key_type: map.key_type,
                value_type: map.value_type,
                max_entries: map.max_entries,
                offset: offset as u32,
                size: map_size,
            });
        }

        let btf = build_btf(&self.btf_maps, self.sections[maps_section as usize].data.len() as u32)?;
        self.add_section(".BTF", SHT_PROGBITS, 0, 4, btf);

        Ok(())
    }

    pub fn add_program(
        &mut self,
        section_name: &str,
        symbol_name: &str,
        program: CompiledProgram,
    ) -> Result<(), String> {
        let text_section = self.add_section(
            section_name,
            SHT_PROGBITS,
            SHF_ALLOC | SHF_EXECINSTR,
            8,
            program.code,
        );
        self.symbols.push(Symbol {
            name: sanitize_ident(symbol_name),
            bind: STB_GLOBAL,
            typ: STT_FUNC,
            section: text_section,
            value: 0,
            size: self.sections[text_section as usize].data.len() as u64,
        });

        if !program.relocs.is_empty() {
            let mut rel_data = Vec::with_capacity(program.relocs.len() * 16);
            for reloc in program.relocs {
                let sym = *self.map_symbols.get(&reloc.symbol).ok_or_else(|| {
                    format!("unknown map referenced by codegen: {}", reloc.symbol)
                })?;
                rel_data.extend_from_slice(&reloc.offset.to_le_bytes());
                let info = ((sym as u64) << 32) | R_BPF_64_64 as u64;
                rel_data.extend_from_slice(&info.to_le_bytes());
            }

            let rel_section_name = format!(".rel{}", section_name);
            let rel_section = self.add_section(&rel_section_name, SHT_REL, 0, 8, rel_data);
            self.sections[rel_section as usize].entsize = 16;
            self.sections[rel_section as usize].info = text_section as u32;
        }

        Ok(())
    }

    pub fn write(mut self, path: &Path) -> Result<(), String> {
        let symtab_index = self.sections.len() as u16;
        let (symtab, strtab) = self.build_symtab();
        let strtab_index = symtab_index + 1;

        for section in &mut self.sections {
            if section.sh_type == SHT_REL {
                section.link = symtab_index as u32;
            }
        }

        self.sections.push(Section {
            name: ".symtab".to_string(),
            sh_type: SHT_SYMTAB,
            flags: 0,
            align: 8,
            entsize: 24,
            link: strtab_index as u32,
            info: 1,
            data: symtab,
        });
        self.sections.push(Section {
            name: ".strtab".to_string(),
            sh_type: SHT_STRTAB,
            flags: 0,
            align: 1,
            entsize: 0,
            link: 0,
            info: 0,
            data: strtab,
        });

        self.sections.push(Section {
            name: ".shstrtab".to_string(),
            sh_type: SHT_STRTAB,
            flags: 0,
            align: 1,
            entsize: 0,
            link: 0,
            info: 0,
            data: Vec::new(),
        });
        let shstrtab_index = (self.sections.len() - 1) as u16;
        let shstrtab = build_shstrtab(&self.sections);
        self.sections[shstrtab_index as usize].data = shstrtab.data;

        let bytes = self.encode(shstrtab_index, &shstrtab.name_offsets)?;
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("failed to create output directory: {e}"))?;
            }
        }
        fs::write(path, bytes).map_err(|e| format!("failed to write object file: {e}"))
    }

    fn add_empty_section(&mut self, name: &str, sh_type: u32, flags: u64, align: u64) -> u16 {
        self.add_section(name, sh_type, flags, align, Vec::new())
    }

    fn add_section(
        &mut self,
        name: &str,
        sh_type: u32,
        flags: u64,
        align: u64,
        data: Vec<u8>,
    ) -> u16 {
        let index = self.sections.len() as u16;
        self.sections.push(Section {
            name: name.to_string(),
            sh_type,
            flags,
            align,
            entsize: 0,
            link: 0,
            info: 0,
            data,
        });
        index
    }

    fn build_symtab(&self) -> (Vec<u8>, Vec<u8>) {
        let mut strtab = vec![0_u8];
        let mut data = Vec::with_capacity(self.symbols.len() * 24);
        for sym in &self.symbols {
            let name_off = add_string(&mut strtab, &sym.name);
            data.extend_from_slice(&name_off.to_le_bytes());
            data.push((sym.bind << 4) | (sym.typ & 0x0f));
            data.push(0);
            data.extend_from_slice(&sym.section.to_le_bytes());
            data.extend_from_slice(&sym.value.to_le_bytes());
            data.extend_from_slice(&sym.size.to_le_bytes());
        }
        (data, strtab)
    }

    fn encode(&self, shstrtab_index: u16, name_offsets: &[u32]) -> Result<Vec<u8>, String> {
        let mut out = vec![0_u8; 64];
        let mut section_offsets = Vec::with_capacity(self.sections.len());

        for section in &self.sections {
            if section.name.is_empty() {
                section_offsets.push(0);
                continue;
            }
            let offset = align_to(out.len() as u64, section.align.max(1));
            while out.len() < offset as usize {
                out.push(0);
            }
            section_offsets.push(offset);
            out.extend_from_slice(&section.data);
        }

        let shoff = align_to(out.len() as u64, 8);
        while out.len() < shoff as usize {
            out.push(0);
        }

        for (idx, section) in self.sections.iter().enumerate() {
            let name_offset = *name_offsets
                .get(idx)
                .ok_or_else(|| format!("missing section name offset for index {}", idx))?;
            out.extend_from_slice(&name_offset.to_le_bytes());
            out.extend_from_slice(&section.sh_type.to_le_bytes());
            out.extend_from_slice(&section.flags.to_le_bytes());
            out.extend_from_slice(&0_u64.to_le_bytes());
            out.extend_from_slice(&section_offsets[idx].to_le_bytes());
            out.extend_from_slice(&(section.data.len() as u64).to_le_bytes());
            out.extend_from_slice(&section.link.to_le_bytes());
            out.extend_from_slice(&section.info.to_le_bytes());
            out.extend_from_slice(&section.align.max(1).to_le_bytes());
            out.extend_from_slice(&section.entsize.to_le_bytes());
        }

        out[0..4].copy_from_slice(&[0x7f, b'E', b'L', b'F']);
        out[4] = 2;
        out[5] = 1;
        out[6] = 1;
        out[16..18].copy_from_slice(&ET_REL.to_le_bytes());
        out[18..20].copy_from_slice(&EM_BPF.to_le_bytes());
        out[20..24].copy_from_slice(&1_u32.to_le_bytes());
        out[32..40].copy_from_slice(&0_u64.to_le_bytes());
        out[40..48].copy_from_slice(&shoff.to_le_bytes());
        out[52..54].copy_from_slice(&64_u16.to_le_bytes());
        out[54..56].copy_from_slice(&0_u16.to_le_bytes());
        out[56..58].copy_from_slice(&0_u16.to_le_bytes());
        out[58..60].copy_from_slice(&64_u16.to_le_bytes());
        out[60..62].copy_from_slice(&(self.sections.len() as u16).to_le_bytes());
        out[62..64].copy_from_slice(&shstrtab_index.to_le_bytes());

        Ok(out)
    }
}

impl Section {
    fn null() -> Self {
        Self {
            name: String::new(),
            sh_type: 0,
            flags: 0,
            align: 0,
            entsize: 0,
            link: 0,
            info: 0,
            data: Vec::new(),
        }
    }
}

impl Symbol {
    fn null() -> Self {
        Self {
            name: String::new(),
            bind: STB_LOCAL,
            typ: 0,
            section: 0,
            value: 0,
            size: 0,
        }
    }
}

#[derive(Clone)]
struct BtfMap {
    name: String,
    map_type: MapType,
    key_type: Option<Type>,
    value_type: Option<Type>,
    max_entries: Option<u32>,
    offset: u32,
    size: u32,
}

#[derive(Clone, Copy)]
struct BtfIntIds {
    int_id: u32,
    u32_id: u32,
    u64_id: u32,
    i32_id: u32,
    i64_id: u32,
}

struct BtfBuilder {
    types: Vec<u8>,
    strings: Vec<u8>,
    next_type_id: u32,
}

impl BtfBuilder {
    fn new() -> Self {
        Self {
            types: Vec::new(),
            strings: vec![0],
            next_type_id: 1,
        }
    }

    fn string(&mut self, value: &str) -> u32 {
        add_string(&mut self.strings, value)
    }

    fn type_id(&mut self) -> u32 {
        let id = self.next_type_id;
        self.next_type_id += 1;
        id
    }

    fn add_int(&mut self, name: &str, size: u32, signed: bool) -> u32 {
        let id = self.type_id();
        let name_off = self.string(name);
        self.types.extend_from_slice(&name_off.to_le_bytes());
        self.types
            .extend_from_slice(&btf_info(BTF_KIND_INT, 0).to_le_bytes());
        self.types.extend_from_slice(&size.to_le_bytes());
        let encoding = if signed { BTF_INT_SIGNED } else { 0 };
        let int_data = (encoding << 24) | (size * 8);
        self.types.extend_from_slice(&int_data.to_le_bytes());
        id
    }

    fn add_ptr(&mut self, pointee_type: u32) -> u32 {
        let id = self.type_id();
        self.types.extend_from_slice(&0_u32.to_le_bytes());
        self.types
            .extend_from_slice(&btf_info(BTF_KIND_PTR, 0).to_le_bytes());
        self.types.extend_from_slice(&pointee_type.to_le_bytes());
        id
    }

    fn add_array(&mut self, elem_type: u32, index_type: u32, nelems: u32) -> u32 {
        let id = self.type_id();
        self.types.extend_from_slice(&0_u32.to_le_bytes());
        self.types
            .extend_from_slice(&btf_info(BTF_KIND_ARRAY, 0).to_le_bytes());
        self.types.extend_from_slice(&0_u32.to_le_bytes());
        self.types.extend_from_slice(&elem_type.to_le_bytes());
        self.types.extend_from_slice(&index_type.to_le_bytes());
        self.types.extend_from_slice(&nelems.to_le_bytes());
        id
    }

    fn add_struct(&mut self, members: &[(String, u32)]) -> u32 {
        let id = self.type_id();
        self.types.extend_from_slice(&0_u32.to_le_bytes());
        self.types
            .extend_from_slice(&btf_info(BTF_KIND_STRUCT, members.len() as u32).to_le_bytes());
        self.types
            .extend_from_slice(&((members.len() as u32) * 8).to_le_bytes());

        for (idx, (name, typ)) in members.iter().enumerate() {
            let name_off = self.string(name);
            self.types.extend_from_slice(&name_off.to_le_bytes());
            self.types.extend_from_slice(&typ.to_le_bytes());
            self.types
                .extend_from_slice(&((idx as u32) * 64).to_le_bytes());
        }
        id
    }

    fn add_var(&mut self, name: &str, typ: u32) -> u32 {
        let id = self.type_id();
        let name_off = self.string(name);
        self.types.extend_from_slice(&name_off.to_le_bytes());
        self.types
            .extend_from_slice(&btf_info(BTF_KIND_VAR, 0).to_le_bytes());
        self.types.extend_from_slice(&typ.to_le_bytes());
        self.types
            .extend_from_slice(&BTF_VAR_GLOBAL_ALLOCATED.to_le_bytes());
        id
    }

    fn add_datasec(&mut self, name: &str, size: u32, vars: &[(u32, u32, u32)]) -> u32 {
        let id = self.type_id();
        let name_off = self.string(name);
        self.types.extend_from_slice(&name_off.to_le_bytes());
        self.types
            .extend_from_slice(&btf_info(BTF_KIND_DATASEC, vars.len() as u32).to_le_bytes());
        self.types.extend_from_slice(&size.to_le_bytes());
        for (typ, offset, size) in vars {
            self.types.extend_from_slice(&typ.to_le_bytes());
            self.types.extend_from_slice(&offset.to_le_bytes());
            self.types.extend_from_slice(&size.to_le_bytes());
        }
        id
    }

    fn finish(self) -> Vec<u8> {
        let hdr_len = 24_u32;
        let type_len = self.types.len() as u32;
        let str_len = self.strings.len() as u32;
        let mut out = Vec::with_capacity(hdr_len as usize + self.types.len() + self.strings.len());
        out.extend_from_slice(&0xeb9f_u16.to_le_bytes());
        out.push(1);
        out.push(0);
        out.extend_from_slice(&hdr_len.to_le_bytes());
        out.extend_from_slice(&0_u32.to_le_bytes());
        out.extend_from_slice(&type_len.to_le_bytes());
        out.extend_from_slice(&type_len.to_le_bytes());
        out.extend_from_slice(&str_len.to_le_bytes());
        out.extend_from_slice(&self.types);
        out.extend_from_slice(&self.strings);
        out
    }
}

fn build_btf(maps: &[BtfMap], maps_sec_size: u32) -> Result<Vec<u8>, String> {
    let mut btf = BtfBuilder::new();
    let ints = BtfIntIds {
        int_id: btf.add_int("int", 4, true),
        u32_id: btf.add_int("u32", 4, false),
        u64_id: btf.add_int("u64", 8, false),
        i32_id: btf.add_int("i32", 4, true),
        i64_id: btf.add_int("i64", 8, true),
    };

    let mut datasec_vars = Vec::with_capacity(maps.len());
    for map in maps {
        let struct_id = add_btf_map_struct(&mut btf, ints, map)?;
        let var_id = btf.add_var(&map.name, struct_id);
        datasec_vars.push((var_id, map.offset, map.size));
    }
    btf.add_datasec(".maps", maps_sec_size, &datasec_vars);
    Ok(btf.finish())
}

fn add_btf_map_struct(
    btf: &mut BtfBuilder,
    ints: BtfIntIds,
    map: &BtfMap,
) -> Result<u32, String> {
    let mut members = Vec::new();
    let type_array = btf.add_array(ints.int_id, ints.int_id, map_type_num(map.map_type));
    let type_ptr = btf.add_ptr(type_array);
    members.push(("type".to_string(), type_ptr));

    if let Some(key_type) = map.key_type {
        let typ = btf_type_for_source_type(ints, key_type);
        members.push(("key".to_string(), btf.add_ptr(typ)));
    }
    if let Some(value_type) = map.value_type {
        let typ = btf_type_for_source_type(ints, value_type);
        members.push(("value".to_string(), btf.add_ptr(typ)));
    }
    if let Some(max_entries) = map.max_entries {
        let max_entries_array = btf.add_array(ints.int_id, ints.int_id, max_entries);
        let max_entries_ptr = btf.add_ptr(max_entries_array);
        members.push(("max_entries".to_string(), max_entries_ptr));
    }

    if members.is_empty() {
        return Err(format!("map '{}' has no BTF attributes", map.name));
    }

    Ok(btf.add_struct(&members))
}

fn btf_type_for_source_type(ints: BtfIntIds, ty: Type) -> u32 {
    match ty {
        Type::U32 => ints.u32_id,
        Type::U64 => ints.u64_id,
        Type::I32 => ints.i32_id,
        Type::I64 => ints.i64_id,
    }
}

fn btf_map_size(map: &MapDecl) -> u32 {
    let mut fields = 1;
    fields += u32::from(map.key_type.is_some());
    fields += u32::from(map.value_type.is_some());
    fields += u32::from(map.max_entries.is_some());
    fields * 8
}

fn btf_info(kind: u32, vlen: u32) -> u32 {
    (kind << 24) | vlen
}

fn map_type_num(map_type: MapType) -> u32 {
    match map_type {
        MapType::Hash => 1,
        MapType::Array => 2,
        MapType::ProgArray => 3,
        MapType::LruHash => 9,
        MapType::Ringbuf => 27,
    }
}

fn sanitize_ident(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for (idx, ch) in name.chars().enumerate() {
        let ok = if idx == 0 {
            ch.is_ascii_alphabetic() || ch == '_'
        } else {
            ch.is_ascii_alphanumeric() || ch == '_'
        };
        out.push(if ok { ch } else { '_' });
    }
    if out.is_empty() {
        "_solnix_symbol".to_string()
    } else {
        out
    }
}

fn add_string(buf: &mut Vec<u8>, s: &str) -> u32 {
    let off = buf.len() as u32;
    buf.extend_from_slice(s.as_bytes());
    buf.push(0);
    off
}

struct ShStrTab {
    data: Vec<u8>,
    name_offsets: Vec<u32>,
}

fn build_shstrtab(sections: &[Section]) -> ShStrTab {
    let mut data = vec![0_u8];
    let mut name_offsets = Vec::with_capacity(sections.len());
    for section in sections {
        if section.name.is_empty() {
            name_offsets.push(0);
        } else {
            let off = data.len() as u32;
            name_offsets.push(off);
            data.extend_from_slice(section.name.as_bytes());
            data.push(0);
        }
    }
    ShStrTab { data, name_offsets }
}

fn align_to(value: u64, align: u64) -> u64 {
    if align <= 1 {
        value
    } else {
        (value + align - 1) & !(align - 1)
    }
}
