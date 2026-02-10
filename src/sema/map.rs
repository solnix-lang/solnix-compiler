
use crate::ast::{MapDecl, MapType, Type};
use crate::diagnostics::{DiagnosticReporter, Severity};
use crate::parser::SourceLoc;
use std::collections::HashSet;
    

#[derive(Debug, thiserror::Error)]
pub enum MapValidationError {
    #[error("Duplicate map name: {0}")]
    DuplicateMapName(String),
    
    #[error("Map 'max_entries' must be greater than zero")]
    InvalidMaxEntries,
    
    #[error("Invalid map type")]
    InvalidType,
}



pub fn check_map(
    map_decl: &MapDecl,
    diagnostics: &mut DiagnosticReporter,
    map_names: &mut HashSet<String>,
) -> Result<(), MapValidationError> {
    if !map_names.insert(map_decl.name.clone()) {
        diagnostics.report_error(
            format!("Duplicate map name: '{}'", map_decl.name),
            map_decl.loc,
        );
        return Err(MapValidationError::DuplicateMapName(map_decl.name.clone()));
    }

    if map_decl.max_entries == 0 {
        diagnostics.report_error(
            "Map 'max_entries' must be greater than zero",
            map_decl.loc,
        );
        return Err(MapValidationError::InvalidMaxEntries);
    }

    match map_decl.key_type {
        Type::U32 | Type::U64 | Type::I32 | Type::I64 => {} // Valid
    }

    match map_decl.value_type {
        Type::U32 | Type::U64 | Type::I32 | Type::I64 => {} // Valid
    }
    
    match map_decl.map_type {
        MapType::Hash | MapType::Array | MapType::Ringbuf | 
        MapType::LruHash | MapType::ProgArray | MapType::PerfEventArray => {} // Valid
    }

    Ok(())
}