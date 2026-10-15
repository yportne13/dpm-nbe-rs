# Category 5 (CLI contract / exit codes / stream discipline) + category 4
# (malformed & edge input) + category 6 (error position correctness).
. "$PSScriptRoot\common.ps1"

# ---------------------------------------------------------------- category 5
Invoke-Probe -Name c5_noargs      -Category cli -Note "no args -> usage on stdout, exit 0"
Invoke-Probe -Name c5_help        -Category cli -Args @('--help') -Note "--help -> usage on stdout, exit 0"
Invoke-Probe -Name c5_h           -Category cli -Args @('-h') -Note "-h is not a documented flag"
Invoke-Probe -Name c5_unknown     -Category cli -Args @('bogus') -Note "unknown arg -> usage, exit 0"
Invoke-Probe -Name c5_dashdash    -Category cli -Args @('--') -Note "bare --"
Invoke-Probe -Name c5_two_flags   -Category cli -Args @('nf','type') -Note "two valid flags"
Invoke-Probe -Name c5_flag_extra  -Category cli -Args @('nf','--help') -Note "valid flag + extra"
Invoke-Probe -Name c5_empty_arg   -Category cli -Args @('') -Note "empty-string argument"
Invoke-Probe -Name c5_nf_upper    -Category cli -Args @('NF') -Text "U" -Note "case sensitivity"
Invoke-Probe -Name c5_help_stdin  -Category cli -Args @('--help') -Text "this is not a program"
Invoke-Probe -Name c5_nf_ok       -Category cli -Args @('nf') -Text "plus (suc zero) (suc zero)"
Invoke-Probe -Name c5_type_ok     -Category cli -Args @('type') -Text "(A : U) -> A -> A"
Invoke-Probe -Name c5_nf_typeerr  -Category cli -Args @('nf') -Text "let bad : Nat = U; U"
Invoke-Probe -Name c5_type_typeerr -Category cli -Args @('type') -Text "let bad : Nat = U; U"
Invoke-Probe -Name c5_nf_parseerr -Category cli -Args @('nf') -Text "plus zero"
Invoke-Probe -Name c5_type_parseerr -Category cli -Args @('type') -Text "plus zero"
Invoke-Probe -Name c5_nf_unbound  -Category cli -Args @('nf') -Text "x"
Invoke-Probe -Name c5_nf_refl_infer -Category cli -Args @('nf') -Text "refl"
Invoke-Probe -Name c5_nf_lam_infer -Category cli -Args @('nf') -Text "\ x . x"

# trailing data after a complete program
Invoke-Probe -Name c5_trailing_word -Category cli -Args @('nf') -Text "U junk"
Invoke-Probe -Name c5_trailing_paren -Category cli -Args @('nf') -Text "U )"
Invoke-Probe -Name c5_trailing_arrow -Category cli -Args @('nf') -Text "U ->"
Invoke-Probe -Name c5_trailing_semi -Category cli -Args @('nf') -Text "U ;"

# ---------------------------------------------------------------- category 4
Invoke-Probe -Name c4_empty       -Category malformed -Args @('nf') -Text ""            -Note "empty stdin"
Invoke-Probe -Name c4_empty_bytes -Category malformed -Args @('nf') -Bytes ([byte[]]@()) -Note "zero bytes"
Invoke-Probe -Name c4_ws_space    -Category malformed -Args @('nf') -Text "     "       -Note "spaces only"
Invoke-Probe -Name c4_ws_mixed    -Category malformed -Args @('nf') -Text " `t`r`n  `n " -Note "mixed whitespace only"
Invoke-Probe -Name c4_comment_line -Category malformed -Args @('nf') -Text "-- just a comment`n" -Note "line comment only"
Invoke-Probe -Name c4_comment_block -Category malformed -Args @('nf') -Text "{- block comment -}" -Note "block comment only"
Invoke-Probe -Name c4_comment_open -Category malformed -Args @('nf') -Text "{- unterminated" -Note "unterminated block comment"
Invoke-Probe -Name c4_comment_open_nested -Category malformed -Args @('nf') -Text "{- {- nested -}" -Note "nested/unterminated"
Invoke-Probe -Name c4_comment_dash -Category malformed -Args @('nf') -Text "--" -Note "empty line comment"
Invoke-Probe -Name c4_comment_after -Category malformed -Args @('nf') -Text "U {- ok -}" -Note "trailing block comment"
Invoke-Probe -Name c4_comment_after_unterm -Category malformed -Args @('nf') -Text "U {- never closed" -Note "trailing unterminated comment"

# non-UTF8 bytes
Invoke-Probe -Name c4_nonutf8_ff   -Category malformed -Args @('nf') -Bytes ([byte[]](0x55,0xFF,0xFE,0x0A)) -Note "U + invalid bytes"
Invoke-Probe -Name c4_nonutf8_lone -Category malformed -Args @('nf') -Bytes ([byte[]](0x4E,0x61,0x74,0xC3,0x28)) -Note "truncated UTF-8 sequence"
Invoke-Probe -Name c4_nonutf8_nul  -Category malformed -Args @('nf') -Bytes ([byte[]](0x55,0x00,0x0A)) -Note "NUL byte"
Invoke-Probe -Name c4_nonutf8_utf16 -Category malformed -Args @('nf') -Bytes ([System.Text.Encoding]::Unicode.GetBytes("U")) -Note "UTF-16LE encoded U"
Invoke-Probe -Name c4_nonutf8_hi   -Category malformed -Args @('nf') -Bytes ([byte[]](0x80,0x81,0x82)) -Note "high bytes only"

# line endings / whitespace / BOM
Invoke-Probe -Name c4_crlf        -Category malformed -Args @('nf') -Text "U`r`n"        -Note "CRLF terminator"
Invoke-Probe -Name c4_cr_only     -Category malformed -Args @('nf') -Text "U`r"          -Note "bare CR"
Invoke-Probe -Name c4_tabs        -Category malformed -Args @('nf') -Text "`tU`t"        -Note "tabs around U"
Invoke-Probe -Name c4_bom         -Category malformed -Args @('nf') -Bytes ([byte[]](0xEF,0xBB,0xBF,0x55,0x0A)) -Note "UTF-8 BOM + U"
Invoke-Probe -Name c4_bom_crlf    -Category malformed -Args @('nf') -Bytes ([byte[]](0xEF,0xBB,0xBF,0x4E,0x61,0x74,0x0D,0x0A)) -Note "BOM + Nat CRLF"
Invoke-Probe -Name c4_vertical_tab -Category malformed -Args @('nf') -Text "U`v"        -Note "vertical tab after U"
Invoke-Probe -Name c4_formfeed    -Category malformed -Args @('nf') -Text "`fU"         -Note "form feed before U"
Invoke-Probe -Name c4_nbsp        -Category malformed -Args @('nf') -Text ([char]0x00A0 + "U") -Note "non-breaking space (not is_whitespace in Rust)"
Invoke-Probe -Name c4_unicode_id   -Category malformed -Args @('nf') -Text ([char]0x03BB + " x . x") -Note "lambda via unicode escape"
Invoke-Probe -Name c4_unicode_arrow -Category malformed -Args @('nf') -Text ("Nat " + [char]0x2192 + " Nat") -Note "unicode arrow Pi"
Invoke-Probe -Name c4_unicode_deep -Category malformed -Args @('nf') -Text ([char]::ConvertFromUtf32(0x1F600)) -Note "emoji (non-alphanumeric, non-ascii)"

# long single line (no newline) and long identifiers
Invoke-Probe -Name c4_long_line_nf  -Category malformed -Args @('nf') -Text ("U" + " " * 200000) -Note "200k spaces after U, no newline"
Invoke-Probe -Name c4_long_ident    -Category malformed -Args @('nf') -Text ("a" * 100000) -Note "100k-char identifier"
Invoke-Probe -Name c4_long_ident_type -Category malformed -Args @('type') -Text ("a" * 100000)
Invoke-Probe -Name c4_long_comment  -Category malformed -Args @('nf') -Text ("{-" + "-" * 200000) -Note "200k-char unterminated comment"
Invoke-Probe -Name c4_long_line_comment -Category malformed -Args @('nf') -Text ("--" + "x" * 200000) -Note "200k-char line comment, no newline"
Invoke-Probe -Name c4_long_numberish -Category malformed -Args @('nf') -Text ("9" * 50000) -Note "50k digits as identifier"

# ---------------------------------------------------------------- category 6
$pos_prog_1 = "let ok : Nat = zero;`nlet bad : Nat = U;`nU"
Invoke-Probe -Name c6_line2_col18 -Category position -Args @('nf') -Text $pos_prog_1 -Note "error at line 2 col 18 (the U)"
Invoke-Probe -Name c6_line2_col18_type -Category position -Args @('type') -Text $pos_prog_1
Invoke-Probe -Name c6_tab_col -Category position -Args @('nf') -Text "let ok : Nat = zero;`n`tlet bad : Nat = U;`nU" -Note "tab on line 2 shifts visual column"
Invoke-Probe -Name c6_crlf_col -Category position -Args @('nf') -Text "let ok : Nat = zero;`r`nlet bad : Nat = U;`r`nU" -Note "CRLF line endings: does col count \\r?"
Invoke-Probe -Name c6_deep_line -Category position -Args @('nf') -Text ("`n" * 9 + "  oops") -Note "unbound var on line 10 col 3"
Invoke-Probe -Name c6_err_on_line3 -Category position -Args @('nf') -Text "U`nU`nplus zero" -Note "trailing incomplete plus on line 3"
Invoke-Probe -Name c6_trailing_err -Category position -Args @('nf') -Text "U`nU`njunk)" -Note "trailing junk on line 3"
Invoke-Probe -Name c6_typeerr_line3 -Category position -Args @('nf') -Text "let a : Nat = zero;`nlet b : Nat = zero;`nlet c : Nat = U;`nU" -Note "type error on line 3 col 17"
Invoke-Probe -Name c6_unbound_line4 -Category position -Args @('nf') -Text "U`nU`nU`nnope" -Note "unbound on line 4 col 1"
Invoke-Probe -Name c6_pi_err -Category position -Args @('nf') -Text "let a : Nat = zero;`n(a b : Nat) -> b" -Note "unbound b on line 2 col 20"

Save-Results -Path (Join-Path $script:Root 'results_cli_malformed_position.json')
