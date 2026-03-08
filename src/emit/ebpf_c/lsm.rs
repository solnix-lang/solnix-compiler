use std::fmt::Write;
use crate::ir::UnitIr;

pub fn emit_lsm(out: &mut String, unit: &UnitIr, sec: &str) -> Result<(), String> {
    emit_license(out, &unit.license)?;

    // LSM section
    writeln!(out, "SEC(\"{}\")", sec).map_err(err)?;
    writeln!(out, "int {}(void *ctx) {{", unit.name).map_err(err)?;
    writeln!(out, "    // ctx is hook-specific (file, task, socket, etc)").map_err(err)?;
    writeln!(out, "    // return 0 to allow, -EPERM to deny").map_err(err)?;
    writeln!(out, "    return 0;").map_err(err)?;
    writeln!(out, "}}").map_err(err)?;
    writeln!(out).map_err(err)?;

    Ok(())
}

fn emit_license(out: &mut String, lic: &str) -> Result<(), String> {
    writeln!(out, "char LICENSE[] SEC(\"license\") = \"{}\";", lic).map_err(err)?;
    writeln!(out).map_err(err)?;
    Ok(())
}

fn err(e: std::fmt::Error) -> String {
    e.to_string()
}
