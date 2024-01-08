// derived from https://github.com/BurntSushi/ripgrep/blob/master/crates/searcher/src/searcher/glue.rs

use streaming_iterator::StreamingIterator;

use crate::{
    lines::{self, LineStep},
    query_context::QueryContext,
    searcher::{core::Core, Config, Range, Searcher},
    sink::Sink,
    treesitter::get_captures,
    CaptureInfo,
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
            let query_context = self.core.query_context();
            let query = query_context.query.clone();
            let filter = query_context.filter.clone();
            let mut matches = get_captures(
                query_context.language,
                self.slice,
                &query,
                query_context.capture_index,
                filter.as_deref(),
                None,
            );
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
        matches: &mut impl StreamingIterator<Item = CaptureInfo<'tree>>,
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
        matches: &mut impl StreamingIterator<Item = CaptureInfo<'tree>>,
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
        matches: &mut impl StreamingIterator<Item = CaptureInfo<'tree>>,
    ) -> Result<Option<Range>, S::Error> {
        Ok(matches
            .next()
            .as_ref()
            .map(|capture_info| (&capture_info.node).into()))
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
