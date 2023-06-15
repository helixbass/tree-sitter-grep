use std::{
    collections::HashMap,
    fs,
    path::Path,
    process,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use rayon::prelude::*;
use termcolor::{BufferWriter, ColorChoice};
use tree_sitter::Query;

mod args;
mod language;
mod macros;
mod matcher;
mod plugin;
mod printer;
mod project_file_walker;
mod searcher;
mod treesitter;

pub use args::Args;
use args::OutputMode;
use language::{maybe_supported_language_from_path, SupportedLanguage, SupportedLanguageName};
use matcher::TreeSitterMatcher;
pub use plugin::PluginInitializeReturn;
use printer::get_printer;
use project_file_walker::get_project_file_parallel_iterator;
use searcher::get_searcher;
use treesitter::maybe_get_query;

struct MaybeInitializedCaptureIndex(AtomicU32);

impl MaybeInitializedCaptureIndex {
    const UNINITIALIZED: u32 = u32::MAX;
    const FAILED: u32 = u32::MAX - 1;

    fn mark_failed(&self) -> bool {
        loop {
            let existing_value = self.0.load(Ordering::Relaxed);
            if existing_value == Self::FAILED {
                return false;
            }
            let did_store = self.0.compare_exchange(
                existing_value,
                Self::FAILED,
                Ordering::Relaxed,
                Ordering::Relaxed,
            );
            if did_store.is_ok() {
                return true;
            }
        }
    }

    pub fn get(&self) -> Result<Option<u32>, ()> {
        let loaded = self.0.load(Ordering::Relaxed);
        match loaded {
            loaded if loaded == Self::UNINITIALIZED => Ok(None),
            loaded if loaded == Self::FAILED => Err(()),
            loaded => Ok(Some(loaded)),
        }
    }

    pub fn get_or_initialize(&self, query: &Query, capture_name: Option<&str>) -> Result<u32, ()> {
        if let Some(already_initialized) = self.get()? {
            return Ok(already_initialized);
        }
        let capture_index = match capture_name {
            None => 0,
            Some(capture_name) => {
                let capture_index = query.capture_index_for_name(capture_name);
                if capture_index.is_none() {
                    let did_mark_failed = self.mark_failed();
                    if did_mark_failed {
                        fail(&format!("invalid capture name '{}'", capture_name));
                    } else {
                        // whichever other thread "won the race" will have called this fail()
                        // so we'll be getting killed shortly?
                        thread::sleep(Duration::from_millis(100_000));
                    }
                }
                capture_index.unwrap()
            }
        };
        self.set(capture_index);
        Ok(capture_index)
    }

    fn set(&self, capture_index: u32) {
        self.0.store(capture_index, Ordering::Relaxed);
    }
}

impl Default for MaybeInitializedCaptureIndex {
    fn default() -> Self {
        Self(AtomicU32::new(Self::UNINITIALIZED))
    }
}

const ALL_NODES_QUERY: &str = "(_) @node";

#[derive(Default)]
struct CachedQueries(Mutex<HashMap<SupportedLanguageName, Option<Arc<Query>>>>);

impl CachedQueries {
    fn get_and_cache_query_for_language(
        &self,
        query_source: &str,
        language: &dyn SupportedLanguage,
    ) -> Option<Arc<Query>> {
        self.0
            .lock()
            .unwrap()
            .entry(language.name())
            .or_insert_with(|| maybe_get_query(query_source, language.language()).map(Arc::new))
            .clone()
    }

    fn error_if_no_successful_query_parsing(&self) {
        if !self.0.lock().unwrap().values().any(|query| query.is_some()) {
            fail("invalid query");
        }
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
    let capture_index = MaybeInitializedCaptureIndex::default();
    let output_mode = args.output_mode();
    let buffer_writer = BufferWriter::stdout(ColorChoice::Never);

    get_project_file_parallel_iterator(specified_supported_language.as_deref(), &args.use_paths())
        .for_each(|project_file_dir_entry| {
            let language = maybe_supported_language_from_path(project_file_dir_entry.path())
                .expect("Walker should've been pre-filtered to just supported file types");
            let query = return_if_none!(
                cached_queries.get_and_cache_query_for_language(&query_source, &*language)
            );
            let capture_index = return_if_none!(capture_index
                .get_or_initialize(&query, args.capture_name.as_deref())
                .ok());
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
