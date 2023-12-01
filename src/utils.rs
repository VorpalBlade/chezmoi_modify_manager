//! Some shared utility functions

use anyhow::anyhow;
use camino::{Utf8Path, Utf8PathBuf};

use duct::cmd;

/// Trait for interacting with chezmoi.
///
/// The purpose of making this a trait is to allow testing without using
/// the real chezmoi directory of the user (or even without chezmoi installed)
pub(crate) trait Chezmoi: std::fmt::Debug {
    fn source_path(&self, path: &Utf8Path) -> anyhow::Result<Option<Utf8PathBuf>>;
    fn source_root(&self) -> anyhow::Result<Option<Utf8PathBuf>>;
    fn add(&self, path: &Utf8Path) -> anyhow::Result<()>;
}

/// Trait implementation using the real chezmoi
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RealChezmoi();

impl Chezmoi for RealChezmoi {
    /// Get the source path of a file
    fn source_path(&self, path: &Utf8Path) -> anyhow::Result<Option<Utf8PathBuf>> {
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
    fn source_root(&self) -> anyhow::Result<Option<Utf8PathBuf>> {
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

    fn add(&self, path: &Utf8Path) -> anyhow::Result<()> {
        let out = cmd!("chezmoi", "add", path)
            .stdout_null()
            .unchecked()
            .run()?;
        if !out.status.success() {
            return Err(anyhow!("chezmoi add failed with error code {}", out.status));
        }
        Ok(())
    }
}
