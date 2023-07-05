use std::sync::atomic::{AtomicBool, Ordering};

// derived from https://github.com/BurntSushi/ripgrep/blob/master/crates/core/messages.rs
static ERRORED: AtomicBool = AtomicBool::new(false);

#[macro_export]
macro_rules! message {
    ($($tt:tt)*) => {
        eprintln!($($tt)*);
    }
}

#[macro_export]
macro_rules! err_message {
    ($($tt:tt)*) => {
        $crate::messages::set_errored();
        $crate::message!($($tt)*);
    }
}

pub fn errored() -> bool {
    ERRORED.load(Ordering::SeqCst)
}

pub fn set_errored() {
    ERRORED.store(true, Ordering::SeqCst);
}
