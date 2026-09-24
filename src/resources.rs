//! Resource discovery never uses the build machine's source directory.
use anyhow::{Result, bail};
use std::path::{Path, PathBuf};
pub fn pack_directory(explicit: Option<&Path>) -> Result<Option<PathBuf>> {
    if let Some(p) = explicit {
        if p.join("manifest.json").is_file() {
            return Ok(Some(p.to_path_buf()));
        }
        bail!("Invalid --packs directory: {}", p.display());
    }
    if let Some(p) = std::env::var_os("MOENOTES_ASSETS_DIR") {
        return pack_directory(Some(Path::new(&p)));
    }
    let mut candidates = vec![];
    if let Ok(exe) = std::env::current_exe()
        && let Some(p) = exe.parent()
    {
        candidates.push(p.join("assets"));
        candidates.push(p.join("../share/moenotes-chart-renderer"));
    }
    if let Some(base) = std::env::var_os("XDG_DATA_HOME") {
        candidates.push(PathBuf::from(base).join("moenotes-chart-renderer"));
    } else if let Some(home) = std::env::var_os("HOME") {
        candidates.push(PathBuf::from(home).join(".local/share/moenotes-chart-renderer"));
    }
    Ok(candidates
        .into_iter()
        .find(|p| p.join("manifest.json").is_file()))
}
