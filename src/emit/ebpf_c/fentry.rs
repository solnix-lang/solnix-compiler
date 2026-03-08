use std::fmt::Write;

use crate::ir::UnitIr;

pub fn emit_fentry(out: &mut String, unit: &UnitIr, sec: &str) -> Result<(), String> {
    emit_license(out, &unit.license)?;

    // sec: "fentry/<func>" or "fexit/<func>"
    writeln!(out, "SEC(\"{}\")", sec).map_err(err)?;
    writeln!(out, "int {}(void *ctx) {{", unit.name).map_err(err)?;
    writeln!(out, "    (void)ctx;").map_err(err)?;
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
