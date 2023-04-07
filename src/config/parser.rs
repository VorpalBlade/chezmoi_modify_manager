use std::collections::HashMap;

use winnow::branch::alt;
use winnow::bytes::take_till0;
use winnow::bytes::take_till1;
use winnow::character::escaped_transform;
use winnow::character::space1;
use winnow::combinator::opt;
use winnow::error::VerboseError;
use winnow::multi::separated0;
use winnow::sequence::delimited;
use winnow::sequence::preceded;
use winnow::IResult;
use winnow::Parser;

pub(crate) type ErrTy<'a> = VerboseError<&'a str>;

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
pub(super) fn parse_config<'a>(i: &'a str) -> IResult<&str, Vec<Directive>, ErrTy<'a>> {
    let alternatives = (
        comment.context("comment"),
        source.context("source"),
        ignore.context("ignore"),
        transform.context("transform"),
        "".map(|_| Directive::WS).context("whitespace"), // Blank lines
    );
    (separated0(alt(alternatives), "\n"), opt("\n"))
        .map(|(val, _)| val)
        .parse_next(i)
}

/// A comment
fn comment<'a>(i: &'a str) -> IResult<&'a str, Directive, ErrTy<'a>> {
    ('#', take_till0("\n\r"))
        .void()
        .map(|_| Directive::WS)
        .parse_next(i)
}

/// A source statement
fn source<'a>(i: &'a str) -> IResult<&str, Directive, ErrTy<'a>> {
    (
        "source",
        alt((
            "auto".map(|_| Directive::SourceAuto),
            (space1, quoted_string).map(|(_, path)| Directive::Source(path)),
        )),
    )
        .map(|(_, result)| result)
        .parse_next(i)
}

/// An ignore statement
fn ignore<'a>(i: &'a str) -> IResult<&str, Directive, ErrTy<'a>> {
    ("ignore", space1, matcher)
        .map(|(_, _, pattern)| Directive::Ignore(pattern))
        .parse_next(i)
}

/// A transform statement
fn transform<'a>(i: &'a str) -> IResult<&str, Directive, ErrTy<'a>> {
    (
        "transform",
        space1,
        matcher_transform,
        space1,
        take_till1(" \n"),
        opt(preceded(space1, separated0(transform_arg, space1))),
    )
        .map(|(_, _, pattern, _, transform, args)| {
            Directive::Transform(pattern, transform.to_owned(), args.unwrap_or_default())
        })
        .parse_next(i)
}

/// One argument to a transformer on the form `arg="value"`
fn transform_arg<'a>(i: &'a str) -> IResult<&str, (String, String), ErrTy<'a>> {
    (take_till1([' ', '=']), '=', quoted_string)
        .map(|(key, _, value)| (key.to_owned(), value))
        .parse_next(i)
}

/// Matcher for a section
fn match_section<'a>(i: &'a str) -> IResult<&str, Matcher, ErrTy<'a>> {
    ("section", space1, quoted_string)
        .map(|(_, _, section)| Matcher::Section(section))
        .parse_next(i)
}

/// Matcher for a regex
fn match_regex<'a>(i: &'a str) -> IResult<&str, Matcher, ErrTy<'a>> {
    ("regex", space1, quoted_string, space1, quoted_string)
        .map(|(_, _, section, _, key)| Matcher::Regex(section, key))
        .parse_next(i)
}

/// Literal matcher
fn match_literal<'a>(i: &'a str) -> IResult<&str, Matcher, ErrTy<'a>> {
    (quoted_string, space1, quoted_string)
        .map(|(section, _, key)| Matcher::Literal(section, key))
        .parse_next(i)
}

/// All valid matchers
fn matcher<'a>(i: &'a str) -> IResult<&str, Matcher, ErrTy<'a>> {
    alt((match_section, match_regex, match_literal)).parse_next(i)
}

/// The valid matchers for a transformer
fn matcher_transform<'a>(i: &'a str) -> IResult<&str, Matcher, ErrTy<'a>> {
    alt((match_regex, match_literal)).parse_next(i)
}

/// Quoted string value
fn quoted_string<'a>(i: &'a str) -> IResult<&str, String, ErrTy<'a>> {
    delimited(
        '"',
        escaped_transform(
            take_till1("\"\\"),
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
        let (rem, out) = quoted_string("\"test \\\" \\\\input\"").unwrap();
        assert_eq!(rem, "");
        assert_eq!(out, "test \" \\input");

        let res = quoted_string("\"invalid");
        assert!(matches!(res, Err(_)));
    }

    #[test]
    fn check_matcher() {
        let (rem, out) = matcher("section \"my-section\"").unwrap();
        assert_eq!(rem, "");
        assert!(matches!(out, Matcher::Section(s) if s == "my-section"));

        let (rem, out) = matcher("\"my-section\" \"my-key\"").unwrap();
        assert_eq!(rem, "");
        assert!(matches!(out, Matcher::Literal(s, k) if s == "my-section" && k == "my-key"));

        let (rem, out) = matcher("regex \"my-section.*\" \"my-key.*\"").unwrap();
        assert_eq!(rem, "");
        assert!(matches!(out, Matcher::Regex(s, k) if s == "my-section.*" && k == "my-key.*"));
    }

    #[test]
    fn check_transform_arg() {
        let (rem, out) = transform_arg("aaa=\"bbb\"").unwrap();
        assert_eq!(rem, "");
        assert_eq!(out.0, "aaa");
        assert_eq!(out.1, "bbb");
    }

    #[test]
    fn check_transform() {
        // Test winnow parser
        let (rem, out) =
            transform("transform regex \"s.*\" \"k.*\" transform-name arg1=\"a\" arg2=\"b\"")
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
        let (rem, out) = transform("transform regex \"s.*\" \"k.*\" transform-name").unwrap();

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
    source "some/path"

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
                Directive::Source("some/path".into()),
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
}
