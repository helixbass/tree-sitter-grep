use crate::{
    lines::{self, LineStep},
    query_context::QueryContext,
    searcher::{Config, Range, Searcher},
    sink::{Sink, SinkContext, SinkContextKind, SinkFinish, SinkMatch},
};

#[derive(Debug)]
pub struct Core<'s, S> {
    config: &'s Config,
    query_context: QueryContext,
    searcher: &'s Searcher,
    sink: S,
    pos: usize,
    absolute_byte_offset: u64,
    line_number: Option<u64>,
    last_line_counted: usize,
    last_line_visited: usize,
    after_context_left: usize,
    has_sunk: bool,
}

impl<'s, S: Sink> Core<'s, S> {
    pub fn new(searcher: &'s Searcher, query_context: QueryContext, sink: S) -> Core<'s, S> {
        let line_number = if searcher.config.line_number {
            Some(1)
        } else {
            None
        };
        Core {
            config: &searcher.config,
            query_context,
            searcher,
            sink,
            pos: 0,
            absolute_byte_offset: 0,
            line_number,
            last_line_counted: 0,
            last_line_visited: 0,
            after_context_left: 0,
            has_sunk: false,
        }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
    }

    pub fn query_context(&self) -> &QueryContext {
        &self.query_context
    }

    pub fn matched(
        &mut self,
        buf: &[u8],
        range: &Range,
        exact_matches: &[Range],
    ) -> Result<bool, S::Error> {
        self.sink_matched(buf, range, exact_matches)
    }

    pub fn begin(&mut self) -> Result<bool, S::Error> {
        self.sink.begin(self.searcher)
    }

    pub fn finish(&mut self, byte_count: u64) -> Result<(), S::Error> {
        self.sink.finish(self.searcher, &SinkFinish { byte_count })
    }

    pub fn before_context_by_line(&mut self, buf: &[u8], upto: usize) -> Result<bool, S::Error> {
        if self.config.before_context == 0 {
            return Ok(true);
        }
        let range = Range::new(self.last_line_visited, upto);
        if range.is_empty() {
            return Ok(true);
        }
        let before_context_start = range.start()
            + lines::preceding(
                &buf[range],
                self.config.line_term.as_byte(),
                self.config.before_context - 1,
            );

        let range = Range::new(before_context_start, range.end());
        let mut stepper =
            LineStep::new(self.config.line_term.as_byte(), range.start(), range.end());
        while let Some(line) = stepper.next_match(buf) {
            if !self.sink_break_context(line.start())? {
                return Ok(false);
            }
            if !self.sink_before_context(buf, &line)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub fn after_context_by_line(&mut self, buf: &[u8], upto: usize) -> Result<bool, S::Error> {
        if self.after_context_left == 0 {
            return Ok(true);
        }
        let range = Range::new(self.last_line_visited, upto);
        let mut stepper =
            LineStep::new(self.config.line_term.as_byte(), range.start(), range.end());
        while let Some(line) = stepper.next_match(buf) {
            if !self.sink_after_context(buf, &line)? {
                return Ok(false);
            }
            if self.after_context_left == 0 {
                break;
            }
        }
        Ok(true)
    }

    pub fn other_context_by_line(&mut self, buf: &[u8], upto: usize) -> Result<bool, S::Error> {
        let range = Range::new(self.last_line_visited, upto);
        let mut stepper =
            LineStep::new(self.config.line_term.as_byte(), range.start(), range.end());
        while let Some(line) = stepper.next_match(buf) {
            if !self.sink_other_context(buf, &line)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    #[inline(always)]
    fn sink_matched(
        &mut self,
        buf: &[u8],
        range: &Range,
        exact_matches: &[Range],
    ) -> Result<bool, S::Error> {
        if !self.sink_break_context(range.start())? {
            return Ok(false);
        }
        self.count_lines(buf, range.start());
        let offset = self.absolute_byte_offset + range.start() as u64;
        let linebuf = &buf[*range];
        let keepgoing = self.sink.matched(
            self.searcher,
            &SinkMatch {
                line_term: self.config.line_term,
                bytes: linebuf,
                absolute_byte_offset: offset,
                line_number: self.line_number,
                buffer: buf,
                bytes_range_in_buffer: range.start()..range.end(),
                exact_matches,
            },
        )?;
        if !keepgoing {
            return Ok(false);
        }
        self.last_line_visited = range.end();
        self.after_context_left = self.config.after_context;
        self.has_sunk = true;
        Ok(true)
    }

    fn sink_before_context(&mut self, buf: &[u8], range: &Range) -> Result<bool, S::Error> {
        self.count_lines(buf, range.start());
        let offset = self.absolute_byte_offset + range.start() as u64;
        let keepgoing = self.sink.context(
            self.searcher,
            &SinkContext {
                #[cfg(test)]
                line_term: self.config.line_term,
                bytes: &buf[*range],
                kind: SinkContextKind::Before,
                absolute_byte_offset: offset,
                line_number: self.line_number,
            },
        )?;
        if !keepgoing {
            return Ok(false);
        }
        self.last_line_visited = range.end();
        self.has_sunk = true;
        Ok(true)
    }

    fn sink_after_context(&mut self, buf: &[u8], range: &Range) -> Result<bool, S::Error> {
        assert!(self.after_context_left >= 1);

        self.count_lines(buf, range.start());
        let offset = self.absolute_byte_offset + range.start() as u64;
        let keepgoing = self.sink.context(
            self.searcher,
            &SinkContext {
                #[cfg(test)]
                line_term: self.config.line_term,
                bytes: &buf[*range],
                kind: SinkContextKind::After,
                absolute_byte_offset: offset,
                line_number: self.line_number,
            },
        )?;
        if !keepgoing {
            return Ok(false);
        }
        self.last_line_visited = range.end();
        self.after_context_left -= 1;
        self.has_sunk = true;
        Ok(true)
    }

    fn sink_other_context(&mut self, buf: &[u8], range: &Range) -> Result<bool, S::Error> {
        self.count_lines(buf, range.start());
        let offset = self.absolute_byte_offset + range.start() as u64;
        let keepgoing = self.sink.context(
            self.searcher,
            &SinkContext {
                #[cfg(test)]
                line_term: self.config.line_term,
                bytes: &buf[*range],
                kind: SinkContextKind::Other,
                absolute_byte_offset: offset,
                line_number: self.line_number,
            },
        )?;
        if !keepgoing {
            return Ok(false);
        }
        self.last_line_visited = range.end();
        self.has_sunk = true;
        Ok(true)
    }

    fn sink_break_context(&mut self, start_of_line: usize) -> Result<bool, S::Error> {
        let is_gap = self.last_line_visited < start_of_line;
        let any_context = self.config.before_context > 0 || self.config.after_context > 0;

        if !any_context || !self.has_sunk || !is_gap {
            Ok(true)
        } else {
            self.sink.context_break(self.searcher)
        }
    }

    fn count_lines(&mut self, buf: &[u8], upto: usize) {
        if let Some(ref mut line_number) = self.line_number {
            if self.last_line_counted >= upto {
                return;
            }
            let slice = &buf[self.last_line_counted..upto];
            let count = lines::count(slice, self.config.line_term.as_byte());
            *line_number += count;
            self.last_line_counted = upto;
        }
    }
}
