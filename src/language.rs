use clap::ValueEnum;
use tree_sitter::Language;

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
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
    fn name_for_ignore_select(&self) -> &'static str;
}

pub struct SupportedLanguageRust;

impl SupportedLanguage for SupportedLanguageRust {
    fn language(&self) -> Language {
        tree_sitter_rust::language()
    }

    fn name_for_ignore_select(&self) -> &'static str {
        "rust"
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

    fn name_for_ignore_select(&self) -> &'static str {
        "ts"
    }
}

pub fn get_typescript_language() -> SupportedLanguageTypescript {
    SupportedLanguageTypescript
}
