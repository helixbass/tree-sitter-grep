// derived from https://github.com/BurntSushi/ripgrep/blob/master/crates/printer/src/stats.rs

use std::{
    ops::{Add, AddAssign},
    time::Duration,
};

use super::util::NiceDuration;

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct Stats {
    elapsed: NiceDuration,
    searches: u64,
    searches_with_match: u64,
    bytes_searched: u64,
    bytes_printed: u64,
    matched_lines: u64,
    matches: u64,
}

impl Add for Stats {
    type Output = Stats;

    fn add(self, rhs: Stats) -> Stats {
        self + &rhs
    }
}

impl<'a> Add<&'a Stats> for Stats {
    type Output = Stats;

    fn add(self, rhs: &'a Stats) -> Stats {
        Stats {
            elapsed: NiceDuration(self.elapsed.0 + rhs.elapsed.0),
            searches: self.searches + rhs.searches,
            searches_with_match: self.searches_with_match + rhs.searches_with_match,
            bytes_searched: self.bytes_searched + rhs.bytes_searched,
            bytes_printed: self.bytes_printed + rhs.bytes_printed,
            matched_lines: self.matched_lines + rhs.matched_lines,
            matches: self.matches + rhs.matches,
        }
    }
}

impl AddAssign for Stats {
    fn add_assign(&mut self, rhs: Stats) {
        *self += &rhs;
    }
}

impl<'a> AddAssign<&'a Stats> for Stats {
    fn add_assign(&mut self, rhs: &'a Stats) {
        self.elapsed.0 += rhs.elapsed.0;
        self.searches += rhs.searches;
        self.searches_with_match += rhs.searches_with_match;
        self.bytes_searched += rhs.bytes_searched;
        self.bytes_printed += rhs.bytes_printed;
        self.matched_lines += rhs.matched_lines;
        self.matches += rhs.matches;
    }
}

impl Stats {
    pub fn new() -> Stats {
        Stats::default()
    }

    pub fn elapsed(&self) -> Duration {
        self.elapsed.0
    }

    pub fn searches(&self) -> u64 {
        self.searches
    }

    pub fn searches_with_match(&self) -> u64 {
        self.searches_with_match
    }

    pub fn bytes_searched(&self) -> u64 {
        self.bytes_searched
    }

    pub fn bytes_printed(&self) -> u64 {
        self.bytes_printed
    }

    pub fn matched_lines(&self) -> u64 {
        self.matched_lines
    }

    pub fn matches(&self) -> u64 {
        self.matches
    }

    pub fn add_elapsed(&mut self, duration: Duration) {
        self.elapsed.0 += duration;
    }

    pub fn add_searches(&mut self, n: u64) {
        self.searches += n;
    }

    pub fn add_searches_with_match(&mut self, n: u64) {
        self.searches_with_match += n;
    }

    pub fn add_bytes_searched(&mut self, n: u64) {
        self.bytes_searched += n;
    }

    pub fn add_bytes_printed(&mut self, n: u64) {
        self.bytes_printed += n;
    }

    pub fn add_matched_lines(&mut self, n: u64) {
        self.matched_lines += n;
    }

    pub fn add_matches(&mut self, n: u64) {
        self.matches += n;
    }
}
