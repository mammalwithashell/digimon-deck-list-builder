"""Worker contract, result validation, retry policy and artifact capture (design D10).

A worker turns one `TaskPacket` into one `WorkerResult`. Everything here is
vendor-agnostic; `claude.py` / `codex.py` only know how to build their CLI's
argv and read its output, and `fake.py` replays canned results for driver tests.

Retry classes (`run_with_retry`):
  * transient  (timeouts, 5xx, overloaded, rate limit / 429) -> exponential backoff
  * schema_invalid                                           -> one retry, with the
                                                                validation errors
                                                                appended to the prompt
  * quota / credit exhaustion                                -> `quota_exhausted`
                                                                immediately, no retry;
                                                                the vendor is disabled
                                                                in `VendorHealth`
  * anything else                                            -> returned as `error`
"""
from __future__ import annotations

import dataclasses
import hashlib
import json
import os
import re
import shutil
import signal
import subprocess
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Iterable, Mapping, Protocol, Sequence, runtime_checkable

from ..contracts import TaskPacket, Usage, WorkerResult

try:  # jsonschema 4.x is installed in the dev env but is not pinned in requirements*.txt
    import jsonschema as _jsonschema
except ImportError:  # pragma: no cover - exercised by forcing the fallback in tests
    _jsonschema = None

JSONSCHEMA_AVAILABLE = _jsonschema is not None


# --------------------------------------------------------------------------- protocol


@runtime_checkable
class Worker(Protocol):
    """Anything that can run a packet. `family` must equal `packet.family`."""

    family: str

    def run(self, packet: TaskPacket) -> WorkerResult: ...


# --------------------------------------------------------------------------- schemas

SCHEMAS_DIR = Path(__file__).resolve().parent.parent / "schemas"


def schema_path(stage: str) -> Path:
    """`schemas/<stage>.json` (classify_qa / encode_ruling are owned by group 5)."""
    return SCHEMAS_DIR / f"{stage}.json"


_SCHEMA_CACHE: dict[tuple[str, float], dict] = {}


def load_schema(path: str | os.PathLike) -> dict:
    p = Path(path)
    key = (str(p.resolve()), p.stat().st_mtime)
    cached = _SCHEMA_CACHE.get(key)
    if cached is None:
        cached = json.loads(p.read_text(encoding="utf-8"))
        _SCHEMA_CACHE[key] = cached
    return cached


def validate_result(result, schema: Mapping, *, use_jsonschema: bool | None = None) -> list[str]:
    """Return human-readable validation errors; an empty list means valid.

    Uses `jsonschema` (Draft 2020-12) when importable, else `_minimal_validate`,
    which covers the keyword subset the stage schemas use (type, enum, const,
    required, properties, additionalProperties, items, anyOf, min/maxItems,
    minLength).
    """
    if use_jsonschema is None:
        use_jsonschema = JSONSCHEMA_AVAILABLE
    if use_jsonschema:
        validator = _jsonschema.Draft202012Validator(schema)
        errors = sorted(validator.iter_errors(result), key=lambda e: list(e.absolute_path))
        return [f"{_fmt_path(e.absolute_path)}: {e.message}" for e in errors]
    return _minimal_validate(result, schema, ())


def _fmt_path(path: Iterable) -> str:
    parts = ["$"]
    for p in path:
        parts.append(f"[{p}]" if isinstance(p, int) else f".{p}")
    return "".join(parts)


_TYPE_CHECKS: dict[str, Callable[[object], bool]] = {
    "object": lambda v: isinstance(v, dict),
    "array": lambda v: isinstance(v, list),
    "string": lambda v: isinstance(v, str),
    "boolean": lambda v: isinstance(v, bool),
    "integer": lambda v: isinstance(v, int) and not isinstance(v, bool),
    "number": lambda v: isinstance(v, (int, float)) and not isinstance(v, bool),
    "null": lambda v: v is None,
}


def _minimal_validate(value, schema: Mapping, path: tuple) -> list[str]:
    where = _fmt_path(path)
    errors: list[str] = []
    if "anyOf" in schema:
        branches = [_minimal_validate(value, s, path) for s in schema["anyOf"]]
        if all(branches):
            errors.append(f"{where}: {value!r} is not valid under any of the given schemas")
        return errors
    types = schema.get("type")
    if types is not None:
        types = [types] if isinstance(types, str) else list(types)
        if not any(_TYPE_CHECKS[t](value) for t in types):
            return [f"{where}: {value!r} is not of type {', '.join(repr(t) for t in types)}"]
    if "const" in schema and value != schema["const"]:
        errors.append(f"{where}: {schema['const']!r} was expected")
    if "enum" in schema and value not in schema["enum"]:
        errors.append(f"{where}: {value!r} is not one of {schema['enum']!r}")
    if isinstance(value, str) and "minLength" in schema and len(value) < schema["minLength"]:
        errors.append(f"{where}: {value!r} is too short")
    if isinstance(value, dict):
        props = schema.get("properties", {})
        for name in schema.get("required", ()):
            if name not in value:
                errors.append(f"{where}: {name!r} is a required property")
        extra = schema.get("additionalProperties", True)
        for name, sub in value.items():
            if name in props:
                errors.extend(_minimal_validate(sub, props[name], path + (name,)))
            elif extra is False:
                errors.append(f"{where}: additional property {name!r} is not allowed")
            elif isinstance(extra, Mapping):
                errors.extend(_minimal_validate(sub, extra, path + (name,)))
    if isinstance(value, list):
        if "minItems" in schema and len(value) < schema["minItems"]:
            errors.append(f"{where}: {value!r} should have at least {schema['minItems']} items")
        if "maxItems" in schema and len(value) > schema["maxItems"]:
            errors.append(f"{where}: {value!r} should have at most {schema['maxItems']} items")
        if isinstance(schema.get("items"), Mapping):
            for i, item in enumerate(value):
                errors.extend(_minimal_validate(item, schema["items"], path + (i,)))
    return errors


def check_result(result, schema_file: str | os.PathLike) -> tuple[str, str | None]:
    """(`"ok"`, None) or (`"schema_invalid"`, error text) for a parsed result."""
    if result is None:
        return "schema_invalid", "no structured result was returned"
    errors = validate_result(result, load_schema(schema_file))
    if errors:
        return "schema_invalid", "; ".join(errors)
    return "ok", None


def parse_json_text(text: str | None):
    """Parse a model's final message as JSON, tolerating a ```json fence."""
    if text is None:
        return None
    s = text.strip()
    fence = re.fullmatch(r"```(?:json)?\s*(.*?)\s*```", s, flags=re.S)
    if fence:
        s = fence.group(1)
    try:
        return json.loads(s)
    except (json.JSONDecodeError, ValueError):
        return None


# --------------------------------------------------------------------------- prompts


def render_prompt(packet: TaskPacket) -> str:
    """The prompt a worker CLI receives: the rendered template plus a references block.

    References are listed, never inlined (design D1/D14: small packets).
    """
    text = packet.prompt.rstrip()
    if packet.references:
        refs = "\n".join(f"- {r}" for r in packet.references)
        text += (
            "\n\n## References\n"
            "Read these as needed; they are not inlined. Paths outside your working "
            "directory are read-only.\n" + refs
        )
    return text + "\n"


def with_validation_feedback(packet: TaskPacket, error: str) -> TaskPacket:
    """The one schema retry: same packet, the validation error appended."""
    note = (
        "\n\n## Your previous result was rejected\n"
        f"It did not validate against the result schema ({Path(packet.schema_path).name}):\n"
        f"{error}\n"
        "Any work you already did in the working directory is still there. "
        "Do not redo it; return a final result that conforms exactly to the schema.\n"
    )
    return dataclasses.replace(packet, prompt=packet.prompt.rstrip() + note)


# --------------------------------------------------------------------------- failure classes

# Quota / credit exhaustion: stop the vendor for the run. Lower-cased substrings.
# Claude Code: API "Credit balance is too low", subscription "Claude AI usage
# limit reached|<epoch>", "You've hit your limit · resets …", "5-hour limit
# reached", "weekly limit". OpenAI / Codex: "insufficient_quota", "You exceeded
# your current quota", "usage_limit_reached", "You've hit your usage limit".
QUOTA_PATTERNS: tuple[str, ...] = (
    "credit balance is too low",
    "credit balance too low",
    "insufficient credit",
    "insufficient_quota",
    "exceeded your current quota",
    "quota exceeded",
    "quota_exceeded",
    "usage limit",
    "usage_limit_reached",
    "usage_limit_exceeded",
    "you've hit your limit",
    "you have hit your limit",
    "hit your usage limit",
    "5-hour limit reached",
    "weekly limit reached",
    "billing_hard_limit_reached",
    "out of credits",
)

# Transient: back off and retry. Checked AFTER quota (a 429 with
# "insufficient_quota" is quota, not a rate limit).
TRANSIENT_PATTERNS: tuple[str, ...] = (
    "timed out",
    "timeout",
    "rate limit",
    "rate_limit",
    "ratelimit",
    "too many requests",
    "overloaded",
    "internal server error",
    "internal_server_error",
    "server_error",
    "api_error",
    "bad gateway",
    "service unavailable",
    "gateway timeout",
    "connection reset",
    "connection refused",
    "connection error",
    "econnreset",
    "etimedout",
    "socket hang up",
    "stream disconnected",
    "temporarily unavailable",
)
# HTTP status codes as whole words only ("BT7-0502" or "15029 tokens" must not match).
TRANSIENT_STATUS_RE = re.compile(r"(?<![\w-])(?:429|500|502|503|504|529)(?![\w-])")

# Authentication: as permanent for the run as quota, but reported truthfully as
# `error` (it is not a quota). Recorded 2026-10-04 from claude 2.1.195 with no
# login: result "Not logged in · Please run /login".
AUTH_PATTERNS: tuple[str, ...] = (
    "not logged in",
    "please run /login",
    "invalid api key",
    "invalid x-api-key",
    "authentication_error",
    "oauth token has expired",
    "token has been revoked",
    "please run `codex login`",
    "codex login",
    "not authenticated",
)


def classify_failure(text: str | None) -> str | None:
    """`"quota"`, `"auth"`, `"transient"` or None (permanent / unknown) for an error text."""
    if not text:
        return None
    t = text.lower()
    if any(p in t for p in QUOTA_PATTERNS):
        return "quota"
    if any(p in t for p in AUTH_PATTERNS):
        return "auth"
    if any(p in t for p in TRANSIENT_PATTERNS) or TRANSIENT_STATUS_RE.search(t):
        return "transient"
    return None


# --------------------------------------------------------------------------- usage


def add_usage(a: Usage, b: Usage) -> Usage:
    """Sum two calls' usage (an attempt's retries are billed to the attempt).

    Cost: None only when neither call reported one; otherwise the sum of the known
    costs (a call that died before reporting contributes 0 — its error is kept on
    the result).
    """
    if a.cost_usd is None and b.cost_usd is None:
        cost = None
    else:
        cost = (a.cost_usd or 0.0) + (b.cost_usd or 0.0)
    return Usage(
        input_tokens=a.input_tokens + b.input_tokens,
        output_tokens=a.output_tokens + b.output_tokens,
        cache_read_tokens=a.cache_read_tokens + b.cache_read_tokens,
        cache_write_tokens=a.cache_write_tokens + b.cache_write_tokens,
        cost_usd=cost,
        cost_derived=a.cost_derived or b.cost_derived,
        wall_seconds=a.wall_seconds + b.wall_seconds,
    )


# --------------------------------------------------------------------------- retry


def run_with_retry(
    worker: Worker,
    packet: TaskPacket,
    *,
    max_transient: int = 3,
    base_delay: float = 2.0,
    max_delay: float = 120.0,
    sleep: Callable[[float], None] = time.sleep,
    health=None,
    validate: bool = True,
) -> WorkerResult:
    """Run `packet` on `worker`, retrying by failure class (design D10, spec "Failures
    are retried by class").

    * transient errors: up to `max_transient` retries, delays base·2^n capped at max_delay;
    * schema_invalid: exactly one retry with the validation error appended;
    * quota_exhausted (reported by the adapter or detected in an error text):
      returned at once; `health.disable(family, reason)` when `health` is given;
    * an authentication error: returned at once as `error`, and the family is
      disabled in `health` too (every later call would fail the same way);
    * a family already disabled in `health` is not called at all.

    Usage is summed over every call made for the attempt.
    """
    if health is not None and not health.is_available(packet.family):
        return WorkerResult(
            status="quota_exhausted",
            error=f"{packet.family} disabled for this run: {health.reason(packet.family)}",
        )
    total = Usage()
    transient_retries = 0
    schema_retried = False
    current = packet
    notes: list[str] = []
    while True:
        res = worker.run(current)
        total = add_usage(total, res.usage)
        status, error = res.status, res.error
        if status == "ok" and validate:
            status, error = check_result(res.result, current.schema_path)
        if status == "error" and classify_failure(error) == "quota":
            status = "quota_exhausted"
        res = dataclasses.replace(res, status=status, error=error, usage=total)

        if status == "ok":
            return _with_notes(res, notes)
        if status == "quota_exhausted":
            if health is not None:
                health.disable(packet.family, error or "quota exhausted")
            return _with_notes(res, notes)
        if status == "error" and classify_failure(error) == "auth":
            if health is not None:
                health.disable(packet.family, f"auth: {error}")
            return _with_notes(res, notes)
        if status == "schema_invalid":
            if schema_retried:
                return _with_notes(res, notes)
            schema_retried = True
            notes.append(f"schema retry after: {error}")
            current = with_validation_feedback(packet, error or "result did not validate")
            continue
        if status == "error" and classify_failure(error) == "transient" and transient_retries < max_transient:
            delay = min(max_delay, base_delay * (2 ** transient_retries))
            transient_retries += 1
            notes.append(f"transient retry {transient_retries} after {delay:.1f}s: {error}")
            sleep(delay)
            continue
        return _with_notes(res, notes)


def _with_notes(res: WorkerResult, notes: list[str]) -> WorkerResult:
    if not notes or res.status == "ok":
        return res
    return dataclasses.replace(res, error=(res.error or "") + " | " + " | ".join(notes))


# --------------------------------------------------------------------------- processes


@dataclass(frozen=True)
class ProcResult:
    returncode: int | None
    stdout: str
    stderr: str
    wall_seconds: float
    timed_out: bool = False


Runner = Callable[..., ProcResult]


def run_process(
    argv: Sequence[str],
    *,
    stdin_text: str,
    cwd: str | os.PathLike,
    env: Mapping[str, str],
    timeout: float | None,
) -> ProcResult:
    """Run a worker CLI, feeding the prompt on stdin; kill the whole tree on timeout."""
    kwargs = {}
    if os.name == "nt":
        kwargs["creationflags"] = subprocess.CREATE_NEW_PROCESS_GROUP
    else:
        kwargs["start_new_session"] = True
    t0 = time.monotonic()
    proc = subprocess.Popen(
        list(argv),
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        cwd=str(cwd),
        env=dict(env),
        **kwargs,
    )
    timed_out = False
    try:
        out, err = proc.communicate(stdin_text.encode("utf-8"), timeout=timeout)
    except subprocess.TimeoutExpired:
        timed_out = True
        _kill_tree(proc)
        out, err = proc.communicate()
    return ProcResult(
        returncode=proc.returncode,
        stdout=out.decode("utf-8", "replace"),
        stderr=err.decode("utf-8", "replace"),
        wall_seconds=time.monotonic() - t0,
        timed_out=timed_out,
    )


def _kill_tree(proc: subprocess.Popen) -> None:
    if os.name == "nt":
        subprocess.run(["taskkill", "/PID", str(proc.pid), "/T", "/F"],
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False)
    else:  # pragma: no cover - POSIX only
        try:
            os.killpg(proc.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
    try:
        proc.kill()
    except OSError:
        pass


def probe_version(exe: str, timeout: float = 30.0) -> str | None:
    """`<exe> --version` first line, or None (for preflight's "CLI runs" check)."""
    try:
        cp = subprocess.run([exe, "--version"], capture_output=True, text=True,
                            timeout=timeout, check=False)
    except (OSError, subprocess.TimeoutExpired):
        return None
    if cp.returncode != 0:
        return None
    line = (cp.stdout or cp.stderr).strip().splitlines()
    return line[0] if line else None


# Environment variables that couple a child process to the *calling* Claude Code
# session (desktop host messaging socket + token, session ids, child-session
# flags). A worker is an independent, stateless process (D1), so they are
# dropped. Auth / provider / config-location variables are kept.
_CLAUDE_ENV_KEEP = frozenset({
    "CLAUDE_CONFIG_DIR",
    "CLAUDE_CODE_OAUTH_TOKEN",
    "CLAUDE_CODE_GIT_BASH_PATH",
    "CLAUDE_CODE_USE_BEDROCK",
    "CLAUDE_CODE_USE_VERTEX",
    "CLAUDE_CODE_USE_FOUNDRY",
    "CLAUDE_CODE_MAX_OUTPUT_TOKENS",
})


def scrubbed_env(base: Mapping[str, str] | None = None) -> dict[str, str]:
    """`base` (default os.environ) minus parent-session CLAUDE* variables."""
    env = dict(os.environ if base is None else base)
    for key in list(env):
        up = key.upper()
        if (up == "CLAUDECODE" or up.startswith("CLAUDE_")) and up not in _CLAUDE_ENV_KEEP:
            del env[key]
    return env


# --------------------------------------------------------------------------- attempt dirs


def attempt_call_dir(root: str | os.PathLike, packet: TaskPacket) -> Path:
    """`<root>/<attempt_id>/call-<n>`: one directory per CLI call of an attempt."""
    safe = re.sub(r"[^A-Za-z0-9._-]+", "_", packet.attempt_id) or "attempt"
    adir = Path(root) / safe
    adir.mkdir(parents=True, exist_ok=True)
    n = sum(1 for p in adir.iterdir() if p.is_dir() and p.name.startswith("call-"))
    cdir = adir / f"call-{n + 1}"
    cdir.mkdir()
    return cdir


def default_artifacts_root() -> Path:
    """Fallback when the driver does not pass `<run_dir>/attempts`."""
    return Path(tempfile.gettempdir()) / "card-loop" / "attempts"


# --------------------------------------------------------------------------- git helpers


def _git(args: Sequence[str], cwd: str | os.PathLike, *, env: Mapping[str, str] | None = None,
         text: bool = True, check: bool = True):
    cp = subprocess.run(["git", *args], cwd=str(cwd), capture_output=True, text=text,
                        env=None if env is None else dict(env), check=False)
    if check and cp.returncode != 0:
        err = cp.stderr if text else cp.stderr.decode("utf-8", "replace")
        raise RuntimeError(f"git {' '.join(args)} failed in {cwd}: {err.strip()}")
    return cp


def git_head(worktree: str | os.PathLike) -> str | None:
    """HEAD sha of `worktree`, or None when it is not a git checkout."""
    try:
        cp = _git(["rev-parse", "HEAD"], worktree, check=False)
    except OSError:  # missing directory
        return None
    return cp.stdout.strip() if cp.returncode == 0 else None


def base_reference_dirs(worktree: str | os.PathLike) -> list[str]:
    """Read-only reference dirs that live only in the BASE repo (CLAUDE.md rules 29, 32):
    the populated DCGO checkout and the rules PDFs. Missing ones are skipped."""
    cp = _git(["rev-parse", "--path-format=absolute", "--git-common-dir"], worktree, check=False)
    if cp.returncode != 0:
        return []
    base = Path(cp.stdout.strip()).parent
    out = []
    for rel in ("DCGO", "Digimon TCG resources"):
        p = base / rel
        if p.is_dir() and any(p.iterdir()):
            out.append(str(p))
    return out


# --------------------------------------------------------------------------- artifacts


def capture_artifacts(worktree: str | os.PathLike, base_sha: str, out_dir: str | os.PathLike) -> dict:
    """Snapshot the worktree's changes vs `base_sha` (harden-card-authoring-pipeline D1).

    Writes `<out_dir>/worktree.diff` — a binary git diff of the full working tree
    (committed, staged, unstaged and new untracked files; .gitignored files are
    excluded) against `base_sha` — and `<out_dir>/manifest.json`
    `{base_sha, files: [{path, status, sha256}]}` (`status` is git's A/M/D/T;
    `sha256` is of the working-tree bytes, None for deletions and gitlinks).

    The worktree's own index is not touched: a copy of it is staged with
    `git add -A` under `GIT_INDEX_FILE`.
    """
    wt = Path(worktree)
    out = Path(out_dir)
    out.mkdir(parents=True, exist_ok=True)
    real_index = _git(["rev-parse", "--git-path", "index"], wt).stdout.strip()
    real_index_path = Path(real_index) if os.path.isabs(real_index) else wt / real_index
    with tempfile.TemporaryDirectory(prefix="card-loop-index-") as td:
        tmp_index = Path(td) / "index"
        if real_index_path.exists():
            shutil.copyfile(real_index_path, tmp_index)
        env = {**os.environ, "GIT_INDEX_FILE": str(tmp_index)}
        _git(["add", "-A"], wt, env=env)
        diff = _git(["diff", "--cached", "--binary", "--no-renames", base_sha, "--"],
                    wt, env=env, text=False).stdout
        raw = _git(["diff", "--cached", "--raw", "-z", "--no-renames", "--no-abbrev",
                    base_sha, "--"], wt, env=env, text=False).stdout
    files = []
    for path, status, new_mode in _parse_raw_z(raw):
        sha = None
        fp = wt / path
        if status != "D" and new_mode != "160000" and fp.is_file():
            sha = hashlib.sha256(fp.read_bytes()).hexdigest()
        files.append({"path": path, "status": status, "sha256": sha})
    files.sort(key=lambda f: f["path"])
    diff_path = out / "worktree.diff"
    diff_path.write_bytes(diff)
    manifest_path = out / "manifest.json"
    manifest_path.write_text(json.dumps({"base_sha": base_sha, "files": files}, indent=2) + "\n",
                             encoding="utf-8")
    return {"diff": str(diff_path), "manifest": str(manifest_path)}


def _parse_raw_z(raw: bytes) -> list[tuple[str, str, str]]:
    """Parse `git diff --raw -z --no-renames` into (path, status letter, new mode)."""
    parts = raw.split(b"\0")
    out = []
    i = 0
    while i < len(parts):
        head = parts[i]
        if not head.startswith(b":"):
            i += 1
            continue
        fields = head[1:].decode("utf-8").split()
        new_mode, status = fields[1], fields[4][:1]
        path = parts[i + 1].decode("utf-8")
        out.append((path, status, new_mode))
        i += 2
    return out


# --------------------------------------------------------------------------- CLI worker skeleton


@dataclass(frozen=True)
class Invocation:
    argv: list[str]
    stdin: str
    cwd: str
    env: dict[str, str]


@dataclass(frozen=True)
class ParsedCall:
    """What an adapter read from one CLI call, before schema validation."""

    status: str                # ok | error | quota_exhausted
    result: object = None      # parsed structured result (status ok)
    usage: Usage = Usage()
    error: str | None = None


class CliWorker:
    """Shared run() for the subprocess adapters: one call directory per CLI call,
    transcript files, schema validation, artifact capture. Subclasses implement
    `build_invocation` and `parse_call` (pure functions of their inputs)."""

    family: str = ""
    transcript_name: str = "stdout.txt"

    def __init__(self, *, runner: Runner | None = None, artifacts_root: str | os.PathLike | None = None,
                 timeout_s: float | None = 3600.0, capture: bool = True):
        self.runner = runner or run_process
        self.artifacts_root = Path(artifacts_root) if artifacts_root else default_artifacts_root()
        self.timeout_s = timeout_s
        self.capture = capture

    def build_invocation(self, packet: TaskPacket, call_dir: Path) -> Invocation:  # pragma: no cover
        raise NotImplementedError

    def parse_call(self, proc: ProcResult, call_dir: Path, packet: TaskPacket) -> ParsedCall:  # pragma: no cover
        raise NotImplementedError

    def run(self, packet: TaskPacket) -> WorkerResult:
        if packet.family != self.family:
            raise ValueError(f"packet for {packet.family!r} sent to the {self.family!r} worker")
        call_dir = attempt_call_dir(self.artifacts_root, packet)
        base_sha = git_head(packet.worktree) if self.capture else None
        # A policy refusal (ValueError) propagates; a CLI that cannot be found or
        # launched is an `error` result the driver can route around.
        try:
            inv = self.build_invocation(packet, call_dir)
            (call_dir / "argv.json").write_text(json.dumps(_redact_argv(inv.argv), indent=2) + "\n",
                                                encoding="utf-8")
            (call_dir / "prompt.md").write_text(inv.stdin, encoding="utf-8")
            proc = self.runner(inv.argv, stdin_text=inv.stdin, cwd=inv.cwd, env=inv.env,
                               timeout=self.timeout_s)
        except OSError as e:
            return WorkerResult(status="error", error=f"{self.family} CLI could not be launched: {e}")
        transcript = call_dir / self.transcript_name
        transcript.write_text(proc.stdout, encoding="utf-8")
        if proc.stderr:
            (call_dir / "stderr.txt").write_text(proc.stderr, encoding="utf-8")
        parsed = self.parse_call(proc, call_dir, packet)
        usage = dataclasses.replace(parsed.usage, wall_seconds=proc.wall_seconds)
        status, result, error = parsed.status, parsed.result, parsed.error
        if status == "ok":
            status, error = check_result(result, packet.schema_path)
            (call_dir / "result.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
        artifacts: dict = {}
        if base_sha:
            try:
                artifacts = capture_artifacts(packet.worktree, base_sha, call_dir)
            except (RuntimeError, OSError) as e:
                error = f"{error + ' | ' if error else ''}artifact capture failed: {e}"
                if status == "ok":
                    status = "error"
        return WorkerResult(status=status, result=result if isinstance(result, dict) else None,
                            artifacts=artifacts, usage=usage, error=error,
                            transcript_path=str(transcript))


def _redact_argv(argv: Sequence[str]) -> list[str]:
    """argv as recorded in the call dir (inline JSON schemas shortened)."""
    return [a if len(a) <= 400 else a[:200] + f"...<{len(a)} chars>" for a in argv]


def tail(text: str | None, n: int = 2000) -> str:
    text = (text or "").strip()
    return text if len(text) <= n else "..." + text[-n:]
