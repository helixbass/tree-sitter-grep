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

fixed_map! {
    name => SupportedLanguageLanguage,
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
        Tsx,
        Typescript,
    ],
}

impl SupportedLanguage {
    pub fn language(&self, path: Option<&Path>) -> Language {
        self.supported_language_language(path).language()
    }

    pub fn supported_language_language(&self, path: Option<&Path>) -> SupportedLanguageLanguage {
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

impl SupportedLanguageLanguage {
    pub fn language(&self) -> Language {
        SUPPORTED_LANGUAGE_LANGUAGE_LANGUAGES[*self]
    }
}

enum SingleLanguageOrLanguageFromPath {
    SingleLanguage(SupportedLanguageLanguage),
    LanguageFromPath(Box<dyn LanguageFromPath>),
}

impl From<SupportedLanguageLanguage> for SingleLanguageOrLanguageFromPath {
    fn from(value: SupportedLanguageLanguage) -> Self {
        Self::SingleLanguage(value)
    }
}

trait LanguageFromPath: Send + Sync {
    #[allow(clippy::wrong_self_convention)]
    fn from_path(&self, path: Option<&Path>) -> SupportedLanguageLanguage;
}

struct TypescriptLanguageFromPath;

impl LanguageFromPath for TypescriptLanguageFromPath {
    fn from_path(&self, path: Option<&Path>) -> SupportedLanguageLanguage {
        match path.and_then(|path| path.extension()) {
            Some(extension) if "tsx" == extension => SupportedLanguageLanguage::Tsx,
            _ => SupportedLanguageLanguage::Typescript,
        }
    }
}

static SUPPORTED_LANGUAGE_LANGUAGES: Lazy<BySupportedLanguage<SingleLanguageOrLanguageFromPath>> =
    Lazy::new(|| {
        by_supported_language!(
            Rust => SupportedLanguageLanguage::Rust.into(),
            Typescript => SingleLanguageOrLanguageFromPath::LanguageFromPath(Box::new(TypescriptLanguageFromPath)),
            Javascript => SupportedLanguageLanguage::Javascript.into(),
            Swift => SupportedLanguageLanguage::Swift.into(),
            ObjectiveC => SupportedLanguageLanguage::ObjectiveC.into(),
            Toml => SupportedLanguageLanguage::Toml.into(),
            Python => SupportedLanguageLanguage::Python.into(),
            Ruby => SupportedLanguageLanguage::Ruby.into(),
            C => SupportedLanguageLanguage::C.into(),
            Cpp => SupportedLanguageLanguage::Cpp.into(),
            Go => SupportedLanguageLanguage::Go.into(),
            Java => SupportedLanguageLanguage::Java.into(),
            CSharp => SupportedLanguageLanguage::CSharp.into(),
            Kotlin => SupportedLanguageLanguage::Kotlin.into(),
            Elisp => SupportedLanguageLanguage::Elisp.into(),
            Elm => SupportedLanguageLanguage::Elm.into(),
            Dockerfile => SupportedLanguageLanguage::Dockerfile.into(),
            Html => SupportedLanguageLanguage::Html.into(),
            TreeSitterQuery => SupportedLanguageLanguage::TreeSitterQuery.into(),
            Json => SupportedLanguageLanguage::Json.into(),
            Css => SupportedLanguageLanguage::Css.into(),
            Lua => SupportedLanguageLanguage::Lua.into(),
        )
    });

static SUPPORTED_LANGUAGE_LANGUAGE_LANGUAGES: Lazy<BySupportedLanguageLanguage<Language>> =
    Lazy::new(|| {
        by_supported_language_language!(
            Rust => tree_sitter_rust::language(),
            Typescript => tree_sitter_typescript::language_typescript(),
            Tsx => tree_sitter_typescript::language_tsx(),
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
