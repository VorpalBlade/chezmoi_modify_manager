use std::io::{stdin, stdout};

use chezmoi_modify_manager::{inner_main, parse_args};

fn main() -> anyhow::Result<()> {
    let opts = parse_args();
    inner_main(opts, || stdin().lock(), || stdout().lock())
}
