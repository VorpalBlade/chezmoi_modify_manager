//! Some shared utility functions

use std::path::{Path, PathBuf};

use duct::cmd;

/// Get the source path of a file
pub(crate) fn chezmoi_source_path(path: &Path) -> anyhow::Result<Option<PathBuf>> {
    let output = cmd!("chezmoi", "source-path", path)
        .stdout_capture()
        .stderr_null()
        .unchecked()
        .run()?;
    if !output.status.success() {
        return Ok(None);
    }
    Ok(Some(String::from_utf8(output.stdout)?.trim_end().into()))
}

/// Get the path of the chezmoi source root
pub(crate) fn chezmoi_source_root() -> anyhow::Result<Option<PathBuf>> {
    let output = cmd!("chezmoi", "source-path")
        .stdout_capture()
        .stderr_null()
        .unchecked()
        .run()?;
    if !output.status.success() {
        return Ok(None);
    }
    Ok(Some(String::from_utf8(output.stdout)?.trim_end().into()))
}
