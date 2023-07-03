use std::{
    cell::{OnceCell, RefCell},
    ptr,
    rc::Rc,
};

use termcolor::{Buffer, BufferWriter};

use crate::{printer::Standard, Args};

pub type Printer = Standard<Buffer>;

thread_local! {
    static PRINTER: OnceCell<(Rc<RefCell<Printer>>, *const Args)> = Default::default();
}
pub(crate) fn get_printer(buffer_writer: &BufferWriter, args: &Args) -> Rc<RefCell<Printer>> {
    PRINTER.with(|printer| {
        let (printer, args_when_initialized) =
            printer.get_or_init(|| (Rc::new(RefCell::new(args.get_printer(buffer_writer))), args));
        assert!(
            ptr::eq(*args_when_initialized, args),
            "Using multiple instances of args not supported"
        );
        printer.clone()
    })
}
