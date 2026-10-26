// Generator/oracle for the lang blackbox cases.
//
// Usage (from the repo root, node >= 18):
//   node blackbox/cases/lang/gen.mjs numeral 1024        print the Peano numeral
//   node blackbox/cases/lang/gen.mjs out 1024 Nat FILE   write a full `.out` body
//   node blackbox/cases/lang/gen.mjs check FILE 1024     check a captured `.out`
//
// `numeral n` is exactly how dpm-nbe prints the Peano numeral for n: the
// innermost application is the atom-argument form `suc zero` (no parens),
// every enclosing one is `suc (` ... `)`.  numeral 0 = "zero".
import { writeFileSync, readFileSync } from "node:fs";

const num = (n) => (n === 0 ? "zero" : "suc (".repeat(n - 1) + "suc zero" + ")".repeat(n - 1));

const [, , cmd, a, b, c] = process.argv;
if (cmd === "numeral") {
  process.stdout.write(num(Number(a)) + "\n");
} else if (cmd === "out") {
  const body = num(Number(a)) + "\n  :\n" + (b ?? "Nat") + "\n";
  if (c) writeFileSync(c, body);
  else process.stdout.write(body);
} else if (cmd === "check") {
  const got = readFileSync(a, "utf8");
  const [nf, , ty] = got.split("\n");
  const n = Number(b);
  const ok = nf === num(n) && ty === "Nat";
  console.log(`${a}: nf==numeral(${n}): ${nf === num(n)}; type: ${JSON.stringify(ty)}; PASS=${ok}`);
  process.exit(ok ? 0 : 1);
} else {
  console.error("usage: gen.mjs numeral N | out N TYPE FILE | check FILE N");
  process.exit(2);
}
