//! Typing contexts and error reporting.
//!
//! Direct port of `Context.hs`, with `M` specialised to `Result`.

use crate::eval::{subst_env, subst_val};
use crate::syntax::{Env, Ix, Lvl, Name, Pos, Sub, Val};
use std::rc::Rc;

/// A typing context.  Since substitutions never delete entries, `lvl` is the
/// total number of bindings ever introduced.
#[derive(Clone, Debug)]
pub struct Ctx {
    pub lvl: Lvl,
    /// Kept for parity with `Context.hs`; only `types` is read.
    pub names: Vec<Name>,
    pub types: Vec<(Name, Val)>,
    /// The context chain as quoted terms, newest first, parallel to
    /// `types`: this is the zoo's `Locals`, kept so closing a fresh meta's
    /// type over the context costs no re-quotation (which would make deeply
    /// nested binders with holes cubic).
    pub locals: Vec<crate::meta::LocalEntry>,
    pub env: Env,
    pub pos: Pos,
}

/// A source-located error message.
#[derive(Clone, Debug)]
pub struct Error {
    pub msg: String,
    pub pos: Pos,
}

/// Longest source line echoed in full.  Longer lines are shown as a window
/// around the caret, so that a pathological input (say a 10 MB line) cannot
/// flood stderr with the whole line.
const MAX_ECHO: usize = 160;

/// Longest identifier quoted verbatim in a message.  Longer names are
/// truncated, so that `unbound variable: <100 000 characters>` cannot flood
/// stderr with the user's own input.
const MAX_NAME_ECHO: usize = 64;

/// Truncate a user-supplied name for an error message.
fn truncate_name(name: &str) -> String {
    let mut chars = name.chars();
    let head: String = chars.by_ref().take(MAX_NAME_ECHO).collect();
    if chars.next().is_some() {
        format!("{head}…")
    } else {
        head
    }
}

impl Error {
    /// Render the error the way the original `displayError` does.
    pub fn render(&self, src: &str) -> String {
        let lnum = self.pos.line.to_string();
        let lpad = " ".repeat(lnum.len());
        let line = src
            .lines()
            .nth(self.pos.line.saturating_sub(1))
            .unwrap_or("");
        let (shown, col) = echo_window(line, self.pos.col);
        format!(
            "{}:{}:{}:\n{} |\n{} | {}\n{} | {}^\n{}",
            self.pos.file,
            self.pos.line,
            self.pos.col,
            lpad,
            lnum,
            shown,
            lpad,
            " ".repeat(col.saturating_sub(1)),
            self.msg
        )
    }
}

/// A window of `line` around column `col` (1-based), plus the column the caret
/// has inside that window.
fn echo_window(line: &str, col: usize) -> (String, usize) {
    let chars: Vec<char> = line.chars().collect();
    if chars.len() <= MAX_ECHO {
        return (line.to_string(), col.max(1));
    }
    let col = col.clamp(1, chars.len());
    let start = col.saturating_sub(1).saturating_sub(MAX_ECHO / 2);
    let end = (start + MAX_ECHO).min(chars.len());
    let start = end.saturating_sub(MAX_ECHO);

    let mut shown = String::new();
    let mut caret = col - start;
    if start > 0 {
        shown.push('…');
        caret += 1;
    }
    shown.extend(&chars[start..end]);
    if end < chars.len() {
        shown.push('…');
    }
    (shown, caret)
}

/// The elaboration monad: `Either (String, SourcePos)` in the original.
pub type M<T> = Result<T, Error>;

impl Ctx {
    pub fn empty(pos: Pos) -> Ctx {
        Ctx {
            lvl: Lvl::ZERO,
            names: Vec::new(),
            types: Vec::new(),
            locals: Vec::new(),
            env: Env::empty(),
            pos,
        }
    }

    pub fn with_pos(&self, pos: Pos) -> Ctx {
        Ctx {
            pos,
            ..self.clone()
        }
    }

    /// Look up a name, newest binding first.
    pub fn lookup(&self, x: &str) -> M<(Ix, Val)> {
        for (i, (y, ty)) in self.types.iter().enumerate() {
            if y == x {
                return Ok((Ix(i), ty.clone()));
            }
        }
        Err(self.error(format!("unbound variable: {}", truncate_name(x))))
    }

    /// Introduce a fresh variable of type `a`.
    pub fn bind(&self, x: Name, a: Val) -> Ctx {
        let v = Val::vvar(self.lvl);
        let local = crate::meta::LocalEntry::Bind(Rc::new(crate::eval::quote_val(
            self.lvl, &a,
        )));
        self.push_binding(x, a, v, local)
    }

    /// Introduce a definition: a new binding whose value is `t`.
    pub fn define(&self, x: Name, t: Val, a: Val) -> Ctx {
        let local = crate::meta::LocalEntry::Define {
            ty: Rc::new(crate::eval::quote_val(self.lvl, &a)),
            val: Rc::new(crate::eval::quote_val(self.lvl, &t)),
        };
        self.push_binding(x, a, t, local)
    }

    /// Extend the context with one binding, newest first.
    fn push_binding(&self, x: Name, ty: Val, value: Val, local: crate::meta::LocalEntry) -> Ctx {
        let mut names = Vec::with_capacity(self.names.len() + 1);
        names.push(x.clone());
        names.extend(self.names.iter().cloned());

        let mut types = Vec::with_capacity(self.types.len() + 1);
        types.push((x, ty));
        types.extend(self.types.iter().cloned());

        let mut locals = Vec::with_capacity(self.locals.len() + 1);
        locals.push(local);
        locals.extend(self.locals.iter().cloned());

        Ctx {
            lvl: self.lvl.inc(),
            names,
            types,
            locals,
            env: self.env.push(value),
            pos: self.pos.clone(),
        }
    }

    pub fn error(&self, msg: String) -> Error {
        Error {
            msg,
            pos: self.pos.clone(),
        }
    }

    /// All type names, newest first (used by the pretty printer).
    pub fn ty_names(&self) -> Vec<Name> {
        self.types.iter().map(|(n, _)| n.clone()).collect()
    }
}

/// Substitute in a context.  Note that `lvl` is *not* changed: substitutions
/// restrict the values, not the number of binders.  The quoted `locals` are
/// dropped: they picture the types as they were *before* the refinement.
pub fn subst_ctx(s: &Sub, ctx: &Ctx) -> Ctx {
    Ctx {
        lvl: ctx.lvl,
        names: ctx.names.clone(),
        types: ctx
            .types
            .iter()
            .map(|(x, ty)| (x.clone(), subst_val(s, ty)))
            .collect(),
        locals: ctx.locals.iter().map(|_| crate::meta::LocalEntry::Stale).collect(),
        env: subst_env(s, &ctx.env),
        pos: ctx.pos.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn error_at(col: usize) -> Error {
        Error {
            msg: "boom".to_string(),
            pos: Pos::new("<t>", 1, col),
        }
    }

    #[test]
    fn short_lines_are_echoed_verbatim() {
        let rendered = error_at(3).render("let bad : Nat = U; U");
        assert!(rendered.contains("1 | let bad : Nat = U; U"));
        assert!(rendered.contains("  |   ^"), "{rendered}");
        assert!(rendered.ends_with("boom"), "{rendered}");
    }

    #[test]
    fn long_lines_are_windowed_around_the_caret() {
        let line = "a".repeat(10_000);
        let rendered = error_at(5_000).render(&line);
        // The header still carries the exact position ...
        assert!(rendered.starts_with("<t>:1:5000:"));
        // ... but the echoed line is bounded.
        let echoed = rendered.lines().nth(2).expect("echoed line");
        assert!(
            echoed.chars().count() <= MAX_ECHO + 6,
            "echoed {} chars",
            echoed.chars().count()
        );
        // The caret line stays bounded too, and ends in a caret.
        let caret = rendered.lines().nth(3).expect("caret line");
        assert!(caret.ends_with('^'));
        assert!(
            caret.chars().count() <= MAX_ECHO + 8,
            "caret {} chars",
            caret.chars().count()
        );
    }

    #[test]
    fn missing_source_lines_are_tolerated() {
        let rendered = error_at(1).render("");
        assert!(rendered.contains("<t>:1:1:"));
        assert!(rendered.ends_with("boom"));
    }

    #[test]
    fn long_identifiers_are_truncated_in_messages() {
        let ctx = Ctx::empty(Pos::initial("<t>"));
        let short = ctx.lookup("x").unwrap_err().msg;
        assert_eq!(short, "unbound variable: x");

        let long = "a".repeat(100_000);
        let msg = ctx.lookup(&long).unwrap_err().msg;
        assert!(
            msg.chars().count() <= MAX_NAME_ECHO + 32,
            "message is {} chars",
            msg.chars().count()
        );
        assert!(msg.ends_with('…'), "truncation should be marked: {msg}");
    }
}
