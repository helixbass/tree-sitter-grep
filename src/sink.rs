use std::{error, fmt, io};

use crate::{
    lines::LineIter,
    matcher::LineTerminator,
    searcher::{ConfigError, Searcher},
};

pub trait SinkError: Sized {
    fn error_message<T: fmt::Display>(message: T) -> Self;

    fn error_io(err: io::Error) -> Self {
        Self::error_message(err)
    }

    fn error_config(err: ConfigError) -> Self {
        Self::error_message(err)
    }
}

impl SinkError for io::Error {
    fn error_message<T: fmt::Display>(message: T) -> io::Error {
        io::Error::new(io::ErrorKind::Other, message.to_string())
    }

    fn error_io(err: io::Error) -> io::Error {
        err
    }
}

impl SinkError for Box<dyn error::Error> {
    fn error_message<T: fmt::Display>(message: T) -> Box<dyn error::Error> {
        Box::<dyn error::Error>::from(message.to_string())
    }
}

pub trait Sink {
    type Error: SinkError;

    fn matched(&mut self, _searcher: &Searcher, _mat: &SinkMatch<'_>) -> Result<bool, Self::Error>;

    #[inline]
    fn context(
        &mut self,
        _searcher: &Searcher,
        _context: &SinkContext<'_>,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }

    #[inline]
    fn context_break(&mut self, _searcher: &Searcher) -> Result<bool, Self::Error> {
        Ok(true)
    }

    #[inline]
    fn begin(&mut self, _searcher: &Searcher) -> Result<bool, Self::Error> {
        Ok(true)
    }

    #[inline]
    fn finish(&mut self, _searcher: &Searcher, _: &SinkFinish) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<'a, S: Sink> Sink for &'a mut S {
    type Error = S::Error;

    #[inline]
    fn matched(&mut self, searcher: &Searcher, mat: &SinkMatch<'_>) -> Result<bool, S::Error> {
        (**self).matched(searcher, mat)
    }

    #[inline]
    fn context(
        &mut self,
        searcher: &Searcher,
        context: &SinkContext<'_>,
    ) -> Result<bool, S::Error> {
        (**self).context(searcher, context)
    }

    #[inline]
    fn context_break(&mut self, searcher: &Searcher) -> Result<bool, S::Error> {
        (**self).context_break(searcher)
    }

    #[inline]
    fn begin(&mut self, searcher: &Searcher) -> Result<bool, S::Error> {
        (**self).begin(searcher)
    }

    #[inline]
    fn finish(&mut self, searcher: &Searcher, sink_finish: &SinkFinish) -> Result<(), S::Error> {
        (**self).finish(searcher, sink_finish)
    }
}

impl<S: Sink + ?Sized> Sink for Box<S> {
    type Error = S::Error;

    #[inline]
    fn matched(&mut self, searcher: &Searcher, mat: &SinkMatch<'_>) -> Result<bool, S::Error> {
        (**self).matched(searcher, mat)
    }

    #[inline]
    fn context(
        &mut self,
        searcher: &Searcher,
        context: &SinkContext<'_>,
    ) -> Result<bool, S::Error> {
        (**self).context(searcher, context)
    }

    #[inline]
    fn context_break(&mut self, searcher: &Searcher) -> Result<bool, S::Error> {
        (**self).context_break(searcher)
    }

    #[inline]
    fn begin(&mut self, searcher: &Searcher) -> Result<bool, S::Error> {
        (**self).begin(searcher)
    }

    #[inline]
    fn finish(&mut self, searcher: &Searcher, sink_finish: &SinkFinish) -> Result<(), S::Error> {
        (**self).finish(searcher, sink_finish)
    }
}

#[derive(Clone, Debug)]
pub struct SinkFinish {
    pub(crate) byte_count: u64,
}

impl SinkFinish {
    #[inline]
    pub fn byte_count(&self) -> u64 {
        self.byte_count
    }
}

#[derive(Clone, Debug)]
pub struct SinkMatch<'b> {
    pub(crate) line_term: LineTerminator,
    pub(crate) bytes: &'b [u8],
    pub(crate) absolute_byte_offset: u64,
    pub(crate) line_number: Option<u64>,
    pub(crate) buffer: &'b [u8],
    pub(crate) bytes_range_in_buffer: std::ops::Range<usize>,
}

impl<'b> SinkMatch<'b> {
    #[inline]
    pub fn bytes(&self) -> &'b [u8] {
        self.bytes
    }

    #[inline]
    pub fn lines(&self) -> LineIter<'b> {
        LineIter::new(self.line_term.as_byte(), self.bytes)
    }

    #[inline]
    pub fn absolute_byte_offset(&self) -> u64 {
        self.absolute_byte_offset
    }

    #[inline]
    pub fn line_number(&self) -> Option<u64> {
        self.line_number
    }

    #[inline]
    pub fn buffer(&self) -> &'b [u8] {
        self.buffer
    }

    #[inline]
    pub fn bytes_range_in_buffer(&self) -> std::ops::Range<usize> {
        self.bytes_range_in_buffer.clone()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SinkContextKind {
    Before,
    After,
    Other,
}

#[derive(Clone, Debug)]
pub struct SinkContext<'b> {
    #[cfg(test)]
    pub(crate) line_term: LineTerminator,
    pub(crate) bytes: &'b [u8],
    pub(crate) kind: SinkContextKind,
    pub(crate) absolute_byte_offset: u64,
    pub(crate) line_number: Option<u64>,
}

impl<'b> SinkContext<'b> {
    #[inline]
    pub fn bytes(&self) -> &'b [u8] {
        self.bytes
    }

    #[inline]
    pub fn kind(&self) -> &SinkContextKind {
        &self.kind
    }

    #[cfg(test)]
    pub(crate) fn lines(&self) -> LineIter<'b> {
        LineIter::new(self.line_term.as_byte(), self.bytes)
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

pub mod sinks {
    use std::{io, str};

    use super::{Sink, SinkError, SinkMatch};
    use crate::searcher::Searcher;

    #[derive(Clone, Debug)]
    pub struct UTF8<F>(pub F)
    where
        F: FnMut(u64, &str) -> Result<bool, io::Error>;

    impl<F> Sink for UTF8<F>
    where
        F: FnMut(u64, &str) -> Result<bool, io::Error>,
    {
        type Error = io::Error;

        fn matched(
            &mut self,
            _searcher: &Searcher,
            mat: &SinkMatch<'_>,
        ) -> Result<bool, io::Error> {
            let matched = match str::from_utf8(mat.bytes()) {
                Ok(matched) => matched,
                Err(err) => return Err(io::Error::error_message(err)),
            };
            let line_number = match mat.line_number() {
                Some(line_number) => line_number,
                None => {
                    let msg = "line numbers not enabled";
                    return Err(io::Error::error_message(msg));
                }
            };
            (self.0)(line_number, &matched)
        }
    }

    #[derive(Clone, Debug)]
    pub struct Lossy<F>(pub F)
    where
        F: FnMut(u64, &str) -> Result<bool, io::Error>;

    impl<F> Sink for Lossy<F>
    where
        F: FnMut(u64, &str) -> Result<bool, io::Error>,
    {
        type Error = io::Error;

        fn matched(
            &mut self,
            _searcher: &Searcher,
            mat: &SinkMatch<'_>,
        ) -> Result<bool, io::Error> {
            use std::borrow::Cow;

            let matched = match str::from_utf8(mat.bytes()) {
                Ok(matched) => Cow::Borrowed(matched),
                Err(_) => String::from_utf8_lossy(mat.bytes()),
            };
            let line_number = match mat.line_number() {
                Some(line_number) => line_number,
                None => {
                    let msg = "line numbers not enabled";
                    return Err(io::Error::error_message(msg));
                }
            };
            (self.0)(line_number, &matched)
        }
    }

    #[derive(Clone, Debug)]
    pub struct Bytes<F>(pub F)
    where
        F: FnMut(u64, &[u8]) -> Result<bool, io::Error>;

    impl<F> Sink for Bytes<F>
    where
        F: FnMut(u64, &[u8]) -> Result<bool, io::Error>,
    {
        type Error = io::Error;

        fn matched(
            &mut self,
            _searcher: &Searcher,
            mat: &SinkMatch<'_>,
        ) -> Result<bool, io::Error> {
            let line_number = match mat.line_number() {
                Some(line_number) => line_number,
                None => {
                    let msg = "line numbers not enabled";
                    return Err(io::Error::error_message(msg));
                }
            };
            (self.0)(line_number, mat.bytes())
        }
    }
}
