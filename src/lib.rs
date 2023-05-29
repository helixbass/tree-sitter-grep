use clap::Parser;
use grep::matcher::Match;
use grep::matcher::Matcher;
use grep::matcher::NoCaptures;
use grep::matcher::NoError;
use grep::searcher::SearcherBuilder;
use ignore::{types::TypesBuilder, DirEntry, WalkBuilder};
use rayon::prelude::*;
use std::cell::RefCell;
use std::fs;
use std::io;
use std::path::PathBuf;
use tree_sitter::{Language, Query};

mod language;
mod macros;
mod treesitter;

use language::{SupportedLanguage, SupportedLanguageName};
use treesitter::{get_matches, get_query};

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

            let matcher = TreeSitterMatcher::new(&query, capture_index, language);

            SearcherBuilder::new()
                .multi_line(true)
                .build()
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
struct TreeSitterMatcher<'query> {
    query: &'query Query,
    capture_index: u32,
    language: Language,
    matches_info: RefCell<Option<PopulatedMatchesInfo>>,
}

impl<'query> TreeSitterMatcher<'query> {
    fn new(query: &'query Query, capture_index: u32, language: Language) -> Self {
        Self {
            query,
            capture_index,
            language,
            matches_info: Default::default(),
        }
    }
}

impl Matcher for TreeSitterMatcher<'_> {
    type Captures = NoCaptures;

    type Error = NoError;

    fn find_at(&self, haystack: &[u8], at: usize) -> Result<Option<Match>, Self::Error> {
        let mut matches_info = self.matches_info.borrow_mut();
        if let Some(matches_info) = matches_info.as_ref() {
            assert!(
                haystack.len() < matches_info.text_len,
                "Expected to get passed subset of file text on subsequent invocations"
            );
        }
        let matches_info = matches_info.get_or_insert_with(|| {
            assert!(at == 0);
            PopulatedMatchesInfo {
                matches: get_matches(self.query, self.capture_index, haystack, self.language),
                text_len: haystack.len(),
            }
        });
        if matches_info.matches.is_empty() {
            return Ok(None);
        }
        let match_ = matches_info.matches.remove(0);
        Ok(Some(adjust_match(
            match_,
            haystack.len(),
            matches_info.text_len,
        )))
    }

    fn new_captures(&self) -> Result<Self::Captures, Self::Error> {
        Ok(NoCaptures::new())
    }
}

#[derive(Debug)]
struct PopulatedMatchesInfo {
    matches: Vec<Match>,
    text_len: usize,
}

fn adjust_match(match_: Match, haystack_len: usize, total_file_text_len: usize) -> Match {
    let offset_in_file = total_file_text_len - haystack_len;
    Match::new(
        match_.start() - offset_in_file,
        match_.end() - offset_in_file,
    )
}
