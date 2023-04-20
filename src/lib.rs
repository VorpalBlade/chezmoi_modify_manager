//! This is not a stable API, and is to be used internally by the binary and
//! the integration tests only.

use std::{
    fs::File,
    io::{Read, Write},
};

pub use add::Style;
use anyhow::Context;
pub use arguments::parse_args;
pub use arguments::ChmmArgs;

mod add;
mod arguments;
mod config;
mod transforms;
mod update;

use ini_merge::merge_ini;

/// Main function, amenable to integration tests.
pub fn inner_main<R: Read, W: Write, FR, FW>(
    opts: ChmmArgs,
    stdin: FR,
    stdout: FW,
) -> anyhow::Result<()>
where
    FR: FnOnce() -> R,
    FW: FnOnce() -> W,
{
    match opts {
        ChmmArgs::Process(file_name) => {
            let mut f = File::open(&file_name)
                .with_context(|| format!("Failed to open script at: {file_name:?}"))?;
            let mut buf = String::new();
            f.read_to_string(&mut buf)
                .with_context(|| format!("Failed to read script from {file_name:?}"))?;
            let c = config::parse(&buf)
                .with_context(|| format!("Failed to parse script from {file_name:?}"))?;

            let mut stdin = stdin();
            let mut stdout = stdout();
            let src_path = c
                .source_path(&file_name)
                .context("Failed to get source path")?;
            let mut src_file = File::open(&src_path)
                .with_context(|| format!("Failed to open source file at: {src_path:?}"))?;
            let merged = merge_ini(&mut stdin, &mut src_file, &c.mutations)?;
            for line in merged {
                writeln!(stdout, "{line}")?;
            }
        }
        ChmmArgs::Add { _a, files, style } => {
            for file in files {
                println!("Adding {file:?}");
                add::add(add::Mode::Normal, style, &file)?;
            }
        }
        ChmmArgs::Smart { _a, files, style } => {
            for file in files {
                println!("Adding {file:?}");
                add::add(add::Mode::Smart, style, &file)?;
            }
        }
        ChmmArgs::Update { _a } => {
            #[cfg(feature = "updater")]
            {
                update::update()?;
            }
            #[cfg(not(feature = "updater"))]
            {
                println!("Support for the updater was not included in this build.");
                println!("Please refer to the way you installed this software to determine how to update it.");
            }
        }
        ChmmArgs::HelpSyntax { _a } => help_syntax(),
        ChmmArgs::HelpTransforms { _a } => transforms::Transform::help(),
    }
    Ok(())
}

fn help_syntax() {
    println!("TODO!");
}
