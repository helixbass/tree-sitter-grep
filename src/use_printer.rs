use std::{
    cell::{OnceCell, RefCell},
    rc::Rc,
};

use termcolor::{Buffer, BufferWriter};

use crate::{
    args::OutputMode,
    printer::{Standard, StandardBuilder},
};

type Printer = Standard<Buffer>;

thread_local! {
    static PRINTER: OnceCell<(Rc<RefCell<Printer>>, OutputMode)> = Default::default();
}
pub(crate) fn get_printer(
    buffer_writer: &BufferWriter,
    output_mode: OutputMode,
) -> Rc<RefCell<Printer>> {
    PRINTER.with(|printer| {
        let (printer, output_mode_when_initialized) = printer.get_or_init(|| {
            (
                Rc::new(RefCell::new(create_printer(buffer_writer, output_mode))),
                output_mode,
            )
        });
        assert!(
            *output_mode_when_initialized == output_mode,
            "Using multiple output modes not supported"
        );
        printer.clone()
    })
}

fn create_printer(buffer_writer: &BufferWriter, output_mode: OutputMode) -> Printer {
    match output_mode {
        OutputMode::Normal => Standard::new(buffer_writer.buffer()),
        OutputMode::Vimgrep => StandardBuilder::new()
            .per_match(true)
            .per_match_one_line(true)
            .column(true)
            .build(buffer_writer.buffer()),
    }
}
