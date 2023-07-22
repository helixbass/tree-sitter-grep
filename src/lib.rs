#![allow(clippy::into_iter_on_ref)]

use std::{
    fmt, io,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock, RwLock,
    },
};

use ignore::DirEntry;
use rayon::prelude::*;
use termcolor::{BufferWriter, ColorChoice};
use thiserror::Error;
use tree_sitter::{Node, Query, QueryError};

mod args;
mod language;
mod line_buffer;
mod lines;
mod macros;
mod matcher;
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
use language::BySupportedLanguage;
pub use language::SupportedLanguage;
pub use plugin::PluginInitializeReturn;
use query_context::QueryContext;
use treesitter::maybe_get_query;
use use_printer::get_printer;
use use_searcher::get_searcher;

#[derive(Debug, Error)]
pub enum Error {
    #[error("couldn't read query file {path_to_query_file:?}")]
    QueryFileReadError {
        path_to_query_file: PathBuf,
        source: io::Error,
    },
    #[error("{}",
        match .0.len() {
            1 => {
                let (supported_language, query_error) = &.0[0];
                format!("couldn't parse query for {supported_language:?}: {query_error}")
            }
            _ => {
                let mut attempted_parsings = .0
                    .iter()
                    .map(|(supported_language, _)| format!("{supported_language:?}"))
                    .collect::<Vec<_>>();
                attempted_parsings.sort();
                format!(
                    "couldn't parse query for {}",
                    join_with_or(&attempted_parsings)
                )
            }
        }
    )]
    NoSuccessfulQueryParsing(Vec<(SupportedLanguage, QueryError)>),
    #[error("query must include at least one capture (\"@whatever\")")]
    NoCaptureInQuery,
    #[error("invalid capture name '{capture_name}'")]
    InvalidCaptureName { capture_name: String },
    #[error("plugin expected '--filter-arg <ARGUMENT>'")]
    FilterPluginExpectedArgument,
    #[error("plugin couldn't parse argument {filter_arg:?}")]
    FilterPluginCouldntParseArgument { filter_arg: String },
    #[error("language is required when passing a slice")]
    LanguageMissingForSlice,
}

#[derive(Clone, Debug, Error)]
pub enum NonFatalError {
    #[error("File {path:?} is not recognized as a {specified_language:?} file")]
    ExplicitPathArgumentNotOfSpecifiedType {
        path: PathBuf,
        specified_language: SupportedLanguage,
    },
    #[error("File {path:?} does not belong to a recognized language")]
    ExplicitPathArgumentNotOfKnownType { path: PathBuf },
    #[error(
        "File {path:?} has ambiguous file-type, could be {}. Try passing the --language flag",
        join_with_or(
            &.languages
                .into_iter()
                .map(|language| format!("{}", language))
                .collect::<Vec<_>>()
        )
    )]
    AmbiguousLanguageForFile {
        path: PathBuf,
        languages: Vec<SupportedLanguage>,
    },
    #[error("No files were searched")]
    NothingSearched,
    #[error("{error}")]
    IgnoreError {
        #[from]
        error: ignore::Error,
    },
}

#[derive(Clone)]
enum CaptureIndexError {
    NoCaptureInQuery,
    InvalidCaptureName { capture_name: String },
}

impl From<CaptureIndexError> for Error {
    fn from(value: CaptureIndexError) -> Self {
        match value {
            CaptureIndexError::NoCaptureInQuery => Self::NoCaptureInQuery,
            CaptureIndexError::InvalidCaptureName { capture_name } => {
                Self::InvalidCaptureName { capture_name }
            }
        }
    }
}

#[derive(Default)]
struct CaptureIndex(OnceLock<Result<u32, CaptureIndexError>>);

impl CaptureIndex {
    pub fn get_or_init(
        &self,
        query: &Query,
        capture_name: Option<&str>,
    ) -> Result<u32, CaptureIndexError> {
        self.0
            .get_or_init(|| match capture_name {
                None => match query.capture_names().len() {
                    0 => Err(CaptureIndexError::NoCaptureInQuery),
                    _ => Ok(0),
                },
                Some(capture_name) => query.capture_index_for_name(capture_name).ok_or_else(|| {
                    CaptureIndexError::InvalidCaptureName {
                        capture_name: capture_name.to_owned(),
                    }
                }),
            })
            .clone()
    }
}

fn join_with_or<TItem: fmt::Display>(list: &[TItem]) -> String {
    let mut ret: String = Default::default();
    for (index, item) in list.iter().enumerate() {
        ret.push_str(&item.to_string());
        if list.len() >= 2 && index < list.len() - 2 {
            ret.push_str(", ");
        } else if list.len() >= 2 && index == list.len() - 2 {
            ret.push_str(if list.len() == 2 { " or " } else { ", or " });
        }
    }
    ret
}

#[derive(Default)]
struct CachedQueries(BySupportedLanguage<OnceLock<Result<Arc<Query>, QueryError>>>);

impl CachedQueries {
    fn get_and_cache_query_for_language(
        &self,
        query_text: &str,
        language: SupportedLanguage,
    ) -> Option<Arc<Query>> {
        self.0[language]
            .get_or_init(|| maybe_get_query(query_text, language.language()).map(Arc::new))
            .as_ref()
            .ok()
            .cloned()
    }

    fn error_if_no_successful_query_parsing(self) -> Result<(), Error> {
        if !self.0.values().any(|query| {
            query
                .get()
                .and_then(|result| result.as_ref().ok())
                .is_some()
        }) {
            let attempted_parsings = self
                .0
                .into_iter()
                .filter(|(_, value)| value.get().is_some())
                .map(|(supported_language, once_lock)| {
                    (
                        supported_language,
                        once_lock.into_inner().unwrap().unwrap_err(),
                    )
                })
                .collect::<Vec<_>>();
            assert!(
                !attempted_parsings.is_empty(),
                "Should've tried to parse in at least one language or else should've already failed on no candidate files"
            );
            return Err(Error::NoSuccessfulQueryParsing(attempted_parsings));
        }

        Ok(())
    }
}

pub struct RunStatus {
    pub matched: bool,
    pub non_fatal_errors: Vec<NonFatalError>,
}

enum SingleFileSearchError {
    NonFatalSearchError(NonFatalError),
    FatalError(Error),
}

impl From<Error> for SingleFileSearchError {
    fn from(value: Error) -> Self {
        Self::FatalError(value)
    }
}

impl From<NonFatalError> for SingleFileSearchError {
    fn from(value: NonFatalError) -> Self {
        Self::NonFatalSearchError(value)
    }
}

impl From<CaptureIndexError> for SingleFileSearchError {
    fn from(value: CaptureIndexError) -> Self {
        Self::FatalError(value.into())
    }
}

enum SingleFileSearchNonFailure {
    QueryNotParseableForFile,
    RanQuery,
}

type SingleFileSearchResult = Result<SingleFileSearchNonFailure, SingleFileSearchError>;

impl From<Error> for SingleFileSearchResult {
    fn from(value: Error) -> Self {
        Err(value.into())
    }
}

impl From<NonFatalError> for SingleFileSearchResult {
    fn from(value: NonFatalError) -> Self {
        Err(value.into())
    }
}

pub struct OutputContext {
    pub buffer_writer: BufferWriter,
}

impl OutputContext {
    pub fn new(buffer_writer: BufferWriter) -> Self {
        Self { buffer_writer }
    }
}

pub fn run_print(args: Args) -> Result<RunStatus, Error> {
    run_for_context(
        args,
        OutputContext::new(BufferWriter::stdout(ColorChoice::Never)),
        |context: &OutputContext,
         args: &Args,
         path: &Path,
         query_context: QueryContext,
         matched: &AtomicBool| {
            let printer = get_printer(&context.buffer_writer, args);
            let mut printer = printer.borrow_mut();

            printer.get_mut().clear();
            let mut sink = printer.sink_with_path(path);
            get_searcher(args)
                .borrow_mut()
                .search_path(query_context, path, &mut sink)
                .unwrap();
            if sink.has_match() {
                matched.store(true, Ordering::SeqCst);
            }
            context.buffer_writer.print(printer.get_mut()).unwrap();
        },
    )
}

pub struct CaptureInfo<'node> {
    pub node: Node<'node>,
    pub pattern_index: usize,
}

pub fn run_with_callback(
    args: Args,
    callback: impl Fn(CaptureInfo, &[u8], &Path) + Sync,
) -> Result<RunStatus, Error> {
    run_for_context(
        args,
        (),
        |_context: &(),
         args: &Args,
         path: &Path,
         query_context: QueryContext,
         matched: &AtomicBool| {
            get_searcher(args)
                .borrow_mut()
                .search_path_callback::<_, io::Error>(
                    query_context,
                    path,
                    |capture_info: CaptureInfo, file_contents: &[u8], path: &Path| {
                        callback(capture_info, file_contents, path);
                        matched.store(true, Ordering::SeqCst);
                    },
                )
                .unwrap();
        },
    )
}

fn run_for_context<TContext: Sync>(
    args: Args,
    context: TContext,
    search_file: impl Fn(&TContext, &Args, &Path, QueryContext, &AtomicBool) + Sync,
) -> Result<RunStatus, Error> {
    let query_text = args.get_loaded_query_text()?;
    let filter = args.get_loaded_filter()?;
    let cached_queries: CachedQueries = Default::default();
    let capture_index = CaptureIndex::default();
    let matched = AtomicBool::new(false);
    let searched = AtomicBool::new(false);
    let non_fatal_errors: Arc<Mutex<Vec<NonFatalError>>> = Default::default();

    for_each_project_file(
        &args,
        non_fatal_errors.clone(),
        |project_file_dir_entry, matched_languages| {
            searched.store(true, Ordering::SeqCst);
            let language = match args.language {
                Some(specified_language) => {
                    if !matched_languages.contains(&specified_language) {
                        return NonFatalError::ExplicitPathArgumentNotOfSpecifiedType {
                            path: project_file_dir_entry.path().to_owned(),
                            specified_language,
                        }
                        .into();
                    }
                    specified_language
                }
                None => match matched_languages.len() {
                    0 => {
                        return NonFatalError::ExplicitPathArgumentNotOfKnownType {
                            path: project_file_dir_entry.path().to_owned(),
                        }
                        .into();
                    }
                    1 => matched_languages[0],
                    _ => {
                        let successfully_parsed_query_languages = matched_languages
                            .iter()
                            .filter_map(|&matched_language| {
                                cached_queries
                                    .get_and_cache_query_for_language(&query_text, matched_language)
                                    .map(|_| matched_language)
                            })
                            .collect::<Vec<_>>();
                        match successfully_parsed_query_languages.len() {
                            0 => {
                                return Ok(SingleFileSearchNonFailure::QueryNotParseableForFile);
                            }
                            1 => successfully_parsed_query_languages[0],
                            _ => {
                                return NonFatalError::AmbiguousLanguageForFile {
                                    path: project_file_dir_entry.path().to_owned(),
                                    languages: successfully_parsed_query_languages,
                                }
                                .into();
                            }
                        }
                    }
                },
            };
            let query = match cached_queries.get_and_cache_query_for_language(&query_text, language)
            {
                Some(query) => query,
                None => return Ok(SingleFileSearchNonFailure::QueryNotParseableForFile),
            };
            let capture_index = capture_index.get_or_init(&query, args.capture_name.as_deref())?;
            let path =
                format_relative_path(project_file_dir_entry.path(), args.is_using_default_paths());

            let query_context =
                QueryContext::new(query, capture_index, language.language(), filter.clone());

            search_file(&context, &args, path, query_context, &matched);

            Ok(SingleFileSearchNonFailure::RanQuery)
        },
    )?;

    let mut non_fatal_errors = non_fatal_errors.lock().unwrap().clone();
    if non_fatal_errors.is_empty() {
        if !searched.load(Ordering::SeqCst) {
            non_fatal_errors.push(NonFatalError::NothingSearched);
        } else {
            cached_queries.error_if_no_successful_query_parsing()?;
        }
    }

    Ok(RunStatus {
        matched: matched.load(Ordering::SeqCst),
        non_fatal_errors,
    })
}

pub fn run_for_slice_with_callback(
    slice: &[u8],
    args: Args,
    mut callback: impl FnMut(CaptureInfo) + Sync,
) -> Result<RunStatus, Error> {
    let language = args.language.ok_or(Error::LanguageMissingForSlice)?;
    let query_text = args.get_loaded_query_text()?;
    let filter = args.get_loaded_filter()?;
    let cached_queries: CachedQueries = Default::default();
    let capture_index = CaptureIndex::default();
    let matched = AtomicBool::new(false);
    let non_fatal_errors: Arc<Mutex<Vec<NonFatalError>>> = Default::default();

    let query = match cached_queries.get_and_cache_query_for_language(&query_text, language) {
        Some(query) => query,
        None => {
            return Err(cached_queries
                .error_if_no_successful_query_parsing()
                .unwrap_err())
        }
    };
    let capture_index = capture_index.get_or_init(&query, args.capture_name.as_deref())?;

    let query_context = QueryContext::new(query, capture_index, language.language(), filter);

    get_searcher(&args)
        .borrow_mut()
        .search_slice_callback_no_path(query_context, slice, |capture_info: CaptureInfo| {
            callback(capture_info);
            matched.store(true, Ordering::SeqCst);
        })
        .unwrap();

    let non_fatal_errors = non_fatal_errors.lock().unwrap().clone();
    if non_fatal_errors.is_empty() {
        cached_queries.error_if_no_successful_query_parsing()?;
    }

    Ok(RunStatus {
        matched: matched.load(Ordering::SeqCst),
        non_fatal_errors,
    })
}

fn for_each_project_file(
    args: &Args,
    non_fatal_errors: Arc<Mutex<Vec<NonFatalError>>>,
    callback: impl Fn(DirEntry, Vec<SupportedLanguage>) -> SingleFileSearchResult + Sync,
) -> Result<(), Error> {
    let fatal_error: RwLock<Option<Error>> = Default::default();
    args.get_project_file_parallel_iterator(non_fatal_errors.clone())
        .for_each(|(project_file_dir_entry, matched_languages)| {
            if fatal_error.read().unwrap().is_some() {
                return;
            }

            if let Err(error) = callback(project_file_dir_entry, matched_languages) {
                match error {
                    SingleFileSearchError::NonFatalSearchError(error) => {
                        non_fatal_errors.lock().unwrap().push(error);
                    }
                    SingleFileSearchError::FatalError(error) => {
                        *fatal_error.write().unwrap() = Some(error);
                    }
                }
            }
        });

    match fatal_error.into_inner().unwrap() {
        Some(fatal_error) => Err(fatal_error),
        None => Ok(()),
    }
}

fn format_relative_path(path: &Path, is_using_default_paths: bool) -> &Path {
    if is_using_default_paths && path.starts_with("./") {
        path.strip_prefix("./").unwrap()
    } else {
        path
    }
}
