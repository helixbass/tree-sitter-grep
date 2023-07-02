use tree_sitter::{Language, Node, Parser, Query};

use crate::matcher::Match;

pub(crate) fn get_parser(language: Language) -> Parser {
    let mut parser = Parser::new();
    parser
        .set_language(language)
        .expect("Error loading grammar");
    parser
}

pub(crate) fn maybe_get_query(source: &str, language: Language) -> Option<Query> {
    Query::new(language, source).ok()
}

pub(crate) fn node_to_match(node: &Node) -> Match {
    let range = node.range();

    Match::new(range.start_byte, range.end_byte)
}
