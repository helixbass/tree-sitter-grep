use std::{cmp, io};

use crate::{
    lines::{self, LineStep},
    matcher::Matcher,
    searcher::{core::Core, Config, Range, Searcher},
    sink::{Sink, SinkError},
};

#[derive(Debug)]
pub struct MultiLine<'s, M, S> {
    config: &'s Config,
    core: Core<'s, M, S>,
    slice: &'s [u8],
    last_match: Option<Range>,
}

impl<'s, M: Matcher, S: Sink> MultiLine<'s, M, S> {
    pub fn new(
        searcher: &'s Searcher,
        matcher: M,
        slice: &'s [u8],
        write_to: S,
    ) -> MultiLine<'s, M, S> {
        MultiLine {
            config: &searcher.config,
            core: Core::new(searcher, matcher, write_to),
            slice,
            last_match: None,
        }
    }

    pub fn run(mut self) -> Result<(), S::Error> {
        if self.core.begin()? {
            let mut keepgoing = true;
            while !self.slice[self.core.pos()..].is_empty() && keepgoing {
                keepgoing = self.sink()?;
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

    fn sink(&mut self) -> Result<bool, S::Error> {
        if self.config.invert_match {
            return self.sink_matched_inverted();
        }
        let mat = match self.find()? {
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
                Ok(true)
            }
            Some(last_match) => {
                if last_match.end() >= line.start() {
                    self.last_match = Some(last_match.with_end(line.end()));
                    Ok(true)
                } else {
                    self.last_match = Some(line);
                    if !self.sink_context(&last_match)? {
                        return Ok(false);
                    }
                    self.sink_matched(&last_match)
                }
            }
        }
    }

    fn sink_matched_inverted(&mut self) -> Result<bool, S::Error> {
        assert!(self.config.invert_match);

        let invert_match = match self.find()? {
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
        self.core.matched(self.slice, range)
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

    fn find(&mut self) -> Result<Option<Range>, S::Error> {
        match self.core.matcher().find(&self.slice[self.core.pos()..]) {
            Err(err) => Err(S::Error::error_message(err)),
            Ok(None) => Ok(None),
            Ok(Some(m)) => Ok(Some(m.offset(self.core.pos()))),
        }
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
