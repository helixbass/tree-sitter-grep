use tree_sitter::Node;
use tree_sitter_grep_plugin_core::{export_plugin, Filterer, PluginRegistrar};

struct FilterAfterLineNumber {
    line_number: usize,
}

impl FilterAfterLineNumber {
    pub fn new(arg: Option<&str>) -> Self {
        Self {
            line_number: arg
                .expect("Expected to be supplied line number on command line")
                .parse()
                .expect("Couldn't parse command-line argument as usize"),
        }
    }
}

impl Filterer for FilterAfterLineNumber {
    fn call(&self, node: &Node) -> bool {
        node.start_position().row > self.line_number
    }
}

export_plugin!(register);

extern "C" fn register(registrar: &mut dyn PluginRegistrar, arg: Option<&str>) {
    registrar.register_filterer(Box::new(FilterAfterLineNumber::new(arg)));
}
