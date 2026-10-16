//! JSON-RPC / Language Server Protocol handling.
//!
//! The whole protocol is deliberately transport agnostic: [`Server::handle_message`]
//! takes one message as a string and returns the messages to send back, as
//! strings.  The native server feeds it from `stdin`/`stdout`; the browser build
//! feeds it from JavaScript.  That is why the crate carries no `tower-lsp` /
//! `lsp-server` dependency: those wrap a socket or a pipe, and this one has to
//! work without either.

use crate::analysis::{self, Diagnostic, DocumentSymbol, LineIndex, Severity, Span};
use crate::{with_big_stack, SERVER_NAME, VERSION};
use serde_json::{json, Value};
use std::collections::HashMap;

/// JSON-RPC error codes used here.
const PARSE_ERROR: i64 = -32700;
const METHOD_NOT_FOUND: i64 = -32601;
const INTERNAL_ERROR: i64 = -32603;

/// One language-server instance, holding the open documents.
#[derive(Debug, Default)]
pub struct Server {
    docs: HashMap<String, String>,
    versions: HashMap<String, i64>,
    initialized: bool,
    shutdown: bool,
    exit: bool,
}

impl Server {
    pub fn new() -> Server {
        Server::default()
    }

    /// The server should stop its loop (after `exit`).
    pub fn should_exit(&self) -> bool {
        self.exit
    }

    /// Whether `initialize` has been seen.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Whether `shutdown` has been seen.
    pub fn is_shutdown(&self) -> bool {
        self.shutdown
    }

    pub fn document(&self, uri: &str) -> Option<&str> {
        self.docs.get(uri).map(String::as_str)
    }

    /// Handle one wire message; returns the messages to send back, in order.
    /// Malformed input never panics — it produces a JSON-RPC error.
    pub fn handle_message(&mut self, msg: &str) -> Vec<String> {
        self.handle_message_values(msg)
            .into_iter()
            .map(stringify)
            .collect()
    }

    /// Like [`Server::handle_message`], but returning parsed JSON so that a
    /// binding layer (the browser module) can re-serialize a proper array.
    pub fn handle_message_values(&mut self, msg: &str) -> Vec<Value> {
        let value: Value = match serde_json::from_str(msg) {
            Ok(v) => v,
            Err(e) => {
                return vec![error_response(
                    Value::Null,
                    PARSE_ERROR,
                    &format!("invalid JSON: {e}"),
                )]
            }
        };
        self.handle(&value)
    }

    /// Same as [`Server::handle_message`] but on an already-parsed value.
    pub fn handle(&mut self, msg: &Value) -> Vec<Value> {
        let method = match msg.get("method").and_then(Value::as_str) {
            Some(m) => m.to_string(),
            // A response to a request this server never sends: ignore it.
            None => return Vec::new(),
        };
        let id = msg.get("id").cloned();
        let params = msg.get("params").cloned().unwrap_or(Value::Null);
        match id {
            Some(id) if !id.is_null() => match self.request(&method, &params) {
                Ok(result) => vec![success_response(id, result)],
                Err((code, message)) => vec![error_response(id, code, &message)],
            },
            // Notifications can still push messages (diagnostics).
            _ => self.notification(&method, &params),
        }
    }

    fn request(&mut self, method: &str, params: &Value) -> Result<Value, (i64, String)> {
        match method {
            "initialize" => {
                self.initialized = true;
                Ok(json!({
                    "capabilities": {
                        "textDocumentSync": {
                            "openClose": true,
                            // 1 = Full.  Ranged changes are also understood,
                            // defensively, in case a client ignores this.
                            "change": 1,
                            "save": { "includeText": true }
                        },
                        "hoverProvider": true,
                        "completionProvider": { "resolveProvider": false },
                        "definitionProvider": true,
                        "documentSymbolProvider": true
                    },
                    "serverInfo": { "name": SERVER_NAME, "version": VERSION }
                }))
            }
            "shutdown" => {
                self.shutdown = true;
                Ok(Value::Null)
            }
            "textDocument/hover" => self.hover(params),
            "textDocument/completion" => self.completion(params),
            "textDocument/definition" => self.definition(params),
            "textDocument/documentSymbol" => self.document_symbols(params),
            "dpm-nbe/evaluate" => self.evaluate(params),
            "dpm-nbe/tokens" => Ok(self.tokens(params)),
            "dpm-nbe/diagnostics" => Ok(Value::Array(self.diagnostics_for(params))),
            other => Err((METHOD_NOT_FOUND, format!("method not found: {other}"))),
        }
    }

    fn notification(&mut self, method: &str, params: &Value) -> Vec<Value> {
        match method {
            "initialized" => Vec::new(),
            "exit" => {
                self.exit = true;
                Vec::new()
            }
            "textDocument/didOpen" => {
                let (uri, text, version) = read_doc(params);
                match uri {
                    Some(uri) => {
                        self.versions.insert(uri.clone(), version.unwrap_or(0));
                        self.docs.insert(uri.clone(), text.unwrap_or_default());
                        self.publish(&uri)
                    }
                    None => Vec::new(),
                }
            }
            "textDocument/didChange" => {
                let (uri, _, version) = read_doc(params);
                let uri = match uri {
                    Some(u) => u,
                    None => return Vec::new(),
                };
                let mut text = self.docs.get(&uri).cloned().unwrap_or_default();
                if let Some(changes) = params.get("contentChanges").and_then(Value::as_array) {
                    for change in changes {
                        text = match change.get("range") {
                            Some(range) => apply_change(&text, range, change.get("text")),
                            None => change
                                .get("text")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string(),
                        };
                    }
                }
                self.docs.insert(uri.clone(), text);
                if let Some(v) = version {
                    self.versions.insert(uri.clone(), v);
                }
                self.publish(&uri)
            }
            "textDocument/didSave" => {
                let (uri, text, _) = read_doc(params);
                let uri = match uri {
                    Some(u) => u,
                    None => return Vec::new(),
                };
                if let Some(text) = text {
                    self.docs.insert(uri.clone(), text);
                }
                self.publish(&uri)
            }
            "textDocument/didClose" => {
                let uri = params
                    .get("textDocument")
                    .and_then(|d| d.get("uri"))
                    .and_then(Value::as_str)
                    .map(str::to_string);
                match uri {
                    Some(uri) => {
                        self.docs.remove(&uri);
                        self.versions.remove(&uri);
                        // Clear the client's squiggles.
                        vec![json!({
                            "jsonrpc": "2.0",
                            "method": "textDocument/publishDiagnostics",
                            "params": { "uri": uri, "diagnostics": [] }
                        })]
                    }
                    None => Vec::new(),
                }
            }
            _ => Vec::new(),
        }
    }

    // -- features ----------------------------------------------------------

    /// Recompute and push diagnostics for one document.
    pub fn publish(&self, uri: &str) -> Vec<Value> {
        match self.docs.get(uri) {
            Some(text) => vec![self.push_diagnostics(uri, text)],
            None => Vec::new(),
        }
    }

    fn push_diagnostics(&self, uri: &str, text: &str) -> Value {
        let messages = self.diagnostic_values(text, uri);
        let mut params = json!({ "uri": uri, "diagnostics": messages });
        if let Some(v) = self.versions.get(uri) {
            params["version"] = json!(v);
        }
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": params
        })
    }

    fn diagnostic_values(&self, text: &str, uri: &str) -> Vec<Value> {
        let index = LineIndex::new(text);
        let owned_text = text.to_string();
        let owned_uri = uri.to_string();
        // The checker recurses over the program, so it runs on a big stack and
        // a panic becomes a diagnostic instead of a dead server.
        let found = with_big_stack(move || analysis::diagnostics(&owned_text, &owned_uri));
        match found {
            Ok(diags) => diags
                .iter()
                .map(|d| diagnostic_json(text, &index, d))
                .collect(),
            Err(message) => vec![json!({
                "range": range_json(text, &index, (0, 0)),
                "severity": 1,
                "code": "internal-error",
                "source": "dpm-nbe",
                "message": message
            })],
        }
    }

    fn diagnostics_for(&self, params: &Value) -> Vec<Value> {
        let (text, uri) = self.text_and_uri(params);
        self.diagnostic_values(&text, &uri)
    }

    fn hover(&self, params: &Value) -> Result<Value, (i64, String)> {
        let (text, _) = self.text_and_uri(params);
        let (line, character) = position_of(params);
        let owned = text.clone();
        let found = with_big_stack(move || analysis::hover(&owned, line, character))
            .map_err(|m| (INTERNAL_ERROR, m))?;
        match found {
            Some(h) => {
                let index = LineIndex::new(&text);
                Ok(json!({
                    "contents": { "kind": "markdown", "value": h.markdown },
                    "range": range_json(&text, &index, h.span)
                }))
            }
            None => Ok(Value::Null),
        }
    }

    fn completion(&self, params: &Value) -> Result<Value, (i64, String)> {
        let (text, _) = self.text_and_uri(params);
        let (line, character) = position_of(params);
        let owned = text.clone();
        let found = with_big_stack(move || analysis::completions(&owned, line, character))
            .map_err(|m| (INTERNAL_ERROR, m))?;
        let items: Vec<Value> = found
            .into_iter()
            .map(|c| {
                // 6 = Variable, 14 = Keyword.
                let kind = if c.kind == "variable" { 6 } else { 14 };
                let mut item = json!({
                    "label": c.label,
                    "kind": kind,
                    "sortText": format!("{}{}", if kind == 6 { 0 } else { 1 }, c.label)
                });
                if let Some(d) = c.detail {
                    item["detail"] = json!(d);
                }
                if let Some(doc) = c.documentation {
                    item["documentation"] = json!({ "kind": "markdown", "value": doc });
                }
                item
            })
            .collect();
        Ok(json!({ "isIncomplete": false, "items": items }))
    }

    fn definition(&self, params: &Value) -> Result<Value, (i64, String)> {
        let (text, uri) = self.text_and_uri(params);
        let (line, character) = position_of(params);
        let owned = text.clone();
        let found = with_big_stack(move || analysis::definition(&owned, line, character))
            .map_err(|m| (INTERNAL_ERROR, m))?;
        match found {
            Some(span) => {
                let index = LineIndex::new(&text);
                Ok(json!({ "uri": uri, "range": range_json(&text, &index, span) }))
            }
            None => Ok(Value::Null),
        }
    }

    fn document_symbols(&self, params: &Value) -> Result<Value, (i64, String)> {
        let (text, _) = self.text_and_uri(params);
        let owned = text.clone();
        let found = with_big_stack(move || analysis::symbols(&owned))
            .map_err(|m| (INTERNAL_ERROR, m))?;
        let index = LineIndex::new(&text);
        Ok(Value::Array(
            found
                .iter()
                .map(|s| symbol_json(&text, &index, s))
                .collect(),
        ))
    }

    fn evaluate(&self, params: &Value) -> Result<Value, (i64, String)> {
        let (text, uri) = self.text_and_uri(params);
        let mode = params
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("nf")
            .to_string();
        let owned = text.clone();
        let owned_uri = uri.clone();
        let owned_mode = mode.clone();
        let result =
            with_big_stack(move || analysis::evaluate(&owned, &owned_uri, &owned_mode))
                .map_err(|m| (INTERNAL_ERROR, m))?;
        let index = LineIndex::new(&text);
        let mut out = json!({ "ok": result.ok, "mode": result.mode });
        if result.ok {
            out["output"] = json!(result.output);
        } else {
            out["error"] = json!(result.error.unwrap_or_default());
            if let Some(span) = result.error_span {
                out["range"] = range_json(&text, &index, span);
            }
        }
        Ok(out)
    }

    fn tokens(&self, params: &Value) -> Value {
        let (text, _) = self.text_and_uri(params);
        let index = LineIndex::new(&text);
        let toks: Vec<Value> = analysis::lex(&text)
            .iter()
            .filter(|t| t.kind != analysis::TokKind::Whitespace)
            .map(|t| {
                let kind = match t.kind {
                    analysis::TokKind::Ident => "ident",
                    analysis::TokKind::Keyword => "keyword",
                    analysis::TokKind::Punct => "punct",
                    analysis::TokKind::Comment => "comment",
                    analysis::TokKind::Whitespace => "whitespace",
                    analysis::TokKind::Unknown => "unknown",
                };
                json!({ "range": range_json(&text, &index, (t.start, t.end)), "type": kind })
            })
            .collect();
        json!({ "tokens": toks })
    }

    /// `params.text` when present, otherwise the open document named by
    /// `params.textDocument.uri`.
    fn text_and_uri(&self, params: &Value) -> (String, String) {
        let uri = params
            .get("textDocument")
            .and_then(|d| d.get("uri"))
            .and_then(Value::as_str)
            .or_else(|| params.get("uri").and_then(Value::as_str))
            .unwrap_or("untitled")
            .to_string();
        if let Some(text) = params.get("text").and_then(Value::as_str) {
            return (text.to_string(), uri);
        }
        let text = self.docs.get(&uri).cloned().unwrap_or_default();
        (text, uri)
    }
}

// ---------------------------------------------------------------------------
// Conversions
// ---------------------------------------------------------------------------

fn stringify(value: Value) -> String {
    // A `Value` built from owned data always serializes.
    serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string())
}

fn success_response(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn range_json(src: &str, index: &LineIndex, span: Span) -> Value {
    let (sl, sc) = index.position(src, span.0);
    let (el, ec) = index.position(src, span.1.max(span.0));
    json!({
        "start": { "line": sl, "character": sc },
        "end": { "line": el, "character": ec }
    })
}

fn diagnostic_json(text: &str, index: &LineIndex, d: &Diagnostic) -> Value {
    let severity = match d.severity {
        Severity::Error => 1,
        Severity::Warning => 2,
    };
    let mut out = json!({
        "range": range_json(text, index, d.span),
        "severity": severity,
        "code": d.code,
        "source": "dpm-nbe",
        "message": d.message
    });
    if let Some(rendered) = &d.rendered {
        // Ignored by editors that do not know it; the web UI shows it verbatim.
        out["data"] = json!({ "rendered": rendered });
    }
    out
}

fn symbol_json(text: &str, index: &LineIndex, s: &DocumentSymbol) -> Value {
    let children: Vec<Value> = s
        .children
        .iter()
        .map(|c| symbol_json(text, index, c))
        .collect();
    // 12 = Function; every definition in this language is a function or a value.
    let mut out = json!({
        "name": s.name,
        "kind": 12,
        "range": range_json(text, index, s.span),
        "selectionRange": range_json(text, index, s.selection)
    });
    if let Some(d) = &s.detail {
        out["detail"] = json!(d);
    }
    if !children.is_empty() {
        out["children"] = Value::Array(children);
    }
    out
}

fn read_doc(params: &Value) -> (Option<String>, Option<String>, Option<i64>) {
    let doc = params.get("textDocument");
    let uri = doc
        .and_then(|d| d.get("uri"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let text = doc
        .and_then(|d| d.get("text"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            params
                .get("text")
                .and_then(Value::as_str)
                .map(str::to_string)
        });
    let version = doc.and_then(|d| d.get("version")).and_then(Value::as_i64);
    (uri, text, version)
}

fn position_of(params: &Value) -> (usize, usize) {
    let pos = params.get("position");
    let line = pos
        .and_then(|p| p.get("line"))
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let character = pos
        .and_then(|p| p.get("character"))
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    (line, character)
}

/// Apply one `TextDocumentContentChangeEvent`.
fn apply_change(text: &str, range: &Value, replacement: Option<&Value>) -> String {
    let index = LineIndex::new(text);
    let start = range
        .get("start")
        .map(|p| point(text, &index, p))
        .unwrap_or(0)
        .min(text.len());
    let end = range
        .get("end")
        .map(|p| point(text, &index, p))
        .unwrap_or(0)
        .min(text.len());
    let (start, end) = if start <= end { (start, end) } else { (end, start) };
    let new = replacement.and_then(Value::as_str).unwrap_or_default();
    let mut out = String::with_capacity(text.len() + new.len());
    out.push_str(&text[..start]);
    out.push_str(new);
    out.push_str(&text[end..]);
    out
}

fn point(text: &str, index: &LineIndex, p: &Value) -> usize {
    let line = p.get("line").and_then(Value::as_u64).unwrap_or(0) as usize;
    let character = p.get("character").and_then(Value::as_u64).unwrap_or(0) as usize;
    index.offset(text, line, character)
}
