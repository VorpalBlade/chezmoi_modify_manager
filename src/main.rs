use chezmoi_modify_manager::{inner_main, parse_args};
use env_logger::{Builder, Env};
use std::io::Write;
use std::io::{stdin, stdout};

fn main() -> anyhow::Result<()> {
    let mut builder = Builder::from_env(Env::default().default_filter_or("warn"));
    builder.format(|buf, record| {
        writeln!(
            buf,
            "[chezmoi_modify_manager: {}] {}",
            record.level(),
            record.args()
        )
    });
    builder.init();

    let opts = parse_args();
    inner_main(opts, || stdin().lock(), || stdout().lock())
}
