use std::io;

use bstr::ByteSlice;

pub(crate) const DEFAULT_BUFFER_CAPACITY: usize = 64 * (1 << 10);

pub fn alloc_error(limit: usize) -> io::Error {
    let msg = format!("configured allocation limit ({}) exceeded", limit);
    io::Error::new(io::ErrorKind::Other, msg)
}

/// Replaces `src` with `replacement` in bytes, and return the offset of the
/// first replacement, if one exists.
fn replace_bytes(bytes: &mut [u8], src: u8, replacement: u8) -> Option<usize> {
    if src == replacement {
        return None;
    }
    let mut first_pos = None;
    let mut pos = 0;
    while let Some(i) = bytes[pos..].find_byte(src).map(|i| pos + i) {
        if first_pos.is_none() {
            first_pos = Some(i);
        }
        bytes[i] = replacement;
        pos = i + 1;
        while bytes.get(pos) == Some(&src) {
            bytes[pos] = replacement;
            pos += 1;
        }
    }
    first_pos
}
