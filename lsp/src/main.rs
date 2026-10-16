//! `dpm-nbe-lsp` — a stdio language server for the `dpm-nbe` language.
//!
//! ```text
//! dpm-nbe-lsp            # speak LSP on stdin/stdout
//! dpm-nbe-lsp --version
//! dpm-nbe-lsp --help
//! ```

use dpm_nbe_lsp::{stdio, VERSION};
use std::process::ExitCode;

const HELP: &str = "\
usage: dpm-nbe-lsp [--help|--version]
  --help     : display this message
  --version  : print the server version
  (no option): speak the Language Server Protocol on stdin/stdout";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [] => run(),
        [a] if a == "--stdio" => run(),
        [a] if a == "--help" || a == "-h" => {
            println!("{HELP}");
            ExitCode::SUCCESS
        }
        [a] if a == "--version" || a == "-V" => {
            println!("dpm-nbe-lsp {VERSION}");
            ExitCode::SUCCESS
        }
        [a] => {
            eprintln!("dpm-nbe-lsp: unknown argument '{a}'");
            eprintln!("{HELP}");
            ExitCode::from(2)
        }
        _ => {
            eprintln!("dpm-nbe-lsp: too many arguments");
            eprintln!("{HELP}");
            ExitCode::from(2)
        }
    }
}

fn run() -> ExitCode {
    match stdio::serve() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("dpm-nbe-lsp: {e}");
            ExitCode::from(2)
        }
    }
}
