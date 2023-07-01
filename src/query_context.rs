use std::sync::Arc;

use tree_sitter::{Language, Query};

#[derive(Debug)]
pub struct QueryContext {
    pub query: Arc<Query>,
    pub capture_index: u32,
    pub language: Language,
    pub filter_library_path: Option<String>,
    pub filter_arg: Option<String>,
}

impl QueryContext {
    pub fn new(
        query: Arc<Query>,
        capture_index: u32,
        language: Language,
        filter_library_path: Option<String>,
        filter_arg: Option<String>,
    ) -> Self {
        Self {
            query,
            capture_index,
            language,
            filter_library_path,
            filter_arg,
        }
    }
}
