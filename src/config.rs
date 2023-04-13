//! Describes configuration file format
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use anyhow::anyhow;
use ini_merge::mutations::transforms;
use ini_merge::mutations::Action;
use ini_merge::mutations::Mutations;
use ini_merge::mutations::MutationsBuilder;
use winnow::Parser;

use self::parser::Directive;
use self::parser::Matcher;

mod parser;

#[derive(Debug)]
pub(crate) enum Source {
    Path(PathBuf),
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
    pub(crate) fn source_path(&self, script_path: &Path) -> Cow<'_, Path> {
        match self.source {
            Source::Path(ref p) => Cow::Borrowed(p),
            Source::Auto => Cow::Owned(script_path.with_extension("src.ini")),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ConfigError {
    #[error("Invalid transform specified: {0}")]
    InvalidTransform(String),
    #[error("Failed to create transform due to {source}")]
    TransformerError {
        #[source]
        #[from]
        source: transforms::TransformerError,
    },
}

/// Create a transformer based on name
fn make_transformer(
    transform: &str,
    args: &HashMap<String, String>,
) -> Result<Box<dyn transforms::Transformer>, ConfigError> {
    use transforms::Transformer;

    match transform {
        "unsorted-list" => Ok(Box::new(
            transforms::TransformUnsortedLists::from_user_input(args)?,
        )),
        "kde-shortcut" => Ok(Box::new(transforms::TransformKdeShortcut::from_user_input(
            args,
        )?)),
        "keyring" => Ok(Box::new(transforms::TransformKeyring::from_user_input(
            args,
        )?)),
        _ => Err(ConfigError::InvalidTransform(transform.into())),
    }
}

/// Parse directives for operation
pub(crate) fn parse(src: &str) -> Result<Config, anyhow::Error> {
    let result = parser::parse_config
        .parse(src)
        .map_err(|e| e.into_owned())?;

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