// src/emit/ebpf_obj/elf.rs;
use std::{collections::HashMap, fs::File, io::Write, path::Path};

use crate::emit::btf::{self, ext::build_btf_ext};

use super::{maps::MapsSection, tracepoint::ProgramSection};

const ET_REL: u16 = 1;
const EM_BPF: u16 = 247;
const EV_CURRENT: u32 = 1;

const SHT_NULL: u32 = 0;
const SHT_PROGBITS: u32 = 1;
const SHT_SYMTAB: u32 = 2;
const SHT_STRTAB: u32 = 3;
const SHT_REL: u32 = 9;

const SHF_WRITE: u64 = 0x1;
const SHF_ALLOC: u64 = 0x2;
const SHF_EXECINSTR: u64 = 0x4;

const STB_LOCAL: u8 = 0;
const STB_GLOBAL: u8 = 1;

const STT_NOTYPE: u8 = 0;
const STT_OBJECT: u8 = 1;
const STT_FUNC: u8 = 2;

const R_BPF_64_64: u32 = 1;

#[derive(Clone, Debug)]
struct Section {
    name: String,
    name_off: u32,
    ty: u32,
    flags: u64,
    align: u64,
    entsize: u64,
    link: u32,
    info: u32,
    data: Vec<u8>,
    file_off: u64,
}

#[derive(Clone, Debug)]
struct SymbolSpec {
    name: String,
    info: u8,
    shndx: u16,
    value: u64,
    size: u64,
}

struct Strtab {
    bytes: Vec<u8>,
    seen: HashMap<String, u32>,
}

impl Strtab {
    fn new() -> Self {
        Self {
            bytes: vec![0],
            seen: HashMap::new(),
        }
    }

    fn put(&mut self, s: &str) -> u32 {
        if let Some(x) = self.seen.get(s) {
            return *x;
        }
        let off = self.bytes.len() as u32;
        self.bytes.extend_from_slice(s.as_bytes());
        self.bytes.push(0);
        self.seen.insert(s.to_string(), off);
        off
    }
}

pub fn write_object(
    output: &Path,
    maps: &MapsSection,
    programs: &[ProgramSection],
    license: &str,
) -> Result<(), String> {
    let mut sections = Vec::<Section>::new();

    let mut btf_builder = btf::BtfBuilder::new();
        
    for prog in programs {
        btf_builder.add_string(&prog.symbol_name);
    }

    let btf_data = btf_builder.emit();

    let btf_ext_data = build_btf_ext();
    
    sections.push(Section {
        name: String::new(),
        name_off: 0,
        ty: SHT_NULL,
        flags: 0,
        align: 0,
        entsize: 0,
        link: 0,
        info: 0,
        data: Vec::new(),
        file_off: 0,
    });

    sections.push(Section {
        name: ".BTF".to_string(),
        name_off: 0,
        ty: SHT_PROGBITS,
        flags: 0,
        align: 4,
        entsize: 0,
        link: 0,
        info: 0,
        data: btf_data,
        file_off: 0,
    });

    sections.push(Section {
        name: ".BTF.ext".to_string(),
        name_off: 0,
        ty: SHT_PROGBITS,
        flags: 0,
        align: 4,
        entsize: 0,
        link: 0,
        info: 0,
        data: btf_ext_data,
        file_off: 0,
    });

    let mut prog_sec_indices = Vec::<u16>::new();
    for p in programs {
        prog_sec_indices.push(sections.len() as u16);
        sections.push(Section {
            name: p.section_name.clone(),
            name_off: 0,
            ty: SHT_PROGBITS,
            flags: SHF_ALLOC | SHF_EXECINSTR,
            align: 8,
            entsize: 0,
            link: 0,
            info: 0,
            data: p.code.clone(),
            file_off: 0,
        });
    }

    let mut rel_sec_meta = Vec::<(usize, u16)>::new();
    for (i, p) in programs.iter().enumerate() {
        if !p.relocs.is_empty() {
            let idx = sections.len() as u16;
            rel_sec_meta.push((i, idx));
            sections.push(Section {
                name: format!(".rel{}", p.section_name),
                name_off: 0,
                ty: SHT_REL,
                flags: 0,
                align: 8,
                entsize: 16,
                link: 0,
                info: prog_sec_indices[i] as u32,
                data: Vec::new(),
                file_off: 0,
            });
        }
    }

    let maps_idx = sections.len() as u16;
    sections.push(Section {
        name: ".maps".to_string(),
        name_off: 0,
        ty: SHT_PROGBITS,
        flags: SHF_ALLOC | SHF_WRITE,
        align: 8,
        entsize: 0,
        link: 0,
        info: 0,
        data: maps.bytes.clone(),
        file_off: 0,
    });
    
    let mut license_bytes = license.as_bytes().to_vec();
    if !license_bytes.ends_with(&[0]) {
        license_bytes.push(0);
    }
    sections.push(Section {
        name: "license".to_string(),
        name_off: 0,
        ty: SHT_PROGBITS,
        flags: SHF_ALLOC,
        align: 1,
        entsize: 0,
        link: 0,
        info: 0,
        data: license_bytes,
        file_off: 0,
    });

    let strtab_idx = sections.len() as u16;
    sections.push(Section {
        name: ".strtab".to_string(),
        name_off: 0,
        ty: SHT_STRTAB,
        flags: 0,
        align: 1,
        entsize: 0,
        link: 0,
        info: 0,
        data: Vec::new(),
        file_off: 0,
    });

    let symtab_idx = sections.len() as u16;
    sections.push(Section {
        name: ".symtab".to_string(),
        name_off: 0,
        ty: SHT_SYMTAB,
        flags: 0,
        align: 8,
        entsize: 24,
        link: strtab_idx as u32,
        info: 1,
        data: Vec::new(),
        file_off: 0,
    });

    let shstrtab_idx = sections.len() as u16;
    sections.push(Section {
        name: ".shstrtab".to_string(),
        name_off: 0,
        ty: SHT_STRTAB,
        flags: 0,
        align: 1,
        entsize: 0,
        link: 0,
        info: 0,
        data: Vec::new(),
        file_off: 0,
    });

    // build symbols
    let mut strtab = Strtab::new();
    let mut symbols = Vec::<SymbolSpec>::new();
    let mut map_sym_idx = HashMap::<String, u32>::new();

    symbols.push(SymbolSpec {
        name: String::new(),
        info: bind_type(STB_LOCAL, STT_NOTYPE),
        shndx: 0,
        value: 0,
        size: 0,
    });

    for (i, p) in programs.iter().enumerate() {
        symbols.push(SymbolSpec {
            name: p.symbol_name.clone(),
            info: bind_type(STB_GLOBAL, STT_FUNC),
            shndx: prog_sec_indices[i],
            value: 0,
            size: p.code.len() as u64,
        });
    }

    for rec in &maps.records {
        let idx = symbols.len() as u32;
        map_sym_idx.insert(rec.name.clone(), idx);
        symbols.push(SymbolSpec {
            name: rec.name.clone(),
            info: bind_type(STB_GLOBAL, STT_OBJECT),
            shndx: maps_idx,
            value: rec.offset,
            size: rec.size,
        });
    }

    let mut symtab_bytes = Vec::new();
    for sym in &symbols {
        let name_off = strtab.put(&sym.name);
        push_sym(
            &mut symtab_bytes,
            name_off,
            sym.info,
            0,
            sym.shndx,
            sym.value,
            sym.size,
        );
    }

    sections[strtab_idx as usize].data = strtab.bytes;
    sections[symtab_idx as usize].data = symtab_bytes;

    // build reloc sections
    for (prog_i, rel_sec_idx) in &rel_sec_meta {
        let p = &programs[*prog_i];
        let mut rel_bytes = Vec::new();

        for rel in &p.relocs {
            let sym_idx = *map_sym_idx
                .get(&rel.map_name)
                .ok_or_else(|| format!("missing relocation symbol for map '{}'", rel.map_name))?;
            push_rel(
                &mut rel_bytes,
                rel.insn_byte_off,
                elf64_r_info(sym_idx, R_BPF_64_64),
            );
        }

        sections[*rel_sec_idx as usize].link = symtab_idx as u32;
        sections[*rel_sec_idx as usize].info = prog_sec_indices[*prog_i] as u32;
        sections[*rel_sec_idx as usize].data = rel_bytes;
    }

    // build shstrtab
    let mut shstr = Strtab::new();
    for sec in sections.iter_mut().skip(1) {
        sec.name_off = shstr.put(&sec.name);
    }
    sections[shstrtab_idx as usize].data = shstr.bytes;

    // file layout
    let ehdr_size = 64u64;
    let shdr_size = 64u64;

    let mut cur = ehdr_size;
    for sec in sections.iter_mut().skip(1) {
        cur = align_up(cur, sec.align.max(1));
        sec.file_off = cur;
        cur += sec.data.len() as u64;
    }

    let shoff = align_up(cur, 8);
    let file_size = shoff + (sections.len() as u64 * shdr_size);

    let mut out = Vec::with_capacity(file_size as usize);

    // ELF header
    push_ehdr(&mut out, shoff, sections.len() as u16, shstrtab_idx);

    // section data
    while out.len() < ehdr_size as usize {
        out.push(0);
    }

    for sec in sections.iter().skip(1) {
        while out.len() < sec.file_off as usize {
            out.push(0);
        }
        out.extend_from_slice(&sec.data);
    }

    while out.len() < shoff as usize {
        out.push(0);
    }

    // section headers
    for sec in &sections {
        push_shdr(
            &mut out,
            sec.name_off,
            sec.ty,
            sec.flags,
            0,
            sec.file_off,
            sec.data.len() as u64,
            sec.link,
            sec.info,
            sec.align,
            sec.entsize,
        );
    }

    let mut f = File::create(output).map_err(|e| e.to_string())?;
    f.write_all(&out).map_err(|e| e.to_string())?;
    Ok(())
}

fn bind_type(bind: u8, ty: u8) -> u8 {
    (bind << 4) | (ty & 0x0f)
}

fn elf64_r_info(sym: u32, ty: u32) -> u64 {
    ((sym as u64) << 32) | (ty as u64)
}

fn align_up(v: u64, a: u64) -> u64 {
    if a <= 1 {
        v
    } else {
        (v + (a - 1)) & !(a - 1)
    }
}

fn push_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn push_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn push_u64(out: &mut Vec<u8>, v: u64) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn push_ehdr(out: &mut Vec<u8>, shoff: u64, shnum: u16, shstrndx: u16) {
    let mut ident = [0u8; 16];
    ident[0] = 0x7f;
    ident[1] = b'E';
    ident[2] = b'L';
    ident[3] = b'F';
    ident[4] = 2; // 64-bit
    ident[5] = 1; // little endian
    ident[6] = 1; // version

    out.extend_from_slice(&ident);
    push_u16(out, ET_REL);
    push_u16(out, EM_BPF);
    push_u32(out, EV_CURRENT);
    push_u64(out, 0);
    push_u64(out, 0);
    push_u64(out, shoff);
    push_u32(out, 0);
    push_u16(out, 64);
    push_u16(out, 0);
    push_u16(out, 0);
    push_u16(out, 64);
    push_u16(out, shnum);
    push_u16(out, shstrndx);
}

fn push_shdr(
    out: &mut Vec<u8>,
    name: u32,
    ty: u32,
    flags: u64,
    addr: u64,
    off: u64,
    size: u64,
    link: u32,
    info: u32,
    addralign: u64,
    entsize: u64,
) {
    push_u32(out, name);
    push_u32(out, ty);
    push_u64(out, flags);
    push_u64(out, addr);
    push_u64(out, off);
    push_u64(out, size);
    push_u32(out, link);
    push_u32(out, info);
    push_u64(out, addralign);
    push_u64(out, entsize);
}

fn push_sym(
    out: &mut Vec<u8>,
    st_name: u32,
    st_info: u8,
    st_other: u8,
    st_shndx: u16,
    st_value: u64,
    st_size: u64,
) {
    push_u32(out, st_name);
    out.push(st_info);
    out.push(st_other);
    push_u16(out, st_shndx);
    push_u64(out, st_value);
    push_u64(out, st_size);
}

fn push_rel(out: &mut Vec<u8>, r_offset: u64, r_info: u64) {
    push_u64(out, r_offset);
    push_u64(out, r_info);
}
