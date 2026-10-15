#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Generate the black-box probe corpus + manifest for dpm-nbe.exe."""
import json
import os
import shutil

HERE = os.path.dirname(os.path.abspath(__file__))
CORPUS = os.path.join(HERE, "corpus")

probes = []          # manifest entries
_counter = [0]


def add(cat, text, mode="nf", timeout=10.0, note="", name=None, raw=False):
    _counter[0] += 1
    pid = "P%03d" % _counter[0]
    if name:
        pid = name
    data = text if isinstance(text, bytes) else text.encode("utf-8")
    fn = os.path.join(CORPUS, pid + ".txt")
    with open(fn, "wb") as fh:
        fh.write(data)
    probes.append({
        "id": pid, "category": cat, "args": [mode],
        "stdin_file": os.path.relpath(fn, HERE),
        "timeout": timeout, "note": note,
    })
    return pid


# ---------------------------------------------------------------------------
# 1. PARSER EDGE CASES
# ---------------------------------------------------------------------------
def parser_probes():
    C = "parser"
    # --- keywords vs identifiers ---
    add(C, "let", note="keyword 'let' alone as term")
    add(C, "letx", note="identifier looking like let")
    add(C, "let x", note="let without type")
    add(C, "let x :", note="let with dangling colon")
    add(C, "let x : Nat", note="let without =")
    add(C, "let x : Nat =", note="let without value")
    add(C, "let x : Nat = zero", note="let without body/;")
    add(C, "let x : Nat = zero;", note="let with empty body")
    add(C, "let x : Nat = zero; x", note="normal let")
    add(C, "let x : Nat = zero;; x", note="double semicolon")
    add(C, "let x : Nat = zero; ; x", note="stray semicolon")
    add(C, "let x : Nat = zero; let y : Nat = x; y", note="two lets")
    add(C, "let x : Nat = zero; let x : Nat = suc x; x", note="shadowing")
    add(C, "let x : Nat = zero; let x : Nat = x; x", note="self shadow (recursive?)")
    add(C, "suc", note="bare suc, primitive not fully applied")
    add(C, "sucx", note="identifier sucx")
    add(C, "suc zero", note="suc applied, normal")
    add(C, "suc zero zero", note="suc over-applied / trailing junk")
    add(C, "suc (suc zero)", note="normal tower")
    add(C, "U", note="bare universe")
    add(C, "Ux", note="identifier Ux")
    add(C, "UU", note="UU")
    add(C, "U U", note="U applied to U")
    add(C, "U0", note="U with digit")
    add(C, "in", note="keyword-looking identifier 'in'")
    add(C, "inx", note="identifier inx")
    add(C, "let in : Nat = zero; in", note="'in' as binder name")
    add(C, "refl", note="bare refl, cannot infer")
    add(C, "reflx", note="identifier reflx")
    add(C, "Nat", note="bare Nat type")
    add(C, "Natx", note="identifier Natx")
    add(C, "NAT", note="upper-case NAT")
    add(C, "Zero", note="upper-case Zero")
    add(C, "zero", note="bare zero")
    add(C, "zerox", note="identifier zerox")
    add(C, "plus", note="bare plus")
    add(C, "plus zero", note="partially applied plus")
    add(C, "plus zero zero", note="fully applied plus")
    add(C, "plus zero zero zero", note="over-applied plus")
    add(C, "plusx zero zero", note="identifier plusx")
    add(C, "Id", note="bare Id")
    add(C, "Id Nat", note="partially applied Id")
    add(C, "Id Nat zero", note="partially applied Id 2")
    add(C, "Id Nat zero zero", note="fully applied Id")
    add(C, "Id Nat zero zero zero", note="over-applied Id")
    add(C, "Idx Nat zero zero", note="identifier Idx")
    add(C, "let Id : Nat = zero; Id", note="rebind primitive Id")
    add(C, "let suc : Nat = zero; suc zero", note="rebind primitive suc")
    add(C, "let Nat : U = U; Nat", note="rebind Nat")
    add(C, "let U : U = U; U", note="rebind U")
    add(C, "let refl : Nat = zero; refl", note="rebind refl")
    add(C, "let plus : Nat = zero; plus", note="rebind plus")
    add(C, "let zero : Nat = suc zero; zero", note="rebind zero")
    add(C, "let lambda : Nat = zero; lambda", note="identifier lambda")
    # --- underscore binders / wildcards ---
    add(C, "let f : (A : U) -> A -> A = \\A _ . _; f", note="underscore binder")
    add(C, "let f : (A : U) -> A -> A = \\A _ . A; f", note="underscore binder, A body")
    add(C, "let _ : Nat = zero; zero", note="underscore in let binder")
    add(C, "let x : Nat = zero; _", note="underscore as term")
    add(C, "_", note="bare underscore")
    add(C, "_x", note="underscore-prefixed identifier")
    add(C, "let _x : Nat = zero; _x", note="identifier starting with underscore")
    # --- lambdas ---
    add(C, "\\x . x", note="inferred lambda at top level")
    add(C, "\\ x . x", note="space after backslash")
    add(C, "\\x .", note="empty lambda body")
    add(C, "\\. x", note="no patterns before dot")
    add(C, "\\", note="bare backslash")
    add(C, "\\x", note="backslash pattern, no dot")
    add(C, "\\x. x", note="no space around dot")
    add(C, "λ x . x", note="unicode lambda")
    add(C, "λx.x", note="unicode lambda tight")
    add(C, "λ", note="bare unicode lambda")
    add(C, "λ . x", note="unicode lambda, empty patterns")
    add(C, "let f : (A : U) -> A -> A = λ A x . x; f Nat zero", note="unicode lambda ok")
    add(C, "let f : (A : U) -> A -> A = \\A x . x; f Nat zero", note="ascii lambda ok")
    add(C, "let f : Nat -> Nat = λ n . suc n; f zero", note="unicode lambda applied")
    add(C, "let f : (A : U) -> A -> A = \\A x . x; f", note="lambda as value (nf is lambda)")
    # --- refl / absurd patterns ---
    README_C = ("let C : (A : U) -> Id Nat (suc zero) zero -> A =\n"
                "  λ A (!);\n\nU\n")
    add(C, README_C, note="README absurd example, final term U")
    add(C, "let C : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!); C",
        note="absurd lambda as final term")
    add(C, "let C : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!); C", mode="type",
        note="absurd lambda as final term, type mode")
    add(C, "let C : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!); C Nat",
        note="absurd lambda partially applied")
    add(C, "let C : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!); C Nat refl",
        note="absurd lambda applied to refl")
    add(C, "let C : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!); U", note="absurd unused")
    add(C, "(!)", note="absurd pattern alone")
    add(C, "( ! )", note="spaced absurd pattern alone")
    add(C, "let f : (A : U) -> A -> A = \\A (!); f", note="absurd pattern on non-empty type")
    add(C, "let f : (A : U) -> A -> A = \\A (!); U", note="absurd pattern on non-empty type, checked")
    add(C, "let f : (A : U) -> Id Nat zero zero -> A = \\A refl . A; f Nat refl", note="refl pattern")
    add(C, "let f : (A : U) -> Id Nat zero zero -> A = \\A (!); f", note="absurd where refl expected -> panic?")
    add(C, "let f : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!); let g : Nat = zero; f",
        note="absurd reached through let")
    # --- parentheses ---
    add(C, "()", note="empty parens")
    add(C, "( )", note="empty parens with space")
    add(C, "(())", note="nested empty parens")
    add(C, "(zero)", note="parenthesised atom")
    add(C, "((zero))", note="double parenthesised atom")
    add(C, "(zero", note="unbalanced open paren")
    add(C, "zero)", note="unbalanced close paren")
    add(C, ")", note="close paren alone")
    add(C, "(", note="open paren alone")
    add(C, "()zero", note="empty parens then atom")
    add(C, "(Nat -> Nat) zero", note="type used as function")
    # --- arrows ---
    add(C, "Nat -> Nat", note="ascii arrow")
    add(C, "Nat → Nat", note="unicode arrow")
    add(C, "Nat -> Nat → Nat", note="mixed arrows")
    add(C, "(A : U) -> A -> A", note="dependent arrow")
    add(C, "(A : U) → A → A", note="dependent unicode arrow")
    add(C, "Nat ->", note="arrow with no codomain")
    add(C, "-> Nat", note="arrow with no domain")
    add(C, "Nat -> -> Nat", note="double arrow")
    add(C, "->", note="arrow alone")
    add(C, "→", note="unicode arrow alone")
    add(C, "Nat-Nat", note="hyphen between")
    add(C, "Nat - > Nat", note="spaced hyphen gt")
    add(C, "Nat > Nat", note="bare gt")
    add(C, "(A : U) -> (B : U) -> A -> B -> A", note="multi-binder type")
    add(C, "(A B : U) -> A -> B -> A", note="grouped binders")
    add(C, "(A : U)(B : U) -> A", note="juxtaposed binder groups (printer output!)")
    add(C, "let f : (A : U)(B : U) -> A -> B -> A = \\A B x y . x; f",
        note="juxtaposed binder groups in source")
    # --- comments ---
    add(C, "zero -- trailing comment", note="line comment")
    add(C, "-- only a comment", note="comment-only input")
    add(C, "--", note="bare --")
    add(C, "----", note="four dashes")
    add(C, "zero --", note="-- at very EOF")
    add(C, b"zero --", note="-- at EOF, no newline")
    add(C, "zero --)", note="-- swallows paren")
    add(C, "zero --(-)", note="-- swallows block open")
    add(C, "(--)", note="line comment starting with (")
    add(C, "{- -}", note="empty block comment alone")
    add(C, "{- comment -} zero", note="block comment then atom")
    add(C, "zero {- comment -}", note="trailing block comment")
    add(C, "zero {- unterminated", note="unterminated block comment")
    add(C, "{- unterminated", note="unterminated block comment only")
    add(C, "{- {- nested -} -} zero", note="nested block comment")
    add(C, "{- {- -} zero", note="nested block comment, unbalanced")
    add(C, "-}", note="close block comment alone")
    add(C, "zero -}", note="stray close block comment")
    add(C, "{--}", note="comment like {--}")
    add(C, "{-", note="open block comment alone")
    add(C, "{- --}", note="line comment inside block comment")
    add(C, "zero {- -} {- -} zero", note="trailing junk after comments")
    add(C, "-- zero\nzero", note="commented-out program then real term")
    # --- whitespace / encodings ---
    add(C, "let x : Nat = zero;\r\nx\r\n", note="CRLF program")
    add(C, "let x : Nat = zero;\rx", note="lone CR separator")
    add(C, "\tzero\t", note="tab padding")
    add(C, "let\tx\t:\tNat\t=\tzero\t;\tx", note="tabs as token separators")
    add(C, "\n\n\n  zero  \n\n", note="blank lines around")
    add(C, "   zero", note="leading spaces")
    add(C, "zero   ", note="trailing spaces")
    add(C, b"", note="empty input")
    add(C, b"\n", note="single newline")
    add(C, b" ", note="single space")
    add(C, "\ufeffzero", note="UTF-8 BOM then atom")
    add(C, "zero\ufeff", note="BOM after atom")
    add(C, "let \ufeffx : Nat = zero; x", note="BOM inside program")
    add(C, "zero\u00a0", note="non-breaking space after atom")
    add(C, "zero\u200b", note="zero-width space after atom")
    add(C, "zero\ufeffx", note="BOM mid-token")
    add(C, b"zero\x00", note="NUL byte after atom")
    add(C, b"\x00zero", note="NUL byte before atom")
    add(C, b"zero\xff", note="invalid UTF-8 byte")
    add(C, b"\xef\xbb\xbf", note="BOM only")
    add(C, b"\xff\xfe", note="UTF-16 LE BOM")
    add(C, "zero\n\u2028extra", note="unicode line separator")
    # --- identifiers ---
    add(C, "let α : Nat = zero; α", note="greek identifier")
    add(C, "let αβγ : Nat = zero; αβγ", note="multi greek identifier")
    add(C, "αβγ", note="unbound greek identifier")
    add(C, "let 日本 : Nat = zero; 日本", note="CJK identifier")
    add(C, "let x1 : Nat = zero; x1", note="digit-suffixed identifier")
    add(C, "123", note="digits-only token")
    add(C, "let 123 : Nat = zero; 123", note="digits-only binder")
    add(C, "1abc", note="digit-leading identifier")
    add(C, "let 1abc : Nat = zero; 1abc", note="digit-leading binder")
    add(C, "x'", note="prime identifier")
    add(C, "let x' : Nat = zero; x'", note="prime binder")
    add(C, "x?", note="question-mark identifier")
    add(C, "let x_y : Nat = zero; x_y", note="underscore inside identifier")
    add(C, "let x-y : Nat = zero; x-y", note="hyphen inside identifier")
    add(C, "let x.y : Nat = zero; x.y", note="dot inside identifier")
    add(C, "let x:y : Nat = zero; x:y", note="colon inside identifier")
    add(C, "let λx : Nat = zero; λx", note="lambda char inside identifier")
    add(C, "let x_λ : Nat = zero; x_λ", note="lambda char in identifier tail")
    add(C, "let x→y : Nat = zero; x→y", note="arrow char inside identifier")
    add(C, "let x=1 : Nat = zero; x=1", note="equals inside identifier")
    add(C, "a" * 100000, note="100k char unbound identifier", timeout=20)
    add(C, "let " + "a" * 100000 + " : Nat = zero; " + "a" * 100000,
        note="100k char binder + reference", timeout=20)
    add(C, "let " + "a" * 1000000 + " : Nat = zero; " + "a" * 1000000,
        note="1M char binder + reference", timeout=30)
    add(C, "a" * 1000000, note="1M char unbound identifier", timeout=30)
    # --- misc syntax ---
    add(C, "zero;", note="trailing semicolon")
    add(C, ";zero", note="leading semicolon")
    add(C, ";;", note="double semicolon only")
    add(C, ".", note="dot alone")
    add(C, ":", note="colon alone")
    add(C, "=", note="equals alone")
    add(C, "zero.zero", note="dot accessor")
    add(C, "let x : Nat = zero; x.", note="trailing dot")
    add(C, "\"zero\"", note="string literal")
    add(C, "'z'", note="char literal")
    add(C, "#zero", note="hash comment attempt")
    add(C, "zero # junk", note="hash mid")
    add(C, "zero % junk", note="percent")
    add(C, "zero @ junk", note="at sign")
    add(C, "zero ~ junk", note="tilde")
    add(C, "zero & junk", note="ampersand")
    add(C, "zero | junk", note="pipe")
    add(C, "zero $ junk", note="dollar")
    add(C, "zero ? junk", note="question")
    add(C, "let x : Nat = zero in x", note="ML-style 'in'")
    add(C, "let x := zero; x", note=":= assignment")
    add(C, "let x : Nat <= zero; x", note="<= assignment")
    add(C, "Nat :: Nat", note="double colon")
    add(C, "(A : U) => A", note="fat arrow")
    add(C, "forall (A : U) . A", note="forall keyword")
    add(C, "let x : Nat = zero; x x", note="apply a Nat")
    add(C, "refl refl", note="refl applied")
    add(C, "Nat Nat", note="Nat applied")
    add(C, "U U U", note="U applied twice")
    add(C, "suc refl", note="suc of refl")
    add(C, "plus U U", note="plus of U")
    add(C, "Id U U U", note="Id in U")
    add(C, "let f : Nat -> Nat = \\n . n; f f", note="self application")
    add(C, "let f : Nat -> Nat = \\n . n; let f : Nat -> Nat = f; f zero",
        note="recursive-looking shadowing")
    add(C, "let x : U = Nat; x -> x", note="type-level let then arrow")
    add(C, "let x : Nat = zero; x zero", note="apply Nat value")
    add(C, "\r\n\r\n", note="only CRLFs")
    add(C, "\x0c" "zero", note="form feed before atom")
    add(C, "\x0b" "zero", note="vertical tab before atom")


# ---------------------------------------------------------------------------
# 2. DEPTH / RESOURCE LIMITS
# ---------------------------------------------------------------------------
def depth_probes():
    C = "depth"

    def parens(n):
        return "(" * n + "zero" + ")" * n

    for n in (100, 500, 512, 513, 1000, 5000, 10000, 50000):
        add(C, parens(n), timeout=25, note="%d nested parens" % n)

    # nested parens around a real term
    for n in (100, 1000, 10000):
        add(C, "let x : Nat = " + "(" * n + "suc zero" + ")" * n + "; x",
            timeout=25, note="%d nested parens around suc zero" % n)

    # nested lambda: a chain of arrows + one flat lambda
    for n in (100, 500, 1000, 2000):
        ty = " -> ".join(["Nat"] * n)
        pats = " ".join("p%d" % i for i in range(n))
        add(C, "let f : %s = \\%s . p0; f" % (ty, pats), timeout=25,
            note="%d-arrow type, %d-pattern lambda" % (n, n))

    # nested application with parens (right-nested)
    for n in (100, 500, 1000, 5000, 10000):
        body = "f " * n + "zero"
        add(C, "let f : Nat -> Nat = \\n . n; " + body, timeout=25,
            note="%d nested f(...) applications (flat)" % n)

    for n in (100, 500, 1000, 5000, 10000):
        body = "zero"
        for _ in range(n):
            body = "f (" + body + ")"
        add(C, "let f : Nat -> Nat = \\n . n; " + body, timeout=25,
            note="%d nested f(...) applications (parenthesised)" % n)

    # suc towers (parenthesised)
    for n in (100, 1000, 10000, 100000):
        body = "zero"
        for _ in range(n):
            body = "suc (" + body + ")"
        add(C, body, timeout=25, note="suc tower depth %d" % n)

    # plus towers
    for n in (50, 200, 1000, 5000):
        body = "zero"
        for _ in range(n):
            body = "plus zero (" + body + ")"
        add(C, body, timeout=25, note="plus tower depth %d" % n)

    # Id nesting
    for n in (50, 200, 1000):
        ty = "Nat"
        for _ in range(n):
            ty = "Id (" + ty + ") refl refl"
        add(C, "let x : %s = refl; x" % ty, timeout=25, note="%d nested Id types" % n)

    # arrow chains
    for n in (100, 500, 1000, 5000, 10000, 50000):
        add(C, " -> ".join(["Nat"] * n), timeout=25, note="%d-arrow chain" % n)

    # arrow chain, nullary-ish final
    for n in (1000, 10000):
        add(C, "let f : " + " -> ".join(["Nat"] * n) + " = \\p . p; f", timeout=25,
            note="%d-arrow chain bound to 1-pattern lambda (ill-typed, parse depth)" % n)

    # deep let nesting / chaining
    for n in (100, 1000, 5000):
        src = "".join("let x%d : Nat = zero; " % i for i in range(n)) + "x0"
        add(C, src, timeout=25, note="%d sequential lets" % n)

    for n in (100, 1000, 5000):
        src = "".join("let x : Nat = suc x; " % () for _ in range(n)) + "x"
        add(C, src, timeout=25, note="%d self-shadowing lets" % n)

    # deep shadowing with distinct bodies
    for n in (1000, 10000):
        src = "".join("let x : Nat = suc zero; " for _ in range(n)) + "x"
        add(C, src, timeout=25, note="%d shadowing lets of same name" % n)

    # deeply nested parentheses in type position
    for n in (1000, 10000):
        add(C, "let x : " + "(" * n + "Nat" + ")" * n + " = zero; x", timeout=25,
            note="%d nested parens in type position" % n)

    # long flat application of a primitive
    for n in (1000, 10000):
        add(C, "plus " + " ".join(["zero"] * 2) + " " + " ".join(["zero"] * n),
            timeout=25, note="plus followed by %d extra atoms" % n)

    # very deep let-bound lambda nesting
    for n in (100, 500):
        body = "x"
        for _ in range(n):
            body = "(\\y . " + body + ")"
        add(C, "let x : Nat = zero; " + body, timeout=25,
            note="%d nested parenthesised lambdas" % n)


# ---------------------------------------------------------------------------
# 3. SEMANTIC STRESS
# ---------------------------------------------------------------------------
def sem_probes():
    C = "semantic"

    def sucs(n):
        out = ["zero"]
        for _ in range(n):
            out.insert(0, "suc (")
            out.append(")")
        return "".join(out)

    # big plus computations
    for a, b in ((100, 100), (1000, 1000), (5000, 5000)):
        add(C, "plus (%s) (%s)" % (sucs(a), sucs(b)), timeout=25,
            note="plus %d %d" % (a, b))

    for n in (1000, 10000, 100000):
        add(C, "plus (%s) zero" % sucs(n), timeout=25, note="plus %d zero" % n)

    for n in (100000, 1000000):
        add(C, sucs(n), timeout=30, note="suc tower %d as normal form" % n)

    # plus associativity shapes
    add(C, "plus (plus (suc zero) (suc zero)) (plus (suc zero) (suc zero))", note="nested plus")
    add(C, "plus (suc (suc (suc zero))) (suc (suc zero))", note="plus 3 2")
    add(C, "plus zero (plus zero (plus zero (suc zero)))", note="plus 0 0 0 1")
    add(C, "let n : Nat = suc (suc zero); plus n n", note="plus of let-bound n")

    # exponential blowup via doubling chain: d (d (d ... zero))
    for k in (10, 16, 20, 22, 24, 26):
        body = "zero"
        for _ in range(k):
            body = "d (" + body + ")"
        add(C, "let d : Nat -> Nat = \\n . plus n n; " + body, timeout=20,
            note="doubling chain k=%d (2^%d)" % (k, k))

    # transN: chained trans
    trans_decl = ("let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> Id A a c =\n"
                  "  \u03bb A a b c refl refl . refl;\n")
    for n in (1, 2, 3, 5, 8, 12, 20):
        body = "refl"
        for _ in range(n):
            body = "trans Nat zero zero zero refl (" + body + ")"
        add(C, trans_decl + body, timeout=25, note="chained trans n=%d" % n)

    # trans applied with variable arguments
    for n in (4, 12):
        body = "refl"
        for _ in range(n):
            body = "trans Nat a b c refl (" + body + ")"
        add(C, trans_decl + "let a : Nat = zero; let b : Nat = zero; let c : Nat = zero; " + body,
            timeout=25, note="chained trans with variables n=%d" % n)

    # README H unification example
    add(C, ("let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero =\n"
            "  \u03bb m n refl refl . refl;\n"
            "H"), timeout=25, note="README H example as final term (nf is lambda)")
    add(C, ("let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero =\n"
            "  \u03bb m n refl refl . refl;\n"
            "U"), timeout=25, note="README H example, final term U")

    # big lambda normal forms
    for n in (100, 1000, 5000):
        pats = " ".join("p%d" % i for i in range(n))
        ty = " -> ".join(["Nat"] * n)
        add(C, "let f : %s = \\%s . p0; f" % (ty, pats), timeout=25,
            note="lambda with %d binders as nf" % n)

    # many simultaneous lets used at once
    for n in (100, 1000):
        decls = "".join("let v%d : Nat = suc zero; " % i for i in range(n))
        add(C, decls + "plus v0 (plus v%d zero)" % (n - 1), timeout=25,
            note="%d lets all in scope" % n)

    # deep let values depending on previous
    for n in (100, 1000):
        decls = "let v0 : Nat = zero; " + "".join(
            "let v%d : Nat = suc v%d; " % (i, i - 1) for i in range(1, n))
        add(C, decls + "v%d" % (n - 1), timeout=25, note="chain of %d dependent lets" % n)

    # pathological conversion checks
    add(C, "let f : (n : Nat) -> Id Nat (plus n zero) n = \\n . refl; f", timeout=25,
        note="plus n zero = n definitional")
    add(C, "let f : (n : Nat) -> Id Nat (plus zero n) n = \\n . refl; f", timeout=25,
        note="plus zero n = n definitional")
    add(C, "let f : (n : Nat) -> Id Nat (plus (suc n) zero) (suc n) = \\n . refl; f", timeout=25,
        note="plus (suc n) zero conversion")
    add(C, "let f : Id Nat (plus (suc zero) (suc zero)) (suc (suc zero)) = refl; f", timeout=25,
        note="plus computes in conversion")
    add(C, "let f : (A : U) (a b : A) -> Id A a b -> Id A b a = \\A a b refl . refl; f", timeout=25,
        note="sym-like with refl patterns")
    add(C, "let f : (A : U) (a : A) -> Id A a a = \\A a . refl; f", timeout=25, note="refl value")
    add(C, "let f : (A : U) (B : A -> U) (x y : A) -> Id A x y -> Id (B x) (B x); f" if False else
        "let f : (A : U) (B : A -> U) (x : A) -> Id (B x) (B x) = \\A B x . refl; f", timeout=25,
        note="dependent refl")
    add(C, "let f : (A : U) -> Id U A A = \\A . refl; f", timeout=25, note="refl in U")
    add(C, "let f : Id U Nat Nat = refl; f", timeout=25, note="refl of Nat at U")
    add(C, "Id U Nat Nat", timeout=25, note="Id U Nat Nat as type")
    add(C, "let f : (A : U) -> (B : U) -> Id U A B -> U = \\A B (!); f", timeout=25,
        note="absurd on Id U A B")
    add(C, "let bad : (A : U) -> Id Nat (suc zero) zero -> A = \\A refl . A; bad", timeout=25,
        note="impossible refl pattern (should be type error)")

    # huge normal form by repeated application
    add(C, "let f : Nat -> Nat = \\n . plus n n; f (f (f (f (f (f (f (f zero)))))))", timeout=25,
        note="doubling 8 -> 256")
    add(C, "let f : Nat -> Nat = \\n . plus n (suc zero); f (f (f (f (f zero))))", timeout=25,
        note="increment chain")
    # term whose type is huge
    add(C, "let f : " + " -> ".join(["Nat"] * 2000) + " = \\%s . p0; f" % " ".join(
        "p%d" % i for i in range(2000)), timeout=25, note="2000-arrow type in output")


# ---------------------------------------------------------------------------
# 4. CLI
# ---------------------------------------------------------------------------
def cli_probes():
    C = "cli"

    def addc(pid, args, text="zero", timeout=10, note=""):
        _counter[0] += 1
        data = text if isinstance(text, bytes) else text.encode("utf-8")
        fn = os.path.join(CORPUS, pid + ".txt")
        with open(fn, "wb") as fh:
            fh.write(data)
        probes.append({"id": pid, "category": C, "args": args,
                       "stdin_file": os.path.relpath(fn, HERE),
                       "timeout": timeout, "note": note})

    addc("C001", [], "zero", note="no args")
    addc("C002", [], "", note="no args, empty stdin")
    addc("C003", ["--help"], "zero", note="--help with stdin")
    addc("C004", ["--help"], "", note="--help with empty stdin")
    addc("C005", ["-h"], "zero", note="-h short help")
    addc("C006", ["help"], "zero", note="help word")
    addc("C007", ["--version"], "zero", note="--version")
    addc("C008", ["-V"], "zero", note="-V")
    addc("C009", ["nf"], "zero", note="nf normal")
    addc("C010", ["type"], "zero", note="type normal")
    addc("C011", ["NF"], "zero", note="uppercase NF")
    addc("C012", ["Type"], "zero", note="capitalised Type")
    addc("C013", ["nf", "extra"], "zero", note="nf plus extra arg")
    addc("C014", ["nf", "type"], "zero", note="nf and type")
    addc("C015", ["type", "nf"], "zero", note="type and nf")
    addc("C016", ["--help", "nf"], "zero", note="--help then nf")
    addc("C017", ["nf", "--help"], "zero", note="nf then --help")
    addc("C018", [""], "zero", note="empty argument")
    addc("C019", [" "], "zero", note="space argument")
    addc("C020", ["nf "], "zero", note="trailing space in mode")
    addc("C021", [" nf"], "zero", note="leading space in mode")
    addc("C022", ["\u03bb"], "zero", note="unicode lambda argument")
    addc("C023", ["--"], "zero", note="double dash")
    addc("C024", ["-"], "zero", note="single dash (stdin?)")
    addc("C025", ["-x"], "zero", note="unknown short flag")
    addc("C026", ["--nf"], "zero", note="--nf long flag")
    addc("C027", ["nf", "nf", "nf"], "zero", note="nf repeated")
    addc("C028", ["--help", "--help"], "zero", note="--help twice")
    addc("C029", ["--HELP"], "zero", note="--HELP uppercase")
    addc("C030", ["--help=1"], "zero", note="--help=1")
    addc("C031", ["type"], "let bad : Nat = U; U", note="type error exit code")
    addc("C032", ["nf"], "let bad : Nat = U; U", note="type error in nf")
    addc("C033", ["nf"], "suc", note="parse error exit code")
    addc("C034", ["type"], "suc", note="parse error in type mode")
    addc("C035", ["nf"], "zero) ", note="parse error, stdout must stay clean")
    addc("C036", ["--help"], "x" * 5000000, note="--help with 5MB stdin (does it read stdin?)",
         timeout=15)
    addc("C037", ["type"], "plus (suc zero) (suc zero)", note="type of a computation")
    addc("C038", ["nf"], "plus (suc zero) (suc zero)", note="nf of a computation")
    addc("C039", ["Nf"], "zero", note="mixed-case Nf")
    addc("C040", ["nf\n"], "zero", note="mode with embedded newline")
    addc("C041", ["nf\t"], "zero", note="mode with embedded tab")
    addc("C042", ["nf", "-h"], "zero", note="nf then -h")
    addc("C043", ["--help", "-h"], "zero", note="--help and -h")
    addc("C044", ["h"], "zero", note="h alone")
    addc("C045", ["nf"], b"", note="nf with empty stdin -> error exit")
    addc("C046", ["type"], b"", note="type with empty stdin -> error exit")
    addc("C047", ["nf"], b"zero\xff", note="nf invalid utf8 exit code")
    addc("C048", ["type"], b"zero\xff", note="type invalid utf8 exit code")
    addc("C049", ["--help"], b"zero\xff", note="--help invalid utf8")
    addc("C050", ["nf"], b"\x00" * 1000, note="1000 NUL bytes")
    addc("C051", ["nf"], b" " * 10000000, note="10MB of spaces", timeout=25)
    addc("C052", ["nf"], "let x : Nat = zero; x\n" * 200000, note="200k lets", timeout=25)


# ---------------------------------------------------------------------------
# 5. BLOWUP (deep values produced by small programs) + REGRESSION probes
# ---------------------------------------------------------------------------
def blowup_probes():
    C = "blowup"

    def sucs(n):
        out = ["zero"]
        for _ in range(n):
            out.insert(0, "suc (")
            out.append(")")
        return "".join(out)

    def double(k, base="suc zero"):
        body = base
        for _ in range(k):
            body = "d (" + body + ")"
        return "let d : Nat -> Nat = \\n . plus n n; " + body

    for k in (10, 12, 14, 16):
        add(C, double(k), timeout=60, note="doubling chain k=%d -> 2^%d succs" % (k, k))
    add(C, double(17), timeout=120,
        note="doubling k=17 -> 2^17 succs (suspected stack overflow)")
    add(C, double(18), timeout=120, note="doubling k=18 -> 2^18 succs")

    # deep conversion: compare two large towers
    add(C, "let x : Id Nat (%s) (%s) = refl; x" % (sucs(400), sucs(400)), timeout=60,
        note="refl between two 400-deep towers")
    add(C, "let d : Nat -> Nat = \\n . plus n n; "
           "let a : Nat = %s; let b : Nat = %s; let x : Id Nat a b = refl; x" % (
               double_body(10), double_body(10)), timeout=60,
        note="refl between two computed 2^10 towers")

    # huge printing only
    add(C, sucs(500), timeout=60, note="500-deep suc tower (max under depth limit)")

    # many simultaneous large values
    add(C, "let d : Nat -> Nat = \\n . plus n n; " + "".join(
        "let v%d : Nat = %s; " % (i, double_body(8)) for i in range(20)) + "v19",
        timeout=60, note="20 x 2^8 towers in scope")


def double_body(k, base="suc zero"):
    body = base
    for _ in range(k):
        body = "d (" + body + ")"
    return body


def regress_probes():
    C = "regress"
    # each of these corresponds to a headline finding; kept in the corpus so the
    # raw artefacts contain the minimal reproducers.
    add(C, "let C : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!); C",
        note="PANIC: absurd-lambda value in nf (rc 101)")
    add(C, "let C : Nat -> Id Nat (suc zero) zero -> Nat = \\n (!); C",
        note="PANIC: minimal absurd lambda, nf")
    add(C, "let C : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!); "
           "let x : Id ((A : U) -> Id Nat (suc zero) zero -> A) C C = refl; x", mode="type",
        note="PANIC: same in type mode")
    add(C, "let f : (n m : Nat) -> Id Nat (plus n m) (plus n m) = \\n m . refl; f",
        note="CONVERSION BUG: identical neutral types judged unequal")
    add(C, "let f : (n m : Nat) -> Id Nat (plus (suc n) m) (suc (plus n m)) = \\n m . refl; f",
        note="CONVERSION BUG: self-contradictory error message")
    add(C, "let k : (n m : Nat) (p : Id Nat (plus n m) (plus n m)) -> "
           "Id Nat (plus n m) (plus n m) = \\n m p . p; k",
        note="CONVERSION BUG: rejects a variable checked against its own type")
    add(C, "suc (suc zero)", note="ROUNDTRIP: printed nf says 'succ', parser wants 'suc'")
    add(C, "(" * 512 + "zero" + ")" * 512,
        note="DEPTH: 512 parens -> wrong message \"'zero' is a keyword\"")
    add(C, "(" * 513 + "zero" + ")" * 513, timeout=20,
        note="DEPTH: 513 parens -> 'expression nesting is too deep'")
    add(C, "zero" + "$" * 1000000, timeout=60,
        note="ECHO: 1MB line echoed into stderr on error")
    add(C, "\u03b1" * 2000 + " )", timeout=20,
        note="UTF8: long multibyte line, error rendering")
    add(C, "let x : Nat = zero; _", note="'_' is neither binder nor term")
    add(C, "zero {- unterminated", note="unterminated {- accepted silently")
    add(C, "\ufeffzero", note="UTF-8 BOM rejected")
    add(C, b"zero\xff", note="invalid UTF-8 -> exit code 2")
    add(C, "let suc : Nat = zero; suc", note="keyword rebinding rejected")
    add(C, "let succ : Nat = suc zero; succ", note="'succ' IS a legal identifier (printer name)")
    add(C, "let f : (A : U) -> A -> A = \\A _ . _; f", note="underscore binder error")
    add(C, "let f : Nat -> Nat -> Nat = \\p0 p1 p2 . p0; f",
        note="more patterns than arrows -> 'Too many patterns.'")
    add(C, "let f : (p0 p1 p2 : Nat) -> Nat = \\p0 p1 p2 . p0; f",
        note="grouped binder, 3 patterns -> OK")
    add(C, "let f : (A : U) -> Id A zero zero -> U = \\A refl . A; f",
        note="refl pattern, declared type checked under refinement -> error at 'zero'")
    add(C, "let f : (x : Nat) -> Id Nat (plus x zero) (suc x) -> Nat = \\x (!); f",
        note="absurd pattern on open contradiction -> conservative error (good)")
    add(C, "let f : (n m : Nat) -> Id Nat (plus (suc n) m) (suc (plus n m)) = \\n m . refl; f",
        mode="type", note="conversion bug also in type mode")


def main():
    if os.path.isdir(CORPUS):
        shutil.rmtree(CORPUS)
    os.makedirs(CORPUS)
    parser_probes()
    depth_probes()
    sem_probes()
    cli_probes()
    blowup_probes()
    regress_probes()
    with open(os.path.join(HERE, "manifest.json"), "w", encoding="utf-8") as fh:
        json.dump(probes, fh, ensure_ascii=False, indent=1)
    cats = {}
    for p in probes:
        cats[p["category"]] = cats.get(p["category"], 0) + 1
    print("total probes:", len(probes), cats)


if __name__ == "__main__":
    main()
