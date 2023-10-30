//! Support for adding files

use crate::{config, utils::chezmoi_source_path};
use anyhow::{anyhow, Context};
use duct::cmd;
use indoc::formatdoc;
use ini_merge::filter::filter_ini;
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
    if cfg!(windows) {
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
fn add_with_script(path: &Path, style: Style, status_out: &mut impl Write) -> anyhow::Result<()> {
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
    filtered_add(path, &data_path, &src_path, None, true, status_out)?;

    maybe_create_script(script_path, style, status_out)?;
    Ok(())
}

/// Maybe run the hook script
///
/// * `input_path`: Path provided by user
/// * `target_path`: Path to write to
/// * `src_path`: Path to actually read file data from
/// * `script_path`: Path to modify script (if it exists)
fn filtered_add(
    input_path: &Path,
    target_path: &Path,
    src_path: &Path,
    script_path: Option<&Path>,
    input_is_temporary: bool,
    status_out: &mut impl Write,
) -> Result<(), anyhow::Error> {
    // First pass through hook if one exists, otherwise load directly.
    let file_contents = if let Some(hook_path) = hook_path()? {
        _ = writeln!(status_out, "Executing hook script...");
        run_hook(&hook_path, input_path, target_path, src_path)?
    } else {
        std::fs::read(src_path).context("Failed to load data from file we are adding")?
    };
    // Remove temporary file if we are supposed to.
    if input_is_temporary {
        std::fs::remove_file(src_path)?;
    }

    // If we are updating an existing script, run the contents through the filtering
    let file_contents = if let Some(sp) = script_path {
        internal_filter(sp, &file_contents, status_out)?
    } else {
        file_contents
    };

    _ = writeln!(status_out, "Writing out file data");
    std::fs::write(target_path, file_contents)?;
    Ok(())
}

fn internal_filter(
    script_path: &Path,
    contents: &[u8],
    status_out: &mut impl Write,
) -> anyhow::Result<Vec<u8>> {
    _ = writeln!(
        status_out,
        "Has existing modify script, parsing to check for filtering..."
    );
    let config = config::parse_for_add(
        &std::fs::read_to_string(script_path).context("Failed to load modify script")?,
    )?;
    let mut file = std::io::Cursor::new(contents);
    let result = filter_ini(&mut file, &config.mutations)?;
    let s: String = itertools::intersperse(result, "\n".into()).collect();
    Ok(s.as_bytes().into())
}

/// Run hook script
fn run_hook(
    hook_path: &Path,
    input_path: &Path,
    target_path: &Path,
    src_path: &Path,
) -> Result<Vec<u8>, anyhow::Error> {
    let out = cmd!(hook_path, "ini", input_path, &target_path)
        .stdin_path(src_path)
        .stdout_capture()
        .unchecked()
        .run()?;

    if !out.status.success() {
        return Err(anyhow!("Hook script failed with error code {}", out.status));
    }
    Ok(out.stdout)
}

/// Create a modify script if one doesn't exist
fn maybe_create_script(
    script_path: PathBuf,
    style: Style,
    status_out: &mut impl Write,
) -> anyhow::Result<()> {
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
    _ = writeln!(status_out, "New script at {script_path:?}");

    Ok(())
}

/// Add a file
pub(crate) fn add(
    mode: Mode,
    style: Style,
    path: &Path,
    status_out: &mut impl Write,
) -> anyhow::Result<()> {
    if !path.is_file() {
        return Err(anyhow!("{:?} is not a regular file", path));
    }
    // First check if the file exists.
    let src_path = chezmoi_source_path(path)?;
    match src_path {
        Some(existing_file) => {
            _ = writeln!(status_out, "Existing (to chezmoi) file: {existing_file:?}");
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
                _ = writeln!(
                    status_out,
                    "Updating existing .src.ini file for {existing_file:?}..."
                );
                readd_managed(&existing_file, src_dir, path, status_out)?;
            } else {
                _ = writeln!(status_out, "Existing, but not a modify script...");
                add_unmanaged(mode, path, style, status_out)?;
            }
        }
        None => {
            // New file
            _ = writeln!(status_out, "New (to chezmoi) file {path:?}");
            add_unmanaged(mode, path, style, status_out)?;
        }
    }
    Ok(())
}

/// Handle case of readding a file that already has a modify script.
fn readd_managed(
    modify_script: &Path,
    src_dir: &Path,
    path: &Path,
    status_out: &mut impl Write,
) -> Result<(), anyhow::Error> {
    let data_file = modify_script
        .file_name()
        .ok_or(anyhow!("Failed to get filename"))?
        .to_string_lossy()
        .strip_prefix("modify_")
        .and_then(|s| s.strip_suffix(".tmpl").or(Some(s)))
        .ok_or(anyhow!("This should never happen"))?
        .to_owned()
        + ".src.ini";
    let mut targeted_file: PathBuf = src_dir.into();
    targeted_file.push(data_file);
    if !targeted_file.exists() {
        let err_str = formatdoc!(
            r#"Found existing modify_ script but no associated .src.ini file (looked at {targeted_file:?}).
                        Possible causes:
                        * Did you change the "source" directive from the default value?
                        * Remove the file by mistake?

                        Either way: the automated adding code is not smart enough to handle this situation by itself."#
        );
        return Err(anyhow!(err_str));
    }
    filtered_add(
        path,
        targeted_file.as_ref(),
        path,
        Some(modify_script),
        false,
        status_out,
    )?;
    Ok(())
}

/// Basic file adding (when the file doesn't exist as a modify script already)
///
/// Note: It *may or may not* exist in chezmoi already, but not managed by this program.
fn add_unmanaged(
    mode: Mode,
    path: &Path,
    style: Style,
    status_out: &mut impl Write,
) -> Result<(), anyhow::Error> {
    match mode {
        Mode::Normal => {
            _ = writeln!(status_out, "Setting up new modify_ script...");
            add_with_script(path, style, status_out)?
        }
        Mode::Smart => {
            _ = writeln!(status_out, "In smart mode... Adding as plain chezmoi...");
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
