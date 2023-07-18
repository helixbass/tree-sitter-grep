use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use clap::{ArgGroup, Parser, ValueEnum};
use ignore::{types::Types, WalkBuilder, WalkParallel};
use rayon::iter::IterBridge;
use termcolor::{BufferWriter, ColorChoice};

use crate::{
    language::SupportedLanguage,
    printer::{default_color_specs, ColorSpecs, StandardBuilder, UserColorSpec},
    project_file_walker::{
        get_project_file_walker_types, into_parallel_iterator, WalkParallelIterator,
    },
    searcher::{Searcher, SearcherBuilder},
    use_printer::Printer,
    NonFatalError,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum ColorChoiceArg {
    /// Colors will never be used.
    Never,
    /// The default. tree-sitter-grep tries to be smart.
    Auto,
    /// Colors will always be used regardless of where output is sent.
    Always,
    /// Like 'always', but emits ANSI escapes (even in a Windows console).
    Ansi,
}

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

    /// This flag specifies color settings for use in the output.
    ///
    /// This flag may be provided multiple times. Settings are applied
    /// iteratively. Colors are limited to one of eight choices: red, blue,
    /// green, cyan, magenta, yellow, white and black. Styles are limited to
    /// nobold, bold, nointense, intense, nounderline or underline.
    ///
    /// The format of the flag is '{type}:{attribute}:{value}'. '{type}' should
    /// be one of path, line, column or match. '{attribute}' can be fg, bg
    /// or style. '{value}' is either a color (for fg and bg) or a text
    /// style. A special format, '{type}:none', will clear all color
    /// settings for '{type}'.
    ///
    /// For example, the following command will change the match color to
    /// magenta and the background color for line numbers to yellow:
    ///
    /// tree-sitter-grep --colors 'match:fg:magenta' --colors 'line:bg:yellow'
    /// -q '(function_item) @f'
    ///
    /// Extended colors can be used for '{value}' when the terminal supports
    /// ANSI color sequences. These are specified as either 'x' (256-color)
    /// or 'x,x,x' (24-bit truecolor) where x is a number between 0 and 255
    /// inclusive. x may be given as a normal decimal number or a
    /// hexadecimal number, which is prefixed by `0x`.
    ///
    /// For example, the following command will change the match background
    /// color to that represented by the rgb value (0,128,255):
    ///
    /// tree-sitter-grep --colors 'match:bg:0,128,255'
    ///
    /// or, equivalently,
    ///
    /// tree-sitter-grep --colors 'match:bg:0x0,0x80,0xFF'
    ///
    /// Note that the intense and nointense style flags will have no effect when
    /// used alongside these extended color codes.
    #[arg(long)]
    pub colors: Vec<UserColorSpec>,

    /// This flag controls when to use colors.
    ///
    /// The default setting is 'auto', which means tree-sitter-grep will try to
    /// guess when to use colors. For example, if tree-sitter-grep is printing
    /// to a terminal, then it will use colors, but if it is redirected to a
    /// file or a pipe, then it will suppress color output. tree-sitter-grep
    /// will suppress color output in some other circumstances as well. For
    /// example, if the TERM environment variable is not set or set to 'dumb',
    /// then tree-sitter-grep will not use colors.
    ///
    /// When the --vimgrep flag is given to tree-sitter-grep, then the default
    /// value for the --color flag changes to 'never'.
    #[arg(long, value_name = "WHEN"/*, default_value_t = ColorChoiceArg::Auto*/)]
    pub color: Option<ColorChoiceArg>,

    /// This is a convenience alias for '--color always --heading
    /// --line-number'.
    ///
    /// This flag is useful when you still want pretty output even if you're
    /// piping tree-sitter-grep to another program or file. For example:
    /// 'tree-sitter-grep -p -q "(function_item) @c" | less -R'.
    #[arg(short = 'p', long)]
    pub pretty: bool,

    /// This flag prints the file path above clusters of matches from each file
    /// instead of printing the file path as a prefix for each matched line.
    ///
    /// This is the default mode when printing to a terminal.
    ///
    /// This overrides the --no-heading flag.
    #[arg(long)]
    pub heading: bool,

    /// Don't group matches by each file.
    ///
    /// If --no-heading is provided in addition to the -H/--with-filename flag,
    /// then file paths will be printed as a prefix for every matched line.
    /// This is the default mode when not printing to a terminal.
    ///
    /// This overrides the --heading flag.
    #[arg(long, overrides_with = "heading")]
    pub no_heading: bool,

    /// Display the file path for matches.
    ///
    /// This is the default when more than one file is searched. If --heading is
    /// enabled (the default when printing to a terminal), the file path will be
    /// shown above clusters of matches from each file; otherwise, the file name
    /// will be shown as a prefix for each matched line.
    ///
    /// This flag overrides --no-filename.
    #[arg(short = 'H', long)]
    pub with_filename: bool,

    /// Never print the file path with the matched lines.
    ///
    /// This is the default when tree-sitter-grep is explicitly instructed to
    /// search one file or stdin.
    ///
    /// This flag overrides --with-filename.
    #[arg(short = 'I', long, overrides_with = "with_filename")]
    pub no_filename: bool,
}

impl Args {
    pub fn use_paths(&self) -> Vec<PathBuf> {
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

    pub(crate) fn get_printer(&self, paths: &[PathBuf], buffer_writer: &BufferWriter) -> Printer {
        StandardBuilder::new()
            .color_specs(self.color_specs())
            .heading(self.heading())
            .path(self.with_filename(paths))
            .per_match(self.per_match())
            .per_match_one_line(self.per_match_one_line())
            .column(self.column())
            .only_matching(self.only_matching)
            .byte_offset(self.byte_offset)
            .separator_context(self.context_separator())
            .separator_search(None)
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

    pub(crate) fn get_project_file_parallel_iterator(
        &self,
        non_fatal_errors: Arc<Mutex<Vec<NonFatalError>>>,
    ) -> IterBridge<WalkParallelIterator> {
        into_parallel_iterator(self.get_project_file_walker(), non_fatal_errors)
    }

    pub fn color_specs(&self) -> ColorSpecs {
        let mut specs = default_color_specs();
        for user_color_spec in &self.colors {
            specs.push(user_color_spec.clone());
        }
        ColorSpecs::new(&specs)
    }

    pub fn buffer_writer(&self) -> BufferWriter {
        let mut wtr = BufferWriter::stdout(self.color_choice());
        wtr.separator(self.file_separator());
        wtr
    }

    fn color_choice(&self) -> ColorChoice {
        match self.color.unwrap_or(ColorChoiceArg::Auto) {
            ColorChoiceArg::Always => ColorChoice::Always,
            ColorChoiceArg::Ansi => ColorChoice::AlwaysAnsi,
            ColorChoiceArg::Auto => {
                if grep_cli::is_tty_stdout() || self.pretty {
                    ColorChoice::Auto
                } else {
                    ColorChoice::Never
                }
            }
            ColorChoiceArg::Never => ColorChoice::Never,
        }
    }

    fn heading(&self) -> bool {
        if self.no_heading || self.vimgrep {
            false
        } else {
            grep_cli::is_tty_stdout() || self.heading || self.pretty
        }
    }

    fn file_separator(&self) -> Option<Vec<u8>> {
        // if self.output_kind() != OutputKind::Standard {
        //     return Ok(None);
        // }

        let (ctx_before, ctx_after) = self.contexts();
        if self.heading() {
            Some(b"".to_vec())
        } else if ctx_before > 0 || ctx_after > 0 {
            self.context_separator()
        } else {
            None
        }
    }

    fn context_separator(&self) -> Option<Vec<u8>> {
        Some(b"--".to_vec())
    }

    fn with_filename(&self, paths: &[PathBuf]) -> bool {
        if self.no_filename {
            false
        } else {
            let path_stdin = Path::new("-");
            self.with_filename
                || self.vimgrep
                || paths.len() > 1
                || paths
                    .get(0)
                    .map_or(false, |p| p != path_stdin && p.is_dir())
        }
    }
}
