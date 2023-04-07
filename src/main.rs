use std::{
    fs::File,
    io::{stdin, stdout, Read, Write},
};

use ini_merge::merge_ini;

mod add;
mod arguments;
mod config;

fn main() -> anyhow::Result<()> {
    let opts = arguments::parse_args();
    match opts {
        arguments::Args::Process(file_name) => {
            let mut f = File::open(&file_name)?;
            let mut buf = String::new();
            f.read_to_string(&mut buf)?;
            let c = config::parse(&buf)?;

            let mut src_file = File::open(c.source_path(&file_name))?;
            let mut tgt_file = stdin().lock();
            let merged = merge_ini(&mut tgt_file, &mut src_file, &c.mutations)?;
            let mut stdout = stdout().lock();
            for line in merged {
                writeln!(stdout, "{line}")?;
            }
        }
        arguments::Args::Add { _a, files, style } => {
            for file in files {
                println!("Adding {file:?}");
                add::add(add::Mode::Normal, style, &file)?;
            }
        }
        arguments::Args::Smart { _a, files, style } => {
            for file in files {
                println!("Adding {file:?}");
                add::add(add::Mode::Smart, style, &file)?;
            }
        }
        #[cfg(feature = "updater")]
        arguments::Args::Update { _a } => {
            println!("TODO: Self update");
            todo!()
        }
    }
    Ok(())
}
