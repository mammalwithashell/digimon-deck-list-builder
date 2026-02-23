# Playwright MCP Setup

This repo includes scripts to run your app and Playwright MCP locally.

## 1) Start app stack (FastAPI + Vite)

```powershell
powershell -ExecutionPolicy Bypass -File .playwright-mcp/start-stack.ps1
```

Defaults:
- Frontend: `http://127.0.0.1:5173`
- Backend: `http://127.0.0.1:8000`

Logs:
- `data/run/frontend_stdout.log`
- `data/run/frontend_stderr.log`
- `data/run/api_stdout.log`
- `data/run/api_stderr.log`

## 2) Start Playwright MCP server (SSE)

```powershell
powershell -ExecutionPolicy Bypass -File .playwright-mcp/start-mcp.ps1 -StartApp
```

Default MCP endpoint:
- `http://127.0.0.1:8931`

Artifacts:
- `data/run/playwright-mcp`

Stop it:

```powershell
powershell -ExecutionPolicy Bypass -File .playwright-mcp/stop-mcp.ps1
```

## 3) Codex MCP registration (already done on this machine)

The following command registers a global Codex MCP server:

```powershell
codex mcp add playwright -- npx.cmd -y @playwright/mcp@latest --headless --isolated --output-dir data/run/playwright-mcp --save-trace --save-session --allowed-hosts 127.0.0.1 localhost --allowed-origins "http://127.0.0.1:5173;http://localhost:5173;http://127.0.0.1:8000;http://localhost:8000"
```

Check:

```powershell
codex mcp list
codex mcp get playwright --json
```
