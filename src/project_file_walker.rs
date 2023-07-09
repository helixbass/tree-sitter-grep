use std::{
    path::PathBuf,
    sync::{mpsc, mpsc::Receiver},
    thread,
    thread::JoinHandle,
};

use ignore::{types::TypesBuilder, DirEntry, WalkBuilder, WalkParallel, WalkState};
use rayon::{iter::IterBridge, prelude::*};

use crate::{
    err_message,
    language::{get_all_supported_languages, SupportedLanguage},
};

trait IntoParallelIterator {
    fn into_parallel_iterator(self) -> IterBridge<WalkParallelIterator>;
}

impl IntoParallelIterator for WalkParallel {
    fn into_parallel_iterator(self) -> IterBridge<WalkParallelIterator> {
        WalkParallelIterator::new(self).par_bridge()
    }
}

pub(crate) struct WalkParallelIterator {
    receiver_iterator: <Receiver<DirEntry> as IntoIterator>::IntoIter,
    _handle: JoinHandle<()>,
}

impl WalkParallelIterator {
    pub fn new(walk_parallel: WalkParallel) -> Self {
        let (sender, receiver) = mpsc::channel::<DirEntry>();
        let handle = thread::spawn(move || {
            walk_parallel.run(move || {
                Box::new({
                    let sender = sender.clone();
                    move |entry| {
                        let entry = match entry {
                            Err(err) => {
                                err_message!("{err}");
                                return WalkState::Continue;
                            }
                            Ok(entry) => entry,
                        };
                        if !entry.metadata().unwrap().is_file() {
                            return WalkState::Continue;
                        }
                        sender.send(entry).unwrap();
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
    type Item = DirEntry;

    fn next(&mut self) -> Option<Self::Item> {
        self.receiver_iterator.next()
    }
}

fn get_project_file_walker(
    language: Option<&dyn SupportedLanguage>,
    paths: &[PathBuf],
) -> WalkParallel {
    assert!(!paths.is_empty());
    let mut builder = WalkBuilder::new(&paths[0]);
    let mut types_builder = TypesBuilder::new();
    types_builder.add_defaults();
    if let Some(language) = language {
        types_builder.select(language.name_for_ignore_select());
    } else {
        for language in get_all_supported_languages().values() {
            types_builder.select(language.name_for_ignore_select());
        }
    }
    builder.types(types_builder.build().unwrap());
    for path in &paths[1..] {
        builder.add(path);
    }
    builder.build_parallel()
}

pub(crate) fn get_project_file_parallel_iterator(
    language: Option<&dyn SupportedLanguage>,
    paths: &[PathBuf],
) -> IterBridge<WalkParallelIterator> {
    get_project_file_walker(language, paths).into_parallel_iterator()
}
