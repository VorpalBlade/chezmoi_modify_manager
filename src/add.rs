//! Support for adding files

// Doc comments are used to generate --help, not to for rustdoc.
#![allow(clippy::doc_markdown)]

use crate::config;
use crate::utils::CHEZMOI_AUTO_SOURCE_VERSION;
use crate::utils::Chezmoi;
use crate::utils::ChezmoiVersion;
use anyhow::Context;
use anyhow::anyhow;
use camino::Utf8Path;
use camino::Utf8PathBuf;
use indoc::formatdoc;
use ini_merge::filter::filter_ini;
use std::fs::File;
use std::io::Write;
use strum::Display;
use strum::EnumIter;
use strum::EnumMessage;
use strum::EnumString;
use strum::IntoStaticStr;

#[cfg(test)]
mod tests;

/// The style of calls to the executable
#[derive(
    Debug, Eq, PartialEq, EnumString, Clone, Copy, EnumIter, EnumMessage, Display, IntoStaticStr,
)]
pub enum Style {
    /// Selects between path and path-tmpl based on detected chezmoi version
    #[strum(serialize = "auto")]
    Auto,
    /// chezmoi_modify_manager is searched for in PATH
    ///    (modify_ script is not templated for best performance)
    #[strum(serialize = "path")]
    InPath,
    /// chezmoi_modify_manager is searched for in PATH
    ///    (modify_ script is templated for your convenience)
    #[strum(serialize = "path-tmpl")]
    InPathTmpl,
    /// Program is in .utils of chezmoi source state
    ///    (modify_ script is always templated)
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
const TEMPLATE: &str = indoc::indoc! {r#"
    #!(PATH)

    (SOURCE)

    # Add your ignores and transforms here
    #ignore section "my-section"
    #ignore "exact section name without brackets" "exact key name"
    #ignore regex "section.*" "key_prefix_.*"
    #transform "section" "key" transform_name read="the docs" for="more detail on transforms"
"#};

const SOURCE_NEW: &str = "source auto";
const SOURCE_OLD: &str = indoc::indoc! {r#"
    # This is needed to figure out where the source file is on older Chezmoi versions.
    # See https://github.com/twpayne/chezmoi/issues/2934
    source "{{ .chezmoi.sourceDir }}/{{ .chezmoi.sourceFile | trimSuffix ".tmpl" | replace "modify_" "" }}.src.ini""#};

/// Shebang line to use when command is in PATH
const IN_PATH: &str = "/usr/bin/env chezmoi_modify_manager";
/// Shebang line to use when command is in dotfile repo.
const IN_SRC: &str =
    "{{ .chezmoi.sourceDir }}/.utils/chezmoi_modify_manager-{{ .chezmoi.os }}-{{ .chezmoi.arch }}";

/// Format the template
fn template(path: &str, version: &ChezmoiVersion) -> String {
    let result = TEMPLATE.replace("(PATH)", path);
    if version < &CHEZMOI_AUTO_SOURCE_VERSION {
        result.replace("(SOURCE)", SOURCE_OLD)
    } else {
        result.replace("(SOURCE)", SOURCE_NEW)
    }
}

/// Perform actual adding with a script
fn add_with_script(
    chezmoi: &impl Chezmoi,
    src_path: Option<Utf8PathBuf>,
    path: &Utf8Path,
    style: Style,
    status_out: &mut impl Write,
) -> anyhow::Result<()> {
    chezmoi.add(path)?;
    // If we don't already know the source path (newly added file), get it now
    let src_path = match src_path {
        Some(path) => path,
        None => chezmoi
            .source_path(path)?
            .context("chezmoi couldn't find added file")?,
    };
    let src_name = src_path.file_name().context("File has no filename")?;
    let data_path = src_path.with_file_name(format!("{src_name}.src.ini"));
    let script_path = match style {
        Style::Auto => panic!("Impossible: Auto should already have been mapped"),
        Style::InPath => src_path.with_file_name(format!("modify_{src_name}")),
        Style::InPathTmpl | Style::InSrc => {
            src_path.with_file_name(format!("modify_{src_name}.tmpl"))
        }
    };
    // Add while respecting filtering directives
    filtered_add(&data_path, &src_path, None, status_out)?;

    // Remove the temporary file that chezmoi added
    std::fs::remove_file(src_path)?;

    maybe_create_script(&script_path, style, status_out, &chezmoi.version()?)?;
    Ok(())
}

/// Add and handle filtering directives (add:remove, add:hide and ignore)
///
/// * `target_path`: Path to write to
/// * `src_path`: Path to actually read file data from
/// * `script_path`: Path to modify script (if it exists)
/// * `status_out`: Where to write status messages
fn filtered_add(
    target_path: &Utf8Path,
    src_path: &Utf8Path,
    script_path: Option<&Utf8Path>,
    status_out: &mut impl Write,
) -> Result<(), anyhow::Error> {
    let file_contents =
        std::fs::read(src_path).context("Failed to load data from file we are adding")?;

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

/// Create a modify script if one doesn't exist
fn maybe_create_script(
    script_path: &Utf8Path,
    style: Style,
    status_out: &mut impl Write,
    version: &ChezmoiVersion,
) -> anyhow::Result<()> {
    if script_path.exists() {
        return Ok(());
    }
    let mut file = File::create(script_path)?;
    file.write_all(
        template(
            match style {
                Style::Auto => panic!("Impossible: Auto should already have been mapped"),
                Style::InPath => IN_PATH,
                Style::InPathTmpl => IN_PATH,
                Style::InSrc => IN_SRC,
            },
            version,
        )
        .as_bytes(),
    )?;
    _ = writeln!(status_out, "New script at {script_path}");

    Ok(())
}

/// Classifies the state of the file in chezmoi source state.
#[derive(Debug)]
enum ChezmoiState {
    NotInChezmoi,
    ExistingNormal {
        data_path: Utf8PathBuf,
    },
    ExistingManaged {
        script_path: Utf8PathBuf,
        data_path: Utf8PathBuf,
    },
}

/// Add a file
pub(crate) fn add(
    chezmoi: &impl Chezmoi,
    mode: Mode,
    mut style: Style,
    path: &Utf8Path,
    status_out: &mut impl Write,
) -> anyhow::Result<()> {
    // Check for auto style
    if style == Style::Auto {
        style = if chezmoi.version()? < CHEZMOI_AUTO_SOURCE_VERSION {
            Style::InPathTmpl
        } else {
            Style::InPath
        }
    }
    // Start with a sanity check on the input file and environment
    sanity_check(path, style, chezmoi)?;

    // Let's check if the managed path exists
    let src_path = chezmoi.source_path(path)?;

    // Then lets classify the situation we are in
    let situation = classify_chezmoi_state(src_path)?;

    // Inform user of what we found
    match &situation {
        ChezmoiState::NotInChezmoi => {
            _ = writeln!(status_out, "State: New (to chezmoi) file");
        }
        ChezmoiState::ExistingNormal { .. } => {
            _ = writeln!(
                status_out,
                "State: Managed by chezmoi, but not a modify script."
            );
        }
        ChezmoiState::ExistingManaged { .. } => {
            _ = writeln!(
                status_out,
                "State: Managed by chezmoi and is a modify script."
            );
        }
    }

    // Finally decide on an action based on source state and the user selected mode.
    match (situation, mode) {
        (ChezmoiState::NotInChezmoi | ChezmoiState::ExistingNormal { .. }, Mode::Smart) => {
            _ = writeln!(
                status_out,
                "Action: Adding as plain chezmoi (since we are in smart mode)."
            );
            chezmoi.add(path)?;
        }
        (ChezmoiState::NotInChezmoi, Mode::Normal) => {
            _ = writeln!(
                status_out,
                "Action: Adding & setting up new modify_ script."
            );
            add_with_script(chezmoi, None, path, style, status_out)?;
        }
        (ChezmoiState::ExistingNormal { data_path }, Mode::Normal) => {
            _ = writeln!(
                status_out,
                "Action: Converting & setting up new modify_ script."
            );
            add_with_script(chezmoi, Some(data_path), path, style, status_out)?;
        }
        (
            ChezmoiState::ExistingManaged {
                script_path,
                data_path,
            },
            _,
        ) => {
            _ = writeln!(
                status_out,
                "Action: Updating existing .src.ini file for {script_path}."
            );
            filtered_add(
                data_path.as_ref(),
                path,
                Some(script_path.as_ref()),
                status_out,
            )?;
        }
    }
    Ok(())
}

/// Find out what the state of the file in chezmoi currently is.
fn classify_chezmoi_state(src_path: Option<Utf8PathBuf>) -> Result<ChezmoiState, anyhow::Error> {
    let situation = match src_path {
        Some(existing_file) => {
            let src_filename = existing_file.file_name().context("No file name?")?;
            let is_mod_script = src_filename.starts_with("modify_");
            if is_mod_script {
                let src_dir = existing_file
                    .parent()
                    .context("Couldn't extract directory")?;
                let targeted_file = find_data_file(&existing_file, src_dir)?;
                ChezmoiState::ExistingManaged {
                    script_path: existing_file,
                    data_path: targeted_file,
                }
            } else {
                ChezmoiState::ExistingNormal {
                    data_path: existing_file,
                }
            }
        }
        None => ChezmoiState::NotInChezmoi,
    };
    Ok(situation)
}

/// Perform preliminary environment sanity checks
fn sanity_check(
    path: &Utf8Path,
    style: Style,
    chezmoi: &impl Chezmoi,
) -> Result<(), anyhow::Error> {
    if !path.is_file() {
        return Err(anyhow!("{} is not a regular file", path));
    }
    if Style::InPath == style && chezmoi.version()? < CHEZMOI_AUTO_SOURCE_VERSION {
        return Err(anyhow!(
            "To use \"--style path\" you need chezmoi {CHEZMOI_AUTO_SOURCE_VERSION} or newer"
        ));
    }
    match crate::doctor::hook_paths(chezmoi)?.as_slice() {
        [] => Ok(()),
        _ => Err(anyhow!(
            "Legacy hook script found, see chezmoi_modify_manager --doctor and please read https://github.com/VorpalBlade/chezmoi_modify_manager/blob/main/doc/migration_3.md"
        )),
    }
}

/// Given a modify script, find the associated .src.ini file
fn find_data_file(
    modify_script: &Utf8Path,
    src_dir: &Utf8Path,
) -> Result<Utf8PathBuf, anyhow::Error> {
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
            r#"Found existing modify_ script but no associated .src.ini file (looked at {targeted_file}).
                        Possible causes:
                        * Did you change the "source" directive from the default value?
                        * Remove the file by mistake?

                        Either way: the automated adding code is not smart enough to handle this situation by itself."#
        );
        return Err(anyhow!(err_str));
    }
    Ok(targeted_file)
}
