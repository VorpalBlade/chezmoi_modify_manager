//! Support for adding files

use crate::utils::chezmoi_source_path;
use anyhow::anyhow;
use duct::cmd;
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};
use strum::{Display, EnumIter, EnumMessage, EnumString, IntoStaticStr};

/// The style of calls to the executable
#[derive(
    Debug, Eq, PartialEq, EnumString, Clone, Copy, EnumIter, EnumMessage, Display, IntoStaticStr,
)]
pub enum Style {
    /// Program is in PATH
    #[strum(serialize = "path")]
    InPath,
    /// Program is in .utils of chezmoi source state
    #[strum(serialize = "src")]
    InSrc,
}

/// The mode for adding
#[derive(Debug, Clone, Copy)]
pub(crate) enum Mode {
    Normal,
    Smart,
}

/// Template for newly created scripts
const TEMPLATE: &str = r#"#!(PATH)

# This is needed to figure out where the source file is.
# See https://github.com/twpayne/chezmoi/issues/2934
source "{{ .chezmoi.sourceDir }}/{{ .chezmoi.sourceFile | trimSuffix ".tmpl" | replace "modify_" "" }}.src.ini"

# Add your ignores and transforms here
#ignore section "my-section"
#ignore "exact section name without brackets" "exact key name"
#ignore regex "section.*" "key_prefix_.*"
#transform "section" "key" transform_name read="the docs" for="more detail on transforms"
"#;

/// Shebang line to use when command is in PATH
const IN_PATH: &str = "/usr/bin/env chezmoi_modify_manager";
/// Shebang line to use when command is in dotfile repo.
const IN_SRC: &str =
    "{{ .chezmoi.sourceDir }}/.utils/chezmoi_modify_manager-{{ .chezmoi.os }}-{{ .chezmoi.arch }}";

/// Format the template
fn template(path: &str) -> String {
    TEMPLATE.replace("(PATH)", path)
}

/// Get the path for the hook script, if it exists
fn hook_path() -> anyhow::Result<Option<PathBuf>> {
    let output = cmd!("chezmoi", "source-path")
        .stdout_capture()
        .unchecked()
        .run()?;
    if !output.status.success() {
        return Err(anyhow!("No chezmoi source directory seems to exist?"));
    }
    let ch_path = PathBuf::from(String::from_utf8(output.stdout)?.trim_end());
    if cfg!(win32) {
        let base_path = ch_path.join(".chezmoi_modify_manager.add_hook.*");
        let mut candidates: Vec<_> = glob::glob_with(
            base_path
                .to_str()
                .ok_or_else(|| anyhow!("Invalid path {base_path:?} for chezmoi source directory: not convertible to UTF-8."))?,
            glob::MatchOptions {
                case_sensitive: true,
                require_literal_separator: true,
                require_literal_leading_dot: true,
            },
        )?.collect();
        match candidates.len() {
            0 => Ok(None),
            1 => Ok(Some(candidates.remove(0)?)),
            _ => Err(anyhow!("Too many add_hook scripts found")),
        }
    } else {
        let path = ch_path.join(".chezmoi_modify_manager.add_hook");
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(path))
    }
}

/// Perform actual adding with a script
fn add_with_script(path: &Path, style: Style) -> anyhow::Result<()> {
    let out = cmd!("chezmoi", "add", path)
        .stdout_null()
        .unchecked()
        .run()?;
    if !out.status.success() {
        return Err(anyhow!("chezmoi add failed with error code {}", out.status));
    }
    let src_path = chezmoi_source_path(path)?.ok_or(anyhow!("chezmoi couldn't find added file"))?;
    let src_name = src_path
        .file_name()
        .ok_or(anyhow!("File has no filename"))?
        .to_string_lossy();
    let data_path = src_path.with_file_name(format!("{src_name}.src.ini"));
    let script_path = src_path.with_file_name(format!("modify_{src_name}.tmpl"));
    // Run user provided hook script (if one exists)
    add_or_hook(path, &data_path, &src_path, true)?;

    maybe_create_script(script_path, style)?;
    Ok(())
}

/// Maybe run the hook script
///
/// * `input_path`: Path provided by user
/// * `target_path`: Path to write to
/// * `src_path`: Path to actually read file data from
fn add_or_hook(
    input_path: &Path,
    target_path: &Path,
    src_path: &Path,
    input_is_temporary: bool,
) -> Result<(), anyhow::Error> {
    if let Some(hook_path) = hook_path()? {
        println!("    Executing hook script...");
        let out = cmd!(hook_path, "ini", input_path, &target_path)
            .stdin_path(src_path)
            .stdout_path(target_path)
            .unchecked()
            .run()?;
        if !out.status.success() {
            return Err(anyhow!("Hook script failed with error code {}", out.status));
        }
        if input_is_temporary {
            std::fs::remove_file(src_path)?;
        }
    } else if input_is_temporary {
        std::fs::rename(dbg!(src_path), dbg!(target_path))?;
    } else {
        std::fs::copy(src_path, target_path)?;
    }
    Ok(())
}

/// Create a modify script if one doesn't exist
fn maybe_create_script(script_path: PathBuf, style: Style) -> anyhow::Result<()> {
    if script_path.exists() {
        return Ok(());
    }
    let mut file = File::create(&script_path)?;
    file.write_all(
        template(match style {
            Style::InPath => IN_PATH,
            Style::InSrc => IN_SRC,
        })
        .as_bytes(),
    )?;
    println!("    New script at {script_path:?}");

    Ok(())
}

/// Add a file
pub(crate) fn add(mode: Mode, style: Style, path: &Path) -> anyhow::Result<()> {
    if !path.is_file() {
        return Err(anyhow!("{:?} is not a regular file", path));
    }
    let src_path = chezmoi_source_path(path)?;
    match src_path {
        Some(existing_file) => {
            println!("  Existing (to chezmoi) file: {existing_file:?}");
            // Existing file
            let src_filename = existing_file
                .file_name()
                .ok_or(anyhow!("No file name?"))?
                .to_string_lossy();
            let src_dir = existing_file
                .parent()
                .ok_or(anyhow!("Couldn't extract directory"))?;
            let is_mod_script = src_filename.starts_with("modify_");
            if is_mod_script {
                println!("    Updating existing .src.ini file for {existing_file:?}...");
                let data_file = src_filename
                    .strip_prefix("modify_")
                    .and_then(|s| s.strip_suffix(".tmpl").or(Some(s)))
                    .ok_or(anyhow!("This should never happen"))?
                    .to_owned()
                    + ".src.ini";
                let mut targeted_file: PathBuf = src_dir.into();
                targeted_file.push(data_file);
                add_or_hook(path, targeted_file.as_ref(), path, false)?;
            } else {
                println!("    Existing, but not a modify script...");
                // Existing, but not modify script.
                add_file_basic(mode, path, style)?;
            }
        }
        None => {
            // New file
            println!("  New (to chezmoi) file {path:?}");
            add_file_basic(mode, path, style)?;
        }
    }
    Ok(())
}

/// Basic file adding
fn add_file_basic(mode: Mode, path: &Path, style: Style) -> Result<(), anyhow::Error> {
    match mode {
        Mode::Normal => {
            println!("    Setting up new modify_ script...");
            add_with_script(path, style)?
        }
        Mode::Smart => {
            println!("    In smart mode... Adding as plain chezmoi...");
            let out = cmd!("chezmoi", "add", path)
                .stdout_null()
                .unchecked()
                .run()?;
            if !out.status.success() {
                return Err(anyhow!("Failed to add file {:?} with chezmoi", path));
            }
        }
    }
    Ok(())
}
