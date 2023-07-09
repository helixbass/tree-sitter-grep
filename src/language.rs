use std::collections::HashMap;

use clap::ValueEnum;
use once_cell::sync::Lazy;
use tree_sitter::Language;

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum, Hash)]
pub enum SupportedLanguageName {
    Rust,
    Typescript,
    Javascript,
}

impl SupportedLanguageName {
    pub fn get_language(&self) -> SupportedLanguage {
        *ALL_SUPPORTED_LANGUAGES_BY_SUPPORTED_LANGUAGE_NAME
            .get(self)
            .unwrap()
    }
}

#[derive(Copy, Clone)]
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
