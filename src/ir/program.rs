use super::{LoweringError, UnitIr};
use crate::ast::{MapDecl, Program};

#[derive(Debug, Clone)]
pub struct ProgramIr {
    pub maps: Vec<MapDecl>,
    pub units: Vec<UnitIr>,
}

pub fn lower_program(program: &Program) -> Result<ProgramIr, LoweringError> {
    let mut units = Vec::new();

    // Build event map at program level
    let mut event_sizes = std::collections::HashMap::new();
    let mut event_decls = std::collections::HashMap::new();

    for event in &program.events {
        let size = crate::sema::event::compute_event_size(event);
        event_sizes.insert(event.name.clone(), size);
        event_decls.insert(event.name.clone(), event.clone());
    }

    for unit in &program.units {
        units.push(UnitIr::lower(unit, &event_sizes, &event_decls)?);
    }

    Ok(ProgramIr {
        maps: program.maps.clone(),
        units,
    })
}
