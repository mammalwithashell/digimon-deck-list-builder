param(
  [string]$BindAddress = "127.0.0.1",
  [int]$Port = 8931,
  [switch]$StartApp
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")

if ($StartApp) {
  & (Join-Path $PSScriptRoot "start-stack.ps1")
}

$outputDir = Join-Path $repoRoot "data/run/playwright-mcp"
New-Item -ItemType Directory -Path $outputDir -Force | Out-Null

$allowedOrigins = "http://127.0.0.1:5173;http://localhost:5173;http://127.0.0.1:8000;http://localhost:8000"

Write-Output "Starting Playwright MCP on http://$BindAddress`:$Port"
Write-Output "Output dir: $outputDir"

& npx.cmd -y @playwright/mcp@latest `
  --host $BindAddress `
  --port $Port `
  --headless `
  --isolated `
  --output-dir $outputDir `
  --save-trace `
  --save-session `
  --allowed-hosts 127.0.0.1 localhost `
  --allowed-origins $allowedOrigins
