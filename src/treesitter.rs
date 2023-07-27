#![allow(clippy::too_many_arguments)]

use std::{borrow::Cow, iter, mem};

use ouroboros::self_referencing;
use ropey::{iter::Chunks, Rope, RopeSlice};
use streaming_iterator::StreamingIterator;
use tree_sitter::{
    Language, Node, Parser, Query, QueryCaptures, QueryCursor, QueryError, TextProvider, Tree,
};

use crate::{matcher::Match, plugin::Filterer};

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

impl<'a> TextProvider<'a> for &'a RopeOrSlice<'a> {
    type I = RopeOrSliceTextProviderIterator<'a>;

    fn text(&mut self, node: Node) -> Self::I {
        match self {
            RopeOrSlice::Slice(slice) => {
                RopeOrSliceTextProviderIterator::Slice(iter::once(&slice[node.byte_range()]))
            }
            RopeOrSlice::Rope(rope) => {
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

impl<'a> Parseable for &'a RopeOrSlice<'a> {
    fn parse(&self, parser: &mut Parser, old_tree: Option<&Tree>) -> Option<Tree> {
        match self {
            RopeOrSlice::Slice(slice) => slice.parse(parser, old_tree),
            RopeOrSlice::Rope(rope) => rope.parse(parser, old_tree),
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

// I believe this type can't be Copy/Clone in order for the
// `get_captures()` unsafe stuff to be sound
pub struct CaptureInfo<'a> {
    pub node: Node<'a>,
    pub pattern_index: usize,
}

#[self_referencing]
pub struct Captures<'a, 'text: 'a, 'tree: 'a> {
    text: RopeOrSlice<'text>,
    query_cursor: QueryCursor,
    query: &'a Query,
    filter: Option<&'a Filterer>,
    tree: Cow<'tree, Tree>,
    capture_index: u32,
    #[borrows(text, mut query_cursor, query, tree)]
    #[covariant]
    captures_iterator: QueryCaptures<'this, 'this, 'this, RopeOrSlice<'this>>,
    #[borrows(tree)]
    #[covariant]
    next_capture: Option<CaptureInfo<'this>>,
}

pub fn get_captures<'a, 'text, 'tree>(
    language: Language,
    // text: impl TextProvider<'a> + Parseable,
    text: impl Into<RopeOrSlice<'text>>,
    query: &'a Query,
    capture_index: u32,
    filter: Option<&'a Filterer>,
    tree: Option<&'tree Tree>,
) -> Captures<'a, 'text, 'tree> {
    let text = text.into();
    let query_cursor = QueryCursor::new();
    let tree: Cow<'tree, Tree> = tree.map_or_else(
        || Cow::Owned(text.parse(&mut get_parser(language), None).unwrap()),
        Cow::Borrowed,
    );
    Captures::new(
        text,
        query_cursor,
        query,
        filter,
        tree,
        capture_index,
        |text, query_cursor, query, tree| query_cursor.captures(query, tree.root_node(), *text),
        |_| None,
    )
}

impl<'a, 'text, 'tree> StreamingIterator for Captures<'a, 'text, 'tree> {
    type Item = CaptureInfo<'tree>;

    fn advance(&mut self) {
        self.with_mut(|all_fields| {
            for (match_, index_into_query_match_captures) in all_fields.captures_iterator.by_ref() {
                let this_capture = &match_.captures[index_into_query_match_captures];
                if this_capture.index != *all_fields.capture_index {
                    continue;
                }
                let single_captured_node = this_capture.node;
                if all_fields
                    .filter
                    .as_ref()
                    .map_or(true, |filter| filter.call(&single_captured_node))
                {
                    *all_fields.next_capture = Some(CaptureInfo {
                        node: single_captured_node,
                        pattern_index: match_.pattern_index,
                    });
                    return;
                }
            }
            *all_fields.next_capture = None;
        });
    }

    fn get<'this>(&'this self) -> Option<&'this Self::Item> {
        let next_capture = self.borrow_next_capture();
        // SAFETY: I think this is ok as long as CaptureInfo isn't
        // Copy/Clone?
        // Since at that point there's no way for the "inner"
        // CaptureInfo's contents to "outlive" the returned reference?
        // Did this because otherwise was running into not being able
        // to express that the "real" Item type for this trait (I think)
        // should be CaptureInfo<'this>, not CaptureInfo<'a>
        let next_capture: &'this Option<CaptureInfo<'tree>> =
            unsafe { mem::transmute(next_capture) };
        next_capture.as_ref()
    }
}
