use std::{
    ffi::CStr,
    sync::atomic::{AtomicUsize, Ordering},
};

use libc::c_char;
use tree_sitter::Node;
use tree_sitter_grep::{
    PluginInitializeReturn, PLUGIN_INITIALIZE_ARGUMENT_NOT_PARSEABLE,
    PLUGIN_INITIALIZE_MISSING_EXPECTED_ARGUMENT, PLUGIN_INITIALIZE_SUCCEEDED,
};

static ROW_NUMBER: AtomicUsize = AtomicUsize::new(0);

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[no_mangle]
pub extern "C" fn initialize(value: *const c_char) -> PluginInitializeReturn {
    if value.is_null() {
        return PLUGIN_INITIALIZE_MISSING_EXPECTED_ARGUMENT;
    }
    let value: usize = match unsafe { CStr::from_ptr(value) }.to_str().unwrap().parse() {
        Err(_) => {
            return PLUGIN_INITIALIZE_ARGUMENT_NOT_PARSEABLE;
        }
        Ok(value) => value,
    };
    ROW_NUMBER.store(value, Ordering::Relaxed);
    PLUGIN_INITIALIZE_SUCCEEDED
}

#[no_mangle]
pub extern "C" fn filterer(node: &Node) -> bool {
    node.start_position().row < ROW_NUMBER.load(Ordering::Relaxed)
}
