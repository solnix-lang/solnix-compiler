pub mod layout;
pub mod vmlinux;

use std::path::Path;
use std::fs;
use crate::emit::ebpf_c::program::emit_program;
use crate::parser::parse;
use layout::prepare_build_dir;
use vmlinux::ensure_vmlinux;

pub fn build(input: &Path) -> Result<(), String> {
    let src = fs::read_to_string(input)
        .map_err(|e| format!("Failed to read input file: {}", e))?;
    
    let program = parse(&src)
        .map_err(|e| format!("{:?}", e))?;

    // Create .snx/build/
    let build_dir = prepare_build_dir()?;

    // Ensure vmlinux.h exists
    ensure_vmlinux(&build_dir)?;

    let file_stem = input
        .file_stem()
        .unwrap()
        .to_string_lossy();

    let output = build_dir.join(format!("{file_stem}.o"));
    
    let program_ir = crate::ir::lower_program(&program)
        .map_err(|e| format!("{:?}", e))?;

    emit_program(&program_ir, &output)?;

    println!("Finished [bpf] → {}", output.display());
    Ok(())
}
