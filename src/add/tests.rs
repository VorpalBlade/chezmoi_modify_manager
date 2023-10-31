use indoc::indoc;
use pretty_assertions::assert_eq;

use super::internal_filter;

#[derive(Debug)]
struct FilterTest {
    cfg: &'static str,
    input: &'static str,
    expected: &'static str,
}

const FILTER_TESTS: &'static [FilterTest] = &[
    FilterTest {
        cfg: indoc!(
            r#"
            source "{{ .chezmoi.sourceDir }}/{{ .chezmoi.sourceFile | trimSuffix ".tmpl" | replace "modify_" "" }}.src.ini"

            add:hide "a" "b"
            add:remove "a" "c"
            ignore "quux" "e"
            "#
        ),
        input: indoc!(
            r#"
            [a]
            b=foo
            c=bar
            d=quux

            [quux]
            e=f
            g=h
            "#
        ),
        expected: indoc!(
            r#"
            [a]
            b=HIDDEN
            d=quux

            [quux]
            g=h
            "#
        ),
    },
    FilterTest {
        cfg: indoc!(
            r#"
            source "{{ .chezmoi.sourceDir }}/{{ .chezmoi.sourceFile | trimSuffix ".tmpl" | replace "modify_" "" }}.src.ini"

            {{ if (chezmoi templating) }}
            set "a" "b" "c"
            {{ endif }}
            add:remove "a" "b"
            "#
        ),
        input: indoc!(
            r#"
            [a]
            b=c
            d=e
            "#
        ),
        expected: indoc!(
            r#"
            [a]
            d=e
            "#
        ),
    },
];

#[test]
fn check_filtering() {
    for test_case in FILTER_TESTS {
        let result = internal_filter(test_case.cfg, test_case.input.as_bytes());
        dbg!(&result);
        let result = result.unwrap();
        assert_eq!(
            String::from_utf8(result).unwrap().trim_end(),
            test_case.expected.trim_end()
        );
    }
}
