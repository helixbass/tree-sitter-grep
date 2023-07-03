use std::{fmt, io, ops, u64};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Match {
    start: usize,
    end: usize,
}

impl Match {
    #[inline]
    pub fn new(start: usize, end: usize) -> Match {
        assert!(start <= end);
        Match { start, end }
    }

    #[inline]
    pub fn zero(offset: usize) -> Match {
        Match {
            start: offset,
            end: offset,
        }
    }

    #[inline]
    pub fn start(&self) -> usize {
        self.start
    }

    #[inline]
    pub fn end(&self) -> usize {
        self.end
    }

    #[inline]
    pub fn with_start(&self, start: usize) -> Match {
        assert!(start <= self.end, "{} is not <= {}", start, self.end);
        Match { start, ..*self }
    }

    #[inline]
    pub fn with_end(&self, end: usize) -> Match {
        assert!(self.start <= end, "{} is not <= {}", self.start, end);
        Match { end, ..*self }
    }

    #[inline]
    pub fn with_end_if_extends(&self, end: usize) -> Match {
        assert!(self.start <= end, "{} is not <= {}", self.start, end);
        if end > self.end {
            Match { end, ..*self }
        } else {
            *self
        }
    }

    #[inline]
    pub fn offset(&self, amount: usize) -> Match {
        Match {
            start: self.start.checked_add(amount).unwrap(),
            end: self.end.checked_add(amount).unwrap(),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl ops::Index<Match> for [u8] {
    type Output = [u8];

    #[inline]
    fn index(&self, index: Match) -> &[u8] {
        &self[index.start..index.end]
    }
}

impl ops::IndexMut<Match> for [u8] {
    #[inline]
    fn index_mut(&mut self, index: Match) -> &mut [u8] {
        &mut self[index.start..index.end]
    }
}

impl ops::Index<Match> for str {
    type Output = str;

    #[inline]
    fn index(&self, index: Match) -> &str {
        &self[index.start..index.end]
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LineTerminator(LineTerminatorImp);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum LineTerminatorImp {
    Byte([u8; 1]),
    CRLF,
}

impl LineTerminator {
    #[inline]
    pub fn byte(byte: u8) -> LineTerminator {
        LineTerminator(LineTerminatorImp::Byte([byte]))
    }

    #[inline]
    pub fn crlf() -> LineTerminator {
        LineTerminator(LineTerminatorImp::CRLF)
    }

    #[inline]
    pub fn is_crlf(&self) -> bool {
        self.0 == LineTerminatorImp::CRLF
    }

    #[inline]
    pub fn as_byte(&self) -> u8 {
        match self.0 {
            LineTerminatorImp::Byte(array) => array[0],
            LineTerminatorImp::CRLF => b'\n',
        }
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        match self.0 {
            LineTerminatorImp::Byte(ref array) => array,
            LineTerminatorImp::CRLF => &[b'\r', b'\n'],
        }
    }

    #[inline]
    pub fn is_suffix(&self, slice: &[u8]) -> bool {
        slice.last().map_or(false, |&b| b == self.as_byte())
    }
}

impl Default for LineTerminator {
    #[inline]
    fn default() -> LineTerminator {
        LineTerminator::byte(b'\n')
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct NoError(());

impl ::std::error::Error for NoError {
    fn description(&self) -> &str {
        "no error"
    }
}

impl fmt::Display for NoError {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        panic!("BUG for NoError: an impossible error occurred")
    }
}

impl From<NoError> for io::Error {
    fn from(_: NoError) -> io::Error {
        panic!("BUG for NoError: an impossible error occurred")
    }
}

#[derive(Clone, Copy, Debug)]
pub enum LineMatchKind {
    Confirmed(usize),
    Candidate(usize),
}
