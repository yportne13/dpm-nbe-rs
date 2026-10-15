# Isolated timing probe: no per-20ms polling, so raw wall time is measured.
# Usage: powershell -File timing_probe.ps1
$ErrorActionPreference = 'Stop'
$exe = 'F:\projects\Haskell\dpm-nbe\rust\target\release\dpm-nbe.exe'

function Measure-Run {
    param([string]$Name, [string]$Text, [int]$TimeoutSec = 180)
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $exe
    $psi.Arguments = 'nf'
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.StandardOutputEncoding = New-Object System.Text.UTF8Encoding($false)
    $psi.StandardErrorEncoding = New-Object System.Text.UTF8Encoding($false)

    $p = New-Object System.Diagnostics.Process
    $p.StartInfo = $psi
    [void]$p.Start()
    $outTask = $p.StandardOutput.ReadToEndAsync()
    $errTask = $p.StandardError.ReadToEndAsync()
    $bytes = (New-Object System.Text.UTF8Encoding($false)).GetBytes($Text)
    try {
        $p.StandardInput.BaseStream.Write($bytes, 0, $bytes.Length)
        $p.StandardInput.BaseStream.Flush()
        $p.StandardInput.Close()
    }
    catch { }

    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $exited = $p.WaitForExit($TimeoutSec * 1000)
    if (-not $exited) { try { $p.Kill() } catch { }; [void]$p.WaitForExit(5000) }
    $sw.Stop()
    $code = $null
    try { $code = $p.ExitCode } catch { }
    $out = ''; $err = ''
    try { $out = $outTask.GetAwaiter().GetResult() } catch { }
    try { $err = $errTask.GetAwaiter().GetResult() } catch { }
    $hex = ''
    if ($null -ne $code -and $code -lt 0) { $hex = '0x' + ('{0:X8}' -f ([int64]$code + [int64]4294967296)) }
    $p.Dispose()
    $errHead = ''
    if ($err.Length -gt 0) { $errHead = ($err -split "`n")[0..([Math]::Min(1, ($err -split "`n").Count - 1))] -join ' / ' }
    Write-Host ("{0,-18} stdin={1,9}B exit={2,6} {3,-12} {4,8}ms outlen={5} errlen={6} err[{7}]" -f `
            $Name, $bytes.Length, $code, $hex, $sw.ElapsedMilliseconds, $out.Length, $err.Length, (Truncate-Str $errHead 60))
}

function Truncate-Str { param([string]$s, [int]$n) if ($s.Length -le $n) { return $s } return $s.Substring(0, $n) + '...' }

foreach ($n in 10000, 20000, 30000, 40000, 50000, 80000) {
    Measure-Run -Name "suc_$n" -Text (('suc ' * $n) + 'zero')
}
foreach ($n in 10000, 20000, 50000, 100000) {
    Measure-Run -Name "spineU_$n" -Text ('U ' * $n)
}
foreach ($n in 5, 8, 10, 12, 14, 16, 18, 20) {
    $binders = (1..$n | ForEach-Object { "(a$_ : A) " }) -join ''
    $args = (1..($n - 1) | ForEach-Object { "-> Id A a$_ a$($_ + 1) " }) -join ''
    $vars = (1..$n | ForEach-Object { "a$_ " }) -join ''
    $refls = (1..($n - 1) | ForEach-Object { 'refl ' }) -join ''
    $src = "let t : (A : U) $binders$args-> Id A a1 a$n = \ A $vars$refls. refl; U"
    Measure-Run -Name "trans_$n" -Text $src -TimeoutSec 300
}
