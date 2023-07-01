use std::{fmt, io, ops, u64};

use interpolate::interpolate;

mod interpolate;

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

pub trait Captures {
    fn len(&self) -> usize;

    fn get(&self, i: usize) -> Option<Match>;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn interpolate<F>(
        &self,
        name_to_index: F,
        haystack: &[u8],
        replacement: &[u8],
        dst: &mut Vec<u8>,
    ) where
        F: FnMut(&str) -> Option<usize>,
    {
        interpolate(
            replacement,
            |i, dst| {
                if let Some(range) = self.get(i) {
                    dst.extend(&haystack[range]);
                }
            },
            name_to_index,
            dst,
        )
    }
}

#[derive(Clone, Debug)]
pub struct NoCaptures(());

impl NoCaptures {
    pub fn new() -> NoCaptures {
        NoCaptures(())
    }
}

impl Captures for NoCaptures {
    fn len(&self) -> usize {
        0
    }
    fn get(&self, _: usize) -> Option<Match> {
        None
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
    type Captures: Captures;

    type Error: fmt::Display;

    fn find_at(&self, haystack: &[u8], at: usize) -> Result<Option<Match>, Self::Error>;

    fn new_captures(&self) -> Result<Self::Captures, Self::Error>;

    fn capture_count(&self) -> usize {
        0
    }

    fn capture_index(&self, _name: &str) -> Option<usize> {
        None
    }

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

    fn captures(&self, haystack: &[u8], caps: &mut Self::Captures) -> Result<bool, Self::Error> {
        self.captures_at(haystack, 0, caps)
    }

    fn captures_iter<F>(
        &self,
        haystack: &[u8],
        caps: &mut Self::Captures,
        matched: F,
    ) -> Result<(), Self::Error>
    where
        F: FnMut(&Self::Captures) -> bool,
    {
        self.captures_iter_at(haystack, 0, caps, matched)
    }

    fn captures_iter_at<F>(
        &self,
        haystack: &[u8],
        at: usize,
        caps: &mut Self::Captures,
        mut matched: F,
    ) -> Result<(), Self::Error>
    where
        F: FnMut(&Self::Captures) -> bool,
    {
        self.try_captures_iter_at(haystack, at, caps, |caps| Ok(matched(caps)))
            .map(|r: Result<(), ()>| r.unwrap())
    }

    fn try_captures_iter<F, E>(
        &self,
        haystack: &[u8],
        caps: &mut Self::Captures,
        matched: F,
    ) -> Result<Result<(), E>, Self::Error>
    where
        F: FnMut(&Self::Captures) -> Result<bool, E>,
    {
        self.try_captures_iter_at(haystack, 0, caps, matched)
    }

    fn try_captures_iter_at<F, E>(
        &self,
        haystack: &[u8],
        at: usize,
        caps: &mut Self::Captures,
        mut matched: F,
    ) -> Result<Result<(), E>, Self::Error>
    where
        F: FnMut(&Self::Captures) -> Result<bool, E>,
    {
        let mut last_end = at;
        let mut last_match = None;

        loop {
            if last_end > haystack.len() {
                return Ok(Ok(()));
            }
            if !self.captures_at(haystack, last_end, caps)? {
                return Ok(Ok(()));
            }
            let m = caps.get(0).unwrap();
            if m.start == m.end {
                last_end = m.end + 1;
                if Some(m.end) == last_match {
                    continue;
                }
            } else {
                last_end = m.end;
            }
            last_match = Some(m.end);
            match matched(caps) {
                Ok(true) => continue,
                Ok(false) => return Ok(Ok(())),
                Err(err) => return Ok(Err(err)),
            }
        }
    }

    fn captures_at(
        &self,
        _haystack: &[u8],
        _at: usize,
        _caps: &mut Self::Captures,
    ) -> Result<bool, Self::Error> {
        Ok(false)
    }

    fn replace<F>(
        &self,
        haystack: &[u8],
        dst: &mut Vec<u8>,
        mut append: F,
    ) -> Result<(), Self::Error>
    where
        F: FnMut(Match, &mut Vec<u8>) -> bool,
    {
        let mut last_match = 0;
        self.find_iter(haystack, |m| {
            dst.extend(&haystack[last_match..m.start]);
            last_match = m.end;
            append(m, dst)
        })?;
        dst.extend(&haystack[last_match..]);
        Ok(())
    }

    fn replace_with_captures<F>(
        &self,
        haystack: &[u8],
        caps: &mut Self::Captures,
        dst: &mut Vec<u8>,
        append: F,
    ) -> Result<(), Self::Error>
    where
        F: FnMut(&Self::Captures, &mut Vec<u8>) -> bool,
    {
        self.replace_with_captures_at(haystack, 0, caps, dst, append)
    }

    fn replace_with_captures_at<F>(
        &self,
        haystack: &[u8],
        at: usize,
        caps: &mut Self::Captures,
        dst: &mut Vec<u8>,
        mut append: F,
    ) -> Result<(), Self::Error>
    where
        F: FnMut(&Self::Captures, &mut Vec<u8>) -> bool,
    {
        let mut last_match = at;
        self.captures_iter_at(haystack, at, caps, |caps| {
            let m = caps.get(0).unwrap();
            dst.extend(&haystack[last_match..m.start]);
            last_match = m.end;
            append(caps, dst)
        })?;
        dst.extend(&haystack[last_match..]);
        Ok(())
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
    type Captures = M::Captures;
    type Error = M::Error;

    fn find_at(&self, haystack: &[u8], at: usize) -> Result<Option<Match>, Self::Error> {
        (*self).find_at(haystack, at)
    }

    fn new_captures(&self) -> Result<Self::Captures, Self::Error> {
        (*self).new_captures()
    }

    fn captures_at(
        &self,
        haystack: &[u8],
        at: usize,
        caps: &mut Self::Captures,
    ) -> Result<bool, Self::Error> {
        (*self).captures_at(haystack, at, caps)
    }

    fn capture_index(&self, name: &str) -> Option<usize> {
        (*self).capture_index(name)
    }

    fn capture_count(&self) -> usize {
        (*self).capture_count()
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

    fn captures(&self, haystack: &[u8], caps: &mut Self::Captures) -> Result<bool, Self::Error> {
        (*self).captures(haystack, caps)
    }

    fn captures_iter<F>(
        &self,
        haystack: &[u8],
        caps: &mut Self::Captures,
        matched: F,
    ) -> Result<(), Self::Error>
    where
        F: FnMut(&Self::Captures) -> bool,
    {
        (*self).captures_iter(haystack, caps, matched)
    }

    fn captures_iter_at<F>(
        &self,
        haystack: &[u8],
        at: usize,
        caps: &mut Self::Captures,
        matched: F,
    ) -> Result<(), Self::Error>
    where
        F: FnMut(&Self::Captures) -> bool,
    {
        (*self).captures_iter_at(haystack, at, caps, matched)
    }

    fn try_captures_iter<F, E>(
        &self,
        haystack: &[u8],
        caps: &mut Self::Captures,
        matched: F,
    ) -> Result<Result<(), E>, Self::Error>
    where
        F: FnMut(&Self::Captures) -> Result<bool, E>,
    {
        (*self).try_captures_iter(haystack, caps, matched)
    }

    fn try_captures_iter_at<F, E>(
        &self,
        haystack: &[u8],
        at: usize,
        caps: &mut Self::Captures,
        matched: F,
    ) -> Result<Result<(), E>, Self::Error>
    where
        F: FnMut(&Self::Captures) -> Result<bool, E>,
    {
        (*self).try_captures_iter_at(haystack, at, caps, matched)
    }

    fn replace<F>(&self, haystack: &[u8], dst: &mut Vec<u8>, append: F) -> Result<(), Self::Error>
    where
        F: FnMut(Match, &mut Vec<u8>) -> bool,
    {
        (*self).replace(haystack, dst, append)
    }

    fn replace_with_captures<F>(
        &self,
        haystack: &[u8],
        caps: &mut Self::Captures,
        dst: &mut Vec<u8>,
        append: F,
    ) -> Result<(), Self::Error>
    where
        F: FnMut(&Self::Captures, &mut Vec<u8>) -> bool,
    {
        (*self).replace_with_captures(haystack, caps, dst, append)
    }

    fn replace_with_captures_at<F>(
        &self,
        haystack: &[u8],
        at: usize,
        caps: &mut Self::Captures,
        dst: &mut Vec<u8>,
        append: F,
    ) -> Result<(), Self::Error>
    where
        F: FnMut(&Self::Captures, &mut Vec<u8>) -> bool,
    {
        (*self).replace_with_captures_at(haystack, at, caps, dst, append)
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
