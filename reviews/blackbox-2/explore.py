import subprocess, sys

BIN = r"F:\projects\Haskell\dpm-nbe\rust\target\release\dpm-nbe.exe"

def run(text, mode="nf", timeout=10):
    if isinstance(text, str):
        data = text.encode("utf-8")
    else:
        data = text
    try:
        p = subprocess.run([BIN, mode], input=data, stdout=subprocess.PIPE,
                           stderr=subprocess.PIPE, timeout=timeout)
        rc, out, err = p.returncode, p.stdout, p.stderr
    except subprocess.TimeoutExpired as e:
        return ("TIMEOUT", e.stdout or b"", e.stderr or b"")
    return (rc, out, err)

def show(label, text, mode="nf", timeout=10):
    rc, out, err = run(text, mode, timeout)
    print("=== %s [%s] rc=%s" % (label, mode, rc))
    if out:
        print("  OUT:", repr(out[:600]))
    if err:
        print("  ERR:", repr(err[:600]))

if __name__ == "__main__":
    probes = [
        ("readme-example", """let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> Id A a c =
  \u03bb A a b c refl refl . refl;

let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero =
  \u03bb m n refl refl . refl;

let C : (A : U) -> Id Nat (suc zero) zero -> A =
  \u03bb A (!);

U
"""),
        ("absurd", """let C : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!); C"""),
        ("und", "let x : Nat = zero; _"),
        ("longtype", "let f : (A : U) -> (B : U) -> (x : A) -> (y : B) -> A = \\A B x y . x; f"),
        ("uniproj", "let f : (A : U) -> A -> A = \u03bb A x . x; f Nat zero"),
        ("arrow-uni", "(A : U) \u2192 A \u2192 A"),
        ("toplevel-type", "(A : U) -> A -> A"),
        ("nested-id", "refl"),
        ("letfun", "let f : Nat -> Nat = \\n . n; f (suc zero)"),
        ("emptyparen", "()"),
        ("unterminated-block", "zero {- oops"),
        ("crlf", "let x : Nat = zero;\r\nx\r\n"),
        ("bom", "\ufeffzero"),
        ("unicode-id", "let \u03b1 : Nat = zero; \u03b1"),
        ("digit-id", "let x1 : Nat = zero; x1"),
        ("tab", "\tzero\t"),
        ("comment-eof", "zero -- trailing"),
        ("comment-eof-nonl", b"zero -- trailing"),
        ("dashdash-only", "-- x"),
        ("closebrace", "-}"),
        ("dashparen", "zero --)"),
    ]
    for label, text in probes:
        show(label, text)
