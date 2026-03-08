// src/emit/ebpf_obj/mod.rs
use std::path::Path;

use crate::ir::ProgramIr;

pub mod elf;
pub mod insn;
pub mod maps;
pub mod tracepoint;

pub fn emit_program(program: &ProgramIr, output: &Path) -> Result<(), String> {
    let maps = maps::build_maps(&program.maps)?;

    let mut license: Option<String> = None;
    let mut programs = Vec::new();

    for unit in &program.units {
        let sec = unit
            .sections
            .get(0)
            .map(|s| s.as_str())
            .ok_or_else(|| format!("unit '{}' missing section", unit.name))?;

        if !sec.starts_with("tracepoint/") {
            return Err(format!(
                "direct object backend currently supports only tracepoint/*, got '{}'",
                sec
            ));
        }

        match &license {
            None => license = Some(unit.license.clone()),
            Some(x) if x == &unit.license => {}
            Some(x) => {
                return Err(format!(
                    "all units in one object must share same license: '{}' vs '{}'",
                    x, unit.license
                ))
            }
        }

        programs.push(tracepoint::emit_tracepoint(unit, sec, &maps)?);
    }

    if programs.is_empty() {
        return Err("no tracepoint units found".to_string());
    }

    elf::write_object(
        output,
        &maps,
        &programs,
        license.as_deref().unwrap_or("GPL"),
    )
}