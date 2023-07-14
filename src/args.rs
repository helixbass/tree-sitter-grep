use std::path::{Path, PathBuf};

use clap::{ArgGroup, Parser};
use ignore::{types::Types, WalkBuilder, WalkParallel};
use rayon::iter::IterBridge;
use termcolor::BufferWriter;

use crate::{
    language::SupportedLanguage,
    printer::StandardBuilder,
    project_file_walker::{
        get_project_file_walker_types, into_parallel_iterator, WalkParallelIterator,
    },
    searcher::{Searcher, SearcherBuilder},
    use_printer::Printer,
};

#[derive(Parser)]
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
    pub path_to_query_file: Option<PathBuf>,

    /// The source text of a tree-sitter query.
    ///
    /// This conflicts with the --query-file option.
    #[arg(short, long = "query", conflicts_with = "path_to_query_file")]
    pub query_text: Option<String>,

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
        get_project_file_walker_types(self.language)
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

    pub(crate) fn get_project_file_parallel_iterator(&self) -> IterBridge<WalkParallelIterator> {
        into_parallel_iterator(self.get_project_file_walker())
    }
}
