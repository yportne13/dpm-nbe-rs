//! Language server for `dpm-nbe`.
//!
//! The crate is split so that the *protocol* is transport agnostic and can run
//! both behind a native `stdin`/`stdout` pair and inside a browser
//! (`wasm32-unknown-unknown`, driven from JavaScript):
//!
//! * [`analysis`] — pure source analysis on top of the `dpm-nbe` lexer and
//!   checker: tokens, binder scopes, hover, completion, definition, symbols and
//!   diagnostics.  No I/O, no threads.
//! * [`protocol`] — JSON-RPC / LSP message handling
//!   ([`protocol::Server::handle_message`] takes one message as a string and
//!   returns the messages to send back).
//! * [`stdio`] — `Content-Length` framing around real `stdin`/`stdout` for the
//!   `dpm-nbe-lsp` binary.  Native only; the browser build never calls it.

pub mod analysis;
pub mod protocol;
pub mod stdio;

pub use protocol::Server;

/// Version reported in the `initialize` result (`serverInfo.version`).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Name reported in the `initialize` result (`serverInfo.name`).
pub const SERVER_NAME: &str = "dpm-nbe-lsp";

/// Run a checking job on a thread with a large stack, converting a panic into
/// an `Err` so that one bad file cannot take the server down.
///
/// Parsing, elaboration and quotation all recurse over the structure of the
/// program, which is why `dpm-nbe`'s own CLI does the same thing.  On
/// `wasm32-unknown-unknown` there are no threads: the job runs inline and the
/// module's shadow stack is sized at link time instead (see `web/api`).
#[cfg(not(target_arch = "wasm32"))]
pub fn with_big_stack<T: Send + 'static>(job: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    let handle = std::thread::Builder::new()
        .name("dpm-nbe-analysis".to_string())
        .stack_size(dpm_nbe::STACK_SIZE)
        .spawn(move || std::panic::catch_unwind(std::panic::AssertUnwindSafe(job)));
    match handle {
        Ok(handle) => match handle.join() {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(payload)) => Err(panic_message(&payload)),
            Err(payload) => Err(panic_message(&payload)),
        },
        Err(e) => Err(format!("cannot spawn the analysis thread: {e}")),
    }
}

#[cfg(target_arch = "wasm32")]
pub fn with_big_stack<T: Send + 'static>(job: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    // `catch_unwind` is a no-op under `panic=abort`, which is what
    // `wasm32-unknown-unknown` uses: a panic there is an instance trap and the
    // page has to reload, exactly like the reference web IDEs behave.
    Ok(job())
}

#[cfg(not(target_arch = "wasm32"))]
fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    let text = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("unknown panic");
    format!("internal error while checking this file: {text}")
}
