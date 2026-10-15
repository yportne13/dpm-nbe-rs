import io

p = 'REPORT.md'
s = io.open(p, encoding='utf-8').read()
tags = {
 "### D1 \u2014 `nf` panics (exit 101) on a normal form containing an absurd-pattern lambda \u2014 **HIGH**":
 "### D1 \u2014 `nf` panics (exit 101) on a normal form containing an absurd-pattern lambda \u2014 **HIGH** \u2014 *build A only, fixed in B (\u2192 N1)*",
 "### D2 \u2014 conversion is not reflexive for stuck primitive (`plus`) neutrals \u2014 **HIGH**":
 "### D2 \u2014 conversion is not reflexive for stuck primitive (`plus`) neutrals \u2014 **HIGH** \u2014 *reproduces on A and B*",
 "### D3 \u2014 printer writes `succ`, parser only accepts `suc`: `nf` output is not valid input \u2014 **HIGH**":
 "### D3 \u2014 printer writes `succ`, parser only accepts `suc`: `nf` output is not valid input \u2014 **HIGH** \u2014 *build A only, fixed in B*",
 "### D4 \u2014 `STATUS_STACK_OVERFLOW` from a 112-byte program, with a ~25 s stall \u2014 **HIGH**":
 "### D4 \u2014 `STATUS_STACK_OVERFLOW` from a 112-byte program, with a ~25 s stall \u2014 **HIGH** \u2014 *reproduces on A and B (~31 s on B)*",
 "### D5 \u2014 depth guard reports the wrong error exactly at the boundary \u2014 **LOW**":
 "### D5 \u2014 depth guard reports the wrong error exactly at the boundary \u2014 **LOW** \u2014 *reproduces on A and B (512 on A, 1 024 on B)*",
 "### D6 \u2014 stdin decoding errors exit 2, everything else exits 1 \u2014 **LOW**":
 "### D6 \u2014 stdin decoding errors exit 2, everything else exits 1 \u2014 **LOW** \u2014 *build A only, fixed in B*",
 "### D7 \u2014 unterminated block comment is silently accepted \u2014 **LOW**":
 "### D7 \u2014 unterminated block comment is silently accepted \u2014 **LOW** \u2014 *build A only, fixed in B*",
 "### D8 \u2014 error renderer echoes the entire source line, so stderr \u2265 the whole line \u2014 **LOW**":
 "### D8 \u2014 error renderer echoes the entire source line, so stderr \u2265 the whole line \u2014 **LOW** \u2014 *reproduces on A and B*",
 "### D9 \u2014 printed normal forms deeper than ~511 successors cannot be re-imported \u2014 **MEDIUM**":
 "### D9 \u2014 printed normal forms deeper than ~511 successors cannot be re-imported \u2014 **MEDIUM** \u2014 *reproduces on A and B*",
}
n = 0
for a, b in tags.items():
    if a in s:
        s = s.replace(a, b)
        n += 1
    else:
        print("NOT FOUND:", a[:70])
io.open(p, 'w', encoding='utf-8').write(s)
print("tagged", n)
