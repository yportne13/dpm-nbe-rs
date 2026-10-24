//! End-to-end tests for the `dpm-nbe` language server.
//!
//! Three layers are exercised here:
//!
//! * the analysis engine on the source of the README (tokens, scopes, hover,
//!   completion, definition, symbols),
//! * the protocol layer over JSON-RPC strings,
//! * the real `dpm-nbe-lsp` process over a pipe, framed with `Content-Length`.

use dpm_nbe_lsp::analysis::{self, BinderKind, LineIndex, TokKind};
use dpm_nbe_lsp::protocol::Server;
use dpm_nbe_lsp::stdio::{read_frame, write_frame};
use serde_json::{json, Value};
use std::io::{BufReader, Cursor};
use std::process::{Command, Stdio};

const README: &str = "\
data Nat : U where zero : Nat; suc : (n : Nat) -> Nat;
let plus : Nat -> Nat -> Nat =
  λ m n . elim Nat (λ _ . Nat) n (λ _ r . suc r) m;
let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> Id A a c =
  λ A a b c refl refl . refl;
let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero =
  λ m n refl refl . refl;
let C : (A : U) -> Id Nat (suc zero) zero -> A =
  λ A (!);
U
";

/// The arithmetic prelude: a `data Nat` declaration and a `plus` defined
/// through its eliminator, for snippets that use arithmetic.
const PRELUDE: &str = concat!(
    "data Nat : U where zero : Nat; suc : (n : Nat) -> Nat;\n",
    "let plus : Nat -> Nat -> Nat = λ m n . elim Nat (λ _ . Nat) n (λ _ r . suc r) m;\n"
);

/// Byte offset of the first occurrence of `needle`.
fn at(src: &str, needle: &str) -> usize {
    src.find(needle).unwrap_or_else(|| panic!("{needle:?} not found"))
}

/// `(line, character)` of the first occurrence of `needle`.
fn pos_of(src: &str, needle: &str) -> (usize, usize) {
    LineIndex::new(src).position(src, at(src, needle))
}

fn request(id: i64, method: &str, params: Value) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }).to_string()
}

fn notification(method: &str, params: Value) -> String {
    json!({ "jsonrpc": "2.0", "method": method, "params": params }).to_string()
}

/// Send one message and parse the single reply.
fn ask(server: &mut Server, method: &str, params: Value) -> Value {
    let out = server.handle_message(&request(1, method, params));
    assert_eq!(out.len(), 1, "expected exactly one reply to {method}");
    serde_json::from_str(&out[0]).expect("a JSON reply")
}

fn open(server: &mut Server, uri: &str, text: &str) -> Vec<Value> {
    server
        .handle_message(&notification(
            "textDocument/didOpen",
            json!({ "textDocument": { "uri": uri, "languageId": "dpm-nbe", "version": 1, "text": text } }),
        ))
        .iter()
        .map(|m| serde_json::from_str(m).expect("a JSON message"))
        .collect()
}

// ---------------------------------------------------------------------------
// Lexer and coordinates
// ---------------------------------------------------------------------------

#[test]
fn lexer_covers_the_source_exactly() {
    for src in [README, "let x' : Nat = zero; -- tail\n{- block -} x'", "λ → (!) _ \\"] {
        let toks = analysis::lex(src);
        let rebuilt: String = toks.iter().map(|t| t.text(src)).collect();
        assert_eq!(rebuilt, src, "tokens must tile the input");
        // And they must be ordered and non-overlapping.
        let mut last = 0;
        for t in &toks {
            assert!(t.start >= last, "tokens must not overlap");
            assert!(t.end > t.start, "tokens must not be empty");
            last = t.end;
        }
    }
}

#[test]
fn lexer_classifies_identifiers_keywords_comments_and_punctuation() {
    let src = "data Nat : U where zero : Nat; -- done\n{- b -} x' (!) -> → λ \\ _";
    let kinds: Vec<(&str, TokKind)> = analysis::lex(src)
        .iter()
        .filter(|t| !t.is_trivia())
        .map(|t| (t.text(src), t.kind))
        .collect();
    assert_eq!(kinds[0], ("data", TokKind::Keyword));
    assert_eq!(kinds[1], ("Nat", TokKind::Ident));
    assert!(kinds.contains(&("where", TokKind::Keyword)));
    assert!(kinds.contains(&("(!)", TokKind::Punct)));
    assert!(kinds.contains(&("->", TokKind::Punct)));
    assert!(kinds.contains(&("→", TokKind::Punct)));
    assert!(kinds.contains(&("λ", TokKind::Keyword)));

    // `--> x` is still a line comment, and a block comment does not nest.
    let toks = analysis::lex("--> x");
    assert_eq!(toks.len(), 1);
    assert_eq!(toks[0].kind, TokKind::Comment);

    // An unterminated block comment runs to the end of the input.
    let toks = analysis::lex("U {- oops");
    assert_eq!(toks.last().unwrap().kind, TokKind::Comment);
    assert_eq!(toks.last().unwrap().end, "U {- oops".len());
}

#[test]
fn lexer_handles_unicode_offsets() {
    let src = "λ→"; // both are single UTF-16 code units, 2 and 3 bytes
    assert_eq!(analysis::lex(src).len(), 2);
    let index = LineIndex::new(src);
    assert_eq!(index.position(src, 0), (0, 0));
    assert_eq!(index.position(src, "λ".len()), (0, 1));
    assert_eq!(index.position(src, src.len()), (0, 2));
    assert_eq!(index.offset(src, 0, 1), "λ".len());
}

#[test]
fn line_index_round_trips_with_crlf_and_multibyte_characters() {
    let src = "a\r\n𝔸b\nλ\n";
    let index = LineIndex::new(src);
    assert_eq!(index.line_count(), 4);
    // Every position *inside a line's content* must round trip exactly;
    // positions inside a line terminator are not valid LSP positions.
    for line in 0..index.line_count() {
        let (start, end) = index.line_range(src, line);
        for offset in start..=end {
            if !src.is_char_boundary(offset) {
                continue;
            }
            let (l, character) = index.position(src, offset);
            assert_eq!(
                index.offset(src, l, character),
                offset,
                "round trip failed at {offset} (line {line})"
            );
        }
    }
    // The astral character counts as two UTF-16 units.
    let (line, character) = index.position(src, at(src, "𝔸b") + "𝔸".len());
    assert_eq!((line, character), (1, 2));
}

// ---------------------------------------------------------------------------
// Binders, hover, completion, definition, symbols
// ---------------------------------------------------------------------------

#[test]
fn scopes_of_the_readme_program() {
    let bs = analysis::binders(README);
    let lets: Vec<&str> = bs
        .iter()
        .filter(|b| b.kind == BinderKind::Let)
        .map(|b| b.name.as_str())
        .collect();
    assert_eq!(lets, ["plus", "trans", "H", "C"]);

    let pis: Vec<&str> = bs
        .iter()
        .filter(|b| b.kind == BinderKind::Pi)
        .map(|b| b.name.as_str())
        .collect();
    // the leading `n` is the binder of the `suc` constructor's type
    assert_eq!(pis, ["n", "A", "a", "b", "c", "m", "n", "A"]);

    // The declaration contributes the type and its constructors.
    let datas: Vec<&str> = bs
        .iter()
        .filter(|b| b.kind == BinderKind::Data)
        .map(|b| b.name.as_str())
        .collect();
    assert_eq!(datas, ["Nat"]);
    let ctors: Vec<&str> = bs
        .iter()
        .filter(|b| b.kind == BinderKind::Ctor)
        .map(|b| b.name.as_str())
        .collect();
    assert_eq!(ctors, ["zero", "suc"]);

    // λ patterns of the first definition.
    let lams: Vec<&str> = bs
        .iter()
        .filter(|b| b.kind == BinderKind::Lambda)
        .map(|b| b.name.as_str())
        .collect();
    assert_eq!(lams, ["m", "n", "_", "_", "r", "A", "a", "b", "c", "m", "n", "A"]);

    // A `let` binding is not in scope inside its own type or value, but is in
    // its body.
    let trans = bs.iter().find(|b| b.name == "trans").unwrap();
    assert!(trans.covers(at(README, "let H")));
    assert!(!trans.covers(at(README, "Id A a b")));

    // A Pi binder is in scope in the codomain and in later binder groups.
    let a = bs
        .iter()
        .find(|b| b.name == "a" && b.kind == BinderKind::Pi)
        .unwrap();
    assert_eq!(a.type_text.as_deref(), Some("A"));
    assert!(a.covers(at(README, "Id A a b")));
}

#[test]
fn symbols_nest_the_way_the_syntax_does() {
    let syms = analysis::symbols(README);
    assert_eq!(syms.len(), 1);
    assert_eq!(syms[0].name, "plus");
    assert!(syms[0].detail.as_deref().unwrap().starts_with("Nat"));
    assert_eq!(syms[0].children.len(), 1);
    assert_eq!(syms[0].children[0].name, "trans");
    assert_eq!(syms[0].children[0].children[0].name, "H");
    assert_eq!(syms[0].children[0].children[0].children[0].name, "C");
    // `range` encloses `selectionRange`.
    for s in [&syms[0], &syms[0].children[0]] {
        assert!(s.span.0 <= s.selection.0 && s.selection.1 <= s.span.1);
    }
}

#[test]
fn hover_reports_the_type_of_a_bound_name() {
    let (line, character) = pos_of(README, "Id A a b");
    let character = character + "Id A ".len();
    let info = analysis::hover(README, line, character).expect("a hover");
    assert!(info.markdown.contains("a : A"), "{}", info.markdown);
    assert!(info.markdown.contains("function-type binder"));

    // A λ pattern has no written type.
    let (line, character) = pos_of(README, "λ A a b c refl refl . refl");
    let info = analysis::hover(README, line, character + 2).expect("a hover");
    assert!(info.markdown.contains("λ-pattern"), "{}", info.markdown);
}

#[test]
fn hover_documents_the_vocabulary() {
    // `plus` is an ordinary let-bound definition now, not vocabulary.
    let (line, character) = pos_of(README, "plus m zero");
    let info = analysis::hover(README, line, character).expect("a hover on plus");
    assert!(info.markdown.contains("plus : Nat"), "{}", info.markdown);

    let (line, character) = pos_of(README, "elim Nat");
    let info = analysis::hover(README, line, character).expect("a hover on elim");
    assert!(info.markdown.contains("eliminator"), "{}", info.markdown);

    let (line, character) = pos_of(README, "(!)");
    let info = analysis::hover(README, line, character).expect("a hover on (!)");
    assert!(info.markdown.contains("absurd pattern"), "{}", info.markdown);
}

#[test]
fn completion_offers_keywords_and_names_in_scope() {
    let src = "let x : Nat = zero;\n";
    let offset = src.len();
    let (line, character) = LineIndex::new(src).position(src, offset);
    let items = analysis::completions(src, line, character);
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"x"), "{labels:?}");
    assert!(labels.contains(&"data"));
    assert!(labels.contains(&"elim"));
    assert!(labels.contains(&"(!)"), "{labels:?}");
    let x = items.iter().find(|i| i.label == "x").unwrap();
    assert_eq!(x.detail.as_deref(), Some("Nat"));
    assert_eq!(x.kind, "variable");

    // Not yet defined: the lambda pattern is in scope only inside the body.
    let src2 = "data Nat : U where zero : Nat; let f : Nat -> Nat = λ n . zero; f zero";
    let inside = at(src2, ". zero; f") + 2;
    let (l, c) = LineIndex::new(src2).position(src2, inside);
    let items = analysis::completions(src2, l, c);
    assert!(items.iter().any(|i| i.label == "n"));
    assert!(items.iter().all(|i| i.label != "f"));
    assert!(items.iter().any(|i| i.label == "zero" && i.kind == "variable"));
}

#[test]
fn definition_points_at_the_binding_site() {
    let (line, character) = pos_of(README, "Id A a b");
    let character = character + "Id A ".len();
    let def = analysis::definition(README, line, character).expect("a definition");
    let name = &README[def.0..def.1];
    assert_eq!(name, "a");
    // The definition is the binder inside `(a b c : A)`.
    assert!(def.0 < at(README, "Id A a b"));

    // A use of a `let`-bound name points at the `let` name.
    let (line, character) = pos_of(README, "Id Nat n (plus m zero)");
    let character = character + "Id Nat ".len();
    let def = analysis::definition(README, line, character).expect("a definition");
    assert_eq!(&README[def.0..def.1], "n");

    // A constructor resolves to its declaration; keywords and syntax do not.
    let (line, character) = pos_of(README, "Nat (suc zero)");
    let character = character + "Nat (".len();
    let def = analysis::definition(README, line, character).expect("a definition");
    assert_eq!(&README[def.0..def.1], "suc");
    let (line, character) = pos_of(README, "(!)");
    assert!(analysis::definition(README, line, character).is_none());
}

// ---------------------------------------------------------------------------
// Diagnostics
// ---------------------------------------------------------------------------

#[test]
fn good_programs_produce_no_diagnostics() {
    assert!(analysis::diagnostics(README, "file:///a.dpm").is_empty());
    assert!(analysis::diagnostics("U", "file:///a.dpm").is_empty());
    assert!(analysis::diagnostics(
        &format!("{PRELUDE}plus (suc zero) (suc zero)"),
        "file:///a.dpm"
    )
    .is_empty());
}

#[test]
fn type_errors_are_reported_with_a_range() {
    let src = format!("{PRELUDE}let bad : Nat = U; U");
    let ds = analysis::diagnostics(&src, "file:///a.dpm");
    assert_eq!(ds.len(), 1);
    assert_eq!(ds[0].code, "type-error");
    assert_eq!(ds[0].severity, analysis::Severity::Error);
    assert!(ds[0].message.contains("type mismatch"), "{}", ds[0].message);
    // The CLI points at the `U` at 1-based column 17 of the third line.
    let index = LineIndex::new(&src);
    assert_eq!(index.position(&src, ds[0].span.0), (2, 16));
    assert!(ds[0].rendered.as_deref().unwrap().contains("^"));
}

#[test]
fn parse_errors_are_reported_as_such() {
    let ds = analysis::diagnostics("elim", "file:///a.dpm");
    assert_eq!(ds.len(), 1);
    assert_eq!(ds[0].code, "parse-error");
    assert!(!ds[0].message.is_empty());

    let ds = analysis::diagnostics("U {- oops", "file:///a.dpm");
    assert_eq!(ds[0].code, "parse-error");
    assert!(ds[0].message.contains("unterminated"), "{}", ds[0].message);
}

#[test]
fn evaluation_matches_the_command_line() {
    let r = analysis::evaluate(&format!("{PRELUDE}plus (suc zero) (suc zero)"), "a.dpm", "nf");
    assert!(r.ok);
    assert_eq!(r.output, "suc (suc zero)\n  :\nNat");

    let r = analysis::evaluate("refl", "a.dpm", "nf");
    assert!(!r.ok);
    assert!(r.error.unwrap().contains("Can't infer type for refl"));
    assert!(r.error_span.is_some());

    let r = analysis::evaluate("(A : U) -> A -> A", "a.dpm", "type");
    assert_eq!(r.output, "U1");
}

// ---------------------------------------------------------------------------
// Protocol
// ---------------------------------------------------------------------------

#[test]
fn initialize_advertises_the_implemented_features() {
    let mut server = Server::new();
    let reply = ask(&mut server, "initialize", json!({ "processId": null }));
    let caps = &reply["result"]["capabilities"];
    assert_eq!(reply["result"]["serverInfo"]["name"], "dpm-nbe-lsp");
    assert_eq!(caps["textDocumentSync"]["change"], 1);
    assert_eq!(caps["textDocumentSync"]["openClose"], true);
    assert_eq!(caps["hoverProvider"], true);
    assert_eq!(caps["definitionProvider"], true);
    assert_eq!(caps["documentSymbolProvider"], true);
    assert!(caps["completionProvider"].is_object());
    assert!(server.is_initialized());
}

#[test]
fn unknown_methods_get_a_json_rpc_error() {
    let mut server = Server::new();
    let reply = ask(&mut server, "textDocument/typeDefinition", json!({}));
    assert_eq!(reply["error"]["code"], -32601);
    assert!(reply["error"]["message"]
        .as_str()
        .unwrap()
        .contains("typeDefinition"));
}

#[test]
fn malformed_input_does_not_panic() {
    let mut server = Server::new();
    let out = server.handle_message("{not json");
    assert_eq!(out.len(), 1);
    let reply: Value = serde_json::from_str(&out[0]).unwrap();
    assert_eq!(reply["error"]["code"], -32700);

    // A notification for an unknown method is silently ignored.
    assert!(server
        .handle_message(&notification("workspace/didChangeConfiguration", json!({})))
        .is_empty());
}

#[test]
fn diagnostics_are_pushed_on_open_and_cleared_on_close() {
    let mut server = Server::new();
    let uri = "file:///bad.dpm";
    let pushed = open(&mut server, uri, &format!("{PRELUDE}let bad : Nat = U; U"));
    assert_eq!(pushed.len(), 1);
    assert_eq!(pushed[0]["method"], "textDocument/publishDiagnostics");
    assert_eq!(pushed[0]["params"]["uri"], uri);
    assert_eq!(pushed[0]["params"]["version"], 1);
    let diag = &pushed[0]["params"]["diagnostics"][0];
    assert_eq!(diag["severity"], 1);
    assert_eq!(diag["source"], "dpm-nbe");
    assert_eq!(diag["range"]["start"]["line"], 2);
    assert_eq!(diag["range"]["start"]["character"], 16);

    let cleared = server.handle_message(&notification(
        "textDocument/didClose",
        json!({ "textDocument": { "uri": uri } }),
    ));
    let cleared: Value = serde_json::from_str(&cleared[0]).unwrap();
    assert_eq!(cleared["params"]["diagnostics"].as_array().unwrap().len(), 0);
    assert!(server.document(uri).is_none());
}

#[test]
fn did_change_accepts_full_and_ranged_updates() {
    let mut server = Server::new();
    let uri = "file:///a.dpm";
    open(&mut server, uri, "U");
    assert!(analysis::diagnostics(server.document(uri).unwrap(), uri).is_empty());

    // Full sync: the text is replaced wholesale.
    server.handle_message(&notification(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": uri, "version": 2 },
            "contentChanges": [ { "text": "refl" } ]
        }),
    ));
    assert_eq!(server.document(uri), Some("refl"));

    // Ranged sync, defensively (the server advertises Full synchronisation).
    let pushed = server.handle_message(&notification(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": uri, "version": 3 },
            "contentChanges": [ {
                "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 4 } },
                "text": "U"
            } ]
        }),
    ));
    assert_eq!(server.document(uri), Some("U"));
    let pushed: Value = serde_json::from_str(&pushed[0]).unwrap();
    assert_eq!(pushed["params"]["diagnostics"].as_array().unwrap().len(), 0);
    assert_eq!(pushed["params"]["version"], 3);
}

#[test]
fn hover_completion_definition_and_symbols_over_the_protocol() {
    let mut server = Server::new();
    let uri = "file:///readme.dpm";
    open(&mut server, uri, README);

    let (line, character) = pos_of(README, "Id A a b");
    let reply = ask(
        &mut server,
        "textDocument/hover",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character + "Id A ".len() }
        }),
    );
    let value = reply["result"]["contents"]["value"].as_str().unwrap();
    assert!(value.contains("a : A"), "{value}");

    let (line, character) = pos_of(README, "let H");
    let reply = ask(
        &mut server,
        "textDocument/completion",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character + 4 }
        }),
    );
    let items = reply["result"]["items"].as_array().unwrap();
    assert!(items.iter().any(|i| i["label"] == "trans"));
    assert!(items.iter().any(|i| i["label"] == "Nat" && i["kind"] == 6));
    assert!(items.iter().any(|i| i["label"] == "data" && i["kind"] == 14));
    assert!(items.iter().all(|i| i["label"] != "H"));

    let (line, character) = pos_of(README, "Id Nat n (plus m zero)");
    let reply = ask(
        &mut server,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character + "Id Nat ".len() }
        }),
    );
    let result = &reply["result"];
    assert_eq!(result["uri"], uri);
    let target = LineIndex::new(README)
        .offset(README, result["range"]["start"]["line"].as_u64().unwrap() as usize,
                result["range"]["start"]["character"].as_u64().unwrap() as usize);
    assert_eq!(&README[target..target + 1], "n");

    let reply = ask(
        &mut server,
        "textDocument/documentSymbol",
        json!({ "textDocument": { "uri": uri } }),
    );
    let syms = reply["result"].as_array().unwrap();
    assert_eq!(syms.len(), 1);
    assert_eq!(syms[0]["name"], "plus");
    assert_eq!(syms[0]["children"][0]["name"], "trans");
    assert_eq!(syms[0]["children"][0]["children"][0]["name"], "H");
}

#[test]
fn custom_requests_serve_the_web_ide() {
    let mut server = Server::new();
    let uri = "file:///a.dpm";
    open(&mut server, uri, &format!("{PRELUDE}plus (suc zero) (suc zero)"));

    let reply = ask(
        &mut server,
        "dpm-nbe/evaluate",
        json!({ "textDocument": { "uri": uri }, "mode": "nf" }),
    );
    assert_eq!(reply["result"]["ok"], true);
    assert_eq!(reply["result"]["output"], "suc (suc zero)\n  :\nNat");

    let reply = ask(
        &mut server,
        "dpm-nbe/evaluate",
        json!({ "textDocument": { "uri": uri }, "mode": "type" }),
    );
    assert_eq!(reply["result"]["output"], "Nat");

    let reply = ask(
        &mut server,
        "dpm-nbe/evaluate",
        json!({ "text": "refl", "mode": "nf" }),
    );
    assert_eq!(reply["result"]["ok"], false);
    assert!(reply["result"]["error"]
        .as_str()
        .unwrap()
        .contains("Can't infer type for refl"));

    let reply = ask(&mut server, "dpm-nbe/tokens", json!({ "text": "λ x . x" }));
    let tokens = reply["result"]["tokens"].as_array().unwrap();
    assert_eq!(tokens[0]["type"], "keyword");
    assert_eq!(tokens[0]["range"]["start"]["character"], 0);
    assert_eq!(tokens[1]["type"], "ident");

    let reply = ask(
        &mut server,
        "dpm-nbe/diagnostics",
        json!({ "textDocument": { "uri": uri } }),
    );
    assert_eq!(reply["result"].as_array().unwrap().len(), 0);
}

#[test]
fn a_panicking_input_is_reported_instead_of_killing_the_server() {
    // Deeply nested input is bounded by the parser's depth budget; this checks
    // the error path is a diagnostic and the server stays alive afterwards.
    let mut server = Server::new();
    let uri = "file:///deep.dpm";
    let deep = format!("{}U", "(".repeat(2000));
    let pushed = open(&mut server, uri, &deep);
    let diags = pushed[0]["params"]["diagnostics"].as_array().unwrap();
    assert!(!diags.is_empty());
    // Still responsive.
    let reply = ask(
        &mut server,
        "textDocument/documentSymbol",
        json!({ "textDocument": { "uri": uri } }),
    );
    assert_eq!(reply["result"].as_array().unwrap().len(), 0);
}

// ---------------------------------------------------------------------------
// Framing and the real binary
// ---------------------------------------------------------------------------

#[test]
fn content_length_framing_round_trips() {
    let mut buffer: Vec<u8> = Vec::new();
    write_frame(&mut buffer, b"{\"a\":1}").unwrap();
    write_frame(&mut buffer, b"{}").unwrap();
    let mut cursor = Cursor::new(buffer);
    let first = read_frame(&mut cursor).unwrap().unwrap();
    let second = read_frame(&mut cursor).unwrap().unwrap();
    assert_eq!(first, b"{\"a\":1}");
    assert_eq!(second, b"{}");
    assert!(read_frame(&mut cursor).unwrap().is_none());
}

#[test]
fn the_binary_speaks_lsp_over_a_pipe() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_dpm-nbe-lsp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the server binary starts");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        let messages = [
            request(1, "initialize", json!({ "processId": null })),
            notification(
                "textDocument/didOpen",
                json!({
                    "textDocument": {
                        "uri": "file:///a.dpm",
                        "languageId": "dpm-nbe",
                        "version": 1,
                        "text": format!("{PRELUDE}let bad : Nat = U; U")
                    }
                }),
            ),
            request(2, "dpm-nbe/evaluate", json!({ "text": format!("{PRELUDE}plus zero zero"), "mode": "nf" })),
            request(3, "shutdown", json!(null)),
            notification("exit", json!(null)),
        ];
        for m in messages {
            write_frame(stdin, m.as_bytes()).unwrap();
        }
    }
    let output = child.wait_with_output().expect("the server exits");
    assert!(
        output.status.success(),
        "exit status {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let mut reader = BufReader::new(Cursor::new(output.stdout));
    let mut replies: Vec<Value> = Vec::new();
    while let Some(frame) = read_frame(&mut reader).unwrap() {
        replies.push(serde_json::from_slice(&frame).expect("a JSON reply"));
    }
    assert_eq!(replies.len(), 4, "{replies:#?}");
    assert_eq!(replies[0]["id"], 1);
    assert!(replies[0]["result"]["capabilities"].is_object());
    assert_eq!(replies[1]["method"], "textDocument/publishDiagnostics");
    assert!(replies[1]["params"]["diagnostics"][0]["message"]
        .as_str()
        .unwrap()
        .contains("type mismatch"));
    assert_eq!(replies[2]["id"], 2);
    assert_eq!(replies[2]["result"]["output"], "zero\n  :\nNat");
    assert_eq!(replies[3]["id"], 3);
}

#[test]
fn the_binary_prints_help_and_version() {
    for (args, expect) in [
        (vec!["--version"], "dpm-nbe-lsp"),
        (vec!["--help"], "usage: dpm-nbe-lsp"),
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_dpm-nbe-lsp"))
            .args(&args)
            .output()
            .expect("the server binary runs");
        assert!(out.status.success());
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(text.contains(expect), "{text}");
    }
}
