#![allow(clippy::into_iter_on_ref, clippy::collapsible_if)]
use std::{borrow::Cow, env, path::PathBuf, process::Command};

use assert_cmd::prelude::*;
use predicates::prelude::*;
use regex::Captures;

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

fn assert_sorted_output(fixture_dir_name: &str, command_and_output: &str) {
    assert_sorted_output_with_exit_code(fixture_dir_name, command_and_output, None);
}

fn assert_sorted_output_with_no_matches_exit_status(
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
    regex!(r#"\r$"#).replace(line, "")
}

fn normalize_match_path(line: &str) -> Cow<'_, str> {
    regex!(r#"^[^:]+[:-]\d+[:-]"#)
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
        .code(2)
        .stderr(predicate::function(|stderr: &str| {
            let stderr = massage_error_output(stderr);
            stderr == output
        }));
}

fn assert_non_match_output(fixture_dir_name: &str, command_and_output: &str) {
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

fn build_example(example_name: &str) {
    // CargoBuild::new().example(example_name).exec().unwrap();
    Command::new("cargo")
        .args(["build", "--example", example_name])
        .status()
        .expect("Build example command failed");
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
            error: couldn't parse query for Rust
        "#,
    );
}

#[test]
fn test_invalid_query_file() {
    assert_failure_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query-file ./function-itemz.scm --language rust
            error: couldn't parse query for Rust
        "#,
    );
}

#[test]
fn test_no_query_or_filter_specified() {
    assert_failure_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --language rust
            error: the following required arguments were not provided:
              <--query-file <PATH_TO_QUERY_FILE>|--query-source <QUERY_SOURCE>|--filter <FILTER>>

            Usage: tree-sitter-grep --language <LANGUAGE> <--query-file <PATH_TO_QUERY_FILE>|--query-source <QUERY_SOURCE>|--filter <FILTER>> [PATHS]...

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
    assert_sorted_output_with_no_matches_exit_status(
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

            Usage: tree-sitter-grep <--query-file <PATH_TO_QUERY_FILE>|--query-source <QUERY_SOURCE>|--filter <FILTER>> <PATHS|--query-file <PATH_TO_QUERY_FILE>|--query-source <QUERY_SOURCE>|--capture <CAPTURE_NAME>|--language <LANGUAGE>|--filter <FILTER>|--filter-arg <FILTER_ARG>|--vimgrep|--after-context <NUM>|--before-context <NUM>|--context <NUM>>

            For more information, try '--help'.
        "#,
    );
}

#[test]
fn test_filter_plugin() {
    build_example("filter_before_line_10");

    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query-source '(function_item) @function_item' --language rust --filter ../../../target/debug/examples/libfilter_before_line_10.so
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/stop.rs:1:fn stop_it() {}
        "#,
    );
}

#[test]
fn test_filter_plugin_with_argument() {
    build_example("filter_before_line_number");

    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query-source '(function_item) @function_item' --language rust --filter ../../../target/debug/examples/libfilter_before_line_number.so --filter-arg 2
            src/helpers.rs:1:pub fn helper() {}
            src/stop.rs:1:fn stop_it() {}
        "#,
    );
}

#[test]
fn test_filter_plugin_expecting_argument_not_received() {
    build_example("filter_before_line_number");

    assert_failure_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query-source '(function_item) @function_item' --language rust --filter ../../../target/debug/examples/libfilter_before_line_number.so
            error: plugin expected '--filter-arg <ARGUMENT>'
        "#,
    );
}

#[test]
fn test_filter_plugin_unparseable_argument() {
    build_example("filter_before_line_number");

    assert_failure_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query-source '(function_item) @function_item' --language rust --filter ../../../target/debug/examples/libfilter_before_line_number.so --filter-arg abc
            error: plugin couldn't parse argument "abc"
        "#,
    );
}

#[test]
fn test_filter_plugin_no_query() {
    build_example("filter_function_items_before_line_10");

    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --language rust --filter ../../../target/debug/examples/libfilter_function_items_before_line_10.so
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/stop.rs:1:fn stop_it() {}
        "#,
    );
}

#[test]
fn test_query_inline_and_query_file_path() {
    assert_failure_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query-source '(function_item) @function_item' --query-file ./function-item.scm --language rust
            error: the argument '--query-source <QUERY_SOURCE>' cannot be used with '--query-file <PATH_TO_QUERY_FILE>'

            Usage: tree-sitter-grep --language <LANGUAGE> <--query-file <PATH_TO_QUERY_FILE>|--query-source <QUERY_SOURCE>|--filter <FILTER>> [PATHS]...

            For more information, try '--help'.
        "#,
    );
}

#[test]
fn test_help_option() {
    assert_non_match_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --help
            Usage: tree-sitter-grep [OPTIONS] <--query-file <PATH_TO_QUERY_FILE>|--query-source <QUERY_SOURCE>|--filter <FILTER>> [PATHS]...

            Arguments:
              [PATHS]...

            Options:
              -Q, --query-file <PATH_TO_QUERY_FILE>
              -q, --query-source <QUERY_SOURCE>
              -c, --capture <CAPTURE_NAME>
              -l, --language <LANGUAGE>              [possible values: rust, typescript, javascript]
              -f, --filter <FILTER>
              -a, --filter-arg <FILTER_ARG>
                  --vimgrep
              -A, --after-context <NUM>
              -B, --before-context <NUM>
              -C, --context <NUM>
              -h, --help                             Print help
        "#,
    );
}

#[test]
fn test_no_arguments() {
    assert_failure_output(
        "rust_project",
        r#"
            $ tree-sitter-grep
            error: the following required arguments were not provided:
              <--query-file <PATH_TO_QUERY_FILE>|--query-source <QUERY_SOURCE>|--filter <FILTER>>

            Usage: tree-sitter-grep <--query-file <PATH_TO_QUERY_FILE>|--query-source <QUERY_SOURCE>|--filter <FILTER>> [PATHS]...

            For more information, try '--help'.
        "#,
    );
}

#[test]
fn test_filter_argument_no_filter() {
    build_example("filter_before_line_number");

    assert_failure_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query-source '(function_item) @function_item' --language rust --filter-arg 2
            error: the following required arguments were not provided:
              --filter <FILTER>

            Usage: tree-sitter-grep --language <LANGUAGE> --filter-arg <FILTER_ARG> <--query-file <PATH_TO_QUERY_FILE>|--query-source <QUERY_SOURCE>|--filter <FILTER>> [PATHS]...

            For more information, try '--help'.
        "#,
    );
}

#[test]
fn test_macro_contents() {
    assert_sorted_output(
        "match_inside_macro",
        r#"
            $ tree-sitter-grep -q '(call_expression) @c' -l rust
            foo.rs:4:        self.factory
            foo.rs:5:            .create_parameter_declaration("whee", Option::<Gc<NodeArray>>::None)
            foo.rs:6:            .wrap(),
        "#,
    );
}

#[test]
fn test_sorting_maybe_nesting_related() {
    assert_sorted_output(
        "sorting_maybe_nesting_related",
        r#"
            $ tree-sitter-grep -Q ./query.scm -l rust --vimgrep
            foo.rs:44:14:            .create_variable_statement_raw(
            foo.rs:47:22:                    .create_variable_declaration_list_raw(
        "#,
    );
}

#[test]
fn test_overlapping_matches() {
    assert_sorted_output(
        "rust_overlapping",
        r#"
            $ tree-sitter-grep --query-source '(closure_expression) @closure_expression' --language rust
            src/lib.rs:2:    let f = || {
            src/lib.rs:3:        || {
            src/lib.rs:4:            println!("whee");
            src/lib.rs:5:        }
            src/lib.rs:6:    };
        "#,
    );
}

#[test]
fn test_overlapping_matches_vimgrep() {
    assert_sorted_output(
        "rust_overlapping",
        r#"
            $ tree-sitter-grep --query-source '(closure_expression) @closure_expression' --language rust --vimgrep
            src/lib.rs:2:13:    let f = || {
            src/lib.rs:3:9:        || {
        "#,
    );
}

#[test]
fn test_after_context() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' -l rust --after-context 2
            src/stop.rs:1:fn stop_it() {}
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/lib.rs-6-
            src/lib.rs-7-#[cfg(test)]
            --
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
            src/lib.rs-16-}
            src/lib.rs-17-
        "#,
    );
}

#[test]
fn test_after_context_matches_overlap_context_lines() {
    assert_sorted_output(
        "rust_overlapping",
        r#"
            $ tree-sitter-grep -q '(call_expression function: (identifier) @function_name (#match? @function_name "^h"))' -l rust -A 2
            src/lib.rs:10:    hello();
            src/lib.rs:11:    hoo();
            src/lib.rs-12-    raa();
            src/lib.rs-13-    roo();
        "#,
    );
}

#[test]
fn test_after_context_overlapping_matches() {
    assert_sorted_output(
        "rust_overlapping",
        r#"
            $ tree-sitter-grep -q '(closure_expression) @c' -l rust --after-context 2
            src/lib.rs:2:    let f = || {
            src/lib.rs:3:        || {
            src/lib.rs:4:            println!("whee");
            src/lib.rs:5:        }
            src/lib.rs:6:    };
            src/lib.rs-7-}
            src/lib.rs-8-
        "#,
    );
}

#[test]
fn test_after_context_overlapping_multiline_matches_vimgrep() {
    assert_sorted_output(
        "rust_overlapping",
        r#"
            $ tree-sitter-grep -q '(closure_expression) @c' -l rust --after-context 2 --vimgrep
            src/lib.rs:2:13:    let f = || {
            src/lib.rs:3:9:        || {
            src/lib.rs-7-}
            src/lib.rs-8-
        "#,
    );
}

#[test]
fn test_after_context_short_option() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' -l rust -A 2
            src/stop.rs:1:fn stop_it() {}
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/lib.rs-6-
            src/lib.rs-7-#[cfg(test)]
            --
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
            src/lib.rs-16-}
            src/lib.rs-17-
        "#,
    );
}

#[test]
fn test_before_context() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' -l rust --before-context 3
            src/stop.rs:1:fn stop_it() {}
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs-1-mod helpers;
            src/lib.rs-2-
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            --
            src/lib.rs-9-    use super::*;
            src/lib.rs-10-
            src/lib.rs-11-    #[test]
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
        "#,
    );
}

#[test]
fn test_before_context_short_option() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' -l rust -B 3
            src/stop.rs:1:fn stop_it() {}
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs-1-mod helpers;
            src/lib.rs-2-
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            --
            src/lib.rs-9-    use super::*;
            src/lib.rs-10-
            src/lib.rs-11-    #[test]
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
        "#,
    );
}

#[test]
fn test_before_context_matches_overlap_context_lines() {
    assert_sorted_output(
        "rust_overlapping",
        r#"
            $ tree-sitter-grep -q '(call_expression function: (identifier) @function_name (#match? @function_name "^h"))' -l rust -B 2
            src/lib.rs-8-
            src/lib.rs-9-fn something_else() {
            src/lib.rs:10:    hello();
            src/lib.rs:11:    hoo();
        "#,
    );
}

#[test]
fn test_before_context_overlapping_matches() {
    assert_sorted_output(
        "rust_overlapping_with_preceding_lines",
        r#"
            $ tree-sitter-grep -q '(closure_expression) @c' -l rust --before-context 2
            src/lib.rs-5-        .i_promise()
            src/lib.rs-6-        .but_it_has_to_be_longer();
            src/lib.rs:7:    let f = || {
            src/lib.rs:8:        || {
            src/lib.rs:9:            println!("whee");
            src/lib.rs:10:        }
            src/lib.rs:11:    };
        "#,
    );
}

#[test]
fn test_before_context_overlapping_multiline_matches_vimgrep() {
    assert_sorted_output(
        "rust_overlapping_with_preceding_lines",
        r#"
            $ tree-sitter-grep -q '(closure_expression) @c' -l rust --before-context 2 --vimgrep
            src/lib.rs-5-        .i_promise()
            src/lib.rs-6-        .but_it_has_to_be_longer();
            src/lib.rs:7:13:    let f = || {
            src/lib.rs:8:9:        || {
        "#,
    );
}

#[test]
fn test_context() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' -l rust --context 2
            src/stop.rs:1:fn stop_it() {}
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs-1-mod helpers;
            src/lib.rs-2-
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/lib.rs-6-
            src/lib.rs-7-#[cfg(test)]
            --
            src/lib.rs-10-
            src/lib.rs-11-    #[test]
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
            src/lib.rs-16-}
            src/lib.rs-17-
        "#,
    );
}

#[test]
fn test_context_adjacent_after_and_before_context_lines() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' -l rust --context 3
            src/stop.rs:1:fn stop_it() {}
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs-1-mod helpers;
            src/lib.rs-2-
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/lib.rs-6-
            src/lib.rs-7-#[cfg(test)]
            src/lib.rs-8-mod tests {
            src/lib.rs-9-    use super::*;
            src/lib.rs-10-
            src/lib.rs-11-    #[test]
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
            src/lib.rs-16-}
            src/lib.rs-17-
            src/lib.rs-18-mod stop;
        "#,
    );
}

#[test]
fn test_context_overlapping_after_and_before_context_lines() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' -l rust --context 4
            src/stop.rs:1:fn stop_it() {}
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs-1-mod helpers;
            src/lib.rs-2-
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/lib.rs-6-
            src/lib.rs-7-#[cfg(test)]
            src/lib.rs-8-mod tests {
            src/lib.rs-9-    use super::*;
            src/lib.rs-10-
            src/lib.rs-11-    #[test]
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
            src/lib.rs-16-}
            src/lib.rs-17-
            src/lib.rs-18-mod stop;
        "#,
    );
}

#[test]
fn test_context_short_option() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' -l rust -C 2
            src/stop.rs:1:fn stop_it() {}
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs-1-mod helpers;
            src/lib.rs-2-
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/lib.rs-6-
            src/lib.rs-7-#[cfg(test)]
            --
            src/lib.rs-10-
            src/lib.rs-11-    #[test]
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
            src/lib.rs-16-}
            src/lib.rs-17-
        "#,
    );
}

#[test]
fn test_before_and_after_context() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' -l rust --before-context 2 --after-context 1
            src/stop.rs:1:fn stop_it() {}
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs-1-mod helpers;
            src/lib.rs-2-
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/lib.rs-6-
            --
            src/lib.rs-10-
            src/lib.rs-11-    #[test]
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
            src/lib.rs-16-}
        "#,
    );
}

#[test]
fn test_no_files_searched_directory_path_argument_with_no_recognized_file_types() {
    assert_failure_output(
        "no_recognized_file_types",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' subdir/
            No files were searched
        "#,
    );
}

#[test]
fn test_no_files_searched_no_recognized_file_types() {
    assert_failure_output(
        "no_recognized_file_types",
        r#"
            $ tree-sitter-grep -q '(function_item) @f'
            No files were searched
        "#,
    );
}

#[test]
fn test_no_files_searched_recognized_files_but_dont_match_specified_language() {
    assert_failure_output(
        "typescript_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' --language rust
            No files were searched
        "#,
    );
}

#[test]
fn test_couldnt_parse_more_than_two_candidate_auto_detected_languages() {
    assert_failure_output(
        "mixed_project",
        r#"
            $ tree-sitter-grep -q '(function_itemz) @f'
            error: couldn't parse query for Javascript, Rust or Typescript
        "#,
    );
}

#[test]
fn test_couldnt_parse_two_candidate_auto_detected_languages() {
    assert_failure_output(
        "mixed_project",
        r#"
            $ tree-sitter-grep -q '(function_itemz) @f' javascript_src/ typescript_src/
            error: couldn't parse query for Javascript or Typescript
        "#,
    );
}

#[test]
fn test_nonexistent_file_specified() {
    assert_failure_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' src/nonexistent.rs
            src/nonexistent.rs: No such file or directory (os error 2)
        "#,
    );
}

#[test]
fn test_nonexistent_directory_specified() {
    assert_failure_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' srcz/
            srcz/: No such file or directory (os error 2)
        "#,
    );
}

// #[test]
// fn test_specify_explicit_file_but_dont_match_specified_language() {
//     assert_failure_output(
//         "mixed_project",
//         r#"
//             $ tree-sitter-grep -q '(function_item) @f' --language rust
// javascript_src/index.js         "#,
//     );
// }

// #[test]
// fn test_specify_explicit_file_of_unrecognized_file_type() {
//     assert_failure_output(
//         "no_recognized_file_types",
//         r#"
//             $ tree-sitter-grep -q '(function_item) @f' something.scala
//         "#,
//     );
// }

// #[test]
// fn test_specify_explicit_file_of_unrecognized_file_type_and_language_flag() {
//     assert_failure_output(
//         "no_recognized_file_types",
//         r#"
//             $ tree-sitter-grep -q '(function_item) @f' --language rust
// something.scala         "#,
//     );
// }
