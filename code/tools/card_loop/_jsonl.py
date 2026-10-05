"""Append-only JSONL files shared by the attempt, correction and audit ledgers.

Contract (all three ledgers):
- one compact JSON object per line, UTF-8, LF line endings on every platform;
- appends only — rows are never rewritten in place;
- cross-branch safety comes from `merge=union` in `.gitattributes` (git
  concatenates both sides' new lines), in-process safety from a per-path lock;
- readers never raise on content: blank lines are skipped, a line that is not
  a JSON object is skipped and reported as a `LedgerProblem`, unknown keys are
  the caller's to keep or ignore.
"""
from __future__ import annotations

import json
import os
import threading
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path

# code/tools/card_loop/_jsonl.py -> parents[3] is the repository root.
REPO_ROOT = Path(__file__).resolve().parents[3]

_LOCKS: dict[str, threading.Lock] = {}
_LOCKS_GUARD = threading.Lock()


def resolve_repo_path(path: str | os.PathLike) -> Path:
    """Absolute paths pass through; relative ones anchor at the repo root.

    The ledgers are committed repo files; anchoring them at the root (not the
    CWD) keeps `python -m tools.card_loop` correct when run from `code/`.
    """
    p = Path(path)
    return p if p.is_absolute() else REPO_ROOT / p


def utc_now_iso() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.%fZ")


def parse_ts(value: object) -> datetime | None:
    """Parse a ledger timestamp; None (not an exception) for anything unusable."""
    if not isinstance(value, str) or not value:
        return None
    try:
        dt = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return None
    return dt if dt.tzinfo else dt.replace(tzinfo=timezone.utc)


def _lock_for(path: Path) -> threading.Lock:
    key = os.path.normcase(str(path.resolve()))
    with _LOCKS_GUARD:
        lock = _LOCKS.get(key)
        if lock is None:
            lock = _LOCKS[key] = threading.Lock()
        return lock


def dumps_row(row: dict) -> str:
    return json.dumps(row, ensure_ascii=False, separators=(",", ":"), allow_nan=False)


def append_row(path: str | os.PathLike, row: dict) -> None:
    """Append one row as a single LF-terminated line.

    Opened in binary mode so Windows never writes CRLF. If an earlier writer
    died mid-line (no trailing newline), a newline is written first so the
    damage stays confined to that one malformed line.
    """
    if not isinstance(row, dict):
        raise TypeError("a ledger row must be a dict")
    data = (dumps_row(row) + "\n").encode("utf-8")
    p = resolve_repo_path(path)
    p.parent.mkdir(parents=True, exist_ok=True)
    with _lock_for(p):
        with open(p, "ab+") as f:
            f.seek(0, os.SEEK_END)
            if f.tell() > 0:
                f.seek(-1, os.SEEK_END)
                if f.read(1) != b"\n":
                    data = b"\n" + data
            f.seek(0, os.SEEK_END)
            f.write(data)
            f.flush()


@dataclass(frozen=True)
class LedgerProblem:
    path: str
    line: int          # 1-based
    kind: str          # malformed_json | not_object | missing_keys | bad_value
    message: str


def read_rows(path: str | os.PathLike) -> tuple[list[tuple[int, dict]], list[LedgerProblem]]:
    """All JSON-object rows with their 1-based line numbers, plus problems.

    A missing file is an empty ledger, not an error.
    """
    p = resolve_repo_path(path)
    rows: list[tuple[int, dict]] = []
    problems: list[LedgerProblem] = []
    if not p.exists():
        return rows, problems
    with open(p, "rb") as f:
        raw = f.read()
    text = raw.decode("utf-8-sig", errors="replace")
    for n, line in enumerate(text.splitlines(), start=1):
        line = line.strip()
        if not line:
            continue
        try:
            obj = json.loads(line)
        except json.JSONDecodeError as e:
            problems.append(LedgerProblem(str(p), n, "malformed_json", str(e)))
            continue
        if not isinstance(obj, dict):
            problems.append(LedgerProblem(str(p), n, "not_object",
                                          f"expected an object, got {type(obj).__name__}"))
            continue
        rows.append((n, obj))
    return rows, problems
