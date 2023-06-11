use std::{
    cell::RefCell,
    fs,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{mpsc, mpsc::Receiver, Arc},
    thread,
    thread::JoinHandle,
};

use clap::Parser;
use grep::{
    matcher::{Match, Matcher, NoCaptures, NoError},
    printer::{Standard, StandardBuilder},
    searcher::{Searcher, SearcherBuilder},
};
use ignore::{types::TypesBuilder, DirEntry, WalkBuilder, WalkParallel, WalkState};
use once_cell::unsync::OnceCell;
use rayon::{iter::IterBridge, prelude::*};
use termcolor::{Buffer, BufferWriter, ColorChoice};
use tree_sitter::{Language, Query};

mod language;
mod macros;
mod plugin;
mod treesitter;

use language::{SupportedLanguage, SupportedLanguageName};
use plugin::get_loaded_filter;
use treesitter::{get_matches, get_query};

#[derive(Parser)]
pub struct Args {
    pub paths: Vec<PathBuf>,
    #[command(flatten)]
    pub query_args: QueryArgs,
    #[arg(short, long = "capture")]
    pub capture_name: Option<String>,
    #[arg(short, long, value_enum)]
    pub language: SupportedLanguageName,
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

pub fn run(args: Args) {
    let query_source = match args.query_args.path_to_query_file.as_ref() {
        Some(path_to_query_file) => fs::read_to_string(path_to_query_file).unwrap(),
        None => args.query_args.query_source.clone().unwrap(),
    };
    let supported_language = args.language.get_language();
    let language = supported_language.language();
    let query = Arc::new(get_query(&query_source, language));
    let capture_index = args.capture_name.as_ref().map_or(0, |capture_name| {
        query
            .capture_index_for_name(capture_name)
            .unwrap_or_else(|| panic!("Unknown capture name: `{}`", capture_name))
    });
    let output_mode = get_output_mode(&args);
    let buffer_writer = BufferWriter::stdout(ColorChoice::Never);

    get_project_file_walker(&*supported_language, &args.use_paths())
        .into_parallel_iterator()
        .for_each(|project_file_dir_entry| {
            let printer = get_printer(&buffer_writer, output_mode);
            let mut printer = printer.borrow_mut();
            let path =
                format_relative_path(project_file_dir_entry.path(), args.is_using_default_paths());

            let matcher = TreeSitterMatcher::new(
                &query,
                capture_index,
                language,
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

fn get_project_file_walker(language: &dyn SupportedLanguage, paths: &[PathBuf]) -> WalkParallel {
    assert!(!paths.is_empty());
    let mut builder = WalkBuilder::new(&paths[0]);
    builder.types(
        TypesBuilder::new()
            .add_defaults()
            .select(language.name_for_ignore_select())
            .build()
            .unwrap(),
    );
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
