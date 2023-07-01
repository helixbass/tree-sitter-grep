use std::{
    collections::HashMap,
    fs,
    path::Path,
    process,
    sync::{Arc, OnceLock},
    thread,
    time::Duration,
};

use rayon::prelude::*;
use termcolor::{BufferWriter, ColorChoice};
use tree_sitter::Query;

mod args;
mod language;
mod line_buffer;
mod lines;
mod macros;
mod matcher;
mod plugin;
mod printer;
mod project_file_walker;
mod searcher;
mod sink;
mod treesitter;
mod use_matcher;
mod use_searcher;

pub use args::Args;
use args::OutputMode;
use language::{
    get_all_supported_languages, maybe_supported_language_from_path, SupportedLanguage,
    SupportedLanguageName,
};
pub use plugin::PluginInitializeReturn;
use printer::get_printer;
use project_file_walker::get_project_file_parallel_iterator;
use treesitter::maybe_get_query;
use use_matcher::TreeSitterMatcher;
use use_searcher::get_searcher;

#[derive(Default)]
struct CaptureIndex(OnceLock<Result<u32, ()>>);

impl CaptureIndex {
    pub fn get_or_init(&self, query: &Query, capture_name: Option<&str>) -> u32 {
        let mut did_mark_failed = false;
        self.0
            .get_or_init(|| match capture_name {
                None => Ok(0),
                Some(capture_name) => query.capture_index_for_name(capture_name).ok_or_else(|| {
                    did_mark_failed = true;
                    Default::default()
                }),
            })
            .unwrap_or_else(|_| {
                if did_mark_failed {
                    fail(&format!("invalid capture name '{}'", capture_name.unwrap()));
                }
                // whichever (other?) thread "won the race" will have called fail()
                // so we'll be getting killed shortly?
                thread::sleep(Duration::from_millis(100_000));
                panic!("Should never get this far");
            })
    }
}

const ALL_NODES_QUERY: &str = "(_) @node";

struct CachedQueries(HashMap<SupportedLanguageName, OnceLock<Option<Arc<Query>>>>);

impl CachedQueries {
    fn get_and_cache_query_for_language(
        &self,
        query_source: &str,
        language: &dyn SupportedLanguage,
    ) -> Option<Arc<Query>> {
        self.0
            .get(&language.name())
            .unwrap()
            .get_or_init(|| maybe_get_query(query_source, language.language()).map(Arc::new))
            .clone()
    }

    fn error_if_no_successful_query_parsing(&self) {
        if !self
            .0
            .values()
            .any(|query| query.get().and_then(|option| option.as_ref()).is_some())
        {
            fail("invalid query");
        }
    }
}

impl Default for CachedQueries {
    fn default() -> Self {
        Self(
            get_all_supported_languages()
                .into_keys()
                .map(|supported_language_name| (supported_language_name, Default::default()))
                .collect(),
        )
    }
}

pub fn run(args: Args) {
    let query_source = match (args.path_to_query_file.as_ref(), args.query_source.as_ref()) {
        (Some(path_to_query_file), None) => fs::read_to_string(path_to_query_file)
            .unwrap_or_else(|_| fail(&format!("couldn't read query file {path_to_query_file:?}"))),
        (None, Some(query_source)) => query_source.clone(),
        (None, None) => ALL_NODES_QUERY.to_owned(),
        _ => unreachable!(),
    };
    let specified_supported_language = args.language.map(|language| language.get_language());
    let cached_queries: CachedQueries = Default::default();
    let capture_index = CaptureIndex::default();
    let output_mode = args.output_mode();
    let buffer_writer = BufferWriter::stdout(ColorChoice::Never);

    get_project_file_parallel_iterator(specified_supported_language.as_deref(), &args.use_paths())
        .for_each(|project_file_dir_entry| {
            let language = maybe_supported_language_from_path(project_file_dir_entry.path())
                .expect("Walker should've been pre-filtered to just supported file types");
            let query = return_if_none!(
                cached_queries.get_and_cache_query_for_language(&query_source, &*language)
            );
            let capture_index = capture_index.get_or_init(&query, args.capture_name.as_deref());
            let printer = get_printer(&buffer_writer, output_mode);
            let mut printer = printer.borrow_mut();
            let path =
                format_relative_path(project_file_dir_entry.path(), args.is_using_default_paths());

            let matcher = TreeSitterMatcher::new(
                &query,
                capture_index,
                language.language(),
                args.filter.clone(),
                args.filter_arg.clone(),
            );

            printer.get_mut().clear();
            get_searcher(output_mode)
                .borrow_mut()
                .search_path(&matcher, path, printer.sink_with_path(&matcher, path))
                .unwrap();
            buffer_writer.print(printer.get_mut()).unwrap();
        });

    cached_queries.error_if_no_successful_query_parsing();
}

fn fail(message: &str) -> ! {
    eprintln!("error: {message}");
    process::exit(1);
}

fn format_relative_path(path: &Path, is_using_default_paths: bool) -> &Path {
    if is_using_default_paths && path.starts_with("./") {
        path.strip_prefix("./").unwrap()
    } else {
        path
    }
}
