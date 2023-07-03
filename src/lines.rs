// derived from https://github.com/BurntSushi/ripgrep/blob/master/crates/searcher/src/lines.rs

use bstr::ByteSlice;

use crate::matcher::{LineTerminator, Match};

#[derive(Debug)]
pub struct LineIter<'b> {
    bytes: &'b [u8],
    stepper: LineStep,
}

impl<'b> LineIter<'b> {
    pub fn new(line_term: u8, bytes: &'b [u8]) -> LineIter<'b> {
        LineIter {
            bytes,
            stepper: LineStep::new(line_term, 0, bytes.len()),
        }
    }
}

impl<'b> Iterator for LineIter<'b> {
    type Item = &'b [u8];

    fn next(&mut self) -> Option<&'b [u8]> {
        self.stepper.next_match(self.bytes).map(|m| &self.bytes[m])
    }
}

#[derive(Debug)]
pub struct LineStep {
    line_term: u8,
    pos: usize,
    end: usize,
}

impl LineStep {
    pub fn new(line_term: u8, start: usize, end: usize) -> LineStep {
        LineStep {
            line_term,
            pos: start,
            end,
        }
    }

    pub fn next(&mut self, bytes: &[u8]) -> Option<(usize, usize)> {
        self.next_impl(bytes)
    }

    #[inline(always)]
    pub(crate) fn next_match(&mut self, bytes: &[u8]) -> Option<Match> {
        self.next_impl(bytes).map(|(s, e)| Match::new(s, e))
    }

    #[inline(always)]
    fn next_impl(&mut self, mut bytes: &[u8]) -> Option<(usize, usize)> {
        bytes = &bytes[..self.end];
        match bytes[self.pos..].find_byte(self.line_term) {
            None => {
                if self.pos < bytes.len() {
                    let m = (self.pos, bytes.len());
                    assert!(m.0 <= m.1);

                    self.pos = m.1;
                    Some(m)
                } else {
                    None
                }
            }
            Some(line_end) => {
                let m = (self.pos, self.pos + line_end + 1);
                assert!(m.0 <= m.1);

                self.pos = m.1;
                Some(m)
            }
        }
    }
}

pub fn count(bytes: &[u8], line_term: u8) -> u64 {
    bytecount::count(bytes, line_term) as u64
}

#[inline(always)]
#[allow(dead_code)]
pub fn without_terminator(bytes: &[u8], line_term: LineTerminator) -> &[u8] {
    let line_term = line_term.as_bytes();
    let start = bytes.len().saturating_sub(line_term.len());
    if bytes.get(start..) == Some(line_term) {
        return &bytes[..bytes.len() - line_term.len()];
    }
    bytes
}

#[inline(always)]
pub fn locate(bytes: &[u8], line_term: u8, range: Match) -> Match {
    let line_start = bytes[..range.start()]
        .rfind_byte(line_term)
        .map_or(0, |i| i + 1);
    let line_end = if range.end() > line_start && bytes[range.end() - 1] == line_term {
        range.end()
    } else {
        bytes[range.end()..]
            .find_byte(line_term)
            .map_or(bytes.len(), |i| range.end() + i + 1)
    };
    Match::new(line_start, line_end)
}

pub fn preceding(bytes: &[u8], line_term: u8, count: usize) -> usize {
    preceding_by_pos(bytes, bytes.len(), line_term, count)
}

fn preceding_by_pos(bytes: &[u8], mut pos: usize, line_term: u8, mut count: usize) -> usize {
    if pos == 0 {
        return 0;
    } else if bytes[pos - 1] == line_term {
        pos -= 1;
    }
    loop {
        match bytes[..pos].rfind_byte(line_term) {
            None => {
                return 0;
            }
            Some(i) => {
                if count == 0 {
                    return i + 1;
                } else if i == 0 {
                    return 0;
                }
                count -= 1;
                pos = i;
            }
        }
    }
}
