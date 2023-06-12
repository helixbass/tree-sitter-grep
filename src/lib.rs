use std::{
    cell::{OnceCell, RefCell},
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process,
    rc::Rc,
    sync::{
        atomic::{AtomicU32, Ordering},
        mpsc,
        mpsc::Receiver,
        Arc, Mutex,
    },
    thread,
    thread::JoinHandle,
    time::Duration,
};

use clap::Parser;
use grep::{
    matcher::{Match, Matcher, NoCaptures, NoError},
    printer::{Standard, StandardBuilder},
    searcher::{Searcher, SearcherBuilder},
};
use ignore::{types::TypesBuilder, DirEntry, WalkBuilder, WalkParallel, WalkState};
use rayon::{iter::IterBridge, prelude::*};
use termcolor::{Buffer, BufferWriter, ColorChoice};
use tree_sitter::{Language, Query};

mod language;
mod macros;
mod plugin;
mod treesitter;

use language::{
    get_all_supported_languages, maybe_supported_language_from_path, SupportedLanguage,
    SupportedLanguageName,
};
use plugin::get_loaded_filter;
use treesitter::{get_matches, maybe_get_query};

#[derive(Parser)]
pub struct Args {
    pub paths: Vec<PathBuf>,
    #[command(flatten)]
    pub query_args: QueryArgs,
    #[arg(short, long = "capture")]
    pub capture_name: Option<String>,
    #[arg(short, long, value_enum)]
    pub language: Option<SupportedLanguageName>,
    #[arg(short, long)]
    pub filter: Option<String>,
    #[arg(short = 'a', long)]
    pub filter_arg: Option<String>,
    #[arg(long)]
    pub vimgrep: bool,
}

impl Args {
    pub fn use_paths(&self) -> Vec<PathBuf> {
        if self.paths.is_empty() {
            vec![Path::new("./").to_owned()]
        } else {
            self.paths.clone()
        }
    }

    pub fn is_using_default_paths(&self) -> bool {
        self.paths.is_empty()
    }
}

#[derive(clap::Args)]
#[group(required = true, multiple = false)]
pub struct QueryArgs {
    #[arg(short = 'Q', long = "query-file")]
    pub path_to_query_file: Option<PathBuf>,
    #[arg(short, long)]
    pub query_source: Option<String>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum OutputMode {
    Normal,
    Vimgrep,
}

fn get_output_mode(args: &Args) -> OutputMode {
    if args.vimgrep {
        OutputMode::Vimgrep
    } else {
        OutputMode::Normal
    }
}

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

pub fn run(args: Args) {
    let query_source = match args.query_args.path_to_query_file.as_ref() {
        Some(path_to_query_file) => fs::read_to_string(path_to_query_file)
            .unwrap_or_else(|_| fail(&format!("couldn't read query file {path_to_query_file:?}"))),
        None => args.query_args.query_source.clone().unwrap(),
    };
    let specified_supported_language = args.language.map(|language| language.get_language());
    let query_or_failure_by_language: Mutex<HashMap<SupportedLanguageName, Option<Arc<Query>>>> =
        Default::default();
    let capture_index = MaybeInitializedCaptureIndex::default();
    let output_mode = get_output_mode(&args);
    let buffer_writer = BufferWriter::stdout(ColorChoice::Never);

    get_project_file_walker(specified_supported_language.as_deref(), &args.use_paths())
        .into_parallel_iterator()
        .for_each(|project_file_dir_entry| {
            let language = maybe_supported_language_from_path(project_file_dir_entry.path())
                .expect("Walker should've been pre-filtered to just supported file types");
            let query = return_if_none!(get_and_cache_query_for_language(
                &query_source,
                &query_or_failure_by_language,
                &*language,
            ));
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

    error_if_no_successful_query_parsing(&query_or_failure_by_language);
}

fn error_if_no_successful_query_parsing(
    query_or_failure_by_language: &Mutex<HashMap<SupportedLanguageName, Option<Arc<Query>>>>,
) {
    let query_or_failure_by_language = query_or_failure_by_language.lock().unwrap();
    if !query_or_failure_by_language
        .values()
        .any(|query| query.is_some())
    {
        fail("invalid query");
    }
}

fn fail(message: &str) -> ! {
    eprintln!("error: {message}");
    process::exit(1);
}

fn get_and_cache_query_for_language(
    query_source: &str,
    query_or_failure_by_language: &Mutex<HashMap<SupportedLanguageName, Option<Arc<Query>>>>,
    language: &dyn SupportedLanguage,
) -> Option<Arc<Query>> {
    query_or_failure_by_language
        .lock()
        .unwrap()
        .entry(language.name())
        .or_insert_with(|| maybe_get_query(query_source, language.language()).map(Arc::new))
        .clone()
}

type Printer = Standard<Buffer>;

thread_local! {
    static PRINTER: OnceCell<(Rc<RefCell<Printer>>, OutputMode)> = Default::default();
}
fn get_printer(buffer_writer: &BufferWriter, output_mode: OutputMode) -> Rc<RefCell<Printer>> {
    PRINTER.with(|printer| {
        let (printer, output_mode_when_initialized) = printer.get_or_init(|| {
            (
                Rc::new(RefCell::new(create_printer(buffer_writer, output_mode))),
                output_mode,
            )
        });
        assert!(
            *output_mode_when_initialized == output_mode,
            "Using multiple output modes not supported"
        );
        printer.clone()
    })
}

fn create_printer(buffer_writer: &BufferWriter, output_mode: OutputMode) -> Printer {
    match output_mode {
        OutputMode::Normal => Standard::new(buffer_writer.buffer()),
        OutputMode::Vimgrep => StandardBuilder::new()
            .per_match(true)
            .per_match_one_line(true)
            .column(true)
            .build(buffer_writer.buffer()),
    }
}

thread_local! {
    static SEARCHER: OnceCell<(Rc<RefCell<Searcher>>, OutputMode)> = Default::default();
}
fn get_searcher(output_mode: OutputMode) -> Rc<RefCell<Searcher>> {
    SEARCHER.with(|searcher| {
        let (searcher, output_mode_when_initialized) = searcher.get_or_init(|| {
            (
                Rc::new(RefCell::new(create_searcher(output_mode))),
                output_mode,
            )
        });
        assert!(
            *output_mode_when_initialized == output_mode,
            "Using multiple output modes not supported"
        );
        searcher.clone()
    })
}

fn create_searcher(output_mode: OutputMode) -> Searcher {
    match output_mode {
        OutputMode::Normal => SearcherBuilder::new().multi_line(true).build(),
        OutputMode::Vimgrep => SearcherBuilder::new()
            .multi_line(true)
            .line_number(true)
            .build(),
    }
}

fn format_relative_path(path: &Path, is_using_default_paths: bool) -> &Path {
    if is_using_default_paths && path.starts_with("./") {
        path.strip_prefix("./").unwrap()
    } else {
        path
    }
}

trait IntoParallelIterator {
    fn into_parallel_iterator(self) -> IterBridge<WalkParallelIterator>;
}

impl IntoParallelIterator for WalkParallel {
    fn into_parallel_iterator(self) -> IterBridge<WalkParallelIterator> {
        WalkParallelIterator::new(self).par_bridge()
    }
}

struct WalkParallelIterator {
    receiver_iterator: <Receiver<DirEntry> as IntoIterator>::IntoIter,
    _handle: JoinHandle<()>,
}

impl WalkParallelIterator {
    pub fn new(walk_parallel: WalkParallel) -> Self {
        let (sender, receiver) = mpsc::channel::<DirEntry>();
        let handle = thread::spawn(move || {
            walk_parallel.run(move || {
                Box::new({
                    let sender = sender.clone();
                    move |entry| {
                        let entry = match entry {
                            Err(_) => return WalkState::Continue,
                            Ok(entry) => entry,
                        };
                        if !entry.metadata().unwrap().is_file() {
                            return WalkState::Continue;
                        }
                        sender.send(entry).unwrap();
                        WalkState::Continue
                    }
                })
            });
        });
        Self {
            receiver_iterator: receiver.into_iter(),
            _handle: handle,
        }
    }
}

impl Iterator for WalkParallelIterator {
    type Item = DirEntry;

    fn next(&mut self) -> Option<Self::Item> {
        self.receiver_iterator.next()
    }
}

fn get_project_file_walker(
    language: Option<&dyn SupportedLanguage>,
    paths: &[PathBuf],
) -> WalkParallel {
    assert!(!paths.is_empty());
    let mut builder = WalkBuilder::new(&paths[0]);
    let mut types_builder = TypesBuilder::new();
    types_builder.add_defaults();
    if let Some(language) = language {
        types_builder.select(language.name_for_ignore_select());
    } else {
        for language in get_all_supported_languages().values() {
            types_builder.select(language.name_for_ignore_select());
        }
    }
    builder.types(types_builder.build().unwrap());
    for path in &paths[1..] {
        builder.add(path);
    }
    builder.build_parallel()
}

#[derive(Debug)]
struct TreeSitterMatcher<'query> {
    query: &'query Query,
    capture_index: u32,
    language: Language,
    filter_library_path: Option<String>,
    filter_arg: Option<String>,
    matches_info: RefCell<Option<PopulatedMatchesInfo>>,
}

impl<'query> TreeSitterMatcher<'query> {
    fn new(
        query: &'query Query,
        capture_index: u32,
        language: Language,
        filter_library_path: Option<String>,
        filter_arg: Option<String>,
    ) -> Self {
        Self {
            query,
            capture_index,
            language,
            filter_library_path,
            filter_arg,
            matches_info: Default::default(),
        }
    }
}

impl Matcher for TreeSitterMatcher<'_> {
    type Captures = NoCaptures;

    type Error = NoError;

    fn find_at(&self, haystack: &[u8], at: usize) -> Result<Option<Match>, Self::Error> {
        let mut matches_info = self.matches_info.borrow_mut();
        let matches_info = matches_info.get_or_insert_with(|| {
            assert!(at == 0);
            PopulatedMatchesInfo {
                matches: get_matches(
                    self.query,
                    self.capture_index,
                    haystack,
                    self.language,
                    get_loaded_filter(
                        self.filter_library_path.as_deref(),
                        self.filter_arg.as_deref(),
                    ),
                ),
                text_len: haystack.len(),
            }
        });
        Ok(matches_info.find_and_adjust_first_in_range_match(haystack.len(), at))
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

impl PopulatedMatchesInfo {
    pub fn find_and_adjust_first_in_range_match(
        &self,
        haystack_len: usize,
        at: usize,
    ) -> Option<Match> {
        self.find_first_in_range_match(haystack_len, at)
            .map(|match_| adjust_match(match_, haystack_len, self.text_len))
    }

    pub fn find_first_in_range_match(&self, haystack_len: usize, at: usize) -> Option<&Match> {
        let start_index = at + (self.text_len - haystack_len);
        self.matches
            .iter()
            .find(|match_| match_.start() >= start_index)
    }
}

fn adjust_match(match_: &Match, haystack_len: usize, total_file_text_len: usize) -> Match {
    let offset_in_file = total_file_text_len - haystack_len;
    Match::new(
        match_.start() - offset_in_file,
        match_.end() - offset_in_file,
    )
}
