"""Claude Code adapter: `claude -p --output-format json --json-schema …` (design D10).

Invocation (prompt on stdin, cwd = the packet's worktree):

    claude -p --output-format json --json-schema <inline schema JSON>
           --model <m> [--effort <e>] [--max-budget-usd <b>]
           --permission-mode acceptEdits --allowedTools <tool> …
           --add-dir <read-only dir> … --disallowedTools Edit(//<read-only dir>/**) …
           --session-id <uuid5(attempt, call)>

* `--json-schema` takes the schema itself (inline JSON), not a path.
* `acceptEdits` auto-approves file edits inside the cwd and every `--add-dir`;
  anything else not in `--allowedTools` is denied (print mode never prompts).
  `--add-dir` is what grants READ access to the base-repo DCGO / rules PDFs, so
  each such dir also gets an `Edit(//…/**)` deny rule (Edit rules cover every
  file-editing tool), keeping it read-only. `bypassPermissions` is refused, the
  Claude analogue of Codex's `danger-full-access`.
* The session id is deterministic per (attempt, call), so the full transcript is
  findable under ~/.claude/projects/ and `claude --resume <id>` works.
"""
from __future__ import annotations

import json
import os
import shutil
import subprocess
import uuid
from pathlib import Path
from typing import Mapping, Sequence

from ..config import LoopConfig
from ..contracts import TaskPacket, Usage
from .base import (
    CliWorker,
    Invocation,
    ParsedCall,
    ProcResult,
    base_reference_dirs,
    classify_failure,
    load_schema,
    parse_json_text,
    render_prompt,
    scrubbed_env,
    tail,
)
from .pool import worktree_env

ENV_EXE = "CARD_LOOP_CLAUDE_EXE"

DEFAULT_PERMISSION_MODE = "acceptEdits"
REFUSED_PERMISSION_MODES = frozenset({"bypassPermissions"})
REFUSED_FLAGS = frozenset({"--dangerously-skip-permissions", "--allow-dangerously-skip-permissions"})

# Proposed config key `claude_allowed_tools`. File tools are covered by
# acceptEdits inside the worktree; Bash is limited to build / test / read-only git
# and the repo's python tools; the exam MCP server is allowed whole.
DEFAULT_ALLOWED_TOOLS: tuple[str, ...] = (
    "Read", "Glob", "Grep", "Edit", "Write",
    "Bash(cargo build:*)", "Bash(cargo check:*)", "Bash(cargo test:*)",
    "Bash(cargo run:*)", "Bash(cargo nextest:*)",
    "Bash(python -m pytest:*)", "Bash(python -m tools.:*)",
    "Bash(git status:*)", "Bash(git diff:*)", "Bash(git log:*)",
    "Bash(git show:*)", "Bash(git blame:*)", "Bash(git rev-parse:*)",
    "mcp__dcgo-exam",
)

DEFAULT_TIMEOUT_S = 3600.0
_SESSION_NS = uuid.UUID("6f1c2a7e-3c0b-4d8e-9a51-0c1d2e3f4a5b")


def resolve_claude_exe() -> str | None:
    """`$CARD_LOOP_CLAUDE_EXE` if set (None if it does not exist), else `claude` on PATH."""
    override = os.environ.get(ENV_EXE)
    if override:
        return override if Path(override).is_file() else None
    return shutil.which("claude")


def claude_auth_status(exe: str | None = None, *, run=subprocess.run, timeout: float = 60.0) -> dict | None:
    """`claude auth status --json` (no API call, no spend), e.g. `{"loggedIn": false,
    "authMethod": "none", ...}`; None if the CLI cannot be run or prints no JSON.
    `--version` succeeds while logged out, so preflight should check this too."""
    exe = exe or resolve_claude_exe()
    if not exe:
        return None
    try:
        cp = run([exe, "auth", "status", "--json"], capture_output=True, text=True,
                 timeout=timeout, check=False, env=scrubbed_env())
    except (OSError, subprocess.TimeoutExpired):
        return None
    try:
        data = json.loads(cp.stdout)
    except (json.JSONDecodeError, TypeError):
        return None
    return data if isinstance(data, dict) else None


def posix_rule_path(path: str | os.PathLike) -> str:
    """Absolute path in Claude Code permission-rule form: `//c/Users/x` on Windows
    (paths are normalised to POSIX form before matching), `//home/x` elsewhere."""
    p = Path(os.path.abspath(str(path)))
    drive = p.drive
    rest = p.as_posix()[len(drive):] if drive else p.as_posix()
    if drive and drive.endswith(":"):
        return "//" + drive[0].lower() + rest
    return "/" + rest if rest.startswith("/") else "//" + rest


def session_id_for(attempt_id: str, call: str) -> str:
    return str(uuid.uuid5(_SESSION_NS, f"{attempt_id}/{call}"))


def build_claude_argv(
    *,
    exe: str,
    schema: Mapping,
    model: str | None,
    effort: str | None = None,
    budget_usd: float | None = None,
    permission_mode: str = DEFAULT_PERMISSION_MODE,
    allowed_tools: Sequence[str] = DEFAULT_ALLOWED_TOOLS,
    read_only_dirs: Sequence[str] = (),
    session_id: str | None = None,
    extra_args: Sequence[str] = (),
) -> list[str]:
    if permission_mode in REFUSED_PERMISSION_MODES:
        raise ValueError(f"permission mode {permission_mode!r} is refused for card-loop workers")
    bad = [a for a in extra_args if a.split("=", 1)[0] in REFUSED_FLAGS
           or a.split("=", 1)[-1] in REFUSED_PERMISSION_MODES]
    if bad:
        raise ValueError(f"refused claude args: {bad}")
    argv = [exe, "-p", "--output-format", "json",
            "--json-schema", json.dumps(schema, separators=(",", ":"))]
    if model:
        argv += ["--model", model]
    if effort:
        argv += ["--effort", effort]
    if budget_usd is not None:
        argv += ["--max-budget-usd", f"{budget_usd:.4f}".rstrip("0").rstrip(".")]
    argv += ["--permission-mode", permission_mode]
    if allowed_tools:
        argv += ["--allowedTools", *allowed_tools]
    for d in read_only_dirs:
        argv += ["--add-dir", str(d)]
    if read_only_dirs:
        argv += ["--disallowedTools", *[f"Edit({posix_rule_path(d)}/**)" for d in read_only_dirs]]
    if session_id:
        argv += ["--session-id", session_id]
    argv += list(extra_args)
    return argv


def _usage_from_envelope(env: Mapping) -> Usage:
    """Tokens from `modelUsage` (every model the session used, consistent with
    `total_cost_usd`), falling back to the top-level `usage` block."""
    cost = env.get("total_cost_usd")
    model_usage = env.get("modelUsage")
    if isinstance(model_usage, Mapping) and model_usage:
        tot = {"in": 0, "out": 0, "cr": 0, "cw": 0}
        for mu in model_usage.values():
            tot["in"] += int(mu.get("inputTokens", 0) or 0)
            tot["out"] += int(mu.get("outputTokens", 0) or 0)
            tot["cr"] += int(mu.get("cacheReadInputTokens", 0) or 0)
            tot["cw"] += int(mu.get("cacheCreationInputTokens", 0) or 0)
        return Usage(input_tokens=tot["in"], output_tokens=tot["out"], cache_read_tokens=tot["cr"],
                     cache_write_tokens=tot["cw"],
                     cost_usd=float(cost) if cost is not None else None)
    u = env.get("usage") or {}
    return Usage(
        input_tokens=int(u.get("input_tokens", 0) or 0),
        output_tokens=int(u.get("output_tokens", 0) or 0),
        cache_read_tokens=int(u.get("cache_read_input_tokens", 0) or 0),
        cache_write_tokens=int(u.get("cache_creation_input_tokens", 0) or 0),
        cost_usd=float(cost) if cost is not None else None,
    )


def _find_envelope(stdout: str) -> dict | None:
    s = stdout.strip()
    if not s:
        return None
    try:
        obj = json.loads(s)
        if isinstance(obj, dict):
            return obj
        if isinstance(obj, list):  # stream-json collected as an array
            results = [o for o in obj if isinstance(o, dict) and o.get("type") == "result"]
            return results[-1] if results else None
    except json.JSONDecodeError:
        pass
    for line in reversed(s.splitlines()):  # stray log lines around the envelope
        line = line.strip()
        if line.startswith("{"):
            try:
                obj = json.loads(line)
            except json.JSONDecodeError:
                continue
            if isinstance(obj, dict) and obj.get("type") == "result":
                return obj
    return None


def parse_claude_envelope(stdout: str, stderr: str = "", returncode: int | None = 0,
                          *, timed_out: bool = False, timeout_s: float | None = None) -> ParsedCall:
    """Map one `--output-format json` call onto (status, result, usage, error)."""
    env = _find_envelope(stdout)
    usage = _usage_from_envelope(env) if env else Usage()
    if timed_out:
        return ParsedCall("error", usage=usage, error=f"timeout: claude exceeded {timeout_s}s")
    if env is None:
        msg = tail(stderr) or tail(stdout) or f"claude exited {returncode} with no output"
        kind = classify_failure(msg)
        return ParsedCall("quota_exhausted" if kind == "quota" else "error", usage=usage,
                          error=f"claude exited {returncode}: {msg}")
    subtype = env.get("subtype")
    if env.get("is_error") or (subtype and subtype != "success"):
        # Recorded: an auth failure arrives as subtype "success" + is_error true.
        parts = [str(subtype)] if subtype and subtype != "success" else []
        if env.get("api_error_status") is not None:
            parts.append(f"api_error_status {env['api_error_status']}")
        if env.get("result"):
            parts.append(str(env["result"]))
        errs = env.get("errors")
        if errs:
            parts.append("; ".join(str(e) for e in errs))
        msg = ": ".join(parts) or "claude reported an error with no message"
        if stderr.strip():
            msg += f" | stderr: {tail(stderr, 500)}"
        kind = classify_failure(msg)
        return ParsedCall("quota_exhausted" if kind == "quota" else "error", usage=usage, error=msg)
    result = env.get("structured_output")
    if result is None:
        result = parse_json_text(env.get("result"))
    return ParsedCall("ok", result=result, usage=usage)


class ClaudeWorker(CliWorker):
    family = "claude"
    transcript_name = "envelope.json"

    def __init__(
        self,
        config: LoopConfig | None = None,
        *,
        exe: str | None = None,
        runner=None,
        artifacts_root=None,
        read_only_dirs: Sequence[str] | None = None,
        permission_mode: str = DEFAULT_PERMISSION_MODE,
        allowed_tools: Sequence[str] = DEFAULT_ALLOWED_TOOLS,
        extra_args: Sequence[str] = (),
        timeout_s: float | None = DEFAULT_TIMEOUT_S,
        capture: bool = True,
    ):
        super().__init__(runner=runner, artifacts_root=artifacts_root, timeout_s=timeout_s, capture=capture)
        self.config = config or LoopConfig()
        self._exe = exe
        self.read_only_dirs = None if read_only_dirs is None else [str(d) for d in read_only_dirs]
        self.permission_mode = permission_mode
        self.allowed_tools = tuple(allowed_tools)
        self.extra_args = tuple(extra_args)
        # Fail at construction, not mid-run, on a refused mode / flag.
        build_claude_argv(exe="claude", schema={}, model=None, permission_mode=permission_mode,
                          allowed_tools=(), extra_args=self.extra_args)

    @property
    def exe(self) -> str:
        exe = self._exe or resolve_claude_exe()
        if not exe:
            raise FileNotFoundError(f"claude CLI not found on PATH (or set {ENV_EXE})")
        return exe

    def _read_only_dirs(self, packet: TaskPacket) -> list[str]:
        dirs = list(self.read_only_dirs) if self.read_only_dirs is not None else base_reference_dirs(packet.worktree)
        wt = os.path.normcase(os.path.abspath(packet.worktree))
        covered = [os.path.normcase(os.path.abspath(d)) for d in dirs]
        for ref in packet.references:
            p = os.path.normcase(os.path.abspath(ref if os.path.isabs(ref) else os.path.join(packet.worktree, ref)))
            if p.startswith(wt + os.sep) or any(p == c or p.startswith(c + os.sep) for c in covered):
                continue
            parent = p if os.path.isdir(p) else os.path.dirname(p)
            dirs.append(parent)
            covered.append(os.path.normcase(parent))
        return dirs

    def build_invocation(self, packet: TaskPacket, call_dir: Path) -> Invocation:
        models = self.config.models.get("claude", {})
        argv = build_claude_argv(
            exe=self.exe,
            schema=load_schema(packet.schema_path),
            model=packet.model or models.get("model"),
            effort=packet.effort or models.get("effort"),
            budget_usd=packet.budget_usd,
            permission_mode=self.permission_mode,
            allowed_tools=self.allowed_tools,
            read_only_dirs=self._read_only_dirs(packet),
            session_id=session_id_for(packet.attempt_id, call_dir.name),
            extra_args=self.extra_args,
        )
        env = worktree_env(packet.worktree, self.config.cargo_target_base, self.config.sccache_dir)
        return Invocation(argv=argv, stdin=render_prompt(packet), cwd=str(packet.worktree), env=env)

    def parse_call(self, proc: ProcResult, call_dir: Path, packet: TaskPacket) -> ParsedCall:
        return parse_claude_envelope(proc.stdout, proc.stderr, proc.returncode,
                                     timed_out=proc.timed_out, timeout_s=self.timeout_s)
