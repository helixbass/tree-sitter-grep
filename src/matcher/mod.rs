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

#[derive(Clone, Debug)]
pub struct ByteSet(BitSet);

#[derive(Clone, Copy)]
struct BitSet([u64; 4]);

impl fmt::Debug for BitSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut fmtd = f.debug_set();
        for b in (0..256).map(|b| b as u8) {
            if ByteSet(*self).contains(b) {
                fmtd.entry(&b);
            }
        }
        fmtd.finish()
    }
}

impl ByteSet {
    pub fn empty() -> ByteSet {
        ByteSet(BitSet([0; 4]))
    }

    pub fn full() -> ByteSet {
        ByteSet(BitSet([u64::MAX; 4]))
    }

    pub fn add(&mut self, byte: u8) {
        let bucket = byte / 64;
        let bit = byte % 64;
        (self.0).0[bucket as usize] |= 1 << bit;
    }

    pub fn add_all(&mut self, start: u8, end: u8) {
        for b in (start as u64..end as u64 + 1).map(|b| b as u8) {
            self.add(b);
        }
    }

    pub fn remove(&mut self, byte: u8) {
        let bucket = byte / 64;
        let bit = byte % 64;
        (self.0).0[bucket as usize] &= !(1 << bit);
    }

    pub fn remove_all(&mut self, start: u8, end: u8) {
        for b in (start as u64..end as u64 + 1).map(|b| b as u8) {
            self.remove(b);
        }
    }

    pub fn contains(&self, byte: u8) -> bool {
        let bucket = byte / 64;
        let bit = byte % 64;
        (self.0).0[bucket as usize] & (1 << bit) > 0
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

pub trait Matcher {
    type Error: fmt::Display;

    fn find_at(&self, haystack: &[u8], at: usize) -> Result<Option<Match>, Self::Error>;

    fn find(&self, haystack: &[u8]) -> Result<Option<Match>, Self::Error> {
        self.find_at(haystack, 0)
    }

    fn find_iter<F>(&self, haystack: &[u8], matched: F) -> Result<(), Self::Error>
    where
        F: FnMut(Match) -> bool,
    {
        self.find_iter_at(haystack, 0, matched)
    }

    fn find_iter_at<F>(&self, haystack: &[u8], at: usize, mut matched: F) -> Result<(), Self::Error>
    where
        F: FnMut(Match) -> bool,
    {
        self.try_find_iter_at(haystack, at, |m| Ok(matched(m)))
            .map(|r: Result<(), ()>| r.unwrap())
    }

    fn try_find_iter<F, E>(&self, haystack: &[u8], matched: F) -> Result<Result<(), E>, Self::Error>
    where
        F: FnMut(Match) -> Result<bool, E>,
    {
        self.try_find_iter_at(haystack, 0, matched)
    }

    fn try_find_iter_at<F, E>(
        &self,
        haystack: &[u8],
        at: usize,
        mut matched: F,
    ) -> Result<Result<(), E>, Self::Error>
    where
        F: FnMut(Match) -> Result<bool, E>,
    {
        let mut last_end = at;
        let mut last_match = None;

        loop {
            if last_end > haystack.len() {
                return Ok(Ok(()));
            }
            let m = match self.find_at(haystack, last_end)? {
                None => return Ok(Ok(())),
                Some(m) => m,
            };
            if m.start == m.end {
                last_end = m.end + 1;
                if Some(m.end) == last_match {
                    continue;
                }
            } else {
                last_end = m.end;
            }
            last_match = Some(m.end);
            match matched(m) {
                Ok(true) => continue,
                Ok(false) => return Ok(Ok(())),
                Err(err) => return Ok(Err(err)),
            }
        }
    }

    fn is_match(&self, haystack: &[u8]) -> Result<bool, Self::Error> {
        self.is_match_at(haystack, 0)
    }

    fn is_match_at(&self, haystack: &[u8], at: usize) -> Result<bool, Self::Error> {
        Ok(self.shortest_match_at(haystack, at)?.is_some())
    }

    fn shortest_match(&self, haystack: &[u8]) -> Result<Option<usize>, Self::Error> {
        self.shortest_match_at(haystack, 0)
    }

    fn shortest_match_at(&self, haystack: &[u8], at: usize) -> Result<Option<usize>, Self::Error> {
        Ok(self.find_at(haystack, at)?.map(|m| m.end))
    }

    fn non_matching_bytes(&self) -> Option<&ByteSet> {
        None
    }

    fn line_terminator(&self) -> Option<LineTerminator> {
        None
    }

    fn find_candidate_line(&self, haystack: &[u8]) -> Result<Option<LineMatchKind>, Self::Error> {
        Ok(self.shortest_match(haystack)?.map(LineMatchKind::Confirmed))
    }
}

impl<'a, M: Matcher> Matcher for &'a M {
    type Error = M::Error;

    fn find_at(&self, haystack: &[u8], at: usize) -> Result<Option<Match>, Self::Error> {
        (*self).find_at(haystack, at)
    }

    fn find(&self, haystack: &[u8]) -> Result<Option<Match>, Self::Error> {
        (*self).find(haystack)
    }

    fn find_iter<F>(&self, haystack: &[u8], matched: F) -> Result<(), Self::Error>
    where
        F: FnMut(Match) -> bool,
    {
        (*self).find_iter(haystack, matched)
    }

    fn find_iter_at<F>(&self, haystack: &[u8], at: usize, matched: F) -> Result<(), Self::Error>
    where
        F: FnMut(Match) -> bool,
    {
        (*self).find_iter_at(haystack, at, matched)
    }

    fn try_find_iter<F, E>(&self, haystack: &[u8], matched: F) -> Result<Result<(), E>, Self::Error>
    where
        F: FnMut(Match) -> Result<bool, E>,
    {
        (*self).try_find_iter(haystack, matched)
    }

    fn try_find_iter_at<F, E>(
        &self,
        haystack: &[u8],
        at: usize,
        matched: F,
    ) -> Result<Result<(), E>, Self::Error>
    where
        F: FnMut(Match) -> Result<bool, E>,
    {
        (*self).try_find_iter_at(haystack, at, matched)
    }

    fn is_match(&self, haystack: &[u8]) -> Result<bool, Self::Error> {
        (*self).is_match(haystack)
    }

    fn is_match_at(&self, haystack: &[u8], at: usize) -> Result<bool, Self::Error> {
        (*self).is_match_at(haystack, at)
    }

    fn shortest_match(&self, haystack: &[u8]) -> Result<Option<usize>, Self::Error> {
        (*self).shortest_match(haystack)
    }

    fn shortest_match_at(&self, haystack: &[u8], at: usize) -> Result<Option<usize>, Self::Error> {
        (*self).shortest_match_at(haystack, at)
    }

    fn non_matching_bytes(&self) -> Option<&ByteSet> {
        (*self).non_matching_bytes()
    }

    fn line_terminator(&self) -> Option<LineTerminator> {
        (*self).line_terminator()
    }

    fn find_candidate_line(&self, haystack: &[u8]) -> Result<Option<LineMatchKind>, Self::Error> {
        (*self).find_candidate_line(haystack)
    }
}
