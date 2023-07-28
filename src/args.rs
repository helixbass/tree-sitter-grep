use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use clap::{ArgGroup, Parser};
use derive_builder::Builder;
use ignore::{types::Types, WalkBuilder, WalkParallel};
use rayon::iter::IterBridge;
use termcolor::BufferWriter;
use tree_sitter::Query;

use crate::{
    language::SupportedLanguage,
    plugin::{get_loaded_filter, Filterer},
    printer::StandardBuilder,
    project_file_walker::{
        get_project_file_walker_types, into_parallel_iterator, WalkParallelIterator,
    },
    searcher::{Searcher, SearcherBuilder},
    use_printer::Printer,
    Error, NonFatalError,
};

const ALL_NODES_QUERY: &str = "(_) @node";

#[derive(Builder, Clone, Default, Parser)]
#[builder(default, setter(strip_option, into))]
#[clap(group(
    ArgGroup::new("query_or_filter")
        .multiple(true)
        .required(true)
        .args(&["path_to_query_file", "query_text", "filter"])
))]
pub struct Args {
    paths: Vec<PathBuf>,

    /// The path to a tree-sitter query file.
    ///
    /// This conflicts with the --query option.
    #[arg(short = 'Q', long = "query-file", conflicts_with = "query_text")]
    path_to_query_file: Option<PathBuf>,

    /// The source text of a tree-sitter query.
    ///
    /// This conflicts with the --query-file option.
    #[arg(short, long = "query", conflicts_with = "path_to_query_file")]
    query_text: Option<String>,

    #[clap(skip)]
    query_per_language: Option<QueryPerLanguage>,

    /// The name of the tree-sitter query capture (without leading "@") whose
    /// matching nodes will be output.
    ///
    /// By default this is the "first" capture encountered in the query source
    /// text.
    #[arg(short, long = "capture")]
    pub capture_name: Option<String>,

    /// The target language for matching.
    ///
    /// By default all files corresponding to supported languages will be
    /// searched if the provided query can be successfully parsed for that
    /// language.
    #[arg(short, long, value_enum)]
    pub language: Option<SupportedLanguage>,

    /// The path to a dynamic library that can be used as a "filter plugin".
    ///
    /// Filter plugins allow for more fine-grained filtering of potentially
    /// matching tree-sitter AST nodes.
    #[arg(short, long, value_name = "PATH_TO_FILTER_PLUGIN_DYNAMIC_LIBRARY")]
    pub filter: Option<String>,

    /// An arbitrary argument to be passed to the specified filter plugin.
    ///
    /// It is up to the specific filter plugin whether it requires an argument
    /// to be passed (eg for self-configuration) and if so what format it
    /// expects that argument to be in.
    #[arg(short = 'a', long, requires = "filter")]
    pub filter_arg: Option<String>,

    /// Show results with every match on its own line, including line numbers
    /// and column numbers.
    ///
    /// With this option, a line with more that one match will be printed more
    /// than once.
    #[arg(long)]
    vimgrep: bool,

    /// Show NUM lines after each match.
    #[arg(short = 'A', long, value_name = "NUM")]
    pub after_context: Option<usize>,

    /// Show NUM lines before each match.
    #[arg(short = 'B', long, value_name = "NUM")]
    pub before_context: Option<usize>,

    /// Show NUM lines before and after each match.
    ///
    /// This is equivalent to providing both the -B/--before-context and
    /// -A/--after-context flags with the same value.
    #[arg(short = 'C', long, value_name = "NUM")]
    pub context: Option<usize>,

    /// Print only the matched (non-empty) parts of a matching line, with each
    /// such part on a separate output line.
    #[arg(short = 'o', long)]
    pub only_matching: bool,

    /// Print the 0-based byte offset within the input
    /// file before each line of output.
    ///
    /// If -o (--only-matching) is specified, print
    /// the offset of the matching part itself.
    #[arg(short = 'b', long)]
    pub byte_offset: bool,
}

impl Args {
    fn use_paths(&self) -> Vec<PathBuf> {
        if self.paths.is_empty() {
            vec![Path::new("./").to_owned()]
        } else {
            self.paths.clone()
        }
    }

    pub(crate) fn is_using_default_paths(&self) -> bool {
        self.paths.is_empty()
    }

    fn line_number(&self) -> bool {
        true
    }

    fn per_match(&self) -> bool {
        self.vimgrep
    }

    fn per_match_one_line(&self) -> bool {
        self.vimgrep
    }

    fn column(&self) -> bool {
        self.vimgrep
    }

    fn contexts(&self) -> (usize, usize) {
        let both = self.context.unwrap_or(0);
        if both > 0 {
            (both, both)
        } else {
            (
                self.before_context.unwrap_or(0),
                self.after_context.unwrap_or(0),
            )
        }
    }

    pub(crate) fn get_searcher(&self) -> Searcher {
        let (before_context, after_context) = self.contexts();
        SearcherBuilder::new()
            .line_number(self.line_number())
            .before_context(before_context)
            .after_context(after_context)
            .build()
    }

    pub(crate) fn get_printer(&self, buffer_writer: &BufferWriter) -> Printer {
        StandardBuilder::new()
            .per_match(self.per_match())
            .per_match_one_line(self.per_match_one_line())
            .column(self.column())
            .only_matching(self.only_matching)
            .byte_offset(self.byte_offset)
            .build(buffer_writer.buffer())
    }

    pub(crate) fn get_project_file_walker_types(&self) -> Types {
        get_project_file_walker_types(self.language.map(|language| vec![language]).or_else(|| {
            self.query_per_language
                .as_ref()
                .map(|query_per_language| query_per_language.keys().cloned().collect())
        }))
    }

    pub(crate) fn get_project_file_walker(&self) -> WalkParallel {
        let paths = self.use_paths();
        assert!(!paths.is_empty());
        let mut builder = WalkBuilder::new(&paths[0]);
        builder.types(self.get_project_file_walker_types());
        for path in &paths[1..] {
            builder.add(path);
        }
        builder.build_parallel()
    }

    pub(crate) fn get_project_file_parallel_iterator(
        &self,
        non_fatal_errors: Arc<Mutex<Vec<NonFatalError>>>,
    ) -> IterBridge<WalkParallelIterator> {
        into_parallel_iterator(self.get_project_file_walker(), non_fatal_errors)
    }

    pub(crate) fn get_loaded_filter(&self) -> Result<Option<Arc<Filterer>>, Error> {
        Ok(get_loaded_filter(self.filter.as_deref(), self.filter_arg.as_deref())?.map(Arc::new))
    }

    pub(crate) fn get_loaded_query_text_per_language(
        &self,
    ) -> Result<QueryOrQueryTextPerLanguage, Error> {
        Ok(
            match (
                self.path_to_query_file.as_ref(),
                self.query_text.as_ref(),
                self.query_per_language.as_ref(),
            ) {
                (Some(path_to_query_file), None, None) => fs::read_to_string(path_to_query_file)
                    .map_err(|source| Error::QueryFileReadError {
                        source,
                        path_to_query_file: path_to_query_file.clone(),
                    })?
                    .into(),
                (None, Some(query_text), None) => query_text.clone().into(),
                (None, None, Some(query_per_language)) => query_per_language.clone().into(),
                (None, None, None) => ALL_NODES_QUERY.to_owned().into(),
                _ => unreachable!(),
            },
        )
    }
}

impl ArgsBuilder {
    pub fn maybe_language(&mut self, language: Option<SupportedLanguage>) -> &mut Self {
        self.language = Some(language);
        self
    }
}

pub type QueryPerLanguage = HashMap<SupportedLanguage, Arc<Query>>;

pub enum QueryOrQueryTextPerLanguage {
    SingleQueryText(String),
    PerLanguage(QueryPerLanguage),
}

impl QueryOrQueryTextPerLanguage {
    pub fn get_query_or_query_text_for_language(
        &self,
        language: SupportedLanguage,
    ) -> QueryOrQueryText {
        match self {
            QueryOrQueryTextPerLanguage::SingleQueryText(query_text) => (&**query_text).into(),
            QueryOrQueryTextPerLanguage::PerLanguage(per_language) => {
                per_language.get(&language).unwrap().clone().into()
            }
        }
    }
}

impl From<String> for QueryOrQueryTextPerLanguage {
    fn from(value: String) -> Self {
        Self::SingleQueryText(value)
    }
}

impl From<QueryPerLanguage> for QueryOrQueryTextPerLanguage {
    fn from(value: QueryPerLanguage) -> Self {
        Self::PerLanguage(value)
    }
}

pub enum QueryOrQueryText<'a> {
    QueryText(&'a str),
    Query(Arc<Query>),
}

impl<'a> From<&'a str> for QueryOrQueryText<'a> {
    fn from(value: &'a str) -> Self {
        Self::QueryText(value)
    }
}

impl<'a> From<Arc<Query>> for QueryOrQueryText<'a> {
    fn from(value: Arc<Query>) -> Self {
        Self::Query(value)
    }
}
