use std::sync::Arc;

use tree_sitter::{Language, Query};

use crate::plugin::Filterer;

pub struct QueryContext {
    pub query: Arc<Query>,
    pub capture_index: u32,
    pub language: Language,
    pub filter: Option<Arc<Filterer>>,
}

impl std::fmt::Debug for QueryContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryContext")
            .field("query", &self.query)
            .field("capture_index", &self.capture_index)
            .field("language", &self.language)
            // .field("filter", &self.filter)
            .finish()
    }
}

impl QueryContext {
    pub fn new(
        query: Arc<Query>,
        capture_index: u32,
        language: Language,
        filter: Option<Arc<Filterer>>,
    ) -> Self {
        Self {
            query,
            capture_index,
            language,
            filter,
        }
    }
}
