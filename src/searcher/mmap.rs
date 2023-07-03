// derived from https://github.com/BurntSushi/ripgrep/blob/master/crates/searcher/src/searcher/mmap.rs

use std::{fs::File, path::Path};

use memmap::Mmap;

#[derive(Clone, Debug)]
pub struct MmapChoice(MmapChoiceImpl);

#[derive(Clone, Debug)]
enum MmapChoiceImpl {
    Auto,
    Never,
}

impl Default for MmapChoice {
    fn default() -> MmapChoice {
        MmapChoice(MmapChoiceImpl::Never)
    }
}

impl MmapChoice {
    pub unsafe fn auto() -> MmapChoice {
        MmapChoice(MmapChoiceImpl::Auto)
    }

    pub fn never() -> MmapChoice {
        MmapChoice(MmapChoiceImpl::Never)
    }

    pub(crate) fn open(&self, file: &File, path: Option<&Path>) -> Option<Mmap> {
        if !self.is_enabled() {
            return None;
        }
        if cfg!(target_os = "macos") {
            return None;
        }
        match unsafe { Mmap::map(file) } {
            Ok(mmap) => Some(mmap),
            Err(err) => {
                if let Some(path) = path {
                    log::debug!("{}: failed to open memory map: {}", path.display(), err);
                } else {
                    log::debug!("failed to open memory map: {}", err);
                }
                None
            }
        }
    }

    pub(crate) fn is_enabled(&self) -> bool {
        match self.0 {
            MmapChoiceImpl::Auto => true,
            MmapChoiceImpl::Never => false,
        }
    }
}
