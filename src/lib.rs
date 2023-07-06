#![allow(clippy::into_iter_on_ref)]

use std::{
    collections::HashMap,
    fmt, fs,
    path::Path,
    process,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, OnceLock,
    },
    thread,
    time::Duration,
};

use ignore::DirEntry;
use rayon::prelude::*;
use termcolor::{BufferWriter, ColorChoice};
use tree_sitter::Query;

mod args;
mod language;
mod line_buffer;
mod lines;
mod macros;
mod matcher;
mod messages;
mod plugin;
mod printer;
mod project_file_walker;
mod query_context;
mod searcher;
mod sink;
mod treesitter;
mod use_printer;
mod use_searcher;

pub use args::Args;
use language::{SupportedLanguage, SupportedLanguageName, ALL_SUPPORTED_LANGUAGES};
pub use plugin::PluginInitializeReturn;
use query_context::QueryContext;
use treesitter::maybe_get_query;
use use_printer::get_printer;
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

fn join_with_or<TItem: fmt::Display>(list: &[TItem]) -> String {
    let mut ret: String = Default::default();
    for (index, item) in list.iter().enumerate() {
        ret.push_str(&item.to_string());
        if list.len() >= 2 && index < list.len() - 2 {
            ret.push_str(", ");
        } else if list.len() >= 2 && index == list.len() - 2 {
            ret.push_str(" or ");
        }
    }
    ret
}

struct CachedQueries(HashMap<SupportedLanguageName, OnceLock<Option<Arc<Query>>>>);

impl CachedQueries {
    fn get_and_cache_query_for_language(
        &self,
        query_source: &str,
        language: SupportedLanguage,
    ) -> Option<Arc<Query>> {
        self.0
            .get(&language.name)
            .unwrap()
            .get_or_init(|| maybe_get_query(query_source, language.language).map(Arc::new))
            .clone()
    }

    fn error_if_no_successful_query_parsing(&self) {
        if !self
            .0
            .values()
            .any(|query| query.get().and_then(|option| option.as_ref()).is_some())
        {
            let mut attempted_parsings = self
                .0
                .iter()
                .filter(|(_, value)| value.get().is_some())
                .map(|(supported_language_name, _)| format!("{supported_language_name:?}"))
                .collect::<Vec<_>>();
            assert!(
                !attempted_parsings.is_empty(),
                "Should've tried to parse in at least one language or else should've already failed on no candidate files"
            );
            attempted_parsings.sort();
            fail(&format!(
                "couldn't parse query for {}",
                join_with_or(&attempted_parsings)
            ));
        }
    }
}

impl Default for CachedQueries {
    fn default() -> Self {
        Self(
            ALL_SUPPORTED_LANGUAGES
                .iter()
                .map(|supported_language| (supported_language.name, Default::default()))
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
    let cached_queries: CachedQueries = Default::default();
    let capture_index = CaptureIndex::default();
    let buffer_writer = BufferWriter::stdout(ColorChoice::Never);
    let matched = AtomicBool::new(false);
    let searched = AtomicBool::new(false);

    args.get_project_file_parallel_iterator().for_each(
        |(project_file_dir_entry, matched_languages)| {
            searched.store(true, Ordering::SeqCst);
            if matched_languages.is_empty() {
                match args.language() {
                    Some(language) => {
                        error_explicit_path_argument_not_of_specified_type(
                            &project_file_dir_entry,
                            language,
                        );
                    }
                    None => {
                        error_explicit_path_argument_not_of_known_type(&project_file_dir_entry);
                    }
                }
                return;
            }
            let language = return_if_none!(if matched_languages.len() > 1
                && args.language().is_none()
            {
                let mut successfully_parsed_query_languages =
                    matched_languages.iter().filter_map(|&matched_language| {
                        cached_queries
                            .get_and_cache_query_for_language(&query_source, matched_language)
                            .map(|_| matched_language)
                    });
                let maybe_first_matched_language = successfully_parsed_query_languages.next();
                match maybe_first_matched_language {
                    Some(first_matched_language) => {
                        let second_matched_language = successfully_parsed_query_languages.next();
                        if let Some(second_matched_language) = second_matched_language {
                            let mut all_matched_languages =
                                vec![first_matched_language, second_matched_language];
                            all_matched_languages.extend(successfully_parsed_query_languages);
                            error_disambiguate_language_for_file(
                                &project_file_dir_entry,
                                &all_matched_languages,
                            );
                            return;
                        }
                        Some(first_matched_language)
                    }
                    None => None,
                }
            } else {
                matched_languages.into_iter().find(|matched_language| {
                    !matches!(
                        args.language(),
                        Some(specified_language) if specified_language != *matched_language
                    )
                })
            });
            let query = return_if_none!(
                cached_queries.get_and_cache_query_for_language(&query_source, language)
            );
            let capture_index = capture_index.get_or_init(&query, args.capture_name.as_deref());
            let printer = get_printer(&buffer_writer, &args);
            let mut printer = printer.borrow_mut();
            let path =
                format_relative_path(project_file_dir_entry.path(), args.is_using_default_paths());

            let query_context = QueryContext::new(
                query,
                capture_index,
                language.language,
                args.filter.clone(),
                args.filter_arg.clone(),
            );

            printer.get_mut().clear();
            let mut sink = printer.sink_with_path(path);
            get_searcher(&args)
                .borrow_mut()
                .search_path(query_context, path, &mut sink)
                .unwrap();
            if sink.has_match() {
                matched.store(true, Ordering::SeqCst);
            }
            buffer_writer.print(printer.get_mut()).unwrap();
        },
    );

    if !messages::errored() {
        if !searched.load(Ordering::SeqCst) {
            eprint_nothing_searched();
        } else {
            cached_queries.error_if_no_successful_query_parsing();
        }
    }

    if messages::errored() {
        process::exit(2);
    } else if matched.load(Ordering::SeqCst) {
        process::exit(0);
    } else {
        process::exit(1);
    }
}

fn eprint_nothing_searched() {
    err_message!("No files were searched");
}

fn error_explicit_path_argument_not_of_known_type(project_file_dir_entry: &DirEntry) {
    // TODO: assert the assumed invariant that this was in fact an explicitly-passed
    // path?
    err_message!(
        "File {:?} does not belong to a recognized language",
        project_file_dir_entry.path()
    );
}

fn error_explicit_path_argument_not_of_specified_type(
    project_file_dir_entry: &DirEntry,
    language: SupportedLanguage,
) {
    // TODO: assert the assumed invariant that this was in fact an explicitly-passed
    // path?
    err_message!(
        "File {:?} is not recognized as a {:?} file",
        project_file_dir_entry.path(),
        language.name
    );
}

#[macro_export]
macro_rules! only_run_once {
    ($block:block) => {
        static ONCE_LOCK: std::sync::OnceLock<()> = OnceLock::new();
        ONCE_LOCK.get_or_init(|| {
            $block;
        });
    };
}

fn error_disambiguate_language_for_file(
    project_file_dir_entry: &DirEntry,
    all_matched_languages: &[SupportedLanguage],
) {
    only_run_once!({
        err_message!(
            "File {:?} has ambiguous file-type, could be {}",
            project_file_dir_entry.path(),
            join_with_or(
                &all_matched_languages
                    .into_iter()
                    .map(|matched_language| format!("{:?}", matched_language.name))
                    .collect::<Vec<_>>()
            )
        );
    });
}

fn fail(message: &str) -> ! {
    eprintln!("error: {message}");
    process::exit(2);
}

fn format_relative_path(path: &Path, is_using_default_paths: bool) -> &Path {
    if is_using_default_paths && path.starts_with("./") {
        path.strip_prefix("./").unwrap()
    } else {
        path
    }
}
