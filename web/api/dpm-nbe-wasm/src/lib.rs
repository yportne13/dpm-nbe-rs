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
use wasm_bindgen::prelude::*;

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
