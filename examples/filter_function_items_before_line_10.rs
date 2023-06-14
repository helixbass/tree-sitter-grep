use tree_sitter::Node;

#[no_mangle]
pub extern "C" fn filterer(node: &Node) -> bool {
    node.kind() == "function_item" && node.start_position().row < 10
}
