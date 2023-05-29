use libloading::Library;
use once_cell::sync::OnceCell;
use std::ffi::OsStr;
use tree_sitter::Node;
use tree_sitter_grep_plugin_core::Filterer;
use tree_sitter_grep_plugin_core::PluginDeclaration;

use std::sync::Arc;

pub struct FiltererProxy {
    filterer: Box<dyn Filterer>,
    _lib: Arc<Library>,
}

impl Filterer for FiltererProxy {
    fn call(&self, node: &Node) -> bool {
        self.filterer.call(node)
    }
}

struct PluginRegistrar {
    filterer: Option<FiltererProxy>,
    lib: Arc<Library>,
}

impl PluginRegistrar {
    pub fn new(lib: Arc<Library>) -> Self {
        Self {
            lib,
            filterer: Default::default(),
        }
    }
}

impl tree_sitter_grep_plugin_core::PluginRegistrar for PluginRegistrar {
    fn register_filterer(&mut self, filterer: Box<dyn Filterer>) {
        let proxy = FiltererProxy {
            filterer,
            _lib: self.lib.clone(),
        };
        self.filterer = Some(proxy);
    }
}

fn load_plugin(library_path: impl AsRef<OsStr>) -> FiltererProxy {
    let library = Arc::new(unsafe {
        Library::new(library_path).expect("Couldn't load filter dynamic library")
    });

    let plugin_declaration = unsafe {
        library
            .get::<*mut PluginDeclaration>(b"plugin_declaration\0")
            .expect("Couldn't load expected symbol from filter dynamic library")
            .read()
    };

    if plugin_declaration.rustc_version != tree_sitter_grep_plugin_core::RUSTC_VERSION
        || plugin_declaration.core_version != tree_sitter_grep_plugin_core::CORE_VERSION
    {
        panic!("Found incompatible plugin version");
    }

    let mut registrar = PluginRegistrar::new(library.clone());

    unsafe {
        (plugin_declaration.register)(&mut registrar);
    }

    registrar.filterer.unwrap()
}

pub fn get_loaded_filter(filter_library_path: Option<&str>) -> Option<&'static FiltererProxy> {
    filter_library_path.map(|filter_library_path| {
        static LOADED_FILTERER: OnceCell<FiltererProxy> = OnceCell::new();
        LOADED_FILTERER.get_or_init(|| load_plugin(filter_library_path))
    })
}
