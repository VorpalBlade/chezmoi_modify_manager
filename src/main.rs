#![warn(unreachable_pub)]
#![warn(clippy::doc_markdown)]
#![warn(clippy::needless_pass_by_value)]
#![warn(clippy::redundant_closure_for_method_calls)]
#![warn(clippy::semicolon_if_nothing_returned)]
#![warn(clippy::wildcard_imports)]

use anstyle::AnsiColor;
use chezmoi_modify_manager::inner_main;
use chezmoi_modify_manager::parse_args;
use env_logger::Builder;
use env_logger::Env;
use log::Level;
use std::io::stdin;
use std::io::stdout;
use std::io::BufWriter;
use std::io::Write;

fn main() -> anyhow::Result<()> {
    // Set up logging
    let mut builder = Builder::from_env(Env::default().default_filter_or("warn"));
    builder.target(env_logger::Target::Pipe(Box::new(anstream::stderr())));
    builder.format(|buf, record| {
        let colour = match record.level() {
            Level::Error => AnsiColor::Red,
            Level::Warn => AnsiColor::Yellow,
            Level::Info => AnsiColor::Green,
            Level::Debug => AnsiColor::Blue,
            Level::Trace => AnsiColor::Cyan,
        }
        .on_default();
        writeln!(
            buf,
            "[chezmoi_modify_manager {}{}{}] {}",
            colour.render(),
            record.level(),
            colour.render_reset(),
            record.args()
        )
    });
    builder.init();

    // Run the program proper
    let opts = parse_args();
    // Use BufWriter, we don't need to flush on every newline
    inner_main(opts, || stdin().lock(), || BufWriter::new(stdout().lock()))
}
