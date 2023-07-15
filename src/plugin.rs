use std::{
    ffi::{CString, OsStr},
    ptr,
};

use libloading::Library;
use tree_sitter::Node;

use crate::Error;

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

#[repr(u8)]
pub enum PluginInitializeReturn {
    Succeeded,
    MissingArgument,
    NotParseable,
}

fn load_plugin(
    library_path: impl AsRef<OsStr>,
    filter_arg: Option<&str>,
) -> Result<Filterer, Error> {
    let library =
        unsafe { Library::new(library_path).expect("Couldn't load filter dynamic library") };

    if let Ok(initialize) = unsafe {
        library.get::<unsafe extern "C" fn(*const libc::c_char) -> PluginInitializeReturn>(
            b"initialize\0",
        )
    } {
        let filter_arg_as_c_string = filter_arg.map(|filter_arg| {
            CString::new(filter_arg).expect("Couldn't convert provided filter arg to CString")
        });
        let did_initialize = unsafe {
            initialize(
                filter_arg_as_c_string
                    .as_ref()
                    .map_or_else(ptr::null, |filter_arg| filter_arg.as_ptr()),
            )
        };
        match did_initialize {
            PluginInitializeReturn::MissingArgument => {
                return Err(Error::FilterPluginExpectedArgument);
            }
            PluginInitializeReturn::NotParseable => {
                return Err(Error::FilterPluginCouldntParseArgument {
                    filter_arg: filter_arg.unwrap().to_owned(),
                });
            }
            _ => (),
        }
    }

    let filterer = unsafe {
        library
            .get::<unsafe extern "C" fn(*const Node) -> bool>(b"filterer\0")
            .expect("Couldn't load expected symbol from filter dynamic library")
            .into_raw()
    };

    Ok(Filterer {
        filterer,
        _library: library,
    })
}

pub(crate) fn get_loaded_filter(
    filter_library_path: Option<&str>,
    filter_arg: Option<&str>,
) -> Result<Option<Filterer>, Error> {
    filter_library_path
        .map(|filter_library_path| load_plugin(filter_library_path, filter_arg))
        .transpose()
}
