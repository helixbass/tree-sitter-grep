use clap::Parser;
use tree_sitter_grep::{run_with_callback, Args};

fn main() {
    let args = Args::parse_from(["tree_sitter_grep", "-q", "(function_item) @f"]);
    run_with_callback(args, |capture_info, file_contents, path| {
        println!(
            "Found match in {path:?}: {}",
            std::str::from_utf8(&file_contents[capture_info.node.byte_range()]).unwrap(),
        );
    })
    .unwrap();
}
