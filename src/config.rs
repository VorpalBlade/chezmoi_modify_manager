//! Describes configuration file format
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::anyhow;
use ini_merge::mutations::transforms;
use ini_merge::mutations::Action;
use ini_merge::mutations::Mutations;
use ini_merge::mutations::MutationsBuilder;
use winnow::Parser;

use crate::transforms::Transform;

use self::parser::Directive;
use self::parser::Matcher;

mod parser;

/// Where to find the source file
#[derive(Debug)]
pub(crate) enum Source {
    /// Specific path for the source file.
    Path(PathBuf),
    /// Auto locate the source file.
    ///
    /// This is currently broken with chezmoi, but needed for integration
    /// tests however.
    Auto,
}

/// The data from the config file
#[derive(Debug)]
pub(crate) struct Config {
    pub(crate) source: Source,
    pub(crate) mutations: Mutations,
}

impl Config {
    /// Compute the source path
    pub(crate) fn source_path(&self, script_path: &Path) -> anyhow::Result<Cow<'_, Path>> {
        match self.source {
            Source::Path(ref p) => Ok(Cow::Borrowed(p)),
            Source::Auto => {
                let script_name = script_path
                    .file_name()
                    .ok_or_else(|| anyhow!("Failed to extract filename from {script_path:?}"))?
                    .to_string_lossy();
                Ok(Cow::Owned(
                    script_path
                        .with_file_name(script_name.strip_prefix("modify_").unwrap_or(&script_name))
                        .with_extension("src.ini"),
                ))
            }
        }
    }
}

/// Create a transformer based on name
fn make_transformer(
    transform: &str,
    args: &HashMap<String, String>,
) -> anyhow::Result<Box<dyn transforms::Transformer>> {
    Ok(Transform::from_str(transform)
        .map_err(|err| anyhow!("Invalid transform specified: {}: {}", transform, err))?
        .construct(args)?)
}

/// Parse directives for operation
pub(crate) fn parse(src: &str) -> Result<Config, anyhow::Error> {
    let result = parser::parse_config
        .parse(src)
        .map_err(|e| anyhow::format_err!("{e}"))?;

    let mut source = None;
    let mut builder = MutationsBuilder::new();

    // Build config object
    for directive in result {
        match directive {
            Directive::WS => (),
            Directive::Source(src) => {
                if source.is_some() {
                    return Err(anyhow!("Duplicate source directives not allowed!"));
                }
                source = Some(Source::Path(src.into()));
            }
            Directive::SourceAuto => {
                if source.is_some() {
                    return Err(anyhow!("Duplicate source directives not allowed!"));
                }
                source = Some(Source::Auto);
            }
            Directive::Ignore(Matcher::Section(section)) => {
                builder = builder.add_ignore_section(section);
            }
            Directive::Ignore(Matcher::Literal(section, key)) => {
                builder = builder.add_literal_action(section, key, Action::Ignore);
            }
            Directive::Ignore(Matcher::Regex(section, key)) => {
                builder = builder.add_regex_action(section, key, Action::Ignore);
            }
            Directive::Transform(matcher, transform, args) => {
                let t = make_transformer(&transform, &args)?;
                match matcher {
                    Matcher::Section(_) => {
                        return Err(anyhow!("Section match is not valid for transforms"));
                    }
                    Matcher::Literal(section, key) => {
                        builder = builder.add_literal_action(section, key, Action::Transform(t));
                    }
                    Matcher::Regex(section, key) => {
                        builder = builder.add_regex_action(section, key, Action::Transform(t));
                    }
                }
            }
        }
    }

    Ok(Config {
        source: source.ok_or(anyhow!("No source directive found"))?,
        mutations: builder.build()?,
    })
}
