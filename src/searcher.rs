use std::{
    cell::{OnceCell, RefCell},
    rc::Rc,
};

use grep::searcher::{Searcher, SearcherBuilder};

use crate::OutputMode;

thread_local! {
    static SEARCHER: OnceCell<(Rc<RefCell<Searcher>>, OutputMode)> = Default::default();
}
pub(crate) fn get_searcher(output_mode: OutputMode) -> Rc<RefCell<Searcher>> {
    SEARCHER.with(|searcher| {
        let (searcher, output_mode_when_initialized) = searcher.get_or_init(|| {
            (
                Rc::new(RefCell::new(create_searcher(output_mode))),
                output_mode,
            )
        });
        assert!(
            *output_mode_when_initialized == output_mode,
            "Using multiple output modes not supported"
        );
        searcher.clone()
    })
}

fn create_searcher(output_mode: OutputMode) -> Searcher {
    match output_mode {
        OutputMode::Normal => SearcherBuilder::new().multi_line(true).build(),
        OutputMode::Vimgrep => SearcherBuilder::new()
            .multi_line(true)
            .line_number(true)
            .build(),
    }
}
