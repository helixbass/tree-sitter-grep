use clap::Parser;
use ignore::{types::TypesBuilder, DirEntry, WalkBuilder};
use rayon::prelude::*;
use std::fs;
use std::path::PathBuf;

mod macros;
mod treesitter;

use treesitter::{get_query, get_results};

#[derive(Parser)]
pub struct Args {
    pub path_to_query_file: PathBuf,
    #[arg(short, long = "capture")]
    pub capture_name: Option<String>,
}

pub fn run(args: Args) {
    let query_source = fs::read_to_string(&args.path_to_query_file).unwrap();
    let query = get_query(&query_source);
    let capture_index = args.capture_name.as_ref().map_or(0, |capture_name| {
        query
            .capture_index_for_name(capture_name)
            .expect(&format!("Unknown capture name: `{}`", capture_name))
    });
    enumerate_project_files()
        .par_iter()
        .flat_map(|project_file_dir_entry| {
            get_results(&query, project_file_dir_entry.path(), capture_index)
        })
        .for_each(|result| {
            println!("{}", result.format());
        });
}

fn enumerate_project_files() -> Vec<DirEntry> {
    WalkBuilder::new(".")
        .types(
            TypesBuilder::new()
                .add_defaults()
                .select("rust")
                .build()
                .unwrap(),
        )
        .build()
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let extension = entry.path().extension();
            if extension.is_none() {
                return false;
            }
            let extension = extension.unwrap();
            "rs" == extension
        })
        .collect()
}
