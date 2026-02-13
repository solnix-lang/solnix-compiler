use std::{fs, path::Path, process::Command};

pub fn compile_to_object(code: &str, out: &Path) -> Result<(), String> {
    let build_dir = out.parent().unwrap();

    let c_file = out.with_extension("c");

    fs::write(&c_file, code)
        .map_err(|e| format!("failed to write C file: {e}"))?;

    let status = Command::new("clang")
        .args([
            "-O2",
            "-g",
            "-Wall",
            "-target", "bpf",
            "-D__TARGET_ARCH_x86",
            "-I",
            build_dir.to_str().unwrap(),
            "-c",
            c_file.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
        ])
        .status()
        .map_err(|e| format!("failed to run clang: {e}"))?;

    if !status.success() {
        return Err("clang compilation failed".into());
    }

    Ok(())
}
