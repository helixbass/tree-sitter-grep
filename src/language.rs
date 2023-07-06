use std::collections::HashMap;

use clap::ValueEnum;
use once_cell::sync::Lazy;
use tree_sitter::Language;

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum, Hash)]
pub enum SupportedLanguageName {
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

impl SupportedLanguageName {
    pub fn get_language(&self) -> SupportedLanguage {
        *ALL_SUPPORTED_LANGUAGES_BY_SUPPORTED_LANGUAGE_NAME
            .get(self)
            .unwrap()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct SupportedLanguage {
    pub language: Language,
    pub name: SupportedLanguageName,
    pub name_for_ignore_select: &'static str,
}

impl PartialEq for SupportedLanguage {
    fn eq(&self, other: &Self) -> bool {
        self.language == other.language
    }
}

pub static ALL_SUPPORTED_LANGUAGES: Lazy<Vec<SupportedLanguage>> = Lazy::new(|| {
    vec![
        SupportedLanguage {
            language: tree_sitter_rust::language(),
            name: SupportedLanguageName::Rust,
            name_for_ignore_select: "rust",
        },
        SupportedLanguage {
            language: tree_sitter_typescript::language_tsx(),
            name: SupportedLanguageName::Typescript,
            name_for_ignore_select: "ts",
        },
        SupportedLanguage {
            language: tree_sitter_javascript::language(),
            name: SupportedLanguageName::Javascript,
            name_for_ignore_select: "js",
        },
        SupportedLanguage {
            language: tree_sitter_swift::language(),
            name: SupportedLanguageName::Swift,
            name_for_ignore_select: "swift",
        },
        SupportedLanguage {
            language: tree_sitter_objc::language(),
            name: SupportedLanguageName::ObjectiveC,
            name_for_ignore_select: "objc",
        },
        SupportedLanguage {
            language: tree_sitter_toml::language(),
            name: SupportedLanguageName::Toml,
            name_for_ignore_select: "toml",
        },
        SupportedLanguage {
            language: tree_sitter_python::language(),
            name: SupportedLanguageName::Python,
            name_for_ignore_select: "py",
        },
        SupportedLanguage {
            language: tree_sitter_ruby::language(),
            name: SupportedLanguageName::Ruby,
            name_for_ignore_select: "ruby",
        },
        SupportedLanguage {
            language: tree_sitter_c::language(),
            name: SupportedLanguageName::C,
            name_for_ignore_select: "c",
        },
        SupportedLanguage {
            language: tree_sitter_cpp::language(),
            name: SupportedLanguageName::Cpp,
            name_for_ignore_select: "cpp",
        },
        SupportedLanguage {
            language: tree_sitter_go::language(),
            name: SupportedLanguageName::Go,
            name_for_ignore_select: "go",
        },
        SupportedLanguage {
            language: tree_sitter_java::language(),
            name: SupportedLanguageName::Java,
            name_for_ignore_select: "java",
        },
        SupportedLanguage {
            language: tree_sitter_c_sharp::language(),
            name: SupportedLanguageName::CSharp,
            name_for_ignore_select: "csharp",
        },
        SupportedLanguage {
            language: tree_sitter_kotlin::language(),
            name: SupportedLanguageName::Kotlin,
            name_for_ignore_select: "kotlin",
        },
        SupportedLanguage {
            language: tree_sitter_elisp::language(),
            name: SupportedLanguageName::Elisp,
            name_for_ignore_select: "elisp",
        },
        SupportedLanguage {
            language: tree_sitter_elm::language(),
            name: SupportedLanguageName::Elm,
            name_for_ignore_select: "elm",
        },
        SupportedLanguage {
            language: tree_sitter_dockerfile::language(),
            name: SupportedLanguageName::Dockerfile,
            name_for_ignore_select: "docker",
        },
        SupportedLanguage {
            language: tree_sitter_html::language(),
            name: SupportedLanguageName::Html,
            name_for_ignore_select: "html",
        },
        SupportedLanguage {
            language: tree_sitter_query::language(),
            name: SupportedLanguageName::TreeSitterQuery,
            name_for_ignore_select: "treesitterquery",
        },
        SupportedLanguage {
            language: tree_sitter_json::language(),
            name: SupportedLanguageName::Json,
            name_for_ignore_select: "json",
        },
        SupportedLanguage {
            language: tree_sitter_css::language(),
            name: SupportedLanguageName::Css,
            name_for_ignore_select: "css",
        },
        SupportedLanguage {
            language: tree_sitter_lua::language(),
            name: SupportedLanguageName::Lua,
            name_for_ignore_select: "lua",
        },
    ]
});

pub static ALL_SUPPORTED_LANGUAGES_BY_NAME_FOR_IGNORE_SELECT: Lazy<
    HashMap<&'static str, SupportedLanguage>,
> = Lazy::new(|| {
    ALL_SUPPORTED_LANGUAGES
        .iter()
        .map(|supported_language| {
            (
                supported_language.name_for_ignore_select,
                *supported_language,
            )
        })
        .collect()
});

pub static ALL_SUPPORTED_LANGUAGES_BY_SUPPORTED_LANGUAGE_NAME: Lazy<
    HashMap<SupportedLanguageName, SupportedLanguage>,
> = Lazy::new(|| {
    ALL_SUPPORTED_LANGUAGES
        .iter()
        .map(|supported_language| (supported_language.name, *supported_language))
        .collect()
});
