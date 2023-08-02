// derived from https://github.com/BurntSushi/ripgrep/blob/master/crates/searcher/src/searcher/mod.rs

use std::{
    cell::{Ref, RefCell},
    cmp, fmt,
    fs::File,
    io::{self, Read},
    ops,
    path::Path,
};

use encoding_rs_io::DecodeReaderBytesBuilder;
use memmap::Mmap;
use streaming_iterator::StreamingIterator;
use tree_sitter::{QueryMatch, Tree};

pub use self::mmap::MmapChoice;
use crate::{
    get_matches,
    line_buffer::{alloc_error, DEFAULT_BUFFER_CAPACITY},
    matcher::{LineTerminator, Match},
    query_context::QueryContext,
    searcher::glue::MultiLine,
    sink::{Sink, SinkError},
    RopeOrSlice,
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
    #[allow(dead_code)]
    fn max_context(&self) -> usize {
        cmp::max(self.before_context, self.after_context)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigError {
    SearchUnavailable,
    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn line_terminator(&mut self, line_term: LineTerminator) -> &mut SearcherBuilder {
        self.config.line_term = line_term;
        self
    }

    #[allow(dead_code)]
    pub fn invert_match(&mut self, yes: bool) -> &mut SearcherBuilder {
        self.config.invert_match = yes;
        self
    }

    pub fn line_number(&mut self, yes: bool) -> &mut SearcherBuilder {
        self.config.line_number = yes;
        self
    }

    #[allow(dead_code)]
    pub fn after_context(&mut self, line_count: usize) -> &mut SearcherBuilder {
        self.config.after_context = line_count;
        self
    }

    #[allow(dead_code)]
    pub fn before_context(&mut self, line_count: usize) -> &mut SearcherBuilder {
        self.config.before_context = line_count;
        self
    }

    #[allow(dead_code)]
    pub fn passthru(&mut self, yes: bool) -> &mut SearcherBuilder {
        self.config.passthru = yes;
        self
    }

    #[allow(dead_code)]
    pub fn heap_limit(&mut self, bytes: Option<usize>) -> &mut SearcherBuilder {
        self.config.heap_limit = bytes;
        self
    }

    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub fn new() -> Searcher {
        SearcherBuilder::new().build()
    }

    pub fn search_path<P, S>(
        &mut self,
        query_context: QueryContext,
        path: P,
        write_to: S,
    ) -> Result<(), S::Error>
    where
        P: AsRef<Path>,
        S: Sink,
    {
        let path = path.as_ref();
        let file = File::open(path).map_err(S::Error::error_io)?;
        self.search_file_maybe_path(query_context, Some(path), &file, write_to)
    }

    pub fn load_file_contents<P, TError: SinkError>(
        &mut self,
        path: P,
    ) -> Result<MmapOrRefByteVec, TError>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        let file = File::open(path).map_err(TError::error_io)?;

        if let Some(mmap) = self.config.mmap.open(&file, Some(path)) {
            return Ok(mmap.into());
        }

        self.fill_multi_line_buffer_from_file(&file)
            .map_err(TError::error_io)?;
        return Ok(self.multi_line_buffer.borrow().into());
    }

    pub fn search_path_callback<P, TError: SinkError>(
        &mut self,
        query_context: QueryContext,
        path: P,
        callback: impl FnMut(&QueryMatch, &[u8], &Path),
    ) -> Result<(), TError>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        let file = File::open(path).map_err(TError::error_io)?;

        if let Some(mmap) = self.config.mmap.open(&file, Some(path)) {
            log::trace!("{:?}: searching via memory map", path);
            return self
                .search_slice_callback(query_context, &mmap, callback, path)
                .map_err(TError::error_config);
        }
        log::trace!("{:?}: reading entire file on to heap for mulitline", path);
        self.fill_multi_line_buffer_from_file(&file)
            .map_err(TError::error_io)?;
        log::trace!("{:?}: searching via multiline strategy", path);
        self.run_with_callback(
            query_context,
            &self.multi_line_buffer.borrow(),
            callback,
            path,
        );

        Ok(())
    }

    #[allow(dead_code)]
    pub fn search_file<S>(
        &mut self,
        query_context: QueryContext,
        file: &File,
        write_to: S,
    ) -> Result<(), S::Error>
    where
        S: Sink,
    {
        self.search_file_maybe_path(query_context, None, file, write_to)
    }

    fn search_file_maybe_path<S>(
        &mut self,
        query_context: QueryContext,
        path: Option<&Path>,
        file: &File,
        write_to: S,
    ) -> Result<(), S::Error>
    where
        S: Sink,
    {
        if let Some(mmap) = self.config.mmap.open(file, path) {
            log::trace!("{:?}: searching via memory map", path);
            return self.search_slice(query_context, &mmap, write_to);
        }
        log::trace!("{:?}: reading entire file on to heap for mulitline", path);

        self.fill_multi_line_buffer_from_file(file)
            .map_err(S::Error::error_io)?;
        log::trace!("{:?}: searching via multiline strategy", path);
        MultiLine::new(
            self,
            query_context,
            &self.multi_line_buffer.borrow(),
            write_to,
        )
        .run()
    }

    #[allow(dead_code)]
    pub fn search_reader<R, S>(
        &mut self,
        query_context: QueryContext,
        read_from: R,
        write_to: S,
    ) -> Result<(), S::Error>
    where
        R: io::Read,
        S: Sink,
    {
        self.check_config().map_err(S::Error::error_config)?;

        let mut decode_buffer = self.decode_buffer.borrow_mut();
        let decoder = self
            .decode_builder
            .build_with_buffer(read_from, &mut *decode_buffer)
            .map_err(S::Error::error_io)?;

        log::trace!("generic reader: reading everything to heap for multiline");
        self.fill_multi_line_buffer_from_reader(decoder)
            .map_err(S::Error::error_io)?;
        log::trace!("generic reader: searching via multiline strategy");
        MultiLine::new(
            self,
            query_context,
            &self.multi_line_buffer.borrow(),
            write_to,
        )
        .run()
    }

    pub fn search_slice<S>(
        &mut self,
        query_context: QueryContext,
        slice: &[u8],
        write_to: S,
    ) -> Result<(), S::Error>
    where
        S: Sink,
    {
        self.check_config().map_err(S::Error::error_config)?;

        log::trace!("slice reader: searching via multiline strategy");
        MultiLine::new(self, query_context, slice, write_to).run()
    }

    fn search_slice_callback(
        &mut self,
        query_context: QueryContext,
        slice: &[u8],
        callback: impl FnMut(&QueryMatch, &[u8], &Path),
        path: &Path,
    ) -> Result<(), ConfigError> {
        self.check_config()?;

        log::trace!("slice reader: searching via multiline strategy");
        self.run_with_callback(query_context, slice, callback, path);

        Ok(())
    }

    pub fn search_slice_callback_no_path<'a, 'text, 'tree>(
        &mut self,
        query_context: QueryContext,
        // slice: impl TextProvider<'a> + Parseable + 'a,
        slice: impl Into<RopeOrSlice<'text>>,
        tree: Option<&'tree Tree>,
        mut callback: impl FnMut(&QueryMatch),
    ) -> Result<(), ConfigError> {
        self.check_config()?;

        log::trace!("slice reader: searching via multiline strategy");
        get_matches(query_context.language, slice, &query_context.query, tree).for_each(
            |query_match| {
                callback(query_match);
            },
        );

        Ok(())
    }

    fn run_with_callback(
        &self,
        query_context: QueryContext,
        slice: &[u8],
        mut callback: impl FnMut(&QueryMatch, &[u8], &Path),
        path: &Path,
    ) {
        get_matches(query_context.language, slice, &query_context.query, None).for_each(|match_| {
            callback(match_, slice, path);
        });
    }

    fn check_config(&self) -> Result<(), ConfigError> {
        if self.config.heap_limit == Some(0) && !self.config.mmap.is_enabled() {
            return Err(ConfigError::SearchUnavailable);
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
    #[allow(dead_code)]
    pub fn invert_match(&self) -> bool {
        self.config.invert_match
    }

    #[inline]
    #[allow(dead_code)]
    pub fn line_number(&self) -> bool {
        self.config.line_number
    }

    #[inline]
    pub fn after_context(&self) -> usize {
        self.config.after_context
    }

    #[inline]
    #[allow(dead_code)]
    pub fn before_context(&self) -> usize {
        self.config.before_context
    }

    #[inline]
    #[allow(dead_code)]
    pub fn passthru(&self) -> bool {
        self.config.passthru
    }

    fn fill_multi_line_buffer_from_file(&self, file: &File) -> io::Result<()> {
        let mut decode_buffer = self.decode_buffer.borrow_mut();
        let mut read_from = self
            .decode_builder
            .build_with_buffer(file, &mut *decode_buffer)?;

        if self.config.heap_limit.is_none() {
            let mut buf = self.multi_line_buffer.borrow_mut();
            buf.clear();
            let cap = file.metadata().map(|m| m.len() as usize + 1).unwrap_or(0);
            buf.reserve(cap);
            read_from.read_to_end(&mut buf)?;
            return Ok(());
        }
        self.fill_multi_line_buffer_from_reader(read_from)
    }

    fn fill_multi_line_buffer_from_reader<R: io::Read>(&self, mut read_from: R) -> io::Result<()> {
        let mut buf = self.multi_line_buffer.borrow_mut();
        buf.clear();

        let heap_limit = match self.config.heap_limit {
            Some(heap_limit) => heap_limit,
            None => {
                read_from.read_to_end(&mut buf)?;
                return Ok(());
            }
        };
        if heap_limit == 0 {
            return Err(alloc_error(heap_limit));
        }

        buf.resize(cmp::min(DEFAULT_BUFFER_CAPACITY, heap_limit), 0);
        let mut pos = 0;
        loop {
            let nread = match read_from.read(&mut buf[pos..]) {
                Ok(nread) => nread,
                Err(ref err) if err.kind() == io::ErrorKind::Interrupted => {
                    continue;
                }
                Err(err) => return Err(err),
            };
            if nread == 0 {
                buf.resize(pos, 0);
                return Ok(());
            }

            pos += nread;
            if buf[pos..].is_empty() {
                let additional = heap_limit - buf.len();
                if additional == 0 {
                    return Err(alloc_error(heap_limit));
                }
                let limit = buf.len() + additional;
                let doubled = 2 * buf.len();
                buf.resize(cmp::min(doubled, limit), 0);
            }
        }
    }
}

pub enum MmapOrRefByteVec<'a> {
    Mmap(Mmap),
    RefByteVec(Ref<'a, Vec<u8>>),
}

impl<'a> From<Mmap> for MmapOrRefByteVec<'a> {
    fn from(value: Mmap) -> Self {
        Self::Mmap(value)
    }
}

impl<'a> From<Ref<'a, Vec<u8>>> for MmapOrRefByteVec<'a> {
    fn from(value: Ref<'a, Vec<u8>>) -> Self {
        Self::RefByteVec(value)
    }
}

impl<'a> ops::Deref for MmapOrRefByteVec<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Mmap(value) => value,
            Self::RefByteVec(value) => value,
        }
    }
}
