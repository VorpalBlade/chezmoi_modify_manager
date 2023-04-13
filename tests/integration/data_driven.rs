//! Data & config driven tests.

use std::{fs::File, io::Read, path::PathBuf};

use assert_cmd::Command;

/// Find all the test cases
fn find_test_cases() -> anyhow::Result<Vec<PathBuf>> {
    let mut path: PathBuf = std::env::var("CARGO_MANIFEST_DIR")?.into();
    path.push("tests");
    path.push("data");

    let mut results = vec![];
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if let Some("cfg") = path.extension().and_then(|x| x.to_str()) {
            results.push(path);
        }
    }
    Ok(results)
}

#[test]
fn test_data() {
    for test_case in find_test_cases().unwrap() {
        // let src = test_case.with_extension("src.ini");
        let sys = test_case.with_extension("sys.ini");
        let expected = test_case.with_extension("expected.ini");
        let mut cmd = Command::cargo_bin("chezmoi_modify_manager").unwrap();
        let assert = cmd.arg(&test_case).pipe_stdin(sys).unwrap().assert();

        let mut expected_data = String::new();
        File::open(&expected)
            .unwrap()
            .read_to_string(&mut expected_data).unwrap();

        assert.success().stdout(expected_data);
    }
}
