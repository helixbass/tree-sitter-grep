use std::{collections::HashMap, ffi::OsStr, path::Path};

use clap::ValueEnum;
use tree_sitter::Language;

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum, Hash)]
pub enum SupportedLanguageName {
    Rust,
    Typescript,
}

impl SupportedLanguageName {
    pub fn get_language(self) -> Box<dyn SupportedLanguage> {
        match self {
            Self::Rust => Box::new(get_rust_language()),
            Self::Typescript => Box::new(get_typescript_language()),
        }
    }
}

pub trait SupportedLanguage {
    fn language(&self) -> Language;
    fn name(&self) -> SupportedLanguageName;
    fn name_for_ignore_select(&self) -> &'static str;
    fn extensions(&self) -> Vec<&'static str>;
}

pub struct SupportedLanguageRust;

impl SupportedLanguage for SupportedLanguageRust {
    fn language(&self) -> Language {
        tree_sitter_rust::language()
    }

    fn name(&self) -> SupportedLanguageName {
        SupportedLanguageName::Rust
    }

    fn name_for_ignore_select(&self) -> &'static str {
        "rust"
    }

    fn extensions(&self) -> Vec<&'static str> {
        vec!["rs"]
    }
}

pub fn get_rust_language() -> SupportedLanguageRust {
    SupportedLanguageRust
}

pub struct SupportedLanguageTypescript;

impl SupportedLanguage for SupportedLanguageTypescript {
    fn language(&self) -> Language {
        tree_sitter_typescript::language_tsx()
    }

    fn name(&self) -> SupportedLanguageName {
        SupportedLanguageName::Typescript
    }

    fn name_for_ignore_select(&self) -> &'static str {
        "ts"
    }

    fn extensions(&self) -> Vec<&'static str> {
        vec!["ts", "tsx"]
    }
}

pub fn get_typescript_language() -> SupportedLanguageTypescript {
    SupportedLanguageTypescript
}

pub fn get_all_supported_languages() -> HashMap<SupportedLanguageName, Box<dyn SupportedLanguage>> {
    HashMap::from_iter([
        (
            SupportedLanguageName::Rust,
            Box::new(get_rust_language()) as Box<dyn SupportedLanguage>,
        ),
        (
            SupportedLanguageName::Typescript,
            Box::new(get_typescript_language()) as Box<dyn SupportedLanguage>,
        ),
    ])
}

pub fn maybe_supported_language_from_path(path: &Path) -> Option<Box<dyn SupportedLanguage>> {
    let extension = path.extension().and_then(OsStr::to_str)?;
    get_all_supported_languages()
        .into_values()
        .find(|language| language.extensions().contains(&extension))
}
