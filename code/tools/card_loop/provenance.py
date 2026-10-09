"""Artifact provenance: which attempt produced a record, a commit, a line.

Two carriers (design D11; spec "Worker artifacts carry provenance"):

- **Records** a worker produces carry `produced_by: <attempt_id>` — a verdict or
  triage row at the top level, a scenario under `meta.produced_by`, a YAML card
  spec as a header comment (`# produced_by: <attempt_id>`).
- **Commits** the driver makes carry the trailers
  `Loop-Attempt: <attempt_id>` and `Loop-Model: <family>/<model>`.

`blame_attempts` closes the loop: lines -> `git blame --porcelain` -> commit ->
`Loop-Attempt` trailer. That is how a later edit is attributed to the attempt
whose content it changed (see `corrections.detect_late_edits`).
"""
from __future__ import annotations

import copy
import os
import re
import subprocess
from dataclasses import dataclass
from typing import Iterable, Mapping

PRODUCED_BY = "produced_by"
TRAILER_ATTEMPT = "Loop-Attempt"
TRAILER_MODEL = "Loop-Model"

_ZERO_SHA = re.compile(r"^0{40}$|^0{64}$")
_YAML_HEADER = re.compile(r"^#\s*produced_by:\s*(\S+)")


# ----------------------------------------------------------------------------
# records


def stamp(record: Mapping, attempt_id: str, *, path: tuple[str, ...] = (PRODUCED_BY,)) -> dict:
    """A deep copy of `record` with `attempt_id` at `path` (default top-level).

    `stamp(scenario, aid, path=("meta", "produced_by"))` for scenarios. A
    re-stamp overwrites: the newest producer owns the record; history lives in
    git and in the correction log.
    """
    if not attempt_id or any(c.isspace() for c in attempt_id):
        raise ValueError(f"bad attempt id {attempt_id!r}")
    if not path:
        raise ValueError("empty stamp path")
    out = copy.deepcopy(dict(record))
    node = out
    for key in path[:-1]:
        child = node.get(key)
        if child is None:
            child = node[key] = {}
        elif not isinstance(child, dict):
            raise ValueError(f"cannot stamp under non-mapping {key!r}")
        node = child
    node[path[-1]] = attempt_id
    return out


def produced_by(record: Mapping, *, path: tuple[str, ...] = (PRODUCED_BY,)) -> str | None:
    node: object = record
    for key in path:
        if not isinstance(node, Mapping) or key not in node:
            return None
        node = node[key]
    return node if isinstance(node, str) and node else None


def yaml_header(attempt_id: str, family: str | None = None, model: str | None = None) -> str:
    who = f" ({family}/{model or 'default'})" if family else ""
    return f"# {PRODUCED_BY}: {attempt_id}{who}"


def stamp_yaml_text(text: str, attempt_id: str, family: str | None = None,
                    model: str | None = None) -> str:
    """Put (or replace) the `# produced_by:` header comment on a YAML document.

    The header is the first line; an existing header is replaced, never
    duplicated. Line endings of the rest of the document are preserved.
    """
    header = yaml_header(attempt_id, family, model)
    newline = "\r\n" if "\r\n" in text else "\n"
    lines = text.splitlines(keepends=True)
    if lines and _YAML_HEADER.match(lines[0]):
        lines = lines[1:]
    return header + newline + "".join(lines)


def yaml_produced_by(text: str) -> str | None:
    first = text.splitlines()[0] if text else ""
    m = _YAML_HEADER.match(first)
    return m.group(1) if m else None


# ----------------------------------------------------------------------------
# commit trailers


def model_label(family: str, model: str | None) -> str:
    return f"{family}/{model or 'default'}"


def commit_trailers(attempt_id: str, family: str, model: str | None) -> list[str]:
    if not attempt_id or any(c.isspace() for c in attempt_id):
        raise ValueError(f"bad attempt id {attempt_id!r}")
    return [f"{TRAILER_ATTEMPT}: {attempt_id}", f"{TRAILER_MODEL}: {model_label(family, model)}"]


_TRAILER_LINE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9-]*:\s")


def add_trailers(message: str, trailers: Iterable[str]) -> str:
    """Append trailers to a commit message the way `git interpret-trailers` would.

    If the message already ends in a trailer block (e.g. `Co-Authored-By:`),
    the new trailers join that block; otherwise a blank line separates them
    from the body. Exact duplicate trailer lines are not repeated.
    """
    trailers = [t.strip() for t in trailers if t and t.strip()]
    body = message.rstrip("\n").replace("\r\n", "\n")
    paragraphs = body.split("\n\n")
    last = paragraphs[-1].splitlines() if body else []
    in_block = len(paragraphs) > 1 and bool(last) and all(_TRAILER_LINE.match(l) for l in last)
    existing = set(last) if in_block else set()
    new = [t for t in trailers if t not in existing]
    if not new:
        return body + "\n"
    sep = "\n" if in_block else "\n\n"
    return body + sep + "\n".join(new) + "\n"


def loop_commit_message(subject: str, body: str | None, *, attempt_id: str, family: str,
                        model: str | None, extra_trailers: Iterable[str] = ()) -> str:
    """A driver commit message: subject, optional body, Loop-* trailers last."""
    msg = subject.strip()
    if body and body.strip():
        msg += "\n\n" + body.strip()
    return add_trailers(msg, [*commit_trailers(attempt_id, family, model), *extra_trailers])


class GitError(RuntimeError):
    pass


def _git(repo: str | os.PathLike, *args: str) -> str:
    proc = subprocess.run(["git", "-C", str(repo), *args], capture_output=True,
                          text=True, encoding="utf-8", errors="replace")
    if proc.returncode != 0:
        raise GitError(f"git {' '.join(args)} failed ({proc.returncode}): {proc.stderr.strip()}")
    return proc.stdout


def commit_attempts(repo: str | os.PathLike, sha: str) -> tuple[str, ...]:
    """Every `Loop-Attempt` trailer value on commit `sha`, in order."""
    out = _git(repo, "log", "-1", f"--format=%(trailers:key={TRAILER_ATTEMPT},valueonly)", sha)
    return tuple(v.strip() for v in out.splitlines() if v.strip())


# ----------------------------------------------------------------------------
# blame -> attempt


@dataclass(frozen=True)
class BlamedLine:
    line: int                    # final line number at `rev`, 1-based
    commit: str | None           # None for not-yet-committed lines
    attempts: tuple[str, ...]    # Loop-Attempt trailers on that commit


def _normalise_ranges(line_ranges) -> list[tuple[int, int]]:
    out = []
    for r in line_ranges:
        if isinstance(r, int):
            start = end = r
        else:
            start, end = r
        if start < 1 or end < start:
            raise ValueError(f"bad line range {r!r}")
        out.append((int(start), int(end)))
    return out


def _parse_porcelain(text: str) -> dict[int, str]:
    """final line number -> commit sha, from `git blame --porcelain` output."""
    line_to_sha: dict[int, str] = {}
    header = re.compile(r"^([0-9a-f]{40}|[0-9a-f]{64}) (\d+) (\d+)(?: (\d+))?$")
    for raw in text.splitlines():
        if raw.startswith("\t"):
            continue
        m = header.match(raw)
        if m:
            line_to_sha[int(m.group(3))] = m.group(1)
    return line_to_sha


def blame_lines(repo: str | os.PathLike, path: str | os.PathLike, line_ranges,
                rev: str | None = "HEAD") -> list[BlamedLine]:
    """Blame the given lines at `rev` and attach each commit's Loop-Attempt trailers.

    `line_ranges` holds 1-based inclusive `(start, end)` pairs or single ints.
    `rev=None` blames the working tree (uncommitted lines get `commit=None`).
    """
    ranges = _normalise_ranges(line_ranges)
    if not ranges:
        return []
    args = ["blame", "--porcelain"]
    for s, e in ranges:
        args += ["-L", f"{s},{e}"]
    if rev:
        args.append(rev)
    args += ["--", str(path)]
    line_to_sha = _parse_porcelain(_git(repo, *args))
    cache: dict[str, tuple[str, ...]] = {}
    out = []
    for line in sorted(line_to_sha):
        sha = line_to_sha[line]
        if _ZERO_SHA.match(sha):
            out.append(BlamedLine(line, None, ()))
            continue
        if sha not in cache:
            cache[sha] = commit_attempts(repo, sha)
        out.append(BlamedLine(line, sha, cache[sha]))
    return out


def blame_attempts(repo: str | os.PathLike, path: str | os.PathLike, line_ranges,
                   rev: str | None = "HEAD") -> dict[int, str | None]:
    """`{line: attempt_id | None}` for the given lines at `rev`.

    None when the line's commit carries no `Loop-Attempt` trailer (a human or
    pre-loop commit), is uncommitted, or carries several (ambiguous: never
    guess an owner — use `blame_lines` to see them all).
    """
    return {b.line: (b.attempts[0] if len(b.attempts) == 1 else None)
            for b in blame_lines(repo, path, line_ranges, rev)}
