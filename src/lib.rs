use clap::Parser;
use ignore::{types::TypesBuilder, DirEntry, WalkBuilder};
use rayon::prelude::*;
use std::fs;
use std::path::PathBuf;

mod language;
mod macros;
mod treesitter;

use language::{SupportedLanguage, SupportedLanguageName};
use treesitter::{get_query, get_results};

#[derive(Parser)]
pub struct Args {
    #[command(flatten)]
    pub query_args: QueryArgs,
    #[arg(short, long = "capture")]
    pub capture_name: Option<String>,
    #[arg(short, long, value_enum)]
    pub language: SupportedLanguageName,
}

#[derive(clap::Args)]
#[group(required = true, multiple = false)]
pub struct QueryArgs {
    pub path_to_query_file: Option<PathBuf>,
    #[arg(short, long)]
    pub query_source: Option<String>,
}

pub fn run(args: Args) {
    let query_source = match args.query_args.path_to_query_file.as_ref() {
        Some(path_to_query_file) => fs::read_to_string(path_to_query_file).unwrap(),
        None => args.query_args.query_source.clone().unwrap(),
    };
    let supported_language = args.language.get_language();
    let language = supported_language.language();
    let query = get_query(&query_source, language);
    let capture_index = args.capture_name.as_ref().map_or(0, |capture_name| {
        query
            .capture_index_for_name(capture_name)
            .expect(&format!("Unknown capture name: `{}`", capture_name))
    });
    enumerate_project_files(&*supported_language)
        .par_iter()
        .flat_map(|project_file_dir_entry| {
            get_results(
                &query,
                project_file_dir_entry.path(),
                capture_index,
                language,
            )
        })
        .for_each(|result| {
            println!("{}", result.format());
        });
}

fn enumerate_project_files(language: &dyn SupportedLanguage) -> Vec<DirEntry> {
    WalkBuilder::new(".")
        .types(
            TypesBuilder::new()
                .add_defaults()
                .select(language.name_for_ignore_select())
                .build()
                .unwrap(),
        )
        .build()
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.metadata().unwrap().is_file())
        .collect()
}
