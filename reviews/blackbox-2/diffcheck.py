#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Differential / consistency checks for dpm-nbe.exe.

Invariants checked for every well-typed seed program P:
  A) nf(P) exits 0
  B) type(P) == the type line printed by nf(P)         (cross-mode agreement)
  C) nf(nf(P)) == nf(P)                                (round-trip idempotence)
  D) type(nf(P)) == type(P)                            (type stability under nf)
  E) nf(type(P)) is well-typed and nf(nf(type(P))) == nf(type(P))
"""
import json
import os
import subprocess

HERE = os.path.dirname(os.path.abspath(__file__))
LIVE = r"F:\projects\Haskell\dpm-nbe\rust\target\release\dpm-nbe.exe"
PINNED = os.path.join(HERE, "bin", "pinned-33052DDC.exe")
BIN = os.environ.get("DPM_BIN") or (PINNED if os.path.exists(PINNED) else LIVE)
SEEDIR = os.path.join(HERE, "corpus", "valid")

SEEDS = [
    ("s01_plus", "plus (suc zero) (suc zero)"),
    ("s02_arrow", "(A : U) -> A -> A"),
    ("s03_id", "Id Nat zero zero"),
    ("s04_refl", "let r : Id Nat zero zero = refl; r"),
    ("s05_lambda", "let f : (A : U) -> A -> A = \u03bb A x . x; f"),
    ("s06_lambda_app", "let f : (A : U) -> A -> A = \u03bb A x . x; f Nat (suc zero)"),
    ("s07_multi_binder", "let f : (A B : U) -> A -> B -> A = \u03bb A B x y . x; f"),
    ("s08_nested_arrow", "let f : Nat -> Nat -> Nat -> Nat = \u03bb a b c . plus a (plus b c); f"),
    ("s09_id_fun", "Id (Nat -> Nat) (\\x . x) (\\y . y)" if False else
        "let g : Nat -> Nat = \u03bb x . x; Id (Nat -> Nat) g g"),
    ("s10_app_of_app", "let g : Nat -> Nat = \u03bb n . suc n; "
                       "let f : Nat -> Nat -> Nat = \u03bb a b . plus a b; f (g zero) (g (suc zero))"),
    ("s11_arrow_arg", "let g : Nat -> Nat = \u03bb n . n; "
                      "let f : (Nat -> Nat) -> Nat = \u03bb h . h zero; f g"),
    ("s12_big_type_out", "(A : U)(B : U)(C : U) -> A -> B -> C -> A"),
    ("s13_unicode", "(A : U) \u2192 A \u2192 A"),
    ("s14_absurd_unused", "let C : (A : U) -> Id Nat (suc zero) zero -> A = \u03bb A (!); U"),
    ("s15_refl_pattern", "let f : (A : U) -> Id A zero zero -> A = \u03bb A refl . A; f"),
    ("s16_plus_conv", "let f : Id Nat (plus (suc zero) (suc zero)) (suc (suc zero)) = refl; f"),
    ("s17_trans", "let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> Id A a c = "
                  "\u03bb A a b c refl refl . refl; trans"),
    ("s18_H", "let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero = "
              "\u03bb m n refl refl . refl; H"),
    ("s19_let_chain", "let a : Nat = zero; let b : Nat = suc a; let c : Nat = suc b; c"),
    ("s20_shadow", "let x : Nat = zero; let x : Nat = suc x; let x : Nat = suc x; x"),
    ("s21_type_of_type", "U"),
    ("s22_nat", "Nat"),
    ("s23_zero", "zero"),
    ("s24_pi_dep", "(A : U) -> A -> A"),
    ("s25_pi_dep2", "(f : Nat -> Nat) -> Id Nat (f zero) (f zero)"),
    ("s26_id_u", "let f : (A : U) -> Id U A A = \u03bb A . refl; f"),
    ("s27_id_id", "let x : Id (Id Nat zero zero) refl refl = refl; x"),
    ("s28_fun_type_in_id", "let g : Nat -> Nat = \u03bb n . n; Id (Nat -> Nat) g (\u03bb n . n)"),
    ("s29_id_of_arrow_type", "Id U (Nat -> Nat) (Nat -> Nat)"),
    ("s30_plus_var", "let n : Nat = suc (suc zero); plus n n"),
    ("s31_big_plus", "plus (suc (suc (suc zero))) (suc (suc zero))"),
    ("s32_absurd_pat_unused_fn",
     "let C : (A : U) -> Id Nat (suc zero) zero -> A = \u03bb A (!); let d : Nat = zero; d"),
    ("s33_dep_pi", "(A : U) (B : A -> U) -> (x : A) -> B x"),
    ("s34_nested_lambda", "let f : Nat -> Nat -> Nat = \u03bb a . \u03bb b . plus a b; f" if False else
        "let f : Nat -> Nat -> Nat = \u03bb a b . plus a b; f"),
    ("s35_higher_order", "let f : (Nat -> Nat) -> Nat -> Nat = \u03bb g n . g (g n); f"),
    ("s36_id_in_arg", "let g : Nat -> Nat = \u03bb n . n; "
                      "let f : Id (Nat -> Nat) g g -> Nat = \u03bb r . zero; f"),
    ("s37_pi_in_arg", "(A : U) -> Id U (A -> A) (A -> A)"),
    ("s38_complex_nf", "let f : (A : U) -> A -> A = \u03bb A x . x; "
                       "let g : Nat -> Nat = f Nat; g (suc zero)"),
    ("s39_id_trans_chain",
     "let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> Id A a c = "
     "\u03bb A a b c refl refl . refl; "
     "trans Nat zero zero zero refl (trans Nat zero zero zero refl refl)"),
    ("s40_lambda_thunk", "let k : Nat -> U = \u03bb n . Nat; k zero"),
    ("s41_type_level_let", "let T : U = Nat -> Nat; T"),
    ("s42_id_sym", "let f : (A : U) (a b : A) -> Id A a b -> Id A b a = "
                   "\u03bb A a b refl . refl; f"),
    ("s43_suc_of_id", "Id Nat (suc zero) (suc zero)"),
    ("s44_pi_codomain_fun", "(A : U) -> (A -> A) -> A -> A"),
    ("s45_deep_apply", "let g : Nat -> Nat = \u03bb n . suc n; g (g (g (g zero)))"),
    ("s46_multi_group_binder", "let f : (A : U) (B : U) (x : A) (y : B) -> A = "
                               "\u03bb A B x y . x; f"),
    ("s47_comment_prog", "let x : Nat = zero; -- comment\n x {- block -}"),
    ("s48_lambda_unicode", "let f : Nat -> Nat = \u03bb n . n; f zero"),
    ("s49_id_pi", "Id U ((A : U) -> A) ((A : U) -> A)"),
    ("s50_absurd_two_args_unused",
     "let C : (A : U) -> Id Nat (suc zero) zero -> Nat -> A = \u03bb A (!); U"),
]


def run(text, mode="nf", timeout=20):
    data = text.encode("utf-8") if isinstance(text, str) else text
    p = subprocess.run([BIN, mode], input=data, stdout=subprocess.PIPE,
                       stderr=subprocess.PIPE, timeout=timeout)
    return p.returncode, p.stdout.decode("utf-8", "replace"), p.stderr.decode("utf-8", "replace")


SEP = "\n  :\n"


def split_nf(out):
    """nf output is '<nf>\\n  :\\n<type>\\n' -> (nf, type)."""
    assert out.endswith("\n"), repr(out[-20:])
    body = out[:-1]
    if SEP not in body:
        return None, None
    nf, ty = body.split(SEP, 1)
    return nf, ty


def main():
    os.makedirs(SEEDIR, exist_ok=True)
    findings = []
    rows = []
    for name, src in SEEDS:
        with open(os.path.join(SEEDIR, name + ".txt"), "w", encoding="utf-8") as fh:
            fh.write(src + "\n")
        rec = {"name": name, "src": src}
        rc, out, err = run(src, "nf")
        rec["nf_rc"] = rc
        rec["nf_out"] = out
        rec["nf_err"] = err
        if rc != 0:
            findings.append(("SEED-NOT-WELLTYPED", name, rc, err.strip()[:200]))
            rows.append(rec)
            continue
        nf, ty = split_nf(out)
        if nf is None:
            findings.append(("NO-SEPARATOR", name, 0, repr(out[:120])))
            rows.append(rec)
            continue
        rec["nf"] = nf
        rec["type_from_nf"] = ty

        rc_t, out_t, err_t = run(src, "type")
        rec["type_rc"] = rc_t
        rec["type_out"] = out_t
        if rc_t != 0 or out_t != ty + "\n":
            findings.append(("B-TYPE-MISMATCH", name, rc_t,
                             "type-mode=%r nf-line=%r err=%r" % (out_t, ty + "\n", err_t[:200])))

        rc2, out2, err2 = run(nf, "nf")
        rec["nf2_rc"] = rc2
        rec["nf2_out"] = out2
        if rc2 != 0 or out2 != out:
            findings.append(("C-NOT-IDEMPOTENT", name, rc2,
                             "nf=%r -> rc=%s out=%r err=%r" % (nf, rc2, out2[:200], err2[:200])))
        else:
            nf2, ty2 = split_nf(out2)
            if ty2 != ty:
                findings.append(("D-TYPE-UNSTABLE", name, rc2, "%r vs %r" % (ty2, ty)))

        rc3, out3, err3 = run(nf, "type")
        rec["nf_type_rc"] = rc3
        rec["nf_type_out"] = out3
        if rc3 != 0 or out3 != ty + "\n":
            findings.append(("D2-TYPE-OF-NF", name, rc3,
                             "type(nf)=%r expected=%r err=%r" % (out3, ty + "\n", err3[:200])))

        rc4, out4, err4 = run(ty, "nf")
        rec["type_nf_rc"] = rc4
        rec["type_nf_out"] = out4[:200]
        if rc4 != 0:
            findings.append(("E-TYPE-NOT-PARSEABLE", name, rc4, "type=%r err=%r" % (ty, err4[:200])))
        else:
            tnf, tty = split_nf(out4)
            if tnf is None or tnf != ty:
                findings.append(("E-TYPE-NF-NOT-SELF", name, rc4,
                                 "nf(type)=%r type=%r" % (tnf, ty)))
        rows.append(rec)

    with open(os.path.join(HERE, "diffcheck.json"), "w", encoding="utf-8") as fh:
        json.dump({"rows": rows, "findings": findings}, fh, ensure_ascii=False, indent=1)

    print("seeds:", len(SEEDS), " findings:", len(findings))
    for f in findings:
        print("  !!", f[0], f[1], "rc=%s" % f[2])
        print("     ", f[3][:400])


if __name__ == "__main__":
    main()
