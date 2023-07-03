// derived from https://github.com/BurntSushi/ripgrep/blob/master/crates/searcher/src/line_buffer.rs

use std::io;

pub(crate) const DEFAULT_BUFFER_CAPACITY: usize = 64 * (1 << 10);

pub fn alloc_error(limit: usize) -> io::Error {
    let msg = format!("configured allocation limit ({}) exceeded", limit);
    io::Error::new(io::ErrorKind::Other, msg)
}
