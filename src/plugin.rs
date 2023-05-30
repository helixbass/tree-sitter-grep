use libloading::Library;
use once_cell::sync::OnceCell;
use std::ffi::OsStr;
use tree_sitter::Node;

#[cfg(unix)]
type RawSymbol<TValue> = libloading::os::unix::Symbol<TValue>;
#[cfg(windows)]
type RawSymbol<TValue> = libloading::os::windows::Symbol<TValue>;

pub struct Filterer {
    // filterer: Symbol<fn(&Node) -> bool>,
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
