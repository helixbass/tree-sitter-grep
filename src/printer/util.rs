use std::{borrow::Cow, fmt, io, path::Path, time};

use bstr::{ByteSlice, ByteVec};
use serde::{Serialize, Serializer};

use crate::{
    lines::LineIter,
    matcher::{Captures, LineTerminator, Match, Matcher},
    searcher::Searcher,
    sink::{SinkContext, SinkContextKind, SinkError, SinkMatch},
};

pub struct Replacer<M: Matcher> {
    space: Option<Space<M>>,
}

struct Space<M: Matcher> {
    caps: M::Captures,
    dst: Vec<u8>,
    matches: Vec<Match>,
}

impl<M: Matcher> fmt::Debug for Replacer<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (dst, matches) = self.replacement().unwrap_or((&[], &[]));
        f.debug_struct("Replacer")
            .field("dst", &dst)
            .field("matches", &matches)
            .finish()
    }
}

impl<M: Matcher> Replacer<M> {
    pub fn new() -> Replacer<M> {
        Replacer { space: None }
    }

    pub fn replace_all<'a>(
        &'a mut self,
        searcher: &Searcher,
        matcher: &M,
        mut subject: &[u8],
        range: std::ops::Range<usize>,
        replacement: &[u8],
    ) -> io::Result<()> {
        todo!();
        // {
        //     let &mut Space { ref mut dst, ref mut caps, ref mut matches } =
        //         self.allocate(matcher)?;
        //     dst.clear();
        //     matches.clear();

        //     replace_with_captures_in_context(
        //         matcher,
        //         subject,
        //         range.clone(),
        //         caps,
        //         dst,
        //         |caps, dst| {
        //             let start = dst.len();
        //             caps.interpolate(
        //                 |name| matcher.capture_index(name),
        //                 subject,
        //                 replacement,
        //                 dst,
        //             );
        //             let end = dst.len();
        //             matches.push(Match::new(start, end));
        //             true
        //         },
        //     )
        //     .map_err(io::Error::error_message)?;
        // }
        Ok(())
    }

    pub fn replacement<'a>(&'a self) -> Option<(&'a [u8], &'a [Match])> {
        match self.space {
            None => None,
            Some(ref space) => {
                if space.matches.is_empty() {
                    None
                } else {
                    Some((&space.dst, &space.matches))
                }
            }
        }
    }

    pub fn clear(&mut self) {
        if let Some(ref mut space) = self.space {
            space.dst.clear();
            space.matches.clear();
        }
    }

    fn allocate(&mut self, matcher: &M) -> io::Result<&mut Space<M>> {
        if self.space.is_none() {
            let caps = matcher.new_captures().map_err(io::Error::error_message)?;
            self.space = Some(Space {
                caps,
                dst: vec![],
                matches: vec![],
            });
        }
        Ok(self.space.as_mut().unwrap())
    }
}

#[derive(Debug)]
pub struct Sunk<'a> {
    bytes: &'a [u8],
    absolute_byte_offset: u64,
    line_number: Option<u64>,
    context_kind: Option<&'a SinkContextKind>,
    matches: &'a [Match],
    original_matches: &'a [Match],
}

impl<'a> Sunk<'a> {
    #[inline]
    pub fn empty() -> Sunk<'static> {
        Sunk {
            bytes: &[],
            absolute_byte_offset: 0,
            line_number: None,
            context_kind: None,
            matches: &[],
            original_matches: &[],
        }
    }

    #[inline]
    pub fn from_sink_match(
        sunk: &'a SinkMatch<'a>,
        original_matches: &'a [Match],
        replacement: Option<(&'a [u8], &'a [Match])>,
    ) -> Sunk<'a> {
        let (bytes, matches) = replacement.unwrap_or_else(|| (sunk.bytes(), original_matches));
        Sunk {
            bytes,
            absolute_byte_offset: sunk.absolute_byte_offset(),
            line_number: sunk.line_number(),
            context_kind: None,
            matches,
            original_matches,
        }
    }

    #[inline]
    pub fn from_sink_context(
        sunk: &'a SinkContext<'a>,
        original_matches: &'a [Match],
        replacement: Option<(&'a [u8], &'a [Match])>,
    ) -> Sunk<'a> {
        let (bytes, matches) = replacement.unwrap_or_else(|| (sunk.bytes(), original_matches));
        Sunk {
            bytes,
            absolute_byte_offset: sunk.absolute_byte_offset(),
            line_number: sunk.line_number(),
            context_kind: Some(sunk.kind()),
            matches,
            original_matches,
        }
    }

    #[inline]
    pub fn context_kind(&self) -> Option<&'a SinkContextKind> {
        self.context_kind
    }

    #[inline]
    pub fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

    #[inline]
    pub fn matches(&self) -> &'a [Match] {
        self.matches
    }

    #[inline]
    pub fn original_matches(&self) -> &'a [Match] {
        self.original_matches
    }

    #[inline]
    pub fn lines(&self, line_term: u8) -> LineIter<'a> {
        LineIter::new(line_term, self.bytes())
    }

    #[inline]
    pub fn absolute_byte_offset(&self) -> u64 {
        self.absolute_byte_offset
    }

    #[inline]
    pub fn line_number(&self) -> Option<u64> {
        self.line_number
    }
}

#[derive(Clone, Debug)]
pub struct PrinterPath<'a>(Cow<'a, [u8]>);

impl<'a> PrinterPath<'a> {
    pub fn new(path: &'a Path) -> PrinterPath<'a> {
        PrinterPath(Vec::from_path_lossy(path))
    }

    pub fn with_separator(path: &'a Path, sep: Option<u8>) -> PrinterPath<'a> {
        let mut ppath = PrinterPath::new(path);
        if let Some(sep) = sep {
            ppath.replace_separator(sep);
        }
        ppath
    }

    fn replace_separator(&mut self, new_sep: u8) {
        let transformed_path: Vec<u8> = self
            .0
            .bytes()
            .map(|b| {
                if b == b'/' || (cfg!(windows) && b == b'\\') {
                    new_sep
                } else {
                    b
                }
            })
            .collect();
        self.0 = Cow::Owned(transformed_path);
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NiceDuration(pub time::Duration);

impl fmt::Display for NiceDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:0.6}s", self.fractional_seconds())
    }
}

impl NiceDuration {
    fn fractional_seconds(&self) -> f64 {
        let fractional = (self.0.subsec_nanos() as f64) / 1_000_000_000.0;
        self.0.as_secs() as f64 + fractional
    }
}

impl Serialize for NiceDuration {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;

        let mut state = ser.serialize_struct("Duration", 2)?;
        state.serialize_field("secs", &self.0.as_secs())?;
        state.serialize_field("nanos", &self.0.subsec_nanos())?;
        state.serialize_field("human", &format!("{}", self))?;
        state.end()
    }
}

pub fn trim_ascii_prefix(line_term: LineTerminator, slice: &[u8], range: Match) -> Match {
    fn is_space(b: u8) -> bool {
        match b {
            b'\t' | b'\n' | b'\x0B' | b'\x0C' | b'\r' | b' ' => true,
            _ => false,
        }
    }

    let count = slice[range]
        .iter()
        .take_while(|&&b| -> bool { is_space(b) && !line_term.as_bytes().contains(&b) })
        .count();
    range.with_start(range.start() + count)
}

pub fn trim_line_terminator(searcher: &Searcher, buf: &[u8], line: &mut Match) {
    let lineterm = searcher.line_terminator();
    if lineterm.is_suffix(&buf[*line]) {
        let mut end = line.end() - 1;
        if lineterm.is_crlf() && end > 0 && buf.get(end - 1) == Some(&b'\r') {
            end -= 1;
        }
        *line = line.with_end(end);
    }
}
