use std::{
    cell::{OnceCell, RefCell},
    ptr,
    rc::Rc,
};

use crate::{searcher::Searcher, Args};

thread_local! {
    static SEARCHER: OnceCell<(Rc<RefCell<Searcher>>, *const Args)> = Default::default();
}
pub(crate) fn get_searcher(args: &Args) -> Rc<RefCell<Searcher>> {
    SEARCHER.with(|searcher| {
        let (searcher, args_when_initialized) = searcher.get_or_init(|| {
            (
                Rc::new(RefCell::new(args.get_searcher(&args.use_paths()))),
                args,
            )
        });
        assert!(
            ptr::eq(*args_when_initialized, args),
            "Using multiple instances of args not supported"
        );
        searcher.clone()
    })
}
