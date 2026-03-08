use std::fmt::Write;
use std::path::Path;

use super::{helpers, maps, write, xdp};
use crate::{
    emit::ebpf_c::{cgroup, fentry, kprobe, lsm, raw_tracepoint, sk, tc, tracepoint},
    ir::ProgramIr,
};

pub fn emit_program(program: &ProgramIr, output: &Path) -> Result<(), String> {
    let mut c = String::new();
    
    emit_prelude(&mut c, program)?;
    
    helpers::emit_helpers(&mut c)?;
    maps::emit_maps(&mut c, &program.maps)?;
    
    for unit in &program.units {
        let sec0 = unit
            .sections
            .get(0)
            .map(|s| s.as_str())
            .unwrap_or("unknown");

        match sec0 {
            "xdp" => xdp::emit_xdp(&mut c, unit)?,
            
            "tc" | "classifier" => tc::emit_tc(&mut c, unit, "classifier")?,
            "tcx" | "tcx/egress" | "tc/egress" => tc::emit_tc(&mut c, unit, "tcx/egress")?,
            "tcx/ingress" | "tc/ingress" => tc::emit_tc(&mut c, unit, "tcx/ingress")?,
            
            "sk_skb/stream_parser" => sk::emit_sk_skb(&mut c, unit, "sk_skb/stream_parser")?,
            "sk_skb/stream_verdict" => sk::emit_sk_skb(&mut c, unit, "sk_skb/stream_verdict")?,
            "sk_msg" => sk::emit_sk_msg(&mut c, unit)?,
            
            "cgroup/skb/ingress" => cgroup::emit_cgroup(&mut c, unit, "cgroup/skb/ingress")?,
            "cgroup/skb/egress" => cgroup::emit_cgroup(&mut c, unit, "cgroup/skb/egress")?,
            "cgroup/sock" => cgroup::emit_cgroup(&mut c, unit, "cgroup/sock")?,
            "cgroup/sock_addr" => cgroup::emit_cgroup_sock_addr(&mut c, unit)?,
            
            s if s.starts_with("kprobe/") || s.starts_with("kretprobe/") => {
                kprobe::emit_kprobe(&mut c, unit, s)?
            }
            s if s.starts_with("raw_tracepoint/") => {
                raw_tracepoint::emit_raw_tracepoint(&mut c, unit, s)?
            }
            s if s.starts_with("tracepoint/") => tracepoint::emit_tracepoint(&mut c, unit, s)?,
            
            s if s.starts_with("fentry/") || s.starts_with("fexit/") => {
                fentry::emit_fentry(&mut c, unit, s)?
            }
            s if s.starts_with("lsm/") => lsm::emit_lsm(&mut c, unit, s)?,

            s => return Err(format!("Unsupported section: {}", s)),
        }
    }
    
    write::compile_to_object(&c, output)?;
    Ok(())
}

fn emit_prelude(out: &mut String, program: &ProgramIr) -> Result<(), String> {
    writeln!(out, "#include \"vmlinux.h\"").map_err(err)?;
    writeln!(out, "#include <bpf/bpf_helpers.h>").map_err(err)?;
    writeln!(out, "#include <bpf/bpf_endian.h>\n").map_err(err)?;
    
    let mut needs_tc = false;
    let mut needs_xdp = false;
    let mut needs_sk = false;
    
    for unit in &program.units {
        for sec in &unit.sections {
            let s = sec.as_str();
            if s.contains("tc") || s == "classifier" { needs_tc = true; }
            if s.contains("xdp") { needs_xdp = true; }
            if s.contains("sk_") { needs_sk = true; }
        }
    }

    if needs_tc {
        writeln!(out, "/* TC Action codes */").map_err(err)?;
        writeln!(out, "#ifndef TC_ACT_OK").map_err(err)?;
        writeln!(out, "#define TC_ACT_OK 0").map_err(err)?;
        writeln!(out, "#define TC_ACT_SHOT 2").map_err(err)?;
        writeln!(out, "#define TC_ACT_UNSPEC -1").map_err(err)?;
        writeln!(out, "#endif\n").map_err(err)?;
    }

    if needs_xdp {
        writeln!(out, "/* XDP Action codes */").map_err(err)?;
        writeln!(out, "#ifndef XDP_ABORTED").map_err(err)?;
        writeln!(out, "#define XDP_ABORTED 0").map_err(err)?;
        writeln!(out, "#define XDP_DROP 1").map_err(err)?;
        writeln!(out, "#define XDP_PASS 2").map_err(err)?;
        writeln!(out, "#define XDP_TX 3").map_err(err)?;
        writeln!(out, "#define XDP_REDIRECT 4").map_err(err)?;
        writeln!(out, "#endif\n").map_err(err)?;
    }

    if needs_sk {
        writeln!(out, "/* Socket Action codes */").map_err(err)?;
        writeln!(out, "#ifndef SK_PASS").map_err(err)?;
        writeln!(out, "#define SK_PASS 1").map_err(err)?;
        writeln!(out, "#define SK_DROP 0").map_err(err)?;
        writeln!(out, "#endif\n").map_err(err)?;
    }

    Ok(())
}

fn err(e: std::fmt::Error) -> String {
    e.to_string()
}