use ignore::types::{Types, TypesBuilder};

use crate::language::{SupportedLanguage, ALL_SUPPORTED_LANGUAGES};

pub(crate) fn get_project_file_walker_types(language: Option<SupportedLanguage>) -> Types {
    let mut types_builder = TypesBuilder::new();
    types_builder.add_defaults();
    if let Some(language) = language {
        types_builder.select(language.name_for_ignore_select);
    } else {
        for language in ALL_SUPPORTED_LANGUAGES.iter() {
            types_builder.select(language.name_for_ignore_select);
        }
    }
    types_builder.build().unwrap()
}
