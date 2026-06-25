mod elf;
mod insn;
mod tracepoint;

use std::path::Path;

use crate::ast::unit::ProgramKind;
use crate::ir::ProgramIr;

pub fn emit_program(program: &ProgramIr, output: &Path) -> Result<(), String> {
    for unit in &program.units {
        let section = unit
            .sections
            .first()
            .ok_or_else(|| format!("unit '{}' has no section", unit.name))?;

        if unit.program_type != ProgramKind::Tracepoint || !section.starts_with("tracepoint/") {
            return Err(format!(
                "native Rust backend currently supports only tracepoint sections, got '{}'",
                section
            ));
        }
    }

    let mut object = elf::BpfObject::new();
    object.add_license(program)?;
    object.add_maps(&program.maps)?;

    for unit in &program.units {
        let section = unit.sections.first().expect("validated above");
        let compiled = tracepoint::compile_tracepoint(unit)?;
        object.add_program(section, &unit.name, compiled)?;
    }

    object.write(output)
}
