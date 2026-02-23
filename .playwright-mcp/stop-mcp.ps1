param(
  [int]$Port = 8931
)

$ErrorActionPreference = "SilentlyContinue"

$listener = Get-NetTCPConnection -LocalPort $Port -State Listen | Select-Object -First 1
if ($null -eq $listener) {
  Write-Output "No MCP listener found on port $Port."
  exit 0
}

$owning = $listener.OwningProcess
Stop-Process -Id $owning -Force
Start-Sleep -Seconds 1

$after = Get-NetTCPConnection -LocalPort $Port -State Listen | Select-Object -First 1
if ($null -eq $after) {
  Write-Output "Stopped MCP listener on port $Port (PID $owning)."
} else {
  Write-Output "Port $Port is still listening (PID $($after.OwningProcess))."
  exit 1
}
