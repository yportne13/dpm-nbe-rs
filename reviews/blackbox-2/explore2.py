import subprocess

BIN = r"F:\projects\Haskell\dpm-nbe\rust\target\release\dpm-nbe.exe"

def run(text, mode="nf", timeout=10, args=None):
    data = text.encode("utf-8") if isinstance(text, str) else text
    cmd = [BIN] + (args if args is not None else [mode])
    try:
        p = subprocess.run(cmd, input=data, stdout=subprocess.PIPE,
                           stderr=subprocess.PIPE, timeout=timeout)
        return (p.returncode, p.stdout, p.stderr)
    except subprocess.TimeoutExpired as e:
        return ("TIMEOUT", e.stdout or b"", e.stderr or b"")

def show(label, text, mode="nf", timeout=10, args=None):
    rc, out, err = run(text, mode, timeout, args)
    print("=== %s [%s] rc=%s" % (label, mode if args is None else args, rc))
    if out:
        print("  OUT:", repr(out[:400]))
    if err:
        print("  ERR:", repr(err[:400]))

def main():
    C = "let C : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!); %s"
    probes = [
        ("C-final", C % "C"),
        ("C-final-type-mode", C % "C", "type"),
        ("C-in-nf-of-type", "let C : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!); U"),
        ("absurd-lambda-nf", "let f : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!); f Nat"),
        ("absurd-applied", "let C : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!); C Nat"),
        ("absurd-applied-refl", "let C : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!); C Nat refl"),
        ("absurd-noannot", "let C = \\A (!); C"),
        ("absurd-in-nested-let", "let C : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!); let D : Nat = plus zero zero; C"),
        ("absurd-shadowed", "let C : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!); let C : Nat = zero; C"),
        ("absurd-under-lambda-unused", "let f : Nat -> ((A : U) -> Id Nat (suc zero) zero -> A) = \\n . (\\A (!)); zero"),
        ("absurd-never-used", "let C : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!); zero"),
        ("absurd-2-patterns", "let C : (A : U) -> Id Nat (suc zero) zero -> Id Nat (suc zero) zero -> A = \\A (!) (!); C"),
        ("absurd-in-type-slot", "let C : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!); let D : Nat = zero; D"),
        # line numbering oddities
        ("no-trailing-nl", b"suc"),
        ("empty-stdin", b""),
        ("only-nl", b"\n"),
        ("only-spaces", b"   "),
        ("suc-midfile", b"suc\n"),
        ("tabs-lines", b"\t\n\tsuc\n"),
        ("cr-only", b"suc\r"),
        ("crlf-eof", b"suc\r\n"),
        ("nul-in-input", b"zero\x00"),
        ("invalid-utf8", b"zero\xff"),
        ("lone-surrogate-utf8", b"zero\xed\xa0\x80"),
        ("overlong-utf8", b"zero\xc0\x80"),
        # multi-line output?
        ("huge-type", "let f : Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat = \\a b c d e g h i j k . a; f"),
        ("huge-type2", "(A : U)(B : U)(C : U)(D : U)(E : U)(F : U)(G : U)(H : U) -> A"),
        ("long-ident-out", "let averyveryveryverylongidentifiernamethatgoesonandon : Nat = zero; averyveryveryverylongidentifiernamethatgoesonandon"),
        ("nested-lambda-out", "let f : (A : U) -> A -> A = \\A x . x; f"),
        ("proj-out", "let f : (A : U) -> (B : U) -> A -> B -> A = \\A B x y . x; f"),
    ]
    for item in probes:
        label, text = item[0], item[1]
        mode = item[2] if len(item) > 2 else "nf"
        show(label, text, mode)

main()
