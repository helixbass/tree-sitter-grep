use std::{
    collections::HashMap,
    ops::{Deref, Index},
};

use once_cell::sync::Lazy;
use proc_macros::fixed_map;
use tree_sitter::Language;

fixed_map! {
    name => SupportedLanguage,
    variants => [
        Rust,
        Typescript,
        Javascript,
        Swift,
        ObjectiveC,
        Toml,
        Python,
        Ruby,
        C,
        Cpp,
        Go,
        Java,
        CSharp,
        Kotlin,
        Elisp,
        Elm,
        Dockerfile,
        Html,
        TreeSitterQuery,
        Json,
        Css,
        Lua,
    ],
}

impl SupportedLanguage {
    pub fn language(&self) -> Language {
        SUPPORTED_LANGUAGE_LANGUAGES[*self]
    }

    pub fn name_for_ignore_select(&self) -> &'static str {
        SUPPORTED_LANGUAGE_NAMES_FOR_IGNORE_SELECT[*self]
    }
}

static SUPPORTED_LANGUAGE_LANGUAGES: Lazy<BySupportedLanguage<Language>> = Lazy::new(|| {
    BySupportedLanguage([
        tree_sitter_rust::language(),
        tree_sitter_typescript::language_tsx(),
        tree_sitter_javascript::language(),
        tree_sitter_swift::language(),
        tree_sitter_objc::language(),
        tree_sitter_toml::language(),
        tree_sitter_python::language(),
        tree_sitter_ruby::language(),
        tree_sitter_c::language(),
        tree_sitter_cpp::language(),
        tree_sitter_go::language(),
        tree_sitter_java::language(),
        tree_sitter_c_sharp::language(),
        tree_sitter_kotlin::language(),
        tree_sitter_elisp::language(),
        tree_sitter_elm::language(),
        tree_sitter_dockerfile::language(),
        tree_sitter_html::language(),
        tree_sitter_query::language(),
        tree_sitter_json::language(),
        tree_sitter_css::language(),
        tree_sitter_lua::language(),
    ])
});

pub static SUPPORTED_LANGUAGE_NAMES_FOR_IGNORE_SELECT: BySupportedLanguage<&'static str> =
    BySupportedLanguage([
        "rust",
        "ts",
        "js",
        "swift",
        "objc",
        "toml",
        "py",
        "ruby",
        "c",
        "cpp",
        "go",
        "java",
        "csharp",
        "kotlin",
        "elisp",
        "elm",
        "docker",
        "html",
        "treesitterquery",
        "json",
        "css",
        "lua",
    ]);

pub static ALL_SUPPORTED_LANGUAGES_BY_NAME_FOR_IGNORE_SELECT: Lazy<
    HashMap<&'static str, SupportedLanguage>,
> = Lazy::new(|| {
    ALL_SUPPORTED_LANGUAGES
        .values()
        .map(|supported_language| {
            (
                supported_language.name_for_ignore_select(),
                *supported_language,
            )
        })
        .collect()
});
