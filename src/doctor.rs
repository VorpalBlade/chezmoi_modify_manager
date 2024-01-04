//! Sanity checking of environment

use itertools::Itertools;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::process::Command;
use strum::Display;

use anyhow::{anyhow, Context};

use crate::utils::{Chezmoi, RealChezmoi};

/// Perform environment sanity check
pub(crate) fn doctor() -> anyhow::Result<()> {
    let mut issues_found = false;
    println!("RESULT    CHECK                MESSAGE");
    for Check { name, func } in &CHECKS {
        match func() {
            Ok((result, text)) => {
                let text = text.replace('\n', "\n                               ");
                println!("{result: <9} {name: <20} {text}");
                if result >= CheckResult::Warning {
                    issues_found = true;
                }
            }
            Err(err) => {
                println!("FATAL     {name: <20} {err}");
                issues_found = true;
            }
        }
    }
    if let Ok(p) = which::which("chezmoi") {
        println!("\nOutput of chezmoi doctor:");
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

    if issues_found {
        println!();
        return Err(anyhow!(
            "Issues found, please rectify these for proper operation"
        ));
    }
    Ok(())
}

/// Result of a check
#[derive(Debug, Display, PartialEq, Eq, PartialOrd, Ord)]
enum CheckResult {
    Ok,
    Info,
    Warning,
    Error,
}

/// A check with a name
#[derive(Debug)]
struct Check {
    name: &'static str,
    func: fn() -> anyhow::Result<(CheckResult, String)>,
}

const CHECKS: [Check; 7] = [
    Check {
        name: "version",
        func: || Ok((CheckResult::Info, env!("CARGO_PKG_VERSION").to_owned())),
    },
    Check {
        name: "rustc-version",
        func: || {
            Ok((
                CheckResult::Info,
                format!("{}", rustc_version_runtime::version()),
            ))
        },
    },
    Check {
        name: "host",
        func: || {
            let info = os_info::get();
            Ok((
                CheckResult::Info,
                format!(
                    "os={}, arch={}, info={}",
                    std::env::consts::OS,
                    std::env::consts::ARCH,
                    info
                ),
            ))
        },
    },
    Check {
        name: "has-chezmoi",
        func: chezmoi_check,
    },
    Check {
        name: "in-path",
        func: || match which::which("chezmoi_modify_manager") {
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
        },
    },
    Check {
        name: "has-ignore",
        func: check_has_ignore,
    },
    Check {
        name: "no-hook-script",
        func: || match hook_paths(&RealChezmoi::default())?.as_slice() {
            [] => Ok((CheckResult::Ok, "No legacy hook script found".to_string())),
            values => {
                let values: String =
                    Itertools::intersperse(values.iter().map(|v| v.as_str()), "\n* ").collect();
                Ok((
                    CheckResult::Error,
                    format!("Legacy hook script(s) found:\n* {values}\nPlease read https://github.com/VorpalBlade/chezmoi_modify_manager/blob/main/doc/migration_3.md"),
                ))
            }
        },
    },
];

// Find any legacy hook paths that might exist
fn hook_paths(chezmoi: &impl Chezmoi) -> anyhow::Result<Vec<camino::Utf8PathBuf>> {
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
fn chezmoi_check() -> anyhow::Result<(CheckResult, String)> {
    match which::which("chezmoi") {
        Ok(p) => {
            let res = Command::new(p).arg("--version").output();
            match res {
                Ok(out) => match std::str::from_utf8(&out.stdout) {
                    Ok(version) => {
                        let version = version.trim_end();
                        Ok((
                            CheckResult::Ok,
                            format!("Chezmoi found. Version: {version}"),
                        ))
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

fn check_has_ignore() -> anyhow::Result<(CheckResult, String)> {
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
