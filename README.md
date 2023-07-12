# tree-sitter-grep

tree-sitter-grep is a grep-like search tool that recursively searches the
current directory for a [tree-sitter query](https://tree-sitter.github.io/tree-sitter/using-parsers#pattern-matching-with-queries)
pattern.

[![Build status](https://github.com/helixbass/tree-sitter-grep/workflows/main/badge.svg)](https://github.com/helixbass/tree-sitter-grep/actions)
[![Crates.io](https://img.shields.io/crates/v/tree-sitter-grep.svg)](https://crates.io/crates/tree-sitter-grep)

Dual-licensed under MIT or the [UNLICENSE](https://unlicense.org).

* [Installation](#installation)
* [Usage](#usage)
* [Performance](#performance)
* [Editor integrations](#editor-integrations)
* [Contributing/issues](#contributing-issues)



## Installation

With a [Rust toolchain](https://rustup.rs/) installed, run:
```
$ cargo install tree-sitter-grep
```



## Usage

```
$ tree-sitter-grep -q '(trait_bounds) @t'
core.rs:14:pub struct Core<'s, M: 's, S> {
core.rs:30:impl<'s, M: Matcher, S: Sink> Core<'s, M, S> {
mod.rs:622:        P: AsRef<Path>,
mod.rs:623:        M: Matcher,
mod.rs:624:        S: Sink,
mod.rs:644:        M: Matcher,
[...]
```



#### Specifying the query

tree-sitter-grep uses [tree-sitter queries](https://tree-sitter.github.io/tree-sitter/using-parsers#pattern-matching-with-queries)
to specify "patterns" to match

You can either specify the query "inline" with the `-q`/`--query-source` argument:
```
$ tree-sitter-grep -q '(trait_bounds) @t'
```

or via a path to a tree-sitter query file (typically `*.scm`) with the `-Q`/`--query-file` argument:
```
$ cat queries/trait_bounds.scm
(trait_bounds) @t
$ tree-sitter-grep -Q queries/trait_bounds.scm
```

tree-sitter-grep uses tree-sitter query "captures" (`@whatever`) to specify "matching" tree-sitter
AST nodes

So your query must always include at least one capture

If your query includes multiple captures (eg if you are using a "pre-composed" query or are using a
[predicate](#supported-query-predicates)), tree-sitter-grep will by default use the first capture in
the query (in lexicographical order) (I think?) as its "target capture"

To override that behavior, you can pass the `-c`/`--capture` argument:
```
$ tree-sitter-grep -q '((field_declaration name: (field_identifier) @field_name (#eq? @field_name "pos")) @f)' --capture f
```



##### How do I figure out what query I want?

It's worth reading the [tree-sitter query docs](https://tree-sitter.github.io/tree-sitter/using-parsers#pattern-matching-with-queries)
as a starting point

Then for figuring out what the relevant tree-sitter AST structure is for a query you'd like
to write, a tree-sitter "playground" is invaluable, eg the [interactive online one](https://tree-sitter.github.io/tree-sitter/playground)
or I use neovim's `:InspectTree`

In my experience while tree-sitter queries are a solid starting point,
they aren't always "expressive" enough to be able to specify exactly the set
of AST nodes you'd like to match

So that's why we also support specifying [filter plugins](#filter-plugins) where you have
"total programmatic control" over what constitutes a match or not




##### Supported query "predicates"

Tree-sitter query [predicates](https://tree-sitter.github.io/tree-sitter/using-parsers#query-syntax)
allow doing some eg "filtering" of matching tree-sitter AST nodes

We use the [Rust tree-sitter bindings](https://docs.rs/tree-sitter) so "we support
whatever predicates they do"

Specifically that includes:
- `#eq?`
```
$ tree-sitter-grep -q '((field_declaration name: (field_identifier) @field_name (#eq? @field_name "pos")) @f)' --capture f
core.rs:20:    pos: usize,
```
- `#match?`
```
$ tree-sitter-grep -q '((field_declaration name: (field_identifier) @field_name (#match? @field_name "^p")) @f)' --capture f
core.rs:20:    pos: usize,
mod.rs:157:    passthru: bool,
```



##### Filter plugins

When you need "the power of a programming language" in order to fully specify
the matching "criteria", you can write a "filter plugin"




###### Using a filter plugin

If you have an existing filter plugin, you specify that you want to use it via the
`-f`/`--filter` argument (with a path to the compiled filter dynamic library `.so`/`.dll`/`.dylib` file):
```
$ tree-sitter-grep -q '(trait_bounds) @t' -f path/to/libmy-filter.so
```

If the filter plugin expects to be passed a "filter argument" (eg for parameterizing/configuring its
behavior in some way) then you specify that with the `-a`/`--filter-arg` argument:
```
$ tree-sitter-grep -q '(trait_bounds) @t' -f path/to/libmy-filter-that-expects-argument.so -a '{ the_filter_plugin_can_parse_this: "however_it_wants" }'
```

It's also worth noting that technically you don't have to pass a tree-sitter query argument at all
if you supply a filter plugin argument (in which case the filter plugin will get invoked against
"every" tree-sitter AST node)





##### Writing filter plugins

TODO: add a "guide" for this

The short version is:

While in theory you could probably write filter plugins in other languages the "happy path" would
be to write them in Rust and use the example filter plugins from [`examples/`](https://github.com/helixbass/tree-sitter-grep/tree/examples)
as a starting point/reference

The basic idea is that for each tree-sitter AST node that is a potential match according to the supplied
query argument, the filter plugin then additionally gets invoked and indicates whether it considers that
node a match or not (basically as a `(&tree_sitter::Node) -> bool` "predicate")





#### Supported target languages

Currently, tree-sitter-grep "bakes in" support for searching the following languages:

- C
- C++
- C#
- CSS
- Dockerfile
- Elisp
- Elm
- Go
- HTML
- Java
- JavaScript
- JSON
- Kotlin
- Lua
- Objective-C
- Python
- Ruby
- Rust
- Swift
- Toml
- tree-sitter queries (how meta!)
- TypeScript

In theory, any language that has a tree-sitter grammar crate published/available
should be "fair game". In the future we may support dynamically specifying/loading
additional languages

Or feel free to [file an issue](https://github.com/helixbass/tree-sitter-grep/issues)
requesting "baked-in" support for other languages




#### Restricting the query to specific files/languages

By default, tree-sitter-grep will recursively search all
"non-ignored/hidden" files of the [supported languages/types](#supported-target-languages)
and if it can parse the provided query against that language's grammar it will
then search that file's contents for matches

To explicitly specify/restrict to a single language, use the `-l`/`--language` argument:
```
$ tree-sitter-grep -q '(trait_bounds) @t' -l rust
```

You can also restrict the search to certain files/directories by providing path arguments:
```
$ tree-sitter-grep -q '(trait_bounds) @t' src/main.rs src/compiler
```




#### Additional flags/arguments

For documentation of additional arguments related to eg customizing the match output, run:
```
$ tree-sitter-grep --help
```

In general, we are aiming to be rather [`ripgrep`](https://github.com/BurntSushi/ripgrep)-"compatible"




## Performance

I haven't done any "real" benchmarking but the general take seems to be that tree-sitter-grep
is pleasantly, surprisingly fast (especially given that tree-sitter is not optimized for the
"parse-from-scratch" use case)

For "not gigantic" code-bases I'm tending to see it run in < 100ms

And for "gigantic" code-bases where it's eg scanning > 300k lines of code and outputting > 7000 matches,
I'm seeing it run in say 360ms, which still feels "quite fast"




## Editor integrations

TODO, I believe that @peterstuart has written an initial version of an Emacs plugin
and I started tinkering with writing a neovim plugin

The basic idea would probably tend to be that you'd be able to interact with matches
from tree-sitter-grep in your editor the way that you'd interact with matches from
eg `grep`/`ripgrep`

Contributions welcome/let us know if you've written a plugin for your editor of choice




## Contributing/issues

The code-base is a rather typical `cargo`-based Rust project

So eg `cargo test` runs the test suite

Feel free to open [issues](https://github.com/helixbass/tree-sitter-grep/issues) or
[pull requests](https://github.com/helixbass/tree-sitter-grep/pulls)
