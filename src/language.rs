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
        C,
        Cpp,
        CSharp,
        Css,
        Dockerfile,
        Elisp,
        Elm,
        Go,
        Html,
        Java,
        Javascript,
        Json,
        Kotlin,
        Lua,
        ObjectiveC,
        Python,
        Ruby,
        Rust,
        Swift,
        Toml,
        TreeSitterQuery,
        Typescript,
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
    by_supported_language!(
        Rust => tree_sitter_rust::language(),
        Typescript => tree_sitter_typescript::language_tsx(),
        Javascript => tree_sitter_javascript::language(),
        Swift => tree_sitter_swift::language(),
        ObjectiveC => tree_sitter_objc::language(),
        Toml => tree_sitter_toml::language(),
        Python => tree_sitter_python::language(),
        Ruby => tree_sitter_ruby::language(),
        C => tree_sitter_c::language(),
        Cpp => tree_sitter_cpp::language(),
        Go => tree_sitter_go::language(),
        Java => tree_sitter_java::language(),
        CSharp => tree_sitter_c_sharp::language(),
        Kotlin => tree_sitter_kotlin::language(),
        Elisp => tree_sitter_elisp::language(),
        Elm => tree_sitter_elm::language(),
        Dockerfile => tree_sitter_dockerfile::language(),
        Html => tree_sitter_html::language(),
        TreeSitterQuery => tree_sitter_query::language(),
        Json => tree_sitter_json::language(),
        Css => tree_sitter_css::language(),
        Lua => tree_sitter_lua::language(),
    )
});

static SUPPORTED_LANGUAGE_NAMES_FOR_IGNORE_SELECT: BySupportedLanguage<&'static str> = by_supported_language!(
    Rust => "rust",
    Typescript => "ts",
    Javascript => "js",
    Swift => "swift",
    ObjectiveC => "objc",
    Toml => "toml",
    Python => "py",
    Ruby => "ruby",
    C => "c",
    Cpp => "cpp",
    Go => "go",
    Java => "java",
    CSharp => "csharp",
    Kotlin => "kotlin",
    Elisp => "elisp",
    Elm => "elm",
    Dockerfile => "docker",
    Html => "html",
    TreeSitterQuery => "treesitterquery",
    Json => "json",
    Css => "css",
    Lua => "lua",
);

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
