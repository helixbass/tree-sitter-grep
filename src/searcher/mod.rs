use std::{
    cell::RefCell,
    cmp, fmt,
    fs::File,
    io::{self, Read},
    path::Path,
};

use encoding_rs;
use encoding_rs_io::DecodeReaderBytesBuilder;

pub use self::mmap::MmapChoice;
use crate::{
    line_buffer::{alloc_error, DEFAULT_BUFFER_CAPACITY},
    matcher::{LineTerminator, Match, Matcher},
    searcher::glue::MultiLine,
    sink::{Sink, SinkError},
};

mod core;
mod glue;
mod mmap;

type Range = Match;

#[derive(Clone, Debug)]
pub struct Config {
    line_term: LineTerminator,
    invert_match: bool,
    after_context: usize,
    before_context: usize,
    passthru: bool,
    line_number: bool,
    heap_limit: Option<usize>,
    mmap: MmapChoice,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            line_term: LineTerminator::default(),
            invert_match: false,
            after_context: 0,
            before_context: 0,
            passthru: false,
            line_number: true,
            heap_limit: None,
            mmap: MmapChoice::default(),
        }
    }
}

impl Config {
    fn max_context(&self) -> usize {
        cmp::max(self.before_context, self.after_context)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigError {
    SearchUnavailable,
    MismatchedLineTerminators {
        matcher: LineTerminator,
        searcher: LineTerminator,
    },
    #[doc(hidden)]
    __Nonexhaustive,
}

impl ::std::error::Error for ConfigError {
    fn description(&self) -> &str {
        "searcher configuration error"
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ConfigError::SearchUnavailable => {
                write!(f, "searcher config error: no available searchers")
            }
            ConfigError::MismatchedLineTerminators { matcher, searcher } => {
                write!(
                    f,
                    "searcher config error: mismatched line terminators, \
                     matcher has {:?} but searcher has {:?}",
                    matcher, searcher
                )
            }
            _ => panic!("BUG: unexpected variant found"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SearcherBuilder {
    config: Config,
}

impl Default for SearcherBuilder {
    fn default() -> SearcherBuilder {
        SearcherBuilder::new()
    }
}

impl SearcherBuilder {
    pub fn new() -> SearcherBuilder {
        SearcherBuilder {
            config: Config::default(),
        }
    }

    pub fn build(&self) -> Searcher {
        let mut config = self.config.clone();
        if config.passthru {
            config.before_context = 0;
            config.after_context = 0;
        }

        let mut decode_builder = DecodeReaderBytesBuilder::new();
        decode_builder
            .encoding(None)
            .utf8_passthru(true)
            .bom_override(true);

        Searcher {
            config,
            decode_builder,
            decode_buffer: RefCell::new(vec![0; 8 * (1 << 10)]),
            multi_line_buffer: RefCell::new(vec![]),
        }
    }

    pub fn line_terminator(&mut self, line_term: LineTerminator) -> &mut SearcherBuilder {
        self.config.line_term = line_term;
        self
    }

    pub fn invert_match(&mut self, yes: bool) -> &mut SearcherBuilder {
        self.config.invert_match = yes;
        self
    }

    pub fn line_number(&mut self, yes: bool) -> &mut SearcherBuilder {
        self.config.line_number = yes;
        self
    }

    pub fn after_context(&mut self, line_count: usize) -> &mut SearcherBuilder {
        self.config.after_context = line_count;
        self
    }

    pub fn before_context(&mut self, line_count: usize) -> &mut SearcherBuilder {
        self.config.before_context = line_count;
        self
    }

    pub fn passthru(&mut self, yes: bool) -> &mut SearcherBuilder {
        self.config.passthru = yes;
        self
    }

    pub fn heap_limit(&mut self, bytes: Option<usize>) -> &mut SearcherBuilder {
        self.config.heap_limit = bytes;
        self
    }

    pub fn memory_map(&mut self, strategy: MmapChoice) -> &mut SearcherBuilder {
        self.config.mmap = strategy;
        self
    }
}

#[derive(Clone, Debug)]
pub struct Searcher {
    config: Config,
    decode_builder: DecodeReaderBytesBuilder,
    decode_buffer: RefCell<Vec<u8>>,
    multi_line_buffer: RefCell<Vec<u8>>,
}

impl Searcher {
    pub fn new() -> Searcher {
        SearcherBuilder::new().build()
    }

    pub fn search_path<P, M, S>(&mut self, matcher: M, path: P, write_to: S) -> Result<(), S::Error>
    where
        P: AsRef<Path>,
        M: Matcher,
        S: Sink,
    {
        let path = path.as_ref();
        let file = File::open(path).map_err(S::Error::error_io)?;
        self.search_file_maybe_path(matcher, Some(path), &file, write_to)
    }

    pub fn search_file<M, S>(
        &mut self,
        matcher: M,
        file: &File,
        write_to: S,
    ) -> Result<(), S::Error>
    where
        M: Matcher,
        S: Sink,
    {
        self.search_file_maybe_path(matcher, None, file, write_to)
    }

    fn search_file_maybe_path<M, S>(
        &mut self,
        matcher: M,
        path: Option<&Path>,
        file: &File,
        write_to: S,
    ) -> Result<(), S::Error>
    where
        M: Matcher,
        S: Sink,
    {
        if let Some(mmap) = self.config.mmap.open(file, path) {
            log::trace!("{:?}: searching via memory map", path);
            return self.search_slice(matcher, &mmap, write_to);
        }
        log::trace!("{:?}: reading entire file on to heap for mulitline", path);
        self.fill_multi_line_buffer_from_file::<S>(file)?;
        log::trace!("{:?}: searching via multiline strategy", path);
        MultiLine::new(self, matcher, &self.multi_line_buffer.borrow(), write_to).run()
    }

    pub fn search_reader<M, R, S>(
        &mut self,
        matcher: M,
        read_from: R,
        write_to: S,
    ) -> Result<(), S::Error>
    where
        M: Matcher,
        R: io::Read,
        S: Sink,
    {
        self.check_config(&matcher)
            .map_err(S::Error::error_config)?;

        let mut decode_buffer = self.decode_buffer.borrow_mut();
        let decoder = self
            .decode_builder
            .build_with_buffer(read_from, &mut *decode_buffer)
            .map_err(S::Error::error_io)?;

        log::trace!("generic reader: reading everything to heap for multiline");
        self.fill_multi_line_buffer_from_reader::<_, S>(decoder)?;
        log::trace!("generic reader: searching via multiline strategy");
        MultiLine::new(self, matcher, &self.multi_line_buffer.borrow(), write_to).run()
    }

    pub fn search_slice<M, S>(
        &mut self,
        matcher: M,
        slice: &[u8],
        write_to: S,
    ) -> Result<(), S::Error>
    where
        M: Matcher,
        S: Sink,
    {
        self.check_config(&matcher)
            .map_err(S::Error::error_config)?;

        log::trace!("slice reader: searching via multiline strategy");
        MultiLine::new(self, matcher, slice, write_to).run()
    }

    fn check_config<M: Matcher>(&self, matcher: M) -> Result<(), ConfigError> {
        if self.config.heap_limit == Some(0) && !self.config.mmap.is_enabled() {
            return Err(ConfigError::SearchUnavailable);
        }
        let matcher_line_term = match matcher.line_terminator() {
            None => return Ok(()),
            Some(line_term) => line_term,
        };
        if matcher_line_term != self.config.line_term {
            return Err(ConfigError::MismatchedLineTerminators {
                matcher: matcher_line_term,
                searcher: self.config.line_term,
            });
        }
        Ok(())
    }
}

impl Searcher {
    #[inline]
    pub fn line_terminator(&self) -> LineTerminator {
        self.config.line_term
    }

    #[inline]
    pub fn invert_match(&self) -> bool {
        self.config.invert_match
    }

    #[inline]
    pub fn line_number(&self) -> bool {
        self.config.line_number
    }

    #[inline]
    pub fn after_context(&self) -> usize {
        self.config.after_context
    }

    #[inline]
    pub fn before_context(&self) -> usize {
        self.config.before_context
    }

    #[inline]
    pub fn passthru(&self) -> bool {
        self.config.passthru
    }

    fn fill_multi_line_buffer_from_file<S: Sink>(&self, file: &File) -> Result<(), S::Error> {
        let mut decode_buffer = self.decode_buffer.borrow_mut();
        let mut read_from = self
            .decode_builder
            .build_with_buffer(file, &mut *decode_buffer)
            .map_err(S::Error::error_io)?;

        if self.config.heap_limit.is_none() {
            let mut buf = self.multi_line_buffer.borrow_mut();
            buf.clear();
            let cap = file.metadata().map(|m| m.len() as usize + 1).unwrap_or(0);
            buf.reserve(cap);
            read_from
                .read_to_end(&mut *buf)
                .map_err(S::Error::error_io)?;
            return Ok(());
        }
        self.fill_multi_line_buffer_from_reader::<_, S>(read_from)
    }

    fn fill_multi_line_buffer_from_reader<R: io::Read, S: Sink>(
        &self,
        mut read_from: R,
    ) -> Result<(), S::Error> {
        let mut buf = self.multi_line_buffer.borrow_mut();
        buf.clear();

        let heap_limit = match self.config.heap_limit {
            Some(heap_limit) => heap_limit,
            None => {
                read_from
                    .read_to_end(&mut *buf)
                    .map_err(S::Error::error_io)?;
                return Ok(());
            }
        };
        if heap_limit == 0 {
            return Err(S::Error::error_io(alloc_error(heap_limit)));
        }

        buf.resize(cmp::min(DEFAULT_BUFFER_CAPACITY, heap_limit), 0);
        let mut pos = 0;
        loop {
            let nread = match read_from.read(&mut buf[pos..]) {
                Ok(nread) => nread,
                Err(ref err) if err.kind() == io::ErrorKind::Interrupted => {
                    continue;
                }
                Err(err) => return Err(S::Error::error_io(err)),
            };
            if nread == 0 {
                buf.resize(pos, 0);
                return Ok(());
            }

            pos += nread;
            if buf[pos..].is_empty() {
                let additional = heap_limit - buf.len();
                if additional == 0 {
                    return Err(S::Error::error_io(alloc_error(heap_limit)));
                }
                let limit = buf.len() + additional;
                let doubled = 2 * buf.len();
                buf.resize(cmp::min(doubled, limit), 0);
            }
        }
    }
}
