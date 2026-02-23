param(
  [string]$FrontendBindAddress = "127.0.0.1",
  [int]$FrontendPort = 5173,
  [string]$BackendBindAddress = "127.0.0.1",
  [int]$BackendPort = 8000
)

$ErrorActionPreference = "Stop"

function Test-ListeningPort {
  param([int]$Port, [string]$Address = "127.0.0.1")
  try {
    $conn = Get-NetTCPConnection -State Listen -LocalPort $Port -ErrorAction Stop |
      Where-Object { $_.LocalAddress -eq $Address -or $_.LocalAddress -eq "0.0.0.0" -or $_.LocalAddress -eq "::" } |
      Select-Object -First 1
    return $null -ne $conn
  } catch {
    return $false
  }
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$runDir = Join-Path $repoRoot "data/run"
New-Item -ItemType Directory -Path $runDir -Force | Out-Null

$backendUp = Test-ListeningPort -Port $BackendPort -Address $BackendBindAddress
if (-not $backendUp) {
  $apiOut = Join-Path $runDir "api_stdout.log"
  $apiErr = Join-Path $runDir "api_stderr.log"
  $apiProc = Start-Process -FilePath "python" `
    -ArgumentList "-m", "uvicorn", "digimon_gym.api:app", "--host", $BackendBindAddress, "--port", $BackendPort `
    -WorkingDirectory $repoRoot `
    -RedirectStandardOutput $apiOut `
    -RedirectStandardError $apiErr `
    -PassThru
  Start-Sleep -Seconds 2
  Write-Output "Started backend PID=$($apiProc.Id) -> http://$BackendBindAddress`:$BackendPort/"
} else {
  Write-Output "Backend already running -> http://$BackendBindAddress`:$BackendPort/"
}

$frontendUp = Test-ListeningPort -Port $FrontendPort -Address $FrontendBindAddress
if (-not $frontendUp) {
  $feOut = Join-Path $runDir "frontend_stdout.log"
  $feErr = Join-Path $runDir "frontend_stderr.log"
  $feProc = Start-Process -FilePath "npm.cmd" `
    -ArgumentList "run", "dev", "--", "--host", $FrontendBindAddress, "--port", $FrontendPort `
    -WorkingDirectory (Join-Path $repoRoot "frontend") `
    -RedirectStandardOutput $feOut `
    -RedirectStandardError $feErr `
    -PassThru
  Start-Sleep -Seconds 2
  Write-Output "Started frontend PID=$($feProc.Id) -> http://$FrontendBindAddress`:$FrontendPort/"
} else {
  Write-Output "Frontend already running -> http://$FrontendBindAddress`:$FrontendPort/"
}

Write-Output "Stack ready."
