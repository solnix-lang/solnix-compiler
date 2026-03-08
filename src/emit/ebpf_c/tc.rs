use std::fmt::Write;
use crate::ir::UnitIr;

pub fn emit_tc(out: &mut String, unit: &UnitIr, sec: &str) -> Result<(), String> {
    emit_license(out, &unit.license)?;

    writeln!(out, "SEC(\"{}\")", sec).map_err(err)?;
    writeln!(out, "int {}(struct __sk_buff *ctx) {{", unit.name).map_err(err)?;
    writeln!(out, "    void *data = (void *)(long)ctx->data;").map_err(err)?;
    writeln!(out, "    void *data_end = (void *)(long)ctx->data_end;").map_err(err)?;
    writeln!(out).map_err(err)?;
    writeln!(out, "    if (data + 14 > data_end) return TC_ACT_OK;").map_err(err)?;
    writeln!(out, "    return TC_ACT_OK;").map_err(err)?;
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
