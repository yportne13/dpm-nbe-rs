#!/usr/bin/env node
// Randomized black-box fuzz harness for the dpm-nbe CLI.
//
// INVARIANT under test: `target/release/dpm-nbe.exe nf|type` reads a program
// from stdin and must always terminate on its own with exit status 0 (ok),
// 1 (user error) or 2 (CLI/IO misuse).  ANY other exit status — in particular
// 101 — or stderr containing `internal panic`, or a hang, is a bug.
//
// Usage:
//   node blackbox/fuzz.mjs [--n=2000] [--seed=20261025] [--timeout=10000]
//
// Determinism: the only randomness is a mulberry32 PRNG seeded with --seed
// (default pinned below).  For a fixed seed and n, the byte-for-byte input
// sequence, the nf/type alternation, and the pass/fail pattern are identical
// across runs.  Inputs that violate the invariant are saved under
// blackbox/fuzz-corpus/<sha256>.dpm for direct reproduction:
//   target/release/dpm-nbe.exe nf < blackbox/fuzz-corpus/<hash>.dpm
//
// Generators (round-robin, i % 4):
//   tokenSoup   random tokens from the language vocabulary, with occasional
//               deep single-token runs sized to straddle the parser limits
//   mutation    byte-level mutations (flip/insert/delete/splice/truncate/
//               duplicate) of embedded valid seed programs
//   structural  balanced-ish nesting of parens/lambdas/Pis/lets/eliminators
//               with random tokens at leaves, sizes straddling depth limits
//               (parens 255/256, lambdas ~507, lets ~507, digits 254/255)
//   byteNoise   random bytes, random UTF-8, invalid UTF-8, BOM, CRLF mixes,
//               NUL bytes
//
// Exit status of this script: 0 iff no violation was found, 1 otherwise.

import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const BIN = join(ROOT, "target", "release", "dpm-nbe.exe");
const CORPUS_DIR = join(ROOT, "blackbox", "fuzz-corpus");

const DEFAULT_N = 2000;
const DEFAULT_SEED = 20261025;
const DEFAULT_TIMEOUT_MS = 10_000;
const NEAR_MISS_MS = 2_000;
const MAX_INPUT_BYTES = 200 * 1024;
const LAM = "\u03bb"; // λ

// --------------------------------------------------------------------------
// CLI flags
// --------------------------------------------------------------------------

function parseArgs(argv) {
  const out = { n: DEFAULT_N, seed: DEFAULT_SEED, timeout: DEFAULT_TIMEOUT_MS };
  for (const a of argv) {
    const m = /^--(n|seed|timeout)=(.+)$/.exec(a);
    if (!m) throw new Error(`unknown flag: ${a} (expected --n=, --seed=, --timeout=)`);
    const v = Number(m[2]);
    if (!Number.isInteger(v) || v <= 0) throw new Error(`bad value for --${m[1]}: ${m[2]}`);
    out[m[1]] = v;
  }
  return out;
}

// --------------------------------------------------------------------------
// Seeded PRNG (mulberry32) and helpers
// --------------------------------------------------------------------------

function mulberry32(seed) {
  let a = seed >>> 0;
  return function () {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const randInt = (rng, lo, hi) => lo + Math.floor(rng() * (hi - lo + 1));
const pick = (rng, arr) => arr[Math.floor(rng() * arr.length)];
const chance = (rng, p) => rng() < p;

// --------------------------------------------------------------------------
// Vocabulary and valid seed programs
// --------------------------------------------------------------------------

const VOCAB = [
  "let", "in", "data", "struct", "where", "elim", "U", "U1", "U42", "Id",
  "refl", LAM, "\\", ".", ";", ":", "->", "(", ")", "_", "(!)", "=",
  "Nat", "zero", "suc", "mkPr", "mkPair", "x", "y", "a0", "a1", "foo",
];

const IDENTS = ["x", "y", "z", "foo", "bar_1", "a0", "a9", "nat2", "getS", "X", "List", "T_"];

const SMALL_TOKENS = ["U", "U1", "U42", "zero", "suc", "refl", "x", "y", "0", "1", "42", "255", "_"];

// Valid programs (from blackbox/cases/lang) used as mutation bases.  Two use
// λ so byte flips can produce invalid UTF-8 as well as garbage tokens.
const VALID_PROGRAMS = [
  `data Nat : U where zero : Nat; suc : Nat -> Nat;
let plus : Nat -> Nat -> Nat =
  ${LAM} a b . elim Nat (${LAM} _ . Nat) b (${LAM} n r . suc r) a;
plus (suc (suc (suc zero))) (suc (suc (suc (suc zero))))`,
  `data Bool : U where true : Bool; false : Bool;
let idb : Bool -> Bool = elim Bool (${LAM} _ . Bool) true false;
idb`,
  `let p : Id U1 U U = refl;
p`,
  `data Nat : U where zero : Nat; suc : Nat -> Nat;
struct Pr (A : U) (B : U) : U where left : A; right : B;
let getS : (A : U) (B : U) -> Pr A B -> B = \\ A B p . p.right;
getS Nat Nat (mkPr Nat Nat (suc zero) (suc (suc zero)))`,
  `data Nat : U where zero : Nat; suc : Nat -> Nat;
let idn : Nat -> Nat = \\ n . n;
let two : Nat = suc (suc zero);
elim Nat (\\ _ . Nat) zero (\\ n r . suc r) (idn two)`,
].map((text, i) => ({ name: `seed${i}`, buf: Buffer.from(text, "utf8") }));

const PRELUDE = `data Nat : U where zero : Nat; suc : Nat -> Nat;\n`;

// --------------------------------------------------------------------------
// Generator A: token soup
// --------------------------------------------------------------------------

function genTokenSoup(rng) {
  const parts = [];
  const n = chance(rng, 0.2) ? randInt(rng, 40, 400) : randInt(rng, 1, 40);
  for (let i = 0; i < n; i++) {
    // Occasional deep single-token runs to straddle the parser depth limits.
    if (chance(rng, 0.03)) {
      const run = pick(rng, ["(", ")", ";", LAM, ".", "\n", "0"]);
      parts.push(run.repeat(randInt(rng, 1, 700)));
      continue;
    }
    const r = rng();
    if (r < 0.7) parts.push(pick(rng, VOCAB));
    else if (r < 0.85)
      parts.push(pick(rng, IDENTS) + String(randInt(rng, 0, 99)));
    else if (r < 0.97) parts.push(String(randInt(rng, 0, 255)));
    else parts.push("\n");
  }
  const seps = [" ", " ", " ", "\n", "  ", ""];
  return Buffer.from(parts.flatMap((p, i) => (i === 0 ? [p] : [pick(rng, seps), p])).join(""), "utf8");
}

// --------------------------------------------------------------------------
// Generator B: byte-level mutations of valid programs
// --------------------------------------------------------------------------

function mutateBytes(rng, buf) {
  let b = Buffer.from(buf);
  const rounds = 1 + randInt(rng, 0, 7);
  for (let i = 0; i < rounds; i++) {
    const kind = randInt(rng, 0, 5);
    if (b.length === 0) break;
    if (kind === 0) {
      // flip a byte
      const pos = randInt(rng, 0, b.length - 1);
      b[pos] = randInt(rng, 0, 255);
    } else if (kind === 1) {
      // insert 1..16 random bytes (sometimes vocabulary text)
      const ins = chance(rng, 0.5)
        ? Buffer.from(pick(rng, VOCAB) + pick(rng, VOCAB), "utf8")
        : Buffer.from(Array.from({ length: randInt(rng, 1, 16) }, () => randInt(rng, 0, 255)));
      const pos = randInt(rng, 0, b.length);
      b = Buffer.concat([b.subarray(0, pos), ins, b.subarray(pos)]);
    } else if (kind === 2) {
      // delete 1..16 bytes
      const pos = randInt(rng, 0, b.length - 1);
      const end = Math.min(b.length, pos + randInt(rng, 1, 16));
      b = Buffer.concat([b.subarray(0, pos), b.subarray(end)]);
    } else if (kind === 3) {
      // splice a fragment of another valid program
      const other = pick(rng, VALID_PROGRAMS).buf;
      const s = randInt(rng, 0, other.length - 1);
      const e = Math.min(other.length, s + randInt(rng, 1, 60));
      const pos = randInt(rng, 0, b.length);
      b = Buffer.concat([b.subarray(0, pos), other.subarray(s, e), b.subarray(pos)]);
    } else if (kind === 4) {
      // truncate
      b = b.subarray(0, randInt(rng, 0, b.length));
    } else {
      // duplicate a region
      const s = randInt(rng, 0, b.length - 1);
      const e = Math.min(b.length, s + randInt(rng, 1, 80));
      const pos = randInt(rng, 0, b.length);
      const dup = b.subarray(s, e);
      if (b.length + dup.length <= MAX_INPUT_BYTES / 8)
        b = Buffer.concat([b.subarray(0, pos), dup, b.subarray(pos)]);
    }
    if (b.length > MAX_INPUT_BYTES) b = b.subarray(0, MAX_INPUT_BYTES);
  }
  return b;
}

// --------------------------------------------------------------------------
// Generator C: structural nesting straddling the depth limits
// --------------------------------------------------------------------------

function genStructural(rng) {
  const kind = pick(rng, ["tree", "parens", "lamnest", "letchain", "numlit", "pinest"]);
  // Half the time aim small, half the time aim across a measured limit.
  const straddle = (lo, hi, loS, hiS) => (chance(rng, 0.5) ? randInt(rng, lo, hi) : randInt(rng, loS, hiS));

  if (kind === "parens") {
    const k = straddle(1, 60, 230, 300);
    const leaf = pick(rng, ["U", "U1", "zero", "refl", "x", "42", "suc zero", ""]);
    return Buffer.from("(".repeat(k) + leaf + ")".repeat(k), "utf8");
  }
  if (kind === "lamnest") {
    const k = straddle(1, 40, 460, 570);
    const binders = Array.from({ length: k }, (_, i) => `${LAM} x${i} . `).join("");
    const body = pick(rng, ["zero", "U", "x0", "suc zero"]);
    return Buffer.from(
      PRELUDE + `let g : U -> U = ${binders}${body};\ng\n`,
      "utf8"
    );
  }
  if (kind === "letchain") {
    const k = straddle(1, 40, 460, 570);
    const lets = Array.from({ length: k }, (_, i) =>
      i === 0 ? "let a0 : Nat = zero; " : `let a${i} : Nat = a${i - 1}; `
    ).join("");
    return Buffer.from(PRELUDE + lets + `a${k - 1}\n`, "utf8");
  }
  if (kind === "numlit") {
    const k = straddle(1, 30, 180, 330);
    const digits = Array.from({ length: k }, () => String(randInt(rng, 0, 9))).join("");
    return Buffer.from(digits, "utf8");
  }
  if (kind === "pinest") {
    const k = straddle(1, 60, 230, 300);
    const pis = Array.from({ length: k }, (_, i) => `(x${i} : U) -> `).join("");
    return Buffer.from(pis + "U", "utf8");
  }
  // tree: balanced-ish random nesting with random tokens at leaves
  const budget = chance(rng, 0.5) ? randInt(rng, 1, 80) : randInt(rng, 280, 600);
  const out = [];
  let outLen = 0;
  const emit = (s) => {
    out.push(s);
    outLen += s.length;
  };
  function node(depth) {
    if (depth <= 0 || outLen > 60_000) {
      emit(pick(rng, SMALL_TOKENS));
      return;
    }
    const r = rng();
    if (r < 0.25) {
      emit("(");
      node(depth - 1);
      emit(")");
    } else if (r < 0.45) {
      emit(`${LAM} ${pick(rng, ["x", "y", "_"])} . `);
      node(depth - 1);
    } else if (r < 0.6) {
      emit(`(${pick(rng, ["x", "y", "_"])} : `);
      node(depth - 1);
      emit(") -> ");
      node(depth - 1);
    } else if (r < 0.75) {
      emit(`let ${pick(rng, IDENTS)} : U = `);
      node(depth - 1);
      emit("; ");
      node(depth - 1);
    } else if (r < 0.9) {
      emit("elim ");
      node(depth - 1);
      emit(` (${LAM} _ . `);
      node(depth - 1);
      emit(") ");
      node(depth - 1);
      emit(" ");
      node(depth - 1);
    } else {
      // application spine of 2-4 shallow children
      const count = randInt(rng, 2, 4);
      for (let i = 0; i < count; i++) {
        if (i > 0) emit(" ");
        node(depth - 1);
      }
    }
  }
  node(budget);
  const text = out.join("");
  return Buffer.from(text.length > MAX_INPUT_BYTES ? text.slice(0, MAX_INPUT_BYTES) : text, "utf8");
}

// --------------------------------------------------------------------------
// Generator D: byte-level noise
// --------------------------------------------------------------------------

const BAD_UTF8 = [
  Buffer.from([0x80]),                          // lone continuation
  Buffer.from([0xc0, 0xaf]),                    // overlong '/'
  Buffer.from([0xe0, 0xa0]),                    // truncated 3-byte
  Buffer.from([0xf5, 0x80, 0x80, 0x80]),        // out of range
  Buffer.from([0xed, 0xa0, 0x80]),              // encoded surrogate
  Buffer.from([0xff]),
  Buffer.from([0xfe]),
];

const UTF8_CHARS = "aZ0 λλ\u2294().; 中文字 \u{1F600}\u{1F680}-_";

function randomUtf8Text(rng, len) {
  let s = "";
  for (let i = 0; i < len; i++) {
    const r = rng();
    if (r < 0.7) s += UTF8_CHARS[Math.floor(rng() * UTF8_CHARS.length)];
    else if (r < 0.9) s += String.fromCharCode(randInt(rng, 32, 126));
    else s += "\n";
  }
  return s;
}

function genByteNoise(rng) {
  const kind = randInt(rng, 0, 5);
  if (kind === 0) {
    // uniform random bytes (mostly invalid UTF-8)
    return Buffer.from(Array.from({ length: randInt(rng, 0, 300) }, () => randInt(rng, 0, 255)));
  }
  if (kind === 1) {
    // random valid UTF-8 text
    return Buffer.from(randomUtf8Text(rng, randInt(rng, 0, 200)), "utf8");
  }
  if (kind === 2) {
    // valid text corrupted with a known-invalid UTF-8 sequence
    const text = Buffer.from(randomUtf8Text(rng, randInt(rng, 0, 120)), "utf8");
    const bad = pick(rng, BAD_UTF8);
    const pos = randInt(rng, 0, text.length);
    return Buffer.concat([text.subarray(0, pos), bad, text.subarray(pos)]);
  }
  if (kind === 3) {
    // BOM in front of a valid program / random text / random bytes
    const bom = Buffer.from([0xef, 0xbb, 0xbf]);
    const body =
      randInt(rng, 0, 2) === 0
        ? pick(rng, VALID_PROGRAMS).buf
        : Buffer.from(randomUtf8Text(rng, randInt(rng, 0, 120)), "utf8");
    return Buffer.concat([bom, body]);
  }
  if (kind === 4) {
    // CRLF / CR chaos over a valid program
    const lines = pick(rng, VALID_PROGRAMS).buf.toString("utf8").split("\n");
    return Buffer.from(
      lines
        .map((l) => {
          let s = "";
          for (const ch of l) s += ch === " " && chance(rng, 0.05) ? pick(rng, ["\r\n", "\r"]) : ch;
          return s;
        })
        .join(pick(rng, ["\r\n", "\r", "\n"])),
      "utf8"
    );
  }
  // NUL bytes sprinkled into a valid program, or a bare NUL run
  const base = pick(rng, VALID_PROGRAMS).buf;
  const out = [];
  for (const byte of base) {
    out.push(byte);
    if (chance(rng, 0.01)) out.push(0);
  }
  if (chance(rng, 0.3)) out.push(...Array.from({ length: randInt(rng, 1, 50) }, () => 0));
  return Buffer.from(out);
}

// --------------------------------------------------------------------------
// Input description for repro prints (hexdump-ish when not valid UTF-8)
// --------------------------------------------------------------------------

function describeInput(buf, maxBytes = 96) {
  const head = buf.subarray(0, maxBytes);
  const valid = Buffer.compare(Buffer.from(head.toString("utf8"), "utf8"), Buffer.from(head)) === 0;
  const more = buf.length > maxBytes ? ` ... (+${buf.length - maxBytes} bytes)` : "";
  if (valid) {
    return JSON.stringify(head.toString("utf8")) + more;
  }
  const hex = [...head].map((b) => b.toString(16).padStart(2, "0")).join(" ");
  return `hex[${buf.length}]: ${hex}${more}`;
}

// --------------------------------------------------------------------------
// Running one input and checking the invariant
// --------------------------------------------------------------------------

function runOne(mode, input, timeoutMs) {
  const t0 = Date.now();
  let res;
  try {
    res = spawnSync(BIN, [mode], {
      input,
      timeout: timeoutMs,
      maxBuffer: 64 * 1024 * 1024,
      encoding: "buffer",
    });
  } catch (e) {
    return { ms: Date.now() - t0, status: null, stderr: "", spawnError: String(e), timedOut: false };
  }
  const ms = Date.now() - t0;
  const stderr = Buffer.isBuffer(res.stderr) ? res.stderr.toString("utf8") : String(res.stderr ?? "");
  const timedOut = res.error?.code === "ETIMEDOUT" || ms >= timeoutMs - 50;
  return {
    ms,
    status: res.status,
    stderr,
    spawnError: res.error && !timedOut ? String(res.error) : null,
    timedOut,
  };
}

function violationOf(r) {
  if (r.timedOut) return "hang (timeout)";
  if (r.spawnError) return `spawn error: ${r.spawnError}`;
  if (r.status === null) return "no exit status (killed?)";
  if (![0, 1, 2].includes(r.status)) return `exit status ${r.status}`;
  if (r.stderr.includes("internal panic")) return "exit ok but stderr contains `internal panic`";
  return null;
}

// --------------------------------------------------------------------------
// Main
// --------------------------------------------------------------------------

const { n, seed, timeout } = parseArgs(process.argv.slice(2));
const binHash = createHash("sha256").update(readFileSync(BIN)).digest("hex");

const rng = mulberry32(seed);
const GENERATORS = [
  { name: "tokenSoup", gen: genTokenSoup },
  { name: "mutation", gen: (r) => mutateBytes(r, pick(r, VALID_PROGRAMS).buf) },
  { name: "structural", gen: genStructural },
  { name: "byteNoise", gen: genByteNoise },
];

const stats = GENERATORS.map((g) => ({ name: g.name, inputs: 0, violations: 0 }));
const exitHist = new Map();
let violations = 0;
const savedHashes = new Set();
const slowest = []; // {i, gen, mode, bytes, ms}
let nearMisses = [];

mkdirSync(CORPUS_DIR, { recursive: true });

console.log(`fuzzing ${BIN}`);
console.log(`  binary sha256 ${binHash}`);
console.log(`  seed ${seed}  n ${n}  timeout ${timeout} ms`);

const t0 = Date.now();
for (let i = 0; i < n; i++) {
  const mode = chance(rng, 0.5) ? "nf" : "type";
  const g = GENERATORS[i % GENERATORS.length];
  const st = stats[i % GENERATORS.length];
  const input = g.gen(rng);
  st.inputs++;

  const r = runOne(mode, input, timeout);
  const v = violationOf(r);
  const bucket = r.timedOut ? "hang" : String(r.status);
  exitHist.set(bucket, (exitHist.get(bucket) ?? 0) + 1);

  if (!v) {
    if (slowest.length < 3 || r.ms > slowest[slowest.length - 1].ms) {
      slowest.push({ i, gen: g.name, mode, bytes: input.length, ms: r.ms });
      slowest.sort((a, b) => b.ms - a.ms);
      slowest.length = Math.min(slowest.length, 3);
    }
    if (r.ms > NEAR_MISS_MS) {
      nearMisses.push({ i, gen: g.name, mode, bytes: input.length, ms: r.ms });
    }
    continue;
  }

  violations++;
  st.violations++;
  const hash = createHash("sha256").update(input).digest("hex");
  if (!savedHashes.has(hash)) {
    savedHashes.add(hash);
    writeFileSync(join(CORPUS_DIR, `${hash}.dpm`), input);
  }
  console.log(`\n[VIOLATION #${violations}] input ${i} generator=${g.name} mode=${mode} bytes=${input.length}`);
  console.log(`  ${v}`);
  console.log(`  status=${r.status} stderr=${JSON.stringify(r.stderr.slice(0, 300))}`);
  console.log(`  input: ${describeInput(input)}`);
  console.log(`  saved: blackbox/fuzz-corpus/${hash}.dpm`);
}
const wall = Date.now() - t0;

console.log(`\n=== fuzz summary ===`);
console.log(`binary sha256 ${binHash}`);
console.log(`seed ${seed}  n ${n}  timeout ${timeout} ms  wall ${(wall / 1000).toFixed(1)} s`);
for (const s of stats) {
  console.log(`  ${s.name.padEnd(11)} inputs ${String(s.inputs).padStart(5)}  violations ${s.violations}`);
}
console.log(`exit statuses: ${[...exitHist.entries()].map(([k, c]) => `${JSON.stringify(k)}:${c}`).sort().join("  ")}`);
console.log(`slowest inputs:`);
for (const s of slowest) {
  console.log(`  input ${s.i} (${s.gen}, ${s.mode}, ${s.bytes} B): ${s.ms} ms`);
}
console.log(`near-misses > ${NEAR_MISS_MS} ms: ${nearMisses.length}`);
for (const s of nearMisses.slice(0, 10)) {
  console.log(`  input ${s.i} (${s.gen}, ${s.mode}, ${s.bytes} B): ${s.ms} ms`);
}
console.log(`violations: ${violations}`);
process.exitCode = violations > 0 ? 1 : 0;
