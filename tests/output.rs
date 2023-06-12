#![allow(clippy::into_iter_on_ref, clippy::collapsible_if)]
use std::{env, path::PathBuf, process::Command};

use assert_cmd::prelude::*;
use predicates::prelude::*;

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

fn parse_command_line(command_line: &str) -> Vec<String> {
    assert!(command_line.starts_with('$'));
    shlex::split(&command_line[1..]).unwrap()
}

fn assert_sorted_output(fixture_dir_name: &str, command_and_output: &str) {
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
        .stdout(predicate::function(|actual_output| {
            do_sorted_lines_match(actual_output, &output)
        }));
}

fn do_sorted_lines_match(actual_output: &str, expected_output: &str) -> bool {
    let mut actual_lines = actual_output.split('\n').collect::<Vec<_>>();
    actual_lines.sort();
    let mut expected_lines = expected_output.split('\n').collect::<Vec<_>>();
    expected_lines.sort();
    actual_lines == expected_lines
}

fn assert_failure_output(fixture_dir_name: &str, command_and_output: &str) {
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
        // .stderr(predicate::function(|stderr: &str| {
        //     println!("stderr: {stderr:#?}, output: {output:#?}");
        //     stderr == output
        // }));
        .stderr(predicate::eq(output));
}

#[test]
fn test_query_inline() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query-source '(function_item) @function_item' --language rust
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
            src/stop.rs:1:fn stop_it() {}
        "#,
    );
}

#[test]
fn test_query_inline_short_option() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @function_item' --language rust
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
            src/stop.rs:1:fn stop_it() {}
        "#,
    );
}

#[test]
fn test_vimgrep_mode() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query-source '(function_item) @function_item' --language rust --vimgrep
            src/helpers.rs:1:1:pub fn helper() {}
            src/lib.rs:3:1:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:12:5:    fn it_works() {
            src/stop.rs:1:1:fn stop_it() {}
       "#,
    );
}

#[test]
fn test_query_file() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query-file ./function-item.scm --language rust
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
            src/stop.rs:1:fn stop_it() {}
       "#,
    );
}

#[test]
fn test_query_file_short_option() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -Q ./function-item.scm --language rust
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
            src/stop.rs:1:fn stop_it() {}
       "#,
    );
}

#[test]
fn test_specify_single_file() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query-source '(function_item) @function_item' --language rust src/lib.rs
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
        "#,
    );
}

#[test]
fn test_specify_single_file_preserves_leading_dot_slash() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query-source '(function_item) @function_item' --language rust ./src/lib.rs
            ./src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            ./src/lib.rs:4:    left + right
            ./src/lib.rs:5:}
            ./src/lib.rs:12:    fn it_works() {
            ./src/lib.rs:13:        let result = add(2, 2);
            ./src/lib.rs:14:        assert_eq!(result, 4);
            ./src/lib.rs:15:    }
        "#,
    );
}

#[test]
fn test_specify_multiple_files() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query-source '(function_item) @function_item' --language rust src/lib.rs ./src/helpers.rs
            ./src/helpers.rs:1:pub fn helper() {}
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
        "#,
    );
}

#[test]
fn test_invalid_query_inline() {
    assert_failure_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query-source '(function_itemz) @function_item' --language rust
            error: invalid query
        "#,
    );
}

#[test]
fn test_invalid_query_file() {
    assert_failure_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query-file ./function-itemz.scm --language rust
            error: invalid query
        "#,
    );
}

#[test]
fn test_no_query_specified() {
    assert_failure_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --language rust
            error: the following required arguments were not provided:
              <--query-file <PATH_TO_QUERY_FILE>|--query-source <QUERY_SOURCE>>
            
            Usage: tree-sitter-grep --language <LANGUAGE> <--query-file <PATH_TO_QUERY_FILE>|--query-source <QUERY_SOURCE>> [PATHS]...
            
            For more information, try '--help'.
        "#,
    );
}

#[test]
fn test_invalid_language_name() {
    assert_failure_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query-source '(function_item) @function_item' --language rustz
            error: invalid value 'rustz' for '--language <LANGUAGE>'
              [possible values: rust, typescript, javascript]

              tip: a similar value exists: 'rust'

            For more information, try '--help'.
        "#,
    );
}

#[test]
fn test_invalid_query_file_path() {
    assert_failure_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query-file ./nonexistent.scm --language rust
            error: couldn't read query file "./nonexistent.scm"
        "#,
    );
}

#[test]
fn test_auto_language_single_known_language_encountered() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query-source '(function_item) @function_item'
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
            src/stop.rs:1:fn stop_it() {}
        "#,
    );
}

#[test]
fn test_auto_language_multiple_parseable_languages() {
    assert_sorted_output(
        "mixed_project",
        r#"
            $ tree-sitter-grep --query-source '(arrow_function) @arrow_function'
            javascript_src/index.js:1:const js_foo = () => {}
            typescript_src/index.tsx:1:const foo = () => {}
        "#,
    );
}

#[test]
fn test_auto_language_single_parseable_languages() {
    assert_sorted_output(
        "mixed_project",
        r#"
            $ tree-sitter-grep --query-source '(function_item) @function_item'
            rust_src/lib.rs:1:fn foo() {}
        "#,
    );
}

#[test]
fn test_capture_name() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query-source '(function_item name: (identifier) @name) @function_item' --language rust --capture function_item
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
            src/stop.rs:1:fn stop_it() {}
        "#,
    );
}

#[test]
fn test_predicate() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query-source '(function_item name: (identifier) @name (#eq? @name "add")) @function_item' --language rust --capture function_item
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
        "#,
    );
}

#[test]
fn test_no_matches() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query-source '(function_item name: (identifier) @name (#eq? @name "addz")) @function_item' --language rust
        "#,
    );
}

#[test]
fn test_invalid_capture_name() {
    assert_failure_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query-source '(function_item) @function_item' --language rust --capture function_itemz
            error: invalid capture name 'function_itemz'
        "#,
    );
}

#[test]
fn test_unknown_option() {
    assert_failure_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query-sourcez '(function_item) @function_item' --language rust
            error: unexpected argument '--query-sourcez' found

              tip: a similar argument exists: '--query-source'

            Usage: tree-sitter-grep <--query-file <PATH_TO_QUERY_FILE>|--query-source <QUERY_SOURCE>> [PATHS]...

            For more information, try '--help'.
        "#,
    );
}
