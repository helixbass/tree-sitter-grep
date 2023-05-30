use std::{
    ffi::CStr,
    sync::atomic::{AtomicUsize, Ordering},
};

use libc::c_char;
use tree_sitter::Node;

static ROW_NUMBER: AtomicUsize = AtomicUsize::new(0);

#[no_mangle]
pub extern "C" fn initialize(value: *const c_char) {
    assert!(!value.is_null(), "Expected filter argument");
    let value: usize = unsafe { CStr::from_ptr(value) }
        .to_str()
        .unwrap()
        .parse()
        .expect("Expected filter argument to be a usize");
    ROW_NUMBER.store(value, Ordering::Relaxed);
}

#[no_mangle]
pub extern "C" fn filterer(node: &Node) -> bool {
    node.start_position().row > ROW_NUMBER.load(Ordering::Relaxed)
}
