"""Codex adapter: `codex exec … --output-schema … -o … --json` (design D10).

Invocation (prompt on stdin via the trailing `-`):

    codex exec -C <worktree> -s workspace-write -c approval_policy="never"
               --add-dir <cargo_target_base>\\<worktree name> --add-dir <sccache_dir>
               --output-schema <schema file> -o <call dir>\\last_message.json
               --json --color never [-m <model>] [-c model_reasoning_effort="<e>"] -

* The binary is resolved at runtime (`resolve_codex_exe`): `$CARD_LOOP_CODEX_EXE`,
  then `codex` on PATH, then the Microsoft Store package
  (`(Get-AppxPackage -Name 'OpenAI.Codex*').InstallLocation\\app\\resources\\codex.exe`),
  whose version directory changes on every Store update.
* The user's `~/.codex/config.toml` defaults to `danger-full-access` +
  `approval_policy = "never"` and registers the MCP servers workers need, so it is
  NOT ignored; instead the sandbox and approval policy are explicit on every call
  (`-s` outranks config.toml and profiles) and any argv that could make the
  effective sandbox `danger-full-access` raises `SandboxPolicyError`.
* `workspace-write` can read the whole disk, so read-only references (base DCGO,
  rules PDFs) need no flag; `--add-dir` makes the per-worktree cargo target and
  the shared sccache dir writable.
* Codex reports tokens, not dollars: cost is priced from `config.prices["codex"]`
  (USD per million tokens) and marked `cost_derived`; unpriced -> `cost_usd=None`.
"""
from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable, Iterable, Mapping, Sequence

from ..config import LoopConfig
from ..contracts import TaskPacket, Usage
from .base import CliWorker, Invocation, ParsedCall, ProcResult, classify_failure, parse_json_text, render_prompt, tail
from .pool import cargo_target_dir, worktree_env

ENV_EXE = "CARD_LOOP_CODEX_EXE"
APPX_QUERY = "(Get-AppxPackage -Name 'OpenAI.Codex*').InstallLocation"
APPX_EXE = ("app", "resources", "codex.exe")

SANDBOX = "workspace-write"
APPROVAL = "never"
#: Codex 0.162 (Store 26.1002, 2026-10-07) gates every MCP tool call behind an
#: approval, which `approval_policy="never"` refuses -- so workers silently lost
#: exam_validate / exam_probe / exam_authoring_guide. Pre-approve the loop's own
#: exam server only (`AppToolApproval`: auto | prompt | writes | approve); no
#: other MCP server is touched. Older builds ignore the unknown key.
MCP_APPROVALS: tuple[str, ...] = ('mcp_servers.dcgo-exam.default_tools_approval_mode="approve"',)
DEFAULT_TIMEOUT_S = 3600.0


class SandboxPolicyError(ValueError):
    """The argv would not run Codex under exactly `-s workspace-write`."""


# --------------------------------------------------------------------------- binary


_APPX_CACHE: list = []  # [] = not looked up yet in this process; [path-or-None]


def resolve_codex_exe() -> str | None:
    """`$CARD_LOOP_CODEX_EXE` (None if missing), `codex` on PATH, else the Store
    package's bundled `codex.exe` (looked up once per process, re-resolved if the
    cached path disappears after a Store update)."""
    override = os.environ.get(ENV_EXE)
    if override:
        return override if Path(override).is_file() else None
    on_path = shutil.which("codex")
    if on_path:
        return on_path
    if os.name != "nt":
        return None
    if not _APPX_CACHE or (_APPX_CACHE[0] and not os.path.isfile(_APPX_CACHE[0])):
        _APPX_CACHE[:] = [appx_codex_exe()]
    return _APPX_CACHE[0]


def _reset_resolver_cache() -> None:
    _APPX_CACHE.clear()


def appx_codex_exe(run: Callable = subprocess.run) -> str | None:
    try:
        cp = run(["powershell", "-NoProfile", "-NonInteractive", "-Command", APPX_QUERY],
                 capture_output=True, text=True, timeout=60, check=False)
    except (OSError, subprocess.TimeoutExpired):
        return None
    return pick_appx_exe((cp.stdout or "").splitlines())


def codex_login_status(exe: str | None = None, *, run: Callable = subprocess.run,
                       timeout: float = 60.0) -> tuple[bool, str]:
    """`codex login status` (no model call): (logged_in, first output line), e.g.
    (True, "Logged in using ChatGPT"). A ChatGPT login means usage is metered by the
    subscription's limits, so priced costs are API-equivalent, not billed dollars."""
    exe = exe or resolve_codex_exe()
    if not exe:
        return False, "codex CLI not found"
    try:
        cp = run([exe, "login", "status"], capture_output=True, text=True, timeout=timeout, check=False)
    except (OSError, subprocess.TimeoutExpired) as e:
        return False, f"codex login status failed: {e}"
    text = ((cp.stdout or "") + (cp.stderr or "")).strip()
    line = text.splitlines()[0] if text else ""
    low = line.lower()
    return cp.returncode == 0 and "logged in" in low and "not logged in" not in low, line


def _version_key(location: str) -> tuple[int, ...]:
    m = re.search(r"_(\d+(?:\.\d+)+)_", Path(location.strip()).name)
    return tuple(int(x) for x in m.group(1).split(".")) if m else ()


def pick_appx_exe(locations: Iterable[str], exists: Callable[[str], bool] = os.path.isfile) -> str | None:
    """Newest-version package location that actually contains codex.exe."""
    found = []
    for loc in locations:
        loc = loc.strip()
        if not loc:
            continue
        exe = os.path.join(loc, *APPX_EXE)
        if exists(exe):
            found.append((_version_key(loc), exe))
    return max(found)[1] if found else None


# --------------------------------------------------------------------------- argv


def _config_kv(kv: str) -> tuple[str, str]:
    key, _, value = kv.partition("=")
    return key.strip(), value.strip().strip('"').strip("'")


def check_sandbox_args(argv: Sequence[str]) -> None:
    """Raise `SandboxPolicyError` unless the effective sandbox is exactly
    `workspace-write` with `approval_policy="never"`."""
    args = list(argv)
    sandboxes: list[str] = []
    for i, a in enumerate(args):
        low = a.lower()
        nxt = args[i + 1] if i + 1 < len(args) else ""
        if "danger-full-access" in low:
            raise SandboxPolicyError(f"danger-full-access is refused: {a!r}")
        if low.startswith("--dangerously-bypass") or low in ("--yolo", "--full-auto"):
            raise SandboxPolicyError(f"refused flag {a!r}")
        if a in ("-s", "--sandbox"):
            sandboxes.append(nxt)
        elif a.startswith("--sandbox="):
            sandboxes.append(a.split("=", 1)[1])
        elif a.startswith("-s") and not a.startswith("--") and len(a) > 2:
            sandboxes.append(a[2:])
        kv = nxt if a in ("-c", "--config") else (a.split("=", 1)[1] if a.startswith("--config=") else None)
        if kv is not None:
            key, value = _config_kv(kv)
            if key == "sandbox_mode" and value != SANDBOX:
                raise SandboxPolicyError(f"sandbox_mode override {value!r} is refused")
            if key == "approval_policy" and value != APPROVAL:
                raise SandboxPolicyError(f"approval_policy override {value!r} is refused")
    if sandboxes != [SANDBOX]:
        raise SandboxPolicyError(f"expected exactly one `-s {SANDBOX}`, got {sandboxes}")


def build_codex_argv(
    *,
    exe: str,
    worktree: str | os.PathLike,
    schema_path: str | os.PathLike,
    result_path: str | os.PathLike,
    writable_dirs: Sequence[str] = (),
    model: str | None = None,
    effort: str | None = None,
    extra_config: Sequence[str] = (),
    extra_args: Sequence[str] = (),
) -> list[str]:
    argv = [exe, "exec", "-C", str(worktree), "-s", SANDBOX, "-c", f'approval_policy="{APPROVAL}"']
    for kv in MCP_APPROVALS:
        argv += ["-c", kv]
    for d in writable_dirs:
        argv += ["--add-dir", str(d)]
    argv += ["--output-schema", str(schema_path), "-o", str(result_path), "--json", "--color", "never"]
    if model:
        argv += ["-m", model]
    if effort:
        argv += ["-c", f'model_reasoning_effort="{effort}"']
    for kv in extra_config:
        argv += ["-c", kv]
    argv += list(extra_args)
    argv.append("-")  # prompt from stdin
    check_sandbox_args(argv)
    return argv


# --------------------------------------------------------------------------- events


@dataclass
class CodexEvents:
    thread_id: str | None = None
    input_tokens: int = 0
    cached_input_tokens: int = 0
    cache_write_input_tokens: int = 0
    output_tokens: int = 0
    reasoning_output_tokens: int = 0
    saw_usage: bool = False
    turns_completed: int = 0
    failures: list[str] = field(default_factory=list)   # turn.failed
    errors: list[str] = field(default_factory=list)     # error events (may be non-fatal retries)
    last_message: str | None = None
    unknown_types: set[str] = field(default_factory=set)


def _msg(obj) -> str:
    if isinstance(obj, Mapping):
        return str(obj.get("message") or obj.get("error") or json.dumps(obj))
    return str(obj)


def parse_codex_events(stdout: str) -> CodexEvents:
    """Fold `codex exec --json` JSONL events (format recorded 2026-10-04, codex-cli 0.159.2):
    thread.started{thread_id}, turn.started, item.started/completed{item: agent_message
    {text} | command_execution{command, aggregated_output, exit_code, status}},
    turn.completed{usage{input_tokens, cached_input_tokens, cache_write_input_tokens,
    output_tokens, reasoning_output_tokens}}, turn.failed{error{message}}, error{message}."""
    ev = CodexEvents()
    for line in stdout.splitlines():
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            obj = json.loads(line)
        except json.JSONDecodeError:
            continue
        t = obj.get("type")
        if t == "thread.started":
            ev.thread_id = obj.get("thread_id")
        elif t == "turn.completed":
            ev.turns_completed += 1
            u = obj.get("usage") or {}
            if u:
                ev.saw_usage = True
            ev.input_tokens += int(u.get("input_tokens", 0) or 0)
            ev.cached_input_tokens += int(u.get("cached_input_tokens", 0) or 0)
            ev.cache_write_input_tokens += int(u.get("cache_write_input_tokens", 0) or 0)
            ev.output_tokens += int(u.get("output_tokens", 0) or 0)
            ev.reasoning_output_tokens += int(u.get("reasoning_output_tokens", 0) or 0)
        elif t == "turn.failed":
            ev.failures.append(_msg(obj.get("error")))
        elif t == "error":
            ev.errors.append(_msg(obj))
        elif t in ("item.completed",):
            item = obj.get("item") or {}
            if item.get("type") in ("agent_message", "assistant_message") and item.get("text") is not None:
                ev.last_message = item["text"]
        elif t in ("turn.started", "item.started", "item.updated"):
            pass
        elif t:
            ev.unknown_types.add(t)
    return ev


def price_codex_usage(input_tokens: int, cached_input_tokens: int, output_tokens: int,
                      prices: Mapping | None, cache_write_input_tokens: int = 0) -> Usage:
    """Map Codex token counts onto `Usage` and price them.

    OpenAI's `input_tokens` INCLUDES the cached tokens, so `Usage.input_tokens` is
    the uncached remainder (matching Claude's meaning) and the cached part goes to
    `cache_read_tokens`. Reasoning tokens are already inside `output_tokens`.
    `cache_write_input_tokens` (new in 0.159; observed 0 so far) is recorded but
    its billing relation to `input_tokens` is undocumented, so a non-zero count
    is priced only via an explicit `cache_write_input` price, treated as a subset
    of `input_tokens`. Any token class with a count but no price ->
    `cost_usd=None`: cost is never guessed.
    """
    cached = max(0, cached_input_tokens)
    writes = max(0, cache_write_input_tokens)
    uncached = max(0, input_tokens - cached)
    counts = {"input": max(0, uncached - writes), "cached_input": cached,
              "cache_write_input": writes, "output": output_tokens}
    cost, derived = None, False
    if prices is not None and all(prices.get(k) is not None for k, n in counts.items() if n):
        cost = sum(n * float(prices[k]) / 1_000_000 for k, n in counts.items() if n)
        derived = True
    return Usage(input_tokens=uncached, output_tokens=output_tokens, cache_read_tokens=cached,
                 cache_write_tokens=writes, cost_usd=cost, cost_derived=derived)


def parse_codex_call(stdout: str, stderr: str, returncode: int | None, result_text: str | None,
                     *, prices: Mapping | None, timed_out: bool = False,
                     timeout_s: float | None = None) -> ParsedCall:
    ev = parse_codex_events(stdout)
    usage = (price_codex_usage(ev.input_tokens, ev.cached_input_tokens, ev.output_tokens, prices,
                               ev.cache_write_input_tokens)
             if ev.saw_usage else Usage())
    if timed_out:
        return ParsedCall("error", usage=usage, error=f"wall-clock cap: codex exceeded {timeout_s}s and was killed")
    failed = bool(ev.failures) or returncode not in (0, None) or not ev.turns_completed
    if failed:
        parts = ev.failures + ev.errors
        msg = "; ".join(parts) if parts else (tail(stderr) or f"codex exited {returncode} with no turn")
        kind = classify_failure(msg)
        return ParsedCall("quota_exhausted" if kind == "quota" else "error", usage=usage,
                          error=f"codex exited {returncode}: {msg}")
    result = parse_json_text(result_text) if result_text is not None else parse_json_text(ev.last_message)
    return ParsedCall("ok", result=result, usage=usage)


# --------------------------------------------------------------------------- worker


class CodexWorker(CliWorker):
    family = "codex"
    transcript_name = "events.jsonl"

    def __init__(
        self,
        config: LoopConfig | None = None,
        *,
        exe: str | None = None,
        runner=None,
        artifacts_root=None,
        extra_config: Sequence[str] = (),
        extra_args: Sequence[str] = (),
        timeout_s: float | None = DEFAULT_TIMEOUT_S,
        capture: bool = True,
    ):
        super().__init__(runner=runner, artifacts_root=artifacts_root, timeout_s=timeout_s, capture=capture)
        self.config = config or LoopConfig()
        self._exe = exe
        self.extra_config = tuple(extra_config)
        self.extra_args = tuple(extra_args)
        # Refuse a sandbox-weakening override at construction, before any spend.
        build_codex_argv(exe="codex", worktree=".", schema_path="s.json", result_path="r.json",
                         extra_config=self.extra_config, extra_args=self.extra_args)

    @property
    def exe(self) -> str:
        exe = self._exe or resolve_codex_exe()
        if not exe:
            raise FileNotFoundError(f"codex CLI not found (PATH, Store package, or set {ENV_EXE})")
        return exe

    def build_invocation(self, packet: TaskPacket, call_dir: Path) -> Invocation:
        models = self.config.models.get("codex", {})
        target = cargo_target_dir(packet.worktree, self.config.cargo_target_base)
        writable = [target]
        if self.config.sccache_dir:
            writable.append(self.config.sccache_dir)
        for d in writable:
            try:
                os.makedirs(d, exist_ok=True)
            except OSError:
                pass
        argv = build_codex_argv(
            exe=self.exe,
            worktree=packet.worktree,
            schema_path=os.path.abspath(packet.schema_path),
            result_path=call_dir / "last_message.json",
            writable_dirs=writable,
            model=packet.model or models.get("model"),
            effort=packet.effort or models.get("effort"),
            extra_config=self.extra_config,
            extra_args=self.extra_args,
        )
        env = worktree_env(packet.worktree, self.config.cargo_target_base, self.config.sccache_dir)
        return Invocation(argv=argv, stdin=render_prompt(packet), cwd=str(packet.worktree), env=env)

    def parse_call(self, proc: ProcResult, call_dir: Path, packet: TaskPacket) -> ParsedCall:
        result_file = call_dir / "last_message.json"
        result_text = result_file.read_text(encoding="utf-8") if result_file.is_file() else None
        return parse_codex_call(proc.stdout, proc.stderr, proc.returncode, result_text,
                                prices=self.config.prices.get("codex"),
                                timed_out=proc.timed_out, timeout_s=self.timeout_s)
