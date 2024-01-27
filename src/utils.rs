//! Some shared utility functions

use std::str::FromStr;

use anyhow::{anyhow, Context};
use camino::{Utf8Path, Utf8PathBuf};
use duct::cmd;
use regex::Regex;

/// Represents the version number of chezmoi
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ChezmoiVersion(pub(crate) i32, pub(crate) i32, pub(crate) i32);

impl FromStr for ChezmoiVersion {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let re = Regex::new(r"v([0-9]+)\.([0-9]+)\.([0-9]+)")?;
        let caps = re
            .captures(s)
            .context("Failed to find chezmoi version string")?;
        Ok(Self(
            caps.get(1).unwrap().as_str().parse()?,
            caps.get(2).unwrap().as_str().parse()?,
            caps.get(3).unwrap().as_str().parse()?,
        ))
    }
}

impl std::fmt::Display for ChezmoiVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}.{}.{}", self.0, self.1, self.2)
    }
}

/// Minimum version of chezmoi to support "source auto" directive
pub(crate) const CHEZMOI_AUTO_SOURCE_VERSION: ChezmoiVersion = ChezmoiVersion(2, 47, 0);

/// Trait for interacting with chezmoi.
///
/// The purpose of making this a trait is to allow testing without using
/// the real chezmoi directory of the user (or even without chezmoi installed)
pub(crate) trait Chezmoi: std::fmt::Debug {
    fn source_path(&self, path: &Utf8Path) -> anyhow::Result<Option<Utf8PathBuf>>;
    fn source_root(&self) -> anyhow::Result<Option<Utf8PathBuf>>;
    fn add(&self, path: &Utf8Path) -> anyhow::Result<()>;
    fn version(&self) -> anyhow::Result<ChezmoiVersion>;
}

/// Trait implementation using the real chezmoi
#[derive(Debug, Clone, Default)]
pub(crate) struct RealChezmoi {
    version: std::cell::Cell<Option<ChezmoiVersion>>,
}

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

    fn version(&self) -> anyhow::Result<ChezmoiVersion> {
        match self.version.get() {
            None => {
                let output = cmd!("chezmoi", "--version")
                    .stdout_capture()
                    .stderr_null()
                    .unchecked()
                    .run()?;
                if !output.status.success() {
                    anyhow::bail!("Failed to run chezmoi --version");
                }
                let version: ChezmoiVersion =
                    String::from_utf8(output.stdout)?.trim_end().parse()?;
                self.version.set(Some(version));
                Ok(version)
            }
            Some(version) => Ok(version),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::ChezmoiVersion;

    #[test]
    fn test_chezmoi_version() {
        let version1 = ChezmoiVersion::from_str("v2.46.0").unwrap();
        assert_eq!(version1, ChezmoiVersion(2, 46, 0));
        let version2 =
            ChezmoiVersion::from_str("chezmoi version v2.47.0, built at 2024-01-26T07:31:10Z")
                .unwrap();
        assert_eq!(version2, ChezmoiVersion(2, 47, 0));
        assert!(version1 < version2);
    }
}
