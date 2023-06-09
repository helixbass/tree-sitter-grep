use std::{
    ffi::{CString, OsStr},
    ptr,
};

use libloading::Library;
use once_cell::sync::OnceCell;
use tree_sitter::Node;

#[cfg(unix)]
type RawSymbol<TExportedSymbol> = libloading::os::unix::Symbol<TExportedSymbol>;
#[cfg(windows)]
type RawSymbol<TExportedSymbol> = libloading::os::windows::Symbol<TExportedSymbol>;

pub struct Filterer {
    filterer: RawSymbol<unsafe extern "C" fn(*const Node) -> bool>,
    _library: Library,
}

impl Filterer {
    pub fn call(&self, node: &Node) -> bool {
        unsafe { (self.filterer)(node) }
    }
}

fn load_plugin(library_path: impl AsRef<OsStr>, filter_arg: Option<&str>) -> Filterer {
    let library =
        unsafe { Library::new(library_path).expect("Couldn't load filter dynamic library") };

    if let Ok(initialize) =
        unsafe { library.get::<unsafe extern "C" fn(*const libc::c_char)>(b"initialize\0") }
    {
        let filter_arg = filter_arg.map(|filter_arg| {
            CString::new(filter_arg).expect("Couldn't convert provided filter arg to CString")
        });
        unsafe {
            initialize(
                filter_arg
                    .as_ref()
                    .map_or_else(ptr::null, |filter_arg| filter_arg.as_ptr()),
            );
        }
    }

    let filterer = unsafe {
        library
            .get::<unsafe extern "C" fn(*const Node) -> bool>(b"filterer\0")
            .expect("Couldn't load expected symbol from filter dynamic library")
            .into_raw()
    };

    Filterer {
        filterer,
        _library: library,
    }
}

pub fn get_loaded_filter(
    filter_library_path: Option<&str>,
    filter_arg: Option<&str>,
) -> Option<&'static Filterer> {
    filter_library_path.map(|filter_library_path| {
        static LOADED_FILTERER: OnceCell<Filterer> = OnceCell::new();
        LOADED_FILTERER.get_or_init(|| load_plugin(filter_library_path, filter_arg))
    })
}
