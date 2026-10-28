//! The `dpm-nbe` language server, compiled to WebAssembly.
//!
//! This is a *thin* binding: the analysis and the JSON-RPC/LSP protocol are the
//! same `dpm-nbe-lsp` code the native `dpm-nbe-lsp` binary runs, so the browser
//! and the editor cannot drift apart.  Only the transport differs — instead of
//! `Content-Length` frames on a pipe, JavaScript hands a message string to
//! [`LspSession::send`] and gets back a JSON array of the messages the server
//! wants to send.
//!
//! See `web/api/CONTRACT.md` for the contract the web IDE relies on.

use dpm_nbe_lsp::analysis;
use dpm_nbe_lsp::{with_big_stack, Server};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn error(message: &str);
}

thread_local! {
    /// Set by the panic hook, read by [`last_panic`] after JS catches the trap.
    static LAST_PANIC: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// A live language-server instance.
///
/// The web IDE keeps one of these and drives it exactly like an editor drives a
/// real language server: `initialize`, `textDocument/didOpen`,
/// `textDocument/didChange`, `textDocument/hover`, …  Every message can also
/// carry its own `text`, which makes any single request self-contained.
#[wasm_bindgen(js_name = LspSession)]
pub struct LspSession {
    server: Server,
}

#[wasm_bindgen(js_class = LspSession)]
impl LspSession {
    #[wasm_bindgen(constructor)]
    pub fn new() -> LspSession {
        LspSession {
            server: Server::new(),
        }
    }

    /// Handle one JSON-RPC message; returns a JSON array of the messages the
    /// server wants to send back (responses and `publishDiagnostics`).
    pub fn send(&mut self, message: &str) -> String {
        let outgoing = self.server.handle_message_values(message);
        serde_json::to_string(&outgoing).unwrap_or_else(|_| "[]".to_string())
    }

    /// Whether the server has seen `initialize`.
    #[wasm_bindgen(getter, js_name = initialized)]
    pub fn initialized(&self) -> bool {
        self.server.is_initialized()
    }
}

impl Default for LspSession {
    fn default() -> Self {
        Self::new()
    }
}

/// Install the panic recorder and say hello on the console.
///
/// A panic on `wasm32-unknown-unknown` traps the call, and the JS side only
/// sees a bare `RuntimeError: unreachable`.  The hook still runs before the
/// trap, so it records what and where for [`last_panic`] to hand back.
#[wasm_bindgen(start)]
pub fn main() {
    std::panic::set_hook(Box::new(|info| {
        let payload = info.payload();
        let message = payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("unknown panic");
        let text = match info.location() {
            Some(at) => {
                format!("panicked at {}:{}:{}: {message}", at.file(), at.line(), at.column())
            }
            None => format!("panicked: {message}"),
        };
        LAST_PANIC.with(|slot| *slot.borrow_mut() = Some(text.clone()));
        error(&format!("dpm-nbe-wasm {text}"));
    }));
}

/// The last Rust panic the module took ("" if none), as
/// `panicked at <file>:<line>:<column>: <message>`.  The trapped *call* is
/// dead, but the instance stays answerable, so the page can call this after
/// catching the trap and show the user what actually went wrong.
#[wasm_bindgen(js_name = lastPanic)]
pub fn last_panic() -> String {
    LAST_PANIC.with(|slot| slot.borrow().clone().unwrap_or_default())
}

/// Server identity and the LSP features it implements, as JSON.  The web IDE
/// uses it to decide what to enable; the smoke test asserts against it.
#[wasm_bindgen]
pub fn info() -> String {
    serde_json::json!({
        "name": dpm_nbe_lsp::SERVER_NAME,
        "version": dpm_nbe_lsp::VERSION,
        "language": { "id": "dpm-nbe", "extensions": [".dpm"] },
        "methods": [
            "initialize",
            "textDocument/didOpen",
            "textDocument/didChange",
            "textDocument/didSave",
            "textDocument/didClose",
            "textDocument/hover",
            "textDocument/completion",
            "textDocument/definition",
            "textDocument/documentSymbol",
            "dpm-nbe/evaluate",
            "dpm-nbe/tokens",
            "dpm-nbe/diagnostics",
            "shutdown",
            "exit"
        ]
    })
    .to_string()
}

/// Convenience wrapper for the "run" button: parse, check and normalize `src`
/// exactly like `dpm-nbe nf` / `dpm-nbe type` do.
///
/// `mode` is `"nf"` (normal form and type) or `"type"`.
#[wasm_bindgen]
pub fn check(src: &str, mode: &str) -> String {
    let owned = src.to_string();
    let requested = mode.to_string();
    let for_job = requested.clone();
    let result = with_big_stack(move || analysis::evaluate(&owned, "<web>", &for_job));
    match result {
        Ok(r) => {
            let mut out = serde_json::json!({ "ok": r.ok, "mode": r.mode });
            if r.ok {
                out["output"] = serde_json::json!(r.output);
            } else {
                out["error"] = serde_json::json!(r.error.unwrap_or_default());
            }
            out.to_string()
        }
        Err(message) => {
            serde_json::json!({ "ok": false, "mode": requested, "error": message }).to_string()
        }
    }
}
