use std::fs;
use std::path::PathBuf;

pub fn prepare_build_dir() -> Result<PathBuf, String> {
    let build_dir = PathBuf::from(".snx/build");

    fs::create_dir_all(&build_dir)
        .map_err(|e| format!("failed to create build directory: {e}"))?;

    Ok(build_dir)
}
