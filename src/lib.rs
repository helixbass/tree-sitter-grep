use clap::Parser;
use grep::matcher::Match;
use grep::matcher::Matcher;
use grep::matcher::NoCaptures;
use grep::matcher::NoError;
use grep::searcher::Searcher;
use ignore::{types::TypesBuilder, DirEntry, WalkBuilder};
use rayon::prelude::*;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::Mutex;

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
        .for_each(|project_file_dir_entry| {
            let mut printer = grep::printer::Standard::new_no_color(io::stdout());
            let path = project_file_dir_entry.path();
            let matches = get_results(&query, path, capture_index, language);

            let matcher = TreeSitterMatcher::new(matches);

            Searcher::new()
                .search_path(&matcher, path, printer.sink_with_path(&matcher, path))
                .unwrap();
        })
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
#[derive(Debug)]
struct TreeSitterMatcher {
    matches: Mutex<Vec<Match>>,
}

impl TreeSitterMatcher {
    fn new(mut matches: Vec<Match>) -> Self {
        matches.sort_by_key(|m| m.start());

        Self {
            matches: Mutex::new(matches),
        }
    }
}

impl Matcher for TreeSitterMatcher {
    type Captures = NoCaptures;

    type Error = NoError;

    fn find_at(&self, _haystack: &[u8], at: usize) -> Result<Option<Match>, Self::Error> {
        let _match = self
            .matches
            .lock()
            .unwrap()
            .pop()
            .map(|m| Match::new(m.start() - at, m.end() - at));
        Ok(_match)
    }

    fn new_captures(&self) -> Result<Self::Captures, Self::Error> {
        Ok(NoCaptures::new())
    }
}
