use assert_cmd::prelude::*;
use once_cell::sync::Lazy;
use predicates::{prelude::*, BoxPredicate};
use std::{
    env,
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
};

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

const MATCH_OUTPUT_LINE_REGEX_STR: &'static str = r#"(^|\n).+:\d+:"#;

fn predicate_from_expected_matches(
    expected_matches: &[ExpectedMatch],
    output_mode: OutputMode,
) -> BoxPredicate<str> {
    expected_matches.into_iter().fold(
        BoxPredicate::new(
            predicate::str::is_match(MATCH_OUTPUT_LINE_REGEX_STR)
                .unwrap()
                .count(output_mode.get_expected_total_number_of_match_lines(expected_matches)),
        ),
        |predicate, expected_match| {
            BoxPredicate::new(
                predicate.and(predicate_from_expected_match(expected_match, output_mode)),
            )
        },
    )
}

fn predicate_from_expected_match(
    expected_match: &ExpectedMatch,
    output_mode: OutputMode,
) -> BoxPredicate<str> {
    BoxPredicate::new(predicate::str::contains(
        output_mode.expected_output_text(expected_match),
    ))
}

fn assert_expected_matches<TArg: AsRef<OsStr>>(
    args: impl IntoIterator<Item = TArg>,
    current_dir: impl AsRef<Path>,
    expected_matches: &[ExpectedMatch],
    output_mode: OutputMode,
) {
    Command::cargo_bin("tree-sitter-grep")
        .unwrap()
        .args(args)
        .current_dir(current_dir)
        .assert()
        .success()
        .stdout(predicate_from_expected_matches(
            expected_matches,
            output_mode,
        ));
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum OutputMode {
    Normal,
    Vimgrep,
}

impl OutputMode {
    pub fn get_expected_total_number_of_match_lines(
        &self,
        expected_matches: &[ExpectedMatch],
    ) -> usize {
        match self {
            Self::Normal => expected_matches
                .into_iter()
                .map(|expected_match| expected_match.lines.len())
                .sum(),
            Self::Vimgrep => expected_matches.len(),
        }
    }

    pub fn expected_output_text(&self, expected_match: &ExpectedMatch) -> String {
        match self {
            Self::Normal => expected_match
                .lines
                .iter()
                .enumerate()
                .map(|(line_index, line)| {
                    format!(
                        "{}:{}:{}",
                        with_leading_dot_slash(&expected_match.relative_file_path),
                        expected_match.range.start.line + line_index + 1,
                        line
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"),
            Self::Vimgrep => format!(
                "{}:{}:{}:{}",
                expected_match.relative_file_path,
                expected_match.range.start.line + 1,
                expected_match.range.start.column + 1,
                &expected_match.lines[0]
            ),
        }
    }
}

fn with_leading_dot_slash(relative_path: &str) -> String {
    if relative_path.starts_with(".") {
        relative_path.to_owned()
    } else {
        format!("./{relative_path}")
    }
}

struct FixtureQueryExpectedMatches {
    pub fixture_dir_name: String,
    pub language: String,
    pub query_source: String,
    pub expected_matches: Vec<ExpectedMatch>,
}

impl FixtureQueryExpectedMatches {
    pub fn assert_expected_matches(&self, output_mode: OutputMode) {
        let mut args = vec![
            "--query-source",
            &self.query_source,
            "--language",
            &self.language,
        ];
        if output_mode == OutputMode::Vimgrep {
            args.push("--vimgrep");
        }
        assert_expected_matches(
            args,
            get_fixture_dir_path_from_name(&self.fixture_dir_name),
            &self.expected_matches,
            output_mode,
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
                relative_file_path: "src/helpers.rs".to_owned(),
                lines: vec!["pub fn helper() {}".to_owned()],
                range: ((0, 0), (0, 18)).into(),
            },
            ExpectedMatch {
                relative_file_path: "src/lib.rs".to_owned(),
                lines: vec![
                    "pub fn add(left: usize, right: usize) -> usize {".to_owned(),
                    "    left + right".to_owned(),
                    "}".to_owned(),
                ],
                range: ((2, 0), (5, 0)).into(),
            },
            ExpectedMatch {
                relative_file_path: "src/lib.rs".to_owned(),
                lines: vec![
                    "    fn it_works() {".to_owned(),
                    "        let result = add(2, 2);".to_owned(),
                    "        assert_eq!(result, 4);".to_owned(),
                    "    }".to_owned(),
                ],
                range: ((11, 4), (14, 4)).into(),
            },
        ],
    });

#[test]
fn test_query_inline() {
    RUST_PROJECT_FUNCTION_ITEM_EXPECTED_MATCHES.assert_expected_matches(OutputMode::Normal);
}

#[test]
fn test_vimgrep_mode() {
    RUST_PROJECT_FUNCTION_ITEM_EXPECTED_MATCHES.assert_expected_matches(OutputMode::Vimgrep);
}
