# Reusable timeout-protected black-box probe runner for dpm-nbe.
# Every probe runs the binary with a hard wall-clock timeout so a hang cannot
# stall the review.  stdin is fed as raw bytes; stdout/stderr are drained
# asynchronously so a large normal form cannot deadlock the pipe, and are saved
# verbatim as artifacts next to a machine-readable summary.
#
# Usage:  . "$PSScriptRoot\common.ps1"
#         Invoke-Probe -Name p01 -Category cli -Args @('nf') -Text "U"
#         Save-Results -Path "...\results.json"

$ErrorActionPreference = 'Stop'

$script:Exe  = 'F:\projects\Haskell\dpm-nbe\rust\target\release\dpm-nbe.exe'
$script:Root = 'F:\projects\Haskell\dpm-nbe\rust\reviews\robustness'
$script:Art  = Join-Path $script:Root 'artifacts'
$script:Results = New-Object System.Collections.ArrayList

$script:Utf8NoBom = New-Object System.Text.UTF8Encoding($false)

function Write-Bytes {
    param([string]$Path, [byte[]]$Bytes)
    [System.IO.File]::WriteAllBytes($Path, $Bytes)
}

function Write-TextFile {
    param([string]$Path, [string]$Text)
    [System.IO.File]::WriteAllText($Path, $Text, $script:Utf8NoBom)
}

function Read-TextOrNull {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path)) { return $null }
    $bytes = [System.IO.File]::ReadAllBytes($Path)
    if ($bytes.Length -eq 0) { return '' }
    $enc = New-Object System.Text.UTF8Encoding($false, $false)
    return $enc.GetString($bytes)
}

function Format-Argv {
    param([string[]]$A)
    if ($null -eq $A -or $A.Count -eq 0) { return '<no args>' }
    return ($A | ForEach-Object { if ($_ -match '[\s"]') { '"' + $_ + '"' } else { $_ } }) -join ' '
}

function ConvertTo-CommandLine {
    param([string[]]$A)
    $parts = @()
    foreach ($a in $A) {
        if ($a -eq '') { $parts += '""'; continue }
        if ($a -match '[\s"]') {
            $q = '"' + ($a -replace '(\\*)"', '$1$1\"') + '"'
            $parts += $q
        }
        else {
            $parts += $a
        }
    }
    return ($parts -join ' ')
}

# Invoke-Probe: run the binary once.
#   -Name        stable probe id
#   -Category    one of the 6 review categories
#   -Args        argv (excluding argv[0])
#   -Text        stdin as UTF-8 text
#   -Bytes       stdin as raw bytes (takes precedence over -Text)
#   -TimeoutSec  hard timeout (default 15)
#   -Note        free-form note stored with the result
function Invoke-Probe {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [string]$Category = 'misc',
        [string[]]$Args = @(),
        $Text = $null,
        [byte[]]$Bytes = $null,
        [int]$TimeoutSec = 15,
        [string]$Note = ''
    )

    $inFile  = Join-Path $script:Art "$Name.stdin"
    $outFile = Join-Path $script:Art "$Name.stdout"
    $errFile = Join-Path $script:Art "$Name.stderr"

    if ($null -ne $Bytes) {
        $inBytes = [byte[]]$Bytes
    }
    elseif ($null -ne $Text) {
        $inBytes = $script:Utf8NoBom.GetBytes([string]$Text)
    }
    else {
        $inBytes = [byte[]]@()
    }
    Write-Bytes -Path $inFile -Bytes $inBytes

    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $script:Exe
    $psi.Arguments = ConvertTo-CommandLine $Args
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.StandardOutputEncoding = $script:Utf8NoBom
    $psi.StandardErrorEncoding = $script:Utf8NoBom

    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $timedOut = $false
    $exitCode = $null
    $startErr = $null
    $stdout = ''
    $stderr = ''
    $peakWs = -1
    $peakPaged = -1
    $p = $null

    try {
        $p = New-Object System.Diagnostics.Process
        $p.StartInfo = $psi
        [void]$p.Start()

        $outTask = $p.StandardOutput.ReadToEndAsync()
        $errTask = $p.StandardError.ReadToEndAsync()

        try {
            $si = $p.StandardInput.BaseStream
            if ($inBytes.Length -gt 0) { $si.Write($inBytes, 0, $inBytes.Length) }
            $si.Flush()
            $si.Close()
        }
        catch {
            # child may exit without draining stdin (e.g. unknown args); harmless
        }

        $exited = $p.WaitForExit(0)
        # Poll while running so we can record the high-water mark of memory use
        # and enforce the wall-clock timeout.
        $maxWs = [int64]0
        $maxPriv = [int64]0
        $deadlineMs = [int64]$TimeoutSec * 1000
        while (-not $p.HasExited) {
            try {
                $ws = [int64]$p.WorkingSet64
                if ($ws -gt $maxWs) { $maxWs = $ws }
            }
            catch { }
            try {
                $pv = [int64]$p.PrivateMemorySize64
                if ($pv -gt $maxPriv) { $maxPriv = $pv }
            }
            catch { }
            if ($sw.ElapsedMilliseconds -ge $deadlineMs) {
                $timedOut = $true
                try { $p.Kill() } catch { }
                try { [void]$p.WaitForExit(10000) } catch { }
                break
            }
            try { [void]$p.WaitForExit(20) } catch { }
        }
        try { $exitCode = $p.ExitCode } catch { $exitCode = $null }
        $peakWs = $maxWs
        $peakPaged = $maxPriv

        try { $stdout = $outTask.GetAwaiter().GetResult() } catch { $stdout = "<stdout read failed: $($_.Exception.Message)>" }
        try { $stderr = $errTask.GetAwaiter().GetResult() } catch { $stderr = "<stderr read failed: $($_.Exception.Message)>" }
    }
    catch {
        $startErr = $_.Exception.Message
    }
    finally {
        if ($null -ne $p) { try { $p.Dispose() } catch { } }
    }
    $sw.Stop()

    # keep raw bytes of what the child wrote, for encoding/BOM inspection
    Write-TextFile -Path $outFile -Text $stdout
    Write-TextFile -Path $errFile -Text $stderr

    $panic = ($stderr -match 'panicked at') -or ($stderr -match 'RUST_BACKTRACE')
    $overflow = ($exitCode -eq -1073741819) -or ($exitCode -eq 3221225477) -or
        ($exitCode -eq -1073741815) -or ($exitCode -eq -1073741571) -or
        ($stderr -match 'stack overflow')
    $abnormal = ($null -eq $exitCode) -or ($exitCode -ne 0 -and $exitCode -ne 1 -and $exitCode -ne 2)
    $exitHex = ''
    if ($null -ne $exitCode -and $exitCode -lt 0) {
        $unsigned = [int64]$exitCode + [int64]4294967296
        $exitHex = '0x' + ('{0:X8}' -f $unsigned)
    }

    $rec = [pscustomobject]@{
        Name          = $Name
        Category      = $Category
        Args          = Format-Argv $Args
        StdinBytes    = $inBytes.Length
        ExitCode      = $exitCode
        ExitHex       = $exitHex
        TimedOut      = $timedOut
        ElapsedMs     = [int]$sw.ElapsedMilliseconds
        StdoutLen     = $stdout.Length
        StderrLen     = $stderr.Length
        PeakWsMB      = [math]::Round($peakWs / 1MB, 1)
        PeakPagedMB   = [math]::Round($peakPaged / 1MB, 1)
        Panic         = $panic
        StackOverflow = $overflow
        Abnormal      = $abnormal
        StartError    = $startErr
        Stdout        = $stdout
        Stderr        = $stderr
        Note          = $Note
        StdoutFile    = $outFile
        StderrFile    = $errFile
        StdinFile     = $inFile
    }
    [void]$script:Results.Add($rec)
    Write-Host ("[{0}] exit={1}{2} {3}ms out={4} err={5} peakWS={6}MB" -f `
            $Name, $exitCode, $(if ($timedOut) { ' TIMEOUT' } else { '' }), `
            $sw.ElapsedMilliseconds, $stdout.Length, $stderr.Length, [math]::Round($peakWs / 1MB, 1))
    return $rec
}

function Truncate {
    param([string]$S, [int]$N)
    if ($null -eq $S) { return '' }
    if ($S.Length -le $N) { return $S }
    return $S.Substring(0, $N) + "...<truncated $($S.Length - $N) chars>"
}

function Save-Results {
    param([Parameter(Mandatory = $true)][string]$Path)
    # The machine-readable summary truncates the bulky text fields; the full
    # text lives in the per-probe .stdout/.stderr artifacts.
    $slim = @()
    foreach ($r in $script:Results) {
        $slim += [pscustomobject]@{
            Name          = $r.Name
            Category      = $r.Category
            Args          = $r.Args
            StdinBytes    = $r.StdinBytes
            ExitCode      = $r.ExitCode
            ExitHex       = $r.ExitHex
            TimedOut      = $r.TimedOut
            ElapsedMs     = $r.ElapsedMs
            StdoutLen     = $r.StdoutLen
            StderrLen     = $r.StderrLen
            PeakWsMB      = $r.PeakWsMB
            PeakPagedMB   = $r.PeakPagedMB
            Panic         = $r.Panic
            StackOverflow = $r.StackOverflow
            Abnormal      = $r.Abnormal
            StartError    = $r.StartError
            Stdout        = Truncate $r.Stdout 600
            Stderr        = Truncate $r.Stderr 2500
            Note          = $r.Note
        }
    }
    $json = $slim | ConvertTo-Json -Depth 4
    [System.IO.File]::WriteAllText($Path, $json, $script:Utf8NoBom)
    Write-Host "wrote $Path ($($script:Results.Count) probes)"
}

function Show-Result {
    param($R)
    Write-Host "--- $($R.Name)  argv: $($R.Args)  stdin=$($R.StdinBytes)B"
    Write-Host "    exit=$($R.ExitCode) hex=$($R.ExitHex) timeout=$($R.TimedOut) elapsed=$($R.ElapsedMs)ms panic=$($R.Panic)"
    if ($R.StdoutLen -gt 0) { Write-Host "    stdout: $(Truncate $R.Stdout 400)" }
    if ($R.StderrLen -gt 0) { Write-Host "    stderr: $(Truncate $R.Stderr 600)" }
    if ($R.StartError) { Write-Host "    START ERROR: $($R.StartError)" }
    return $R
}
