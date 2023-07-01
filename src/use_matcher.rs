use std::cell::RefCell;

use grep::matcher::{Match, Matcher, NoCaptures, NoError};
use tree_sitter::{Language, Query};

use crate::{plugin::get_loaded_filter, treesitter::get_sorted_matches};

#[derive(Debug)]
pub(crate) struct TreeSitterMatcher<'query> {
    query: &'query Query,
    capture_index: u32,
    language: Language,
    filter_library_path: Option<String>,
    filter_arg: Option<String>,
    matches_info: RefCell<Option<PopulatedMatchesInfo>>,
}

impl<'query> TreeSitterMatcher<'query> {
    pub fn new(
        query: &'query Query,
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
            matches_info: Default::default(),
        }
    }
}

impl Matcher for TreeSitterMatcher<'_> {
    type Captures = NoCaptures;

    type Error = NoError;

    fn find_at(&self, haystack: &[u8], at: usize) -> Result<Option<Match>, Self::Error> {
        let mut matches_info = self.matches_info.borrow_mut();
        let matches_info = matches_info.get_or_insert_with(|| {
            assert!(at == 0);
            PopulatedMatchesInfo {
                matches: get_sorted_matches(
                    self.query,
                    self.capture_index,
                    haystack,
                    self.language,
                    get_loaded_filter(
                        self.filter_library_path.as_deref(),
                        self.filter_arg.as_deref(),
                    ),
                ),
                text_len: haystack.len(),
            }
        });
        Ok(matches_info.find_and_adjust_first_in_range_match(haystack.len(), at))
    }

    fn new_captures(&self) -> Result<Self::Captures, Self::Error> {
        Ok(NoCaptures::new())
    }
}

#[derive(Debug)]
struct PopulatedMatchesInfo {
    matches: Vec<Match>,
    text_len: usize,
}

impl PopulatedMatchesInfo {
    pub fn find_and_adjust_first_in_range_match(
        &self,
        haystack_len: usize,
        at: usize,
    ) -> Option<Match> {
        self.find_first_in_range_match(haystack_len, at)
            .map(|match_| adjust_match(match_, haystack_len, self.text_len))
    }

    pub fn find_first_in_range_match(&self, haystack_len: usize, at: usize) -> Option<&Match> {
        let start_index = at + (self.text_len - haystack_len);
        self.matches
            .iter()
            .find(|match_| match_.start() >= start_index)
    }
}

fn adjust_match(match_: &Match, haystack_len: usize, total_file_text_len: usize) -> Match {
    let offset_in_file = total_file_text_len - haystack_len;
    Match::new(
        match_.start() - offset_in_file,
        match_.end() - offset_in_file,
    )
}
