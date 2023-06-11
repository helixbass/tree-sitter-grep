use std::{
    env,
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
};

use assert_cmd::prelude::*;
use once_cell::sync::Lazy;
use predicates::{prelude::*, BoxPredicate};

struct Location {
    line: usize,
    #[allow(dead_code)]
    column: usize,
}

impl From<(usize, usize)> for Location {
    fn from(value: (usize, usize)) -> Self {
        Self {
            line: value.0,
            column: value.1,
        }
    }
}

struct Range {
    pub start: Location,
    #[allow(dead_code)]
    pub end: Location,
}

impl From<((usize, usize), (usize, usize))> for Range {
    fn from(value: ((usize, usize), (usize, usize))) -> Self {
        Self {
            start: value.0.into(),
            end: value.1.into(),
        }
    }
}

struct ExpectedMatch {
    pub relative_file_path: String,
    pub lines: Vec<String>,
    pub range: Range,
}

impl ExpectedMatch {
    pub fn expected_output_text(&self) -> String {
        self.lines
            .iter()
            .enumerate()
            .map(|(line_index, line)| {
                format!(
                    "{}:{}:{}",
                    self.relative_file_path,
                    self.range.start.line + line_index + 1,
                    line
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

const MATCH_OUTPUT_LINE_REGEX_STR: &'static str = r#"(^|\n).+:\d+:"#;

fn predicate_from_expected_matches(expected_matches: &[ExpectedMatch]) -> BoxPredicate<str> {
    expected_matches.into_iter().fold(
        BoxPredicate::new(
            predicate::str::is_match(MATCH_OUTPUT_LINE_REGEX_STR)
                .unwrap()
                .count(
                    expected_matches
                        .into_iter()
                        .map(|expected_match| expected_match.lines.len())
                        .sum(),
                ),
        ),
        |predicate, expected_match| {
            BoxPredicate::new(predicate.and(predicate_from_expected_match(expected_match)))
        },
    )
}

fn predicate_from_expected_match(expected_match: &ExpectedMatch) -> BoxPredicate<str> {
    BoxPredicate::new(predicate::str::contains(
        expected_match.expected_output_text(),
    ))
}

fn assert_expected_matches<TArg: AsRef<OsStr>>(
    args: impl IntoIterator<Item = TArg>,
    current_dir: impl AsRef<Path>,
    expected_matches: &[ExpectedMatch],
) {
    Command::cargo_bin("tree-sitter-grep")
        .unwrap()
        .args(args)
        .current_dir(current_dir)
        .assert()
        .success()
        .stdout(predicate_from_expected_matches(expected_matches));
}

struct FixtureQueryExpectedMatches {
    pub fixture_dir_name: String,
    pub language: String,
    pub query_source: String,
    pub expected_matches: Vec<ExpectedMatch>,
}

impl FixtureQueryExpectedMatches {
    pub fn assert_expected_matches(&self) {
        assert_expected_matches(
            [
                "--query-source",
                &self.query_source,
                "--language",
                &self.language,
            ],
            get_fixture_dir_path_from_name(&self.fixture_dir_name),
            &self.expected_matches,
        );
    }
}

fn get_fixture_dir_path_from_name(fixture_dir_name: &str) -> PathBuf {
    // per https://andrewra.dev/2019/03/01/testing-in-rust-temporary-files/
    let root_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut path: PathBuf = root_dir.into();
    path.push("tests/fixtures");
    path.push(fixture_dir_name);
    path
}

const FUNCTION_ITEM_QUERY_SOURCE: &'static str = "(function_item) @function_item";

static RUST_PROJECT_FUNCTION_ITEM_EXPECTED_MATCHES: Lazy<FixtureQueryExpectedMatches> =
    Lazy::new(|| FixtureQueryExpectedMatches {
        fixture_dir_name: "rust_project".into(),
        query_source: FUNCTION_ITEM_QUERY_SOURCE.to_owned(),
        language: "rust".to_owned(),
        expected_matches: vec![
            ExpectedMatch {
                relative_file_path: "./src/helpers.rs".to_owned(),
                lines: vec!["pub fn helper() {}".to_owned()],
                range: ((0, 0), (0, 18)).into(),
            },
            ExpectedMatch {
                relative_file_path: "./src/lib.rs".to_owned(),
                lines: vec![
                    "pub fn add(left: usize, right: usize) -> usize {".to_owned(),
                    "    left + right".to_owned(),
                    "}".to_owned(),
                ],
                range: ((2, 0), (5, 0)).into(),
            },
            ExpectedMatch {
                relative_file_path: "./src/lib.rs".to_owned(),
                lines: vec![
                    "    fn it_works() {".to_owned(),
                    "        let result = add(2, 2);".to_owned(),
                    "        assert_eq!(result, 4);".to_owned(),
                    "    }".to_owned(),
                ],
                range: ((11, 0), (14, 4)).into(),
            },
        ],
    });

#[test]
fn test_query_inline() {
    RUST_PROJECT_FUNCTION_ITEM_EXPECTED_MATCHES.assert_expected_matches();
}
