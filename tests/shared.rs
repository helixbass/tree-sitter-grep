#![allow(clippy::into_iter_on_ref, clippy::collapsible_if, dead_code)]
use std::{borrow::Cow, env, path::PathBuf, process::Command};

use assert_cmd::prelude::*;
use predicates::prelude::*;
use regex::Captures;
use text_diff::print_diff;

#[macro_export]
macro_rules! regex {
    ($re:literal $(,)?) => {{
        static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        RE.get_or_init(|| regex::Regex::new($re).unwrap())
    }};
}

fn get_fixture_dir_path_from_name(fixture_dir_name: &str) -> PathBuf {
    // per https://andrewra.dev/2019/03/01/testing-in-rust-temporary-files/
    let root_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut path: PathBuf = root_dir.into();
    path.push("tests/fixtures");
    path.push(fixture_dir_name);
    path
}

fn parse_command_and_output(command_and_output: &str) -> CommandAndOutput {
    let mut lines = command_and_output.split('\n').collect::<Vec<_>>();
    if lines.is_empty() {
        panic!("Expected at least a command line");
    }
    if lines[0].trim().is_empty() {
        lines.remove(0);
    }
    let command_line = lines.remove(0);
    let indent = regex!(r#"^\s*"#).find(command_line).unwrap().as_str();
    let command_line_args = parse_command_line(strip_indent(command_line, indent));
    if !lines.is_empty() {
        if lines[lines.len() - 1].trim().is_empty() {
            lines.pop();
        }
    }
    let output: String = lines
        .into_iter()
        .map(|line| {
            if line.is_empty() {
                "\n".to_owned()
            } else {
                assert!(line.starts_with(indent));
                format!("{}\n", strip_indent(line, indent))
            }
        })
        .collect();
    CommandAndOutput {
        command_line_args,
        output,
    }
}

struct CommandAndOutput {
    command_line_args: Vec<String>,
    output: String,
}

fn strip_indent<'line>(line: &'line str, indent: &str) -> &'line str {
    &line[indent.len()..]
}

const DYNAMIC_LIBRARY_EXTENSION: &str = if cfg!(target_os = "macos") {
    ".dylib"
} else if cfg!(windows) {
    ".dll"
} else {
    ".so"
};

fn get_dynamic_library_name(library_name: &str) -> String {
    if cfg!(windows) {
        format!("{library_name}{}", DYNAMIC_LIBRARY_EXTENSION)
    } else {
        format!("lib{library_name}{}", DYNAMIC_LIBRARY_EXTENSION)
    }
}

fn parse_command_line(command_line: &str) -> Vec<String> {
    assert!(command_line.starts_with('$'));
    shlex::split(&command_line[1..])
        .unwrap()
        .iter()
        .map(|arg| {
            regex!(r#"lib(\S+)\.so$"#)
                .replace(arg, |captures: &Captures| {
                    get_dynamic_library_name(&captures[1])
                })
                .into_owned()
        })
        .collect()
}

fn assert_sorted_output_with_exit_code(
    fixture_dir_name: &str,
    command_and_output: &str,
    failure_code: Option<i32>,
) {
    let CommandAndOutput {
        mut command_line_args,
        output,
    } = parse_command_and_output(command_and_output);
    let command_name = command_line_args.remove(0);
    let mut command = Command::cargo_bin(command_name).unwrap();
    command
        .args(command_line_args)
        .current_dir(get_fixture_dir_path_from_name(fixture_dir_name));
    let command = if let Some(failure_code) = failure_code {
        command.assert().failure().code(failure_code)
    } else {
        command.assert().success()
    };
    command.stdout(predicate::function(|actual_output| {
        do_sorted_lines_match(actual_output, &output)
    }));
}

pub fn assert_sorted_output(fixture_dir_name: &str, command_and_output: &str) {
    assert_sorted_output_with_exit_code(fixture_dir_name, command_and_output, None);
}

pub fn assert_sorted_output_with_no_matches_exit_status(
    fixture_dir_name: &str,
    command_and_output: &str,
) {
    assert_sorted_output_with_exit_code(fixture_dir_name, command_and_output, Some(1));
}

fn massage_windows_line(line: &str) -> String {
    if cfg!(windows) {
        let line = strip_trailing_carriage_return(line);
        let line = normalize_match_path(&line);
        line.into_owned()
    } else {
        line.to_owned()
    }
}

fn strip_trailing_carriage_return(line: &str) -> Cow<'_, str> {
    regex!(r#"\r((?:\u{1b}\[\d+m)*)$"#).replace(line, "$1")
}

fn normalize_match_path(line: &str) -> Cow<'_, str> {
    regex!(r#"^(?:\u{1b}\[\d+m)*[a-zA-Z_\-\\/]+\.(?:rs|js|tsx)"#)
        .replace(line, |captures: &Captures| captures[0].replace('\\', "/"))
}

fn do_sorted_lines_match(actual_output: &str, expected_output: &str) -> bool {
    let mut actual_lines = actual_output
        .split('\n')
        .map(massage_windows_line)
        .collect::<Vec<_>>();
    actual_lines.sort();
    let mut expected_lines = expected_output.split('\n').collect::<Vec<_>>();
    expected_lines.sort();
    actual_lines == expected_lines
}

pub fn assert_failure_output(fixture_dir_name: &str, command_and_output: &str) {
    let CommandAndOutput {
        mut command_line_args,
        output,
    } = parse_command_and_output(command_and_output);
    let command_name = command_line_args.remove(0);
    Command::cargo_bin(command_name)
        .unwrap()
        .args(command_line_args)
        .current_dir(get_fixture_dir_path_from_name(fixture_dir_name))
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::function(|stderr: &str| {
            let stderr = massage_error_output(stderr);
            stderr == output
        }));
}

pub fn assert_non_match_output(fixture_dir_name: &str, command_and_output: &str) {
    let CommandAndOutput {
        mut command_line_args,
        output,
    } = parse_command_and_output(command_and_output);
    let command_name = command_line_args.remove(0);
    Command::cargo_bin(command_name)
        .unwrap()
        .args(command_line_args)
        .current_dir(get_fixture_dir_path_from_name(fixture_dir_name))
        .assert()
        .success()
        .stdout(predicate::function(|stdout: &str| {
            let stdout = massage_error_output(stdout);
            if stdout != output {
                print_diff(&stdout, &output, " ");
            }
            stdout == output
        }));
}

fn massage_error_output(output: &str) -> String {
    if cfg!(windows) {
        output.replace(".exe", "").replace(
            "The system cannot find the file specified.",
            "No such file or directory",
        )
    } else {
        output.to_owned()
    }
    .split('\n')
    .map(|line| line.trim_end())
    .collect::<Vec<_>>()
    .join("\n")
}

pub fn build_example(example_name: &str) {
    // CargoBuild::new().example(example_name).exec().unwrap();
    Command::new("cargo")
        .args(["build", "--example", example_name])
        .status()
        .expect("Build example command failed");
}
