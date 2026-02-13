use std::{fs, path::Path, process::Command};

pub fn ensure_vmlinux(build_dir: &Path) -> Result<(), String> {
    let header = build_dir.join("vmlinux.h");

    if header.exists() {
        return Ok(());
    }

    println!("Generating vmlinux.h...");

    let output = Command::new("bpftool")
        .args([
            "btf",
            "dump",
            "file",
            "/sys/kernel/btf/vmlinux",
            "format",
            "c",
        ])
        .output()
        .map_err(|e| format!("failed to run bpftool: {e}"))?;

    if !output.status.success() {
        return Err("bpftool failed to generate vmlinux.h".into());
    }

    fs::write(&header, output.stdout)
        .map_err(|e| format!("failed to write vmlinux.h: {e}"))?;

    Ok(())
}
