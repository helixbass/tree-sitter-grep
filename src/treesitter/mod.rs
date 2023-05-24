use crate::regex;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use tree_sitter::{Language, Parser, Point, Query, QueryCursor};

pub fn get_rust_language() -> Language {
    tree_sitter_rust::language()
}

pub fn get_parser() -> Parser {
    let mut parser = Parser::new();
    parser
        .set_language(get_rust_language())
        .expect("Error loading Rust grammar");
    parser
}

pub fn get_query(source: &str) -> Query {
    Query::new(get_rust_language(), source).unwrap()
}

pub struct Result {
    pub point: Point,
    pub line_text: String,
    pub file_path: PathBuf,
}

impl Result {
    pub fn format(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            format_path(&self.file_path),
            self.point.row + 1,
            self.point.column + 1,
            self.line_text,
        )
    }
}

fn format_path(path: &Path) -> String {
    regex!(r#"^\./"#)
        .replace(&format!("{}", path.display()), "")
        .into_owned()
}

pub fn get_results(query: &Query, file_path: impl AsRef<Path>, capture_index: u32) -> Vec<Result> {
    let mut query_cursor = QueryCursor::new();
    let file_path = file_path.as_ref();
    let file_text = fs::read_to_string(file_path).unwrap();
    let tree = get_parser().parse(&file_text, None).unwrap();
    query_cursor
        .matches(query, tree.root_node(), file_text.as_bytes())
        .flat_map(|match_| {
            match_
                .nodes_for_capture_index(capture_index)
                .collect::<Vec<_>>()
        })
        .map(|node| {
            let point = node.start_position();
            Result {
                point,
                line_text: {
                    let file = File::open(file_path).unwrap();
                    BufReader::new(file)
                        .lines()
                        .nth(point.row)
                        .unwrap()
                        .unwrap()
                },
                file_path: file_path.to_owned(),
            }
        })
        .collect()
}
