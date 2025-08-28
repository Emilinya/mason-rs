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

fn try_run(program: &str, args: &[&str]) {
    let command = Command::new(program).args(args).output().unwrap();
    if !command.status.success() {
        if !command.stdout.is_empty() {
            eprintln!("{}", String::from_utf8_lossy(&command.stdout));
        }
        if !command.stderr.is_empty() {
            eprintln!("{}", String::from_utf8_lossy(&command.stderr));
        }
        panic!("Failed to run {} {}", program, args.join(" "));
    }
}

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

        let json_file = path.to_str().unwrap();

        tests += 1;
        if check_similarity(json_file, json_file) {
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

        let mason_file = path.to_str().unwrap();
        let json_file = mason_file.replace(".mason", ".json");

        tests += 1;
        if check_similarity(mason_file, &json_file) {
            successes += 1;
        }
    }

    (tests, successes)
}

fn check_similarity(mason_file: &str, json_file: &str) -> bool {
    let path_buf = PathBuf::from(mason_file);
    let end = path_buf.file_name().unwrap().to_str().unwrap();

    let mut success = true;
    if end.starts_with('y') {
        if let Err(err) = compare_output(mason_file, json_file, false) {
            eprintln!("{mason_file:?}: Expected success, but failed (without serde): {err}\n");
            success = false;
        }
        if let Err(err) = compare_output(mason_file, json_file, true) {
            eprintln!("{mason_file:?}: Expected success, but failed (with serde): {err}\n");
            success = false;
        }
    } else {
        if let Ok(value) = MasonValue::from_reader(File::open(mason_file).unwrap()) {
            eprintln!(
                "{mason_file:?}: Expected failure, but succeeded (without serde): {value:?}\n"
            );
            success = false;
        }
        if let Ok(value) = from_reader::<MasonValue, _>(File::open(mason_file).unwrap()) {
            eprintln!("{mason_file:?}: Expected failure, but succeeded (with serde): {value:?}\n");
            success = false;
        }
    }
    success
}

fn deep_equals(json: &JsonValue, mason: &MasonValue) -> bool {
    match (json, mason) {
        (JsonValue::Null, MasonValue::Null) => true,
        (JsonValue::Bool(bool1), MasonValue::Bool(bool2)) => bool1 == bool2,
        (JsonValue::Number(number1), MasonValue::Number(number2)) => {
            number1.as_f64().is_some_and(|number1| number1 == *number2)
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

fn compare_output(mason_file: &str, json_file: &str, use_serde: bool) -> io::Result<()> {
    let json_value: JsonValue = serde_json::from_reader(File::open(json_file).unwrap()).unwrap();
    let mason_value = if use_serde {
        from_reader(File::open(mason_file).unwrap())
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?
    } else {
        MasonValue::from_reader(File::open(mason_file).unwrap())?
    };

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
    if !fs::exists("mason").unwrap() {
        try_run(
            "git",
            &[
                "clone",
                "https://github.com/mortie/mason.git",
                "-c",
                // replacing \n with \r\n breaks some tests
                "core.autocrlf=false",
            ],
        );
    } else {
        try_run("git", &["-C", "mason", "fetch"]);
    }

    let revision = "3ac297d9f8bbe49a2909d533c9d438c49b2f143c";
    try_run("git", &["-C", "mason", "checkout", revision]);

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
