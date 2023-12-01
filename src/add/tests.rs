//! Two things are tested here:
//! * New style filtering
//! * Overall basic successful adding
//! 
//! ## Full tests for adding
//! 
//! There are several scenarios for adding files, all of these are tested
//! separately.
//!
//! | Previous source state | Command | Expected state                 |
//! | --------------------- | ------- | ------------------------------ |
//! | Missing               | Normal  | chezmoi add and convert/create |
//! | Missing               | Smart   | chezmoi add                    |
//! | Existing (basic)      | Normal  | chezmoi add and convert/create |
//! | Existing (basic)      | Smart   | chezmoi add                    |
//! | Existing (modify_)    | Normal  | Update data file               |
//! | Existing (modify_)    | Smart   | Update data file               |

use camino::{Utf8Path, Utf8PathBuf};
use indoc::indoc;
use pathdiff::diff_utf8_paths;
use pretty_assertions::assert_eq;
use tempfile::{tempdir, TempDir};

use crate::utils::Chezmoi;

use super::internal_filter;

#[derive(Debug)]
struct FilterTest {
    cfg: &'static str,
    input: &'static str,
    expected: &'static str,
}

const FILTER_TESTS: &[FilterTest] = &[
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

/// Very simple dummy chezmoi implementation
#[derive(Debug)]
struct DummyChezmoi {
    // tmp_dir is a RAII guard
    #[allow(dead_code)]
    tmp_dir: TempDir,
    input_dir: Utf8PathBuf,
    src_dir: Utf8PathBuf,
    dummy_file: Utf8PathBuf,
}

impl DummyChezmoi {
    fn new() -> Self {
        let tmp_dir = tempdir().unwrap();
        let input_dir: Utf8PathBuf = tmp_dir.path().join("input").try_into().unwrap();
        let src_dir: Utf8PathBuf = tmp_dir.path().join("source").try_into().unwrap();
        let dummy_file: Utf8PathBuf = input_dir.join("dummy_file").try_into().unwrap();
        std::fs::create_dir(input_dir.as_path()).unwrap();
        std::fs::create_dir(src_dir.as_path()).unwrap();
        std::fs::write(dummy_file.as_path(), "[a]\nb=c").unwrap();
        Self {
            tmp_dir,
            input_dir,
            src_dir,
            dummy_file,
        }
    }

    fn basic_source_path(&self, path: &Utf8Path) -> Utf8PathBuf {
        let rel_path = diff_utf8_paths(path, self.input_dir.as_path()).unwrap();
        self.src_dir.join(rel_path)
    }
}

impl Chezmoi for DummyChezmoi {
    fn source_path(&self, path: &Utf8Path) -> anyhow::Result<Option<Utf8PathBuf>> {
        let normal_path = self.basic_source_path(path);
        let script_path =
            normal_path.with_file_name(format!("modify_{}.tmpl", normal_path.file_name().unwrap()));
        if script_path.exists() {
            Ok(Some(script_path))
        } else if normal_path.exists() {
            Ok(Some(normal_path))
        } else {
            Ok(None)
        }
    }

    fn source_root(&self) -> anyhow::Result<Option<Utf8PathBuf>> {
        Ok(Some(self.src_dir.clone()))
    }

    fn add(&self, path: &Utf8Path) -> anyhow::Result<()> {
        let expected_path = self.basic_source_path(path);
        std::fs::copy(path, expected_path).unwrap();
        Ok(())
    }
}

fn assert_default_script(chezmoi: &DummyChezmoi) {
    let file_data = std::fs::read(chezmoi.src_dir.join("dummy_file.src.ini")).unwrap();
    assert_eq!(file_data.strip_suffix(b"\n").unwrap(), b"[a]\nb=c");

    let file_data = std::fs::read(chezmoi.src_dir.join("modify_dummy_file.tmpl")).unwrap();
    let file_data = String::from_utf8(file_data).unwrap();
    assert!(file_data.starts_with("#!/usr/bin/env chezmoi_modify_manager\n"));

    // No dummy basic file should exist
    assert!(!chezmoi.src_dir.join("dummy_file").try_exists().unwrap());
}

fn assert_unchanged_script(chezmoi: &DummyChezmoi) {
    let file_data = std::fs::read(chezmoi.src_dir.join("dummy_file.src.ini")).unwrap();
    assert_eq!(file_data.strip_suffix(b"\n").unwrap(), b"[a]\nb=c");

    let file_data = std::fs::read(chezmoi.src_dir.join("modify_dummy_file.tmpl")).unwrap();
    let file_data = String::from_utf8(file_data).unwrap();
    assert!(file_data.starts_with("#!/usr/bin/env chezmoi_modify_manager\n#UNTOUCHED\nsource auto"));

    // No dummy basic file should exist
    assert!(!chezmoi.src_dir.join("dummy_file").try_exists().unwrap());
}

fn assert_default_basic(chezmoi: &DummyChezmoi) {
    let file_data = std::fs::read(chezmoi.src_dir.join("dummy_file")).unwrap();
    assert_eq!(file_data, b"[a]\nb=c");

    // No modify script should exist
    assert!(!chezmoi
        .src_dir
        .join("dummy_file.src.ini")
        .try_exists()
        .unwrap());
    assert!(!chezmoi
        .src_dir
        .join("modify_dummy_file.tmpl")
        .try_exists()
        .unwrap());
}

#[test]
fn check_add_normal_missing() {
    let chezmoi = DummyChezmoi::new();
    let mut stdout: Vec<u8> = vec![];

    super::add(
        &chezmoi,
        super::Mode::Normal,
        super::Style::InPath,
        chezmoi.dummy_file.as_path(),
        &mut stdout,
    )
    .unwrap();

    assert_default_script(&chezmoi);
}

#[test]
fn check_add_smart_missing() {
    let chezmoi = DummyChezmoi::new();
    let mut stdout: Vec<u8> = vec![];

    super::add(
        &chezmoi,
        super::Mode::Smart,
        super::Style::InPath,
        chezmoi.dummy_file.as_path(),
        &mut stdout,
    )
    .unwrap();

    assert_default_basic(&chezmoi);
}

#[test]
fn check_add_normal_basic() {
    let chezmoi = DummyChezmoi::new();
    let mut stdout: Vec<u8> = vec![];

    std::fs::write(chezmoi.src_dir.join("dummy_file"), "old_contents").unwrap();

    super::add(
        &chezmoi,
        super::Mode::Normal,
        super::Style::InPath,
        chezmoi.dummy_file.as_path(),
        &mut stdout,
    )
    .unwrap();

    assert_default_script(&chezmoi);
}

#[test]
fn check_add_smart_basic() {
    let chezmoi = DummyChezmoi::new();
    let mut stdout: Vec<u8> = vec![];

    std::fs::write(chezmoi.src_dir.join("dummy_file"), "old_contents").unwrap();

    super::add(
        &chezmoi,
        super::Mode::Smart,
        super::Style::InPath,
        chezmoi.dummy_file.as_path(),
        &mut stdout,
    )
    .unwrap();

    assert_default_basic(&chezmoi);
}

#[test]
fn check_add_normal_script() {
    let chezmoi = DummyChezmoi::new();
    let mut stdout: Vec<u8> = vec![];

    std::fs::write(chezmoi.src_dir.join("dummy_file.src.ini"), "old_contents").unwrap();
    std::fs::write(
        chezmoi.src_dir.join("modify_dummy_file.tmpl"),
        "#!/usr/bin/env chezmoi_modify_manager\n#UNTOUCHED\nsource auto",
    )
    .unwrap();

    super::add(
        &chezmoi,
        super::Mode::Normal,
        super::Style::InPath,
        chezmoi.dummy_file.as_path(),
        &mut stdout,
    )
    .unwrap();

    assert_unchanged_script(&chezmoi);
}

#[test]
fn check_add_smart_script() {
    let chezmoi = DummyChezmoi::new();
    let mut stdout: Vec<u8> = vec![];

    std::fs::write(chezmoi.src_dir.join("dummy_file.src.ini"), "old_contents").unwrap();
    std::fs::write(
        chezmoi.src_dir.join("modify_dummy_file.tmpl"),
        "#!/usr/bin/env chezmoi_modify_manager\n#UNTOUCHED\nsource auto",
    )
    .unwrap();

    super::add(
        &chezmoi,
        super::Mode::Smart,
        super::Style::InPath,
        chezmoi.dummy_file.as_path(),
        &mut stdout,
    )
    .unwrap();

    assert_unchanged_script(&chezmoi);
}
