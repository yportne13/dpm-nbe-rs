# Probe table (machine generated)

Binary under test: `bin/pinned-33052DDC.exe` (SHA256 33052DDC9B0E113E6BFC57F04822F932CE14A8574B1C63A0EBD1303415D34854)

Probes: **419**. Raw bytes for every probe: `logs/<id>.out` / `logs/<id>.err`.


## parser (225 probes)

| id | command | stdin (truncated) | exit | stdout | stderr last line | verdict | note |
|---|---|---|---|---|---|---|---|
| P001 | `dpm-nbe nf` | `let` | 1 | `` | `expected '_'` | clean error (exit 1) | keyword 'let' alone as term |
| P002 | `dpm-nbe nf` | `letx` | 1 | `` | `unbound variable: letx` | clean error (exit 1) | identifier looking like let |
| P003 | `dpm-nbe nf` | `let x` | 1 | `` | `expected ':'` | clean error (exit 1) | let without type |
| P004 | `dpm-nbe nf` | `let x :` | 1 | `` | `expected an atom` | clean error (exit 1) | let with dangling colon |
| P005 | `dpm-nbe nf` | `let x : Nat` | 1 | `` | `expected '='` | clean error (exit 1) | let without = |
| P006 | `dpm-nbe nf` | `let x : Nat =` | 1 | `` | `expected an atom` | clean error (exit 1) | let without value |
| P007 | `dpm-nbe nf` | `let x : Nat = zero` | 1 | `` | `expected ';'` | clean error (exit 1) | let without body/; |
| P008 | `dpm-nbe nf` | `let x : Nat = zero;` | 1 | `` | `expected an atom` | clean error (exit 1) | let with empty body |
| P009 | `dpm-nbe nf` | `let x : Nat = zero; x` | 0 | `zero\n  :\nNat\n` | `` | ok | normal let |
| P010 | `dpm-nbe nf` | `let x : Nat = zero;; x` | 1 | `` | `expected an atom` | clean error (exit 1) | double semicolon |
| P011 | `dpm-nbe nf` | `let x : Nat = zero; ; x` | 1 | `` | `expected an atom` | clean error (exit 1) | stray semicolon |
| P012 | `dpm-nbe nf` | `let x : Nat = zero; let y : Nat = x; y` | 0 | `zero\n  :\nNat\n` | `` | ok | two lets |
| P013 | `dpm-nbe nf` | `let x : Nat = zero; let x : Nat = suc x; x` | 0 | `succ zero\n  :\nNat\n` | `` | ok | shadowing |
| P014 | `dpm-nbe nf` | `let x : Nat = zero; let x : Nat = x; x` | 0 | `zero\n  :\nNat\n` | `` | ok | self shadow (recursive?) |
| P015 | `dpm-nbe nf` | `suc` | 1 | `` | `expected an atom` | clean error (exit 1) | bare suc, primitive not fully applied |
| P016 | `dpm-nbe nf` | `sucx` | 1 | `` | `unbound variable: sucx` | clean error (exit 1) | identifier sucx |
| P017 | `dpm-nbe nf` | `suc zero` | 0 | `succ zero\n  :\nNat\n` | `` | ok | suc applied, normal |
| P018 | `dpm-nbe nf` | `suc zero zero` | 1 | `` | `  Nat` | clean error (exit 1) | suc over-applied / trailing junk |
| P019 | `dpm-nbe nf` | `suc (suc zero)` | 0 | `succ (succ zero)\n  :\nNat\n` | `` | ok | normal tower |
| P020 | `dpm-nbe nf` | `U` | 0 | `U\n  :\nU\n` | `` | ok | bare universe |
| P021 | `dpm-nbe nf` | `Ux` | 1 | `` | `unbound variable: Ux` | clean error (exit 1) | identifier Ux |
| P022 | `dpm-nbe nf` | `UU` | 1 | `` | `unbound variable: UU` | clean error (exit 1) | UU |
| P023 | `dpm-nbe nf` | `U U` | 1 | `` | `  U` | clean error (exit 1) | U applied to U |
| P024 | `dpm-nbe nf` | `U0` | 1 | `` | `unbound variable: U0` | clean error (exit 1) | U with digit |
| P025 | `dpm-nbe nf` | `in` | 1 | `` | `unexpected keyword 'in'` | clean error (exit 1) | keyword-looking identifier 'in' |
| P026 | `dpm-nbe nf` | `inx` | 1 | `` | `unbound variable: inx` | clean error (exit 1) | identifier inx |
| P027 | `dpm-nbe nf` | `let in : Nat = zero; in` | 1 | `` | `'in' is a keyword` | clean error (exit 1) | 'in' as binder name |
| P028 | `dpm-nbe nf` | `refl` | 1 | `` | `Can't infer type for refl` | clean error (exit 1) | bare refl, cannot infer |
| P029 | `dpm-nbe nf` | `reflx` | 1 | `` | `unbound variable: reflx` | clean error (exit 1) | identifier reflx |
| P030 | `dpm-nbe nf` | `Nat` | 0 | `Nat\n  :\nU\n` | `` | ok | bare Nat type |
| P031 | `dpm-nbe nf` | `Natx` | 1 | `` | `unbound variable: Natx` | clean error (exit 1) | identifier Natx |
| P032 | `dpm-nbe nf` | `NAT` | 1 | `` | `unbound variable: NAT` | clean error (exit 1) | upper-case NAT |
| P033 | `dpm-nbe nf` | `Zero` | 1 | `` | `unbound variable: Zero` | clean error (exit 1) | upper-case Zero |
| P034 | `dpm-nbe nf` | `zero` | 0 | `zero\n  :\nNat\n` | `` | ok | bare zero |
| P035 | `dpm-nbe nf` | `zerox` | 1 | `` | `unbound variable: zerox` | clean error (exit 1) | identifier zerox |
| P036 | `dpm-nbe nf` | `plus` | 1 | `` | `expected an atom` | clean error (exit 1) | bare plus |
| P037 | `dpm-nbe nf` | `plus zero` | 1 | `` | `expected an atom` | clean error (exit 1) | partially applied plus |
| P038 | `dpm-nbe nf` | `plus zero zero` | 0 | `zero\n  :\nNat\n` | `` | ok | fully applied plus |
| P039 | `dpm-nbe nf` | `plus zero zero zero` | 1 | `` | `  Nat` | clean error (exit 1) | over-applied plus |
| P040 | `dpm-nbe nf` | `plusx zero zero` | 1 | `` | `unbound variable: plusx` | clean error (exit 1) | identifier plusx |
| P041 | `dpm-nbe nf` | `Id` | 1 | `` | `expected an atom` | clean error (exit 1) | bare Id |
| P042 | `dpm-nbe nf` | `Id Nat` | 1 | `` | `expected an atom` | clean error (exit 1) | partially applied Id |
| P043 | `dpm-nbe nf` | `Id Nat zero` | 1 | `` | `expected an atom` | clean error (exit 1) | partially applied Id 2 |
| P044 | `dpm-nbe nf` | `Id Nat zero zero` | 0 | `Id Nat zero zero\n  :\nU\n` | `` | ok | fully applied Id |
| P045 | `dpm-nbe nf` | `Id Nat zero zero zero` | 1 | `` | `  U` | clean error (exit 1) | over-applied Id |
| P046 | `dpm-nbe nf` | `Idx Nat zero zero` | 1 | `` | `unbound variable: Idx` | clean error (exit 1) | identifier Idx |
| P047 | `dpm-nbe nf` | `let Id : Nat = zero; Id` | 1 | `` | `'Id' is a keyword` | clean error (exit 1) | rebind primitive Id |
| P048 | `dpm-nbe nf` | `let suc : Nat = zero; suc zero` | 1 | `` | `'suc' is a keyword` | clean error (exit 1) | rebind primitive suc |
| P049 | `dpm-nbe nf` | `let Nat : U = U; Nat` | 1 | `` | `'Nat' is a keyword` | clean error (exit 1) | rebind Nat |
| P050 | `dpm-nbe nf` | `let U : U = U; U` | 1 | `` | `'U' is a keyword` | clean error (exit 1) | rebind U |
| P051 | `dpm-nbe nf` | `let refl : Nat = zero; refl` | 1 | `` | `'refl' is a keyword` | clean error (exit 1) | rebind refl |
| P052 | `dpm-nbe nf` | `let plus : Nat = zero; plus` | 1 | `` | `'plus' is a keyword` | clean error (exit 1) | rebind plus |
| P053 | `dpm-nbe nf` | `let zero : Nat = suc zero; zero` | 1 | `` | `'zero' is a keyword` | clean error (exit 1) | rebind zero |
| P054 | `dpm-nbe nf` | `let lambda : Nat = zero; lambda` | 0 | `zero\n  :\nNat\n` | `` | ok | identifier lambda |
| P055 | `dpm-nbe nf` | `let f : (A : U) -> A -> A = \\A _ . _; f` | 1 | `` | `expected '.'` | clean error (exit 1) | underscore binder |
| P056 | `dpm-nbe nf` | `let f : (A : U) -> A -> A = \\A _ . A; f` | 1 | `` | `expected '.'` | clean error (exit 1) | underscore binder, A body |
| P057 | `dpm-nbe nf` | `let _ : Nat = zero; zero` | 0 | `zero\n  :\nNat\n` | `` | ok | underscore in let binder |
| P058 | `dpm-nbe nf` | `let x : Nat = zero; _` | 1 | `` | `expected an atom` | clean error (exit 1) | underscore as term |
| P059 | `dpm-nbe nf` | `_` | 1 | `` | `expected an atom` | clean error (exit 1) | bare underscore |
| P060 | `dpm-nbe nf` | `_x` | 1 | `` | `expected an atom` | clean error (exit 1) | underscore-prefixed identifier |
| P061 | `dpm-nbe nf` | `let _x : Nat = zero; _x` | 1 | `` | `expected ':'` | clean error (exit 1) | identifier starting with underscore |
| P062 | `dpm-nbe nf` | `\\x . x` | 1 | `` | `Can't infer type for lambda expression` | clean error (exit 1) | inferred lambda at top level |
| P063 | `dpm-nbe nf` | `\\ x . x` | 1 | `` | `Can't infer type for lambda expression` | clean error (exit 1) | space after backslash |
| P064 | `dpm-nbe nf` | `\\x .` | 1 | `` | `expected an atom` | clean error (exit 1) | empty lambda body |
| P065 | `dpm-nbe nf` | `\\. x` | 1 | `` | `expected at least one pattern` | clean error (exit 1) | no patterns before dot |
| P066 | `dpm-nbe nf` | `\\` | 1 | `` | `expected at least one pattern` | clean error (exit 1) | bare backslash |
| P067 | `dpm-nbe nf` | `\\x` | 1 | `` | `expected '.'` | clean error (exit 1) | backslash pattern, no dot |
| P068 | `dpm-nbe nf` | `\\x. x` | 1 | `` | `Can't infer type for lambda expression` | clean error (exit 1) | no space around dot |
| P069 | `dpm-nbe nf` | `λ x . x` | 1 | `` | `Can't infer type for lambda expression` | clean error (exit 1) | unicode lambda |
| P070 | `dpm-nbe nf` | `λx.x` | 1 | `` | `Can't infer type for lambda expression` | clean error (exit 1) | unicode lambda tight |
| P071 | `dpm-nbe nf` | `λ` | 1 | `` | `expected at least one pattern` | clean error (exit 1) | bare unicode lambda |
| P072 | `dpm-nbe nf` | `λ . x` | 1 | `` | `expected at least one pattern` | clean error (exit 1) | unicode lambda, empty patterns |
| P073 | `dpm-nbe nf` | `let f : (A : U) -> A -> A = λ A x . x; f Nat zero` | 0 | `zero\n  :\nNat\n` | `` | ok | unicode lambda ok |
| P074 | `dpm-nbe nf` | `let f : (A : U) -> A -> A = \\A x . x; f Nat zero` | 0 | `zero\n  :\nNat\n` | `` | ok | ascii lambda ok |
| P075 | `dpm-nbe nf` | `let f : Nat -> Nat = λ n . suc n; f zero` | 0 | `succ zero\n  :\nNat\n` | `` | ok | unicode lambda applied |
| P076 | `dpm-nbe nf` | `let f : (A : U) -> A -> A = \\A x . x; f` | 0 | `λ A x . x\n  :\n(A : U) → A → A\n` | `` | ok | lambda as value (nf is lambda) |
| P077 | `dpm-nbe nf` | `let C : (A : U) -> Id Nat (suc zero) zero -> A =\n  λ A (!);…(64 B)` | 0 | `U\n  :\nU\n` | `` | ok | README absurd example, final term U |
| P078 | `dpm-nbe nf` | `let C : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!); C` | 101 | `` | `thread 'dpm-nbe' panicked ... error: internal panic while elaborating the program` | **RUST PANIC (exit 101)** | absurd lambda as final term |
| P079 | `dpm-nbe type` | `let C : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!); C` | 0 | `(A : U) → Id Nat (succ zero) zero → A\n` | `` | ok | absurd lambda as final term, type mode |
| P080 | `dpm-nbe nf` | `let C : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!); C N…(62 B)` | 101 | `` | `thread 'dpm-nbe' panicked ... error: internal panic while elaborating the program` | **RUST PANIC (exit 101)** | absurd lambda partially applied |
| P081 | `dpm-nbe nf` | `let C : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!); C N…(67 B)` | 1 | `` | `expecting succ zero equals to zero, but it is not` | clean error (exit 1) | absurd lambda applied to refl |
| P082 | `dpm-nbe nf` | `let C : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!); U` | 0 | `U\n  :\nU\n` | `` | ok | absurd unused |
| P083 | `dpm-nbe nf` | `(!)` | 1 | `` | `expected an atom` | clean error (exit 1) | absurd pattern alone |
| P084 | `dpm-nbe nf` | `( ! )` | 1 | `` | `expected an atom` | clean error (exit 1) | spaced absurd pattern alone |
| P085 | `dpm-nbe nf` | `let f : (A : U) -> A -> A = \\A (!); f` | 1 | `` | `  A` | clean error (exit 1) | absurd pattern on non-empty type |
| P086 | `dpm-nbe nf` | `let f : (A : U) -> A -> A = \\A (!); U` | 1 | `` | `  A` | clean error (exit 1) | absurd pattern on non-empty type, checked |
| P087 | `dpm-nbe nf` | `let f : (A : U) -> Id Nat zero zero -> A = \\A refl . A; f Na…(66 B)` | 1 | `` | `  U` | clean error (exit 1) | refl pattern |
| P088 | `dpm-nbe nf` | `let f : (A : U) -> Id Nat zero zero -> A = \\A (!); f` | 1 | `` | `It should not be '(!)' for equation between zero and zero, in fact you should try pattern 'refl'.` | clean error (exit 1) | absurd where refl expected -> panic? |
| P089 | `dpm-nbe nf` | `let f : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!); let…(78 B)` | 101 | `` | `thread 'dpm-nbe' panicked ... error: internal panic while elaborating the program` | **RUST PANIC (exit 101)** | absurd reached through let |
| P090 | `dpm-nbe nf` | `()` | 1 | `` | `expected an atom` | clean error (exit 1) | empty parens |
| P091 | `dpm-nbe nf` | `( )` | 1 | `` | `expected an atom` | clean error (exit 1) | empty parens with space |
| P092 | `dpm-nbe nf` | `(())` | 1 | `` | `expected an atom` | clean error (exit 1) | nested empty parens |
| P093 | `dpm-nbe nf` | `(zero)` | 0 | `zero\n  :\nNat\n` | `` | ok | parenthesised atom |
| P094 | `dpm-nbe nf` | `((zero))` | 0 | `zero\n  :\nNat\n` | `` | ok | double parenthesised atom |
| P095 | `dpm-nbe nf` | `(zero` | 1 | `` | `expected ')'` | clean error (exit 1) | unbalanced open paren |
| P096 | `dpm-nbe nf` | `zero)` | 1 | `` | `expected end of input` | clean error (exit 1) | unbalanced close paren |
| P097 | `dpm-nbe nf` | `)` | 1 | `` | `expected an atom` | clean error (exit 1) | close paren alone |
| P098 | `dpm-nbe nf` | `(` | 1 | `` | `expected an atom` | clean error (exit 1) | open paren alone |
| P099 | `dpm-nbe nf` | `()zero` | 1 | `` | `expected an atom` | clean error (exit 1) | empty parens then atom |
| P100 | `dpm-nbe nf` | `(Nat -> Nat) zero` | 1 | `` | `  U` | clean error (exit 1) | type used as function |
| P101 | `dpm-nbe nf` | `Nat -> Nat` | 0 | `Nat → Nat\n  :\nU\n` | `` | ok | ascii arrow |
| P102 | `dpm-nbe nf` | `Nat → Nat` | 0 | `Nat → Nat\n  :\nU\n` | `` | ok | unicode arrow |
| P103 | `dpm-nbe nf` | `Nat -> Nat → Nat` | 0 | `Nat → Nat → Nat\n  :\nU\n` | `` | ok | mixed arrows |
| P104 | `dpm-nbe nf` | `(A : U) -> A -> A` | 0 | `(A : U) → A → A\n  :\nU\n` | `` | ok | dependent arrow |
| P105 | `dpm-nbe nf` | `(A : U) → A → A` | 0 | `(A : U) → A → A\n  :\nU\n` | `` | ok | dependent unicode arrow |
| P106 | `dpm-nbe nf` | `Nat ->` | 1 | `` | `expected an atom` | clean error (exit 1) | arrow with no codomain |
| P107 | `dpm-nbe nf` | `-> Nat` | 1 | `` | `expected an atom` | clean error (exit 1) | arrow with no domain |
| P108 | `dpm-nbe nf` | `Nat -> -> Nat` | 1 | `` | `expected an atom` | clean error (exit 1) | double arrow |
| P109 | `dpm-nbe nf` | `->` | 1 | `` | `expected an atom` | clean error (exit 1) | arrow alone |
| P110 | `dpm-nbe nf` | `→` | 1 | `` | `expected an atom` | clean error (exit 1) | unicode arrow alone |
| P111 | `dpm-nbe nf` | `Nat-Nat` | 1 | `` | `expected end of input` | clean error (exit 1) | hyphen between |
| P112 | `dpm-nbe nf` | `Nat - > Nat` | 1 | `` | `expected end of input` | clean error (exit 1) | spaced hyphen gt |
| P113 | `dpm-nbe nf` | `Nat > Nat` | 1 | `` | `expected end of input` | clean error (exit 1) | bare gt |
| P114 | `dpm-nbe nf` | `(A : U) -> (B : U) -> A -> B -> A` | 0 | `(A : U)(B : U) → A → B → A\n  :\nU\n` | `` | ok | multi-binder type |
| P115 | `dpm-nbe nf` | `(A B : U) -> A -> B -> A` | 0 | `(A : U)(B : U) → A → B → A\n  :\nU\n` | `` | ok | grouped binders |
| P116 | `dpm-nbe nf` | `(A : U)(B : U) -> A` | 0 | `(A : U)(B : U) → A\n  :\nU\n` | `` | ok | juxtaposed binder groups (printer output!) |
| P117 | `dpm-nbe nf` | `let f : (A : U)(B : U) -> A -> B -> A = \\A B x y . x; f` | 0 | `λ A B x y . x\n  :\n(A : U)(B : U) → A → B → A\n` | `` | ok | juxtaposed binder groups in source |
| P118 | `dpm-nbe nf` | `zero -- trailing comment` | 0 | `zero\n  :\nNat\n` | `` | ok | line comment |
| P119 | `dpm-nbe nf` | `-- only a comment` | 1 | `` | `expected an atom` | clean error (exit 1) | comment-only input |
| P120 | `dpm-nbe nf` | `--` | 1 | `` | `expected an atom` | clean error (exit 1) | bare -- |
| P121 | `dpm-nbe nf` | `----` | 1 | `` | `expected an atom` | clean error (exit 1) | four dashes |
| P122 | `dpm-nbe nf` | `zero --` | 0 | `zero\n  :\nNat\n` | `` | ok | -- at very EOF |
| P123 | `dpm-nbe nf` | `zero --` | 0 | `zero\n  :\nNat\n` | `` | ok | -- at EOF, no newline |
| P124 | `dpm-nbe nf` | `zero --)` | 0 | `zero\n  :\nNat\n` | `` | ok | -- swallows paren |
| P125 | `dpm-nbe nf` | `zero --(-)` | 0 | `zero\n  :\nNat\n` | `` | ok | -- swallows block open |
| P126 | `dpm-nbe nf` | `(--)` | 1 | `` | `expected an atom` | clean error (exit 1) | line comment starting with ( |
| P127 | `dpm-nbe nf` | `{- -}` | 1 | `` | `expected an atom` | clean error (exit 1) | empty block comment alone |
| P128 | `dpm-nbe nf` | `{- comment -} zero` | 0 | `zero\n  :\nNat\n` | `` | ok | block comment then atom |
| P129 | `dpm-nbe nf` | `zero {- comment -}` | 0 | `zero\n  :\nNat\n` | `` | ok | trailing block comment |
| P130 | `dpm-nbe nf` | `zero {- unterminated` | 0 | `zero\n  :\nNat\n` | `` | ok | unterminated block comment |
| P131 | `dpm-nbe nf` | `{- unterminated` | 1 | `` | `expected an atom` | clean error (exit 1) | unterminated block comment only |
| P132 | `dpm-nbe nf` | `{- {- nested -} -} zero` | 1 | `` | `expected an atom` | clean error (exit 1) | nested block comment |
| P133 | `dpm-nbe nf` | `{- {- -} zero` | 0 | `zero\n  :\nNat\n` | `` | ok | nested block comment, unbalanced |
| P134 | `dpm-nbe nf` | `-}` | 1 | `` | `expected an atom` | clean error (exit 1) | close block comment alone |
| P135 | `dpm-nbe nf` | `zero -}` | 1 | `` | `expected end of input` | clean error (exit 1) | stray close block comment |
| P136 | `dpm-nbe nf` | `{--}` | 1 | `` | `expected an atom` | clean error (exit 1) | comment like {--} |
| P137 | `dpm-nbe nf` | `{-` | 1 | `` | `expected an atom` | clean error (exit 1) | open block comment alone |
| P138 | `dpm-nbe nf` | `{- --}` | 1 | `` | `expected an atom` | clean error (exit 1) | line comment inside block comment |
| P139 | `dpm-nbe nf` | `zero {- -} {- -} zero` | 1 | `` | `  Nat` | clean error (exit 1) | trailing junk after comments |
| P140 | `dpm-nbe nf` | `-- zero\nzero` | 0 | `zero\n  :\nNat\n` | `` | ok | commented-out program then real term |
| P141 | `dpm-nbe nf` | `let x : Nat = zero;\r\nx\r\n` | 0 | `zero\n  :\nNat\n` | `` | ok | CRLF program |
| P142 | `dpm-nbe nf` | `let x : Nat = zero;\rx` | 0 | `zero\n  :\nNat\n` | `` | ok | lone CR separator |
| P143 | `dpm-nbe nf` | `\tzero\t` | 0 | `zero\n  :\nNat\n` | `` | ok | tab padding |
| P144 | `dpm-nbe nf` | `let\tx\t:\tNat\t=\tzero\t;\tx` | 0 | `zero\n  :\nNat\n` | `` | ok | tabs as token separators |
| P145 | `dpm-nbe nf` | `\n\n\n  zero  \n\n` | 0 | `zero\n  :\nNat\n` | `` | ok | blank lines around |
| P146 | `dpm-nbe nf` | `   zero` | 0 | `zero\n  :\nNat\n` | `` | ok | leading spaces |
| P147 | `dpm-nbe nf` | `zero   ` | 0 | `zero\n  :\nNat\n` | `` | ok | trailing spaces |
| P148 | `dpm-nbe nf` | `` | 1 | `` | `expected an atom` | clean error (exit 1) | empty input |
| P149 | `dpm-nbe nf` | `\n` | 1 | `` | `expected an atom` | clean error (exit 1) | single newline |
| P150 | `dpm-nbe nf` | ` ` | 1 | `` | `expected an atom` | clean error (exit 1) | single space |
| P151 | `dpm-nbe nf` | `﻿zero` | 1 | `` | `expected an atom` | clean error (exit 1) | UTF-8 BOM then atom |
| P152 | `dpm-nbe nf` | `zero﻿` | 1 | `` | `expected end of input` | clean error (exit 1) | BOM after atom |
| P153 | `dpm-nbe nf` | `let ﻿x : Nat = zero; x` | 1 | `` | `expected '_'` | clean error (exit 1) | BOM inside program |
| P154 | `dpm-nbe nf` | `zero ` | 0 | `zero\n  :\nNat\n` | `` | ok | non-breaking space after atom |
| P155 | `dpm-nbe nf` | `zero​` | 1 | `` | `expected end of input` | clean error (exit 1) | zero-width space after atom |
| P156 | `dpm-nbe nf` | `zero﻿x` | 1 | `` | `expected end of input` | clean error (exit 1) | BOM mid-token |
| P157 | `dpm-nbe nf` | `zero\0` | 1 | `` | `expected end of input` | clean error (exit 1) | NUL byte after atom |
| P158 | `dpm-nbe nf` | `\0zero` | 1 | `` | `expected an atom` | clean error (exit 1) | NUL byte before atom |
| P159 | `dpm-nbe nf` | `zero�` | 2 | `` | `error: cannot read stdin: stream did not contain valid UTF-8` | exit 2 (stdin not UTF-8) | invalid UTF-8 byte |
| P160 | `dpm-nbe nf` | `﻿` | 1 | `` | `expected an atom` | clean error (exit 1) | BOM only |
| P161 | `dpm-nbe nf` | `��` | 2 | `` | `error: cannot read stdin: stream did not contain valid UTF-8` | exit 2 (stdin not UTF-8) | UTF-16 LE BOM |
| P162 | `dpm-nbe nf` | `zero\n extra` | 1 | `` | `  Nat` | clean error (exit 1) | unicode line separator |
| P163 | `dpm-nbe nf` | `let α : Nat = zero; α` | 0 | `zero\n  :\nNat\n` | `` | ok | greek identifier |
| P164 | `dpm-nbe nf` | `let αβγ : Nat = zero; αβγ` | 0 | `zero\n  :\nNat\n` | `` | ok | multi greek identifier |
| P165 | `dpm-nbe nf` | `αβγ` | 1 | `` | `unbound variable: αβγ` | clean error (exit 1) | unbound greek identifier |
| P166 | `dpm-nbe nf` | `let 日本 : Nat = zero; 日本` | 0 | `zero\n  :\nNat\n` | `` | ok | CJK identifier |
| P167 | `dpm-nbe nf` | `let x1 : Nat = zero; x1` | 0 | `zero\n  :\nNat\n` | `` | ok | digit-suffixed identifier |
| P168 | `dpm-nbe nf` | `123` | 1 | `` | `unbound variable: 123` | clean error (exit 1) | digits-only token |
| P169 | `dpm-nbe nf` | `let 123 : Nat = zero; 123` | 0 | `zero\n  :\nNat\n` | `` | ok | digits-only binder |
| P170 | `dpm-nbe nf` | `1abc` | 1 | `` | `unbound variable: 1abc` | clean error (exit 1) | digit-leading identifier |
| P171 | `dpm-nbe nf` | `let 1abc : Nat = zero; 1abc` | 0 | `zero\n  :\nNat\n` | `` | ok | digit-leading binder |
| P172 | `dpm-nbe nf` | `x'` | 1 | `` | `expected end of input` | clean error (exit 1) | prime identifier |
| P173 | `dpm-nbe nf` | `let x' : Nat = zero; x'` | 1 | `` | `expected ':'` | clean error (exit 1) | prime binder |
| P174 | `dpm-nbe nf` | `x?` | 1 | `` | `expected end of input` | clean error (exit 1) | question-mark identifier |
| P175 | `dpm-nbe nf` | `let x_y : Nat = zero; x_y` | 1 | `` | `expected ':'` | clean error (exit 1) | underscore inside identifier |
| P176 | `dpm-nbe nf` | `let x-y : Nat = zero; x-y` | 1 | `` | `expected ':'` | clean error (exit 1) | hyphen inside identifier |
| P177 | `dpm-nbe nf` | `let x.y : Nat = zero; x.y` | 1 | `` | `expected ':'` | clean error (exit 1) | dot inside identifier |
| P178 | `dpm-nbe nf` | `let x:y : Nat = zero; x:y` | 1 | `` | `expected '='` | clean error (exit 1) | colon inside identifier |
| P179 | `dpm-nbe nf` | `let λx : Nat = zero; λx` | 1 | `` | `expected '.'` | clean error (exit 1) | lambda char inside identifier |
| P180 | `dpm-nbe nf` | `let x_λ : Nat = zero; x_λ` | 1 | `` | `expected ':'` | clean error (exit 1) | lambda char in identifier tail |
| P181 | `dpm-nbe nf` | `let x→y : Nat = zero; x→y` | 1 | `` | `expected ':'` | clean error (exit 1) | arrow char inside identifier |
| P182 | `dpm-nbe nf` | `let x=1 : Nat = zero; x=1` | 1 | `` | `expected ':'` | clean error (exit 1) | equals inside identifier |
| P183 | `dpm-nbe nf` | `aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa…(100000 B)` | 1 | `` | `unbound variable: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa` | clean error (exit 1) | 100k char unbound identifier |
| P184 | `dpm-nbe nf` | `let aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa…(200019 B)` | 0 | `zero\n  :\nNat\n` | `` | ok | 100k char binder + reference |
| P185 | `dpm-nbe nf` | `let aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa…(2000019 B)` | 0 | `zero\n  :\nNat\n` | `` | ok | 1M char binder + reference |
| P186 | `dpm-nbe nf` | `aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa…(1000000 B)` | 1 | `` | `unbound variable: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa` | clean error (exit 1) | 1M char unbound identifier |
| P187 | `dpm-nbe nf` | `zero;` | 1 | `` | `expected end of input` | clean error (exit 1) | trailing semicolon |
| P188 | `dpm-nbe nf` | `;zero` | 1 | `` | `expected an atom` | clean error (exit 1) | leading semicolon |
| P189 | `dpm-nbe nf` | `;;` | 1 | `` | `expected an atom` | clean error (exit 1) | double semicolon only |
| P190 | `dpm-nbe nf` | `.` | 1 | `` | `expected an atom` | clean error (exit 1) | dot alone |
| P191 | `dpm-nbe nf` | `:` | 1 | `` | `expected an atom` | clean error (exit 1) | colon alone |
| P192 | `dpm-nbe nf` | `=` | 1 | `` | `expected an atom` | clean error (exit 1) | equals alone |
| P193 | `dpm-nbe nf` | `zero.zero` | 1 | `` | `expected end of input` | clean error (exit 1) | dot accessor |
| P194 | `dpm-nbe nf` | `let x : Nat = zero; x.` | 1 | `` | `expected end of input` | clean error (exit 1) | trailing dot |
| P195 | `dpm-nbe nf` | `"zero"` | 1 | `` | `expected an atom` | clean error (exit 1) | string literal |
| P196 | `dpm-nbe nf` | `'z'` | 1 | `` | `expected an atom` | clean error (exit 1) | char literal |
| P197 | `dpm-nbe nf` | `#zero` | 1 | `` | `expected an atom` | clean error (exit 1) | hash comment attempt |
| P198 | `dpm-nbe nf` | `zero # junk` | 1 | `` | `expected end of input` | clean error (exit 1) | hash mid |
| P199 | `dpm-nbe nf` | `zero % junk` | 1 | `` | `expected end of input` | clean error (exit 1) | percent |
| P200 | `dpm-nbe nf` | `zero @ junk` | 1 | `` | `expected end of input` | clean error (exit 1) | at sign |
| P201 | `dpm-nbe nf` | `zero ~ junk` | 1 | `` | `expected end of input` | clean error (exit 1) | tilde |
| P202 | `dpm-nbe nf` | `zero & junk` | 1 | `` | `expected end of input` | clean error (exit 1) | ampersand |
| P203 | `dpm-nbe nf` | `zero \| junk` | 1 | `` | `expected end of input` | clean error (exit 1) | pipe |
| P204 | `dpm-nbe nf` | `zero $ junk` | 1 | `` | `expected end of input` | clean error (exit 1) | dollar |
| P205 | `dpm-nbe nf` | `zero ? junk` | 1 | `` | `expected end of input` | clean error (exit 1) | question |
| P206 | `dpm-nbe nf` | `let x : Nat = zero in x` | 1 | `` | `unexpected keyword 'in'` | clean error (exit 1) | ML-style 'in' |
| P207 | `dpm-nbe nf` | `let x := zero; x` | 1 | `` | `expected an atom` | clean error (exit 1) | := assignment |
| P208 | `dpm-nbe nf` | `let x : Nat <= zero; x` | 1 | `` | `expected '='` | clean error (exit 1) | <= assignment |
| P209 | `dpm-nbe nf` | `Nat :: Nat` | 1 | `` | `expected end of input` | clean error (exit 1) | double colon |
| P210 | `dpm-nbe nf` | `(A : U) => A` | 1 | `` | `expected '->'` | clean error (exit 1) | fat arrow |
| P211 | `dpm-nbe nf` | `forall (A : U) . A` | 1 | `` | `expected ')'` | clean error (exit 1) | forall keyword |
| P212 | `dpm-nbe nf` | `let x : Nat = zero; x x` | 1 | `` | `  Nat` | clean error (exit 1) | apply a Nat |
| P213 | `dpm-nbe nf` | `refl refl` | 1 | `` | `Can't infer type for refl` | clean error (exit 1) | refl applied |
| P214 | `dpm-nbe nf` | `Nat Nat` | 1 | `` | `  U` | clean error (exit 1) | Nat applied |
| P215 | `dpm-nbe nf` | `U U U` | 1 | `` | `  U` | clean error (exit 1) | U applied twice |
| P216 | `dpm-nbe nf` | `suc refl` | 1 | `` | `Can't infer type for refl` | clean error (exit 1) | suc of refl |
| P217 | `dpm-nbe nf` | `plus U U` | 1 | `` | `  U` | clean error (exit 1) | plus of U |
| P218 | `dpm-nbe nf` | `Id U U U` | 0 | `Id U U U\n  :\nU\n` | `` | ok | Id in U |
| P219 | `dpm-nbe nf` | `let f : Nat -> Nat = \\n . n; f f` | 1 | `` | `  Nat → Nat` | clean error (exit 1) | self application |
| P220 | `dpm-nbe nf` | `let f : Nat -> Nat = \\n . n; let f : Nat -> Nat = f; f zero` | 0 | `zero\n  :\nNat\n` | `` | ok | recursive-looking shadowing |
| P221 | `dpm-nbe nf` | `let x : U = Nat; x -> x` | 0 | `Nat → Nat\n  :\nU\n` | `` | ok | type-level let then arrow |
| P222 | `dpm-nbe nf` | `let x : Nat = zero; x zero` | 1 | `` | `  Nat` | clean error (exit 1) | apply Nat value |
| P223 | `dpm-nbe nf` | `\r\n\r\n` | 1 | `` | `expected an atom` | clean error (exit 1) | only CRLFs |
| P224 | `dpm-nbe nf` | `zero` | 0 | `zero\n  :\nNat\n` | `` | ok | form feed before atom |
| P225 | `dpm-nbe nf` | `zero` | 0 | `zero\n  :\nNat\n` | `` | ok | vertical tab before atom |

## depth (58 probes)

| id | command | stdin (truncated) | exit | stdout | stderr last line | verdict | note |
|---|---|---|---|---|---|---|---|
| P226 | `dpm-nbe nf` | `((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((…(204 B)` | 0 | `zero\n  :\nNat\n` | `` | ok | 100 nested parens |
| P227 | `dpm-nbe nf` | `((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((…(1004 B)` | 0 | `zero\n  :\nNat\n` | `` | ok | 500 nested parens |
| P228 | `dpm-nbe nf` | `((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((…(1028 B)` | 1 | `` | `'zero' is a keyword` | clean error (exit 1) | 512 nested parens |
| P229 | `dpm-nbe nf` | `((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((…(1030 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | 513 nested parens |
| P230 | `dpm-nbe nf` | `((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((…(2004 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | 1000 nested parens |
| P231 | `dpm-nbe nf` | `((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((…(10004 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | 5000 nested parens |
| P232 | `dpm-nbe nf` | `((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((…(20004 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | 10000 nested parens |
| P233 | `dpm-nbe nf` | `((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((…(100004 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | 50000 nested parens |
| P234 | `dpm-nbe nf` | `let x : Nat = ((((((((((((((((((((((((((((((((((((((((((((((…(225 B)` | 0 | `succ zero\n  :\nNat\n` | `` | ok | 100 nested parens around suc zero |
| P235 | `dpm-nbe nf` | `let x : Nat = ((((((((((((((((((((((((((((((((((((((((((((((…(2025 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | 1000 nested parens around suc zero |
| P236 | `dpm-nbe nf` | `let x : Nat = ((((((((((((((((((((((((((((((((((((((((((((((…(20025 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | 10000 nested parens around suc zero |
| P237 | `dpm-nbe nf` | `let f : Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat…(1105 B)` | 1 | `` | `Too many patterns.` | clean error (exit 1) | 100-arrow type, 100-pattern lambda |
| P238 | `dpm-nbe nf` | `let f : Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat…(5905 B)` | 1 | `` | `Too many patterns.` | clean error (exit 1) | 500-arrow type, 500-pattern lambda |
| P239 | `dpm-nbe nf` | `let f : Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat…(11905 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | 1000-arrow type, 1000-pattern lambda |
| P240 | `dpm-nbe nf` | `let f : Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat…(24905 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | 2000-arrow type, 2000-pattern lambda |
| P241 | `dpm-nbe nf` | `let f : Nat -> Nat = \\n . n; f f f f f f f f f f f f f f f f…(233 B)` | 1 | `` | `  Nat → Nat` | clean error (exit 1) | 100 nested f(...) applications (flat) |
| P242 | `dpm-nbe nf` | `let f : Nat -> Nat = \\n . n; f f f f f f f f f f f f f f f f…(1033 B)` | 1 | `` | `  Nat → Nat` | clean error (exit 1) | 500 nested f(...) applications (flat) |
| P243 | `dpm-nbe nf` | `let f : Nat -> Nat = \\n . n; f f f f f f f f f f f f f f f f…(2033 B)` | 1 | `` | `  Nat → Nat` | clean error (exit 1) | 1000 nested f(...) applications (flat) |
| P244 | `dpm-nbe nf` | `let f : Nat -> Nat = \\n . n; f f f f f f f f f f f f f f f f…(10033 B)` | 1 | `` | `  Nat → Nat` | clean error (exit 1) | 5000 nested f(...) applications (flat) |
| P245 | `dpm-nbe nf` | `let f : Nat -> Nat = \\n . n; f f f f f f f f f f f f f f f f…(20033 B)` | 1 | `` | `  Nat → Nat` | clean error (exit 1) | 10000 nested f(...) applications (flat) |
| P246 | `dpm-nbe nf` | `let f : Nat -> Nat = \\n . n; f (f (f (f (f (f (f (f (f (f (f…(433 B)` | 0 | `zero\n  :\nNat\n` | `` | ok | 100 nested f(...) applications (parenthesised) |
| P247 | `dpm-nbe nf` | `let f : Nat -> Nat = \\n . n; f (f (f (f (f (f (f (f (f (f (f…(2033 B)` | 0 | `zero\n  :\nNat\n` | `` | ok | 500 nested f(...) applications (parenthesised) |
| P248 | `dpm-nbe nf` | `let f : Nat -> Nat = \\n . n; f (f (f (f (f (f (f (f (f (f (f…(4033 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | 1000 nested f(...) applications (parenthesised) |
| P249 | `dpm-nbe nf` | `let f : Nat -> Nat = \\n . n; f (f (f (f (f (f (f (f (f (f (f…(20033 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | 5000 nested f(...) applications (parenthesised) |
| P250 | `dpm-nbe nf` | `let f : Nat -> Nat = \\n . n; f (f (f (f (f (f (f (f (f (f (f…(40033 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | 10000 nested f(...) applications (parenthesised) |
| P251 | `dpm-nbe nf` | `suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (…(604 B)` | 0 | `succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (su` | `` | ok | suc tower depth 100 |
| P252 | `dpm-nbe nf` | `suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (…(6004 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | suc tower depth 1000 |
| P253 | `dpm-nbe nf` | `suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (…(60004 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | suc tower depth 10000 |
| P254 | `dpm-nbe nf` | `suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (…(600004 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | suc tower depth 100000 |
| P255 | `dpm-nbe nf` | `plus zero (plus zero (plus zero (plus zero (plus zero (plus …(604 B)` | 0 | `zero\n  :\nNat\n` | `` | ok | plus tower depth 50 |
| P256 | `dpm-nbe nf` | `plus zero (plus zero (plus zero (plus zero (plus zero (plus …(2404 B)` | 0 | `zero\n  :\nNat\n` | `` | ok | plus tower depth 200 |
| P257 | `dpm-nbe nf` | `plus zero (plus zero (plus zero (plus zero (plus zero (plus …(12004 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | plus tower depth 1000 |
| P258 | `dpm-nbe nf` | `plus zero (plus zero (plus zero (plus zero (plus zero (plus …(60004 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | plus tower depth 5000 |
| P259 | `dpm-nbe nf` | `let x : Id (Id (Id (Id (Id (Id (Id (Id (Id (Id (Id (Id (Id (…(771 B)` | 1 | `` | `Can't infer type for refl` | clean error (exit 1) | 50 nested Id types |
| P260 | `dpm-nbe nf` | `let x : Id (Id (Id (Id (Id (Id (Id (Id (Id (Id (Id (Id (Id (…(3021 B)` | 1 | `` | `Can't infer type for refl` | clean error (exit 1) | 200 nested Id types |
| P261 | `dpm-nbe nf` | `let x : Id (Id (Id (Id (Id (Id (Id (Id (Id (Id (Id (Id (Id (…(15021 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | 1000 nested Id types |
| P262 | `dpm-nbe nf` | `Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat …(696 B)` | 0 | `Nat → Nat → Nat → Nat → Nat → Nat → Nat → Nat → Nat → Nat → Nat → Nat → Nat → Nat → Nat → Nat → Nat → Nat → Na` | `` | ok | 100-arrow chain |
| P263 | `dpm-nbe nf` | `Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat …(3496 B)` | 0 | `Nat → Nat → Nat → Nat → Nat → Nat → Nat → Nat → Nat → Nat → Nat → Nat → Nat → Nat → Nat → Nat → Nat → Nat → Na` | `` | ok | 500-arrow chain |
| P264 | `dpm-nbe nf` | `Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat …(6996 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | 1000-arrow chain |
| P265 | `dpm-nbe nf` | `Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat …(34996 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | 5000-arrow chain |
| P266 | `dpm-nbe nf` | `Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat …(69996 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | 10000-arrow chain |
| P267 | `dpm-nbe nf` | `Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat …(349996 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | 50000-arrow chain |
| P268 | `dpm-nbe nf` | `let f : Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat…(7016 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | 1000-arrow chain bound to 1-pattern lambda (ill-typed, parse depth) |
| P269 | `dpm-nbe nf` | `let f : Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat…(70016 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | 10000-arrow chain bound to 1-pattern lambda (ill-typed, parse depth) |
| P270 | `dpm-nbe nf` | `let x0 : Nat = zero; let x1 : Nat = zero; let x2 : Nat = zer…(2192 B)` | 0 | `zero\n  :\nNat\n` | `` | ok | 100 sequential lets |
| P271 | `dpm-nbe nf` | `let x0 : Nat = zero; let x1 : Nat = zero; let x2 : Nat = zer…(22892 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | 1000 sequential lets |
| P272 | `dpm-nbe nf` | `let x0 : Nat = zero; let x1 : Nat = zero; let x2 : Nat = zer…(118892 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | 5000 sequential lets |
| P273 | `dpm-nbe nf` | `let x : Nat = suc x; let x : Nat = suc x; let x : Nat = suc …(2101 B)` | 1 | `` | `unbound variable: x` | clean error (exit 1) | 100 self-shadowing lets |
| P274 | `dpm-nbe nf` | `let x : Nat = suc x; let x : Nat = suc x; let x : Nat = suc …(21001 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | 1000 self-shadowing lets |
| P275 | `dpm-nbe nf` | `let x : Nat = suc x; let x : Nat = suc x; let x : Nat = suc …(105001 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | 5000 self-shadowing lets |
| P276 | `dpm-nbe nf` | `let x : Nat = suc zero; let x : Nat = suc zero; let x : Nat …(24001 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | 1000 shadowing lets of same name |
| P277 | `dpm-nbe nf` | `let x : Nat = suc zero; let x : Nat = suc zero; let x : Nat …(240001 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | 10000 shadowing lets of same name |
| P278 | `dpm-nbe nf` | `let x : ((((((((((((((((((((((((((((((((((((((((((((((((((((…(2021 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | 1000 nested parens in type position |
| P279 | `dpm-nbe nf` | `let x : ((((((((((((((((((((((((((((((((((((((((((((((((((((…(20021 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | 10000 nested parens in type position |
| P280 | `dpm-nbe nf` | `plus zero zero zero zero zero zero zero zero zero zero zero …(5014 B)` | 1 | `` | `  Nat` | clean error (exit 1) | plus followed by 1000 extra atoms |
| P281 | `dpm-nbe nf` | `plus zero zero zero zero zero zero zero zero zero zero zero …(50014 B)` | 1 | `` | `  Nat` | clean error (exit 1) | plus followed by 10000 extra atoms |
| P282 | `dpm-nbe nf` | `let x : Nat = zero; (\\y . (\\y . (\\y . (\\y . (\\y . (\\y . (\\y …(721 B)` | 1 | `` | `Can't infer type for lambda expression` | clean error (exit 1) | 100 nested parenthesised lambdas |
| P283 | `dpm-nbe nf` | `let x : Nat = zero; (\\y . (\\y . (\\y . (\\y . (\\y . (\\y . (\\y …(3521 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | 500 nested parenthesised lambdas |

## blowup (10 probes)

| id | command | stdin (truncated) | exit | stdout | stderr last line | verdict | note |
|---|---|---|---|---|---|---|---|
| P387 | `dpm-nbe nf` | `let d : Nat -> Nat = \\n . plus n n; d (d (d (d (d (d (d (d (…(84 B)` | 0 | `succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (su` | `` | ok | doubling chain k=10 -> 2^10 succs |
| P388 | `dpm-nbe nf` | `let d : Nat -> Nat = \\n . plus n n; d (d (d (d (d (d (d (d (…(92 B)` | 0 | `succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (su` | `` | ok | doubling chain k=12 -> 2^12 succs |
| P389 | `dpm-nbe nf` | `let d : Nat -> Nat = \\n . plus n n; d (d (d (d (d (d (d (d (…(100 B)` | 0 | `succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (su` | `` | ok | doubling chain k=14 -> 2^14 succs |
| P390 | `dpm-nbe nf` | `let d : Nat -> Nat = \\n . plus n n; d (d (d (d (d (d (d (d (…(108 B)` | 0 | `succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (su` | `` | ok | doubling chain k=16 -> 2^16 succs |
| P391 | `dpm-nbe nf` | `let d : Nat -> Nat = \\n . plus n n; d (d (d (d (d (d (d (d (…(112 B)` | 3221225725 | `` | `thread 'dpm-nbe' has overflowed its stack` | **STATUS_STACK_OVERFLOW (0xC00000FD)** | doubling k=17 -> 2^17 succs (suspected stack overflow) |
| P392 | `dpm-nbe nf` | `let d : Nat -> Nat = \\n . plus n n; d (d (d (d (d (d (d (d (…(116 B)` | 3221225725 | `` | `thread 'dpm-nbe' has overflowed its stack` | **STATUS_STACK_OVERFLOW (0xC00000FD)** | doubling k=18 -> 2^18 succs |
| P393 | `dpm-nbe nf` | `let x : Id Nat (suc (suc (suc (suc (suc (suc (suc (suc (suc …(4838 B)` | 0 | `refl\n  :\nId Nat (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (s…` | `` | ok | refl between two 400-deep towers |
| P394 | `dpm-nbe nf` | `let d : Nat -> Nat = \\n . plus n n; let a : Nat = d (d (d (d…(192 B)` | 0 | `refl\n  :\nId Nat (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (s…` | `` | ok | refl between two computed 2^10 towers |
| P395 | `dpm-nbe nf` | `suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (…(3004 B)` | 0 | `succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (su` | `` | ok | 500-deep suc tower (max under depth limit) |
| P396 | `dpm-nbe nf` | `let d : Nat -> Nat = \\n . plus n n; let v0 : Nat = d (d (d (…(1189 B)` | 0 | `succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (su` | `` | ok | 20 x 2^8 towers in scope |

## semantic (51 probes)

| id | command | stdin (truncated) | exit | stdout | stderr last line | verdict | note |
|---|---|---|---|---|---|---|---|
| P284 | `dpm-nbe nf` | `plus (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc …(1218 B)` | 0 | `succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (su` | `` | ok | plus 100 100 |
| P285 | `dpm-nbe nf` | `plus (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc …(12018 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | plus 1000 1000 |
| P286 | `dpm-nbe nf` | `plus (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc …(60018 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | plus 5000 5000 |
| P287 | `dpm-nbe nf` | `plus (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc …(6016 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | plus 1000 zero |
| P288 | `dpm-nbe nf` | `plus (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc …(60016 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | plus 10000 zero |
| P289 | `dpm-nbe nf` | `plus (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc …(600016 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | plus 100000 zero |
| P290 | `dpm-nbe nf` | `suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (…(600004 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | suc tower 100000 as normal form |
| P291 | `dpm-nbe nf` | `suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (…(6000004 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | suc tower 1000000 as normal form |
| P292 | `dpm-nbe nf` | `plus (plus (suc zero) (suc zero)) (plus (suc zero) (suc zero…(62 B)` | 0 | `succ (succ (succ (succ zero)))\n  :\nNat\n` | `` | ok | nested plus |
| P293 | `dpm-nbe nf` | `plus (suc (suc (suc zero))) (suc (suc zero))` | 0 | `succ (succ (succ (succ (succ zero))))\n  :\nNat\n` | `` | ok | plus 3 2 |
| P294 | `dpm-nbe nf` | `plus zero (plus zero (plus zero (suc zero)))` | 0 | `succ zero\n  :\nNat\n` | `` | ok | plus 0 0 0 1 |
| P295 | `dpm-nbe nf` | `let n : Nat = suc (suc zero); plus n n` | 0 | `succ (succ (succ (succ zero)))\n  :\nNat\n` | `` | ok | plus of let-bound n |
| P296 | `dpm-nbe nf` | `let d : Nat -> Nat = \\n . plus n n; d (d (d (d (d (d (d (d (…(80 B)` | 0 | `zero\n  :\nNat\n` | `` | ok | doubling chain k=10 (2^10) |
| P297 | `dpm-nbe nf` | `let d : Nat -> Nat = \\n . plus n n; d (d (d (d (d (d (d (d (…(104 B)` | 0 | `zero\n  :\nNat\n` | `` | ok | doubling chain k=16 (2^16) |
| P298 | `dpm-nbe nf` | `let d : Nat -> Nat = \\n . plus n n; d (d (d (d (d (d (d (d (…(120 B)` | 0 | `zero\n  :\nNat\n` | `` | ok | doubling chain k=20 (2^20) |
| P299 | `dpm-nbe nf` | `let d : Nat -> Nat = \\n . plus n n; d (d (d (d (d (d (d (d (…(128 B)` | 0 | `zero\n  :\nNat\n` | `` | ok | doubling chain k=22 (2^22) |
| P300 | `dpm-nbe nf` | `let d : Nat -> Nat = \\n . plus n n; d (d (d (d (d (d (d (d (…(136 B)` | 0 | `zero\n  :\nNat\n` | `` | ok | doubling chain k=24 (2^24) |
| P301 | `dpm-nbe nf` | `let d : Nat -> Nat = \\n . plus n n; d (d (d (d (d (d (d (d (…(144 B)` | 0 | `zero\n  :\nNat\n` | `` | ok | doubling chain k=26 (2^26) |
| P302 | `dpm-nbe nf` | `let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> I…(137 B)` | 0 | `refl\n  :\nId Nat zero zero\n` | `` | ok | chained trans n=1 |
| P303 | `dpm-nbe nf` | `let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> I…(169 B)` | 0 | `refl\n  :\nId Nat zero zero\n` | `` | ok | chained trans n=2 |
| P304 | `dpm-nbe nf` | `let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> I…(201 B)` | 0 | `refl\n  :\nId Nat zero zero\n` | `` | ok | chained trans n=3 |
| P305 | `dpm-nbe nf` | `let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> I…(265 B)` | 0 | `refl\n  :\nId Nat zero zero\n` | `` | ok | chained trans n=5 |
| P306 | `dpm-nbe nf` | `let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> I…(361 B)` | 0 | `refl\n  :\nId Nat zero zero\n` | `` | ok | chained trans n=8 |
| P307 | `dpm-nbe nf` | `let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> I…(489 B)` | 0 | `refl\n  :\nId Nat zero zero\n` | `` | ok | chained trans n=12 |
| P308 | `dpm-nbe nf` | `let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> I…(745 B)` | 0 | `refl\n  :\nId Nat zero zero\n` | `` | ok | chained trans n=20 |
| P309 | `dpm-nbe nf` | `let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> I…(257 B)` | 0 | `refl\n  :\nId Nat zero zero\n` | `` | ok | chained trans with variables n=4 |
| P310 | `dpm-nbe nf` | `let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> I…(441 B)` | 0 | `refl\n  :\nId Nat zero zero\n` | `` | ok | chained trans with variables n=12 |
| P311 | `dpm-nbe nf` | `let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m ze…(110 B)` | 0 | `λ m n refl refl . refl\n  :\n(m : Nat)(n : Nat) → Id Nat n (plus m zero) → Id Nat m zero → Id Nat n zero\n` | `` | ok | README H example as final term (nf is lambda) |
| P312 | `dpm-nbe nf` | `let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m ze…(110 B)` | 0 | `U\n  :\nU\n` | `` | ok | README H example, final term U |
| P313 | `dpm-nbe nf` | `let f : Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat…(1105 B)` | 1 | `` | `Too many patterns.` | clean error (exit 1) | lambda with 100 binders as nf |
| P314 | `dpm-nbe nf` | `let f : Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat…(11905 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | lambda with 1000 binders as nf |
| P315 | `dpm-nbe nf` | `let f : Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat…(63905 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | lambda with 5000 binders as nf |
| P316 | `dpm-nbe nf` | `let v0 : Nat = suc zero; let v1 : Nat = suc zero; let v2 : N…(2613 B)` | 0 | `succ (succ zero)\n  :\nNat\n` | `` | ok | 100 lets all in scope |
| P317 | `dpm-nbe nf` | `let v0 : Nat = suc zero; let v1 : Nat = suc zero; let v2 : N…(26914 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | 1000 lets all in scope |
| P318 | `dpm-nbe nf` | `let v0 : Nat = zero; let v1 : Nat = suc v0; let v2 : Nat = s…(2480 B)` | 0 | `succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (succ (su` | `` | ok | chain of 100 dependent lets |
| P319 | `dpm-nbe nf` | `let v0 : Nat = zero; let v1 : Nat = suc v0; let v2 : Nat = s…(26780 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | chain of 1000 dependent lets |
| P320 | `dpm-nbe nf` | `let f : (n : Nat) -> Id Nat (plus n zero) n = \\n . refl; f` | 1 | `` | `expecting plus n zero equals to n, but it is not` | clean error (exit 1) | plus n zero = n definitional |
| P321 | `dpm-nbe nf` | `let f : (n : Nat) -> Id Nat (plus zero n) n = \\n . refl; f` | 0 | `λ n . refl\n  :\n(n : Nat) → Id Nat n n\n` | `` | ok | plus zero n = n definitional |
| P322 | `dpm-nbe nf` | `let f : (n : Nat) -> Id Nat (plus (suc n) zero) (suc n) = \\n…(70 B)` | 1 | `` | `expecting succ (plus n zero) equals to succ n, but it is not` | clean error (exit 1) | plus (suc n) zero conversion |
| P323 | `dpm-nbe nf` | `let f : Id Nat (plus (suc zero) (suc zero)) (suc (suc zero))…(70 B)` | 0 | `refl\n  :\nId Nat (succ (succ zero)) (succ (succ zero))\n` | `` | ok | plus computes in conversion |
| P324 | `dpm-nbe nf` | `let f : (A : U) (a b : A) -> Id A a b -> Id A b a = \\A a b r…(73 B)` | 0 | `λ A a b refl . refl\n  :\n(A : U)(a : A)(b : A) → Id A a b → Id A b a\n` | `` | ok | sym-like with refl patterns |
| P325 | `dpm-nbe nf` | `let f : (A : U) (a : A) -> Id A a a = \\A a . refl; f` | 0 | `λ A a . refl\n  :\n(A : U)(a : A) → Id A a a\n` | `` | ok | refl value |
| P326 | `dpm-nbe nf` | `let f : (A : U) (B : A -> U) (x : A) -> Id (B x) (B x) = \\A …(73 B)` | 1 | `` | `expected an atom` | clean error (exit 1) | dependent refl |
| P327 | `dpm-nbe nf` | `let f : (A : U) -> Id U A A = \\A . refl; f` | 0 | `λ A . refl\n  :\n(A : U) → Id U A A\n` | `` | ok | refl in U |
| P328 | `dpm-nbe nf` | `let f : Id U Nat Nat = refl; f` | 0 | `refl\n  :\nId U Nat Nat\n` | `` | ok | refl of Nat at U |
| P329 | `dpm-nbe nf` | `Id U Nat Nat` | 0 | `Id U Nat Nat\n  :\nU\n` | `` | ok | Id U Nat Nat as type |
| P330 | `dpm-nbe nf` | `let f : (A : U) -> (B : U) -> Id U A B -> U = \\A B (!); f` | 1 | `` | `It should not be '(!)' for equation between A and B, in fact you should try pattern 'refl'.` | clean error (exit 1) | absurd on Id U A B |
| P331 | `dpm-nbe nf` | `let bad : (A : U) -> Id Nat (suc zero) zero -> A = \\A refl .…(67 B)` | 1 | `` | `It should not be 'refl' for equation between succ zero and zero, in fact you should try pattern '(!)'.` | clean error (exit 1) | impossible refl pattern (should be type error) |
| P332 | `dpm-nbe nf` | `let f : Nat -> Nat = \\n . plus n n; f (f (f (f (f (f (f (f z…(70 B)` | 0 | `zero\n  :\nNat\n` | `` | ok | doubling 8 -> 256 |
| P333 | `dpm-nbe nf` | `let f : Nat -> Nat = \\n . plus n (suc zero); f (f (f (f (f z…(67 B)` | 0 | `succ (succ (succ (succ (succ zero))))\n  :\nNat\n` | `` | ok | increment chain |
| P334 | `dpm-nbe nf` | `let f : Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat -> Nat…(24905 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | 2000-arrow type in output |

## cli (52 probes)

| id | command | stdin (truncated) | exit | stdout | stderr last line | verdict | note |
|---|---|---|---|---|---|---|---|
| C001 | `dpm-nbe (no args)` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | no args |
| C002 | `dpm-nbe (no args)` | `` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | no args, empty stdin |
| C003 | `dpm-nbe --help` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | --help with stdin |
| C004 | `dpm-nbe --help` | `` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | --help with empty stdin |
| C005 | `dpm-nbe -h` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | -h short help |
| C006 | `dpm-nbe help` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | help word |
| C007 | `dpm-nbe --version` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | --version |
| C008 | `dpm-nbe -V` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | -V |
| C009 | `dpm-nbe nf` | `zero` | 0 | `zero\n  :\nNat\n` | `` | ok | nf normal |
| C010 | `dpm-nbe type` | `zero` | 0 | `Nat\n` | `` | ok | type normal |
| C011 | `dpm-nbe NF` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | uppercase NF |
| C012 | `dpm-nbe Type` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | capitalised Type |
| C013 | `dpm-nbe nf extra` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | nf plus extra arg |
| C014 | `dpm-nbe nf type` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | nf and type |
| C015 | `dpm-nbe type nf` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | type and nf |
| C016 | `dpm-nbe --help nf` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | --help then nf |
| C017 | `dpm-nbe nf --help` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | nf then --help |
| C018 | `dpm-nbe (no args)` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | empty argument |
| C019 | `dpm-nbe  ` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | space argument |
| C020 | `dpm-nbe nf ` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | trailing space in mode |
| C021 | `dpm-nbe  nf` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | leading space in mode |
| C022 | `dpm-nbe λ` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | unicode lambda argument |
| C023 | `dpm-nbe --` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | double dash |
| C024 | `dpm-nbe -` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | single dash (stdin?) |
| C025 | `dpm-nbe -x` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | unknown short flag |
| C026 | `dpm-nbe --nf` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | --nf long flag |
| C027 | `dpm-nbe nf nf nf` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | nf repeated |
| C028 | `dpm-nbe --help --help` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | --help twice |
| C029 | `dpm-nbe --HELP` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | --HELP uppercase |
| C030 | `dpm-nbe --help=1` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | --help=1 |
| C031 | `dpm-nbe type` | `let bad : Nat = U; U` | 1 | `` | `  U` | clean error (exit 1) | type error exit code |
| C032 | `dpm-nbe nf` | `let bad : Nat = U; U` | 1 | `` | `  U` | clean error (exit 1) | type error in nf |
| C033 | `dpm-nbe nf` | `suc` | 1 | `` | `expected an atom` | clean error (exit 1) | parse error exit code |
| C034 | `dpm-nbe type` | `suc` | 1 | `` | `expected an atom` | clean error (exit 1) | parse error in type mode |
| C035 | `dpm-nbe nf` | `zero) ` | 1 | `` | `expected end of input` | clean error (exit 1) | parse error, stdout must stay clean |
| C036 | `dpm-nbe --help` | `xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx…(5000000 B)` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | --help with 5MB stdin (does it read stdin?) |
| C037 | `dpm-nbe type` | `plus (suc zero) (suc zero)` | 0 | `Nat\n` | `` | ok | type of a computation |
| C038 | `dpm-nbe nf` | `plus (suc zero) (suc zero)` | 0 | `succ (succ zero)\n  :\nNat\n` | `` | ok | nf of a computation |
| C039 | `dpm-nbe Nf` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | mixed-case Nf |
| C040 | `dpm-nbe nf
` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | mode with embedded newline |
| C041 | `dpm-nbe nf	` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | mode with embedded tab |
| C042 | `dpm-nbe nf -h` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | nf then -h |
| C043 | `dpm-nbe --help -h` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | --help and -h |
| C044 | `dpm-nbe h` | `zero` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | h alone |
| C045 | `dpm-nbe nf` | `` | 1 | `` | `expected an atom` | clean error (exit 1) | nf with empty stdin -> error exit |
| C046 | `dpm-nbe type` | `` | 1 | `` | `expected an atom` | clean error (exit 1) | type with empty stdin -> error exit |
| C047 | `dpm-nbe nf` | `zero�` | 2 | `` | `error: cannot read stdin: stream did not contain valid UTF-8` | exit 2 (stdin not UTF-8) | nf invalid utf8 exit code |
| C048 | `dpm-nbe type` | `zero�` | 2 | `` | `error: cannot read stdin: stream did not contain valid UTF-8` | exit 2 (stdin not UTF-8) | type invalid utf8 exit code |
| C049 | `dpm-nbe --help` | `zero�` | 0 | `usage: dpm-nbe [--help\|nf\|type]\n  --help : display this message\n  nf     : read & typecheck a program from…` | `` | ok | --help invalid utf8 |
| C050 | `dpm-nbe nf` | `\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0……(1000 B)` | 1 | `` | `expected an atom` | clean error (exit 1) | 1000 NUL bytes |
| C051 | `dpm-nbe nf` | `                                                            …(10000000 B)` | 1 | `` | `expected an atom` | clean error (exit 1) | 10MB of spaces |
| C052 | `dpm-nbe nf` | `let x : Nat = zero; x\nlet x : Nat = zero; x\nlet x : Nat = ze…(4400000 B)` | 1 | `` | `unexpected keyword 'let'` | clean error (exit 1) | 200k lets |

## regress (23 probes)

| id | command | stdin (truncated) | exit | stdout | stderr last line | verdict | note |
|---|---|---|---|---|---|---|---|
| P397 | `dpm-nbe nf` | `let C : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!); C` | 101 | `` | `thread 'dpm-nbe' panicked ... error: internal panic while elaborating the program` | **RUST PANIC (exit 101)** | PANIC: absurd-lambda value in nf (rc 101) |
| P398 | `dpm-nbe nf` | `let C : Nat -> Id Nat (suc zero) zero -> Nat = \\n (!); C` | 101 | `` | `thread 'dpm-nbe' panicked ... error: internal panic while elaborating the program` | **RUST PANIC (exit 101)** | PANIC: minimal absurd lambda, nf |
| P399 | `dpm-nbe type` | `let C : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!); let…(122 B)` | 101 | `` | `thread 'dpm-nbe' panicked ... error: internal panic while elaborating the program` | **RUST PANIC (exit 101)** | PANIC: same in type mode |
| P400 | `dpm-nbe nf` | `let f : (n m : Nat) -> Id Nat (plus n m) (plus n m) = \\n m .…(68 B)` | 1 | `` | `expecting plus n m equals to plus n m, but it is not` | clean error (exit 1) | CONVERSION BUG: identical neutral types judged unequal |
| P401 | `dpm-nbe nf` | `let f : (n m : Nat) -> Id Nat (plus (suc n) m) (suc (plus n …(80 B)` | 1 | `` | `expecting succ (plus n m) equals to succ (plus n m), but it is not` | clean error (exit 1) | CONVERSION BUG: self-contradictory error message |
| P402 | `dpm-nbe nf` | `let k : (n m : Nat) (p : Id Nat (plus n m) (plus n m)) -> Id…(102 B)` | 1 | `` | `  Id Nat (plus n m) (plus n m)` | clean error (exit 1) | CONVERSION BUG: rejects a variable checked against its own type |
| P403 | `dpm-nbe nf` | `suc (suc zero)` | 0 | `succ (succ zero)\n  :\nNat\n` | `` | ok | ROUNDTRIP: printed nf says 'succ', parser wants 'suc' |
| P404 | `dpm-nbe nf` | `((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((…(1028 B)` | 1 | `` | `'zero' is a keyword` | clean error (exit 1) | DEPTH: 512 parens -> wrong message "'zero' is a keyword" |
| P405 | `dpm-nbe nf` | `((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((…(1030 B)` | 1 | `` | `expression nesting is too deep` | clean error (exit 1) | DEPTH: 513 parens -> 'expression nesting is too deep' |
| P406 | `dpm-nbe nf` | `zero$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$…(1000004 B)` | 1 | `` | `expected end of input` | clean error (exit 1) | ECHO: 1MB line echoed into stderr on error |
| P407 | `dpm-nbe nf` | `αααααααααααααααααααααααααααααα…(4002 B)` | 1 | `` | `expected end of input` | clean error (exit 1) | UTF8: long multibyte line, error rendering |
| P408 | `dpm-nbe nf` | `let x : Nat = zero; _` | 1 | `` | `expected an atom` | clean error (exit 1) | '_' is neither binder nor term |
| P409 | `dpm-nbe nf` | `zero {- unterminated` | 0 | `zero\n  :\nNat\n` | `` | ok | unterminated {- accepted silently |
| P410 | `dpm-nbe nf` | `﻿zero` | 1 | `` | `expected an atom` | clean error (exit 1) | UTF-8 BOM rejected |
| P411 | `dpm-nbe nf` | `zero�` | 2 | `` | `error: cannot read stdin: stream did not contain valid UTF-8` | exit 2 (stdin not UTF-8) | invalid UTF-8 -> exit code 2 |
| P412 | `dpm-nbe nf` | `let suc : Nat = zero; suc` | 1 | `` | `'suc' is a keyword` | clean error (exit 1) | keyword rebinding rejected |
| P413 | `dpm-nbe nf` | `let succ : Nat = suc zero; succ` | 0 | `succ zero\n  :\nNat\n` | `` | ok | 'succ' IS a legal identifier (printer name) |
| P414 | `dpm-nbe nf` | `let f : (A : U) -> A -> A = \\A _ . _; f` | 1 | `` | `expected '.'` | clean error (exit 1) | underscore binder error |
| P415 | `dpm-nbe nf` | `let f : Nat -> Nat -> Nat = \\p0 p1 p2 . p0; f` | 1 | `` | `Too many patterns.` | clean error (exit 1) | more patterns than arrows -> 'Too many patterns.' |
| P416 | `dpm-nbe nf` | `let f : (p0 p1 p2 : Nat) -> Nat = \\p0 p1 p2 . p0; f` | 0 | `λ p0 p1 p2 . p0\n  :\n(p0 : Nat)(p1 : Nat)(p2 : Nat) → Nat\n` | `` | ok | grouped binder, 3 patterns -> OK |
| P417 | `dpm-nbe nf` | `let f : (A : U) -> Id A zero zero -> U = \\A refl . A; f` | 1 | `` | `  Nat` | clean error (exit 1) | refl pattern, declared type checked under refinement -> error at 'zero… |
| P418 | `dpm-nbe nf` | `let f : (x : Nat) -> Id Nat (plus x zero) (suc x) -> Nat = \\…(68 B)` | 1 | `` | `I am not sure if there should be a pattern '(!)' for equation between plus x zero and succ x.` | clean error (exit 1) | absurd pattern on open contradiction -> conservative error (good) |
| P419 | `dpm-nbe type` | `let f : (n m : Nat) -> Id Nat (plus (suc n) m) (suc (plus n …(80 B)` | 1 | `` | `expecting succ (plus n m) equals to succ (plus n m), but it is not` | clean error (exit 1) | conversion bug also in type mode |
