use crate::ast::{MapDecl, MapType};
use crate::parser::SourceLoc;
use std::collections::HashSet;

#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum MapValidationError {
    #[error("Duplicate map name: {0}")]
    DuplicateMapName(String, SourceLoc),

    #[error("Map 'max_entries' must be greater than zero")]
    InvalidMaxEntries(SourceLoc),

    #[error("Invalid map type")]
    InvalidType(SourceLoc),

    #[error("Map missing required field: key")]
    MissingKey(SourceLoc),

    #[error("Map missing required field: value")]
    MissingValue(SourceLoc),

    #[error("Map should not define key/value for this map type")]
    UnexpectedKeyValue(SourceLoc),

    #[error("Unknown map method '{0}'")]
    UnknownMapMethod(String, SourceLoc),

    #[error("Map method '{0}' expects {1} args, got {2}")]
    InvalidMapMethodArity(String, usize, usize, SourceLoc),

    #[error("Unknown map '{0}'")]
    UnknownMapName(String, SourceLoc),
}

pub fn check_map(
    map_decl: &MapDecl,
    map_names: &mut HashSet<String>,
) -> Result<(), MapValidationError> {
    if !map_names.insert(map_decl.name.clone()) {
        return Err(MapValidationError::DuplicateMapName(
            map_decl.name.clone(),
            map_decl.loc,
        ));
    }

    match map_decl.map_type {
        MapType::Ringbuf => {
            if map_decl.max_entries.unwrap_or(0) == 0 {
                return Err(MapValidationError::InvalidMaxEntries(map_decl.loc));
            }

            if map_decl.key_type.is_some() || map_decl.value_type.is_some() {
                return Err(MapValidationError::UnexpectedKeyValue(map_decl.loc));
            }
        }

        MapType::Hash | MapType::LruHash => {
            if map_decl.key_type.is_none() {
                return Err(MapValidationError::MissingKey(map_decl.loc));
            }
            if map_decl.value_type.is_none() {
                return Err(MapValidationError::MissingValue(map_decl.loc));
            }
            if map_decl.max_entries.unwrap_or(0) == 0 {
                return Err(MapValidationError::InvalidMaxEntries(map_decl.loc));
            }
        }

        MapType::Array | MapType::ProgArray => {
            if map_decl.value_type.is_none() {
                return Err(MapValidationError::MissingValue(map_decl.loc));
            }
            if map_decl.max_entries.unwrap_or(0) == 0 {
                return Err(MapValidationError::InvalidMaxEntries(map_decl.loc));
            }
        }
    }

    Ok(())
}
