
use crate::ast::{MapDecl, MapType, Type};
use crate::parser::SourceLoc;
use std::collections::HashSet;
    


#[derive(Debug, thiserror::Error)]
pub enum MapValidationError {
    #[error("Duplicate map name: {0}")]
    DuplicateMapName(String, SourceLoc),

    #[error("Map 'max_entries' must be greater than zero")]
    InvalidMaxEntries(SourceLoc),

    #[error("Invalid map type")]
    InvalidType(SourceLoc),

    // NEW:
    #[error("Unknown map method '{0}'")]
    UnknownMapMethod(String, SourceLoc),

    #[error("Map method '{0}' expects {1} args, got {2}")]
    InvalidMapMethodArity(String, usize, usize, SourceLoc),

    #[error("Unknown map '{0}'")]
    UnknownMapName(String, SourceLoc),
}


pub fn check_map(map_decl: &MapDecl, map_names: &mut HashSet<String>) -> Result<(), MapValidationError> {
    if !map_names.insert(map_decl.name.clone()) {
        return Err(MapValidationError::DuplicateMapName(map_decl.name.clone(), map_decl.loc));
    }

    if map_decl.max_entries == 0 {
        return Err(MapValidationError::InvalidMaxEntries(map_decl.loc));
    }

    match map_decl.key_type {
        Type::U32 | Type::U64 | Type::I32 | Type::I64 => {}
    }

    match map_decl.value_type {
        Type::U32 | Type::U64 | Type::I32 | Type::I64 => {}
    }
    
    match map_decl.map_type {
        MapType::Hash | MapType::Array | MapType::Ringbuf | 
        MapType::LruHash | MapType::ProgArray | MapType::PerfEventArray => {}
    }

    Ok(())
}
