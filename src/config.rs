//! Describes configuration file format
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Debug;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;
use std::str::FromStr;

use anyhow::anyhow;
use ini_merge::filter::FilterAction;
use ini_merge::filter::FilterActions;
use ini_merge::filter::FilterActionsBuilder;
use ini_merge::mutations::transforms;
use ini_merge::mutations::Action;
use ini_merge::mutations::Mutations;
use ini_merge::mutations::MutationsBuilder;
use ini_merge::mutations::SectionAction;
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
pub(crate) struct Config<ActionType>
where
    ActionType: Debug,
{
    pub(crate) source: Source,
    pub(crate) mutations: ActionType,
}

impl<ActionType> Config<ActionType>
where
    ActionType: Debug,
{
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
) -> anyhow::Result<Rc<dyn transforms::Transformer>> {
    Ok(Transform::from_str(transform)
        .map_err(|err| anyhow!("Invalid transform specified: {}: {}", transform, err))?
        .construct(args)?)
}

/// Parse directives for operation
pub(crate) fn parse_for_merge(src: &str) -> Result<Config<Mutations>, anyhow::Error> {
    let result = parser::parse_config
        .parse(src)
        .map_err(|e| anyhow::format_err!("{e}"))?;

    let mut source = None;
    let mut builder = MutationsBuilder::new();

    // Build config object
    for directive in result {
        match directive {
            Directive::WS => (),
            // Not relevant for merging
            Directive::AddRemove(_) | Directive::AddHide(_) => (),
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
                builder.add_section_action(section, SectionAction::Ignore);
            }
            Directive::Ignore(matcher) => {
                add_merge_action(&mut builder, matcher, Action::Ignore);
            }
            Directive::Transform(matcher, transform, args) => {
                let t = make_transformer(&transform, &args)?;
                add_merge_action(&mut builder, matcher, Action::Transform(t));
            }
            Directive::Set {
                section,
                key,
                value,
                separator,
            } => {
                // Set is a transform under the hood, but needs special support
                // to enable adding lines that don't exist. This is handled inside
                // the mutations builder.
                builder.add_setter(
                    section,
                    key,
                    value,
                    separator.unwrap_or_else(|| " = ".to_string()),
                );
            }
            Directive::Remove(Matcher::Section(section)) => {
                builder.add_section_action(section, SectionAction::Delete);
            }
            Directive::Remove(matcher) => {
                add_merge_action(&mut builder, matcher, Action::Delete);
            }
        }
    }

    Ok(Config {
        source: source.ok_or(anyhow!("No source directive found"))?,
        mutations: builder.build()?,
    })
}

/// Parse directives for operation
pub(crate) fn parse_for_add(src: &str) -> Result<Config<FilterActions>, anyhow::Error> {
    let result = parser::parse_config
        .parse(src)
        .map_err(|e| anyhow::format_err!("{e}"))?;

    let mut source = None;
    let mut builder = FilterActionsBuilder::new();

    // Build config object
    for directive in result {
        match directive {
            Directive::WS => (),
            Directive::AddHide(matcher) => {
                add_filter_action(&mut builder, matcher, FilterAction::Replace("HIDDEN"));
            }
            Directive::AddRemove(matcher) | Directive::Ignore(matcher) => {
                add_filter_action(&mut builder, matcher, FilterAction::Remove);
            }
            // Common
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
            // Not relevant for filtering
            Directive::Set { .. } => (),
            Directive::Transform(_, _, _) => (),
            Directive::Remove(_) => (),
        }
    }

    Ok(Config {
        source: source.ok_or(anyhow!("No source directive found"))?,
        mutations: builder.build()?,
    })
}

fn add_merge_action(builder: &mut MutationsBuilder, matcher: Matcher, action: Action) {
    match matcher {
        Matcher::Section(_) => panic!("Section match not valid in add_merge_action()"),
        Matcher::Literal(section, key) => builder.add_literal_action(section, key, action),
        Matcher::Regex(section, key) => builder.add_regex_action(section, key, action),
    }
}

fn add_filter_action(builder: &mut FilterActionsBuilder, matcher: Matcher, action: FilterAction) {
    match matcher {
        Matcher::Section(section) => builder.add_section_action(section, action),
        Matcher::Literal(section, key) => builder.add_literal_action(section, key, action),
        Matcher::Regex(section, key) => builder.add_regex_action(section, key, action),
    }
}
