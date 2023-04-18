//! This is not a stable API, and is to be used internally by the binary and
//! the integration tests only.

use std::{
    fs::File,
    io::{Read, Write},
};

pub use add::Style;
pub use arguments::parse_args;
pub use arguments::ChmmArgs;

mod add;
mod arguments;
mod config;
mod update;

use ini_merge::merge_ini;

/// Main function, amenable to integration tests.
pub fn inner_main(
    opts: ChmmArgs,
    stdin: &mut impl Read,
    stdout: &mut impl Write,
) -> anyhow::Result<()> {
    match opts {
        arguments::ChmmArgs::Process(file_name) => {
            let mut f = File::open(&file_name)?;
            let mut buf = String::new();
            f.read_to_string(&mut buf)?;
            let c = config::parse(&buf)?;

            let mut src_file = File::open(c.source_path(&file_name))?;
            let merged = merge_ini(stdin, &mut src_file, &c.mutations)?;
            for line in merged {
                writeln!(stdout, "{line}")?;
            }
        }
        arguments::ChmmArgs::Add { _a, files, style } => {
            for file in files {
                println!("Adding {file:?}");
                add::add(add::Mode::Normal, style, &file)?;
            }
        }
        arguments::ChmmArgs::Smart { _a, files, style } => {
            for file in files {
                println!("Adding {file:?}");
                add::add(add::Mode::Smart, style, &file)?;
            }
        }
        #[cfg(feature = "updater")]
        arguments::ChmmArgs::Update { _a } => {
            update::update()?;
        }
    }
    Ok(())
}
