use std::{
    collections::{HashMap, HashSet},
    ops::{Deref, Index},
    path::Path,
};

use once_cell::sync::Lazy;
use proc_macros::fixed_map;
use tree_sitter::Language;

fixed_map! {
    name => SupportedLanguage,
    variants => [
        C,
        #[value(name = "c++")]
        #[strum(serialize = "C++")]
        Cpp,
        #[strum(serialize = "C#")]
        CSharp,
        #[strum(serialize = "CSS")]
        Css,
        Dockerfile,
        Elisp,
        Elm,
        Go,
        #[strum(serialize = "HTML")]
        Html,
        Java,
        Javascript,
        #[strum(serialize = "JSON")]
        Json,
        Kotlin,
        Lua,
        #[strum(serialize = "Objective-C")]
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
    pub fn language(&self, path: Option<&Path>) -> Language {
        match &SUPPORTED_LANGUAGE_LANGUAGES[*self] {
            SingleLanguageOrLanguageFromPath::SingleLanguage(language) => *language,
            SingleLanguageOrLanguageFromPath::LanguageFromPath(language_from_path) => {
                language_from_path.from_path(path)
            }
        }
    }

    pub fn name_for_ignore_select(&self) -> &'static str {
        SUPPORTED_LANGUAGE_NAMES_FOR_IGNORE_SELECT[*self]
    }

    pub fn comment_kinds(&self) -> &'static HashSet<&'static str> {
        &SUPPORTED_LANGUAGE_COMMENT_KINDS[*self]
    }
}

enum SingleLanguageOrLanguageFromPath {
    SingleLanguage(Language),
    LanguageFromPath(Box<dyn LanguageFromPath>),
}

impl From<Language> for SingleLanguageOrLanguageFromPath {
    fn from(value: Language) -> Self {
        Self::SingleLanguage(value)
    }
}

trait LanguageFromPath: Send + Sync {
    #[allow(clippy::wrong_self_convention)]
    fn from_path(&self, path: Option<&Path>) -> Language;
}

struct TypescriptLanguageFromPath;

impl LanguageFromPath for TypescriptLanguageFromPath {
    fn from_path(&self, path: Option<&Path>) -> Language {
        match path.and_then(|path| path.extension()) {
            Some(extension) if "tsx" == extension => tree_sitter_typescript::language_tsx(),
            _ => tree_sitter_typescript::language_typescript(),
        }
    }
}

static SUPPORTED_LANGUAGE_LANGUAGES: Lazy<BySupportedLanguage<SingleLanguageOrLanguageFromPath>> =
    Lazy::new(|| {
        by_supported_language!(
            Rust => tree_sitter_rust::language().into(),
            Typescript => SingleLanguageOrLanguageFromPath::LanguageFromPath(Box::new(TypescriptLanguageFromPath)),
            Javascript => tree_sitter_javascript::language().into(),
            Swift => tree_sitter_swift::language().into(),
            ObjectiveC => tree_sitter_objc::language().into(),
            Toml => tree_sitter_toml::language().into(),
            Python => tree_sitter_python::language().into(),
            Ruby => tree_sitter_ruby::language().into(),
            C => tree_sitter_c::language().into(),
            Cpp => tree_sitter_cpp::language().into(),
            Go => tree_sitter_go::language().into(),
            Java => tree_sitter_java::language().into(),
            CSharp => tree_sitter_c_sharp::language().into(),
            Kotlin => tree_sitter_kotlin::language().into(),
            Elisp => tree_sitter_elisp::language().into(),
            Elm => tree_sitter_elm::language().into(),
            Dockerfile => tree_sitter_dockerfile::language().into(),
            Html => tree_sitter_html::language().into(),
            TreeSitterQuery => tree_sitter_query::language().into(),
            Json => tree_sitter_json::language().into(),
            Css => tree_sitter_css::language().into(),
            Lua => tree_sitter_lua::language().into(),
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

static SUPPORTED_LANGUAGE_COMMENT_KINDS: Lazy<BySupportedLanguage<HashSet<&'static str>>> =
    Lazy::new(|| {
        by_supported_language!(
            Rust => ["line_comment", "block_comment"].into(),
            Typescript => ["comment"].into(),
            Javascript => ["comment"].into(),
            Swift => ["comment"].into(),
            ObjectiveC => ["comment"].into(),
            Toml => ["comment"].into(),
            Python => ["comment"].into(),
            Ruby => ["comment"].into(),
            C => ["comment"].into(),
            Cpp => ["comment"].into(),
            Go => ["comment"].into(),
            Java => ["comment"].into(),
            CSharp => ["comment"].into(),
            Kotlin => ["comment"].into(),
            Elisp => ["comment"].into(),
            Elm => ["comment"].into(),
            Dockerfile => ["comment"].into(),
            Html => ["comment"].into(),
            TreeSitterQuery => ["comment"].into(),
            Json => ["comment"].into(),
            Css => ["comment"].into(),
            Lua => ["comment"].into(),
        )
    });

#[cfg(test)]
mod tests {
    use speculoos::prelude::*;

    use super::*;

    #[test]
    fn test_supported_language_language_simple() {
        assert_that!(&SupportedLanguage::Rust.language(Some("foo.rs".as_ref())))
            .is_equal_to(tree_sitter_rust::language());
        assert_that!(&SupportedLanguage::Rust.language(None))
            .is_equal_to(tree_sitter_rust::language());
    }

    #[test]
    fn test_supported_language_language_typescript() {
        assert_that!(&SupportedLanguage::Typescript.language(Some("foo.tsx".as_ref())))
            .is_equal_to(tree_sitter_typescript::language_tsx());
        assert_that!(&SupportedLanguage::Typescript.language(Some("foo.ts".as_ref())))
            .is_equal_to(tree_sitter_typescript::language_typescript());
        assert_that!(&SupportedLanguage::Typescript.language(None))
            .is_equal_to(tree_sitter_typescript::language_typescript());
    }
}
