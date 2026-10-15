# Shared harness for black-box testing of dpm-nbe.
# Dot-source this file, then call Run-Case / Run-Table.
$ErrorActionPreference = 'Stop'
try { [Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false) } catch {}
$script:BIN = 'F:\projects\Haskell\dpm-nbe\rust\target\release\dpm-nbe.exe'
$script:WORK = Join-Path $PSScriptRoot '_tmp'
if (-not (Test-Path $script:WORK)) { New-Item -ItemType Directory -Force -Path $script:WORK | Out-Null }
$script:UTC8 = New-Object System.Text.UTF8Encoding($false)

function Write-Text([string]$path, [string]$text) {
  [System.IO.File]::WriteAllText($path, $text, $script:UTC8)
}
function Read-Bytes([string]$path) {
  if (Test-Path $path) { return [System.IO.File]::ReadAllBytes($path) } else { return @() }
}

# Run one case. Returns a PSCustomObject with Id, Mode, StdIn, Exit, StdOut, StdErr, TimedOut, Ms.
function Run-Case {
  param(
    [Parameter(Mandatory)][string]$Id,
    [AllowEmptyString()][string]$Mode = '',
    [Parameter(Mandatory)][AllowEmptyString()][string]$Text,
    [int]$TimeoutSec = 10,
    [string]$Tag = ''
  )
  $inF  = Join-Path $script:WORK "$Id.in.txt"
  $outF = Join-Path $script:WORK "$Id.out.txt"
  $errF = Join-Path $script:WORK "$Id.err.txt"
  Remove-Item $outF,$errF -ErrorAction SilentlyContinue
  Write-Text $inF $Text
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  $timedOut = $false
  $exit = $null
  try {
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $script:BIN
    $psi.Arguments = $Mode
    $psi.UseShellExecute = $false
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.StandardOutputEncoding = $script:UTC8
    $psi.StandardErrorEncoding = $script:UTC8
    $p = [System.Diagnostics.Process]::Start($psi)
    $outTask = $p.StandardOutput.ReadToEndAsync()
    $errTask = $p.StandardError.ReadToEndAsync()
    # write stdin as raw UTF-8 bytes (StandardInput.Write would use the OEM codepage)
    $inBytes = $script:UTC8.GetBytes($Text)
    $p.StandardInput.BaseStream.Write($inBytes, 0, $inBytes.Length)
    $p.StandardInput.BaseStream.Flush()
    $p.StandardInput.Close()
    if (-not $p.WaitForExit($TimeoutSec * 1000)) {
      $timedOut = $true
      try { Stop-Process -Id $p.Id -Force -ErrorAction Stop } catch { try { $p.Kill() } catch {} }
      try { $p.WaitForExit(3000) | Out-Null } catch {}
    } else {
      $p.WaitForExit()
      $exit = $p.ExitCode
    }
    if (-not $timedOut) {
      # drain
      if (-not $outTask.Wait(3000)) {}
      if (-not $errTask.Wait(3000)) {}
      Write-Text $outF $outTask.Result
      Write-Text $errF $errTask.Result
    }
    $p.Dispose()
  } catch {
    $exit = -999
    Write-Text $errF ("HARNESS-ERROR: " + $_.Exception.Message)
  }
  $sw.Stop()
  if ($timedOut) { $exit = 'TIMEOUT' }
  $outTxt = ''
  if (Test-Path $outF) { $outTxt = [System.IO.File]::ReadAllText($outF) }
  $errTxt = ''
  if (Test-Path $errF) { $errTxt = [System.IO.File]::ReadAllText($errF) }
  [PSCustomObject]@{
    Id = $Id; Tag = $Tag; Mode = $Mode; Ms = [int]$sw.ElapsedMilliseconds
    Exit = $exit; StdOut = $outTxt; StdErr = $errTxt; StdIn = $Text
  }
}

# Run-Case with an expectation, appends a verdict.
# Kind: 'exact'  -> StdOut -eq Expect (and exit 0)
#       'regex'  -> StdOut matches Expect (and exit 0)
#       'err'    -> exit 1 and StdErr matches Expect (regex, multiline)
#       'any'    -> no verdict, just record
function Assert-Case {
  param($R, [string]$Kind, [string]$Expect = '')
  $verdict = 'FAIL'; $why = ''
  switch ($Kind) {
    'exact' {
      if ($R.Exit -eq 0 -and $R.StdOut -ceq $Expect) { $verdict = 'PASS' }
      else { $why = "exit=$($R.Exit) stdout=" + ($R.StdOut -replace "`r?`n", '\n') }
    }
    'regex' {
      if ($R.Exit -eq 0 -and ([regex]::IsMatch($R.StdOut, $Expect))) { $verdict = 'PASS' }
      else { $why = "exit=$($R.Exit) stdout=" + ($R.StdOut -replace "`r?`n", '\n') }
    }
    'err' {
      if ($R.Exit -eq 1 -and ([regex]::IsMatch($R.StdErr, $Expect))) { $verdict = 'PASS' }
      else { $why = "exit=$($R.Exit) stderr=" + ($R.StdErr -replace "`r?`n", '\n') }
    }
    'any' { $verdict = 'INFO' }
    'ok' {
      if ($R.Exit -eq 0) { $verdict = 'PASS' }
      else { $why = "expected exit 0 (no crash), got exit=$($R.Exit) stderr=" + ($R.StdErr -replace "`r?`n", '\n') }
    }
  }
  $R | Add-Member -NotePropertyName Kind -NotePropertyValue $Kind -Force
  $R | Add-Member -NotePropertyName Expect -NotePropertyValue $Expect -Force
  $R | Add-Member -NotePropertyName Verdict -NotePropertyValue $verdict -Force
  $R | Add-Member -NotePropertyName Why -NotePropertyValue $why -Force
  return $R
}

function Render-Record($R) {
  $sb = New-Object System.Text.StringBuilder
  [void]$sb.AppendLine("=== [$($R.Id)] tag=$($R.Tag) mode=$($R.Mode) exit=$($R.Exit) ms=$($R.Ms) verdict=$($R.Verdict)")
  [void]$sb.AppendLine("--- stdin:")
  foreach ($l in ($R.StdIn -split "`r?`n")) { [void]$sb.AppendLine("    | " + $l) }
  [void]$sb.AppendLine("--- stdout:")
  foreach ($l in ($R.StdOut -split "`r?`n")) { [void]$sb.AppendLine("    > " + $l) }
  [void]$sb.AppendLine("--- stderr:")
  foreach ($l in ($R.StdErr -split "`r?`n")) { [void]$sb.AppendLine("    ! " + $l) }
  if ($R.Kind -ne '') { [void]$sb.AppendLine("--- expect($($R.Kind)): $($R.Expect)") }
  if ($R.Why -ne '') { [void]$sb.AppendLine("--- WHY: $($R.Why)") }
  [void]$sb.AppendLine("")
  return $sb.ToString()
}

function Save-Results {
  param($Results, [string]$BaseName)
  $log = New-Object System.Text.StringBuilder
  foreach ($r in $Results) { [void]$log.Append((Render-Record $r)) }
  Write-Text (Join-Path $script:WORK "$BaseName.log") $log.ToString()
  $csv = $Results | Select-Object Id,Tag,Mode,Exit,Verdict,Kind,Ms,Why |
         ConvertTo-Csv -NoTypeInformation
  Write-Text (Join-Path $script:WORK "$BaseName.csv") ($csv -join "`r`n")
  $Results | Format-Table Id,Tag,Mode,Exit,Verdict,Ms -AutoSize | Out-String -Width 200
}
