use std::path::{Path, PathBuf};

use clap::{ArgGroup, Parser};
use termcolor::BufferWriter;

use crate::{
    language::SupportedLanguageName,
    printer::StandardBuilder,
    searcher::{Searcher, SearcherBuilder},
    use_printer::Printer,
};

#[derive(Parser)]
#[clap(group(
    ArgGroup::new("query_or_filter")
        .multiple(true)
        .required(true)
        .args(&["path_to_query_file", "query_source", "filter"])
))]
pub struct Args {
    paths: Vec<PathBuf>,

    #[arg(short = 'Q', long = "query-file", conflicts_with = "query_source")]
    pub path_to_query_file: Option<PathBuf>,

    #[arg(short, long, conflicts_with = "path_to_query_file")]
    pub query_source: Option<String>,

    #[arg(short, long = "capture")]
    pub capture_name: Option<String>,

    #[arg(short, long, value_enum)]
    pub language: Option<SupportedLanguageName>,

    #[arg(short, long)]
    pub filter: Option<String>,

    #[arg(short = 'a', long, requires = "filter")]
    pub filter_arg: Option<String>,

    #[arg(long)]
    vimgrep: bool,

    #[arg(short = 'A', long, value_name = "NUM")]
    pub after_context: Option<usize>,

    #[arg(short = 'B', long, value_name = "NUM")]
    pub before_context: Option<usize>,

    #[arg(short = 'C', long, value_name = "NUM")]
    pub context: Option<usize>,
}

impl Args {
    pub(crate) fn use_paths(&self) -> Vec<PathBuf> {
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
            .build(buffer_writer.buffer())
    }
}
