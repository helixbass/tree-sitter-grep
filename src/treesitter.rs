use tree_sitter::{Language, Node, Parser, Query, QueryCursor};

use crate::{matcher::Match, plugin::Filterer};

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

impl From<&'_ Node<'_>> for Match {
    fn from(node: &Node) -> Self {
        let range = node.range();

        Self::new(range.start_byte, range.end_byte)
    }
}
