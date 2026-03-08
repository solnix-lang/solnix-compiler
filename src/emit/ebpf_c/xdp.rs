use std::fmt::Write;
use crate::ir::UnitIr;

pub fn emit_xdp(out: &mut String, unit: &UnitIr) -> Result<(), String> {
    emit_license(out, &unit.license)?;

    writeln!(out, "SEC(\"xdp\")").map_err(err)?;
    writeln!(out, "int {}(struct xdp_md *ctx) {{", unit.name).map_err(err)?;

    writeln!(out, "    void *data = (void *)(long)ctx->data;").map_err(err)?;
    writeln!(out, "    void *data_end = (void *)(long)ctx->data_end;").map_err(err)?;
    writeln!(out).map_err(err)?;

    writeln!(out, "    if (data + 26 + 4 > data_end) return XDP_PASS;").map_err(err)?;
    writeln!(out, "    __u32 src_ip = *(__u32 *)(data + 26);").map_err(err)?;
    writeln!(out).map_err(err)?;

    writeln!(out, "    __u32 key = src_ip;").map_err(err)?;
    writeln!(out, "    __u64 *count_ptr = bpf_map_lookup_elem(&connection_counter, &key);").map_err(err)?;
    writeln!(out, "    if (count_ptr) {{").map_err(err)?;
    writeln!(out, "        __sync_fetch_and_add(count_ptr, 1);").map_err(err)?;
    writeln!(out, "    }}").map_err(err)?;
    writeln!(out).map_err(err)?;

    writeln!(out, "    return 1;").map_err(err)?;
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
