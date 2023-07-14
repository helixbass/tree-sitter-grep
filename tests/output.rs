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
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
            src/stop.rs:1:fn stop_it() {}
        "#,
    );
}

#[test]
fn test_query_inline_short_option() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @function_item' --language rust
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
            src/stop.rs:1:fn stop_it() {}
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
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
            src/stop.rs:1:fn stop_it() {}
       "#,
    );
}

#[test]
fn test_query_file_short_option() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -Q ./function-item.scm --language rust
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
            src/stop.rs:1:fn stop_it() {}
       "#,
    );
}

#[test]
fn test_specify_single_file() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query '(function_item) @function_item' --language rust src/lib.rs
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
fn test_specify_single_file_preserves_leading_dot_slash() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query '(function_item) @function_item' --language rust ./src/lib.rs
            ./src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            ./src/lib.rs:4:    left + right
            ./src/lib.rs:5:}
            ./src/lib.rs:12:    fn it_works() {
            ./src/lib.rs:13:        let result = add(2, 2);
            ./src/lib.rs:14:        assert_eq!(result, 4);
            ./src/lib.rs:15:    }
        "#,
    );
}

#[test]
fn test_specify_multiple_files() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep --query '(function_item) @function_item' --language rust src/lib.rs ./src/helpers.rs
            ./src/helpers.rs:1:pub fn helper() {}
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
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
            src/stop.rs:1:fn stop_it() {}
        "#,
    );
}

#[test]
fn test_auto_language_multiple_parseable_languages() {
    assert_sorted_output(
        "mixed_project",
        r#"
            $ tree-sitter-grep -q '(arrow_function) @arrow_function'
            javascript_src/index.js:1:const js_foo = () => {}
            typescript_src/index.tsx:1:const foo = () => {}
        "#,
    );
}

#[test]
fn test_auto_language_single_parseable_languages() {
    assert_sorted_output(
        "mixed_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @function_item'
            rust_src/lib.rs:1:fn foo() {}
        "#,
    );
}

#[test]
fn test_capture_name() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item name: (identifier) @name) @function_item' --language rust --capture function_item
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
            src/stop.rs:1:fn stop_it() {}
        "#,
    );
}

#[test]
fn test_predicate() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item name: (identifier) @name (#eq? @name "add")) @function_item' --language rust --capture function_item
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
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

            Usage: tree-sitter-grep <--query-file <PATH_TO_QUERY_FILE>|--query <QUERY_TEXT>|--filter <PATH_TO_FILTER_PLUGIN_DYNAMIC_LIBRARY>> <PATHS|--query-file <PATH_TO_QUERY_FILE>|--query <QUERY_TEXT>|--capture <CAPTURE_NAME>|--language <LANGUAGE>|--filter <PATH_TO_FILTER_PLUGIN_DYNAMIC_LIBRARY>|--filter-arg <FILTER_ARG>|--vimgrep|--after-context <NUM>|--before-context <NUM>|--context <NUM>|--only-matching|--byte-offset>

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
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/stop.rs:1:fn stop_it() {}
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
            src/helpers.rs:1:pub fn helper() {}
            src/stop.rs:1:fn stop_it() {}
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
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/stop.rs:1:fn stop_it() {}
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
            foo.rs:4:        self.factory
            foo.rs:5:            .create_parameter_declaration("whee", Option::<Gc<NodeArray>>::None)
            foo.rs:6:            .wrap(),
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
            src/lib.rs:2:    let f = || {
            src/lib.rs:3:        || {
            src/lib.rs:4:            println!("whee");
            src/lib.rs:5:        }
            src/lib.rs:6:    };
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
            src/stop.rs:1:fn stop_it() {}
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/lib.rs-6-
            src/lib.rs-7-#[cfg(test)]
            --
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
            src/lib.rs-16-}
            src/lib.rs-17-
        "#,
    );
}

#[test]
fn test_after_context_matches_overlap_context_lines() {
    assert_sorted_output(
        "rust_overlapping",
        r#"
            $ tree-sitter-grep -q '(call_expression function: (identifier) @function_name (#match? @function_name "^h"))' -l rust -A 2
            src/lib.rs:10:    hello();
            src/lib.rs:11:    hoo();
            src/lib.rs-12-    raa();
            src/lib.rs-13-    roo();
        "#,
    );
}

#[test]
fn test_after_context_overlapping_matches() {
    assert_sorted_output(
        "rust_overlapping",
        r#"
            $ tree-sitter-grep -q '(closure_expression) @c' -l rust --after-context 2
            src/lib.rs:2:    let f = || {
            src/lib.rs:3:        || {
            src/lib.rs:4:            println!("whee");
            src/lib.rs:5:        }
            src/lib.rs:6:    };
            src/lib.rs-7-}
            src/lib.rs-8-
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
            src/stop.rs:1:fn stop_it() {}
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/lib.rs-6-
            src/lib.rs-7-#[cfg(test)]
            --
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
            src/lib.rs-16-}
            src/lib.rs-17-
        "#,
    );
}

#[test]
fn test_before_context() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' -l rust --before-context 3
            src/stop.rs:1:fn stop_it() {}
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs-1-mod helpers;
            src/lib.rs-2-
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            --
            src/lib.rs-9-    use super::*;
            src/lib.rs-10-
            src/lib.rs-11-    #[test]
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
        "#,
    );
}

#[test]
fn test_before_context_short_option() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' -l rust -B 3
            src/stop.rs:1:fn stop_it() {}
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs-1-mod helpers;
            src/lib.rs-2-
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            --
            src/lib.rs-9-    use super::*;
            src/lib.rs-10-
            src/lib.rs-11-    #[test]
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
        "#,
    );
}

#[test]
fn test_before_context_matches_overlap_context_lines() {
    assert_sorted_output(
        "rust_overlapping",
        r#"
            $ tree-sitter-grep -q '(call_expression function: (identifier) @function_name (#match? @function_name "^h"))' -l rust -B 2
            src/lib.rs-8-
            src/lib.rs-9-fn something_else() {
            src/lib.rs:10:    hello();
            src/lib.rs:11:    hoo();
        "#,
    );
}

#[test]
fn test_before_context_overlapping_matches() {
    assert_sorted_output(
        "rust_overlapping_with_preceding_lines",
        r#"
            $ tree-sitter-grep -q '(closure_expression) @c' -l rust --before-context 2
            src/lib.rs-5-        .i_promise()
            src/lib.rs-6-        .but_it_has_to_be_longer();
            src/lib.rs:7:    let f = || {
            src/lib.rs:8:        || {
            src/lib.rs:9:            println!("whee");
            src/lib.rs:10:        }
            src/lib.rs:11:    };
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
            src/stop.rs:1:fn stop_it() {}
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs-1-mod helpers;
            src/lib.rs-2-
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/lib.rs-6-
            src/lib.rs-7-#[cfg(test)]
            --
            src/lib.rs-10-
            src/lib.rs-11-    #[test]
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
            src/lib.rs-16-}
            src/lib.rs-17-
        "#,
    );
}

#[test]
fn test_context_adjacent_after_and_before_context_lines() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' -l rust --context 3
            src/stop.rs:1:fn stop_it() {}
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs-1-mod helpers;
            src/lib.rs-2-
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/lib.rs-6-
            src/lib.rs-7-#[cfg(test)]
            src/lib.rs-8-mod tests {
            src/lib.rs-9-    use super::*;
            src/lib.rs-10-
            src/lib.rs-11-    #[test]
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
            src/lib.rs-16-}
            src/lib.rs-17-
            src/lib.rs-18-mod stop;
        "#,
    );
}

#[test]
fn test_context_overlapping_after_and_before_context_lines() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' -l rust --context 4
            src/stop.rs:1:fn stop_it() {}
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs-1-mod helpers;
            src/lib.rs-2-
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/lib.rs-6-
            src/lib.rs-7-#[cfg(test)]
            src/lib.rs-8-mod tests {
            src/lib.rs-9-    use super::*;
            src/lib.rs-10-
            src/lib.rs-11-    #[test]
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
            src/lib.rs-16-}
            src/lib.rs-17-
            src/lib.rs-18-mod stop;
        "#,
    );
}

#[test]
fn test_context_short_option() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' -l rust -C 2
            src/stop.rs:1:fn stop_it() {}
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs-1-mod helpers;
            src/lib.rs-2-
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/lib.rs-6-
            src/lib.rs-7-#[cfg(test)]
            --
            src/lib.rs-10-
            src/lib.rs-11-    #[test]
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
            src/lib.rs-16-}
            src/lib.rs-17-
        "#,
    );
}

#[test]
fn test_before_and_after_context() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(function_item) @f' -l rust --before-context 2 --after-context 1
            src/stop.rs:1:fn stop_it() {}
            src/helpers.rs:1:pub fn helper() {}
            src/lib.rs-1-mod helpers;
            src/lib.rs-2-
            src/lib.rs:3:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:    left + right
            src/lib.rs:5:}
            src/lib.rs-6-
            --
            src/lib.rs-10-
            src/lib.rs-11-    #[test]
            src/lib.rs:12:    fn it_works() {
            src/lib.rs:13:        let result = add(2, 2);
            src/lib.rs:14:        assert_eq!(result, 4);
            src/lib.rs:15:    }
            src/lib.rs-16-}
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
            src/lib.rs:3:left: usize
            src/lib.rs:3:right: usize
        "#,
    );
}

#[test]
fn test_only_matching_short_option() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(parameter) @c' --language rust -o
            src/lib.rs:3:left: usize
            src/lib.rs:3:right: usize
        "#,
    );
}

#[test]
fn test_only_matching_multiline_overlapping_matches() {
    assert_sorted_output(
        "rust_overlapping",
        r#"
            $ tree-sitter-grep -q '(closure_expression) @c' -l rust --only-matching
            src/lib.rs:2:|| {
            src/lib.rs:3:        || {
            src/lib.rs:4:            println!("whee");
            src/lib.rs:5:        }
            src/lib.rs:6:    }
        "#,
    );
}

#[test]
fn test_only_matching_multiline_overlapping_matches_starting_on_same_line() {
    assert_sorted_output(
        "rust_overlapping_start_same_line",
        r#"
            $ tree-sitter-grep -q '(closure_expression) @c' -l rust --only-matching
            src/lib.rs:3:|| { || {
            src/lib.rs:4:            println!("whee");
            src/lib.rs:5:        }
            src/lib.rs:6:    }
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
            src/helpers.rs:1:0:pub fn helper() {}
            src/stop.rs:1:0:fn stop_it() {}
            src/lib.rs:3:14:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:63:    left + right
            src/lib.rs:5:80:}
            src/lib.rs:12:139:    fn it_works() {
            src/lib.rs:13:159:        let result = add(2, 2);
            src/lib.rs:14:191:        assert_eq!(result, 4);
            src/lib.rs:15:222:    }
        "#,
    );
}

#[test]
fn test_byte_offset_short_option() {
    assert_sorted_output(
        "rust_project_byte_offset",
        r#"
            $ tree-sitter-grep -q '(function_item) @function_item' -l rust -b
            src/helpers.rs:1:0:pub fn helper() {}
            src/stop.rs:1:0:fn stop_it() {}
            src/lib.rs:3:14:pub fn add(left: usize, right: usize) -> usize {
            src/lib.rs:4:63:    left + right
            src/lib.rs:5:80:}
            src/lib.rs:12:139:    fn it_works() {
            src/lib.rs:13:159:        let result = add(2, 2);
            src/lib.rs:14:191:        assert_eq!(result, 4);
            src/lib.rs:15:222:    }
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
            src/lib.rs:3:25:left: usize
            src/lib.rs:3:38:right: usize
        "#,
    );
}
