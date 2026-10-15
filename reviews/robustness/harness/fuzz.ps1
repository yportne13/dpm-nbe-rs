# Deterministic fuzzer for dpm-nbe.
#   - grammar mode : structurally valid surface programs (reaches the elaborator)
#   - template mode: pattern-matching/refl/absurd shapes (reaches unify/subst_ctx)
#   - raw mode     : random bytes (reaches the parser/UTF-8 reader)
# Any case whose exit code is not 0/1/2, or that panics/aborts/times out, is
# saved to fuzz\findings\ together with its bytes and a JSONL record.
# ASCII only: Windows PowerShell 5.1 reads .ps1 files as ANSI.
param(
    [int]$Seed = 20261014,
    [int]$GrammarCases = 3000,
    [int]$TemplateCases = 1000,
    [int]$RawCases = 2000,
    [int]$TimeoutSec = 5
)

$ErrorActionPreference = 'Stop'
$exe = 'F:\projects\Haskell\dpm-nbe\rust\target\release\dpm-nbe.exe'
$root = 'F:\projects\Haskell\dpm-nbe\rust\reviews\robustness'
$findDir = Join-Path $root 'fuzz\findings'
New-Item -ItemType Directory -Force -Path $findDir | Out-Null
$logPath = Join-Path $root 'fuzz\findings.jsonl'
$corpusPath = Join-Path $root 'fuzz\corpus_sample.txt'
$utf8 = New-Object System.Text.UTF8Encoding($false)
$rng = New-Object System.Random($Seed)
$findings = New-Object System.Collections.ArrayList
$sample = New-Object System.Collections.ArrayList

function Invoke-Case {
    param([byte[]]$Bytes, [string]$Mode, [int]$Index)
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $exe
    $psi.Arguments = $Mode
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.StandardOutputEncoding = $utf8
    $psi.StandardErrorEncoding = $utf8
    $p = New-Object System.Diagnostics.Process
    $p.StartInfo = $psi
    [void]$p.Start()
    $ot = $p.StandardOutput.ReadToEndAsync()
    $et = $p.StandardError.ReadToEndAsync()
    try {
        if ($Bytes.Length -gt 0) { $p.StandardInput.BaseStream.Write($Bytes, 0, $Bytes.Length) }
        $p.StandardInput.BaseStream.Flush()
        $p.StandardInput.Close()
    }
    catch { }
    $timedOut = $false
    try {
        if (-not $p.WaitForExit($TimeoutSec * 1000)) {
            $timedOut = $true
            try { $p.Kill() } catch { }
            [void]$p.WaitForExit(5000)
        }
    }
    catch { $timedOut = $true }
    $code = $null
    try { $code = $p.ExitCode } catch { }
    $out = ''; $err = ''
    try { $out = $ot.GetAwaiter().GetResult() } catch { }
    try { $err = $et.GetAwaiter().GetResult() } catch { }
    $p.Dispose()
    return [pscustomobject]@{
        Index    = $Index
        Mode     = $Mode
        ExitCode = $code
        TimedOut = $timedOut
        Stdout   = $out
        Stderr   = $err
        Bytes    = $Bytes
    }
}

function Test-Interesting {
    param($R)
    if ($R.TimedOut) { return $true }
    if ($null -eq $R.ExitCode) { return $true }
    if ($R.ExitCode -ne 0 -and $R.ExitCode -ne 1 -and $R.ExitCode -ne 2) { return $true }
    if ($R.Stderr -match 'panicked at' -or $R.Stderr -match 'overflowed its stack') { return $true }
    if ($R.ExitCode -eq 0 -and $R.Stderr.Length -gt 0) { return $true }
    if ($R.ExitCode -ne 0 -and $R.Stdout.Length -gt 0) { return $true }
    return $false
}

function Save-Finding {
    param($R, [string]$Kind)
    $hex = ''
    if ($null -ne $R.ExitCode -and $R.ExitCode -lt 0) { $hex = '0x' + ('{0:X8}' -f ([int64]$R.ExitCode + [int64]4294967296)) }
    $name = "f_{0}_{1}" -f $Kind, $R.Index
    $binPath = Join-Path $findDir "$name.bin"
    [System.IO.File]::WriteAllBytes($binPath, $R.Bytes)
    $txt = ''
    if ($R.Bytes.Length -gt 0) { $txt = $utf8.GetString($R.Bytes) }
    [System.IO.File]::WriteAllText((Join-Path $findDir "$name.txt"), $txt, $utf8)
    $rec = [pscustomobject]@{
        Name     = $name
        Kind     = $Kind
        Index    = $R.Index
        Mode     = $R.Mode
        ExitCode = $R.ExitCode
        ExitHex  = $hex
        TimedOut = $R.TimedOut
        Stdin    = Truncate-Text $txt 500
        Stdout   = Truncate-Text $R.Stdout 300
        Stderr   = Truncate-Text $R.Stderr 600
        Bin      = $binPath
    }
    [void]$findings.Add($rec)
    Write-Host ("  !! FINDING {0}: exit={1} {2} timeout={3} stdin={4}" -f $name, $R.ExitCode, $hex, $R.TimedOut, (Truncate-Text $txt 90))
}

function Truncate-Text {
    param([string]$s, [int]$n)
    if ($null -eq $s) { return '' }
    if ($s.Length -le $n) { return $s }
    return $s.Substring(0, $n) + '...'
}

# ---------------------------------------------------------------- generators
$atoms = @('U', 'Nat', 'zero', 'refl', 'x', 'y', 'z', 'A', 'B', 'a', 'b', 'f', 'g', 'suc', 'plus', 'Id')
$ids = @('x', 'y', 'z', 'A', 'B', 'a', 'b', 'f', 'g', 'h')

function New-GenTerm {
    param([int]$Depth)
    if ($Depth -le 0) { return $atoms[$rng.Next($atoms.Length)] }
    switch ($rng.Next(12)) {
        0 { return 'U' }
        1 { return 'Nat' }
        2 { return 'zero' }
        3 { return 'refl' }
        4 { return $ids[$rng.Next($ids.Length)] }
        5 { return 'suc (' + (New-GenTerm ($Depth - 1)) + ')' }
        6 { return 'plus (' + (New-GenTerm ($Depth - 1)) + ') (' + (New-GenTerm ($Depth - 1)) + ')' }
        7 { return 'Id (' + (New-GenTerm ($Depth - 1)) + ') (' + (New-GenTerm ($Depth - 1)) + ') (' + (New-GenTerm ($Depth - 1)) + ')' }
        8 { return '\ ' + $ids[$rng.Next($ids.Length)] + ' . ' + (New-GenTerm ($Depth - 1)) }
        9 { return '\ A (!)' }
        10 { return 'let ' + $ids[$rng.Next($ids.Length)] + ' : ' + (New-GenTerm ($Depth - 1)) + ' = ' + (New-GenTerm ($Depth - 1)) + '; ' + (New-GenTerm ($Depth - 1)) }
        11 { return '(' + $ids[$rng.Next($ids.Length)] + ' : ' + (New-GenTerm ($Depth - 1)) + ') -> ' + (New-GenTerm ($Depth - 1)) }
    }
    return 'U'
}

$pats = @('refl', '(!)', 'A', 'a', 'b', 'n', 'm')

function New-Template {
    switch ($rng.Next(8)) {
        0 {
            $p = $pats[$rng.Next($pats.Length)]
            return "let f : (A : U) (n : Nat) -> Id Nat (suc n) n -> A = \ A n $p; U"
        }
        1 {
            $p = $pats[$rng.Next($pats.Length)]
            return "let f : (A : U) (a b : A) -> Id A a b -> Id A b a = \ A a b $p . refl; U"
        }
        2 {
            $p1 = $pats[$rng.Next($pats.Length)]; $p2 = $pats[$rng.Next($pats.Length)]
            return "let f : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Nat = \ m n $p1 $p2 . zero; U"
        }
        3 {
            return "let C : (A : U) -> Id Nat (suc zero) zero -> A = \ A (!); C"
        }
        4 {
            $t1 = New-GenTerm 2; $t2 = New-GenTerm 2
            return "let T : U = $t1; let C : T = $t2; C"
        }
        5 {
            $n = $rng.Next(1, 7)
            $binders = (1..$n | ForEach-Object { "(a$_ : A) " }) -join ''
            $args = (1..($n - 1) | ForEach-Object { "-> Id A a$_ a$($_ + 1) " }) -join ''
            $vars = (1..$n | ForEach-Object { "a$_ " }) -join ''
            $refls = (1..($n - 1) | ForEach-Object { 'refl ' }) -join ''
            return "let t : (A : U) $binders$args-> Id A a1 a$n = \ A $vars$refls. refl; U"
        }
        6 {
            $inner = New-GenTerm 3
            return "let g : (A : U) (a : A) -> Id A a a -> Nat = \ A a refl . zero; $inner"
        }
        7 {
            $p1 = $pats[$rng.Next($pats.Length)]
            return "let f : (A : U) (a : A) -> Id A a a -> Id A a a -> A = \ A a $p1 $p1; U"
        }
    }
    return 'U'
}

$rawPool = @()
foreach ($c in 9, 10, 13, 32, 33, 34, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 59, 61, 65, 66, 73, 78, 85, 92, 97, 98, 101, 102, 103, 104, 105, 108, 110, 112, 114, 115, 116, 117, 120, 121, 122, 123, 125, 126, 127) { $rawPool += [byte]$c }
$rawPool += [byte]0x00; $rawPool += [byte]0x7F; $rawPool += [byte]0x80; $rawPool += [byte]0xC3; $rawPool += [byte]0xE2; $rawPool += [byte]0xFF

# ---------------------------------------------------------------- main loop
$total = $GrammarCases + $TemplateCases + $RawCases
$done = 0
$interesting = 0
$sw = [System.Diagnostics.Stopwatch]::StartNew()

for ($i = 1; $i -le $GrammarCases; $i++) {
    $src = New-GenTerm (2 + $rng.Next(4))
    $bytes = $utf8.GetBytes($src)
    $r = Invoke-Case -Bytes $bytes -Mode 'nf' -Index $i
    if (Test-Interesting $r) { $interesting++; Save-Finding $r 'grammar' }
    elseif ($sample.Count -lt 200) { [void]$sample.Add($src) }
    $done++
    if ($done % 500 -eq 0) { Write-Host ("grammar {0}/{1}  {2}s  findings={3}" -f $done, $total, [int]$sw.Elapsed.TotalSeconds, $interesting) }
}

for ($i = 1; $i -le $TemplateCases; $i++) {
    $src = New-Template
    $bytes = $utf8.GetBytes($src)
    $r = Invoke-Case -Bytes $bytes -Mode 'nf' -Index $i
    if (Test-Interesting $r) { $interesting++; Save-Finding $r 'template' }
    $done++
    if ($done % 500 -eq 0) { Write-Host ("template {0}/{1}  {2}s  findings={3}" -f $done, $total, [int]$sw.Elapsed.TotalSeconds, $interesting) }
}

for ($i = 1; $i -le $RawCases; $i++) {
    $len = $rng.Next(0, 300)
    $bytes = New-Object byte[] $len
    for ($j = 0; $j -lt $len; $j++) { $bytes[$j] = $rawPool[$rng.Next($rawPool.Length)] }
    $r = Invoke-Case -Bytes $bytes -Mode 'nf' -Index $i
    if (Test-Interesting $r) { $interesting++; Save-Finding $r 'raw' }
    $done++
    if ($done % 500 -eq 0) { Write-Host ("raw {0}/{1}  {2}s  findings={3}" -f $done, $total, [int]$sw.Elapsed.TotalSeconds, $interesting) }
}

$sw.Stop()
Write-Host ("DONE {0} cases in {1}s, {2} interesting" -f $total, [int]$sw.Elapsed.TotalSeconds, $interesting)
$findings | ConvertTo-Json -Depth 4 | ForEach-Object { [System.IO.File]::WriteAllText($logPath, $_, $utf8) }
[System.IO.File]::WriteAllLines($corpusPath, $sample.ToArray(), $utf8)
Write-Host "findings -> $logPath"
