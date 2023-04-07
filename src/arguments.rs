//! Command line argument parser
use std::path::PathBuf;

use crate::add::Style;
use bpaf::short;
use bpaf::Bpaf;
use bpaf::Parser;
use bpaf::ShellComp;
use itertools::Itertools;
use strum::EnumMessage;
use strum::IntoEnumIterator;

fn style() -> impl Parser<Style> {
    const DEFAULT: Style = Style::Normal;
    let iter = Style::iter().map(|x| -> String {
        if x == DEFAULT {
            format!("{} (default)", x)
        } else {
            x.to_string()
        }
    });
    let help_msg = format!(
        "How to call the modify manager in the generated file [{}]",
        Itertools::intersperse(iter, ", ".to_string()).collect::<String>()
    );

    fn complete_fn(input: &String) -> Vec<(&'static str, Option<&'static str>)> {
        Style::iter()
            .map(|x| {
                (
                    <Style as Into<&'static str>>::into(x),
                    x.get_documentation(),
                )
            })
            .filter(|(name, _)| name.starts_with(input))
            .collect()
    }

    short('t')
        .long("style")
        .help(help_msg)
        .argument::<String>("STYLE")
        .complete(complete_fn)
        .parse(|x| x.parse())
        .fallback(DEFAULT)
}

/// Arg parser
#[derive(Debug, Bpaf)]
#[bpaf(options, version)]
pub(super) enum Args {
    /// Process a single file (containing settings).
    Process(#[bpaf(positional("FILE"), complete_shell(ShellComp::File{mask: None}))] PathBuf),
    Add {
        /// Add a file to be tracked by chezmoi_mm
        #[bpaf(short('a'), long("add"))]
        _a: (),
        #[bpaf(external)]
        style: Style,
        #[bpaf(positional("FILE"), complete_shell(ShellComp::File{mask: None}))]
        files: Vec<PathBuf>,
    },
    Smart {
        /// Smartly add a file to be tracked by either chezmoi or chezmoi_mm
        #[bpaf(short('s'), long("smart-add"))]
        _a: (),
        #[bpaf(external)]
        style: Style,
        #[bpaf(positional("FILE"), complete_shell(ShellComp::File{mask: None}))]
        files: Vec<PathBuf>,
    },
    #[cfg(feature = "updater")]
    Update {
        /// Perform self update
        #[bpaf(long("upgrade"))]
        _a: (),
    },
}

/// Apply arg parser to standard arguments
pub(super) fn parse_args() -> Args {
    args().run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_options() {
        args().check_invariants(false)
    }
}
