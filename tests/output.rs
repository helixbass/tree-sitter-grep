mod shared;

use shared::{
    assert_failure_output, assert_non_match_output, assert_sorted_output,
    assert_sorted_output_with_no_matches_exit_status, build_example,
};

#[test]
fn test_query_inline() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query '(function_item) @function_item' --language rust
            src/helpers.rs:pub fn helper() {}
            src/lib.rs:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:    left + right
            src/lib.rs:}
            src/lib.rs:    fn it_works() {
            src/lib.rs:        let result = add(2, 2);
            src/lib.rs:        assert_eq!(result, 4);
            src/lib.rs:    }
            src/stop.rs:fn stop_it() {}
        "#,
    );
}

#[test]
fn test_query_inline_short_option() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @function_item' --language rust
            src/helpers.rs:pub fn helper() {}
            src/lib.rs:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:    left + right
            src/lib.rs:}
            src/lib.rs:    fn it_works() {
            src/lib.rs:        let result = add(2, 2);
            src/lib.rs:        assert_eq!(result, 4);
            src/lib.rs:    }
            src/stop.rs:fn stop_it() {}
        "#,
    );
}

#[test]
fn test_vimgrep_mode() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query '(function_item) @function_item' --language rust --vimgrep
            src/helpers.rs:1:1:pub fn helper() {}
            src/lib.rs:3:1:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:12:5:    fn it_works() {
            src/stop.rs:1:1:fn stop_it() {}
       "#,
    );
}

#[test]
fn test_query_file() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query-file ./function-item.scm --language rust
            src/helpers.rs:pub fn helper() {}
            src/lib.rs:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:    left + right
            src/lib.rs:}
            src/lib.rs:    fn it_works() {
            src/lib.rs:        let result = add(2, 2);
            src/lib.rs:        assert_eq!(result, 4);
            src/lib.rs:    }
            src/stop.rs:fn stop_it() {}
       "#,
    );
}

#[test]
fn test_query_file_short_option() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -Q ./function-item.scm --language rust
            src/helpers.rs:pub fn helper() {}
            src/lib.rs:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:    left + right
            src/lib.rs:}
            src/lib.rs:    fn it_works() {
            src/lib.rs:        let result = add(2, 2);
            src/lib.rs:        assert_eq!(result, 4);
            src/lib.rs:    }
            src/stop.rs:fn stop_it() {}
       "#,
    );
}

#[test]
fn test_specify_single_file() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query '(function_item) @function_item' --language rust src/lib.rs
            pub fn add(left: usize, right: usize) -> usize {
                left + right
            }
                fn it_works() {
                    let result = add(2, 2);
                    assert_eq!(result, 4);
                }
        "#,
    );
}

#[test]
fn test_specify_single_file_preserves_leading_dot_slash() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query '(function_item) @function_item' --language rust --with-filename ./src/lib.rs
            ./src/lib.rs:pub fn add(left: usize, right: usize) -> usize {
            ./src/lib.rs:    left + right
            ./src/lib.rs:}
            ./src/lib.rs:    fn it_works() {
            ./src/lib.rs:        let result = add(2, 2);
            ./src/lib.rs:        assert_eq!(result, 4);
            ./src/lib.rs:    }
        "#,
    );
}

#[test]
fn test_specify_multiple_files() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query '(function_item) @function_item' --language rust src/lib.rs ./src/helpers.rs
            ./src/helpers.rs:pub fn helper() {}
            src/lib.rs:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:    left + right
            src/lib.rs:}
            src/lib.rs:    fn it_works() {
            src/lib.rs:        let result = add(2, 2);
            src/lib.rs:        assert_eq!(result, 4);
            src/lib.rs:    }
        "#,
    );
}

#[test]
fn test_invalid_query_inline() {
    assert_failure_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query '(function_itemz) @function_item' --language rust
            error: couldn't parse query for Rust: Query error at 1:2. Invalid node type function_itemz
        "#,
    );
}

#[test]
fn test_invalid_query_file() {
    assert_failure_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query-file ./function-itemz.scm --language rust
            error: couldn't parse query for Rust: Query error at 1:2. Invalid node type function_itemz
        "#,
    );
}

#[test]
fn test_no_query_or_filter_specified() {
    assert_failure_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --language rust
            error: the following required arguments were not provided:
              <--query-file <PATH_TO_QUERY_FILE>|--query <QUERY_TEXT>|--filter <PATH_TO_FILTER_PLUGIN_DYNAMIC_LIBRARY>>

            Usage: tree-sitter-grep --language <LANGUAGE> <--query-file <PATH_TO_QUERY_FILE>|--query <QUERY_TEXT>|--filter <PATH_TO_FILTER_PLUGIN_DYNAMIC_LIBRARY>> [PATHS]...

            For more information, try '--help'.
        "#,
    );
}

#[test]
fn test_invalid_language_name() {
    assert_failure_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @function_item' --language rustz
            error: invalid value 'rustz' for '--language <LANGUAGE>'
              [possible values: c, c++, c-sharp, css, dockerfile, elisp, elm, go, html, java, javascript, json, kotlin, lua, objective-c, python, ruby, rust, swift, toml, tree-sitter-query, typescript]

              tip: a similar value exists: 'rust'

            For more information, try '--help'.
        "#,
    );
}

#[test]
fn test_invalid_query_file_path() {
    assert_failure_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query-file ./nonexistent.scm --language rust
            error: couldn't read query file "./nonexistent.scm"
        "#,
    );
}

#[test]
fn test_auto_language_single_known_language_encountered() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @function_item'
            src/helpers.rs:pub fn helper() {}
            src/lib.rs:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:    left + right
            src/lib.rs:}
            src/lib.rs:    fn it_works() {
            src/lib.rs:        let result = add(2, 2);
            src/lib.rs:        assert_eq!(result, 4);
            src/lib.rs:    }
            src/stop.rs:fn stop_it() {}
        "#,
    );
}

#[test]
fn test_auto_language_multiple_parseable_languages() {
    assert_sorted_output(
        "mixed_project",
        r#"
            $ tree-sitter-grep -q '(arrow_function) @arrow_function'
            javascript_src/index.js:const js_foo = () => {}
            typescript_src/index.tsx:const foo = () => {}
        "#,
    );
}

#[test]
fn test_auto_language_single_parseable_languages() {
    assert_sorted_output(
        "mixed_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @function_item'
            rust_src/lib.rs:fn foo() {}
        "#,
    );
}

#[test]
fn test_capture_name() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item name: (identifier) @name) @function_item' --language rust --capture function_item
            src/helpers.rs:pub fn helper() {}
            src/lib.rs:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:    left + right
            src/lib.rs:}
            src/lib.rs:    fn it_works() {
            src/lib.rs:        let result = add(2, 2);
            src/lib.rs:        assert_eq!(result, 4);
            src/lib.rs:    }
            src/stop.rs:fn stop_it() {}
        "#,
    );
}

#[test]
fn test_predicate() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item name: (identifier) @name (#eq? @name "add")) @function_item' --language rust --capture function_item
            src/lib.rs:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:    left + right
            src/lib.rs:}
        "#,
    );
}

#[test]
fn test_no_matches() {
    assert_sorted_output_with_no_matches_exit_status(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item name: (identifier) @name (#eq? @name "addz")) @function_item' --language rust
        "#,
    );
}

#[test]
fn test_invalid_capture_name() {
    assert_failure_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @function_item' --language rust --capture function_itemz
            error: invalid capture name 'function_itemz'
        "#,
    );
}

#[test]
fn test_unknown_option() {
    assert_failure_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --queryz '(function_item) @function_item' --language rust
            error: unexpected argument '--queryz' found

              tip: a similar argument exists: '--query'

            Usage: tree-sitter-grep <--query-file <PATH_TO_QUERY_FILE>|--query <QUERY_TEXT>|--filter <PATH_TO_FILTER_PLUGIN_DYNAMIC_LIBRARY>> <PATHS|--query-file <PATH_TO_QUERY_FILE>|--query <QUERY_TEXT>|--capture <CAPTURE_NAME>|--language <LANGUAGE>|--filter <PATH_TO_FILTER_PLUGIN_DYNAMIC_LIBRARY>|--filter-arg <FILTER_ARG>|--vimgrep|--after-context <NUM>|--before-context <NUM>|--context <NUM>|--only-matching|--byte-offset|--colors <COLORS>|--color <WHEN>|--pretty|--heading|--no-heading|--with-filename|--no-filename|--line-number|--no-line-number|--column|--no-column>

            For more information, try '--help'.
        "#,
    );
}

#[test]
fn test_filter_plugin() {
    build_example("filter_before_line_10");

    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @function_item' --language rust --filter ../../../target/debug/examples/libfilter_before_line_10.so
            src/helpers.rs:pub fn helper() {}
            src/lib.rs:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:    left + right
            src/lib.rs:}
            src/stop.rs:fn stop_it() {}
        "#,
    );
}

#[test]
fn test_filter_plugin_with_argument() {
    build_example("filter_before_line_number");

    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @function_item' --language rust --filter ../../../target/debug/examples/libfilter_before_line_number.so --filter-arg 2
            src/helpers.rs:pub fn helper() {}
            src/stop.rs:fn stop_it() {}
        "#,
    );
}

#[test]
fn test_filter_plugin_expecting_argument_not_received() {
    build_example("filter_before_line_number");

    assert_failure_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @function_item' --language rust --filter ../../../target/debug/examples/libfilter_before_line_number.so
            error: plugin expected '--filter-arg <ARGUMENT>'
        "#,
    );
}

#[test]
fn test_filter_plugin_unparseable_argument() {
    build_example("filter_before_line_number");

    assert_failure_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @function_item' --language rust --filter ../../../target/debug/examples/libfilter_before_line_number.so --filter-arg abc
            error: plugin couldn't parse argument "abc"
        "#,
    );
}

#[test]
fn test_filter_plugin_no_query() {
    build_example("filter_function_items_before_line_10");

    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --language rust --filter ../../../target/debug/examples/libfilter_function_items_before_line_10.so
            src/helpers.rs:pub fn helper() {}
            src/lib.rs:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:    left + right
            src/lib.rs:}
            src/stop.rs:fn stop_it() {}
        "#,
    );
}

#[test]
fn test_query_inline_and_query_file_path() {
    assert_failure_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query '(function_item) @function_item' --query-file ./function-item.scm --language rust
            error: the argument '--query <QUERY_TEXT>' cannot be used with '--query-file <PATH_TO_QUERY_FILE>'

            Usage: tree-sitter-grep --language <LANGUAGE> <--query-file <PATH_TO_QUERY_FILE>|--query <QUERY_TEXT>|--filter <PATH_TO_FILTER_PLUGIN_DYNAMIC_LIBRARY>> [PATHS]...

            For more information, try '--help'.
        "#,
    );
}

#[test]
fn test_help_option() {
    assert_non_match_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --help
            Usage: tree-sitter-grep [OPTIONS] <--query-file <PATH_TO_QUERY_FILE>|--query <QUERY_TEXT>|--filter <PATH_TO_FILTER_PLUGIN_DYNAMIC_LIBRARY>> [PATHS]...

            Arguments:
              [PATHS]...


            Options:
              -Q, --query-file <PATH_TO_QUERY_FILE>
                      The path to a tree-sitter query file.

                      This conflicts with the --query option.

              -q, --query <QUERY_TEXT>
                      The source text of a tree-sitter query.

                      This conflicts with the --query-file option.

              -c, --capture <CAPTURE_NAME>
                      The name of the tree-sitter query capture (without leading "@") whose matching nodes will
                      be output.

                      By default this is the "first" capture encountered in the query source text.

              -l, --language <LANGUAGE>
                      The target language for matching.

                      By default all files corresponding to supported languages will be searched if the provided
                      query can be successfully parsed for that language.

                      [possible values: c, c++, c-sharp, css, dockerfile, elisp, elm, go, html, java,
                      javascript, json, kotlin, lua, objective-c, python, ruby, rust, swift, toml,
                      tree-sitter-query, typescript]

              -f, --filter <PATH_TO_FILTER_PLUGIN_DYNAMIC_LIBRARY>
                      The path to a dynamic library that can be used as a "filter plugin".

                      Filter plugins allow for more fine-grained filtering of potentially matching tree-sitter
                      AST nodes.

              -a, --filter-arg <FILTER_ARG>
                      An arbitrary argument to be passed to the specified filter plugin.

                      It is up to the specific filter plugin whether it requires an argument to be passed (eg
                      for self-configuration) and if so what format it expects that argument to be in.

                  --vimgrep
                      Show results with every match on its own line, including line numbers and column numbers.

                      With this option, a line with more that one match will be printed more than once.

              -A, --after-context <NUM>
                      Show NUM lines after each match

              -B, --before-context <NUM>
                      Show NUM lines before each match

              -C, --context <NUM>
                      Show NUM lines before and after each match.

                      This is equivalent to providing both the -B/--before-context and -A/--after-context flags
                      with the same value.

              -o, --only-matching
                      Print only the matched (non-empty) parts of a matching line, with each such part on a
                      separate output line

              -b, --byte-offset
                      Print the 0-based byte offset within the input file before each line of output.

                      If -o (--only-matching) is specified, print the offset of the matching part itself.

                  --colors <COLORS>
                      This flag specifies color settings for use in the output.

                      This flag may be provided multiple times. Settings are applied iteratively. Colors are
                      limited to one of eight choices: red, blue, green, cyan, magenta, yellow, white and black.
                      Styles are limited to nobold, bold, nointense, intense, nounderline or underline.

                      The format of the flag is '{type}:{attribute}:{value}'. '{type}' should be one of path,
                      line, column or match. '{attribute}' can be fg, bg or style. '{value}' is either a color
                      (for fg and bg) or a text style. A special format, '{type}:none', will clear all color
                      settings for '{type}'.

                      For example, the following command will change the match color to magenta and the
                      background color for line numbers to yellow:

                      tree-sitter-grep --colors 'match:fg:magenta' --colors 'line:bg:yellow' -q '(function_item)
                      @f'

                      Extended colors can be used for '{value}' when the terminal supports ANSI color sequences.
                      These are specified as either 'x' (256-color) or 'x,x,x' (24-bit truecolor) where x is a
                      number between 0 and 255 inclusive. x may be given as a normal decimal number or a
                      hexadecimal number, which is prefixed by `0x`.

                      For example, the following command will change the match background color to that
                      represented by the rgb value (0,128,255):

                      tree-sitter-grep --colors 'match:bg:0,128,255'

                      or, equivalently,

                      tree-sitter-grep --colors 'match:bg:0x0,0x80,0xFF'

                      Note that the intense and nointense style flags will have no effect when used alongside
                      these extended color codes.

                  --color <WHEN>
                      This flag controls when to use colors.

                      The default setting is 'auto', which means tree-sitter-grep will try to guess when to use
                      colors. For example, if tree-sitter-grep is printing to a terminal, then it will use
                      colors, but if it is redirected to a file or a pipe, then it will suppress color output.
                      tree-sitter-grep will suppress color output in some other circumstances as well. For
                      example, if the TERM environment variable is not set or set to 'dumb', then
                      tree-sitter-grep will not use colors.

                      When the --vimgrep flag is given to tree-sitter-grep, then the default value for the
                      --color flag changes to 'never'.

                      Possible values:
                      - never:  Colors will never be used
                      - auto:   The default. tree-sitter-grep tries to be smart
                      - always: Colors will always be used regardless of where output is sent
                      - ansi:   Like 'always', but emits ANSI escapes (even in a Windows console)

              -p, --pretty
                      This is a convenience alias for '--color always --heading --line-number'.

                      This flag is useful when you still want pretty output even if you're piping
                      tree-sitter-grep to another program or file. For example: 'tree-sitter-grep -p -q
                      "(function_item) @c" | less -R'.

                  --heading
                      This flag prints the file path above clusters of matches from each file instead of
                      printing the file path as a prefix for each matched line.

                      This is the default mode when printing to a terminal.

                      This overrides the --no-heading flag.

                  --no-heading
                      Don't group matches by each file.

                      If --no-heading is provided in addition to the -H/--with-filename flag, then file paths
                      will be printed as a prefix for every matched line. This is the default mode when not
                      printing to a terminal.

                      This overrides the --heading flag.

              -H, --with-filename
                      Display the file path for matches.

                      This is the default when more than one file is searched. If --heading is enabled (the
                      default when printing to a terminal), the file path will be shown above clusters of
                      matches from each file; otherwise, the file name will be shown as a prefix for each
                      matched line.

                      This flag overrides --no-filename.

              -I, --no-filename
                      Never print the file path with the matched lines.

                      This is the default when tree-sitter-grep is explicitly instructed to search one file or
                      stdin.

                      This flag overrides --with-filename.

              -n, --line-number
                      Show line numbers (1-based).

                      This is enabled by default when searching in a terminal.

              -N, --no-line-number
                      Suppress line numbers.

                      This is enabled by default when not searching in a terminal.

                  --column
                      Show column numbers (1-based).

                      This only shows the column numbers for the first match on each line. This does not try to
                      account for Unicode. One byte is equal to one column. This implies --line-number.

                      This flag can be disabled with --no-column.

              -h, --help
                      Print help (see a summary with '-h')
        "#,
    );
}

#[test]
fn test_help_short_option() {
    assert_non_match_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -h
            Usage: tree-sitter-grep [OPTIONS] <--query-file <PATH_TO_QUERY_FILE>|--query <QUERY_TEXT>|--filter <PATH_TO_FILTER_PLUGIN_DYNAMIC_LIBRARY>> [PATHS]...

            Arguments:
              [PATHS]...

            Options:
              -Q, --query-file <PATH_TO_QUERY_FILE>
                      The path to a tree-sitter query file
              -q, --query <QUERY_TEXT>
                      The source text of a tree-sitter query
              -c, --capture <CAPTURE_NAME>
                      The name of the tree-sitter query capture (without leading "@") whose matching nodes will
                      be output
              -l, --language <LANGUAGE>
                      The target language for matching [possible values: c, c++, c-sharp, css, dockerfile,
                      elisp, elm, go, html, java, javascript, json, kotlin, lua, objective-c, python, ruby,
                      rust, swift, toml, tree-sitter-query, typescript]
              -f, --filter <PATH_TO_FILTER_PLUGIN_DYNAMIC_LIBRARY>
                      The path to a dynamic library that can be used as a "filter plugin"
              -a, --filter-arg <FILTER_ARG>
                      An arbitrary argument to be passed to the specified filter plugin
                  --vimgrep
                      Show results with every match on its own line, including line numbers and column numbers
              -A, --after-context <NUM>
                      Show NUM lines after each match
              -B, --before-context <NUM>
                      Show NUM lines before each match
              -C, --context <NUM>
                      Show NUM lines before and after each match
              -o, --only-matching
                      Print only the matched (non-empty) parts of a matching line, with each such part on a
                      separate output line
              -b, --byte-offset
                      Print the 0-based byte offset within the input file before each line of output
                  --colors <COLORS>
                      This flag specifies color settings for use in the output
                  --color <WHEN>
                      This flag controls when to use colors [possible values: never, auto, always, ansi]
              -p, --pretty
                      This is a convenience alias for '--color always --heading --line-number'
                  --heading
                      This flag prints the file path above clusters of matches from each file instead of
                      printing the file path as a prefix for each matched line
                  --no-heading
                      Don't group matches by each file
              -H, --with-filename
                      Display the file path for matches
              -I, --no-filename
                      Never print the file path with the matched lines
              -n, --line-number
                      Show line numbers (1-based)
              -N, --no-line-number
                      Suppress line numbers
                  --column
                      Show column numbers (1-based)
              -h, --help
                      Print help (see more with '--help')
        "#,
    );
}

#[test]
fn test_no_arguments() {
    assert_failure_output(
        "rust_project",
        r#"
            $ tree-sitter-grep
            error: the following required arguments were not provided:
              <--query-file <PATH_TO_QUERY_FILE>|--query <QUERY_TEXT>|--filter <PATH_TO_FILTER_PLUGIN_DYNAMIC_LIBRARY>>

            Usage: tree-sitter-grep <--query-file <PATH_TO_QUERY_FILE>|--query <QUERY_TEXT>|--filter <PATH_TO_FILTER_PLUGIN_DYNAMIC_LIBRARY>> [PATHS]...

            For more information, try '--help'.
        "#,
    );
}

#[test]
fn test_filter_argument_no_filter() {
    build_example("filter_before_line_number");

    assert_failure_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @function_item' --language rust --filter-arg 2
            error: the following required arguments were not provided:
              --filter <PATH_TO_FILTER_PLUGIN_DYNAMIC_LIBRARY>

            Usage: tree-sitter-grep --language <LANGUAGE> --filter-arg <FILTER_ARG> <--query-file <PATH_TO_QUERY_FILE>|--query <QUERY_TEXT>|--filter <PATH_TO_FILTER_PLUGIN_DYNAMIC_LIBRARY>> [PATHS]...

            For more information, try '--help'.
        "#,
    );
}

#[test]
fn test_macro_contents() {
    assert_sorted_output(
        "match_inside_macro",
        r#"
            $ tree-sitter-grep -q '(call_expression) @c' -l rust
            foo.rs:        self.factory
            foo.rs:            .create_parameter_declaration("whee", Option::<Gc<NodeArray>>::None)
            foo.rs:            .wrap(),
        "#,
    );
}

#[test]
fn test_sorting_maybe_nesting_related() {
    assert_sorted_output(
        "sorting_maybe_nesting_related",
        r#"
            $ tree-sitter-grep -Q ./query.scm -l rust --vimgrep
            foo.rs:44:14:            .create_variable_statement_raw(
            foo.rs:47:22:                    .create_variable_declaration_list_raw(
        "#,
    );
}

#[test]
fn test_overlapping_matches() {
    assert_sorted_output(
        "rust_overlapping",
        r#"
            $ tree-sitter-grep -q '(closure_expression) @closure_expression' --language rust
            src/lib.rs:    let f = || {
            src/lib.rs:        || {
            src/lib.rs:            println!("whee");
            src/lib.rs:        }
            src/lib.rs:    };
        "#,
    );
}

#[test]
fn test_overlapping_matches_vimgrep() {
    assert_sorted_output(
        "rust_overlapping",
        r#"
            $ tree-sitter-grep -q '(closure_expression) @closure_expression' --language rust --vimgrep
            src/lib.rs:2:13:    let f = || {
            src/lib.rs:3:9:        || {
        "#,
    );
}

#[test]
fn test_after_context() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' -l rust --after-context 2
            src/stop.rs:fn stop_it() {}
            --
            src/helpers.rs:pub fn helper() {}
            --
            src/lib.rs:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:    left + right
            src/lib.rs:}
            src/lib.rs-
            src/lib.rs-#[cfg(test)]
            --
            src/lib.rs:    fn it_works() {
            src/lib.rs:        let result = add(2, 2);
            src/lib.rs:        assert_eq!(result, 4);
            src/lib.rs:    }
            src/lib.rs-}
            src/lib.rs-
        "#,
    );
}

#[test]
fn test_after_context_matches_overlap_context_lines() {
    assert_sorted_output(
        "rust_overlapping",
        r#"
            $ tree-sitter-grep -q '(call_expression function: (identifier) @function_name (#match? @function_name "^h"))' -l rust -A 2
            src/lib.rs:    hello();
            src/lib.rs:    hoo();
            src/lib.rs-    raa();
            src/lib.rs-    roo();
        "#,
    );
}

#[test]
fn test_after_context_overlapping_matches() {
    assert_sorted_output(
        "rust_overlapping",
        r#"
            $ tree-sitter-grep -q '(closure_expression) @c' -l rust --after-context 2
            src/lib.rs:    let f = || {
            src/lib.rs:        || {
            src/lib.rs:            println!("whee");
            src/lib.rs:        }
            src/lib.rs:    };
            src/lib.rs-}
            src/lib.rs-
        "#,
    );
}

#[test]
fn test_after_context_overlapping_multiline_matches_vimgrep() {
    assert_sorted_output(
        "rust_overlapping",
        r#"
            $ tree-sitter-grep -q '(closure_expression) @c' -l rust --after-context 2 --vimgrep
            src/lib.rs:2:13:    let f = || {
            src/lib.rs:3:9:        || {
            src/lib.rs-7-}
            src/lib.rs-8-
        "#,
    );
}

#[test]
fn test_after_context_short_option() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' -l rust -A 2
            src/stop.rs:fn stop_it() {}
            --
            src/helpers.rs:pub fn helper() {}
            --
            src/lib.rs:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:    left + right
            src/lib.rs:}
            src/lib.rs-
            src/lib.rs-#[cfg(test)]
            --
            src/lib.rs:    fn it_works() {
            src/lib.rs:        let result = add(2, 2);
            src/lib.rs:        assert_eq!(result, 4);
            src/lib.rs:    }
            src/lib.rs-}
            src/lib.rs-
        "#,
    );
}

#[test]
fn test_before_context() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' -l rust --before-context 3
            src/stop.rs:fn stop_it() {}
            --
            src/helpers.rs:pub fn helper() {}
            --
            src/lib.rs-mod helpers;
            src/lib.rs-
            src/lib.rs:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:    left + right
            src/lib.rs:}
            --
            src/lib.rs-    use super::*;
            src/lib.rs-
            src/lib.rs-    #[test]
            src/lib.rs:    fn it_works() {
            src/lib.rs:        let result = add(2, 2);
            src/lib.rs:        assert_eq!(result, 4);
            src/lib.rs:    }
        "#,
    );
}

#[test]
fn test_before_context_short_option() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' -l rust -B 3
            src/stop.rs:fn stop_it() {}
            --
            src/helpers.rs:pub fn helper() {}
            --
            src/lib.rs-mod helpers;
            src/lib.rs-
            src/lib.rs:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:    left + right
            src/lib.rs:}
            --
            src/lib.rs-    use super::*;
            src/lib.rs-
            src/lib.rs-    #[test]
            src/lib.rs:    fn it_works() {
            src/lib.rs:        let result = add(2, 2);
            src/lib.rs:        assert_eq!(result, 4);
            src/lib.rs:    }
        "#,
    );
}

#[test]
fn test_before_context_matches_overlap_context_lines() {
    assert_sorted_output(
        "rust_overlapping",
        r#"
            $ tree-sitter-grep -q '(call_expression function: (identifier) @function_name (#match? @function_name "^h"))' -l rust -B 2
            src/lib.rs-
            src/lib.rs-fn something_else() {
            src/lib.rs:    hello();
            src/lib.rs:    hoo();
        "#,
    );
}

#[test]
fn test_before_context_overlapping_matches() {
    assert_sorted_output(
        "rust_overlapping_with_preceding_lines",
        r#"
            $ tree-sitter-grep -q '(closure_expression) @c' -l rust --before-context 2
            src/lib.rs-        .i_promise()
            src/lib.rs-        .but_it_has_to_be_longer();
            src/lib.rs:    let f = || {
            src/lib.rs:        || {
            src/lib.rs:            println!("whee");
            src/lib.rs:        }
            src/lib.rs:    };
        "#,
    );
}

#[test]
fn test_before_context_overlapping_multiline_matches_vimgrep() {
    assert_sorted_output(
        "rust_overlapping_with_preceding_lines",
        r#"
            $ tree-sitter-grep -q '(closure_expression) @c' -l rust --before-context 2 --vimgrep
            src/lib.rs-5-        .i_promise()
            src/lib.rs-6-        .but_it_has_to_be_longer();
            src/lib.rs:7:13:    let f = || {
            src/lib.rs:8:9:        || {
        "#,
    );
}

#[test]
fn test_context() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' -l rust --context 2
            src/stop.rs:fn stop_it() {}
            --
            src/helpers.rs:pub fn helper() {}
            --
            src/lib.rs-mod helpers;
            src/lib.rs-
            src/lib.rs:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:    left + right
            src/lib.rs:}
            src/lib.rs-
            src/lib.rs-#[cfg(test)]
            --
            src/lib.rs-
            src/lib.rs-    #[test]
            src/lib.rs:    fn it_works() {
            src/lib.rs:        let result = add(2, 2);
            src/lib.rs:        assert_eq!(result, 4);
            src/lib.rs:    }
            src/lib.rs-}
            src/lib.rs-
        "#,
    );
}

#[test]
fn test_context_adjacent_after_and_before_context_lines() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' -l rust --context 3
            src/stop.rs:fn stop_it() {}
            --
            src/helpers.rs:pub fn helper() {}
            --
            src/lib.rs-mod helpers;
            src/lib.rs-
            src/lib.rs:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:    left + right
            src/lib.rs:}
            src/lib.rs-
            src/lib.rs-#[cfg(test)]
            src/lib.rs-mod tests {
            src/lib.rs-    use super::*;
            src/lib.rs-
            src/lib.rs-    #[test]
            src/lib.rs:    fn it_works() {
            src/lib.rs:        let result = add(2, 2);
            src/lib.rs:        assert_eq!(result, 4);
            src/lib.rs:    }
            src/lib.rs-}
            src/lib.rs-
            src/lib.rs-mod stop;
        "#,
    );
}

#[test]
fn test_context_overlapping_after_and_before_context_lines() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' -l rust --context 4
            src/stop.rs:fn stop_it() {}
            --
            src/helpers.rs:pub fn helper() {}
            --
            src/lib.rs-mod helpers;
            src/lib.rs-
            src/lib.rs:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:    left + right
            src/lib.rs:}
            src/lib.rs-
            src/lib.rs-#[cfg(test)]
            src/lib.rs-mod tests {
            src/lib.rs-    use super::*;
            src/lib.rs-
            src/lib.rs-    #[test]
            src/lib.rs:    fn it_works() {
            src/lib.rs:        let result = add(2, 2);
            src/lib.rs:        assert_eq!(result, 4);
            src/lib.rs:    }
            src/lib.rs-}
            src/lib.rs-
            src/lib.rs-mod stop;
        "#,
    );
}

#[test]
fn test_context_short_option() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' -l rust -C 2
            src/stop.rs:fn stop_it() {}
            --
            src/helpers.rs:pub fn helper() {}
            --
            src/lib.rs-mod helpers;
            src/lib.rs-
            src/lib.rs:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:    left + right
            src/lib.rs:}
            src/lib.rs-
            src/lib.rs-#[cfg(test)]
            --
            src/lib.rs-
            src/lib.rs-    #[test]
            src/lib.rs:    fn it_works() {
            src/lib.rs:        let result = add(2, 2);
            src/lib.rs:        assert_eq!(result, 4);
            src/lib.rs:    }
            src/lib.rs-}
            src/lib.rs-
        "#,
    );
}

#[test]
fn test_before_and_after_context() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' -l rust --before-context 2 --after-context 1
            src/stop.rs:fn stop_it() {}
            --
            src/helpers.rs:pub fn helper() {}
            --
            src/lib.rs-mod helpers;
            src/lib.rs-
            src/lib.rs:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:    left + right
            src/lib.rs:}
            src/lib.rs-
            --
            src/lib.rs-
            src/lib.rs-    #[test]
            src/lib.rs:    fn it_works() {
            src/lib.rs:        let result = add(2, 2);
            src/lib.rs:        assert_eq!(result, 4);
            src/lib.rs:    }
            src/lib.rs-}
        "#,
    );
}

#[test]
fn test_no_files_searched_directory_path_argument_with_no_recognized_file_types() {
    assert_failure_output(
        "no_recognized_file_types",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' subdir/
            No files were searched
        "#,
    );
}

#[test]
fn test_no_files_searched_no_recognized_file_types() {
    assert_failure_output(
        "no_recognized_file_types",
        r#"
            $ tree-sitter-grep -q '(function_item) @f'
            No files were searched
        "#,
    );
}

#[test]
fn test_no_files_searched_recognized_files_but_dont_match_specified_language() {
    assert_failure_output(
        "typescript_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' --language rust
            No files were searched
        "#,
    );
}

#[test]
fn test_couldnt_parse_more_than_two_candidate_auto_detected_languages() {
    assert_failure_output(
        "mixed_project",
        r#"
            $ tree-sitter-grep -q '(function_itemz) @f'
            error: couldn't parse query for Javascript, Rust, or Typescript
        "#,
    );
}

#[test]
fn test_couldnt_parse_two_candidate_auto_detected_languages() {
    assert_failure_output(
        "mixed_project",
        r#"
            $ tree-sitter-grep -q '(function_itemz) @f' javascript_src/ typescript_src/
            error: couldn't parse query for Javascript or Typescript
        "#,
    );
}

#[test]
fn test_nonexistent_file_specified() {
    assert_failure_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' src/nonexistent.rs
            src/nonexistent.rs: No such file or directory (os error 2)
        "#,
    );
}

#[test]
fn test_nonexistent_directory_specified() {
    assert_failure_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' srcz/
            srcz/: No such file or directory (os error 2)
        "#,
    );
}

#[test]
fn test_specify_explicit_file_but_dont_match_specified_language() {
    assert_failure_output(
        "mixed_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' --language rust javascript_src/index.js
            File "javascript_src/index.js" is not recognized as a Rust file
        "#,
    );
}

#[test]
fn test_specify_explicit_file_of_unrecognized_file_type() {
    assert_failure_output(
        "no_recognized_file_types",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' something.scala
            File "something.scala" does not belong to a recognized language
        "#,
    );
}

#[test]
fn test_specify_explicit_file_of_unrecognized_file_type_and_language_flag() {
    assert_failure_output(
        "no_recognized_file_types",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' --language rust something.scala
            File "something.scala" is not recognized as a Rust file
        "#,
    );
}

#[test]
fn test_only_matching() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(parameter) @c' --language rust --only-matching
            src/lib.rs:left: usize
            src/lib.rs:right: usize
        "#,
    );
}

#[test]
fn test_only_matching_short_option() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(parameter) @c' --language rust -o
            src/lib.rs:left: usize
            src/lib.rs:right: usize
        "#,
    );
}

#[test]
fn test_only_matching_multiline_overlapping_matches() {
    assert_sorted_output(
        "rust_overlapping",
        r#"
            $ tree-sitter-grep -q '(closure_expression) @c' -l rust --only-matching
            src/lib.rs:|| {
            src/lib.rs:        || {
            src/lib.rs:            println!("whee");
            src/lib.rs:        }
            src/lib.rs:    }
        "#,
    );
}

#[test]
fn test_only_matching_multiline_overlapping_matches_starting_on_same_line() {
    assert_sorted_output(
        "rust_overlapping_start_same_line",
        r#"
            $ tree-sitter-grep -q '(closure_expression) @c' -l rust --only-matching
            src/lib.rs:|| { || {
            src/lib.rs:            println!("whee");
            src/lib.rs:        }
            src/lib.rs:    }
        "#,
    );
}

#[test]
fn test_no_captures() {
    assert_failure_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item)' --language rust
            error: query must include at least one capture ("@whatever")
        "#,
    );
}

#[test]
fn test_byte_offset() {
    assert_sorted_output(
        "rust_project_byte_offset",
        r#"
            $ tree-sitter-grep -q '(function_item) @function_item' -l rust --byte-offset
            src/helpers.rs:0:pub fn helper() {}
            src/stop.rs:0:fn stop_it() {}
            src/lib.rs:14:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:63:    left + right
            src/lib.rs:80:}
            src/lib.rs:139:    fn it_works() {
            src/lib.rs:159:        let result = add(2, 2);
            src/lib.rs:191:        assert_eq!(result, 4);
            src/lib.rs:222:    }
        "#,
    );
}

#[test]
fn test_byte_offset_short_option() {
    assert_sorted_output(
        "rust_project_byte_offset",
        r#"
            $ tree-sitter-grep -q '(function_item) @function_item' -l rust -b
            src/helpers.rs:0:pub fn helper() {}
            src/stop.rs:0:fn stop_it() {}
            src/lib.rs:14:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:63:    left + right
            src/lib.rs:80:}
            src/lib.rs:139:    fn it_works() {
            src/lib.rs:159:        let result = add(2, 2);
            src/lib.rs:191:        assert_eq!(result, 4);
            src/lib.rs:222:    }
        "#,
    );
}

#[test]
fn test_byte_offset_vimgrep() {
    assert_sorted_output(
        "rust_project_byte_offset",
        r#"
            $ tree-sitter-grep -q '(function_item) @function_item' -l rust --byte-offset --vimgrep
            src/helpers.rs:1:1:0:pub fn helper() {}
            src/stop.rs:1:1:0:fn stop_it() {}
            src/lib.rs:3:1:14:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:12:5:139:    fn it_works() {
        "#,
    );
}

#[test]
fn test_byte_offset_only_matching() {
    assert_sorted_output(
        "rust_project_byte_offset",
        r#"
            $ tree-sitter-grep -q '(parameter) @c' -l rust --byte-offset --only-matching
            src/lib.rs:25:left: usize
            src/lib.rs:38:right: usize
        "#,
    );
}

#[test]
fn test_line_number() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' -l rust --line-number
            src/helpers.rs:1:pub fn helper() {}
            src/stop.rs:1:fn stop_it() {}
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
        "#,
    );
}

#[test]
fn test_line_number_context() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' -l rust --line-number -A 1
            src/stop.rs:1:fn stop_it() {}
            --
            src/helpers.rs:1:pub fn helper() {}
            --
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/lib.rs-6-
            --
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
            src/lib.rs-16-}
        "#,
    );
}

#[test]
fn test_no_filename() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' -l rust --no-filename
            fn stop_it() {}
            pub fn helper() {}
            pub fn add(left: usize, right: usize) -> usize {
                left + right
            }
                fn it_works() {
                    let result = add(2, 2);
                    assert_eq!(result, 4);
                }
        "#,
    );
}

#[test]
fn test_no_filename_short_option() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' -l rust -I
            fn stop_it() {}
            pub fn helper() {}
            pub fn add(left: usize, right: usize) -> usize {
                left + right
            }
                fn it_works() {
                    let result = add(2, 2);
                    assert_eq!(result, 4);
                }
        "#,
    );
}

#[test]
fn test_no_filename_context() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' -l rust --no-filename -A 1
            pub fn helper() {}
            --
            fn stop_it() {}
            --
            pub fn add(left: usize, right: usize) -> usize {
                left + right
            }

            --
                fn it_works() {
                    let result = add(2, 2);
                    assert_eq!(result, 4);
                }
            }
        "#,
    );
}

#[test]
fn test_with_filename_single_file() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @c' -l rust --with-filename src/lib.rs
            src/lib.rs:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:    left + right
            src/lib.rs:}
            src/lib.rs:    fn it_works() {
            src/lib.rs:        let result = add(2, 2);
            src/lib.rs:        assert_eq!(result, 4);
            src/lib.rs:    }
        "#,
    );
}

#[test]
fn test_with_filename_short_option_single_file() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @c' -l rust -H src/lib.rs
            src/lib.rs:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:    left + right
            src/lib.rs:}
            src/lib.rs:    fn it_works() {
            src/lib.rs:        let result = add(2, 2);
            src/lib.rs:        assert_eq!(result, 4);
            src/lib.rs:    }
        "#,
    );
}

#[test]
fn test_with_filename_context_single_file() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @c' -l rust --with-filename -A 1 src/lib.rs
            src/lib.rs:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:    left + right
            src/lib.rs:}
            src/lib.rs-
            --
            src/lib.rs:    fn it_works() {
            src/lib.rs:        let result = add(2, 2);
            src/lib.rs:        assert_eq!(result, 4);
            src/lib.rs:    }
            src/lib.rs-}
        "#,
    );
}

#[test]
fn test_no_option_overrides_preceding_yes_option() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' -l rust --with-filename --no-filename
            fn stop_it() {}
            pub fn helper() {}
            pub fn add(left: usize, right: usize) -> usize {
                left + right
            }
                fn it_works() {
                    let result = add(2, 2);
                    assert_eq!(result, 4);
                }
        "#,
    );
}

#[test]
fn test_yes_option_overrides_preceding_no_option() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' -l rust --no-filename --with-filename
            src/helpers.rs:pub fn helper() {}
            src/lib.rs:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:    left + right
            src/lib.rs:}
            src/lib.rs:    fn it_works() {
            src/lib.rs:        let result = add(2, 2);
            src/lib.rs:        assert_eq!(result, 4);
            src/lib.rs:    }
            src/stop.rs:fn stop_it() {}
        "#,
    );
}

#[test]
fn test_column() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @c' -l rust --column
            src/stop.rs:1:1:fn stop_it() {}
            src/helpers.rs:1:1:pub fn helper() {}
            src/lib.rs:3:1:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:1:    left + right
            src/lib.rs:5:1:}
            src/lib.rs:12:5:    fn it_works() {
            src/lib.rs:13:5:        let result = add(2, 2);
            src/lib.rs:14:5:        assert_eq!(result, 4);
            src/lib.rs:15:5:    }
        "#,
    );
}

#[test]
fn test_column_context() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @c' -l rust --column -A 1
            src/stop.rs:1:1:fn stop_it() {}
            --
            src/helpers.rs:1:1:pub fn helper() {}
            --
            src/lib.rs:3:1:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:1:    left + right
            src/lib.rs:5:1:}
            src/lib.rs-6-
            --
            src/lib.rs:12:5:    fn it_works() {
            src/lib.rs:13:5:        let result = add(2, 2);
            src/lib.rs:14:5:        assert_eq!(result, 4);
            src/lib.rs:15:5:    }
            src/lib.rs-16-}
        "#,
    );
}

#[test]
fn test_no_column() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @c' -l rust --no-column
            src/helpers.rs:pub fn helper() {}
            src/lib.rs:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:    left + right
            src/lib.rs:}
            src/lib.rs:    fn it_works() {
            src/lib.rs:        let result = add(2, 2);
            src/lib.rs:        assert_eq!(result, 4);
            src/lib.rs:    }
            src/stop.rs:fn stop_it() {}
        "#,
    );
}

#[test]
fn test_heading() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @c' -l rust --heading
            src/helpers.rs
            pub fn helper() {}

            src/stop.rs
            fn stop_it() {}

            src/lib.rs
            pub fn add(left: usize, right: usize) -> usize {
                left + right
            }
                fn it_works() {
                    let result = add(2, 2);
                    assert_eq!(result, 4);
                }
        "#,
    );
}

#[test]
fn test_pretty() {
    panic!("TERM: {:?}", std::env::var("TERM"));
    // assert_sorted_output(
    //     "rust_project",
    //     r#"
    //          $ tree-sitter-grep -q '(function_item) @c' -l rust --pretty
    //          [0m[1m[32msrc/stop.rs[0m
    //          [0m[1m[33m1[0m:[0m[30m[43mfn stop_it() {}[0m

    //          [0m[1m[32msrc/helpers.rs[0m
    //          [0m[1m[33m1[0m:[0m[30m[43mpub fn helper() {}[0m

    //          [0m[1m[32msrc/lib.rs[0m
    //          [0m[1m[33m3[0m:[0m[30m[43mpub fn add(left: usize, right: usize)
    // -> usize {[0m          [0m[1m[33m4[0m:[0m[30m[43m    left + right[0m
    //          [0m[1m[33m5[0m:[0m[30m[43m}[0m
    //          [0m[1m[33m12[0m:    [0m[30m[43mfn it_works() {[0m
    //          [0m[1m[33m13[0m:[0m[30m[43m        let result = add(2, 2);[0m
    //          [0m[1m[33m14[0m:[0m[30m[43m        assert_eq!(result, 4);[0m
    //          [0m[1m[33m15[0m:[0m[30m[43m    }[0m
    //     "#,
    // );
}

#[test]
fn test_color_always() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @c' -l rust --color always
            [0m[1m[32msrc/stop.rs[0m:[0m[30m[43mfn stop_it() {}[0m
            [0m[1m[32msrc/helpers.rs[0m:[0m[30m[43mpub fn helper() {}[0m
            [0m[1m[32msrc/lib.rs[0m:[0m[30m[43mpub fn add(left: usize, right: usize) -> usize {[0m
            [0m[1m[32msrc/lib.rs[0m:[0m[30m[43m    left + right[0m
            [0m[1m[32msrc/lib.rs[0m:[0m[30m[43m}[0m
            [0m[1m[32msrc/lib.rs[0m:    [0m[30m[43mfn it_works() {[0m
            [0m[1m[32msrc/lib.rs[0m:[0m[30m[43m        let result = add(2, 2);[0m
            [0m[1m[32msrc/lib.rs[0m:[0m[30m[43m        assert_eq!(result, 4);[0m
            [0m[1m[32msrc/lib.rs[0m:[0m[30m[43m    }[0m
        "#,
    );
}

#[test]
fn test_colors() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' --pretty --colors 'match:fg:magenta' --colors 'line:bg:cyan' --colors 'path:fg:blue'
            [0m[1m[34msrc/stop.rs[0m
            [0m[1m[33m[46m1[0m:[0m[35m[43mfn stop_it() {}[0m

            [0m[1m[34msrc/helpers.rs[0m
            [0m[1m[33m[46m1[0m:[0m[35m[43mpub fn helper() {}[0m

            [0m[1m[34msrc/lib.rs[0m
            [0m[1m[33m[46m3[0m:[0m[35m[43mpub fn add(left: usize, right: usize) -> usize {[0m
            [0m[1m[33m[46m4[0m:[0m[35m[43m    left + right[0m
            [0m[1m[33m[46m5[0m:[0m[35m[43m}[0m
            [0m[1m[33m[46m12[0m:    [0m[35m[43mfn it_works() {[0m
            [0m[1m[33m[46m13[0m:[0m[35m[43m        let result = add(2, 2);[0m
            [0m[1m[33m[46m14[0m:[0m[35m[43m        assert_eq!(result, 4);[0m
            [0m[1m[33m[46m15[0m:[0m[35m[43m    }[0m
        "#,
    );
}
