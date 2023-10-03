//! Sanity checking of environment

use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::process::Command;
use strum::Display;

use anyhow::anyhow;

use crate::utils::chezmoi_source_root;

/// Perform environment sanity check
pub(crate) fn doctor() -> anyhow::Result<()> {
    let mut issues_found = false;
    println!("RESULT    CHECK                MESSAGE");
    for Check { name, func } in &CHECKS {
        match func() {
            Ok((result, text)) => {
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

const CHECKS: [Check; 6] = [
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
];

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
                            CheckResult::Info,
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
        let src_path = chezmoi_source_root()?;
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
                return Ok((CheckResult::Info, "Ignore of **/*.src.ini found".to_owned()));
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
