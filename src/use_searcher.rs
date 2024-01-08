use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{searcher::Searcher, Args};

thread_local! {
    static SEARCHER_PER_ARGS_INSTANCE: RefCell<HashMap<*const Args, Rc<RefCell<Searcher>>>> = Default::default();
}
pub(crate) fn get_searcher(args: &Args) -> Rc<RefCell<Searcher>> {
    SEARCHER_PER_ARGS_INSTANCE.with(|searcher_per_args_instance| {
        searcher_per_args_instance
            .borrow_mut()
            .entry(args)
            .or_insert_with(|| Rc::new(RefCell::new(args.get_searcher())))
            .clone()
    })
}
