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

use super::internal_filter;
use crate::Style;
use crate::utils::CHEZMOI_AUTO_SOURCE_VERSION;
use crate::utils::Chezmoi;
use crate::utils::ChezmoiVersion;
use camino::Utf8Path;
use camino::Utf8PathBuf;
use indoc::indoc;
use pathdiff::diff_utf8_paths;
use pretty_assertions::assert_eq;
use tempfile::TempDir;
use tempfile::tempdir;

const DUMMY_FILE_NAME_PREFIX: &str = "dummy_file";

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
            r"
            [a]
            b=foo
            c=bar
            d=quux

            [quux]
            e=f
            g=h
            "
        ),
        expected: indoc!(
            r"
            [a]
            b=HIDDEN
            d=quux

            [quux]
            g=h
            "
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
            r"
            [a]
            b=c
            d=e
            "
        ),
        expected: indoc!(
            r"
            [a]
            d=e
            "
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
    dummy_file0_name: String,
    dummy_file1_name: String,
    dummy_file0_path: Utf8PathBuf,
    dummy_file0_source_path: Utf8PathBuf,
    version: ChezmoiVersion,
}

impl DummyChezmoi {
    fn new() -> Self {
        let tmp_dir = tempdir().unwrap();

        let input_dir: Utf8PathBuf = tmp_dir.path().join("input").try_into().unwrap();
        let src_dir: Utf8PathBuf = tmp_dir.path().join("source").try_into().unwrap();

        std::fs::create_dir(input_dir.as_path()).unwrap();
        std::fs::create_dir(src_dir.as_path()).unwrap();

        let dummy_file0_name = format!("{DUMMY_FILE_NAME_PREFIX}0");
        let dummy_file1_name = format!("{DUMMY_FILE_NAME_PREFIX}1");

        let dummy_file0_path: Utf8PathBuf = input_dir.join(dummy_file0_name.as_str());
        let dummy_file1_path: Utf8PathBuf = input_dir.join(dummy_file1_name.as_str());

        std::fs::write(dummy_file0_path.as_path(), "[a]\nb=c").unwrap();
        std::fs::write(dummy_file1_path.as_path(), "[a]\nb=c").unwrap();

        let dummy_file0_source_path: Utf8PathBuf = src_dir.join(dummy_file0_name.as_str());

        Self {
            tmp_dir,
            input_dir,
            src_dir,
            dummy_file0_name,
            dummy_file1_name,
            dummy_file0_path,
            dummy_file0_source_path,
            version: CHEZMOI_AUTO_SOURCE_VERSION,
        }
    }

    fn basic_source_path(&self, path: &Utf8Path) -> Utf8PathBuf {
        let rel_path = diff_utf8_paths(path, self.input_dir.as_path()).unwrap();
        self.src_dir.join(rel_path)
    }

    fn make_script_path(&self, file_name: &str, style: Style) -> Utf8PathBuf {
        match style {
            Style::Auto => todo!("Not implemented in test yet"),
            Style::InPath => self.src_dir.join(format!("modify_{file_name}")),
            Style::InPathTmpl | Style::InSrc => {
                self.src_dir.join(format!("modify_{file_name}.tmpl"))
            }
        }
    }
}

impl Chezmoi for DummyChezmoi {
    fn source_path(&self, path: &Utf8Path) -> anyhow::Result<Option<Utf8PathBuf>> {
        let normal_path = self.basic_source_path(path);
        let script_path_tmpl =
            normal_path.with_file_name(format!("modify_{}.tmpl", normal_path.file_name().unwrap()));
        let script_path_plain =
            normal_path.with_file_name(format!("modify_{}", normal_path.file_name().unwrap()));
        if script_path_tmpl.exists() {
            Ok(Some(script_path_tmpl))
        } else if script_path_plain.exists() {
            Ok(Some(script_path_plain))
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

    fn version(&self) -> anyhow::Result<ChezmoiVersion> {
        Ok(self.version)
    }
}

fn assert_default_script(chezmoi: &DummyChezmoi, style: Style, dummy_file_name: &str) {
    let file_data =
        std::fs::read(chezmoi.src_dir.join(format!("{dummy_file_name}.src.ini"))).unwrap();
    assert_eq!(file_data.strip_suffix(b"\n").unwrap(), b"[a]\nb=c");

    let file_data = std::fs::read(chezmoi.make_script_path(dummy_file_name, style)).unwrap();
    let file_data = String::from_utf8(file_data).unwrap();
    assert!(file_data.starts_with("#!/usr/bin/env chezmoi_modify_manager\n"));

    // No dummy basic file should exist
    assert!(!chezmoi.src_dir.join(dummy_file_name).try_exists().unwrap());
}

fn assert_unchanged_script(chezmoi: &DummyChezmoi, style: Style, dummy_file_name: &str) {
    let file_data =
        std::fs::read(chezmoi.src_dir.join(format!("{dummy_file_name}.src.ini"))).unwrap();
    assert_eq!(file_data.strip_suffix(b"\n").unwrap(), b"[a]\nb=c");

    let file_data = std::fs::read(chezmoi.make_script_path(dummy_file_name, style)).unwrap();
    let file_data = String::from_utf8(file_data).unwrap();
    assert!(
        file_data.starts_with("#!/usr/bin/env chezmoi_modify_manager\n#UNTOUCHED\nsource auto")
    );

    // No dummy basic file should exist
    assert!(!chezmoi.src_dir.join(dummy_file_name).try_exists().unwrap());
}

fn assert_default_basic(chezmoi: &DummyChezmoi, dummy_file_name: &str) {
    let file_data = std::fs::read(chezmoi.src_dir.join(dummy_file_name)).unwrap();
    assert_eq!(file_data, b"[a]\nb=c");

    // No modify script should exist
    assert!(
        !chezmoi
            .src_dir
            .join(format!("{dummy_file_name}.src.ini"))
            .try_exists()
            .unwrap()
    );
    assert!(
        !chezmoi
            .src_dir
            .join(format!("modify_{dummy_file_name}"))
            .try_exists()
            .unwrap()
    );
    assert!(
        !chezmoi
            .src_dir
            .join(format!("modify_{dummy_file_name}.tmpl"))
            .try_exists()
            .unwrap()
    );
}

fn assert_nothing_added(chezmoi: &DummyChezmoi, dummy_file_name: &str) {
    // No files added
    assert!(!chezmoi.src_dir.join(dummy_file_name).try_exists().unwrap());
    assert!(
        !chezmoi
            .src_dir
            .join(format!("{dummy_file_name}.src.ini"))
            .try_exists()
            .unwrap()
    );
    assert!(
        !chezmoi
            .src_dir
            .join(format!("modify_{dummy_file_name}"))
            .try_exists()
            .unwrap()
    );
    assert!(
        !chezmoi
            .src_dir
            .join(format!("modify_{dummy_file_name}.tmpl"))
            .try_exists()
            .unwrap()
    );
}

mod versions {
    use super::DummyChezmoi;
    use super::assert_nothing_added;
    use crate::Style;
    use crate::add::Mode;
    use crate::add::add;

    #[test]
    fn check_error_on_old_chezmoi() {
        // Check that --style path errors on old chezmoi
        let mut chezmoi = DummyChezmoi::new();
        chezmoi.version.1 -= 1;
        let chezmoi = chezmoi;

        let mut stdout: Vec<u8> = vec![];

        let error = add(
            &chezmoi,
            Mode::Normal,
            false,
            Style::InPath,
            chezmoi.dummy_file0_path.as_path(),
            &mut stdout,
        );
        assert!(error.is_err());

        assert_nothing_added(&chezmoi, chezmoi.dummy_file0_name.as_str());
    }
}

mod path_tmpl {
    use super::DummyChezmoi;
    use super::assert_default_basic;
    use super::assert_default_script;
    use super::assert_unchanged_script;
    use crate::Style;
    use crate::add::Mode;
    use crate::add::add;

    #[test]
    fn check_add_normal_missing() {
        let chezmoi = DummyChezmoi::new();
        let mut stdout: Vec<u8> = vec![];

        add(
            &chezmoi,
            Mode::Normal,
            false,
            Style::InPathTmpl,
            chezmoi.dummy_file0_path.as_path(),
            &mut stdout,
        )
        .unwrap();

        assert_default_script(&chezmoi, Style::InPathTmpl, chezmoi.dummy_file0_name.as_str());
    }

    #[test]
    fn check_add_smart_missing() {
        let chezmoi = DummyChezmoi::new();
        let mut stdout: Vec<u8> = vec![];

        add(
            &chezmoi,
            Mode::Smart,
            false,
            Style::InPathTmpl,
            chezmoi.dummy_file0_path.as_path(),
            &mut stdout,
        )
        .unwrap();

        assert_default_basic(&chezmoi, chezmoi.dummy_file0_name.as_str());
    }

    #[test]
    fn check_add_normal_basic() {
        let chezmoi = DummyChezmoi::new();
        let mut stdout: Vec<u8> = vec![];

        std::fs::write(chezmoi.dummy_file0_source_path.as_path(), "old_contents").unwrap();

        add(
            &chezmoi,
            Mode::Normal,
            false,
            Style::InPathTmpl,
            chezmoi.dummy_file0_path.as_path(),
            &mut stdout,
        )
        .unwrap();

        assert_default_script(&chezmoi, Style::InPathTmpl, chezmoi.dummy_file0_name.as_str());
    }

    #[test]
    fn check_add_smart_basic() {
        let chezmoi = DummyChezmoi::new();
        let mut stdout: Vec<u8> = vec![];

        std::fs::write(chezmoi.dummy_file0_source_path.as_path(), "old_contents").unwrap();

        add(
            &chezmoi,
            Mode::Smart,
            false,
            Style::InPathTmpl,
            chezmoi.dummy_file0_path.as_path(),
            &mut stdout,
        )
        .unwrap();

        assert_default_basic(&chezmoi, chezmoi.dummy_file0_name.as_str());
    }

    #[test]
    fn check_add_normal_script() {
        let chezmoi = DummyChezmoi::new();
        let mut stdout: Vec<u8> = vec![];

        let filename = chezmoi.dummy_file0_name.as_str();

        std::fs::write(chezmoi.src_dir.join(format!("{filename}.src.ini")), "old_contents").unwrap();
        std::fs::write(
            chezmoi.src_dir.join(format!("modify_{filename}.tmpl")),
            "#!/usr/bin/env chezmoi_modify_manager\n#UNTOUCHED\nsource auto",
        )
        .unwrap();

        add(
            &chezmoi,
            Mode::Normal,
            false,
            Style::InPathTmpl,
            chezmoi.dummy_file0_path.as_path(),
            &mut stdout,
        )
        .unwrap();

        assert_unchanged_script(&chezmoi, Style::InPathTmpl, filename);
    }

    #[test]
    fn check_add_smart_script() {
        let chezmoi = DummyChezmoi::new();
        let mut stdout: Vec<u8> = vec![];

        let filename = chezmoi.dummy_file0_name.as_str();

        std::fs::write(chezmoi.src_dir.join(format!("{filename}.src.ini")), "old_contents").unwrap();
        std::fs::write(
            chezmoi.src_dir.join(format!("modify_{filename}.tmpl")),
            "#!/usr/bin/env chezmoi_modify_manager\n#UNTOUCHED\nsource auto",
        )
        .unwrap();

        add(
            &chezmoi,
            Mode::Smart,
            false,
            Style::InPathTmpl,
            chezmoi.dummy_file0_path.as_path(),
            &mut stdout,
        )
        .unwrap();

        assert_unchanged_script(&chezmoi, Style::InPathTmpl, filename);
    }
}

mod path {
    use super::DummyChezmoi;
    use super::assert_default_basic;
    use super::assert_default_script;
    use super::assert_unchanged_script;
    use crate::Style;
    use crate::add::Mode;
    use crate::add::add;

    #[test]
    fn check_add_normal_missing() {
        let chezmoi = DummyChezmoi::new();
        let mut stdout: Vec<u8> = vec![];

        add(
            &chezmoi,
            Mode::Normal,
            false,
            Style::InPath,
            chezmoi.dummy_file0_path.as_path(),
            &mut stdout,
        )
        .unwrap();

        assert_default_script(&chezmoi, Style::InPath, chezmoi.dummy_file0_name.as_str());
    }

    #[test]
    fn check_add_smart_missing() {
        let chezmoi = DummyChezmoi::new();
        let mut stdout: Vec<u8> = vec![];

        add(
            &chezmoi,
            Mode::Smart,
            false,
            Style::InPath,
            chezmoi.dummy_file0_path.as_path(),
            &mut stdout,
        )
        .unwrap();

        assert_default_basic(&chezmoi, chezmoi.dummy_file0_name.as_str());
    }

    #[test]
    fn check_add_normal_basic() {
        let chezmoi = DummyChezmoi::new();
        let mut stdout: Vec<u8> = vec![];

        std::fs::write(chezmoi.dummy_file0_source_path.as_path(), "old_contents").unwrap();

        add(
            &chezmoi,
            Mode::Normal,
            false,
            Style::InPath,
            chezmoi.dummy_file0_path.as_path(),
            &mut stdout,
        )
        .unwrap();

        assert_default_script(&chezmoi, Style::InPath, chezmoi.dummy_file0_name.as_str());
    }

    #[test]
    fn check_add_smart_basic() {
        let chezmoi = DummyChezmoi::new();
        let mut stdout: Vec<u8> = vec![];

        std::fs::write(chezmoi.dummy_file0_source_path.as_path(), "old_contents").unwrap();

        add(
            &chezmoi,
            Mode::Smart,
            false,
            Style::InPath,
            chezmoi.dummy_file0_path.as_path(),
            &mut stdout,
        )
        .unwrap();

        assert_default_basic(&chezmoi, chezmoi.dummy_file0_name.as_str());
    }

    #[test]
    fn check_add_normal_script() {
        let chezmoi = DummyChezmoi::new();
        let mut stdout: Vec<u8> = vec![];

        let filename = chezmoi.dummy_file0_name.as_str();

        std::fs::write(chezmoi.src_dir.join(format!("{filename}.src.ini")), "old_contents").unwrap();
        std::fs::write(
            chezmoi.src_dir.join(format!("modify_{filename}")),
            "#!/usr/bin/env chezmoi_modify_manager\n#UNTOUCHED\nsource auto",
        )
        .unwrap();

        add(
            &chezmoi,
            Mode::Normal,
            false,
            Style::InPath,
            chezmoi.dummy_file0_path.as_path(),
            &mut stdout,
        )
        .unwrap();

        assert_unchanged_script(&chezmoi, Style::InPath, filename);
    }

    #[test]
    fn check_add_smart_script() {
        let chezmoi = DummyChezmoi::new();
        let mut stdout: Vec<u8> = vec![];

        let filename = chezmoi.dummy_file0_name.as_str();

        std::fs::write(chezmoi.src_dir.join(format!("{filename}.src.ini")), "old_contents").unwrap();
        std::fs::write(
            chezmoi.src_dir.join(format!("modify_{filename}")),
            "#!/usr/bin/env chezmoi_modify_manager\n#UNTOUCHED\nsource auto",
        )
        .unwrap();

        add(
            &chezmoi,
            Mode::Smart,
            false,
            Style::InPath,
            chezmoi.dummy_file0_path.as_path(),
            &mut stdout,
        )
        .unwrap();

        dbg!(String::from_utf8_lossy(&stdout));

        assert_unchanged_script(&chezmoi, Style::InPath, filename);
    }
}