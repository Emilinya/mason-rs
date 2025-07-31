use std::{
    fs::{self, File},
    io,
    iter::zip,
    path::{Path, PathBuf},
    process::Command,
};

use base64::{Engine, prelude::BASE64_STANDARD};

use crate::Value as MasonValue;
use serde_json::Value as JsonValue;

use crate::from_reader;

fn run_json_tests(folder: PathBuf) -> (usize, usize) {
    let (mut tests, mut successes) = (0, 0);

    for file in fs::read_dir(folder).unwrap() {
        let file = file.unwrap();
        let path = file.path();
        let name = file.file_name().into_string().unwrap();

        if !name.starts_with('y') && !name.starts_with('n') {
            eprintln!("{file:?}: Unknown file name prefix");
            continue;
        }
        let should_succeed = name.starts_with('y');
        let parse_result = from_reader(File::open(&path).unwrap());

        tests += 1;
        if should_succeed && parse_result.is_err() {
            eprintln!(
                "{path:?}: Expected success, but failed: {}\n",
                parse_result.unwrap_err()
            );
        } else if !should_succeed && parse_result.is_ok() {
            eprintln!(
                "{path:?}: Expected failure, but succeeded: {:?}\n",
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
        let path = file.path();
        if path.extension().unwrap() == "json" {
            continue;
        }

        let name = file.file_name().into_string().unwrap();
        if !name.starts_with('y') && !name.starts_with('n') {
            eprintln!("{path:?}: Unknown file name prefix");
            continue;
        }

        tests += 1;
        if name.starts_with('y') {
            if let Err(err) = compare_output(path.to_str().unwrap()) {
                eprintln!("{path:?}: Expected success, but failed: {err}\n");
            } else {
                successes += 1;
            }
        } else {
            #[allow(clippy::collapsible_else_if)]
            if let Ok(value) = from_reader(File::open(&path).unwrap()) {
                eprintln!("{path:?}: Expected failure, but succeeded: {value:?}\n");
            } else {
                successes += 1;
            }
        }
    }

    (tests, successes)
}

fn deep_equals(json: &JsonValue, mason: &MasonValue) -> bool {
    match (json, mason) {
        (JsonValue::Null, MasonValue::Null) => true,
        (JsonValue::Bool(bool1), MasonValue::Bool(bool2)) => bool1 == bool2,
        (JsonValue::Number(number1), MasonValue::Number(number2)) => {
            if let Some(number1) = number1.as_f64()
                && number1 == *number2
            {
                true
            } else {
                false
            }
        }
        (JsonValue::String(string1), MasonValue::String(string2)) => string1 == string2,
        (JsonValue::String(string1), MasonValue::ByteString(string2)) => {
            *string1 == BASE64_STANDARD.encode(string2)
        }
        (JsonValue::Array(array1), MasonValue::Array(array2)) => {
            if array1.len() != array2.len() {
                return false;
            }
            for (value1, value2) in zip(array1, array2) {
                if !deep_equals(value1, value2) {
                    return false;
                }
            }
            true
        }
        (JsonValue::Object(object1), MasonValue::Object(object2)) => {
            if object1.len() != object2.len() {
                return false;
            }
            for (key, value1) in object1 {
                let Some(value2) = object2.get(key) else {
                    return false;
                };
                if !deep_equals(value1, value2) {
                    return false;
                }
            }
            for (key, value2) in object2 {
                let Some(value1) = object1.get(key) else {
                    return false;
                };
                if !deep_equals(value1, value2) {
                    return false;
                }
            }
            true
        }
        _ => false,
    }
}

fn compare_output(mason_file: &str) -> io::Result<()> {
    let json_file = mason_file.replace(".mason", ".json");

    let json_value: JsonValue = serde_json::from_reader(File::open(json_file).unwrap()).unwrap();
    let mason_value = from_reader(File::open(mason_file).unwrap())?;

    if deep_equals(&json_value, &mason_value) {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("mason value != json value:\n{mason_value:?} != {json_value:?}\n"),
        ))
    }
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
    #[allow(clippy::single_element_loop)]
    for json_test in ["json-suite"] {
        let folder = Path::new("mason/test-suite").join(json_test);
        let (tests, successes) = run_json_tests(folder);
        total_tests += tests;
        total_successes += successes;
    }
    for mason_test in ["alt-json-suite", "mason-suite"] {
        let folder = Path::new("mason/test-suite").join(mason_test);
        let (tests, successes) = run_mason_tests(folder);
        total_tests += tests;
        total_successes += successes;
    }

    eprintln!("{total_successes}/{total_tests} tests succeeded");
    if total_successes != total_tests {
        panic!("some tests failed")
    }
}
