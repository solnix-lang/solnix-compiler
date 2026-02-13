
pub mod map;
pub mod unit;
pub mod section;

use crate::ast::Program;
use std::collections::HashSet;
    
#[derive(Debug, thiserror::Error)]
pub enum SemanticError {
    #[error("Map validation failed")]
    MapError(#[from] map::MapValidationError),
    
    #[error("Unit validation failed")]
    UnitError(#[from] unit::UnitValidationError),
}

pub fn check_program(program: &Program) -> Result<(), SemanticError> {
    let mut map_names = HashSet::new();

    for map_decl in &program.maps {
        map::check_map(map_decl, &mut map_names)?;
    }

    for unit_decl in &program.units {
        unit::check_unit(unit_decl)?;
    }

    Ok(())
}