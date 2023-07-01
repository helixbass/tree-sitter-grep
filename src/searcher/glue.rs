use tree_sitter::{Node, QueryCursor};

use crate::{
    lines::{self, LineStep},
    plugin::get_loaded_filter,
    query_context::QueryContext,
    searcher::{core::Core, Config, Range, Searcher},
    sink::Sink,
    treesitter::{get_parser, node_to_match},
};

#[derive(Debug, Default)]
struct AccumulatedExactMatches {
    matches_with_offsets_relative_to_reference_beginning_of_line_offset: Vec<Range>,
    reference_beginning_of_line_offset: Option<usize>,
}

impl AccumulatedExactMatches {
    pub fn clear(&mut self) {
        self.matches_with_offsets_relative_to_reference_beginning_of_line_offset
            .clear();
        self.reference_beginning_of_line_offset = None;
    }

    pub fn push(
        &mut self,
        match_with_absolute_offsets: Range,
        current_beginning_of_line_offset: usize,
    ) {
        if self.reference_beginning_of_line_offset.is_none() {
            self.reference_beginning_of_line_offset = Some(current_beginning_of_line_offset);
        }
        self.matches_with_offsets_relative_to_reference_beginning_of_line_offset
            .push(Range::new(
                match_with_absolute_offsets.start()
                    - self.reference_beginning_of_line_offset.unwrap(),
                match_with_absolute_offsets.end()
                    - self.reference_beginning_of_line_offset.unwrap(),
            ));
    }
}

impl AsRef<[Range]> for AccumulatedExactMatches {
    fn as_ref(&self) -> &[Range] {
        &self.matches_with_offsets_relative_to_reference_beginning_of_line_offset
    }
}

#[derive(Debug)]
pub struct MultiLine<'s, S> {
    config: &'s Config,
    core: Core<'s, S>,
    slice: &'s [u8],
    last_match: Option<Range>,
    accumulated_exact_matches: AccumulatedExactMatches,
}

impl<'s, S: Sink> MultiLine<'s, S> {
    pub fn new(
        searcher: &'s Searcher,
        query_context: QueryContext,
        slice: &'s [u8],
        write_to: S,
    ) -> MultiLine<'s, S> {
        MultiLine {
            config: &searcher.config,
            core: Core::new(searcher, query_context, write_to),
            slice,
            last_match: None,
            accumulated_exact_matches: Default::default(),
        }
    }

    pub fn run(mut self) -> Result<(), S::Error> {
        if self.core.begin()? {
            let mut keepgoing = true;
            let mut query_cursor = QueryCursor::new();
            let tree = get_parser(self.core.query_context().language)
                .parse(self.slice, None)
                .unwrap();
            let filter = get_loaded_filter(
                self.core.query_context().filter_library_path.as_deref(),
                self.core.query_context().filter_arg.as_deref(),
            );
            let query = self.core.query_context().query.clone();
            let capture_index = self.core.query_context().capture_index;
            let mut matches = query_cursor
                .captures(&query, tree.root_node(), self.slice)
                .filter_map(|(match_, found_capture_index)| {
                    let found_capture_index = found_capture_index as u32;
                    if found_capture_index != capture_index {
                        return None;
                    }
                    let mut nodes_for_this_capture = match_.nodes_for_capture_index(capture_index);
                    let single_captured_node = nodes_for_this_capture.next().unwrap();
                    assert!(
                        nodes_for_this_capture.next().is_none(),
                        "I guess .captures() always wraps up the single capture like this?"
                    );
                    match filter {
                        None => Some(single_captured_node),
                        Some(filter) => filter
                            .call(&single_captured_node)
                            .then_some(single_captured_node),
                    }
                });
            while !self.slice[self.core.pos()..].is_empty() && keepgoing {
                keepgoing = self.sink(&mut matches)?;
            }
            if keepgoing {
                keepgoing = match self.last_match.take() {
                    None => true,
                    Some(last_match) => {
                        if self.sink_context(&last_match)? {
                            self.sink_matched(&last_match)?;
                        }
                        true
                    }
                };
            }
            if keepgoing {
                if self.config.passthru {
                    self.core
                        .other_context_by_line(self.slice, self.slice.len())?;
                } else {
                    self.core
                        .after_context_by_line(self.slice, self.slice.len())?;
                }
            }
        }
        let byte_count = self.byte_count();
        self.core.finish(byte_count)
    }

    fn sink<'tree>(
        &mut self,
        matches: &mut impl Iterator<Item = Node<'tree>>,
    ) -> Result<bool, S::Error> {
        if self.config.invert_match {
            return self.sink_matched_inverted(matches);
        }
        let mat = match self.find(matches)? {
            Some(range) => range,
            None => {
                self.core.set_pos(self.slice.len());
                return Ok(true);
            }
        };
        self.advance(&mat);

        let line = lines::locate(self.slice, self.config.line_term.as_byte(), mat);
        match self.last_match.take() {
            None => {
                self.last_match = Some(line);
                self.accumulated_exact_matches.push(mat, line.start());
                Ok(true)
            }
            Some(last_match) => {
                if last_match.end() >= line.start() {
                    self.last_match = Some(last_match.with_end_if_extends(line.end()));
                    self.accumulated_exact_matches.push(mat, line.start());
                    Ok(true)
                } else {
                    self.last_match = Some(line);
                    if !self.sink_context(&last_match)? {
                        return Ok(false);
                    }
                    let ret = self.sink_matched(&last_match);
                    self.accumulated_exact_matches.push(mat, line.start());
                    ret
                }
            }
        }
    }

    fn sink_matched_inverted<'tree>(
        &mut self,
        matches: &mut impl Iterator<Item = Node<'tree>>,
    ) -> Result<bool, S::Error> {
        assert!(self.config.invert_match);

        let invert_match = match self.find(matches)? {
            None => {
                let range = Range::new(self.core.pos(), self.slice.len());
                self.core.set_pos(range.end());
                range
            }
            Some(mat) => {
                let line = lines::locate(self.slice, self.config.line_term.as_byte(), mat);
                let range = Range::new(self.core.pos(), line.start());
                self.advance(&line);
                range
            }
        };
        if invert_match.is_empty() {
            return Ok(true);
        }
        if !self.sink_context(&invert_match)? {
            return Ok(false);
        }
        let mut stepper = LineStep::new(
            self.config.line_term.as_byte(),
            invert_match.start(),
            invert_match.end(),
        );
        while let Some(line) = stepper.next_match(self.slice) {
            if !self.sink_matched(&line)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn sink_matched(&mut self, range: &Range) -> Result<bool, S::Error> {
        if range.is_empty() {
            return Ok(false);
        }
        let ret = self
            .core
            .matched(self.slice, range, self.accumulated_exact_matches.as_ref());
        self.accumulated_exact_matches.clear();
        ret
    }

    fn sink_context(&mut self, range: &Range) -> Result<bool, S::Error> {
        if self.config.passthru {
            if !self.core.other_context_by_line(self.slice, range.start())? {
                return Ok(false);
            }
        } else {
            if !self.core.after_context_by_line(self.slice, range.start())? {
                return Ok(false);
            }
            if !self
                .core
                .before_context_by_line(self.slice, range.start())?
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn find<'tree>(
        &mut self,
        matches: &mut impl Iterator<Item = Node<'tree>>,
    ) -> Result<Option<Range>, S::Error> {
        Ok(matches.next().as_ref().map(node_to_match))
    }

    fn advance(&mut self, range: &Range) {
        self.core.set_pos(range.end());
        if range.is_empty() && self.core.pos() < self.slice.len() {
            let newpos = self.core.pos() + 1;
            self.core.set_pos(newpos);
        }
    }

    fn byte_count(&mut self) -> u64 {
        self.core.pos() as u64
    }
}
