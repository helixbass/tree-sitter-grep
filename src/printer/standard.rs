use std::{
    cell::{Cell, RefCell},
    cmp,
    io::{self, Write},
    path::Path,
    sync::Arc,
    time::Instant,
};

use bstr::ByteSlice;
use termcolor::{ColorSpec, NoColor, WriteColor};

use super::{
    color::ColorSpecs,
    counter::CounterWriter,
    stats::Stats,
    util::{trim_ascii_prefix, trim_line_terminator, PrinterPath, Replacer, Sunk},
};
use crate::{
    lines::LineStep,
    matcher::{Match, Matcher},
    searcher::Searcher,
    sink::{Sink, SinkContext, SinkContextKind, SinkFinish, SinkMatch},
};

#[derive(Debug, Clone)]
struct Config {
    colors: ColorSpecs,
    stats: bool,
    heading: bool,
    path: bool,
    only_matching: bool,
    per_match: bool,
    per_match_one_line: bool,
    replacement: Arc<Option<Vec<u8>>>,
    max_columns: Option<u64>,
    max_columns_preview: bool,
    max_matches: Option<u64>,
    column: bool,
    byte_offset: bool,
    trim_ascii: bool,
    separator_search: Arc<Option<Vec<u8>>>,
    separator_context: Arc<Option<Vec<u8>>>,
    separator_field_match: Arc<Vec<u8>>,
    separator_field_context: Arc<Vec<u8>>,
    separator_path: Option<u8>,
    path_terminator: Option<u8>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            colors: ColorSpecs::default(),
            stats: false,
            heading: false,
            path: true,
            only_matching: false,
            per_match: false,
            per_match_one_line: false,
            replacement: Arc::new(None),
            max_columns: None,
            max_columns_preview: false,
            max_matches: None,
            column: false,
            byte_offset: false,
            trim_ascii: false,
            separator_search: Arc::new(None),
            separator_context: Arc::new(Some(b"--".to_vec())),
            separator_field_match: Arc::new(b":".to_vec()),
            separator_field_context: Arc::new(b"-".to_vec()),
            separator_path: None,
            path_terminator: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct StandardBuilder {
    config: Config,
}

impl StandardBuilder {
    pub fn new() -> StandardBuilder {
        StandardBuilder {
            config: Config::default(),
        }
    }

    pub fn build<W: WriteColor>(&self, wtr: W) -> Standard<W> {
        Standard {
            config: self.config.clone(),
            wtr: RefCell::new(CounterWriter::new(wtr)),
            matches: vec![],
        }
    }

    pub fn build_no_color<W: io::Write>(&self, wtr: W) -> Standard<NoColor<W>> {
        self.build(NoColor::new(wtr))
    }

    pub fn color_specs(&mut self, specs: ColorSpecs) -> &mut StandardBuilder {
        self.config.colors = specs;
        self
    }

    pub fn stats(&mut self, yes: bool) -> &mut StandardBuilder {
        self.config.stats = yes;
        self
    }

    pub fn heading(&mut self, yes: bool) -> &mut StandardBuilder {
        self.config.heading = yes;
        self
    }

    pub fn path(&mut self, yes: bool) -> &mut StandardBuilder {
        self.config.path = yes;
        self
    }

    pub fn only_matching(&mut self, yes: bool) -> &mut StandardBuilder {
        self.config.only_matching = yes;
        self
    }

    pub fn per_match(&mut self, yes: bool) -> &mut StandardBuilder {
        self.config.per_match = yes;
        self
    }

    pub fn per_match_one_line(&mut self, yes: bool) -> &mut StandardBuilder {
        self.config.per_match_one_line = yes;
        self
    }

    pub fn replacement(&mut self, replacement: Option<Vec<u8>>) -> &mut StandardBuilder {
        self.config.replacement = Arc::new(replacement);
        self
    }

    pub fn max_columns(&mut self, limit: Option<u64>) -> &mut StandardBuilder {
        self.config.max_columns = limit;
        self
    }

    pub fn max_columns_preview(&mut self, yes: bool) -> &mut StandardBuilder {
        self.config.max_columns_preview = yes;
        self
    }

    pub fn max_matches(&mut self, limit: Option<u64>) -> &mut StandardBuilder {
        self.config.max_matches = limit;
        self
    }

    pub fn column(&mut self, yes: bool) -> &mut StandardBuilder {
        self.config.column = yes;
        self
    }

    pub fn byte_offset(&mut self, yes: bool) -> &mut StandardBuilder {
        self.config.byte_offset = yes;
        self
    }

    pub fn trim_ascii(&mut self, yes: bool) -> &mut StandardBuilder {
        self.config.trim_ascii = yes;
        self
    }

    pub fn separator_search(&mut self, sep: Option<Vec<u8>>) -> &mut StandardBuilder {
        self.config.separator_search = Arc::new(sep);
        self
    }

    pub fn separator_context(&mut self, sep: Option<Vec<u8>>) -> &mut StandardBuilder {
        self.config.separator_context = Arc::new(sep);
        self
    }

    pub fn separator_field_match(&mut self, sep: Vec<u8>) -> &mut StandardBuilder {
        self.config.separator_field_match = Arc::new(sep);
        self
    }

    pub fn separator_field_context(&mut self, sep: Vec<u8>) -> &mut StandardBuilder {
        self.config.separator_field_context = Arc::new(sep);
        self
    }

    pub fn separator_path(&mut self, sep: Option<u8>) -> &mut StandardBuilder {
        self.config.separator_path = sep;
        self
    }

    pub fn path_terminator(&mut self, terminator: Option<u8>) -> &mut StandardBuilder {
        self.config.path_terminator = terminator;
        self
    }
}

#[derive(Debug)]
pub struct Standard<W> {
    config: Config,
    wtr: RefCell<CounterWriter<W>>,
    matches: Vec<Match>,
}

impl<W: WriteColor> Standard<W> {
    pub fn new(wtr: W) -> Standard<W> {
        StandardBuilder::new().build(wtr)
    }
}

impl<W: io::Write> Standard<NoColor<W>> {
    pub fn new_no_color(wtr: W) -> Standard<NoColor<W>> {
        StandardBuilder::new().build_no_color(wtr)
    }
}

impl<W: WriteColor> Standard<W> {
    pub fn sink<'s, M: Matcher>(&'s mut self, matcher: M) -> StandardSink<'static, 's, M, W> {
        let stats = if self.config.stats {
            Some(Stats::new())
        } else {
            None
        };
        let needs_match_granularity = self.needs_match_granularity();
        StandardSink {
            matcher,
            standard: self,
            replacer: Replacer::new(),
            path: None,
            start_time: Instant::now(),
            match_count: 0,
            after_context_remaining: 0,
            stats,
            needs_match_granularity,
        }
    }

    pub fn sink_with_path<'p, 's, M, P>(
        &'s mut self,
        matcher: M,
        path: &'p P,
    ) -> StandardSink<'p, 's, M, W>
    where
        M: Matcher,
        P: ?Sized + AsRef<Path>,
    {
        if !self.config.path {
            return self.sink(matcher);
        }
        let stats = if self.config.stats {
            Some(Stats::new())
        } else {
            None
        };
        let ppath = PrinterPath::with_separator(path.as_ref(), self.config.separator_path);
        let needs_match_granularity = self.needs_match_granularity();
        StandardSink {
            matcher,
            standard: self,
            replacer: Replacer::new(),
            path: Some(ppath),
            start_time: Instant::now(),
            match_count: 0,
            after_context_remaining: 0,
            stats,
            needs_match_granularity,
        }
    }

    fn needs_match_granularity(&self) -> bool {
        let supports_color = self.wtr.borrow().supports_color();
        let match_colored = !self.config.colors.matched().is_none();

        (supports_color && match_colored)
            || self.config.column
            || self.config.replacement.is_some()
            || self.config.per_match
            || self.config.only_matching
            || self.config.stats
    }
}

impl<W> Standard<W> {
    pub fn has_written(&self) -> bool {
        self.wtr.borrow().total_count() > 0
    }

    pub fn get_mut(&mut self) -> &mut W {
        self.wtr.get_mut().get_mut()
    }

    pub fn into_inner(self) -> W {
        self.wtr.into_inner().into_inner()
    }
}

#[derive(Debug)]
pub struct StandardSink<'p, 's, M: Matcher, W> {
    matcher: M,
    standard: &'s mut Standard<W>,
    replacer: Replacer<M>,
    path: Option<PrinterPath<'p>>,
    start_time: Instant,
    match_count: u64,
    after_context_remaining: u64,
    stats: Option<Stats>,
    needs_match_granularity: bool,
}

impl<'p, 's, M: Matcher, W: WriteColor> StandardSink<'p, 's, M, W> {
    pub fn has_match(&self) -> bool {
        self.match_count > 0
    }

    pub fn match_count(&self) -> u64 {
        self.match_count
    }

    pub fn stats(&self) -> Option<&Stats> {
        self.stats.as_ref()
    }

    fn record_matches(
        &mut self,
        searcher: &Searcher,
        bytes: &[u8],
        range: std::ops::Range<usize>,
    ) -> io::Result<()> {
        self.standard.matches.clear();
        if !self.needs_match_granularity {
            return Ok(());
        }
        let matches = &mut self.standard.matches;
        todo!();
        // find_iter_at_in_context(
        //     searcher,
        //     &self.matcher,
        //     bytes,
        //     range.clone(),
        //     |m| {
        //         let (s, e) = (m.start() - range.start, m.end() - range.start);
        //         matches.push(Match::new(s, e));
        //         true
        //     },
        // )?;
        if !matches.is_empty()
            && matches.last().unwrap().is_empty()
            && matches.last().unwrap().start() >= range.end
        {
            matches.pop().unwrap();
        }
        Ok(())
    }

    fn replace(
        &mut self,
        searcher: &Searcher,
        bytes: &[u8],
        range: std::ops::Range<usize>,
    ) -> io::Result<()> {
        self.replacer.clear();
        if self.standard.config.replacement.is_some() {
            let replacement = (*self.standard.config.replacement)
                .as_ref()
                .map(|r| &*r)
                .unwrap();
            self.replacer
                .replace_all(searcher, &self.matcher, bytes, range, replacement)?;
        }
        Ok(())
    }

    fn should_quit(&self) -> bool {
        let limit = match self.standard.config.max_matches {
            None => return false,
            Some(limit) => limit,
        };
        if self.match_count < limit {
            return false;
        }
        self.after_context_remaining == 0
    }

    fn match_more_than_limit(&self) -> bool {
        let limit = match self.standard.config.max_matches {
            None => return false,
            Some(limit) => limit,
        };
        self.match_count > limit
    }
}

impl<'p, 's, M: Matcher, W: WriteColor> Sink for StandardSink<'p, 's, M, W> {
    type Error = io::Error;

    fn matched(&mut self, searcher: &Searcher, mat: &SinkMatch<'_>) -> Result<bool, io::Error> {
        self.match_count += 1;
        if self.match_more_than_limit() {
            self.after_context_remaining = self.after_context_remaining.saturating_sub(1);
        } else {
            self.after_context_remaining = searcher.after_context() as u64;
        }

        self.record_matches(searcher, mat.buffer(), mat.bytes_range_in_buffer())?;
        self.replace(searcher, mat.buffer(), mat.bytes_range_in_buffer())?;

        if let Some(ref mut stats) = self.stats {
            stats.add_matches(self.standard.matches.len() as u64);
            stats.add_matched_lines(mat.lines().count() as u64);
        }

        StandardImpl::from_match(searcher, self, mat).sink()?;
        Ok(!self.should_quit())
    }

    fn context(&mut self, searcher: &Searcher, ctx: &SinkContext<'_>) -> Result<bool, io::Error> {
        self.standard.matches.clear();
        self.replacer.clear();

        if ctx.kind() == &SinkContextKind::After {
            self.after_context_remaining = self.after_context_remaining.saturating_sub(1);
        }
        if searcher.invert_match() {
            self.record_matches(searcher, ctx.bytes(), 0..ctx.bytes().len())?;
            self.replace(searcher, ctx.bytes(), 0..ctx.bytes().len())?;
        }

        StandardImpl::from_context(searcher, self, ctx).sink()?;
        Ok(!self.should_quit())
    }

    fn context_break(&mut self, searcher: &Searcher) -> Result<bool, io::Error> {
        StandardImpl::new(searcher, self).write_context_separator()?;
        Ok(true)
    }

    fn begin(&mut self, _searcher: &Searcher) -> Result<bool, io::Error> {
        self.standard.wtr.borrow_mut().reset_count();
        self.start_time = Instant::now();
        self.match_count = 0;
        self.after_context_remaining = 0;
        if self.standard.config.max_matches == Some(0) {
            return Ok(false);
        }
        Ok(true)
    }

    fn finish(&mut self, searcher: &Searcher, finish: &SinkFinish) -> Result<(), io::Error> {
        if let Some(stats) = self.stats.as_mut() {
            stats.add_elapsed(self.start_time.elapsed());
            stats.add_searches(1);
            if self.match_count > 0 {
                stats.add_searches_with_match(1);
            }
            stats.add_bytes_searched(finish.byte_count());
            stats.add_bytes_printed(self.standard.wtr.borrow().count());
        }
        Ok(())
    }
}

#[derive(Debug)]
struct StandardImpl<'a, M: Matcher, W> {
    searcher: &'a Searcher,
    sink: &'a StandardSink<'a, 'a, M, W>,
    sunk: Sunk<'a>,
    in_color_match: Cell<bool>,
}

impl<'a, M: Matcher, W: WriteColor> StandardImpl<'a, M, W> {
    fn new(searcher: &'a Searcher, sink: &'a StandardSink<'_, '_, M, W>) -> StandardImpl<'a, M, W> {
        StandardImpl {
            searcher,
            sink,
            sunk: Sunk::empty(),
            in_color_match: Cell::new(false),
        }
    }

    fn from_match(
        searcher: &'a Searcher,
        sink: &'a StandardSink<'_, '_, M, W>,
        mat: &'a SinkMatch<'a>,
    ) -> StandardImpl<'a, M, W> {
        let sunk = Sunk::from_sink_match(mat, &sink.standard.matches, sink.replacer.replacement());
        StandardImpl {
            sunk,
            ..StandardImpl::new(searcher, sink)
        }
    }

    fn from_context(
        searcher: &'a Searcher,
        sink: &'a StandardSink<'_, '_, M, W>,
        ctx: &'a SinkContext<'a>,
    ) -> StandardImpl<'a, M, W> {
        let sunk =
            Sunk::from_sink_context(ctx, &sink.standard.matches, sink.replacer.replacement());
        StandardImpl {
            sunk,
            ..StandardImpl::new(searcher, sink)
        }
    }

    fn sink(&self) -> io::Result<()> {
        self.write_search_prelude()?;
        if self.sunk.matches().is_empty() {
            if !self.is_context() {
                self.sink_fast_multi_line()
            } else {
                self.sink_fast()
            }
        } else {
            if !self.is_context() {
                self.sink_slow_multi_line()
            } else {
                self.sink_slow()
            }
        }
    }

    fn sink_fast(&self) -> io::Result<()> {
        debug_assert!(self.sunk.matches().is_empty());
        debug_assert!(self.is_context());

        self.write_prelude(
            self.sunk.absolute_byte_offset(),
            self.sunk.line_number(),
            None,
        )?;
        self.write_line(self.sunk.bytes())
    }

    fn sink_fast_multi_line(&self) -> io::Result<()> {
        debug_assert!(self.sunk.matches().is_empty());

        let line_term = self.searcher.line_terminator().as_byte();
        let mut absolute_byte_offset = self.sunk.absolute_byte_offset();
        for (i, line) in self.sunk.lines(line_term).enumerate() {
            self.write_prelude(
                absolute_byte_offset,
                self.sunk.line_number().map(|n| n + i as u64),
                None,
            )?;
            absolute_byte_offset += line.len() as u64;

            self.write_line(line)?;
        }
        Ok(())
    }

    fn sink_slow(&self) -> io::Result<()> {
        debug_assert!(!self.sunk.matches().is_empty());
        debug_assert!(self.is_context());

        if self.config().only_matching {
            for &m in self.sunk.matches() {
                self.write_prelude(
                    self.sunk.absolute_byte_offset() + m.start() as u64,
                    self.sunk.line_number(),
                    Some(m.start() as u64 + 1),
                )?;

                let buf = &self.sunk.bytes()[m];
                self.write_colored_line(&[Match::new(0, buf.len())], buf)?;
            }
        } else if self.config().per_match {
            for &m in self.sunk.matches() {
                self.write_prelude(
                    self.sunk.absolute_byte_offset() + m.start() as u64,
                    self.sunk.line_number(),
                    Some(m.start() as u64 + 1),
                )?;
                self.write_colored_line(&[m], self.sunk.bytes())?;
            }
        } else {
            self.write_prelude(
                self.sunk.absolute_byte_offset(),
                self.sunk.line_number(),
                Some(self.sunk.matches()[0].start() as u64 + 1),
            )?;
            self.write_colored_line(self.sunk.matches(), self.sunk.bytes())?;
        }
        Ok(())
    }

    fn sink_slow_multi_line(&self) -> io::Result<()> {
        debug_assert!(!self.sunk.matches().is_empty());

        if self.config().only_matching {
            return self.sink_slow_multi_line_only_matching();
        } else if self.config().per_match {
            return self.sink_slow_multi_per_match();
        }

        let line_term = self.searcher.line_terminator().as_byte();
        let bytes = self.sunk.bytes();
        let matches = self.sunk.matches();
        let mut midx = 0;
        let mut count = 0;
        let mut stepper = LineStep::new(line_term, 0, bytes.len());
        while let Some((start, end)) = stepper.next(bytes) {
            let line = Match::new(start, end);
            self.write_prelude(
                self.sunk.absolute_byte_offset() + line.start() as u64,
                self.sunk.line_number().map(|n| n + count),
                Some(matches[0].start() as u64 + 1),
            )?;
            count += 1;
            if self.exceeds_max_columns(&bytes[line]) {
                self.write_exceeded_line(bytes, line, matches, &mut midx)?;
            } else {
                self.write_colored_matches(bytes, line, matches, &mut midx)?;
                self.write_line_term()?;
            }
        }
        Ok(())
    }

    fn sink_slow_multi_line_only_matching(&self) -> io::Result<()> {
        let line_term = self.searcher.line_terminator().as_byte();
        let spec = self.config().colors.matched();
        let bytes = self.sunk.bytes();
        let matches = self.sunk.matches();
        let mut midx = 0;
        let mut count = 0;
        let mut stepper = LineStep::new(line_term, 0, bytes.len());
        while let Some((start, end)) = stepper.next(bytes) {
            let mut line = Match::new(start, end);
            self.trim_line_terminator(bytes, &mut line);
            self.trim_ascii_prefix(bytes, &mut line);
            while !line.is_empty() {
                if matches[midx].end() <= line.start() {
                    if midx + 1 < matches.len() {
                        midx += 1;
                        continue;
                    } else {
                        break;
                    }
                }
                let m = matches[midx];

                if line.start() < m.start() {
                    let upto = cmp::min(line.end(), m.start());
                    line = line.with_start(upto);
                } else {
                    let upto = cmp::min(line.end(), m.end());
                    self.write_prelude(
                        self.sunk.absolute_byte_offset() + m.start() as u64,
                        self.sunk.line_number().map(|n| n + count),
                        Some(m.start() as u64 + 1),
                    )?;

                    let this_line = line.with_end(upto);
                    line = line.with_start(upto);
                    if self.exceeds_max_columns(&bytes[this_line]) {
                        self.write_exceeded_line(bytes, this_line, matches, &mut midx)?;
                    } else {
                        self.write_spec(spec, &bytes[this_line])?;
                        self.write_line_term()?;
                    }
                }
            }
            count += 1;
        }
        Ok(())
    }

    fn sink_slow_multi_per_match(&self) -> io::Result<()> {
        let line_term = self.searcher.line_terminator().as_byte();
        let spec = self.config().colors.matched();
        let bytes = self.sunk.bytes();
        for &m in self.sunk.matches() {
            let mut count = 0;
            let mut stepper = LineStep::new(line_term, 0, bytes.len());
            while let Some((start, end)) = stepper.next(bytes) {
                let mut line = Match::new(start, end);
                if line.start() >= m.end() {
                    break;
                } else if line.end() <= m.start() {
                    count += 1;
                    continue;
                }
                self.write_prelude(
                    self.sunk.absolute_byte_offset() + line.start() as u64,
                    self.sunk.line_number().map(|n| n + count),
                    Some(m.start().saturating_sub(line.start()) as u64 + 1),
                )?;
                count += 1;
                if self.exceeds_max_columns(&bytes[line]) {
                    self.write_exceeded_line(bytes, line, &[m], &mut 0)?;
                    continue;
                }
                self.trim_line_terminator(bytes, &mut line);
                self.trim_ascii_prefix(bytes, &mut line);

                while !line.is_empty() {
                    if m.end() <= line.start() {
                        self.write(&bytes[line])?;
                        line = line.with_start(line.end());
                    } else if line.start() < m.start() {
                        let upto = cmp::min(line.end(), m.start());
                        self.write(&bytes[line.with_end(upto)])?;
                        line = line.with_start(upto);
                    } else {
                        let upto = cmp::min(line.end(), m.end());
                        self.write_spec(spec, &bytes[line.with_end(upto)])?;
                        line = line.with_start(upto);
                    }
                }
                self.write_line_term()?;
                if self.config().per_match_one_line {
                    break;
                }
            }
        }
        Ok(())
    }

    #[inline(always)]
    fn write_prelude(
        &self,
        absolute_byte_offset: u64,
        line_number: Option<u64>,
        column: Option<u64>,
    ) -> io::Result<()> {
        let sep = self.separator_field();

        if !self.config().heading {
            self.write_path_field(sep)?;
        }
        if let Some(n) = line_number {
            self.write_line_number(n, sep)?;
        }
        if let Some(n) = column {
            if self.config().column {
                self.write_column_number(n, sep)?;
            }
        }
        if self.config().byte_offset {
            self.write_byte_offset(absolute_byte_offset, sep)?;
        }
        Ok(())
    }

    #[inline(always)]
    fn write_line(&self, line: &[u8]) -> io::Result<()> {
        if self.exceeds_max_columns(line) {
            let range = Match::new(0, line.len());
            self.write_exceeded_line(line, range, self.sunk.matches(), &mut 0)?;
        } else {
            self.write_trim(line)?;
            if !self.has_line_terminator(line) {
                self.write_line_term()?;
            }
        }
        Ok(())
    }

    fn write_colored_line(&self, matches: &[Match], bytes: &[u8]) -> io::Result<()> {
        let spec = self.config().colors.matched();
        if !self.wtr().borrow().supports_color() || spec.is_none() {
            return self.write_line(bytes);
        }

        let line = Match::new(0, bytes.len());
        if self.exceeds_max_columns(bytes) {
            self.write_exceeded_line(bytes, line, matches, &mut 0)
        } else {
            self.write_colored_matches(bytes, line, matches, &mut 0)?;
            self.write_line_term()?;
            Ok(())
        }
    }

    fn write_colored_matches(
        &self,
        bytes: &[u8],
        mut line: Match,
        matches: &[Match],
        match_index: &mut usize,
    ) -> io::Result<()> {
        self.trim_line_terminator(bytes, &mut line);
        self.trim_ascii_prefix(bytes, &mut line);
        if matches.is_empty() {
            self.write(&bytes[line])?;
            return Ok(());
        }
        while !line.is_empty() {
            if matches[*match_index].end() <= line.start() {
                if *match_index + 1 < matches.len() {
                    *match_index += 1;
                    continue;
                } else {
                    self.end_color_match()?;
                    self.write(&bytes[line])?;
                    break;
                }
            }

            let m = matches[*match_index];
            if line.start() < m.start() {
                let upto = cmp::min(line.end(), m.start());
                self.end_color_match()?;
                self.write(&bytes[line.with_end(upto)])?;
                line = line.with_start(upto);
            } else {
                let upto = cmp::min(line.end(), m.end());
                self.start_color_match()?;
                self.write(&bytes[line.with_end(upto)])?;
                line = line.with_start(upto);
            }
        }
        self.end_color_match()?;
        Ok(())
    }

    fn write_exceeded_line(
        &self,
        bytes: &[u8],
        mut line: Match,
        matches: &[Match],
        match_index: &mut usize,
    ) -> io::Result<()> {
        if self.config().max_columns_preview {
            let original = line;
            let end = bytes[line]
                .grapheme_indices()
                .map(|(_, end, _)| end)
                .take(self.config().max_columns.unwrap_or(0) as usize)
                .last()
                .unwrap_or(0)
                + line.start();
            line = line.with_end(end);
            self.write_colored_matches(bytes, line, matches, match_index)?;

            if matches.is_empty() {
                self.write(b" [... omitted end of long line]")?;
            } else {
                let remaining = matches
                    .iter()
                    .filter(|m| m.start() >= line.end() && m.start() < original.end())
                    .count();
                let tense = if remaining == 1 { "match" } else { "matches" };
                write!(
                    self.wtr().borrow_mut(),
                    " [... {} more {}]",
                    remaining,
                    tense,
                )?;
            }
            self.write_line_term()?;
            return Ok(());
        }
        if self.sunk.original_matches().is_empty() {
            if self.is_context() {
                self.write(b"[Omitted long context line]")?;
            } else {
                self.write(b"[Omitted long matching line]")?;
            }
        } else {
            if self.config().only_matching {
                if self.is_context() {
                    self.write(b"[Omitted long context line]")?;
                } else {
                    self.write(b"[Omitted long matching line]")?;
                }
            } else {
                write!(
                    self.wtr().borrow_mut(),
                    "[Omitted long line with {} matches]",
                    self.sunk.original_matches().len(),
                )?;
            }
        }
        self.write_line_term()?;
        Ok(())
    }

    fn write_path_line(&self) -> io::Result<()> {
        if let Some(path) = self.path() {
            self.write_spec(self.config().colors.path(), path.as_bytes())?;
            if let Some(term) = self.config().path_terminator {
                self.write(&[term])?;
            } else {
                self.write_line_term()?;
            }
        }
        Ok(())
    }

    fn write_path_field(&self, field_separator: &[u8]) -> io::Result<()> {
        if let Some(path) = self.path() {
            self.write_spec(self.config().colors.path(), path.as_bytes())?;
            if let Some(term) = self.config().path_terminator {
                self.write(&[term])?;
            } else {
                self.write(field_separator)?;
            }
        }
        Ok(())
    }

    fn write_search_prelude(&self) -> io::Result<()> {
        let this_search_written = self.wtr().borrow().count() > 0;
        if this_search_written {
            return Ok(());
        }
        if let Some(ref sep) = *self.config().separator_search {
            let ever_written = self.wtr().borrow().total_count() > 0;
            if ever_written {
                self.write(sep)?;
                self.write_line_term()?;
            }
        }
        if self.config().heading {
            self.write_path_line()?;
        }
        Ok(())
    }

    fn write_context_separator(&self) -> io::Result<()> {
        if let Some(ref sep) = *self.config().separator_context {
            self.write(sep)?;
            self.write_line_term()?;
        }
        Ok(())
    }

    fn write_line_number(&self, line_number: u64, field_separator: &[u8]) -> io::Result<()> {
        let n = line_number.to_string();
        self.write_spec(self.config().colors.line(), n.as_bytes())?;
        self.write(field_separator)?;
        Ok(())
    }

    fn write_column_number(&self, column_number: u64, field_separator: &[u8]) -> io::Result<()> {
        let n = column_number.to_string();
        self.write_spec(self.config().colors.column(), n.as_bytes())?;
        self.write(field_separator)?;
        Ok(())
    }

    fn write_byte_offset(&self, offset: u64, field_separator: &[u8]) -> io::Result<()> {
        let n = offset.to_string();
        self.write_spec(self.config().colors.column(), n.as_bytes())?;
        self.write(field_separator)?;
        Ok(())
    }

    fn write_line_term(&self) -> io::Result<()> {
        self.write(self.searcher.line_terminator().as_bytes())
    }

    fn write_spec(&self, spec: &ColorSpec, buf: &[u8]) -> io::Result<()> {
        let mut wtr = self.wtr().borrow_mut();
        wtr.set_color(spec)?;
        wtr.write_all(buf)?;
        wtr.reset()?;
        Ok(())
    }

    fn start_color_match(&self) -> io::Result<()> {
        if self.in_color_match.get() {
            return Ok(());
        }
        self.wtr()
            .borrow_mut()
            .set_color(self.config().colors.matched())?;
        self.in_color_match.set(true);
        Ok(())
    }

    fn end_color_match(&self) -> io::Result<()> {
        if !self.in_color_match.get() {
            return Ok(());
        }
        self.wtr().borrow_mut().reset()?;
        self.in_color_match.set(false);
        Ok(())
    }

    fn write_trim(&self, buf: &[u8]) -> io::Result<()> {
        if !self.config().trim_ascii {
            return self.write(buf);
        }
        let mut range = Match::new(0, buf.len());
        self.trim_ascii_prefix(buf, &mut range);
        self.write(&buf[range])
    }

    fn write(&self, buf: &[u8]) -> io::Result<()> {
        self.wtr().borrow_mut().write_all(buf)
    }

    fn trim_line_terminator(&self, buf: &[u8], line: &mut Match) {
        trim_line_terminator(&self.searcher, buf, line);
    }

    fn has_line_terminator(&self, buf: &[u8]) -> bool {
        self.searcher.line_terminator().is_suffix(buf)
    }

    fn is_context(&self) -> bool {
        self.sunk.context_kind().is_some()
    }

    fn config(&self) -> &'a Config {
        &self.sink.standard.config
    }

    fn wtr(&self) -> &'a RefCell<CounterWriter<W>> {
        &self.sink.standard.wtr
    }

    fn path(&self) -> Option<&'a PrinterPath<'a>> {
        self.sink.path.as_ref()
    }

    fn separator_field(&self) -> &[u8] {
        if self.is_context() {
            &self.config().separator_field_context
        } else {
            &self.config().separator_field_match
        }
    }

    fn exceeds_max_columns(&self, line: &[u8]) -> bool {
        self.config()
            .max_columns
            .map_or(false, |m| line.len() as u64 > m)
    }

    fn trim_ascii_prefix(&self, slice: &[u8], range: &mut Match) {
        if !self.config().trim_ascii {
            return;
        }
        let lineterm = self.searcher.line_terminator();
        *range = trim_ascii_prefix(lineterm, slice, *range)
    }
}
