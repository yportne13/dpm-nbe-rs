#!/usr/bin/env bash
# Black-box test runner for the dpm-nbe CLI.
#
# Usage:  blackbox/run.sh [case-dir]     (default: every case under blackbox/cases)
#
# A *case* is a source file `NAME.dpm` plus optional sidecar files:
#   NAME.dpm.mode    "nf" (default), "type" or "prune"
#   NAME.dpm.out     expected stdout, trailing whitespace ignored (missing = expect empty stdout)
#   NAME.dpm.err     a fragment that must appear in stderr (missing = stderr unchecked)
#   NAME.dpm.code    expected exit status (default 0)
#   NAME.dpm.timeout per-case timeout in seconds (default 10; needs GNU timeout)
#
# Rename a case to NAME.dpm.skip (sidecars keep their names) to skip it --
# that is how documented, still-open bugs are parked.
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$ROOT/target/release/dpm-nbe.exe"
[ -x "$BIN" ] || BIN="$ROOT/target/release/dpm-nbe"
if [ ! -x "$BIN" ]; then
  echo "run.sh: binary not found: $BIN (build it with: cargo build --release)" >&2
  exit 2
fi
CASES="${1:-$ROOT/blackbox/cases}"
LOGS="$ROOT/blackbox/logs"
mkdir -p "$LOGS"
LOG="$LOGS/run-$(date +%Y%m%d-%H%M%S).log"
BINHASH="$(sha256sum "$BIN" | cut -c1-16)"
pass=0; fail=0; skip=0
failed_names=""

while IFS= read -r src; do
  name="${src#"$CASES"/}"
  case "$src" in
    *.dpm.skip)
      skip=$((skip + 1))
      echo "SKIP  $name" | tee -a "$LOG"
      continue
      ;;
  esac
  mode="nf";   [ -f "$src.mode" ]    && mode="$(tr -d '[:space:]' < "$src.mode")"
  want_code=0; [ -f "$src.code" ]    && want_code="$(tr -d '[:space:]' < "$src.code")"
  tmo=10;      [ -f "$src.timeout" ] && tmo="$(tr -d '[:space:]' < "$src.timeout")"
  out="$(mktemp)"; err="$(mktemp)"
  if command -v timeout >/dev/null 2>&1; then
    timeout "$tmo" "$BIN" "$mode" < "$src" > "$out" 2> "$err"; code=$?
  else
    "$BIN" "$mode" < "$src" > "$out" 2> "$err"; code=$?
  fi
  why=""
  [ "$code" = "$want_code" ] || why="exit $code, want $want_code"
  if [ -f "$src.out" ]; then
    [ "$(cat "$out")" = "$(cat "$src.out")" ] || why="${why:+$why; }stdout mismatch"
  else
    [ -s "$out" ] && why="${why:+$why; }unexpected stdout"
  fi
  if [ -f "$src.err" ] && ! grep -F -q -- "$(cat "$src.err")" "$err"; then
    why="${why:+$why; }stderr fragment missing"
  fi
  if grep -q "internal panic" "$err"; then
    why="${why:+$why; }INTERNAL PANIC"
  fi
  if [ -n "$why" ]; then
    fail=$((fail + 1)); failed_names="$failed_names $name"
    { echo "FAIL  $name  ($why)"
      sed 's/^/      | /' "$err" | head -12; } | tee -a "$LOG"
  else
    pass=$((pass + 1))
    echo "PASS  $name" >> "$LOG"
  fi
  rm -f "$out" "$err"
done < <(find "$CASES" \( -type f -name '*.dpm' -o -type f -name '*.dpm.skip' \) 2>/dev/null | sort)

{ echo
  echo "binary: $BIN (sha256 $BINHASH)"
  echo "cases:  $pass passed, $fail failed, $skip skipped"
} | tee -a "$LOG"
if [ "$fail" -ne 0 ]; then
  printf 'failed:%s\n' "$failed_names"
  exit 1
fi
exit 0
