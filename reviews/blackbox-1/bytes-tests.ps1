# Byte-level CLI robustness tests: non-UTF-8 stdin, UTF-8 BOM, UTF-16, CRLF, empty.
. (Join-Path $PSScriptRoot 'lib.ps1')
$b = $script:BIN
$W = $script:WORK
$log = New-Object System.Text.StringBuilder
function BLog([string]$s) { [void]$log.AppendLine($s); Write-Host $s }

function Run-Bytes {
  param([string]$Id, [byte[]]$Bytes, [string]$Mode = 'nf')
  $inF = Join-Path $W "$Id.in"; $outF = Join-Path $W "$Id.out"; $errF = Join-Path $W "$Id.err"
  [System.IO.File]::WriteAllBytes($inF, $Bytes)
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  & cmd /c "`"$b`" $Mode < `"$inF`" > `"$outF`" 2> `"$errF`""
  $code = $LASTEXITCODE
  $sw.Stop()
  $out = [System.IO.File]::ReadAllText($outF); $err = [System.IO.File]::ReadAllText($errF)
  BLog ("=== [$Id] mode=$Mode exit=$code ms=$($sw.ElapsedMilliseconds)")
  BLog ("    bytes : " + (($Bytes | Select-Object -First 24 | ForEach-Object { $_.ToString('x2') }) -join ' '))
  BLog ("    stdout: " + ($out -replace "`r?`n", ' | '))
  BLog ("    stderr: " + ($err -replace "`r?`n", ' | '))
  BLog ""
  return $code
}

BLog "byte-level stdin tests for $b"
BLog ("run at " + (Get-Date -Format 'yyyy-MM-dd HH:mm:ss'))
BLog ""

$e1 = Run-Bytes 'b-nonutf8' ([byte[]](0x7A,0x65,0x72,0x6F,0x20,0xFF,0xFE))
$e2 = Run-Bytes 'b-loneff'  ([byte[]](0xFF))
$e3 = Run-Bytes 'b-bom8'    ([byte[]](0xEF,0xBB,0xBF) + [System.Text.Encoding]::ASCII.GetBytes("zero"))
$e4 = Run-Bytes 'b-bom8nat' ([byte[]](0xEF,0xBB,0xBF) + [System.Text.Encoding]::ASCII.GetBytes("Nat"))
$e5 = Run-Bytes 'b-utf16'   ([System.Text.Encoding]::Unicode.GetBytes("zero"))
$e6 = Run-Bytes 'b-crlf'    ([System.Text.Encoding]::ASCII.GetBytes("zero`r`n"))
$e7 = Run-Bytes 'b-empty'   ([byte[]]@())
$e8 = Run-Bytes 'b-nul'     ([byte[]](0x7A,0x65,0x72,0x6F,0x00))

BLog "SUMMARY"
BLog ("  non-UTF-8 bytes      -> exit $e1   (documented contract says every error exits 1)")
BLog ("  lone 0xFF byte       -> exit $e2")
BLog ("  UTF-8 BOM + zero     -> exit $e3   (BOM rejected as a token)")
BLog ("  UTF-8 BOM + Nat      -> exit $e4")
BLog ("  UTF-16LE zero        -> exit $e5")
BLog ("  CRLF terminated      -> exit $e6")
BLog ("  empty stdin          -> exit $e7")
BLog ("  trailing NUL         -> exit $e8")
Write-Text (Join-Path $PSScriptRoot 'bytes.log') $log.ToString()
