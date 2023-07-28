use std::{
    sync::{mpsc, mpsc::Receiver, Arc, Mutex},
    thread,
    thread::JoinHandle,
};

use ignore::{
    types::{Types, TypesBuilder},
    DirEntry, WalkParallel, WalkState,
};
use rayon::{iter::IterBridge, prelude::*};

use crate::{
    language::{
        SupportedLanguage, ALL_SUPPORTED_LANGUAGES,
        ALL_SUPPORTED_LANGUAGES_BY_NAME_FOR_IGNORE_SELECT,
    },
    NonFatalError,
};

pub(crate) fn into_parallel_iterator(
    walk_parallel: WalkParallel,
    non_fatal_errors: Arc<Mutex<Vec<NonFatalError>>>,
) -> IterBridge<WalkParallelIterator> {
    WalkParallelIterator::new(walk_parallel, non_fatal_errors).par_bridge()
}

pub(crate) struct WalkParallelIterator {
    receiver_iterator: <Receiver<(DirEntry, Vec<SupportedLanguage>)> as IntoIterator>::IntoIter,
    _handle: JoinHandle<()>,
}

impl WalkParallelIterator {
    pub fn new(
        walk_parallel: WalkParallel,
        non_fatal_errors: Arc<Mutex<Vec<NonFatalError>>>,
    ) -> Self {
        let (sender, receiver) = mpsc::channel::<(DirEntry, Vec<SupportedLanguage>)>();
        let handle = thread::spawn(move || {
            let ignore = &walk_parallel.ignore();
            walk_parallel.run(move || {
                Box::new({
                    let sender = sender.clone();
                    let non_fatal_errors = non_fatal_errors.clone();
                    move |entry_and_match_metadata| {
                        let (entry, match_metadata) = match entry_and_match_metadata {
                            Err(err) => {
                                non_fatal_errors.lock().unwrap().push(err.into());
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

pub(crate) fn get_project_file_walker_types(
    languages: Option<impl IntoIterator<Item = SupportedLanguage>>,
) -> Types {
    let mut types_builder = TypesBuilder::new();
    types_builder.add_defaults();
    if let Some(languages) = languages {
        for language in languages {
            types_builder.select(language.name_for_ignore_select());
        }
    } else {
        for language in ALL_SUPPORTED_LANGUAGES.values() {
            types_builder.select(language.name_for_ignore_select());
        }
    }
    types_builder.build().unwrap()
}
