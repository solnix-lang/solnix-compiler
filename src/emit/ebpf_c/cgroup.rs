use std::fmt::Write;
use crate::ir::UnitIr;

pub fn emit_cgroup(out: &mut String, unit: &UnitIr, section: &str) -> Result<(), String> {
    emit_license(out, &unit.license)?;

    writeln!(out, "SEC(\"{}\")", section).map_err(err)?;
    writeln!(out, "int {}(struct __sk_buff *skb) {{", unit.name).map_err(err)?;
    writeln!(out, "    void *data = (void *)(long)skb->data;").map_err(err)?;
    writeln!(out, "    void *data_end = (void *)(long)skb->data_end;").map_err(err)?;
    writeln!(out, "    if (data >= data_end) return SK_DROP;").map_err(err)?;
    writeln!(out, "    return SK_PASS;").map_err(err)?;
    writeln!(out, "}}").map_err(err)?;
    writeln!(out).map_err(err)?;

    Ok(())
}

pub fn emit_cgroup_sock_addr(out: &mut String, unit: &UnitIr) -> Result<(), String> {
    emit_license(out, &unit.license)?;

    writeln!(out, "SEC(\"cgroup/sock_addr\")").map_err(err)?;
    writeln!(out, "int {}(struct bpf_sock_addr *ctx) {{", unit.name).map_err(err)?;
    writeln!(out, "    return 1; // allow connection").map_err(err)?;
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
