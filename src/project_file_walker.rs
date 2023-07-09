use std::{
    sync::{mpsc, mpsc::Receiver},
    thread,
    thread::JoinHandle,
};

use ignore::{
    types::{Types, TypesBuilder},
    DirEntry, WalkParallel, WalkState,
};
use rayon::{iter::IterBridge, prelude::*};

use crate::{
    err_message,
    language::{
        SupportedLanguage, ALL_SUPPORTED_LANGUAGES,
        ALL_SUPPORTED_LANGUAGES_BY_NAME_FOR_IGNORE_SELECT,
    },
};

pub(crate) fn into_parallel_iterator(
    walk_parallel: WalkParallel,
) -> IterBridge<WalkParallelIterator> {
    WalkParallelIterator::new(walk_parallel).par_bridge()
}

pub(crate) struct WalkParallelIterator {
    receiver_iterator: <Receiver<(DirEntry, Vec<SupportedLanguage>)> as IntoIterator>::IntoIter,
    _handle: JoinHandle<()>,
}

impl WalkParallelIterator {
    pub fn new(walk_parallel: WalkParallel) -> Self {
        let (sender, receiver) = mpsc::channel::<(DirEntry, Vec<SupportedLanguage>)>();
        let handle = thread::spawn(move || {
            let ignore = &walk_parallel.ignore();
            walk_parallel.run(move || {
                Box::new({
                    let sender = sender.clone();
                    move |entry_and_match_metadata| {
                        let (entry, match_metadata) = match entry_and_match_metadata {
                            Err(err) => {
                                err_message!("{err}");
                                return WalkState::Continue;
                            }
                            Ok(entry_and_match_metadata) => entry_and_match_metadata,
                        };
                        if !entry.metadata().unwrap().is_file() {
                            return WalkState::Continue;
                        }
                        let matched_languages = match_metadata
                            .or_else(|| {
                                ignore
                                    .should_skip_entry_with_match_metadata_token(&entry)
                                    .1
                                    .map(|match_metadata_token| {
                                        ignore.get_match_metadata(match_metadata_token)
                                    })
                            })
                            .map(|match_metadata| {
                                match_metadata
                                    .matching_file_types
                                    .map(|file_type_def| {
                                        ALL_SUPPORTED_LANGUAGES_BY_NAME_FOR_IGNORE_SELECT
                                            .get(&file_type_def.name())
                                            .copied()
                                            .unwrap()
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default();
                        sender.send((entry, matched_languages)).unwrap();
                        WalkState::Continue
                    }
                })
            });
        });
        Self {
            receiver_iterator: receiver.into_iter(),
            _handle: handle,
        }
    }
}

impl Iterator for WalkParallelIterator {
    type Item = (DirEntry, Vec<SupportedLanguage>);

    fn next(&mut self) -> Option<Self::Item> {
        self.receiver_iterator.next()
    }
}

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
