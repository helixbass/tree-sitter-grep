use tree_sitter::Node;

#[no_mangle]
pub extern "C" fn filterer(node: &Node) -> bool {
    node.start_position().row < 10
}
