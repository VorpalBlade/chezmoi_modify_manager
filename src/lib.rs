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

use indoc::printdoc;
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
            #[cfg(feature = "self_update")]
            {
                update::update()?;
            }
            #[cfg(not(feature = "self_update"))]
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

/// Print help for the overall syntax of the configuration language.
fn help_syntax() {
    printdoc! {r#"
    Configuration files
    ===================

    chezmoi_modify_manager uses basic configuration files to control how to
    merge INI files. The easiest way to get started is to use -a to add a
    file and generate a skeleton configuration file.

    Syntax
    ======

    The file consists of directives, one per line. Comments are supported
    by prefixing a line with #. Comments are only supported at the start
    of lines.

    Directives
    ==========

    source
    ------
    This directive is required. It specifies where to find the source file
    (i.e. the file in the dotfile repo). It should have the following format:

    {}

    ignore
    ------
    Ignore a certain line, always taking it from the target file (i.e. file
    in your home directory), instead of the source state. The following
    variants are supported:

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

    transform
    ---------
    Some specific situations need more complicated merging that a simple
    ignore. For those situations you can use transforms. Supported variants
    are:

    transform "section" "key" transform-name arg1="value" arg2="value" ...
    transform regex "section-regex.*" "key-regex.*" transform-name arg1="value" ...

    For example, to treat mykey in mysection as an unsorted comma separated
    list, you could use:

    transform "mysection" "mykey" unsorted-list separator=","

    The full list of supported transforms, and how to use them can be listed
    using --help-transforms.
    "#,
    r#"source "{{ .chezmoi.sourceDir }}/{{ .chezmoi.sourceFile | trimSuffix ".tmpl" | replace "modify_" "" }}.src.ini""#};
}
