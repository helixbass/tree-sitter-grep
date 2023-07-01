use std::sync::Arc;

use grep::matcher::{Match, NoCaptures, NoError};
use tree_sitter::{Language, Query};

use crate::{plugin::get_loaded_filter, treesitter::get_sorted_matches};

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

// impl Matcher for TreeSitterMatcher<'_> {
//     type Captures = NoCaptures;

//     type Error = NoError;

//     fn find_at(&self, haystack: &[u8], at: usize) -> Result<Option<Match>,
// Self::Error> {         let mut matches_info = self.matches_info.borrow_mut();
//         let matches_info = matches_info.get_or_insert_with(|| {
//             assert!(at == 0);
//             PopulatedMatchesInfo {
//                 matches: get_sorted_matches(
//                     self.query,
//                     self.capture_index,
//                     haystack,
//                     self.language,
//                     get_loaded_filter(
//                         self.filter_library_path.as_deref(),
//                         self.filter_arg.as_deref(),
//                     ),
//                 ),
//                 text_len: haystack.len(),
//             }
//         });
//         Ok(matches_info.find_and_adjust_first_in_range_match(haystack.len(),
// at))     }

//     fn new_captures(&self) -> Result<Self::Captures, Self::Error> {
//         Ok(NoCaptures::new())
//     }
// }
