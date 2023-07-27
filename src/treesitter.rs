use tree_sitter::{Language, Node, Parser, Query, QueryError, Tree};

use crate::matcher::Match;

pub(crate) fn get_parser(language: Language) -> Parser {
    let mut parser = Parser::new();
    parser
        .set_language(language)
        .expect("Error loading grammar");
    parser
}

pub(crate) fn maybe_get_query(source: &str, language: Language) -> Result<Query, QueryError> {
    Query::new(language, source)
}

impl From<&'_ Node<'_>> for Match {
    fn from(node: &Node) -> Self {
        let range = node.range();

        Self::new(range.start_byte, range.end_byte)
    }
}

pub trait Parseable {
    fn parse(&self, parser: &mut Parser, old_tree: Option<&Tree>) -> Option<Tree>;
}

impl<'a> Parseable for &'a [u8] {
    fn parse(&self, parser: &mut Parser, old_tree: Option<&Tree>) -> Option<Tree> {
        parser.parse(self, old_tree)
    }
}
