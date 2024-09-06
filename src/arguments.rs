//! Command line argument parser

// Doc comments are used to generate --help, not to for rustdoc.
#![allow(clippy::doc_markdown)]

use crate::add::Style;
use bpaf::short;
use bpaf::Bpaf;
use bpaf::Parser;
use bpaf::ShellComp;
use camino::Utf8PathBuf;
use itertools::Itertools;
use strum::EnumMessage;
use strum::IntoEnumIterator;

/// Parser for `--style`
fn style() -> impl Parser<Style> {
    const DEFAULT: Style = Style::Auto;
    let iter = Style::iter().map(|x| -> String {
        if x == DEFAULT {
            format!("{x} (default)")
        } else {
            x.to_string()
        }
    });
    let help_msg = format!(
        "Style of newly generated modify script [{}]",
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
        .help(help_msg.as_str())
        .argument::<String>("STYLE")
        .complete(complete_fn)
        .parse(|x| x.parse())
        .fallback(DEFAULT)
}

/// Add-on for chezmoi to handle mixed settings and state
#[derive(Debug, Bpaf)]
#[bpaf(options, version)]
pub enum ChmmArgs {
    /// Process a single file (containing settings).
    Process(#[bpaf(positional("FILE"), complete_shell(ShellComp::File{mask: None}))] Utf8PathBuf),
    Add {
        /// Add a file to be tracked by chezmoi_modify_manager
        #[bpaf(short('a'), long("add"))]
        _a: (),
        #[bpaf(external)]
        style: Style,
        #[bpaf(positional("FILE"), complete_shell(ShellComp::File{mask: None}))]
        files: Vec<Utf8PathBuf>,
    },
    Smart {
        /// Smartly add a file to be tracked by either chezmoi or
        /// chezmoi_modify_manager
        #[bpaf(short('s'), long("smart-add"))]
        _a: (),
        #[bpaf(positional("FILE"), complete_shell(ShellComp::File{mask: None}))]
        files: Vec<Utf8PathBuf>,
    },
    HelpSyntax {
        /// Print help about the config file syntax
        #[bpaf(long("help-syntax"))]
        _a: (),
    },
    HelpTransforms {
        /// Print help about supported transforms
        #[bpaf(long("help-transforms"))]
        _a: (),
    },
    Doctor {
        /// Perform environment sanity check
        #[bpaf(long("doctor"))]
        _a: (),
    },
    Update {
        /// Perform self update
        #[bpaf(short('u'), long("upgrade"))]
        _a: (),
        /// Do not ask for confirmation before applying updates
        #[bpaf(long("no-confirm"))]
        no_confirm: bool,
    },
    #[cfg(feature = "keyring")]
    KeyringSet {
        /// Store a password in the system keyring (password is read from stdin)
        #[bpaf(long("keyring-set"))]
        _a: (),
        /// Service for keyring command
        #[bpaf(positional("SERVICE"))]
        service: String,
        /// User name for keyring command
        #[bpaf(positional("USERNAME"))]
        username: String,
    },
    #[cfg(feature = "keyring")]
    KeyringRemove {
        /// Remove a password from the system keyring
        #[bpaf(long("keyring-remove"))]
        _a: (),
        /// Service for keyring command
        #[bpaf(positional("SERVICE"))]
        service: String,
        /// User name for keyring command
        #[bpaf(positional("USERNAME"))]
        username: String,
    },
}

/// Construct bpaf --help footer
fn footer() -> bpaf::Doc {
    // Leading spaces forces newlines to be inserted in bpaf documentation
    let mut doc = bpaf::Doc::default();
    doc.text("The --style flag controls how the script that --add generates looks:\n \n");
    for style in Style::iter() {
        let mut text = format!(
            " * {}: {}",
            style,
            style.get_documentation().expect("Cannot happen")
        );
        if !text.ends_with('\n') {
            text.push('\n');
        }
        doc.text(&text);
    }
    doc
}

/// Apply arg parser to standard arguments
#[must_use]
pub fn parse_args() -> ChmmArgs {
    chmm_args().footer(footer()).run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_options() {
        chmm_args().check_invariants(false);
    }
}
