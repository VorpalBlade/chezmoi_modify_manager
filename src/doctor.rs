//! Sanity checking of environment

use anstream::{println, stdout};
use anstyle::{Effects, Reset};
use itertools::Itertools;
use std::env::VarError;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::process::Command;

use medic::{Check, CheckResult};

use anyhow::{anyhow, Context};

use crate::utils::{Chezmoi, ChezmoiVersion, RealChezmoi, CHEZMOI_AUTO_SOURCE_VERSION};

/// Perform environment sanity check
pub(crate) fn doctor() -> anyhow::Result<()> {
    let worst_issues_found = medic::medic(&mut stdout(), CHECKS.iter())?;

    run_chezmoi_doctor();

    medic::summary(&mut stdout(), worst_issues_found)?;
    if worst_issues_found >= CheckResult::Warning {
        // There isn't a good way to get a non-zero exit code without also
        // getting an anyhow error printed from here.
        std::process::exit(1);
    }
    Ok(())
}

fn run_chezmoi_doctor() {
    if let Ok(p) = which::which("chezmoi") {
        println!(
            "\n{}Output of chezmoi doctor:{}",
            Effects::BOLD.render(),
            Reset.render()
        );
        _ = std::io::stdout().flush();
        match Command::new(p).arg("doctor").spawn().as_mut() {
            Ok(child) => {
                if let Err(err) = child.wait() {
                    println!("chezmoi doctor failed with {err}");
                }
            }
            Err(_) => println!("Failed to run chezmoi doctor!"),
        }
    } else {
        println!("\nchezmoi doctor output not included since binary wasn't found");
    }
}

const CHECKS: [Check; 9] = [
    medic::checks::crate_version_check!(),
    Check::new("build", || {
        match option_env!("CHEZMOI_MODIFY_MANAGER_BUILDER") {
            Some("github-release") => Ok((CheckResult::Ok, "Official release build".to_string())),
            Some("github-ci") => Ok((
                CheckResult::Warning,
                "Github CI build (not official release)".to_string(),
            )),
            Some(s) => Ok((
                CheckResult::Info,
                format!("Other builder, identifies as: {s}"),
            )),
            None => Ok((
                CheckResult::Warning,
                "Unknown builder, no identity set".to_string(),
            )),
        }
    }),
    medic::checks::CHECK_RUSTC_VERSION,
    medic::checks::CHECK_HOST,
    Check::new("has-chezmoi", chezmoi_check),
    Check::new("chezmoi-override", chezmoi_version_override_check),
    Check::new("in-path", || match which::which("chezmoi_modify_manager") {
        Ok(p) => {
            let p = p.to_string_lossy();
            Ok((
                CheckResult::Ok,
                format!("chezmoi_modify_manager is in PATH at {p}"),
            ))
        }
        Err(err) => Ok((
            CheckResult::Error,
            format!("chezmoi_modify_manager is NOT in PATH: {err}"),
        )),
    }),
    Check::new("has-ignore", check_has_ignore),
    Check::new("no-hook-script", || {
        match hook_paths(&RealChezmoi::default())?.as_slice() {
            [] => Ok((CheckResult::Ok, "No legacy hook script found".to_string())),
            values => {
                let values: String =
                    Itertools::intersperse(values.iter().map(|v| v.as_str()), "\n* ").collect();
                Ok((
                    CheckResult::Error,
                    format!("Legacy hook script(s) found:\n* {values}\nPlease read https://github.com/VorpalBlade/chezmoi_modify_manager/blob/main/doc/migration_3.md"),
                ))
            }
        }
    }),
];

// Find any legacy hook paths that might exist
pub(crate) fn hook_paths(chezmoi: &impl Chezmoi) -> anyhow::Result<Vec<camino::Utf8PathBuf>> {
    let ch_path = chezmoi
        .source_root()
        .context("Failed to run chezmoi")?
        .context("No chezmoi source directory seems to exist?")?;
    let mut paths = vec![];
    let path = ch_path.join(".chezmoi_modify_manager.add_hook");
    if path.exists() {
        paths.push(path);
    }
    let base_path = ch_path.join(".chezmoi_modify_manager.add_hook.*");
    for candidate in glob::glob_with(
        base_path.as_str(),
        glob::MatchOptions {
            case_sensitive: true,
            require_literal_separator: true,
            require_literal_leading_dot: true,
        },
    )? {
        paths.push(candidate?.try_into()?);
    }
    Ok(paths)
}

/// Find chezmoi and check it's version
fn chezmoi_check() -> Result<(CheckResult, String), Box<dyn std::error::Error + Send + Sync>> {
    match which::which("chezmoi") {
        Ok(p) => {
            let res = Command::new(p).arg("--version").output();
            match res {
                Ok(out) => match std::str::from_utf8(&out.stdout) {
                    Ok(version) => {
                        let version = version.trim_end();
                        let parsed_version: ChezmoiVersion =
                            ChezmoiVersion::from_version_output(version)
                                .context("Failed to parse chezmoi --version")?;
                        if parsed_version < CHEZMOI_AUTO_SOURCE_VERSION {
                            Ok((
                                CheckResult::Warning,
                                format!("Chezmoi found. Version is old and doesn't support \"source auto\" directive: {version}"),
                            ))
                        } else {
                            Ok((
                                CheckResult::Ok,
                                format!("Chezmoi found. Version: {version}"),
                            ))
                        }
                    }
                    Err(err) => Ok((
                        CheckResult::Error,
                        format!("Failed to parse --version output as UTF-8: {err}"),
                    )),
                },
                Err(err) => Ok((
                    CheckResult::Error,
                    format!("Failed to execute chezmoi: {err}"),
                )),
            }
        }
        Err(err) => Ok((
            CheckResult::Error,
            format!("chezmoi not found in PATH: {err}"),
        )),
    }
}

#[derive(Debug, thiserror::Error)]
enum ChezmoiVersionOverrideCheckError {
    #[error("Failed to extract CHEZMOI_MODIFY_MANAGER_ASSUME_CHEZMOI_VERSION: {0}")]
    DecodeError(#[from] VarError),
    #[error("Failed to parse CHEZMOI_MODIFY_MANAGER_ASSUME_CHEZMOI_VERSION: {0}")]
    ParseError(#[from] anyhow::Error),
}

fn chezmoi_version_override_check(
) -> Result<(CheckResult, String), Box<dyn std::error::Error + Send + Sync>> {
    match std::env::var("CHEZMOI_MODIFY_MANAGER_ASSUME_CHEZMOI_VERSION") {
        Ok(value) => match ChezmoiVersion::from_env_var(&value) {
            Ok(parsed) => Ok((
                CheckResult::Warning,
                format!("CHEZMOI_MODIFY_MANAGER_ASSUME_CHEZMOI_VERSION is set: {parsed}"),
            )),
            Err(err) => Err(Box::new(ChezmoiVersionOverrideCheckError::from(err))),
        },
        Err(VarError::NotPresent) => Ok((
            CheckResult::Ok,
            "CHEZMOI_MODIFY_MANAGER_ASSUME_CHEZMOI_VERSION is not set".to_string(),
        )),
        Err(err) => Err(Box::new(ChezmoiVersionOverrideCheckError::from(err))),
    }
}

fn check_has_ignore() -> Result<(CheckResult, String), Box<dyn std::error::Error + Send + Sync>> {
    if which::which("chezmoi").is_ok() {
        let src_path = RealChezmoi::default().source_root()?;
        let mut src_path = src_path.ok_or(anyhow!("No chezmoi source root found"))?;
        src_path.push(".chezmoiignore");
        let file = File::open(src_path)?;
        let mut reader = BufReader::new(file);

        let mut buffer = String::new();
        while let Ok(len) = reader.read_line(&mut buffer) {
            if len == 0 {
                break;
            }
            if buffer.trim_end() == "**/*.src.ini" {
                return Ok((CheckResult::Ok, "Ignore of **/*.src.ini found".to_owned()));
            }
            buffer.clear();
        }

        Ok((
            CheckResult::Error,
            "Ignore of **/*.src.ini is missing from root .chezmoiignore".to_owned(),
        ))
    } else {
        Ok((
            CheckResult::Error,
            "chezmoi not found, can't check source directory".to_owned(),
        ))
    }
}
