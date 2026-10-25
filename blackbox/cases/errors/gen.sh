#!/usr/bin/env bash
# Generator for the cases whose sources (or sidecars) need raw bytes or are
# too long to keep as literal files.  Run once after checkout:
#
#   bash blackbox/cases/errors/gen.sh
#
# It (re)writes these cases next to itself; everything else in this folder is
# a literal file.  Idempotent.
set -eu
D="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$D"

code() { printf '%s' "$2" > "$1.dpm.code"; }
errf() { printf '%s' "$2" > "$1.dpm.err"; }
outf() { printf '%s' "$2" > "$1.dpm.out"; }

PRE1='data Nat : U where zero : Nat; suc : (n : Nat) -> Nat;'
PRE3="$PRE1
let plus : Nat -> Nat -> Nat =
  \\ m n . elim Nat (\\ _ . Nat) n (\\ _ r . suc r) m;"

# ---- encoding ---------------------------------------------------------------

# UTF-8 BOM in front of an otherwise valid program: accepted, exit 0.
{ printf '\xef\xbb\xbf'; printf 'U\n'; } > enc-bom-success.dpm
outf enc-bom-success 'U
  :
U1
'

# BOM + CRLF together (files produced by some Windows editors).
{ printf '\xef\xbb\xbf'; printf '%s\r\nsuc zero\r\n' "$PRE1"; } > enc-bom-crlf.dpm
outf enc-bom-crlf 'suc zero
  :
Nat
'

# A lone 0xFF byte: invalid UTF-8, rejected with exit 1.
printf 'U\xff\n' > enc-invalid-utf8.dpm
code enc-invalid-utf8 1
errf enc-invalid-utf8 'input is not valid UTF-8'

# Zero bytes of input.
: > enc-empty-input.dpm
code enc-empty-input 1
errf enc-empty-input '<stdin>:1:1:'

# Whitespace only (spaces, newlines, a tab).
printf '   \n\t\n' > enc-whitespace-only.dpm
code enc-whitespace-only 1
errf enc-whitespace-only '<stdin>:3:1:'

# CRLF line endings in a valid program: accepted.
printf '%s\r\nsuc zero\r\n' "$PRE1" > enc-crlf-valid.dpm
outf enc-crlf-valid 'suc zero
  :
Nat
'

# CRLF does not shift the reported position (error on line 2, col 5).
printf '%s\r\nsuc U\r\n' "$PRE1" > enc-crlf-error-pos.dpm
code enc-crlf-error-pos 1
errf enc-crlf-error-pos '<stdin>:2:5:'

# A NUL byte after a complete program is trailing garbage, not a crash.
printf 'U\x00' > enc-nul-byte.dpm
code enc-nul-byte 1
errf enc-nul-byte 'expected end of input'

# A 100 000-character identifier: exit 1 with a SHORT diagnostic; the name in
# the message is truncated to 64 characters plus an ellipsis.
{ for i in {1..100}; do printf '%s' "$(printf 'a%.0s' {1..1000})"; done; } > enc-ident-100k.dpm
code enc-ident-100k 1
errf enc-ident-100k "unbound variable: $(printf 'a%.0s' {1..64})…"

# A 401-column line with the error at the very end: the echoed source line is
# windowed (starts with an ellipsis), so stderr stays bounded.
printf '%s@' "$(printf 'q%.0s' {1..400})" > longline-late-parse-error.dpm
code longline-late-parse-error 1
errf longline-late-parse-error '1 | …'

# Two long identifiers on one line: the unbound-variable message truncates.
printf '%s %s' "$(printf 'b%.0s' {1..150})" "$(printf 'c%.0s' {1..150})" > longline-late-unbound.dpm
code longline-late-unbound 1
errf longline-late-unbound "unbound variable: $(printf 'b%.0s' {1..64})…"

# ---- positions (long-line variants) ----------------------------------------

# Error on line 2 after a >300-character first line.
printf '%s -- %s\nsuc U\n' "$PRE1" "$(printf 'c%.0s' {1..300})" > pos-long-first-line.dpm
code pos-long-first-line 1
errf pos-long-first-line '<stdin>:2:5:'

# ---- nesting-depth budget (parse::MAX_DEPTH = 512, shared) ------------------
# Measured boundaries on the pinned binary:
#   nested parens      255 ok / 256 rejected (2 units per level)
#   nested lets        510 ok / 511 rejected
#   let-wrapped lambdas 509 ok / 510 rejected (let + type + value share)
#   bare lambdas       510 parses / 511 rejected
#   flat spine         512 atoms parse / 513 rejected

n=255; printf '%sU%s\n' "$(yes '(' | head -n "$n" | tr -d '\n')" "$(yes ')' | head -n "$n" | tr -d '\n')" > depth-parens-255-ok.dpm
outf depth-parens-255-ok 'U
  :
U1
'
n=256; printf '%sU%s\n' "$(yes '(' | head -n "$n" | tr -d '\n')" "$(yes ')' | head -n "$n" | tr -d '\n')" > depth-parens-256.dpm
code depth-parens-256 1
errf depth-parens-256 'expression nesting is too deep'

for n in 510 511; do
  { for i in $(seq 1 "$n"); do printf 'let x%s : U1 = U; ' "$i"; done; printf 'U\n'; } > "depth-lets-$n.dpm"
done
outf depth-lets-510 'U
  :
U1
'
code depth-lets-511 1
errf depth-lets-511 'expression nesting is too deep'

for n in 509 510; do
  ty="U$(printf ' -> U%.0s' $(seq 1 "$n"))"
  { printf 'let f : %s = ' "$ty"; for i in $(seq 1 "$n"); do printf '\\ x%s . ' "$i"; done; printf 'x1; U1\n'; } > "depth-lambda-wrapped-$n.dpm"
done
outf depth-lambda-wrapped-509 'U1
  :
U2
'
code depth-lambda-wrapped-510 1
errf depth-lambda-wrapped-510 'expression nesting is too deep'

for n in 510 511; do
  { for i in $(seq 1 "$n"); do printf '\\ x%s . ' "$i"; done; printf 'U\n'; } > "depth-lambda-standalone-$n.dpm"
done
code depth-lambda-standalone-510 1
errf depth-lambda-standalone-510 "Can't infer type for lambda expression"
code depth-lambda-standalone-511 1
errf depth-lambda-standalone-511 'expression nesting is too deep'

# Flat application spine: 512 atoms still parse (the error is the ordinary
# unbound variable), 513 atoms exhaust the budget.
printf '%sz\n' "$(yes 'z ' | head -n 511 | tr -d '\n')" > depth-spine-512.dpm
code depth-spine-512 1
errf depth-spine-512 'unbound variable: z'
printf '%sz\n' "$(yes 'z ' | head -n 512 | tr -d '\n')" > depth-spine-513.dpm
code depth-spine-513 1
errf depth-spine-513 'expression nesting is too deep'

echo "generated cases written to $D"
