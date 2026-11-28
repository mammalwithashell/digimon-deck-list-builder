"""Line-delimited JSON-RPC 2.0 Stdio MCP Client + Deterministic `MockMcpTransport`.

Speaks the MCP stdio wire protocol (`initialize`, `notifications/initialized`,
`tools/list`, `tools/call`) and unwraps the `{ "content": [{ "type": "text", "text": "<JSON>" }] }`
envelope shared by `digimon-engine-mcp` (§2.2) and `digimon-training-mcp` (§2.3).
"""

from __future__ import annotations

import collections
import json
import subprocess
import sys
import threading
from abc import ABC, abstractmethod
from pathlib import Path
from typing import Any, Callable, Mapping, Optional, Sequence

from . import resolve_repo_root
from .llm_router import ToolSpec

MCP_PROTOCOL_VERSION = "2024-11-05"


class McpError(RuntimeError):
    """Base exception for MCP transport or protocol errors."""


class McpRpcError(McpError):
    """Raised when the MCP server returns a JSON-RPC `error` object."""

    def __init__(self, code: int, message: str, data: Any = None) -> None:
        super().__init__(f"MCP JSON-RPC error ({code}): {message}")
        self.code = code
        self.rpc_message = message
        self.data = data


class McpToolError(McpError):
    """Raised when `check_ok=True` and a tool payload reports `ok=False` or `isError=True`."""

    def __init__(self, tool_name: str, payload: Mapping[str, Any]) -> None:
        err_msg = payload.get("error") or payload.get("message") or str(payload)
        super().__init__(f"MCP tool {tool_name!r} failed: {err_msg}")
        self.tool_name = tool_name
        self.payload = dict(payload)


class McpTransport(ABC):
    """Line-oriented JSON-RPC transport abstraction."""

    @abstractmethod
    def send_line(self, line: str) -> None:
        raise NotImplementedError

    @abstractmethod
    def recv_line(self) -> str:
        raise NotImplementedError

    @abstractmethod
    def close(self) -> None:
        raise NotImplementedError


class SubprocessStdioTransport(McpTransport):
    """Stdio transport backed by a child `subprocess.Popen` MCP server."""

    def __init__(
        self,
        command: Sequence[str],
        *,
        cwd: Optional[Path] = None,
        env: Optional[Mapping[str, str]] = None,
    ) -> None:
        self.command = [str(c) for c in command]
        self._stderr_tail: collections.deque[str] = collections.deque(maxlen=100)
        self._proc = subprocess.Popen(
            self.command,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            bufsize=1,
            cwd=str(cwd) if cwd is not None else None,
            env=dict(env) if env is not None else None,
        )
        self._stderr_thread = threading.Thread(
            target=self._drain_stderr,
            name="mcp-stderr-drain",
            daemon=True,
        )
        self._stderr_thread.start()

    def _drain_stderr(self) -> None:
        assert self._proc.stderr is not None
        try:
            for line in self._proc.stderr:
                self._stderr_tail.append(line.rstrip("\r\n"))
        except Exception:
            pass

    @property
    def stderr_tail(self) -> list[str]:
        return list(self._stderr_tail)

    def send_line(self, line: str) -> None:
        if self._proc.stdin is None or self._proc.poll() is not None:
            tail = "\n".join(self._stderr_tail)
            raise McpError(f"MCP subprocess {self.command!r} is not running. Stderr:\n{tail}")
        self._proc.stdin.write(line.rstrip("\r\n") + "\n")
        self._proc.stdin.flush()

    def recv_line(self) -> str:
        if self._proc.stdout is None:
            raise McpError("MCP subprocess stdout pipe is closed")
        while True:
            line = self._proc.stdout.readline()
            if line == "":
                rc = self._proc.poll()
                tail = "\n".join(self._stderr_tail)
                raise McpError(
                    f"EOF from MCP subprocess {self.command!r} (returncode={rc}). Stderr:\n{tail}"
                )
            stripped = line.strip()
            if stripped:
                return stripped

    def close(self) -> None:
        try:
            if self._proc.stdin and not self._proc.stdin.closed:
                self._proc.stdin.close()
        except Exception:
            pass
        if self._proc.poll() is None:
            try:
                self._proc.wait(timeout=3.0)
            except subprocess.TimeoutExpired:
                self._proc.terminate()
                try:
                    self._proc.wait(timeout=2.0)
                except subprocess.TimeoutExpired:
                    self._proc.kill()
        for pipe in (self._proc.stdout, self._proc.stderr):
            try:
                if pipe and not pipe.closed:
                    pipe.close()
            except Exception:
                pass


class MockMcpTransport(McpTransport):
    """Deterministic in-memory JSON-RPC 2.0 MCP server transport for offline tests."""

    def __init__(
        self,
        *,
        server_name: str = "mock-digimon-mcp",
        server_version: str = "0.1.0",
        tools: Optional[Mapping[str, Callable[[dict[str, Any]], Mapping[str, Any]]]] = None,
    ) -> None:
        self.server_name = server_name
        self.server_version = server_version
        self._handlers: dict[str, Callable[[dict[str, Any]], Mapping[str, Any]]] = {}
        self._tool_defs: dict[str, dict[str, Any]] = {}
        self._outbox: collections.deque[str] = collections.deque()
        self.raw_requests: list[dict[str, Any]] = []
        self.calls: list[tuple[str, dict[str, Any]]] = []
        self.initialized = False
        self.closed = False

        for name, fn in (tools or {}).items():
            self.register_tool(name, fn)

    def register_tool(
        self,
        name: str,
        handler: Callable[[dict[str, Any]], Mapping[str, Any]],
        *,
        description: Optional[str] = None,
        input_schema: Optional[dict[str, Any]] = None,
    ) -> None:
        self._handlers[name] = handler
        self._tool_defs[name] = {
            "name": name,
            "description": description or f"Mock MCP tool {name}",
            "inputSchema": input_schema
            or {
                "type": "object",
                "properties": {},
            },
        }

    def has_tool(self, name: str) -> bool:
        return name in self._handlers

    def send_line(self, line: str) -> None:
        if self.closed:
            raise McpError("MockMcpTransport is closed")
        stripped = line.strip()
        if not stripped:
            return
        try:
            req = json.loads(stripped)
        except Exception as exc:
            err_resp = {
                "jsonrpc": "2.0",
                "id": None,
                "error": {"code": -32700, "message": f"parse error: {exc}"},
            }
            self._outbox.append(json.dumps(err_resp))
            return

        self.raw_requests.append(req)
        req_id = req.get("id")
        method = req.get("method", "")
        params = req.get("params") or {}

        # Notifications (`id` absent) must NOT emit a wire response.
        if method in ("notifications/initialized", "initialized"):
            self.initialized = True
            if "id" in req and req_id is not None:
                self._outbox.append(json.dumps({"jsonrpc": "2.0", "id": req_id, "result": None}))
            return

        if method == "initialize":
            resp = {
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {
                    "protocolVersion": MCP_PROTOCOL_VERSION,
                    "capabilities": {"tools": {}},
                    "serverInfo": {
                        "name": self.server_name,
                        "version": self.server_version,
                    },
                },
            }
            self._outbox.append(json.dumps(resp))
            return

        if method == "ping":
            self._outbox.append(json.dumps({"jsonrpc": "2.0", "id": req_id, "result": {}}))
            return

        if method == "tools/list":
            resp = {
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {"tools": list(self._tool_defs.values())},
            }
            self._outbox.append(json.dumps(resp))
            return

        if method == "tools/call":
            tname = params.get("name")
            targs = dict(params.get("arguments") or {})
            if not tname or tname not in self._handlers:
                err_resp = {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "error": {"code": -32602, "message": f"unknown tool: {tname!r}"},
                }
                self._outbox.append(json.dumps(err_resp))
                return
            self.calls.append((str(tname), targs))
            try:
                payload = dict(self._handlers[str(tname)](targs))
            except Exception as exc:
                payload = {"ok": False, "error": str(exc)}
            resp = {
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {
                    "content": [
                        {
                            "type": "text",
                            "text": json.dumps(payload, ensure_ascii=False),
                        }
                    ]
                },
            }
            self._outbox.append(json.dumps(resp))
            return

        err_resp = {
            "jsonrpc": "2.0",
            "id": req_id,
            "error": {"code": -32601, "message": f"method not found: {method}"},
        }
        self._outbox.append(json.dumps(err_resp))

    def recv_line(self) -> str:
        if self.closed:
            raise McpError("MockMcpTransport is closed")
        if not self._outbox:
            raise McpError("MockMcpTransport outbox is empty (no pending response)")
        return self._outbox.popleft()

    def close(self) -> None:
        self.closed = True


class StdioMcpClient:
    """Synchronous JSON-RPC 2.0 MCP client over stdio or `MockMcpTransport`."""

    def __init__(
        self,
        transport: McpTransport,
        *,
        auto_initialize: bool = False,
    ) -> None:
        self.transport = transport
        self._next_id = 1
        self._initialized = False
        self.server_info: dict[str, Any] = {}
        self._cached_tools: Optional[list[dict[str, Any]]] = None
        if auto_initialize:
            self.initialize()

    @classmethod
    def from_command(
        cls,
        command: Sequence[str],
        *,
        cwd: Optional[Path] = None,
        env: Optional[Mapping[str, str]] = None,
    ) -> "StdioMcpClient":
        return cls(SubprocessStdioTransport(command, cwd=cwd, env=env))

    @classmethod
    def for_engine_mcp(
        cls,
        *,
        repo_root: Optional[Path] = None,
        python_exe: Optional[str] = None,
        pool: str = "implemented",
        cards_json: Optional[Path] = None,
    ) -> "StdioMcpClient":
        root = repo_root or resolve_repo_root()
        py = python_exe or sys.executable
        cmd = [py, str(root / "scripts" / "mcp_launch.py"), "digimon-engine-mcp", "--pool", pool]
        if cards_json is not None:
            cmd.extend(["--cards-json", str(cards_json)])
        return cls.from_command(cmd, cwd=root)

    @classmethod
    def for_training_mcp(
        cls,
        *,
        repo_root: Optional[Path] = None,
        python_exe: Optional[str] = None,
        runs_dir: Optional[Path] = None,
        models_dir: Optional[Path] = None,
    ) -> "StdioMcpClient":
        root = repo_root or resolve_repo_root()
        py = python_exe or sys.executable
        cmd = [py, "-m", "digimon_training_mcp"]
        if runs_dir is not None:
            cmd.extend(["--runs-dir", str(runs_dir)])
        if models_dir is not None:
            cmd.extend(["--models-dir", str(models_dir)])
        return cls.from_command(cmd, cwd=root)

    def __enter__(self) -> "StdioMcpClient":
        if not self._initialized:
            self.initialize()
        return self

    def __exit__(self, exc_type: Any, exc: Any, tb: Any) -> None:
        self.close()

    def close(self) -> None:
        self.transport.close()

    def _request(self, method: str, params: Optional[Mapping[str, Any]] = None) -> Any:
        req_id = self._next_id
        self._next_id += 1
        wire_req: dict[str, Any] = {
            "jsonrpc": "2.0",
            "id": req_id,
            "method": method,
        }
        if params is not None:
            wire_req["params"] = dict(params)

        self.transport.send_line(json.dumps(wire_req, ensure_ascii=False))

        while True:
            raw_line = self.transport.recv_line()
            resp = json.loads(raw_line)
            # Ignore stray notifications if a server emits any.
            if "id" not in resp and "method" in resp:
                continue
            if resp.get("id") != req_id:
                raise McpError(
                    f"Mismatched JSON-RPC response id: expected {req_id}, got {resp.get('id')!r}"
                )
            if "error" in resp and resp["error"] is not None:
                err = resp["error"]
                raise McpRpcError(
                    int(err.get("code", -32000)),
                    str(err.get("message", "unknown RPC error")),
                    err.get("data"),
                )
            return resp.get("result")

    def _notify(self, method: str, params: Optional[Mapping[str, Any]] = None) -> None:
        wire_notif: dict[str, Any] = {
            "jsonrpc": "2.0",
            "method": method,
        }
        if params is not None:
            wire_notif["params"] = dict(params)
        self.transport.send_line(json.dumps(wire_notif, ensure_ascii=False))

    def initialize(self) -> dict[str, Any]:
        result = self._request(
            "initialize",
            {
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {
                    "name": "digimon-puzzle-lab",
                    "version": "0.1.0",
                },
            },
        )
        self._notify("notifications/initialized")
        self._initialized = True
        self.server_info = dict((result or {}).get("serverInfo") or {})
        return dict(result or {})

    def ensure_initialized(self) -> None:
        if not self._initialized:
            self.initialize()

    def ping(self) -> None:
        self.ensure_initialized()
        self._request("ping")

    def list_tools(self, *, refresh: bool = False) -> list[dict[str, Any]]:
        self.ensure_initialized()
        if self._cached_tools is None or refresh:
            res = self._request("tools/list", {}) or {}
            self._cached_tools = list(res.get("tools") or [])
        return list(self._cached_tools)

    def has_tool(self, name: str) -> bool:
        return any(t.get("name") == name for t in self.list_tools())

    def to_llm_tool_specs(
        self,
        *,
        allowed_names: Optional[Sequence[str]] = None,
    ) -> list[ToolSpec]:
        allowed = set(allowed_names) if allowed_names is not None else None
        specs: list[ToolSpec] = []
        for t in self.list_tools():
            tname = str(t.get("name") or "")
            if not tname or (allowed is not None and tname not in allowed):
                continue
            schema = dict(t.get("inputSchema") or {"type": "object", "properties": {}})
            specs.append(
                ToolSpec(
                    name=tname,
                    description=str(t.get("description") or ""),
                    parameters=schema,
                )
            )
        return specs

    def call_tool(
        self,
        name: str,
        arguments: Optional[Mapping[str, Any]] = None,
        *,
        check_ok: bool = False,
    ) -> dict[str, Any]:
        """Call `tools/call` and parse the `{ content: [{ type: 'text', text: '<JSON>' }] }` envelope."""
        self.ensure_initialized()
        res = self._request(
            "tools/call",
            {
                "name": name,
                "arguments": dict(arguments or {}),
            },
        )
        if not isinstance(res, Mapping):
            raise McpError(f"MCP tools/call {name!r} returned non-object result: {res!r}")

        content = res.get("content") or []
        parsed: dict[str, Any]
        if isinstance(content, list) and content:
            first = content[0]
            if isinstance(first, Mapping) and first.get("type") == "text":
                raw_text = str(first.get("text") or "{}")
                try:
                    loaded = json.loads(raw_text)
                    parsed = loaded if isinstance(loaded, dict) else {"ok": True, "value": loaded}
                except Exception:
                    parsed = {"ok": not bool(res.get("isError")), "text": raw_text}
            else:
                parsed = {"ok": not bool(res.get("isError")), "content": content}
        else:
            parsed = dict(res)

        if res.get("isError") and "ok" not in parsed:
            parsed["ok"] = False

        if check_ok and (res.get("isError") or parsed.get("ok") is False):
            raise McpToolError(name, parsed)
        return parsed
