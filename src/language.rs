use std::{collections::HashMap, ops::Index};

use clap::ValueEnum;
use once_cell::sync::Lazy;
use tree_sitter::Language;

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum SupportedLanguage {
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
}

pub type BySupportedLanguage<T> = [T; 22];

impl<T> Index<SupportedLanguage> for BySupportedLanguage<T> {
    type Output = T;

    fn index(&self, index: SupportedLanguage) -> &Self::Output {
        &self[index as usize]
    }
}

impl From<usize> for SupportedLanguage {
    fn from(value: usize) -> Self {
        match value {
            value if value == Self::Rust as usize => Self::Rust,
            value if value == Self::Typescript as usize => Self::Typescript,
            value if value == Self::Javascript as usize => Self::Javascript,
            value if value == Self::Swift as usize => Self::Swift,
            value if value == Self::ObjectiveC as usize => Self::ObjectiveC,
            value if value == Self::Toml as usize => Self::Toml,
            value if value == Self::Python as usize => Self::Python,
            value if value == Self::Ruby as usize => Self::Ruby,
            value if value == Self::C as usize => Self::C,
            value if value == Self::Cpp as usize => Self::Cpp,
            value if value == Self::Go as usize => Self::Go,
            value if value == Self::Java as usize => Self::Java,
            value if value == Self::CSharp as usize => Self::CSharp,
            value if value == Self::Kotlin as usize => Self::Kotlin,
            value if value == Self::Elisp as usize => Self::Elisp,
            value if value == Self::Elm as usize => Self::Elm,
            value if value == Self::Dockerfile as usize => Self::Dockerfile,
            value if value == Self::Html as usize => Self::Html,
            value if value == Self::TreeSitterQuery as usize => Self::TreeSitterQuery,
            value if value == Self::Json as usize => Self::Json,
            value if value == Self::Css as usize => Self::Css,
            value if value == Self::Lua as usize => Self::Lua,
            _ => unreachable!(),
        }
    }
}

pub static ALL_SUPPORTED_LANGUAGES: BySupportedLanguage<SupportedLanguage> = {
    use SupportedLanguage::*;
    [
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
    ]
};

static SUPPORTED_LANGUAGE_LANGUAGES: Lazy<BySupportedLanguage<Language>> = Lazy::new(|| {
    [
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
    ]
});

#[inline(always)]
pub fn get_supported_language_language(supported_language: SupportedLanguage) -> Language {
    SUPPORTED_LANGUAGE_LANGUAGES[supported_language]
}

pub static SUPPORTED_LANGUAGE_NAMES_FOR_IGNORE_SELECT: BySupportedLanguage<&'static str> = [
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
];

#[inline(always)]
pub fn get_supported_language_name_for_ignore_select(
    supported_language: SupportedLanguage,
) -> &'static str {
    SUPPORTED_LANGUAGE_NAMES_FOR_IGNORE_SELECT[supported_language]
}

pub static ALL_SUPPORTED_LANGUAGES_BY_NAME_FOR_IGNORE_SELECT: Lazy<
    HashMap<&'static str, SupportedLanguage>,
> = Lazy::new(|| {
    ALL_SUPPORTED_LANGUAGES
        .iter()
        .map(|supported_language| {
            (
                get_supported_language_name_for_ignore_select(*supported_language),
                *supported_language,
            )
        })
        .collect()
});
