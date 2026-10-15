//! Command line interface.
//!
//! ```text
//! dpm-nbe nf    # read a program from stdin, print its normal form and type
//! dpm-nbe type  # read a program from stdin, print its type
//! dpm-nbe --help
//! ```

use dpm_nbe::{run_named, Error, Mode, Pos, STACK_SIZE};
use std::io::Read;
use std::process::ExitCode;

const HELP: &str = "\
usage: dpm-nbe [--help|nf|type]
  --help : display this message
  nf     : read & typecheck a program from stdin, print its normal form and type
  type   : read & typecheck a program from stdin, print its type";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = match args.as_slice() {
        [a] if a == "nf" => Mode::Nf,
        [a] if a == "type" => Mode::Type,
        _ => {
            println!("{HELP}");
            return ExitCode::SUCCESS;
        }
    };

    let mut bytes = Vec::new();
    if let Err(e) = std::io::stdin().read_to_end(&mut bytes) {
        eprintln!("error: cannot read stdin: {e}");
        return ExitCode::from(2);
    }
    let src = match String::from_utf8(bytes) {
        Ok(src) => src,
        Err(_) => {
            // A malformed encoding is an input error, so report it like any
            // other and exit 1.
            let e = Error {
                msg: "input is not valid UTF-8".to_string(),
                pos: Pos::initial("<stdin>"),
            };
            eprintln!("{}", e.render(""));
            return ExitCode::from(1);
        }
    };

    // Parsing, elaborating, evaluating and quoting are all structural
    // recursions over the program, so run them on a thread with a generous
    // stack instead of relying on the platform default.
    let worker = std::thread::Builder::new()
        .name("dpm-nbe".to_string())
        .stack_size(STACK_SIZE)
        .spawn(move || match run_named(&src, "<stdin>", mode) {
            Ok(out) => (ExitCode::SUCCESS, out, String::new()),
            Err(e) => (ExitCode::from(1), String::new(), e.render(&src)),
        });

    match worker {
        Ok(handle) => match handle.join() {
            Ok((code, out, err)) => {
                if !out.is_empty() {
                    println!("{out}");
                }
                if !err.is_empty() {
                    eprintln!("{err}");
                }
                code
            }
            Err(payload) => {
                // Surface the invariant message instead of swallowing it.
                let msg = payload
                    .downcast_ref::<&str>()
                    .copied()
                    .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
                    .unwrap_or("unknown panic");
                eprintln!("error: internal panic while elaborating the program: {msg}");
                ExitCode::from(101)
            }
        },
        Err(e) => {
            eprintln!("error: cannot spawn the elaborator thread: {e}");
            ExitCode::from(2)
        }
    }
}
