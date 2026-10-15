# Consolidate probe results and fuzz findings into an inventory (no new runs).
$ErrorActionPreference = 'Stop'
$root = 'F:\projects\Haskell\dpm-nbe\rust\reviews\robustness'
$utf8 = New-Object System.Text.UTF8Encoding($false)

$rows = @()
foreach ($f in Get-ChildItem $root -Filter 'results_*.json' | Sort-Object Name) {
    $data = Get-Content -Raw -Encoding UTF8 $f.FullName | ConvertFrom-Json
    foreach ($r in $data) {
        $rows += [pscustomobject]@{
            Source     = $f.Name
            Name       = $r.Name
            Category   = $r.Category
            Args       = $r.Args
            ExitCode   = $r.ExitCode
            ExitHex    = $r.ExitHex
            TimedOut   = $r.TimedOut
            ElapsedMs  = $r.ElapsedMs
            StdoutLen  = $r.StdoutLen
            StderrLen  = $r.StderrLen
            Panic      = $r.Panic
            Abnormal   = $r.Abnormal
            StdinBytes = $r.StdinBytes
            Note       = $r.Note
        }
    }
}

Write-Host ("probe rows: {0}" -f $rows.Count)
Write-Host "--- by category ---"
$rows | Group-Object Category | Sort-Object Name | ForEach-Object { Write-Host ("{0,-14} {1}" -f $_.Name, $_.Count) }

$abnormal = $rows | Where-Object { $_.Panic -or $_.Abnormal -or $_.TimedOut -or ($_.ExitCode -ne 0 -and $_.StdoutLen -gt 0) -or ($_.ExitCode -eq 0 -and $_.StderrLen -gt 0) }
Write-Host ("--- noteworthy ({0}) ---" -f $abnormal.Count)
$abnormal | Sort-Object Category, Name | Format-Table -AutoSize Source, Name, ExitCode, ExitHex, TimedOut, ElapsedMs, Panic, Note | Out-String -Width 240 | Write-Host

Write-Host "--- exit code distribution ---"
$rows | Group-Object ExitCode | Sort-Object Name | ForEach-Object { Write-Host ("exit {0,-14} {1}" -f $_.Name, $_.Count) }

# stream discipline check
$viol = $rows | Where-Object { ($_.ExitCode -eq 0 -and $_.StderrLen -gt 0) -or ($_.ExitCode -ne 0 -and $_.StdoutLen -gt 0 -and -not $_.Panic) }
Write-Host ("stream-discipline violations (excluding panics): {0}" -f $viol.Count)

# fuzz findings: unique signatures
$fz = Join-Path $root 'fuzz\findings.jsonl'
if (Test-Path $fz) {
    $text = Get-Content -Raw -Encoding UTF8 $fz
    if ($text.Trim().Length -gt 0) {
        $items = $text | ConvertFrom-Json
        Write-Host ("--- fuzz findings: {0} ---" -f $items.Count)
        $items | Group-Object Stderr | Sort-Object Count -Descending | ForEach-Object {
            Write-Host ("count={0}" -f $_.Count)
            Write-Host ("  stderr: {0}" -f (($_.Name -replace "`r?`n", ' | ').Substring(0, [Math]::Min(160, ($_.Name -replace "`r?`n", ' | ').Length))))
            Write-Host ("  sample stdin: {0}" -f ($_.Group[0].Stdin -replace "`r?`n", ' '))
        }
        $uniqueInputs = $items | Group-Object Stdin | Sort-Object Count -Descending
        Write-Host ("unique failing inputs: {0}" -f $uniqueInputs.Count)
        $uniqueInputs | ForEach-Object { Write-Host ("  x{0}: {1}" -f $_.Count, ($_.Name -replace "`r?`n", ' ')) }
    }
}
