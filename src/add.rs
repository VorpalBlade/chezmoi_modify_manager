//! Support for adding files

#[cfg(test)]
mod tests;

use crate::{config, utils::Chezmoi};
use anyhow::{anyhow, Context};
use camino::{Utf8Path, Utf8PathBuf};
use duct::cmd;
use indoc::formatdoc;
use ini_merge::filter::filter_ini;
use std::{fs::File, io::Write};
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
fn hook_path(chezmoi: &impl Chezmoi) -> anyhow::Result<Option<Utf8PathBuf>> {
    let ch_path = chezmoi
        .source_root()
        .context("Failed to run chezmoi")?
        .context("No chezmoi source directory seems to exist?")?;
    if cfg!(windows) {
        let base_path = ch_path.join(".chezmoi_modify_manager.add_hook.*");
        let mut candidates: Vec<_> = glob::glob_with(
            base_path.as_str(),
            glob::MatchOptions {
                case_sensitive: true,
                require_literal_separator: true,
                require_literal_leading_dot: true,
            },
        )?
        .collect();
        match candidates.len() {
            0 => Ok(None),
            1 => Ok(Some(
                candidates
                    .remove(0)?
                    .try_into()
                    .context("Only valid Unicode file names supported")?,
            )),
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
fn add_with_script(
    chezmoi: &impl Chezmoi,
    path: &Utf8Path,
    style: Style,
    status_out: &mut impl Write,
) -> anyhow::Result<()> {
    chezmoi.add(path)?;
    let src_path = chezmoi
        .source_path(path)?
        .context("chezmoi couldn't find added file")?;
    let src_name = src_path.file_name().context("File has no filename")?;
    let data_path = src_path.with_file_name(format!("{src_name}.src.ini"));
    let script_path = src_path.with_file_name(format!("modify_{src_name}.tmpl"));
    // Run user provided hook script (if one exists)
    filtered_add(chezmoi, path, &data_path, &src_path, None, true, status_out)?;

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
    chezmoi: &impl Chezmoi,
    input_path: &Utf8Path,
    target_path: &Utf8Path,
    src_path: &Utf8Path,
    script_path: Option<&Utf8Path>,
    input_is_temporary: bool,
    status_out: &mut impl Write,
) -> Result<(), anyhow::Error> {
    // First pass through hook if one exists, otherwise load directly.
    let file_contents = if let Some(hook_path) = hook_path(chezmoi)? {
        _ = writeln!(status_out, "Executing hook script...");
        _ = writeln!(
            status_out,
            "\n!!! Hook scripts are deprecated (and will be removed at a future point) !!!"
        );
        _ = writeln!(
            status_out,
            "    Migrate to the new add:remove and add:hide attributes if possible."
        );
        _ = writeln!(
            status_out,
            "    If not possible, please leave a comment on this issue describing your use case:"
        );
        _ = writeln!(
            status_out,
            "    https://github.com/VorpalBlade/chezmoi_modify_manager/issues/46\n"
        );
        run_hook(&hook_path, input_path, target_path, src_path)?
    } else {
        std::fs::read(src_path).context("Failed to load data from file we are adding")?
    };
    // Remove temporary file if we are supposed to.
    if input_is_temporary {
        std::fs::remove_file(src_path)?;
    }

    // If we are updating an existing script, run the contents through the filtering
    let mut file_contents = if let Some(sp) = script_path {
        _ = writeln!(
            status_out,
            "Has existing modify script, parsing to check for filtering..."
        );
        let config_data = std::fs::read_to_string(sp).context("Failed to load modify script")?;
        internal_filter(&config_data, &file_contents)?
    } else {
        file_contents
    };

    if !file_contents.ends_with(b"\n") {
        file_contents.push(b'\n');
    }

    _ = writeln!(status_out, "Writing out file data");
    std::fs::write(target_path, file_contents)?;
    Ok(())
}

/// Perform internal filtering using add:hide and add:remove (modern filtering)
fn internal_filter(config_data: &str, contents: &[u8]) -> anyhow::Result<Vec<u8>> {
    let config = config::parse_for_add(config_data)?;
    let mut file = std::io::Cursor::new(contents);
    let result = filter_ini(&mut file, &config.mutations)?;
    let s: String = itertools::intersperse(result, "\n".into()).collect();
    Ok(s.as_bytes().into())
}

/// Run hook script (legacy filtering)
fn run_hook(
    hook_path: &Utf8Path,
    input_path: &Utf8Path,
    target_path: &Utf8Path,
    src_path: &Utf8Path,
) -> Result<Vec<u8>, anyhow::Error> {
    let out = cmd!(hook_path.as_std_path(), "ini", input_path, &target_path)
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
    script_path: Utf8PathBuf,
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
    chezmoi: &impl Chezmoi,
    mode: Mode,
    style: Style,
    path: &Utf8Path,
    status_out: &mut impl Write,
) -> anyhow::Result<()> {
    if !path.is_file() {
        return Err(anyhow!("{:?} is not a regular file", path));
    }
    // First check if the file exists.
    let src_path = chezmoi.source_path(path)?;
    match src_path {
        Some(existing_file) => {
            _ = writeln!(status_out, "Existing (to chezmoi) file: {existing_file:?}");
            // Existing file
            let src_filename = existing_file.file_name().context("No file name?")?;
            let src_dir = existing_file
                .parent()
                .context("Couldn't extract directory")?;
            let is_mod_script = src_filename.starts_with("modify_");
            if is_mod_script {
                _ = writeln!(
                    status_out,
                    "Updating existing .src.ini file for {existing_file:?}..."
                );
                readd_managed(chezmoi, &existing_file, src_dir, path, status_out)?;
            } else {
                _ = writeln!(status_out, "Existing, but not a modify script...");
                add_unmanaged(chezmoi, mode, path, style, status_out)?;
            }
        }
        None => {
            // New file
            _ = writeln!(status_out, "New (to chezmoi) file {path:?}");
            add_unmanaged(chezmoi, mode, path, style, status_out)?;
        }
    }
    Ok(())
}

/// Handle case of readding a file that already has a modify script.
fn readd_managed(
    chezmoi: &impl Chezmoi,
    modify_script: &Utf8Path,
    src_dir: &Utf8Path,
    path: &Utf8Path,
    status_out: &mut impl Write,
) -> Result<(), anyhow::Error> {
    let data_file = modify_script
        .file_name()
        .context("Failed to get filename")?
        .strip_prefix("modify_")
        .and_then(|s| s.strip_suffix(".tmpl").or(Some(s)))
        .context("This should never happen")?
        .to_owned()
        + ".src.ini";
    let mut targeted_file: Utf8PathBuf = src_dir.into();
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
        chezmoi,
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
    chezmoi: &impl Chezmoi,
    mode: Mode,
    path: &Utf8Path,
    style: Style,
    status_out: &mut impl Write,
) -> Result<(), anyhow::Error> {
    match mode {
        Mode::Normal => {
            _ = writeln!(status_out, "Setting up new modify_ script...");
            add_with_script(chezmoi, path, style, status_out)?
        }
        Mode::Smart => {
            _ = writeln!(status_out, "In smart mode... Adding as plain chezmoi...");
            chezmoi.add(path)?;
        }
    }
    Ok(())
}
