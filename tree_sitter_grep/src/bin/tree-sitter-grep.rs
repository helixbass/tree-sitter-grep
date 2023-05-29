use clap::Parser;
use tree_sitter_grep::{run, Args};

pub fn main() {
    let args = Args::parse();
    run(args);
}
