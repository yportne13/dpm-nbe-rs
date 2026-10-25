//! A hand-written recursive-descent parser for the surface syntax.
//!
//! The grammar extends the one of `Parse.hs` (originally written with
//! megaparsec) with inductive declarations and their eliminators:
//!
//! ```text
//! Raw     ::= Lam | Let | Data | Pi | Spine [-> Raw]
//! Lam     ::= ('λ' | '\') Pn+ ('.' Raw | '(!)')
//! Pn      ::= 'refl' | ident
//! Let     ::= 'let' binder ':' Raw '=' Raw ';' Raw
//! Data    ::= 'data' Ident Group* ':' Raw 'where' Ctor* Raw
//! Group   ::= '(' binder+ ':' Raw ')'
//! Ctor    ::= Ident ':' Raw ';'
//! Pi      ::= Group+ '->' Raw
//! Spine   ::= Atom+
//! Atom    ::= ident | U | refl | 'elim' Atom | Id Atom Atom Atom
//!           | '(' Raw ')'
//! ```
//!
//! There are no built-in data types: `Nat` and friends are ordinary
//! declarations (`data Nat : U where zero : Nat; suc : Nat -> Nat;`).
//!
//! One deliberate change with respect to the original: alternatives backtrack
//! unconditionally, so an identifier such as `letx` parses as a variable
//! instead of a hard parse error.

use crate::syntax::{Name, Pn, Pos, Raw};
use std::rc::Rc;

/// A parse error with the position of the furthest failure.
#[derive(Clone, Debug)]
pub struct ParseError {
    pub msg: String,
    pub pos: Pos,
}

/// Parse a program from `src`, reporting positions as `<stdin>`.
pub fn parse(src: &str) -> Result<Raw, ParseError> {
    parse_named(src, "<stdin>")
}

/// Parse a program from `src`, using `file` in reported positions.
pub fn parse_named(src: &str, file: &str) -> Result<Raw, ParseError> {
    let chars: Vec<char> = src.chars().collect();
    let mut p = Parser {
        src: &chars,
        i: 0,
        line: 1,
        col: 1,
        file: Rc::from(file),
        best: None,
        unterminated: None,
        too_deep: None,
        depth: 0,
    };
    match p.program() {
        Ok(r) => Ok(r),
        Err(()) => Err(p.into_error()),
    }
}

/// Maximum expression nesting; keeps a pathological input from blowing the
/// stack in the recursive-descent parser (and, indirectly, in the elaborator).
///
/// It bounds every way the grammar can nest: `raw`/`atom` recursion, the number
/// of arguments in an application, the number of `(x : A)` binder groups and
/// the number of lambda patterns.  The budget is shared, so a construct that
/// goes through several of these (e.g. `suc (...)`) spends several units per
/// level.
pub const MAX_DEPTH: usize = 512;

/// Highest universe level the parser accepts: 2^30 - 1, so every accepted
/// level is strictly below 2^30 (keeps `U999...9` from overflowing while
/// staying far above anything a program needs).
pub const MAX_LEVEL: u32 = (1 << 30) - 1;

fn parse_level(digits: &str) -> Option<u32> {
    digits.parse::<u32>().ok().filter(|l| *l <= MAX_LEVEL)
}

fn is_keyword(s: &str) -> bool {
    // `in` is listed by the original's `keyword` helper but the grammar never
    // accepts it; keeping it here means `in` is rejected as a variable too.
    matches!(
        s,
        "let" | "in" | "λ" | "U" | "refl" | "Id" | "data" | "struct"
            | "where" | "elim"
    )
}

/// Identifier start: an alphanumeric character.
fn is_ident_start(c: char) -> bool {
    c.is_alphanumeric()
}

/// Identifier continuation.  Besides alphanumerics this accepts `'`, which the
/// pretty printer uses to freshen names (`A'`); without it printed terms could
/// not be read back.
fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '\''
}

struct Parser<'a> {
    src: &'a [char],
    i: usize,
    line: usize,
    col: usize,
    file: Rc<str>,
    /// Furthest failure so far: `(offset, position, message)`.
    best: Option<(usize, Pos, String)>,
    /// Position of an unterminated `{-` comment, if one was seen.
    unterminated: Option<Pos>,
    /// Position of the nesting that exceeded `MAX_DEPTH`, if any.
    too_deep: Option<Pos>,
    depth: usize,
}

type PResult<T> = Result<T, ()>;

impl<'a> Parser<'a> {
    // -- positions ---------------------------------------------------------

    fn save(&self) -> (usize, usize, usize) {
        (self.i, self.line, self.col)
    }

    fn restore(&mut self, s: (usize, usize, usize)) {
        self.i = s.0;
        self.line = s.1;
        self.col = s.2;
    }

    fn pos(&self) -> Pos {
        Pos {
            file: self.file.clone(),
            line: self.line,
            col: self.col,
        }
    }

    fn peek(&self) -> Option<char> {
        self.src.get(self.i).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = *self.src.get(self.i)?;
        self.i += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    // -- errors ------------------------------------------------------------

    fn fail<T>(&mut self, msg: &str) -> PResult<T> {
        self.fail_at(self.i, self.pos(), msg)
    }

    /// Record a failure at an explicit offset (so that a message can point at
    /// the start of a token even after the token was consumed).
    fn fail_at<T>(&mut self, offset: usize, pos: Pos, msg: &str) -> PResult<T> {
        let better = match &self.best {
            None => true,
            Some((i, _, _)) => offset >= *i,
        };
        if better {
            self.best = Some((offset, pos, msg.to_string()));
        }
        Err(())
    }

    /// Report that the nesting budget is exhausted.  The marker is sticky, so
    /// a sibling alternative cannot replace it with an unrelated message.
    fn too_deep<T>(&mut self) -> PResult<T> {
        if self.too_deep.is_none() {
            self.too_deep = Some(self.pos());
        }
        Err(())
    }

    fn into_error(self) -> ParseError {
        match self.best {
            Some((_, pos, msg)) => ParseError { msg, pos },
            None => ParseError {
                msg: "unexpected end of input".to_string(),
                pos: self.pos(),
            },
        }
    }

    /// Run `f`, restoring the position if it fails.
    fn alt<T>(&mut self, f: impl FnOnce(&mut Self) -> PResult<T>) -> PResult<T> {
        let s = self.save();
        match f(self) {
            Ok(v) => Ok(v),
            Err(()) => {
                self.restore(s);
                Err(())
            }
        }
    }

    // -- lexing ------------------------------------------------------------

    fn skip_ws(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() || c == '\u{feff}' => {
                    self.bump();
                }
                Some('-') if self.src.get(self.i + 1) == Some(&'-') => {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.bump();
                    }
                }
                Some('{') if self.src.get(self.i + 1) == Some(&'-') => {
                    let start = self.pos();
                    self.bump();
                    self.bump();
                    let mut closed = false;
                    while self.i < self.src.len() {
                        if self.peek() == Some('-') && self.src.get(self.i + 1) == Some(&'}') {
                            self.bump();
                            self.bump();
                            closed = true;
                            break;
                        }
                        self.bump();
                    }
                    if !closed && self.unterminated.is_none() {
                        self.unterminated = Some(start);
                    }
                }
                _ => break,
            }
        }
    }

    /// Match a fixed string, consuming surrounding whitespace.
    fn sym(&mut self, s: &str) -> PResult<()> {
        self.skip_ws();
        let n = s.chars().count();
        let matches = self.i + n <= self.src.len()
            && self.src[self.i..self.i + n].iter().copied().eq(s.chars());
        if matches {
            for _ in 0..n {
                self.bump();
            }
            self.skip_ws();
            Ok(())
        } else {
            self.fail(&format!("expected '{s}'"))
        }
    }

    fn expect_char(&mut self, c: char) -> PResult<()> {
        self.skip_ws();
        if self.peek() == Some(c) {
            self.bump();
            self.skip_ws();
            Ok(())
        } else {
            self.fail(&format!("expected '{c}'"))
        }
    }

    /// Read an identifier word (keywords included).
    fn word(&mut self) -> PResult<Name> {
        self.skip_ws();
        let start = self.i;
        while self.peek().is_some_and(is_ident_continue) {
            self.bump();
        }
        if self.i == start {
            return self.fail("expected an identifier");
        }
        let s: String = self.src[start..self.i].iter().collect();
        self.skip_ws();
        Ok(s)
    }

    /// Read an identifier (a word that is not a keyword).
    fn ident(&mut self) -> PResult<Name> {
        self.skip_ws();
        let start = self.i;
        let pos = self.pos();
        let s = self.word()?;
        if is_keyword(&s) {
            // `word` consumed the token and the whitespace after it, so report
            // the failure at the start of the token.
            self.fail_at(start, pos, &format!("'{s}' is a keyword"))
        } else {
            Ok(s)
        }
    }

    /// The word starting at the current position, without consuming it.
    fn peek_word(&self) -> &'a [char] {
        let mut j = self.i;
        while self.src.get(j).is_some_and(|c| c.is_whitespace()) {
            j += 1;
        }
        let s = j;
        while self.src.get(j).is_some_and(|c| is_ident_continue(*c)) {
            j += 1;
        }
        &self.src[s..j]
    }

    fn peek_word_is(&self, w: &str) -> bool {
        self.peek_word().iter().copied().eq(w.chars())
    }

    // -- grammar -----------------------------------------------------------

    fn program(&mut self) -> PResult<Raw> {
        self.skip_ws();
        // Parse first, then let the sticky markers override whatever failure was
        // recorded deepest: exhausting the nesting budget is the real reason.
        let r = self.raw();
        self.skip_ws();
        if let Some(pos) = self.too_deep.take() {
            self.best = Some((
                usize::MAX,
                pos,
                "expression nesting is too deep".to_string(),
            ));
            return Err(());
        }
        if let Some(pos) = self.unterminated.take() {
            // An unterminated block comment is an error, as in the original
            // (`skipBlockComment` fails); `usize::MAX` makes it win over any
            // other recorded failure.
            self.best = Some((usize::MAX, pos, "unterminated block comment".to_string()));
            return Err(());
        }
        let r = r?;
        if self.i == self.src.len() {
            Ok(r)
        } else {
            self.fail("expected end of input")
        }
    }

    fn raw(&mut self) -> PResult<Raw> {
        self.skip_ws();
        let pos = self.pos();
        if self.depth >= MAX_DEPTH {
            return self.too_deep();
        }
        self.depth += 1;
        let r = self.raw_inner();
        self.depth -= 1;
        Ok(Raw::SrcPos(pos, Box::new(r?)))
    }

    fn raw_inner(&mut self) -> PResult<Raw> {
        match self.peek() {
            Some('λ') | Some('\\') => self.lam(),
            _ if self.peek_word_is("let") => self.let_expr(),
            _ if self.peek_word_is("data") => self.data_decl(),
            _ if self.peek_word_is("struct") => self.struct_decl(),
            Some('(') => match self.alt(Self::pi) {
                Ok(x) => Ok(x),
                Err(()) => self.fun_or_spine(),
            },
            _ => self.fun_or_spine(),
        }
    }

    /// `data D groups* ':' Raw where Ctor* Raw`.
    fn data_decl(&mut self) -> PResult<Raw> {
        self.skip_ws();
        self.word()?; // "data"
        self.decl_tail(false)
    }

    /// `struct S groups* ':' Raw where Field* Raw` -- parsed exactly like a
    /// `data` declaration; the elaborator is what makes the difference.
    fn struct_decl(&mut self) -> PResult<Raw> {
        self.skip_ws();
        self.word()?; // "struct"
        self.decl_tail(true)
    }

    /// The shared tail of a `data`/`struct` declaration:
    /// `name groups* ':' Raw where (name ':' Raw ';')* Raw`.
    fn decl_tail(&mut self, is_struct: bool) -> PResult<Raw> {
        let name = self.ident()?;

        let mut params: Vec<(Name, Raw)> = Vec::new();
        while self
            .alt(|p| p.param_group(&mut params))
            .is_ok()
        {
            if params.len() > MAX_DEPTH {
                return self.too_deep();
            }
        }
        self.expect_char(':')?;
        let ty = self.raw()?;
        self.sym("where")?;
        let mut ctors: Vec<(Name, Raw)> = Vec::new();
        loop {
            let s = self.save();
            match self.ctor() {
                Ok(c) => {
                    ctors.push(c);
                    if ctors.len() > MAX_DEPTH {
                        return self.too_deep();
                    }
                }
                Err(()) => {
                    self.restore(s);
                    break;
                }
            }
        }
        // Zero constructors is allowed: it declares the empty type.
        let body = self.raw()?;
        Ok(if is_struct {
            Raw::Struct(name, params, Box::new(ty), ctors, Box::new(body))
        } else {
            Raw::Data(name, params, Box::new(ty), ctors, Box::new(body))
        })
    }

    /// `'(' binder+ ':' Raw ')'`, appending `(name, type)` pairs to `out`.
    fn param_group(&mut self, out: &mut Vec<(Name, Raw)>) -> PResult<()> {
        self.expect_char('(')?;
        let mut binders: Vec<Name> = Vec::new();
        while let Ok(b) = self.alt(Self::binder) {
            binders.push(b);
            if binders.len() > MAX_DEPTH {
                return self.too_deep();
            }
        }
        if binders.is_empty() {
            return self.fail("expected a binder");
        }
        self.expect_char(':')?;
        let ty = self.raw()?;
        self.expect_char(')')?;
        for x in binders {
            out.push((x, ty.clone()));
        }
        Ok(())
    }

    /// `Ident ':' Raw ';'`.  A body expression can never begin with this shape
    /// (a bare `x :` is not an expression), so constructor lists need no
    /// terminator: parsing stops at the first item without a trailing `;`.
    fn ctor(&mut self) -> PResult<(Name, Raw)> {
        let name = self.ident()?;
        self.expect_char(':')?;
        let ty = self.raw()?;
        self.expect_char(';')?;
        Ok((name, ty))
    }

    fn lam(&mut self) -> PResult<Raw> {
        self.skip_ws();
        match self.peek() {
            Some('λ') | Some('\\') => {
                self.bump();
                self.skip_ws();
            }
            _ => return self.fail("expected 'λ'"),
        }
        let mut ps: Vec<Pn> = Vec::new();
        while let Ok(p) = self.alt(Self::pn) {
            ps.push(p);
            if ps.len() > MAX_DEPTH {
                return self.too_deep();
            }
        }
        if ps.is_empty() {
            return self.fail("expected at least one pattern");
        }
        let s = self.save();
        if self.sym("(!)").is_ok() {
            ps.push(Pn::Abs);
            return Ok(Raw::Lam(ps, Box::new(Raw::Bot)));
        }
        self.restore(s);
        self.expect_char('.')?;
        let body = self.raw()?;
        Ok(Raw::Lam(ps, Box::new(body)))
    }

    fn pn(&mut self) -> PResult<Pn> {
        self.skip_ws();
        if self.peek_word_is("refl") {
            self.word()?;
            return Ok(Pn::Refl);
        }
        // `_` is a wildcard: a variable pattern that is never used.
        let s = self.save();
        if self.sym("_").is_ok() {
            return Ok(Pn::Var("_".to_string()));
        }
        self.restore(s);
        Ok(Pn::Var(self.ident()?))
    }

    fn pi(&mut self) -> PResult<Raw> {
        let mut doms: Vec<(Vec<Name>, Raw)> = Vec::new();
        while self.peek() == Some('(') {
            if doms.len() >= MAX_DEPTH {
                // The groups become a right-nested `Pi` chain, so they need a
                // budget of their own.
                return self.too_deep();
            }
            self.expect_char('(')?;
            let mut binders: Vec<Name> = Vec::new();
            while let Ok(b) = self.alt(Self::binder) {
                binders.push(b);
                if binders.len() > MAX_DEPTH {
                    return self.too_deep();
                }
            }
            if binders.is_empty() {
                return self.fail("expected a binder");
            }
            self.expect_char(':')?;
            let ty = self.raw()?;
            self.expect_char(')')?;
            doms.push((binders, ty));
        }
        if doms.is_empty() {
            return self.fail("expected a function type");
        }
        self.arrow()?;
        let cod = self.raw()?;
        let mut acc = cod;
        for (xs, a) in doms.into_iter().rev() {
            for x in xs.into_iter().rev() {
                acc = Raw::Pi(x, Box::new(a.clone()), Box::new(acc));
            }
        }
        Ok(acc)
    }

    fn fun_or_spine(&mut self) -> PResult<Raw> {
        let sp = self.spine()?;
        let s = self.save();
        if self.arrow().is_ok() {
            let cod = self.raw()?;
            Ok(Raw::Pi("_".to_string(), Box::new(sp), Box::new(cod)))
        } else {
            self.restore(s);
            Ok(sp)
        }
    }

    fn arrow(&mut self) -> PResult<()> {
        match self.alt(|p| p.sym("→")) {
            Ok(()) => Ok(()),
            Err(()) => self.sym("->"),
        }
    }

    fn spine(&mut self) -> PResult<Raw> {
        let mut acc = self.atom()?;
        let mut args = 1usize;
        while let Ok(a) = self.alt(Self::atom) {
            // Each argument adds a level to the left-nested `App` chain, so
            // this loop needs its own budget (the depth counter is back at its
            // starting value after every `atom`).
            args += 1;
            if args > MAX_DEPTH {
                return self.too_deep();
            }
            acc = Raw::App(Box::new(acc), Box::new(a));
        }
        Ok(acc)
    }

    fn atom(&mut self) -> PResult<Raw> {
        self.skip_ws();
        if self.depth >= MAX_DEPTH {
            return self.too_deep();
        }
        self.depth += 1;
        let r = self.atom_inner();
        self.depth -= 1;
        r
    }

    fn atom_inner(&mut self) -> PResult<Raw> {
        self.skip_ws();
        let pos = self.pos();
        let r = match self.peek() {
            Some('(') => {
                self.bump();
                self.skip_ws();
                let r = self.raw()?;
                self.expect_char(')')?;
                r
            }
            Some(c) if is_ident_start(c) => {
                let start = self.i;
                let name = self.word()?;
                match name.as_str() {
                    // `U` alone is level 0; `U1`, `U2`, ... name the higher
                    // universes (`U` followed by digits only).
                    _ if name == "U" => Raw::U(0),
                    _ if name.len() > 1
                        && name.starts_with('U')
                        && name[1..].bytes().all(|b| b.is_ascii_digit()) =>
                    {
                        match parse_level(&name[1..]) {
                            Some(l) => Raw::U(l),
                            None => {
                                return self.fail_at(
                                    start,
                                    pos,
                                    "universe level is too large",
                                )
                            }
                        }
                    }
                    "refl" => Raw::Refl,
                    // `elim D` takes the data type itself as an atom; the
                    // spine it is applied to (motive, methods, target) is
                    // ordinary application.
                    "elim" => Raw::Elim(Box::new(self.atom()?)),
                    "Id" => Raw::Id(
                        Box::new(self.atom()?),
                        Box::new(self.atom()?),
                        Box::new(self.atom()?),
                    ),
                    _ if is_keyword(&name) => {
                        return self.fail_at(start, pos, &format!("unexpected keyword '{name}'"))
                    }
                    _ => Raw::Var(name),
                }
            }
            _ => return self.fail("expected an atom"),
        };
        // Postfix projections: `x.f1.f2`, as long as a `.` is followed by a
        // plain identifier (the `.` of a lambda comes after patterns, never
        // after a complete atom, so there is no ambiguity).
        let mut r = r;
        let mut projections = 0usize;
        loop {
            let s = self.save();
            if self.expect_char('.').is_err() {
                break;
            }
            match self.ident() {
                Ok(f) => {
                    projections += 1;
                    if projections > MAX_DEPTH {
                        return self.too_deep();
                    }
                    r = Raw::Proj(Box::new(r), f);
                }
                Err(()) => {
                    self.restore(s);
                    break;
                }
            }
        }
        Ok(Raw::SrcPos(pos, Box::new(r)))
    }

    fn binder(&mut self) -> PResult<Name> {
        let s = self.save();
        match self.ident() {
            Ok(x) => Ok(x),
            Err(()) => {
                self.restore(s);
                self.sym("_").map(|_| "_".to_string())
            }
        }
    }

    fn let_expr(&mut self) -> PResult<Raw> {
        self.skip_ws();
        self.word()?; // "let"
        let x = self.binder()?;
        self.expect_char(':')?;
        let a = self.raw()?;
        self.expect_char('=')?;
        let t = self.raw()?;
        self.expect_char(';')?;
        let u = self.raw()?;
        Ok(Raw::Let(x, Box::new(a), Box::new(t), Box::new(u)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::{Pn, Raw};

    fn strip(src: &str) -> Raw {
        parse(src)
            .unwrap_or_else(|e| panic!("{src:?} should parse: {}", e.msg))
            .strip_pos()
    }

    fn var(x: &str) -> Box<Raw> {
        Box::new(Raw::Var(x.to_string()))
    }

    #[test]
    fn parses_atoms() {
        assert_eq!(strip("U"), Raw::U(0));
        assert_eq!(strip("U1"), Raw::U(1));
        assert_eq!(strip("U23"), Raw::U(23));
        assert!(parse("U99999999999").is_err(), "level overflow rejected");
        assert_eq!(strip("U1073741823"), Raw::U(1073741823));
        assert!(parse("U1073741824").is_err(), "level 2^30 rejected");
        assert_eq!(strip("refl"), Raw::Refl);
        assert_eq!(strip("zero"), *var("zero"));
        assert_eq!(strip("Nat"), *var("Nat"));
        assert_eq!(strip("elim Nat"), Raw::Elim(var("Nat")));
        assert_eq!(
            strip("Id Nat zero zero"),
            Raw::Id(var("Nat"), var("zero"), var("zero"))
        );
    }

    #[test]
    fn application_is_left_associative() {
        assert_eq!(
            strip("f x y"),
            Raw::App(Box::new(Raw::App(var("f"), var("x"))), var("y"))
        );
    }

    #[test]
    fn parses_pi_types() {
        assert_eq!(
            strip("(A : U) -> A -> A"),
            Raw::Pi(
                "A".into(),
                Box::new(Raw::U(0)),
                Box::new(Raw::Pi("_".into(), var("A"), var("A")))
            )
        );
        // anonymous domain
        assert_eq!(
            strip("Nat -> Nat"),
            Raw::Pi("_".into(), var("Nat"), var("Nat"))
        );
    }

    #[test]
    fn groups_binders_sharing_a_type() {
        assert_eq!(
            strip("(a b : Nat) -> Nat"),
            Raw::Pi(
                "a".into(),
                var("Nat"),
                Box::new(Raw::Pi("b".into(), var("Nat"), var("Nat")))
            )
        );
    }

    #[test]
    fn several_domains_nest_to_the_right() {
        assert_eq!(
            strip("(A : U) (a : A) -> A"),
            Raw::Pi(
                "A".into(),
                Box::new(Raw::U(0)),
                Box::new(Raw::Pi("a".into(), var("A"), var("A")))
            )
        );
    }

    #[test]
    fn parses_lambdas_and_patterns() {
        assert_eq!(
            strip("\\ x refl . refl"),
            Raw::Lam(vec![Pn::Var("x".into()), Pn::Refl], Box::new(Raw::Refl))
        );
        assert_eq!(
            strip("λ x . x"),
            Raw::Lam(vec![Pn::Var("x".into())], var("x"))
        );
    }

    #[test]
    fn parses_absurd_lambda() {
        assert_eq!(
            strip("\\ A (!)"),
            Raw::Lam(vec![Pn::Var("A".into()), Pn::Abs], Box::new(Raw::Bot))
        );
    }

    #[test]
    fn parses_let_expressions() {
        assert_eq!(
            strip("let x : Nat = zero; x"),
            Raw::Let("x".into(), var("Nat"), var("zero"), var("x"))
        );
        assert_eq!(
            strip("let _ : Nat = zero; U"),
            Raw::Let("_".into(), var("Nat"), var("zero"), Box::new(Raw::U(0)))
        );
    }

    #[test]
    fn parses_data_declarations() {
        assert_eq!(
            strip("data Nat : U where zero : Nat; suc : Nat -> Nat; zero"),
            Raw::Data(
                "Nat".into(),
                vec![],
                Box::new(Raw::U(0)),
                vec![
                    ("zero".into(), *var("Nat")),
                    (
                        "suc".into(),
                        Raw::Pi("_".into(), var("Nat"), var("Nat"))
                    )
                ],
                var("zero")
            )
        );
        // parameterised, and a body that starts with a constructor-shaped
        // word (it must not be eaten as one more constructor)
        assert_eq!(
            strip(
                "data List (A : U) : U where nil : List A; \
                 cons : A -> List A -> List A; nil A"
            ),
            Raw::Data(
                "List".into(),
                vec![("A".into(), Raw::U(0))],
                Box::new(Raw::U(0)),
                vec![
                    ("nil".into(), Raw::App(var("List"), var("A"))),
                    (
                        "cons".into(),
                        Raw::Pi(
                            "_".into(),
                            var("A"),
                            Box::new(Raw::Pi(
                                "_".into(),
                                Box::new(Raw::App(var("List"), var("A"))),
                                Box::new(Raw::App(var("List"), var("A")))
                            ))
                        )
                    )
                ],
                Box::new(Raw::App(var("nil"), var("A")))
            )
        );
        // the empty type: no constructors at all
        assert_eq!(
            strip("data Empty : U where U"),
            Raw::Data(
                "Empty".into(),
                vec![],
                Box::new(Raw::U(0)),
                vec![],
                Box::new(Raw::U(0))
            )
        );
    }

    #[test]
    fn skips_comments_and_unicode_arrows() {
        assert_eq!(strip("{- block -} Nat -- line\n"), *var("Nat"));
        assert_eq!(
            strip("Nat → Nat"),
            Raw::Pi("_".into(), var("Nat"), var("Nat"))
        );
        assert_eq!(strip("-- only a comment\nU"), Raw::U(0));
    }

    #[test]
    fn parenthesised_expressions_are_just_expressions() {
        assert_eq!(strip("(Nat)"), *var("Nat"));
        assert_eq!(strip("(f x)"), Raw::App(var("f"), var("x")));
    }

    #[test]
    fn keywords_are_not_identifiers() {
        // `elim` needs its data-type atom, so this is a parse error
        assert!(parse("elim").is_err());
        assert!(parse("data").is_err());
        assert!(parse("where").is_err());
        // ... but an identifier that merely starts like a keyword is fine,
        // and the former built-ins are ordinary identifiers now
        assert_eq!(strip("sucx"), *var("sucx"));
        assert_eq!(strip("letx"), *var("letx"));
        assert_eq!(strip("plus"), *var("plus"));
        assert_eq!(strip("succ"), *var("succ"));
    }

    #[test]
    fn reports_the_furthest_failure_position() {
        let e = parse("Id Nat zero").unwrap_err();
        assert_eq!(e.pos.line, 1);
        assert!(e.pos.col > 1, "column should point past the parsed prefix");

        let e = parse("let x : Nat =\n  zero;\n  x\n  +").unwrap_err();
        assert_eq!(e.pos.line, 4);
    }

    #[test]
    fn deeply_nested_input_is_rejected_without_overflowing() {
        let src = format!("{}U{}", "(".repeat(2000), ")".repeat(2000));
        let e = parse(&src).unwrap_err();
        assert!(e.msg.contains("deep"), "unexpected message: {}", e.msg);
    }

    #[test]
    fn every_nesting_path_is_bounded() {
        // These inputs used to bypass MAX_DEPTH (only raw checked it) and
        // abort the process with a stack overflow.
        let cases = [
            format!("{}zero", "elim ".repeat(5000)),
            "zero ".repeat(5000),
            format!("{}-> U", "(a : U) ".repeat(5000)),
            format!("({} : U) -> U", "a ".repeat(5000)),
            format!("\\ {} . x", "x ".repeat(5000)),
        ];
        for src in cases {
            let e = parse(&src).unwrap_err();
            assert_eq!(e.msg, "expression nesting is too deep");
        }
    }

    #[test]
    fn nesting_below_the_limit_is_accepted() {
        assert!(parse(&format!("{}zero", "elim ".repeat(400))).is_ok());
        assert!(parse(&format!("{}zero{}", "suc (".repeat(150), ")".repeat(150))).is_ok());
        assert!(parse(&format!("{}-> U", "(a : U) ".repeat(400))).is_ok());
    }

    #[test]
    fn parses_the_readme_program() {
        let src = "data Nat : U where\n  zero : Nat;\n  suc : Nat -> Nat;\n\
let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> Id A a c =\n  λ A a b c refl refl . refl;\n\
let cong : (A B : U) (f : A -> B) (a b : A) -> Id A a b -> Id B (f a) (f b) =\n  λ A B f a b refl . refl;\n\
let plus : Nat -> Nat -> Nat =\n  λ m n . elim Nat (λ _ . Nat) n (λ _ r . suc r) m;\n\
let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero =\n  λ m n refl refl . refl;\n\
let C : (A : U) -> Id Nat (suc zero) zero -> A =\n  λ A (!);\n\
U\n";
        assert!(
            parse(src).is_ok(),
            "the README example program should parse"
        );
    }


    #[test]
    fn parses_struct_declarations_and_projections() {
        assert_eq!(
            strip("struct Pair (A : U) : U where fst : A; snd : A; snd"),
            Raw::Struct(
                "Pair".into(),
                vec![("A".into(), Raw::U(0))],
                Box::new(Raw::U(0)),
                vec![
                    ("fst".into(), *var("A")),
                    ("snd".into(), *var("A")),
                ],
                var("snd"),
            )
        );
        // postfix projections, chained
        assert_eq!(
            strip("p.fst.snd"),
            Raw::Proj(
                Box::new(Raw::Proj(var("p"), "fst".into())),
                "snd".into()
            )
        );
        // the lambda dot is not a projection (a projection needs an
        // identifier after the dot)
        assert_eq!(
            strip("\\ x . x"),
            Raw::Lam(vec![Pn::Var("x".into())], var("x"))
        );
        assert!(parse("p.").is_err());
    }

    #[test]
    fn positions_are_attached_to_atoms() {
        match parse("zero").unwrap() {
            Raw::SrcPos(pos, _) => assert_eq!((pos.line, pos.col), (1, 1)),
            other => panic!("expected a position wrapper, got {other:?}"),
        }
    }
}
