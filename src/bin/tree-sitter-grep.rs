use std::process;

use clap::Parser;
use tree_sitter_grep::{run, Args, RunStatus};

pub fn main() {
    let args = Args::parse();
    match run(args) {
        Ok(RunStatus {
            non_fatal_errors,
            matched,
        }) => {
            if !non_fatal_errors.is_empty() {
                for non_fatal_error in non_fatal_errors {
                    eprintln!("{non_fatal_error}");
                }
                exit(ExitCode::Error);
            } else if matched {
                exit(ExitCode::Success);
            } else {
                exit(ExitCode::NoMatches);
            }
        }
        Err(error) => {
            eprintln!("error: {error}");
            exit(ExitCode::Error);
        }
    }
}

#[derive(Copy, Clone)]
enum ExitCode {
    Success = 0,
    NoMatches = 1,
    Error = 2,
}

fn exit(exit_code: ExitCode) -> ! {
    process::exit(exit_code as i32);
}
