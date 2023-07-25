use std::iter;

use ouroboros::self_referencing;
use ropey::{iter::Chunks, Rope, RopeSlice};
use tree_sitter::{Language, Node, Parser, Query, QueryError, TextProvider, Tree};

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

impl<'a> Parseable for &'a Rope {
    fn parse(&self, parser: &mut Parser, old_tree: Option<&Tree>) -> Option<Tree> {
        parser.parse_with(
            &mut |byte_offset, _| {
                let (chunk, chunk_start_byte_index, _, _) = self.chunk_at_byte(byte_offset);
                &chunk[byte_offset - chunk_start_byte_index..]
            },
            old_tree,
        )
    }
}

#[derive(Copy, Clone)]
pub enum RopeOrSlice<'a> {
    Slice(&'a [u8]),
    Rope(&'a Rope),
}

impl<'a> TextProvider<'a> for RopeOrSlice<'a> {
    type I = RopeOrSliceTextProviderIterator<'a>;

    fn text(&mut self, node: Node) -> Self::I {
        match self {
            Self::Slice(slice) => {
                RopeOrSliceTextProviderIterator::Slice(iter::once(&slice[node.byte_range()]))
            }
            Self::Rope(rope) => {
                let rope_slice = rope.byte_slice(node.byte_range());
                RopeOrSliceTextProviderIterator::Rope(RopeOrSliceRopeTextProviderIterator::new(
                    rope_slice,
                    |rope_slice| rope_slice.chunks(),
                ))
            }
        }
    }
}

impl<'a> Parseable for RopeOrSlice<'a> {
    fn parse(&self, parser: &mut Parser, old_tree: Option<&Tree>) -> Option<Tree> {
        match self {
            Self::Slice(slice) => slice.parse(parser, old_tree),
            Self::Rope(rope) => rope.parse(parser, old_tree),
        }
    }
}

impl<'a> From<&'a [u8]> for RopeOrSlice<'a> {
    fn from(value: &'a [u8]) -> Self {
        Self::Slice(value)
    }
}

impl<'a> From<&'a Rope> for RopeOrSlice<'a> {
    fn from(value: &'a Rope) -> Self {
        Self::Rope(value)
    }
}

pub enum RopeOrSliceTextProviderIterator<'a> {
    Slice(iter::Once<&'a [u8]>),
    Rope(RopeOrSliceRopeTextProviderIterator<'a>),
}

impl<'a> Iterator for RopeOrSliceTextProviderIterator<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Slice(slice_iterator) => slice_iterator.next(),
            Self::Rope(rope_iterator) => rope_iterator.next().map(str::as_bytes),
        }
    }
}

#[self_referencing]
pub struct RopeOrSliceRopeTextProviderIterator<'a> {
    rope_slice: RopeSlice<'a>,

    #[borrows(rope_slice)]
    chunks_iterator: Chunks<'a>,
}

impl<'a> Iterator for RopeOrSliceRopeTextProviderIterator<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        self.with_chunks_iterator_mut(|chunks_iterator| chunks_iterator.next())
    }
}
