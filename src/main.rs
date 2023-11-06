use anstyle::AnsiColor;
use chezmoi_modify_manager::{inner_main, parse_args};
use env_logger::{Builder, Env};
use log::Level;
use std::io::Write;
use std::io::{stdin, stdout};

fn main() -> anyhow::Result<()> {
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

    let opts = parse_args();
    inner_main(opts, || stdin().lock(), || stdout().lock())
}
