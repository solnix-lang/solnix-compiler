// src/emit/ebpf_obj/maps.rs
use std::collections::HashMap;

use crate::ast::{MapDecl, MapType, Type};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BpfMapDef {
    pub map_type: u32,
    pub key_size: u32,
    pub value_size: u32,
    pub max_entries: u32,
    pub map_flags: u32,
}

#[derive(Clone, Debug)]
pub struct MapRecord {
    pub name: String,
    pub offset: u64,
    pub size: u64,
}

#[derive(Clone, Debug)]
pub struct MapsSection {
    pub bytes: Vec<u8>,
    pub records: Vec<MapRecord>,
    pub by_name: HashMap<String, usize>,
}

pub fn build_maps(maps: &[MapDecl]) -> Result<MapsSection, String> {
    let mut bytes = Vec::new();
    let mut records = Vec::new();
    let mut by_name = HashMap::new();

    for m in maps {
        let name = sanitize_ident(&m.name);
        if by_name.contains_key(&name) {
            return Err(format!("duplicate map name '{}'", name));
        }

        let def = match m.map_type {
            MapType::Ringbuf => BpfMapDef {
                map_type: map_type_to_u32(m.map_type),
                key_size: 0,
                value_size: 0,
                max_entries: m.max_entries.unwrap_or(0),
                map_flags: 0,
            },
            _ => BpfMapDef {
                map_type: map_type_to_u32(m.map_type),
                key_size: type_size(
                    m.key_type
                        .ok_or_else(|| format!("map '{}' missing key type", m.name))?,
                ),
                value_size: type_size(
                    m.value_type
                        .ok_or_else(|| format!("map '{}' missing value type", m.name))?,
                ),
                max_entries: m.max_entries.unwrap_or(0),
                map_flags: 0,
            },
        };

        let offset = bytes.len() as u64;
        push_map_def(&mut bytes, def);

        let rec = MapRecord {
            name: name.clone(),
            offset,
            size: std::mem::size_of::<BpfMapDef>() as u64,
        };

        by_name.insert(name, records.len());
        records.push(rec);
    }

    Ok(MapsSection {
        bytes,
        records,
        by_name,
    })
}

fn push_map_def(out: &mut Vec<u8>, def: BpfMapDef) {
    out.extend_from_slice(&def.map_type.to_le_bytes());
    out.extend_from_slice(&def.key_size.to_le_bytes());
    out.extend_from_slice(&def.value_size.to_le_bytes());
    out.extend_from_slice(&def.max_entries.to_le_bytes());
    out.extend_from_slice(&def.map_flags.to_le_bytes());
}

fn type_size(t: Type) -> u32 {
    match t {
        Type::U32 | Type::I32 => 4,
        Type::U64 | Type::I64 => 8,
    }
}

fn map_type_to_u32(t: MapType) -> u32 {
    match t {
        MapType::Hash => 1,
        MapType::Array => 2,
        MapType::ProgArray => 3,
        MapType::PerfEventArray => 4,
        MapType::LruHash => 9,
        MapType::Ringbuf => 27,
    }
}

fn sanitize_ident(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for (i, ch) in name.chars().enumerate() {
        let ok = if i == 0 {
            ch.is_ascii_alphabetic() || ch == '_'
        } else {
            ch.is_ascii_alphanumeric() || ch == '_'
        };
        out.push(if ok { ch } else { '_' });
    }
    if out.is_empty() {
        "_map".to_string()
    } else {
        out
    }
}