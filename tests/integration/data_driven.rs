//! Data & config driven tests.

use std::{
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
};

use pretty_assertions::assert_eq;

use chezmoi_modify_manager::{inner_main, ChmmArgs};

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

        let mut expected_data: Vec<u8> = vec![];
        File::open(&expected)
            .unwrap()
            .read_to_end(&mut expected_data)
            .unwrap();

        let mut stdout: Vec<u8> = vec![];

        inner_main(
            ChmmArgs::Process(test_case),
            || BufReader::new(File::open(&sys).unwrap()),
            || &mut stdout,
        )
        .unwrap();
        assert_eq!(String::from_utf8(stdout), String::from_utf8(expected_data));
    }
}
