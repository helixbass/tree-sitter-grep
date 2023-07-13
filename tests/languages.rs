mod shared;

use shared::{assert_failure_output, assert_sorted_output};

#[test]
fn test_swift() {
    assert_sorted_output(
        "swift_project",
        r#"
            $ tree-sitter-grep -q '(value_argument) @c' --language swift
            example.swift:2:    atPath: "native"
        "#,
    );
}

#[test]
fn test_swift_auto_language() {
    assert_sorted_output(
        "swift_project",
        r#"
            $ tree-sitter-grep -q '(value_argument) @c'
            example.swift:2:    atPath: "native"
        "#,
    );
}

#[test]
fn test_objective_c() {
    assert_sorted_output(
        "objective_c_project",
        r#"
            $ tree-sitter-grep -q '(struct_declaration) @c' --language objective-c
            example.h:4:@property (nonatomic, strong, nullable) NSString *baseURL;
        "#,
    );
}

#[test]
fn test_objective_c_auto_language() {
    assert_sorted_output(
        "objective_c_project",
        r#"
            $ tree-sitter-grep -q '(struct_declaration) @c'
            example.h:4:@property (nonatomic, strong, nullable) NSString *baseURL;
        "#,
    );
}

#[test]
fn test_objective_c_auto_language_ambiguous_query() {
    assert_failure_output(
        "objective_c_project",
        r#"
            $ tree-sitter-grep -q '(identifier) @c'
            File "./example.h" has ambiguous file-type, could be C, C++, or Objective-C. Try passing the --language flag
        "#,
    );
}

#[test]
fn test_toml() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(string) @c' --language toml
            Cargo.toml:2:name = "rust_project"
            Cargo.toml:3:version = "0.1.0"
            Cargo.toml:4:edition = "2021"
        "#,
    );
}

#[test]
fn test_toml_auto_language() {
    assert_sorted_output(
        "rust_project",
        r#"
            $ tree-sitter-grep -q '(string) @c'
            Cargo.toml:2:name = "rust_project"
            Cargo.toml:3:version = "0.1.0"
            Cargo.toml:4:edition = "2021"
        "#,
    );
}

#[test]
fn test_python() {
    assert_sorted_output(
        "python_project",
        r#"
            $ tree-sitter-grep -q '(for_statement) @c' --language python
            example.py:2:    for x in y:
            example.py:3:        something()
        "#,
    );
}

#[test]
fn test_python_auto_language() {
    assert_sorted_output(
        "python_project",
        r#"
            $ tree-sitter-grep -q '(for_statement) @c'
            example.py:2:    for x in y:
            example.py:3:        something()
        "#,
    );
}

#[test]
fn test_ruby() {
    assert_sorted_output(
        "ruby_project",
        r#"
            $ tree-sitter-grep -q '(binary) @c' --language ruby
            example.rb:1:if x > y
        "#,
    );
}

#[test]
fn test_ruby_auto_language() {
    assert_sorted_output(
        "ruby_project",
        r#"
            $ tree-sitter-grep -q '(binary) @c'
            example.rb:1:if x > y
        "#,
    );
}

#[test]
fn test_c() {
    assert_sorted_output(
        "c_project",
        r#"
            $ tree-sitter-grep -q '(pointer_declarator) @c' --language c
            example.h:1:void r_bin_object_free(void /*RBinObject*/ *o_);
        "#,
    );
}

#[test]
fn test_c_auto_language() {
    assert_failure_output(
        "c_project",
        r#"
            $ tree-sitter-grep -q '(pointer_declarator) @c'
            File "./example.h" has ambiguous file-type, could be C, C++, or Objective-C. Try passing the --language flag
        "#,
    );
}

#[test]
fn test_cpp() {
    assert_sorted_output(
        "cpp_project",
        r#"
            $ tree-sitter-grep -q '(namespace_identifier) @c' --language c++
            example.cpp:1:const AvailableAttr *DeclAttributes::getUnavailable(
        "#,
    );
}

#[test]
fn test_cpp_auto_language() {
    assert_sorted_output(
        "cpp_project",
        r#"
            $ tree-sitter-grep -q '(namespace_identifier) @c'
            example.cpp:1:const AvailableAttr *DeclAttributes::getUnavailable(
        "#,
    );
}

#[test]
fn test_go() {
    assert_sorted_output(
        "go_project",
        r#"
            $ tree-sitter-grep -q '(import_spec) @c' --language go
            example.go:2:        "context"
        "#,
    );
}

#[test]
fn test_go_auto_language() {
    assert_sorted_output(
        "go_project",
        r#"
            $ tree-sitter-grep -q '(import_spec) @c'
            example.go:2:        "context"
        "#,
    );
}

#[test]
fn test_java() {
    assert_sorted_output(
        "java_project",
        r#"
            $ tree-sitter-grep -q '(marker_annotation) @c' --language java
            example.java:1:@ThreadSafe
        "#,
    );
}

#[test]
fn test_java_auto_language() {
    assert_sorted_output(
        "java_project",
        r#"
            $ tree-sitter-grep -q '(marker_annotation) @c'
            example.java:1:@ThreadSafe
        "#,
    );
}

#[test]
fn test_c_sharp() {
    assert_sorted_output(
        "csharp_project",
        r#"
            $ tree-sitter-grep -q '(qualified_name) @c' --language c-sharp
            example.cs:1:namespace YL.Utils.Json {}
        "#,
    );
}

#[test]
fn test_c_sharp_auto_language() {
    assert_sorted_output(
        "csharp_project",
        r#"
            $ tree-sitter-grep -q '(qualified_name) @c'
            example.cs:1:namespace YL.Utils.Json {}
        "#,
    );
}

#[test]
fn test_kotlin() {
    assert_sorted_output(
        "kotlin_project",
        r#"
            $ tree-sitter-grep -q '(user_type) @c' --language kotlin
            example.kt:2:    val barA: Int
        "#,
    );
}

#[test]
fn test_kotlin_auto_language() {
    assert_sorted_output(
        "kotlin_project",
        r#"
            $ tree-sitter-grep -q '(user_type) @c'
            example.kt:2:    val barA: Int
        "#,
    );
}

#[test]
fn test_elisp() {
    assert_sorted_output(
        "elisp_project",
        r#"
            $ tree-sitter-grep -q '(quote) @c' --language elisp
            example.el:3:  :group 'lsp-sourcekit
            example.el:4:  :type 'file)
        "#,
    );
}

#[test]
fn test_elisp_auto_language() {
    assert_sorted_output(
        "elisp_project",
        r#"
            $ tree-sitter-grep -q '(quote) @c'
            example.el:3:  :group 'lsp-sourcekit
            example.el:4:  :type 'file)
        "#,
    );
}

#[test]
fn test_elm() {
    assert_sorted_output(
        "elm_project",
        r#"
            $ tree-sitter-grep -q '(upper_case_qid) @c' --language elm
            example.elm:1:import Lofi.Schema exposing (Schema, Item, Kind(..))
        "#,
    );
}

#[test]
fn test_elm_auto_language() {
    assert_sorted_output(
        "elm_project",
        r#"
            $ tree-sitter-grep -q '(upper_case_qid) @c'
            example.elm:1:import Lofi.Schema exposing (Schema, Item, Kind(..))
        "#,
    );
}

#[test]
fn test_dockerfile() {
    assert_sorted_output(
        "dockerfile_project",
        r#"
            $ tree-sitter-grep -q '(path) @c' --language dockerfile
            Dockerfile:1:WORKDIR /usr/src/app
        "#,
    );
}

#[test]
fn test_dockerfile_auto_language() {
    assert_sorted_output(
        "dockerfile_project",
        r#"
            $ tree-sitter-grep -q '(path) @c'
            Dockerfile:1:WORKDIR /usr/src/app
        "#,
    );
}

#[test]
fn test_html() {
    assert_sorted_output(
        "html_project",
        r#"
            $ tree-sitter-grep -q '(text) @c' --language html
            example.html:3:    <p>hello</p>
        "#,
    );
}

#[test]
fn test_html_auto_language() {
    assert_sorted_output(
        "html_project",
        r#"
            $ tree-sitter-grep -q '(text) @c'
            example.html:3:    <p>hello</p>
        "#,
    );
}

#[test]
fn test_tree_sitter_query() {
    assert_sorted_output(
        "tree_sitter_query_project",
        r#"
            $ tree-sitter-grep -q '(capture) @c' --language tree-sitter-query
            example.scm:1:(function_item) @f
        "#,
    );
}

#[test]
fn test_tree_sitter_query_auto_language() {
    assert_sorted_output(
        "tree_sitter_query_project",
        r#"
            $ tree-sitter-grep -q '(capture) @c'
            example.scm:1:(function_item) @f
        "#,
    );
}

#[test]
fn test_json() {
    assert_sorted_output(
        "json_project",
        r#"
            $ tree-sitter-grep -q '(string_content) @c' --language json
            example.json:2:  "hello": "ok"
        "#,
    );
}

#[test]
fn test_json_auto_language() {
    assert_sorted_output(
        "json_project",
        r#"
            $ tree-sitter-grep -q '(string_content) @c'
            example.json:2:  "hello": "ok"
        "#,
    );
}

#[test]
fn test_css() {
    assert_sorted_output(
        "css_project",
        r#"
            $ tree-sitter-grep -q '(tag_name) @c' --language css
            example.css:1:h1 {
        "#,
    );
}

#[test]
fn test_css_auto_language() {
    assert_sorted_output(
        "css_project",
        r#"
            $ tree-sitter-grep -q '(tag_name) @c'
            example.css:1:h1 {
        "#,
    );
}

#[test]
fn test_lua() {
    assert_sorted_output(
        "lua_project",
        r#"
            $ tree-sitter-grep -q '(identifier) @c' --language lua
            example.lua:1:function hello()
        "#,
    );
}

#[test]
fn test_lua_auto_language() {
    assert_sorted_output(
        "lua_project",
        r#"
            $ tree-sitter-grep -q '(identifier) @c'
            example.lua:1:function hello()
        "#,
    );
}
