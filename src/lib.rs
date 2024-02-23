//! This is not a stable API, and is to be used internally by the binary and
//! the integration tests only.

#![warn(unreachable_pub)]
#![warn(clippy::doc_markdown)]
#![warn(clippy::needless_pass_by_value)]
#![warn(clippy::redundant_closure_for_method_calls)]
#![warn(clippy::semicolon_if_nothing_returned)]
#![warn(clippy::wildcard_imports)]

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
mod doctor;
mod transforms;
mod update;
mod utils;

use indoc::printdoc;
use ini_merge::merge::merge_ini;

use crate::utils::{RealChezmoi, CHEZMOI_AUTO_SOURCE_VERSION};

/// Main function, amenable to integration tests.
///
/// In order to support integration tests we need to be able to provide stdin
/// and capture stdout. It would be very nice if the non-test case could simply
/// call us with `stdin.lock()` and `stdout.lock()`. However, that breaks the
/// self-updater case, which uses stdio directly. Instead call with functions
/// that return stdio streams. Note! stderr is not captured, it is used for
/// logging.
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
            let buf = std::fs::read_to_string(&file_name)
                .with_context(|| format!("Failed to load {file_name}"))?;
            let c = config::parse_for_merge(&buf)
                .with_context(|| format!("Failed to parse {file_name}"))?;

            let mut stdin = stdin();
            let src_path = c
                .source_path(&file_name)
                .context("Failed to get source path")?;
            let mut src_file = File::open(src_path.as_std_path())
                .with_context(|| format!("Failed to open source file at: {src_path}"))?;
            let merged = merge_ini(&mut stdin, &mut src_file, &c.mutations)?;
            let mut stdout = stdout();
            for line in merged {
                writeln!(stdout, "{line}")?;
            }
        }
        ChmmArgs::Add { _a, files, style } => {
            let mut stdout = stdout();
            for file in files {
                add::add(
                    &RealChezmoi::default(),
                    add::Mode::Normal,
                    style,
                    &file,
                    &mut stdout,
                )?;
            }
        }
        ChmmArgs::Smart { _a, files } => {
            let mut stdout = stdout();
            for file in files {
                add::add(
                    &RealChezmoi::default(),
                    add::Mode::Smart,
                    Style::Auto, // Style unused for the smart case, so doesn't matter
                    &file,
                    &mut stdout,
                )?;
            }
        }
        #[cfg(feature = "updater-tls-rusttls")]
        ChmmArgs::Update { _a, no_confirm } => {
            update::update(no_confirm)?;
        }
        #[cfg(not(feature = "updater-tls-rusttls"))]
        ChmmArgs::Update { .. } => {
            println!("Support for the updater was not included in this build.");
            println!("Please refer to the way you installed this software to determine how to update it.");
            std::process::exit(1);
        }
        ChmmArgs::Doctor { _a } => doctor::doctor()?,
        ChmmArgs::HelpSyntax { _a } => help_syntax(),
        ChmmArgs::HelpTransforms { _a } => transforms::Transform::help(),
    }
    Ok(())
}

/// Print help for the overall syntax of the configuration language.
fn help_syntax() {
    printdoc! {r#"
    Configuration files
    ===================

    chezmoi_modify_manager uses basic configuration files to control how to
    merge INI files. The easiest way to get started is to use -a to add a file
    and generate a skeleton configuration file.

    Syntax
    ======

    The file consists of directives, one per line. Comments are supported by
    prefixing a line with #. Comments are only supported at the start of lines.

    Directives
    ==========

    source
    ------
    This directive is required. It specifies where to find the source file
    (i.e. the file in the dotfile repo). It should have the following format
    to support Chezmoi versions older than {}:

    {}

    From Chezmoi {} and forward the following also works instead:

    source auto

    ignore
    ------
    Ignore a certain line, always taking it from the target file (i.e. file in
    your home directory), instead of the source state. The following variants
    are supported:

    ignore section "my-section"
    ignore "my-section" "my-key"
    ignore regex "section.*regex" "key regex.*"

    The first form ignores a whole section (exact literal match).
    The second form ignores a specific key (exact literal match).
    The third form uses a regex to ignore a specific key.

    Prefer the exact literal match variants where possible, they will be
    marginally faster.

    An additional effect is that lines that are missing in the source state
    will not be deleted if they are ignored.

    Finally, ignored lines will not be added back when using --add or
    --smart-add, in order to reduce git diffs.

    set
    ---
    Set an entry to a specific value. This is primarily useful together with
    chezmoi templates, allowing you to override a specific value for only some
    of your computers. The following variants are supported:

    set "section" "key" "value"
    set "section" "key" "value" separator="="

    By default separator is " = ", which might not match what the program that
    the ini files belongs to uses.

    Notes:
    * Only exact literal matches are supported.
    * It works better if the line exists in the source & target state, otherwise
      it is likely the line will get formatted weirdly (which will often be
      changed by the program the INI file belongs to).

    remove
    ------
    Unconditionally remove everything matching the directive. This is primarily
    useful together with chezmoi templates, allowing you to remove a specific
    key or section for only some of your computers. The following variants are
    supported:

    remove section "my-section"
    remove "my-section" "my-key"
    remove regex "section.*regex" "key regex.*"

    (Matching works identically to ignore, see above for more details.)

    transform
    ---------
    Some specific situations need more complicated merging that a simple
    ignore. For those situations you can use transforms. Supported variants
    are:

    transform "section" "key" transform-name arg1="value" arg2="value" ...
    transform regex "section-regex.*" "key-regex.*" transform-name arg1="value" ...

    (Matching works identically to ignore except matching entire sections is
    not supported. See above for more details.)

    For example, to treat mykey in mysection as an unsorted comma separated
    list, you could use:

    transform "mysection" "mykey" unsorted-list separator=","

    The full list of supported transforms, and how to use them can be listed
    using --help-transforms.

    add:remove & add:hide
    ---------------------
    These two directives control the behaviour when using --add or --smart-add.
    In particular, these allow filtering lines that will be added back to the
    source state.

    add:remove will remove the matching lines entirely. The following forms are
    supported:

    add:remove section "section name"
    add:remove "section name" "key"
    add:remove regex  "section-regex.*" "key-regex.*"

    (Matching works identically to ignore, see above for more details.)

    add:hide will instead keep the entries but replace the value associated with
    those keys. This is useful together with the keyring transform in particular,
    as the key needs to exist in the source or target state for it to trigger
    the replacement. The following forms are supported:

    add:hide section "section name"
    add:hide "section name" "key"
    add:hide regex  "section-regex.*" "key-regex.*"

    (Matching works identically to ignore, see above for more details.)
    "#,
    CHEZMOI_AUTO_SOURCE_VERSION,
    r#"source "{{ .chezmoi.sourceDir }}/{{ .chezmoi.sourceFile | trimSuffix ".tmpl" | replace "modify_" "" }}.src.ini""#,
    CHEZMOI_AUTO_SOURCE_VERSION};
}
