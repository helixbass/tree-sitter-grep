use tree_sitter::Node;
use tree_sitter_grep_plugin_core::{export_plugin, Filterer, PluginRegistrar};

struct FilterAfterLine15;

impl Filterer for FilterAfterLine15 {
    fn call(&self, node: &Node) -> bool {
        node.start_position().row > 15
    }
}

export_plugin!(register);

extern "C" fn register(registrar: &mut dyn PluginRegistrar) {
    registrar.register_filterer(Box::new(FilterAfterLine15));
}
