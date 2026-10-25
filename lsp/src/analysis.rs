//! Source analysis for the `dpm-nbe` language.
//!
//! Everything in this module is pure: it takes a source string and returns
//! plain data.  No I/O, no threads, no globals — which is what lets the very
//! same code run behind `stdin`/`stdout` and inside a browser.
//!
//! Two layers:
//!
//! * [`lex`] mirrors the lexer embedded in `dpm-nbe`'s recursive-descent parser
//!   (`src/parse.rs`): same identifier rule (alphanumerics plus `'`), same
//!   comment rules (`--` to end of line, non-nesting `{- -}`), same keyword set.
//! * [`binders`] runs a small, deliberately *permissive* scanner over the token
//!   stream that mirrors the surface grammar (`Raw ::= Lam | Let | Pi | Spine
//!   [-> Raw]`) only far enough to know where every name is bound and how far
//!   that binding reaches.  It never fails: on input the real parser would
//!   reject it just advances a token, so hover/completion keep working while the
//!   user is mid-edit.  Diagnostics come from the real parser/elaborator, so a
//!   wrong guess here can never turn into a false error.

use dpm_nbe::{parse, run_named, typecheck_named, Error, Mode};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Coordinates
// ---------------------------------------------------------------------------

/// Byte offsets of the start of every line, plus UTF-16 conversion.
///
/// Offsets inside this module are byte offsets (Rust strings are UTF-8), while
/// LSP positions are `(line, utf16 code unit)`; the two must never be mixed up,
/// which is why every conversion goes through this type.
#[derive(Clone, Debug)]
pub struct LineIndex {
    starts: Vec<usize>,
    len: usize,
}

impl LineIndex {
    pub fn new(src: &str) -> LineIndex {
        let mut starts = vec![0usize];
        for (i, b) in src.bytes().enumerate() {
            if b == b'\n' {
                starts.push(i + 1);
            }
        }
        LineIndex {
            starts,
            len: src.len(),
        }
    }

    /// Number of lines (a trailing newline opens a final empty line).
    pub fn line_count(&self) -> usize {
        self.starts.len()
    }

    /// Byte offset of the first character of `line` (0-based).
    pub fn line_start(&self, line: usize) -> usize {
        self.starts.get(line).copied().unwrap_or(self.len)
    }

    /// Byte range of `line` (0-based), excluding the line terminator.
    pub fn line_range(&self, src: &str, line: usize) -> (usize, usize) {
        let start = self.line_start(line).min(self.len);
        let end = if line + 1 < self.starts.len() {
            let next = self.starts[line + 1];
            let mut e = next.saturating_sub(1);
            if e > start && src.as_bytes().get(e - 1) == Some(&b'\r') {
                e -= 1;
            }
            e.min(self.len)
        } else {
            self.len
        };
        (start, end.max(start))
    }

    /// 0-based `(line, utf16 character)` of a byte offset.
    pub fn position(&self, src: &str, offset: usize) -> (usize, usize) {
        let offset = offset.min(self.len);
        let line = match self.starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };
        let start = self.starts[line];
        (line, utf16_len(&src[start..offset]))
    }

    /// Byte offset of a 0-based `(line, utf16 character)`, clamped to the line.
    pub fn offset(&self, src: &str, line: usize, character: usize) -> usize {
        let (start, end) = self.line_range(src, line);
        let mut units = 0usize;
        for (i, c) in src[start..end].char_indices() {
            if units >= character {
                return start + i;
            }
            units += c.len_utf16();
        }
        end
    }

    /// Length in UTF-16 code units of the character starting at `offset`.
    pub fn char_width(&self, src: &str, offset: usize) -> usize {
        src[offset.min(self.len)..]
            .chars()
            .next()
            .map(|c| c.len_utf16())
            .unwrap_or(0)
    }
}

fn utf16_len(s: &str) -> usize {
    s.chars().map(|c| c.len_utf16()).sum()
}

/// A half-open byte range.
pub type Span = (usize, usize);

// ---------------------------------------------------------------------------
// Lexer
// ---------------------------------------------------------------------------

/// What a token is, for highlighting and for the binder scanner.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TokKind {
    Ident,
    /// A reserved word, or a word the grammar gives special meaning to.
    Keyword,
    /// `(`, `)`, `:`, `=`, `;`, `.`, `\`, `_`, `(!)`, `->`, `→`
    Punct,
    Comment,
    Whitespace,
    /// A character the grammar has no rule for.
    Unknown,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Token {
    pub start: usize,
    pub end: usize,
    pub kind: TokKind,
}

impl Token {
    pub fn text<'a>(&self, src: &'a str) -> &'a str {
        &src[self.start..self.end]
    }

    pub fn is_trivia(&self) -> bool {
        matches!(self.kind, TokKind::Whitespace | TokKind::Comment)
    }
}

/// Words the grammar reserves (`is_keyword` in `dpm-nbe`'s `src/parse.rs`).
pub const KEYWORDS: &[&str] = &[
    "let", "in", "λ", "U", "refl", "Id", "data", "struct", "where", "elim",
];

/// Multi-character punctuation, checked before the single characters.
const LONG_PUNCT: &[&str] = &["(!)", "->", "→"];
/// Single-character punctuation.
const PUNCT: &[&str] = &["(", ")", ":", "=", ";", ".", "\\", "_"];

pub fn is_keyword(s: &str) -> bool {
    KEYWORDS.contains(&s)
}

/// A universe of the tower beyond level 0: `U` followed by digits only
/// (`U1`, `U2`, ...), the spelling `src/parse.rs` reads as `Raw::U(l)`.
/// Like the reserved words this is fixed vocabulary, so it lexes as a
/// keyword rather than as an identifier.
pub fn is_universe(s: &str) -> bool {
    s.len() > 1 && s.starts_with('U') && s[1..].bytes().all(|b| b.is_ascii_digit())
}

/// `λ` and `\` both start a lambda.
pub fn is_lambda(s: &str) -> bool {
    s == "λ" || s == "\\"
}

pub fn is_arrow(s: &str) -> bool {
    s == "->" || s == "→"
}

fn is_ident_start(c: char) -> bool {
    c.is_alphanumeric()
}

fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '\''
}

/// Tokenize `src` the way the parser's lexer does.
///
/// The token stream covers the whole input: concatenating every token's text
/// reproduces `src` exactly (whitespace and comments included).  An
/// unterminated `{-` comment becomes a `Comment` token running to the end.
pub fn lex(src: &str) -> Vec<Token> {
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;

    while i < bytes.len() {
        let start = i;
        let c = src[i..].chars().next().unwrap();
        let clen = c.len_utf8();

        // Whitespace (the parser also swallows a byte-order mark).
        if c.is_whitespace() || c == '\u{feff}' {
            i += clen;
            while i < bytes.len() {
                let d = src[i..].chars().next().unwrap();
                if d.is_whitespace() || d == '\u{feff}' {
                    i += d.len_utf8();
                } else {
                    break;
                }
            }
            out.push(Token {
                start,
                end: i,
                kind: TokKind::Whitespace,
            });
            continue;
        }

        // Line comment.
        if c == '-' && bytes.get(i + 1) == Some(&b'-') {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            out.push(Token {
                start,
                end: i,
                kind: TokKind::Comment,
            });
            continue;
        }

        // Block comment (the language does not nest them).
        if c == '{' && bytes.get(i + 1) == Some(&b'-') {
            i += 2;
            while i < bytes.len() {
                if bytes[i] == b'-' && bytes.get(i + 1) == Some(&b'}') {
                    i += 2;
                    break;
                }
                i += 1;
            }
            out.push(Token {
                start,
                end: i,
                kind: TokKind::Comment,
            });
            continue;
        }

        // `(!)` and `->`: two-character forms that must not be split.
        if let Some(p) = LONG_PUNCT.iter().find(|p| src[i..].starts_with(**p)) {
            i += p.len();
            out.push(Token {
                start,
                end: i,
                kind: TokKind::Punct,
            });
            continue;
        }

        // Identifier-shaped words (keywords included; classified below).
        if is_ident_start(c) {
            i += clen;
            while i < bytes.len() {
                let d = src[i..].chars().next().unwrap();
                if is_ident_continue(d) {
                    i += d.len_utf8();
                } else {
                    break;
                }
            }
            let word = &src[start..i];
            let kind = if is_keyword(word) || is_universe(word) {
                TokKind::Keyword
            } else {
                TokKind::Ident
            };
            out.push(Token {
                start,
                end: i,
                kind,
            });
            continue;
        }

        // `λ`, and the single-character punctuation.
        if src[i..].starts_with('λ') {
            i += 'λ'.len_utf8();
            out.push(Token {
                start,
                end: i,
                kind: TokKind::Keyword,
            });
            continue;
        }
        if let Some(p) = PUNCT.iter().find(|p| src[i..].starts_with(**p)) {
            i += p.len();
            out.push(Token {
                start,
                end: i,
                kind: TokKind::Punct,
            });
            continue;
        }

        // Anything else: one character, flagged so the editor can mark it.
        i += clen;
        out.push(Token {
            start,
            end: i,
            kind: TokKind::Unknown,
        });
    }

    out
}

/// Indices of the tokens a human would call "the program": no trivia.
fn significant(toks: &[Token]) -> Vec<usize> {
    toks.iter()
        .enumerate()
        .filter(|(_, t)| !t.is_trivia())
        .map(|(i, _)| i)
        .collect()
}

/// The token under `offset`, preferring real tokens over trivia and, at a
/// boundary, the token that *starts* there.
pub fn token_at(toks: &[Token], offset: usize) -> Option<usize> {
    let mut best: Option<usize> = None;
    for (i, t) in toks.iter().enumerate() {
        if t.start <= offset && offset <= t.end {
            let better = match best {
                None => true,
                Some(b) => {
                    let prev = &toks[b];
                    let rank = |k: TokKind| if k == TokKind::Whitespace { 0 } else { 1 };
                    (rank(t.kind), t.start) >= (rank(prev.kind), prev.start)
                }
            };
            if better {
                best = Some(i);
            }
        }
        if t.start > offset {
            break;
        }
    }
    best
}

// ---------------------------------------------------------------------------
// Binders and scopes
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BinderKind {
    /// `let x : A = t; u`
    Let,
    /// `λ ... x ... . t`
    Lambda,
    /// `(x : A) -> B`
    Pi,
    /// `data D ... where ...` -- the declared type itself.
    Data,
    /// `data D ... where c : A; ...` -- a constructor.
    Ctor,
}

impl BinderKind {
    pub fn describe(self) -> &'static str {
        match self {
            BinderKind::Let => "let",
            BinderKind::Lambda => "λ-pattern",
            BinderKind::Pi => "binder",
            BinderKind::Data => "data declaration",
            BinderKind::Ctor => "constructor",
        }
    }
}

/// One name binding and the region it covers.
#[derive(Clone, Debug)]
pub struct Binder {
    pub name: String,
    pub kind: BinderKind,
    /// Byte range of the identifier that introduces the name.
    pub def: Span,
    /// Byte range in which the name is visible (its de Bruijn scope).
    pub scope: Span,
    /// The type as written in the source, when the syntax gives one
    /// (`let x : A = ...`, `(x : A) -> ...`); λ patterns have none.
    pub type_text: Option<String>,
    /// For `Let` binders: the whole `let x : A = t;` region, for the outline.
    pub full: Span,
}

impl Binder {
    pub fn covers(&self, offset: usize) -> bool {
        offset >= self.scope.0 && offset <= self.scope.1
    }
}

/// How far an expression is scanned before the caller takes over.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Stop {
    /// Everything up to the end of the input.
    End,
    /// Up to the `)` that closes the enclosing group.
    RParen,
    /// Up to the `;` that ends a `let` value.
    Semi,
    /// Up to the `=` that precedes a `let` value.
    Eq,
}

struct Scanner<'a> {
    src: &'a str,
    toks: &'a [Token],
    sig: Vec<usize>,
    i: usize,
    binders: Vec<Binder>,
}

/// A binder group of a function type, filled in once its codomain is known.
struct PendingPi {
    name: String,
    def: Span,
    group: usize,
    type_text: Option<String>,
    after_group: usize,
}

impl<'a> Scanner<'a> {
    fn new(src: &'a str, toks: &'a [Token]) -> Scanner<'a> {
        Scanner {
            src,
            toks,
            sig: significant(toks),
            i: 0,
            binders: Vec::new(),
        }
    }

    fn n(&self) -> usize {
        self.sig.len()
    }

    fn text(&self, k: usize) -> &'a str {
        match self.sig.get(k) {
            Some(t) => self.toks[*t].text(self.src),
            None => "",
        }
    }

    fn span(&self, k: usize) -> Span {
        match self.sig.get(k) {
            Some(t) => (self.toks[*t].start, self.toks[*t].end),
            None => (self.src.len(), self.src.len()),
        }
    }

    fn is_ident(&self, k: usize) -> bool {
        self.sig
            .get(k)
            .is_some_and(|t| self.toks[*t].kind == TokKind::Ident)
    }

    fn at(&self, s: &str) -> bool {
        self.i < self.n() && self.text(self.i) == s
    }

    fn done(&self, stop: Stop) -> bool {
        if self.i >= self.n() {
            return true;
        }
        match stop {
            Stop::End => false,
            Stop::RParen => self.at(")"),
            Stop::Semi => self.at(";"),
            Stop::Eq => self.at("=") || self.at(";") || self.at(")"),
        }
    }

    /// Byte range covered by the significant tokens `[from, to)`.
    fn range_of(&self, from: usize, to: usize) -> Span {
        if to <= from {
            (self.span(from).0, self.span(from).0)
        } else {
            (self.span(from).0, self.span(to - 1).1)
        }
    }

    fn bump(&mut self) -> bool {
        if self.i < self.n() {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn save(&self) -> usize {
        self.i
    }

    fn restore(&mut self, i: usize) {
        self.i = i;
    }

    fn eat(&mut self, s: &str) -> bool {
        if self.at(s) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn eat_arrow(&mut self) -> bool {
        if self.i < self.n() && is_arrow(self.text(self.i)) {
            self.bump();
            true
        } else {
            false
        }
    }

    /// A token that may start an atom (used to decide where a spine ends).
    /// `where` must end the type of a `data` declaration; the other reserved
    /// words (`U`, `Id`, `refl`, `elim`, ...) are ordinary atoms here.
    fn starts_atom(&self) -> bool {
        if self.i >= self.n() {
            return false;
        }
        let t = self.text(self.i);
        !matches!(t, ")" | ";" | "=" | "." | ":" | "(!)" | "where")
            && !is_arrow(t)
            && !is_lambda(t)
    }

    /// `(` binder+ `:` — a dependent function type rather than a parenthesis.
    fn looks_like_pi(&self) -> bool {
        if self.text(self.i) != "(" {
            return false;
        }
        let mut k = self.i + 1;
        let mut names = 0usize;
        while k < self.n() {
            if self.text(k) == "_" || self.is_ident(k) {
                names += 1;
                k += 1;
            } else {
                break;
            }
        }
        names > 0 && k < self.n() && self.text(k) == ":"
    }

    // -- grammar mirror ----------------------------------------------------

    /// Scan one expression, stopping before whatever `stop` selects.
    fn expr(&mut self, stop: Stop) -> usize {
        if self.done(stop) {
            return self.i;
        }
        if is_lambda(self.text(self.i)) {
            return self.lambda(stop);
        }
        match self.text(self.i) {
            "let" => self.let_expr(stop),
            "data" | "struct" => self.data_expr(stop),
            "(" if self.looks_like_pi() => self.pi_expr(stop),
            _ => self.fun_or_spine(stop),
        }
    }

    /// `data D groups* ':' Raw 'where' Ctor* Raw`, with
    /// `Ctor = Ident ':' Raw ';'`.  The declared type and every constructor
    /// become binders; each is visible from just after its own introduction to
    /// the end of the declaration's body.
    fn data_expr(&mut self, stop: Stop) -> usize {
        self.bump(); // data
        let name = self.text(self.i).to_string();
        let data_def = self.span(self.i);
        self.bump(); // the type name
        // parameter groups, shaped like the ones of a Pi
        while self.at("(") && self.looks_like_pi() {
            self.bump(); // (
            while self.i < self.n() && (self.text(self.i) == "_" || self.is_ident(self.i)) {
                self.bump();
            }
            self.eat(":");
            self.expr(Stop::RParen);
            self.eat(")");
        }
        self.eat(":");
        let ty_start = self.i;
        let ty_end = self.expr(Stop::RParen);
        let ty_text = self.src[self.range_of(ty_start, ty_end).0..]
            .split("where")
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        if !self.eat("where") {
            return self.i;
        }
        // constructors: `name : type ;`, as long as they parse
        let mut ctors: Vec<(String, Span, Option<String>, usize)> = Vec::new();
        loop {
            let s = self.save();
            let mut consumed = None;
            if self.is_ident(self.i) {
                let def = self.span(self.i);
                let cname = self.text(self.i).to_string();
                self.bump();
                if self.eat(":") {
                    let t0 = self.i;
                    let t1 = self.expr(Stop::Semi);
                    let range = self.range_of(t0, t1);
                    let type_text = if range.1 > range.0 {
                        Some(self.src[range.0..range.1].trim().to_string())
                    } else {
                        None
                    };
                    if self.eat(";") {
                        consumed = Some((cname, def, type_text, self.i));
                    }
                }
            }
            match consumed {
                Some(c) => ctors.push(c),
                None => {
                    self.restore(s);
                    break;
                }
            }
        }
        let body_start = self.i;
        let body_end = self.expr(stop);
        let body_span = self.range_of(body_start, body_end);
        let data_type_text = (!ty_text.is_empty()).then_some(ty_text);
        self.binders.push(Binder {
            name,
            kind: BinderKind::Data,
            def: data_def,
            scope: body_span,
            type_text: data_type_text,
            full: data_def,
        });
        for (cname, def, type_text, after) in ctors {
            let from = self.range_of(after, body_end).0.min(body_span.1);
            self.binders.push(Binder {
                name: cname,
                kind: BinderKind::Ctor,
                def,
                scope: (from, body_span.1.max(from)),
                type_text,
                full: def,
            });
        }
        body_end
    }

    /// `Spine [-> Raw]`
    fn fun_or_spine(&mut self, stop: Stop) -> usize {
        self.spine(stop);
        if self.eat_arrow() {
            self.expr(stop);
        }
        self.i
    }

    fn spine(&mut self, stop: Stop) -> usize {
        if !self.starts_atom() {
            // Defensive: never loop without progress, even on rejected input.
            self.bump();
            return self.i;
        }
        self.atom(stop);
        while !self.done(stop) && self.starts_atom() {
            let before = self.i;
            self.atom(stop);
            if self.i == before {
                break;
            }
        }
        self.i
    }

    fn atom(&mut self, stop: Stop) -> usize {
        match self.text(self.i) {
            "elim" => {
                self.bump();
                self.atom(stop);
            }
            "Id" => {
                self.bump();
                self.atom(stop);
                self.atom(stop);
                self.atom(stop);
            }
            "(" => {
                self.bump();
                self.expr(Stop::RParen);
                self.eat(")");
            }
            _ => {
                self.bump();
            }
        }
        // `x.f1.f2`: a projection is a dot followed by an identifier, and
        // belongs to the atom (so the field name is not read as a new head).
        while self.at(".") {
            let dot = self.i;
            self.bump();
            if self.i < self.n() && (self.text(self.i) == "_" || self.is_ident(self.i)) {
                self.bump();
            } else {
                self.i = dot;
                break;
            }
        }
        self.i
    }

    /// `('λ' | '\') Pn+ ('.' Raw | '(!)')`
    fn lambda(&mut self, stop: Stop) -> usize {
        self.bump(); // λ or \
        let mut pending: Vec<(String, Span)> = Vec::new();
        loop {
            let t = self.text(self.i);
            if t == "refl" {
                self.bump();
            } else if t == "_" || self.is_ident(self.i) {
                // `_` is a wildcard variable pattern
                let name = t.to_string();
                let span = self.span(self.i);
                self.bump();
                pending.push((name, span));
            } else {
                break;
            }
        }
        if self.at("(!)") {
            // `λ p .. (!)` has no body, so the patterns bind nothing; record
            // them anyway with an empty scope so hover still explains them.
            let here = self.span(self.i).0;
            self.bump();
            for (name, def) in pending {
                self.binders.push(Binder {
                    name,
                    kind: BinderKind::Lambda,
                    def,
                    scope: (here, here),
                    type_text: None,
                    full: def,
                });
            }
            return self.i;
        }
        self.eat(".");
        let body_start = self.i;
        let body_end = self.expr(stop);
        let scope = self.range_of(body_start, body_end);
        for (name, def) in pending {
            self.binders.push(Binder {
                name,
                kind: BinderKind::Lambda,
                def,
                scope,
                type_text: None,
                full: def,
            });
        }
        body_end
    }

    /// `('(' binder+ ':' Raw ')')+ '->' Raw`
    fn pi_expr(&mut self, stop: Stop) -> usize {
        let mut pending: Vec<PendingPi> = Vec::new();
        // (index of the first token after the group, the group's type text)
        let mut groups: Vec<(usize, String)> = Vec::new();
        while self.at("(") && self.looks_like_pi() {
            let group = groups.len();
            self.bump(); // (
            while self.i < self.n() && (self.text(self.i) == "_" || self.is_ident(self.i)) {
                let name = self.text(self.i).to_string();
                let def = self.span(self.i);
                self.bump();
                pending.push(PendingPi {
                    name,
                    def,
                    group,
                    type_text: None,
                    after_group: usize::MAX,
                });
            }
            self.eat(":");
            let ty_start = self.i;
            let ty_end = self.expr(Stop::RParen);
            let ty_range = self.range_of(ty_start, ty_end);
            let type_text = self.src[ty_range.0..ty_range.1].trim().to_string();
            self.eat(")");
            let after_group = self.i;
            groups.push((after_group, type_text));
            for p in pending.iter_mut().filter(|p| p.group == group) {
                p.after_group = after_group;
                p.type_text = Some(groups[group].1.clone());
            }
        }
        self.eat_arrow();
        let cod_start = self.i;
        let cod_end = self.expr(stop);
        let cod_span = self.range_of(cod_start, cod_end);
        for p in pending {
            // de Bruijn scope: a binder is visible from the text after its own
            // group (so later groups may refer to it) up to the codomain's end.
            let after = if p.after_group == usize::MAX {
                cod_start
            } else {
                p.after_group
            };
            let from = self.range_of(after, cod_end).0.min(cod_span.1);
            self.binders.push(Binder {
                name: p.name,
                kind: BinderKind::Pi,
                def: p.def,
                scope: (from, cod_span.1.max(from)),
                type_text: p.type_text,
                full: p.def,
            });
        }
        cod_end
    }

    /// `let x : A = t; u`
    fn let_expr(&mut self, stop: Stop) -> usize {
        let let_start = self.span(self.i).0;
        self.bump(); // let
        let (name, def) = if self.text(self.i) == "_" || self.is_ident(self.i) {
            let name = self.text(self.i).to_string();
            let span = self.span(self.i);
            self.bump();
            (name, span)
        } else {
            let here = self.span(self.i);
            (String::new(), (here.0, here.0))
        };
        self.eat(":");
        let ty_start = self.i;
        let ty_end = self.expr(Stop::Eq);
        let ty_range = self.range_of(ty_start, ty_end);
        let type_text = if ty_range.1 > ty_range.0 {
            Some(self.src[ty_range.0..ty_range.1].trim().to_string())
        } else {
            None
        };
        self.eat("=");
        self.expr(Stop::Semi);
        let semi_end = if self.at(";") {
            let e = self.span(self.i).1;
            self.bump();
            e
        } else {
            self.span(self.i.saturating_sub(1)).1
        };
        let body_start = self.i;
        let body_end = self.expr(stop);
        let scope = self.range_of(body_start, body_end);
        if !name.is_empty() && name != "_" {
            self.binders.push(Binder {
                name,
                kind: BinderKind::Let,
                def,
                scope,
                type_text,
                full: (let_start, semi_end.max(let_start)),
            });
        }
        body_end
    }
}

/// Every name binding in `src`, ordered by position.
///
/// The scan is best-effort by design: it is a *lexical* view of the program, so
/// hover/completion/definition still work on a file the elaborator rejects.
pub fn binders(src: &str) -> Vec<Binder> {
    let toks = lex(src);
    let mut sc = Scanner::new(src, &toks);
    while sc.i < sc.n() {
        let before = sc.i;
        sc.expr(Stop::End);
        if sc.i == before {
            sc.i += 1;
        }
    }
    let mut out = sc.binders;
    out.sort_by_key(|b| (b.def.0, b.def.1));
    out
}

/// The binder a name refers to at `offset`: the innermost one still in scope,
/// falling back to the closest definition before `offset` (which is what makes
/// definition lookup work while the user is editing).
pub fn resolve<'a>(bs: &'a [Binder], name: &str, offset: usize) -> Option<&'a Binder> {
    let mut in_scope: Option<&Binder> = None;
    let mut preceding: Option<&Binder> = None;
    for b in bs.iter().filter(|b| b.name == name) {
        if b.covers(offset) && in_scope.map_or(true, |c| b.scope.0 >= c.scope.0) {
            in_scope = Some(b);
        }
        if b.def.0 <= offset && preceding.map_or(true, |c| b.def.0 >= c.def.0) {
            preceding = Some(b);
        }
    }
    in_scope.or(preceding)
}

// ---------------------------------------------------------------------------
// Hover
// ---------------------------------------------------------------------------

/// What the cursor points at, ready to be turned into an LSP `Hover`.
#[derive(Clone, Debug, PartialEq)]
pub struct HoverInfo {
    pub span: Span,
    pub markdown: String,
}

/// Documentation for the fixed vocabulary, used by hover and completion.
pub fn keyword_doc(word: &str) -> Option<&'static str> {
    Some(match word {
        "U" => "`U` — the universe of level 0.  The tower is predicative: `U : U1 : U2 : ...`.",
        _ if is_universe(word) => {
            "`U` followed by digits (`U1`, `U2`, ...) — the universe of that level in the \
             predicative tower `U : U1 : U2 : ...`."
        }
        "Id" => "`Id A x y` — the identity (propositional equality) type of `A` at `x` and `y`.",
        "refl" => "`refl` — the constructor of `Id A x x`.\n\nAs a λ pattern it refines the context by unifying the indices of the identity type.",
        "data" => "`data D (params) : U where c : ty; ...` — declare an inductive type.  No data types are built in: `Nat` is a declaration like any other.",
        "struct" => "`struct S (params) : U where f : ty; ...` — sugar for a single-constructor `data`: it binds the constructor `mkS` and, per field, a projection `f`.  Fields are read with `x.f`.",
        "where" => "`where` — introduces the constructor list of a `data` declaration.",
        "elim" => "`elim D` — the eliminator generated for a data type `D`: parameters, a motive, one method per constructor, then the target.",
        "let" => "`let x : A = t; u` — a local definition, transparent when evaluating.",
        "λ" => "`λ p1 p2 ... . t` — a lambda.  Patterns may be variables (including `_`), `refl`, or the absurd pattern `(!)`.",
        "in" => "`in` — reserved by the grammar but never accepted, so it cannot be used as an identifier.",
        "(!)" => "`(!)` — the absurd pattern.  It must be the last pattern, and the equation must be contradictory.",
        "->" | "→" => "`A -> B` — a function type.  Write `(x : A) -> B` when `B` depends on `x`.",
        "_" => "`_` — a wildcard binder (`(x : A) -> B`, `let _ : A = t; u`, and a λ pattern).",
        "\\" => "`\\` — ASCII spelling of `λ`.",
        _ => return None,
    })
}

/// Hover information at a 0-based LSP position.
pub fn hover(src: &str, line: usize, character: usize) -> Option<HoverInfo> {
    let index = LineIndex::new(src);
    let offset = index.offset(src, line, character);
    let toks = lex(src);
    let k = token_at(&toks, offset)?;
    let t = &toks[k];
    if t.is_trivia() {
        return None;
    }
    let text = t.text(src);
    let span = (t.start, t.end);

    match t.kind {
        TokKind::Ident => {
            let bs = binders(src);
            let b = resolve(&bs, text, offset)?;
            let mut md = String::from("```dpm-nbe\n");
            md.push_str(&b.name);
            if let Some(ty) = &b.type_text {
                md.push_str(" : ");
                md.push_str(ty);
            }
            md.push_str("\n```");
            md.push('\n');
            match b.kind {
                BinderKind::Let => {
                    md.push_str("bound by `let`");
                    let value = &src[b.full.0..b.full.1];
                    if value.len() <= 120 {
                        md.push_str(&format!("\n\n```dpm-nbe\n{}\n```", value.trim()));
                    }
                }
                BinderKind::Lambda => md.push_str("bound by a λ-pattern"),
                BinderKind::Pi => md.push_str("bound by a function-type binder"),
                BinderKind::Data => md.push_str("declared by `data`"),
                BinderKind::Ctor => md.push_str("a constructor of a `data` declaration"),
            }
            Some(HoverInfo { span, markdown: md })
        }
        _ => keyword_doc(text).map(|doc| HoverInfo {
            span,
            markdown: doc.to_string(),
        }),
    }
}

// ---------------------------------------------------------------------------
// Definition
// ---------------------------------------------------------------------------

/// The definition site of the name at a 0-based LSP position.
pub fn definition(src: &str, line: usize, character: usize) -> Option<Span> {
    let index = LineIndex::new(src);
    let offset = index.offset(src, line, character);
    let toks = lex(src);
    let k = token_at(&toks, offset)?;
    let t = &toks[k];
    if t.kind != TokKind::Ident {
        return None;
    }
    let bs = binders(src);
    resolve(&bs, t.text(src), offset).map(|b| b.def)
}

// ---------------------------------------------------------------------------
// Completion
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: String,
    /// `"keyword"` or `"variable"` (mapped to an LSP `CompletionItemKind` by
    /// the protocol layer).
    pub kind: &'static str,
    pub detail: Option<String>,
    pub documentation: Option<String>,
}

/// Everything that could be typed at a 0-based LSP position: the fixed
/// vocabulary plus every name in scope.
pub fn completions(src: &str, line: usize, character: usize) -> Vec<CompletionItem> {
    let index = LineIndex::new(src);
    let offset = index.offset(src, line, character);
    let mut out: Vec<CompletionItem> = Vec::new();
    for k in KEYWORDS {
        out.push(CompletionItem {
            label: (*k).to_string(),
            kind: "keyword",
            detail: Some("keyword".to_string()),
            documentation: keyword_doc(k).map(str::to_string),
        });
    }
    for p in ["->", "→", "(!)", "_"] {
        out.push(CompletionItem {
            label: p.to_string(),
            kind: "keyword",
            detail: Some("syntax".to_string()),
            documentation: keyword_doc(p).map(str::to_string),
        });
    }

    // Names in scope, innermost binding wins.
    let mut seen: BTreeMap<String, CompletionItem> = BTreeMap::new();
    for b in binders(src) {
        if !b.covers(offset) {
            continue;
        }
        let item = CompletionItem {
            label: b.name.clone(),
            kind: "variable",
            detail: b.type_text.clone(),
            documentation: Some(format!("bound by {}", b.kind.describe())),
        };
        match seen.get(&b.name) {
            // Keep the better description: a typed binding beats a bare one.
            Some(existing) if existing.detail.is_some() && item.detail.is_none() => {}
            _ => {
                seen.insert(b.name.clone(), item);
            }
        }
    }
    out.extend(seen.into_values());
    out.sort_by(|a, b| (a.kind, &a.label).cmp(&(b.kind, &b.label)));
    out.dedup_by(|a, b| a.label == b.label && a.kind == b.kind);
    out
}

// ---------------------------------------------------------------------------
// Document symbols
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct DocumentSymbol {
    pub name: String,
    pub detail: Option<String>,
    /// The whole definition (`let x : A = t;`).
    pub span: Span,
    /// The name itself, for "go to symbol in file".
    pub selection: Span,
    pub children: Vec<DocumentSymbol>,
}

/// Top-level definitions, nested the way the syntax nests them: `let a = ...;`
/// whose body holds `let b = ...;` becomes `a` containing `b`.
pub fn symbols(src: &str) -> Vec<DocumentSymbol> {
    let lets: Vec<Binder> = binders(src)
        .into_iter()
        .filter(|b| b.kind == BinderKind::Let)
        .collect();
    nest(&lets)
}

fn nest(items: &[Binder]) -> Vec<DocumentSymbol> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < items.len() {
        let b = &items[i];
        // Following definitions written inside this one's body are children.
        let mut j = i + 1;
        while j < items.len() && b.covers(items[j].def.0) {
            j += 1;
        }
        out.push(DocumentSymbol {
            name: b.name.clone(),
            detail: b.type_text.clone(),
            span: b.full,
            selection: b.def,
            children: nest(&items[i + 1..j]),
        });
        i = j;
    }
    out
}

// ---------------------------------------------------------------------------
// Diagnostics and evaluation
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    pub span: Span,
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
    /// Extra lines (the rendered caret diagram) for a tooltip long description.
    pub rendered: Option<String>,
}

/// Above this size the server refuses to run the checker and says so, instead
/// of blocking the editor on a pathological file.
pub const MAX_CHECK_BYTES: usize = 200_000;

/// Parse and elaborate `src`, returning at most one diagnostic.
pub fn diagnostics(src: &str, uri: &str) -> Vec<Diagnostic> {
    if src.len() > MAX_CHECK_BYTES {
        let index = LineIndex::new(src);
        let (start, end) = index.line_range(src, 0);
        return vec![Diagnostic {
            span: (start, end),
            severity: Severity::Warning,
            code: "file-too-large",
            message: format!(
                "file is {} bytes; the checker is skipped above {} bytes",
                src.len(),
                MAX_CHECK_BYTES
            ),
            rendered: None,
        }];
    }

    match parse::parse_named(src, uri) {
        Err(e) => vec![from_error(src, &Error::from(e), "parse-error")],
        Ok(_) => match typecheck_named(src, uri) {
            Err(e) => vec![from_error(src, &e, "type-error")],
            Ok(_) => Vec::new(),
        },
    }
}

/// Byte span of a 1-based `(line, character-index)` error position.
pub fn span_of_pos(src: &str, e: &Error) -> Span {
    let index = LineIndex::new(src);
    let line = e.pos.line.saturating_sub(1);
    let (start, end) = index.line_range(src, line);
    let want = e.pos.col.saturating_sub(1);
    let mut byte = end;
    for (i, (off, _c)) in src[start..end].char_indices().enumerate() {
        if i == want {
            byte = start + off;
            break;
        }
    }
    let width = index.char_width(src, byte);
    (byte, (byte + width).min(src.len()).max(byte))
}

fn from_error(src: &str, e: &Error, code: &'static str) -> Diagnostic {
    Diagnostic {
        span: span_of_pos(src, e),
        severity: Severity::Error,
        code,
        message: e.msg.clone(),
        rendered: Some(e.render(src)),
    }
}

/// The result of running the checker like the `dpm-nbe` CLI does.
#[derive(Clone, Debug, PartialEq)]
pub struct EvalResult {
    pub ok: bool,
    pub mode: String,
    /// The CLI's stdout on success.
    pub output: String,
    /// The human-readable error on failure.
    pub error: Option<String>,
    /// Where to point when it failed.
    pub error_span: Option<Span>,
}

/// `mode` is `"nf"` (normal form and type) or `"type"`.
pub fn evaluate(src: &str, uri: &str, mode: &str) -> EvalResult {
    let m = match mode {
        "type" => Mode::Type,
        _ => Mode::Nf,
    };
    match run_named(src, uri, m) {
        Ok(output) => EvalResult {
            ok: true,
            mode: mode.to_string(),
            output,
            error: None,
            error_span: None,
        },
        Err(e) => EvalResult {
            ok: false,
            mode: mode.to_string(),
            output: String::new(),
            error: Some(e.render(src)),
            error_span: Some(span_of_pos(src, &e)),
        },
    }
}
