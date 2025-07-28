use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    process::Command,
};

use crate::Parser;

fn run_json_tests(folder: PathBuf) -> (usize, usize) {
    let (mut tests, mut successes) = (0, 0);

    for file in fs::read_dir(folder).unwrap() {
        let file = file.unwrap();
        let name = file.file_name().into_string().unwrap();

        if !name.starts_with('y') && !name.starts_with('n') {
            eprintln!("{file:?}: Unknown file name prefix");
            continue;
        }
        let should_succeed = name.starts_with('y');
        let parse_result = Parser::new(File::open(file.path()).unwrap()).parse();

        tests += 1;
        if should_succeed && parse_result.is_err() {
            eprintln!(
                "{file:?}: Expected success, but failed: {}",
                parse_result.unwrap_err()
            );
        } else if !should_succeed && parse_result.is_ok() {
            eprintln!(
                "{file:?}: Expected failure, but succeeded: {:?}",
                parse_result.unwrap()
            );
        } else {
            successes += 1;
        }
    }

    (tests, successes)
}

fn run_mason_tests(folder: PathBuf) -> (usize, usize) {
    let (mut tests, mut successes) = (0, 0);

    for file in fs::read_dir(folder).unwrap() {
        let file = file.unwrap();
        if file.path().extension().unwrap() == "json" {
            continue;
        }

        let name = file.file_name().into_string().unwrap();
        if !name.starts_with('y') && !name.starts_with('n') {
            eprintln!("{file:?}: Unknown file name prefix");
            continue;
        }

        let should_succeed = name.starts_with('y');
        let parse_result = Parser::new(File::open(file.path()).unwrap()).parse();

        tests += 1;
        if should_succeed && parse_result.is_err() {
            eprintln!(
                "{file:?}: Expected success, but failed: {}",
                parse_result.unwrap_err()
            );
        } else if !should_succeed && parse_result.is_ok() {
            eprintln!(
                "{file:?}: Expected failure, but succeeded: {:?}",
                parse_result.unwrap()
            );
        } else {
            successes += 1;
        }
    }

    (tests, successes)
}

#[test]
fn test_parser() {
    let command = if !fs::exists("mason").unwrap() {
        Command::new("git")
            .args(["clone", "https://github.com/mortie/mason.git"])
            .output()
            .unwrap()
    } else {
        Command::new("sh")
            .args(["-c", "cd mason && git pull"])
            .output()
            .unwrap()
    };
    if !command.status.success() {
        if !command.stdout.is_empty() {
            eprintln!("{}", String::from_utf8_lossy(&command.stdout));
        }
        if !command.stderr.is_empty() {
            eprintln!("{}", String::from_utf8_lossy(&command.stderr));
        }
        panic!("Failed to download tests");
    }

    let (mut total_tests, mut total_successes) = (0, 0);
    for json_test in ["alt-json-suite", "json-suite"] {
        let folder = Path::new("mason/test-suite").join(json_test);
        let (tests, successes) = run_json_tests(folder);
        total_tests += tests;
        total_successes += successes;
    }
    {
        let folder = Path::new("mason/test-suite").join("mason-suite");
        let (tests, successes) = run_mason_tests(folder);
        total_tests += tests;
        total_successes += successes;
    }

    eprintln!("{total_successes}/{total_tests} tests succeeded");
    if total_successes != total_tests {
        panic!("some tests failed")
    }
}
