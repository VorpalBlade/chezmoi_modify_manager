//! Defines the winnow parser for the config file format.
use std::collections::HashMap;

use winnow::ascii::escaped_transform;
use winnow::ascii::space1;
use winnow::combinator::alt;
use winnow::combinator::delimited;
use winnow::combinator::opt;
use winnow::combinator::preceded;
use winnow::combinator::separated0;
use winnow::error::StrContext;
use winnow::prelude::*;
use winnow::token::take_till0;
use winnow::token::take_till1;
use winnow::Parser;

/// A directive in the config file
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Directive {
    /// Whitespace, ignore
    WS,
    /// A source path
    Source(String),
    /// Automatic source localisation
    SourceAuto,
    /// An ignore directive
    Ignore(Matcher),
    /// A transform directive
    Transform(Matcher, String, HashMap<String, String>),
    /// Set an section, key to a specific value
    Set {
        section: String,
        key: String,
        value: String,
        separator: Option<String>,
    },
    /// Remove everything matching a specific matcher
    Remove(Matcher),
}

/// The different ways things can be matched.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Matcher {
    /// Match a whole section (exact name)
    Section(String),
    /// Match exact section and key names
    Literal(String, String),
    /// Match section and key names using regexes
    Regex(String, String),
}

/// Top level parser for the config file
pub(super) fn parse_config(i: &mut &str) -> PResult<Vec<Directive>> {
    let alternatives = (
        comment.context(StrContext::Label("comment")),
        source.context(StrContext::Label("source")),
        ignore.context(StrContext::Label("ignore")),
        transform.context(StrContext::Label("transform")),
        set.context(StrContext::Label("set")),
        remove.context(StrContext::Label("remove")),
        "".map(|_| Directive::WS)
            .context(StrContext::Label("whitespace")), // Blank lines
    );
    (separated0(alt(alternatives), newline), opt(newline))
        .map(|(val, _)| val)
        .parse_next(i)
}

/// A newline (LF, CR or CRLF)
fn newline(i: &mut &str) -> PResult<()> {
    alt(("\r\n", "\n", "\r")).void().parse_next(i)
}

/// A comment
fn comment(i: &mut &str) -> PResult<Directive> {
    ('#', take_till0(['\n', '\r']))
        .void()
        .map(|_| Directive::WS)
        .parse_next(i)
}

/// A source statement
fn source(i: &mut &str) -> PResult<Directive> {
    (
        "source",
        space1,
        alt((
            "auto".map(|_| Directive::SourceAuto),
            quoted_string.map(Directive::Source),
        )),
    )
        .map(|(_, _, result)| result)
        .parse_next(i)
}

/// An ignore statement
fn ignore(i: &mut &str) -> PResult<Directive> {
    ("ignore", space1, matcher)
        .map(|(_, _, pattern)| Directive::Ignore(pattern))
        .parse_next(i)
}

fn set(i: &mut &str) -> PResult<Directive> {
    (
        "set",
        space1,
        quoted_string,
        space1,
        quoted_string,
        space1,
        quoted_string,
        opt((space1, "separator=", quoted_string).map(|(_, _, v)| v)),
    )
        .map(
            |(_, _, section, _, key, _, value, separator)| Directive::Set {
                section,
                key,
                value,
                separator,
            },
        )
        .parse_next(i)
}

fn remove(i: &mut &str) -> PResult<Directive> {
    ("remove", space1, matcher)
        .map(|(_, _, matcher)| Directive::Remove(matcher))
        .parse_next(i)
}

/// A transform statement
fn transform(i: &mut &str) -> PResult<Directive> {
    (
        "transform",
        space1,
        matcher_transform,
        space1,
        take_till1([' ', '\r', '\n']),
        opt(preceded(space1, separated0(transform_arg, space1))),
    )
        .map(|(_, _, pattern, _, transform, args)| {
            Directive::Transform(pattern, transform.to_owned(), args.unwrap_or_default())
        })
        .parse_next(i)
}

/// One argument to a transformer on the form `arg="value"`
fn transform_arg(i: &mut &str) -> PResult<(String, String)> {
    (take_till1([' ', '=']), '=', quoted_string)
        .map(|(key, _, value)| (key.to_owned(), value))
        .parse_next(i)
}

/// Matcher for a section
fn match_section(i: &mut &str) -> PResult<Matcher> {
    ("section", space1, quoted_string)
        .map(|(_, _, section)| Matcher::Section(section))
        .parse_next(i)
}

/// Matcher for a regex
fn match_regex(i: &mut &str) -> PResult<Matcher> {
    ("regex", space1, quoted_string, space1, quoted_string)
        .map(|(_, _, section, _, key)| Matcher::Regex(section, key))
        .parse_next(i)
}

/// Literal matcher
fn match_literal(i: &mut &str) -> PResult<Matcher> {
    (quoted_string, space1, quoted_string)
        .map(|(section, _, key)| Matcher::Literal(section, key))
        .parse_next(i)
}

/// All valid matchers
fn matcher(i: &mut &str) -> PResult<Matcher> {
    alt((match_section, match_regex, match_literal)).parse_next(i)
}

/// The valid matchers for a transformer
fn matcher_transform(i: &mut &str) -> PResult<Matcher> {
    alt((match_regex, match_literal)).parse_next(i)
}

/// Quoted string value
fn quoted_string(i: &mut &str) -> PResult<String> {
    delimited(
        '"',
        escaped_transform(
            take_till1(['"', '\\']),
            '\\',
            alt(("\\".value("\\"), "\"".value("\""), "n".value("\n"))),
        ),
        '"',
    )
    .parse_next(i)
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::*;

    #[test]
    fn check_quoted_string() {
        let (rem, out) = quoted_string.parse_peek("\"test \\\" \\\\input\"").unwrap();
        assert_eq!(rem, "");
        assert_eq!(out, "test \" \\input");

        let res = quoted_string.parse_peek("\"invalid");
        assert!(res.is_err());
    }

    #[test]
    fn check_matcher() {
        let (rem, out) = matcher.parse_peek("section \"my-section\"").unwrap();
        assert_eq!(rem, "");
        assert!(matches!(out, Matcher::Section(s) if s == "my-section"));

        let (rem, out) = matcher.parse_peek("\"my-section\" \"my-key\"").unwrap();
        assert_eq!(rem, "");
        assert!(matches!(out, Matcher::Literal(s, k) if s == "my-section" && k == "my-key"));

        let (rem, out) = matcher
            .parse_peek("regex \"my-section.*\" \"my-key.*\"")
            .unwrap();
        assert_eq!(rem, "");
        assert!(matches!(out, Matcher::Regex(s, k) if s == "my-section.*" && k == "my-key.*"));
    }

    #[test]
    fn check_transform_arg() {
        let (rem, out) = transform_arg.parse_peek("aaa=\"bbb\"").unwrap();
        assert_eq!(rem, "");
        assert_eq!(out.0, "aaa");
        assert_eq!(out.1, "bbb");
    }

    #[test]
    fn check_transform() {
        // Test winnow parser
        let (rem, out) = transform
            .parse_peek("transform regex \"s.*\" \"k.*\" transform-name arg1=\"a\" arg2=\"b\"")
            .unwrap();

        assert_eq!(rem, "");
        assert_eq!(
            out,
            Directive::Transform(
                Matcher::Regex("s.*".into(), "k.*".into()),
                "transform-name".into(),
                HashMap::from([("arg1".into(), "a".into()), ("arg2".into(), "b".into())]),
            )
        );
    }

    #[test]
    fn check_transform_no_args() {
        // Test winnow parser
        let (rem, out) = transform
            .parse_peek("transform regex \"s.*\" \"k.*\" transform-name")
            .unwrap();

        assert_eq!(rem, "");
        assert_eq!(
            out,
            Directive::Transform(
                Matcher::Regex("s.*".into(), "k.*".into()),
                "transform-name".into(),
                HashMap::new(),
            )
        );
    }

    const FULL_EXAMPLE: &str = indoc! {r#"
    #!/path
    source auto

    ignore section "c"
    ignore "a" "b"
    transform "d" "e" unsorted-list separator=","
    transform "f g" "h" keyring service="srv" user="usr"
    transform "a" "b" kde-shortcut

    ignore regex "a.*" "b.*"
    transform regex "d.*" "e.*" kde-shortcut
    # Random comment

    "#};

    #[test]
    fn test_parse() {
        let out = parse_config.parse(FULL_EXAMPLE).unwrap();

        // Get rid of whitespace, we don't care about those
        let out: Vec<_> = out.into_iter().filter(|v| *v != Directive::WS).collect();

        assert_eq!(
            out,
            vec![
                Directive::SourceAuto,
                Directive::Ignore(Matcher::Section("c".into())),
                Directive::Ignore(Matcher::Literal("a".into(), "b".into())),
                Directive::Transform(
                    Matcher::Literal("d".into(), "e".into()),
                    "unsorted-list".into(),
                    HashMap::from([("separator".into(), ",".into())])
                ),
                Directive::Transform(
                    Matcher::Literal("f g".into(), "h".into()),
                    "keyring".into(),
                    HashMap::from([
                        ("service".into(), "srv".into()),
                        ("user".into(), "usr".into())
                    ])
                ),
                Directive::Transform(
                    Matcher::Literal("a".into(), "b".into()),
                    "kde-shortcut".into(),
                    HashMap::new()
                ),
                Directive::Ignore(Matcher::Regex("a.*".into(), "b.*".into())),
                Directive::Transform(
                    Matcher::Regex("d.*".into(), "e.*".into()),
                    "kde-shortcut".into(),
                    HashMap::new()
                ),
            ]
        )
    }

    #[test]
    fn test_parse_newlines() {
        let out = parse_config.parse("source auto\rsource \"foo\"\r\nignore section \"bar\"\nignore section \"quux\"\r\n").unwrap();

        // Get rid of whitespace, we don't care about those
        let out: Vec<_> = out.into_iter().filter(|v| *v != Directive::WS).collect();

        assert_eq!(
            out,
            vec![
                Directive::SourceAuto,
                Directive::Source("foo".into()),
                Directive::Ignore(Matcher::Section("bar".into())),
                Directive::Ignore(Matcher::Section("quux".into()))
            ]
        )
    }
}
