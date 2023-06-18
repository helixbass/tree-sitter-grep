use std::path::{Path, PathBuf};

use clap::{ArgGroup, Parser};

use crate::language::SupportedLanguageName;

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

    pub(crate) fn output_mode(&self) -> OutputMode {
        if self.vimgrep {
            OutputMode::Vimgrep
        } else {
            OutputMode::Normal
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum OutputMode {
    Normal,
    Vimgrep,
}
