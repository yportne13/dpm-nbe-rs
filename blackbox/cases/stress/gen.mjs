// Deterministic generator for the randomized-property and stress black-box
// cases of `blackbox/cases/stress/`.
//
//   node gen.mjs        (re)writes every case below; sidecars included
//
// Running it twice produces byte-identical files: the only randomness is a
// mulberry32 PRNG driven by the fixed seeds in SEEDS, and the stress families
// are pure templates.  No timestamps, no environment reads, no binary runs.
//
// Groups written (see findings-stress.md for the measured boundaries):
//   rand-arith-s{k}-NN   50 per seed: random well-typed plus/mult/suc
//                        expressions; the oracle computes the value and the
//                        .out sidecar pins the exact `suc (...)` normal form.
//   rand-bad-s{k}-NN     10 per seed: a type error sprinkled into a random
//                        program; exit 1 + a pinned stderr fragment per shape.
//   depth-*              nesting 511/512/513 per form plus the measured
//                        pass/fail boundary pair for each form (.timeout 30).
//   width-*              argument/field counts at and past the spine limit.
//   scale-*              big numerals / elim-heavy programs (.timeout ~3x the
//                        measured runtime).  Two parked .skip cases document
//                        that huge numeral *literals* exceed the depth budget.
//   det-*                three byte-identical pairs under different names.
//
// The arithmetic prelude mirrors tests/golden.rs (lambda spelled λ; case files
// may contain λ, shell command lines must not).

import { writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const DIR = dirname(fileURLToPath(import.meta.url));

// ---------------------------------------------------------------------------
// seeded PRNG
// ---------------------------------------------------------------------------

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

// Fixed committed seeds for the three random families.
export const SEEDS = { s1: 1988, s2: 44444, s3: 20251025 };
const ARITH_PER_SEED = 50;
const BAD_PER_SEED = 10;

// ---------------------------------------------------------------------------
// shared source builders
// ---------------------------------------------------------------------------

export const LAM = "\u03bb"; // λ

export const PRELUDE =
  "data Nat : U where zero : Nat; suc : (n : Nat) -> Nat;\n" +
  `let plus : Nat -> Nat -> Nat =\n  ${LAM} m n . elim Nat (${LAM} _ . Nat) n (${LAM} _ r . suc r) m;\n` +
  `let mult : Nat -> Nat -> Nat =\n  ${LAM} m n . elim Nat (${LAM} _ . Nat) zero (${LAM} _ r . plus n r) m;\n`;

const FIB_PRELUDE =
  PRELUDE +
  "struct Pair : U where a : Nat; b : Nat;\n" +
  `let fib : Nat -> Nat = ${LAM} n . (elim Nat (${LAM} _ . Pair) (mkPair zero (suc zero)) (${LAM} _ r . mkPair r.b (plus r.a r.b)) n).a;\n`;

const POWER_PRELUDE =
  PRELUDE +
  `let power : Nat -> Nat -> Nat = ${LAM} m n . elim Nat (${LAM} _ . Nat) (suc zero) (${LAM} _ r . mult m r) n;\n`;

/** `zero`, `suc zero`, `suc (suc zero)`, ... exactly as the NF printer does. */
export function numeral(n) {
  if (n === 0) return "zero";
  if (n === 1) return "suc zero";
  return `suc (${numeral(n - 1)})`;
}

const nfOut = (nf, ty) => `${nf}\n  :\n${ty}\n`;
const natOut = (v) => nfOut(numeral(v), "Nat");

const TOO_DEEP = "expression nesting is too deep";

function writeCase(name, { src, out, err, code, timeout, skip = false }) {
  writeFileSync(join(DIR, skip ? `${name}.dpm.skip` : `${name}.dpm`), src);
  // Parked (.skip) cases keep their sidecar names per the README; the runner
  // ignores them, they just document the expectation.
  const side = join(DIR, `${name}.dpm`);
  if (out !== undefined) writeFileSync(`${side}.out`, out);
  if (err !== undefined) writeFileSync(`${side}.err`, err);
  if (code !== undefined) writeFileSync(`${side}.code`, String(code));
  if (timeout !== undefined) writeFileSync(`${side}.timeout`, String(timeout));
}

// ---------------------------------------------------------------------------
// 1. random well-typed arithmetic, oracle-checked
// ---------------------------------------------------------------------------
//
// Value trees over numerals (<= 30 at the leaves), unary `suc` wrappers and
// binary `plus`/`mult`, generated under a value budget so the normal form
// stays small enough to pin.  A mult node picks a factor bound `va` for its
// left child and `floor(budget/va)` for its right one, which keeps the
// product inside the budget by construction.

export function genNode(rng, budget, depth) {
  if (budget < 0) budget = 0;
  const roll = rng();
  if (depth === 0 || roll < 0.3 || (roll < 0.55 && budget < 1)) {
    return { k: "num", v: randInt(rng, 0, Math.min(30, budget)) };
  }
  if (roll < 0.55) {
    const a = genNode(rng, budget - 1, depth - 1);
    return { k: "suc", v: a.v + 1, a };
  }
  if (roll < 0.8) {
    const split = randInt(rng, 0, budget);
    const a = genNode(rng, split, depth - 1);
    const b = genNode(rng, budget - split, depth - 1);
    return { k: "plus", v: a.v + b.v, a, b };
  }
  // mult: constrain the children's budgets so a.v * b.v <= budget holds.
  if (budget < 2) {
    return { k: "num", v: randInt(rng, 0, Math.min(30, budget)) };
  }
  const va = randInt(rng, 1, Math.min(30, budget));
  const a = genNode(rng, va, depth - 1);
  const b = genNode(rng, Math.floor(budget / va), depth - 1);
  return { k: "mult", v: a.v * b.v, a, b };
}

/** Render a node as surface syntax; every compound argument is parenthesised. */
export function render(n) {
  switch (n.k) {
    case "num":
      return numeral(n.v);
    case "suc":
      return `suc (${render(n.a)})`;
    case "plus":
      return `plus (${render(n.a)}) (${render(n.b)})`;
    case "mult":
      return `mult (${render(n.a)}) (${render(n.b)})`;
  }
}

// ---------------------------------------------------------------------------
// 3. depth families (sources and expected normal forms)
// ---------------------------------------------------------------------------
//
// Measured pass/fail boundaries of the pinned binary in the exact source
// context used below (the prelude consumes a few depth units itself, and a
// parenthesised nesting level costs ~2 units against MAX_DEPTH = 512):
//
//   appnest     f (f (... zero))      253 / 254
//   lamnest     nested λ under let    506 / 507
//   letchain    let a2 = a1; ...      507 / 508
//   parens      ((((zero))))          253 / 254
//   projchain   x.b.b...b             512 / 513
//   datagroups  data D (p1 : U) ...   512 / 513
//   numlit      suc^N zero literal    254 / 255
//
// Every form gets the requested 511/512/513 probes plus its exact boundary
// pair; failures all exit 1 reporting "expression nesting is too deep".

const telescope = (n, pfx) =>
  Array.from({ length: n }, (_, i) => `(${pfx}${i + 1} : Nat) `).join("");

const appnestSrc = (n) =>
  PRELUDE +
  `let f : Nat -> Nat = ${LAM} x . suc x; ` +
  "f (".repeat(n) + "zero" + ")".repeat(n) + "\n";

const lamnestSrc = (n) =>
  PRELUDE +
  `let g : ${telescope(n, "x")}-> Nat = ` +
  Array.from({ length: n }, (_, i) => `${LAM} x${i + 1} . `).join("") +
  "zero; g\n";

// The NF printer merges nested lambdas into one binder list; the type prints
// as concatenated named telescope groups (both checked against the binary).
const lamnestOut = (n) =>
  nfOut(
    `λ ${Array.from({ length: n }, (_, i) => `x${i + 1}`).join(" ")} . zero`,
    `${Array.from({ length: n }, (_, i) => `(x${i + 1} : Nat)`).join("")} → Nat`,
  );

const letchainSrc = (n) =>
  PRELUDE +
  "let a1 : Nat = zero; " +
  Array.from({ length: n - 1 }, (_, i) => `let a${i + 2} : Nat = a${i + 1}; `).join("") +
  `a${n}\n`;

const parensSrc = (n) => PRELUDE + "(".repeat(n) + "zero" + ")".repeat(n) + "\n";

const projchainSrc = (n) =>
  "data Nat : U where zero : Nat; suc : Nat -> Nat;\n" +
  "struct Box : U where n : Nat; b : Box;\n" +
  `let d : Box -> Box = ${LAM} x . x${".b".repeat(n)}; U\n`;

const datagroupsSrc = (n) =>
  "data D " +
  Array.from({ length: n }, (_, i) => `(p${i + 1} : U) `).join("") +
  ": U where U\n";

const numlitSrc = (n) => PRELUDE + numeral(n) + "\n";

const DEPTH_FORMS = [
  { name: "appnest", src: appnestSrc, out: (n) => natOut(n), boundary: 253 }, // f = suc: n applications build the numeral n
  { name: "lamnest", src: lamnestSrc, out: lamnestOut, boundary: 506 },
  { name: "letchain", src: letchainSrc, out: () => natOut(0), boundary: 507 },
  { name: "parens", src: parensSrc, out: () => natOut(0), boundary: 253 },
  { name: "projchain", src: projchainSrc, out: () => nfOut("U", "U1"), boundary: 512 },
  { name: "datagroups", src: datagroupsSrc, out: () => nfOut("U", "U1"), boundary: 512 },
  { name: "numlit", src: numlitSrc, out: (n) => natOut(n), boundary: 254 },
];

// ---------------------------------------------------------------------------
// 4. width families
// ---------------------------------------------------------------------------

const widthFnSrc = (n) =>
  PRELUDE +
  `let f : ${telescope(n, "x")}-> Nat = ` +
  `${LAM} ${Array.from({ length: n }, (_, i) => `x${i + 1} `).join("")}. zero; ` +
  "f " + "zero ".repeat(n).trim() + "\n";

const widthCtorSrc = (n) =>
  PRELUDE +
  "data D : U where c : " +
  telescope(n, "x") +
  "-> D; c " +
  "zero ".repeat(n).trim() + "\n";

const widthCtorOut = (n) => nfOut(`c ${"zero ".repeat(n).trim()}`, "D");

const structFieldsSrc = (n) =>
  "data Nat : U where zero : Nat; suc : Nat -> Nat;\n" +
  "struct S : U where " +
  Array.from({ length: n }, (_, i) => `f${i + 1} : Nat; `).join("") +
  "let v : S = mkS " + "zero ".repeat(n).trim() + `; v.f${n}\n`;

// ---------------------------------------------------------------------------
// generation (runs only when executed directly, not on import)
// ---------------------------------------------------------------------------

function generate() {
  const counters = { arith: 0, bad: 0, depth: 0, width: 0, scale: 0, det: 0 };

  for (const [tag, seed] of Object.entries(SEEDS)) {
    const rng = mulberry32(seed);

    // -- 1. random well-typed arithmetic -------------------------------
    for (let i = 0; i < ARITH_PER_SEED; i++) {
      const budget = randInt(rng, 60, 500);
      const depth = randInt(rng, 2, 5);
      const node = genNode(rng, budget, depth);
      writeCase(`rand-arith-${tag}-${String(i).padStart(2, "0")}`, {
        src: PRELUDE + render(node) + "\n",
        out: natOut(node.v),
      });
      counters.arith++;
    }

    // -- 2. random ill-typed programs (exit 1, pinned fragment) --------
    const shapes = [
      // wrong-typed application argument
      (g) => ({ src: `plus (${render(g)}) U\n`, err: "type mismatch" }),
      // unbound variable in an argument
      (g) => ({ src: `suc (mult (${render(g)}) q)\n`, err: "unbound variable: q" }),
      // over-application of a Nat-valued function
      (g) => ({
        src: `plus (${render(g)}) (${numeral(2)}) (${numeral(3)})\n`,
        err: "Expected a function type",
      }),
      // lambda arity vs. ascribed type
      (g) => ({
        src: `let f : Nat -> Nat = ${LAM} x y . ${render(g)}; zero\n`,
        err: "Too many patterns.",
      }),
    ];
    for (let i = 0; i < BAD_PER_SEED; i++) {
      const g = genNode(rng, randInt(rng, 20, 120), randInt(rng, 1, 3));
      const { src, err } = shapes[i % shapes.length](g);
      writeCase(`rand-bad-${tag}-${String(i).padStart(2, "0")}`, {
        src: PRELUDE + src,
        err,
        code: 1,
      });
      counters.bad++;
    }
  }

  // -- 3. depth boundaries (.timeout 30 everywhere in this group) ------
  // Per form: the requested 511/512/513 probes, plus the exact measured
  // boundary pair (deduplicated when the probes already cover it).
  for (const form of DEPTH_FORMS) {
    const sizes = [...new Set([511, 512, 513, form.boundary, form.boundary + 1])];
    sizes.sort((a, b) => a - b);
    for (const n of sizes) {
      const ok = n <= form.boundary;
      writeCase(`depth-${form.name}-${n}`, {
        src: form.src(n),
        ...(ok ? { out: form.out(n) } : { err: TOO_DEEP, code: 1 }),
        timeout: 30,
      });
      counters.depth++;
    }
  }

  // -- 4. width --------------------------------------------------------
  // function applied to 511 (pass) / 512 / 513 arguments (spine atom limit)
  for (const n of [511, 512, 513]) {
    const ok = n <= 511;
    writeCase(`width-app-${n}`, {
      src: widthFnSrc(n),
      ...(ok ? { out: natOut(0) } : { err: TOO_DEEP, code: 1 }),
      timeout: 30,
    });
    counters.width++;
  }
  // constructor applied to 511 (pass) / 512 / 513 arguments
  for (const n of [511, 512, 513]) {
    const ok = n <= 511;
    writeCase(`width-ctor-${n}`, {
      src: widthCtorSrc(n),
      ...(ok ? { out: widthCtorOut(n) } : { err: TOO_DEEP, code: 1 }),
      timeout: 30,
    });
    counters.width++;
  }
  // 500 chained lets
  writeCase("width-let-500", { src: letchainSrc(500), out: natOut(0), timeout: 10 });
  counters.width++;
  // struct with 200 fields, projecting the last one (also exercises the 200
  // projection bindings the desugaring creates)
  writeCase("width-struct-200", { src: structFieldsSrc(200), out: natOut(0), timeout: 30 });
  counters.width++;
  // the struct-field boundary itself: 511 fields pass, 512 fail
  writeCase("width-struct-511", { src: structFieldsSrc(511), out: natOut(0), timeout: 30 });
  writeCase("width-struct-512", { src: structFieldsSrc(512), err: TOO_DEEP, code: 1, timeout: 30 });
  counters.width += 2;

  // -- 5. scale (timeouts ~3x measured; see findings-stress.md) --------
  // 1000 as a plus of four <=254 literals (literals cannot nest past 254)
  writeCase("scale-num-1000", {
    src: PRELUDE + `plus (${numeral(250)}) (plus (${numeral(250)}) (plus (${numeral(250)}) (${numeral(250)})))\n`,
    out: natOut(1000),
    timeout: 5,
  });
  // plus 1000 1000: NF has 2000 successors
  writeCase("scale-plus-1000-1000", {
    src:
      PRELUDE +
      `let x : Nat = plus (${numeral(250)}) (plus (${numeral(250)}) (plus (${numeral(250)}) (${numeral(250)}))); plus x x\n`,
    out: natOut(2000),
    timeout: 10,
  });
  writeCase("scale-mult-25-25", {
    src: PRELUDE + `mult (${numeral(25)}) (${numeral(25)})\n`,
    out: natOut(625),
    timeout: 10,
  });
  // elim-based fib on a pair accumulator: 0,1,1,2,... fib(18) = 2584
  writeCase("scale-fib-18", {
    src: FIB_PRELUDE + `fib (${numeral(18)})\n`,
    out: natOut(2584),
    timeout: 20,
  });
  writeCase("scale-power-2-12", {
    src: POWER_PRELUDE + `power (${numeral(2)}) (${numeral(12)})\n`,
    out: natOut(4096),
    timeout: 30,
  });
  // 10000 successors via mult 250 40 (~60 KB of normal form)
  writeCase("scale-num-10000", {
    src: PRELUDE + `mult (${numeral(250)}) (${numeral(40)})\n`,
    out: natOut(10000),
    timeout: 130,
  });
  counters.scale += 6;
  // suc^1000 zero / suc^10000 zero as *literals* are unrepresentable: a
  // numeral literal costs ~2 depth units per successor, so the parser rejects
  // them far below 1000.  Parked as documented findings; the sidecars record
  // the (clean, exit-1) rejection.
  writeCase("scale-suc1000-literal", {
    src: PRELUDE + numeral(1000) + "\n",
    err: TOO_DEEP,
    code: 1,
    timeout: 30,
    skip: true,
  });
  writeCase("scale-suc10000-literal", {
    src: PRELUDE + numeral(10000) + "\n",
    err: TOO_DEEP,
    code: 1,
    timeout: 30,
    skip: true,
  });

  // -- 6. determinism: byte-identical pairs under different names -------
  const detA =
    PRELUDE +
    render({
      k: "mult",
      a: { k: "plus", a: { k: "num", v: 2 }, b: { k: "mult", a: { k: "num", v: 1 }, b: { k: "num", v: 3 } } },
      b: { k: "plus", a: { k: "num", v: 2 }, b: { k: "num", v: 1 } },
    }) + "\n";
  const detAOut = natOut((2 + 1 * 3) * (2 + 1));
  const detB = FIB_PRELUDE + `fib (${numeral(18)})\n`;
  const detC = lamnestSrc(200);

  for (const suffix of ["1", "2"]) {
    writeCase(`det-a${suffix}`, { src: detA, out: detAOut });
    writeCase(`det-b${suffix}`, { src: detB, out: natOut(2584), timeout: 20 });
    writeCase(`det-c${suffix}`, { src: detC, out: lamnestOut(200), timeout: 30 });
    counters.det += 3;
  }

  console.log(
    `wrote ${counters.arith} rand-arith, ${counters.bad} rand-bad, ` +
      `${counters.depth} depth, ${counters.width} width, ${counters.scale} scale ` +
      `(+2 parked), ${counters.det} det cases into ${DIR}`,
  );
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  generate();
}
