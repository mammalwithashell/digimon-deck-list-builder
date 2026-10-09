"""The gap lane (design D4 PARKED, D13; spec "Gaps are a ranked lane that parks
and unparks cards"; task 6.6).

When implementation hits a DSL-vocabulary or engine gap, the driver parks the
item on the gap id. `RunGapLane` is the run's record of that:

    runs/card-loop/<run-id>/gaps.json
    {"version": 1,
     "gaps": [{"gap_id", "items": [...], "core_items": [...], "is_core",
               "parked_at", "branch"?, "branch_sha"?, "kind"?}],
     "unparked": [{...row, "unparked_at"}]}

`items` are item ids (`card:BT7-056`, `clause:BT7-056#effect#0`, ...);
`core_items` the subset parked with `is_core=True` (the row's `is_core` is
true when any is). `ranked()` orders open gaps by the number of CORE clauses
they block -- a card counts its clause entries in the plan (`plan["exam"]`,
the `clause_coverage.campaign` entry shape `{card_id, clause_id, ...}`;
`plan["campaign"]["exam"]` and `plan["exams"]` are read too) or 1 when the
plan has none; a clause or interaction item counts 1 -- with ties broken by
the number of items parked, then the earliest park, then the id.

A gap is `closed(ctx)` when its fix has landed on the run branch: the engine
branch the merger noted (`note_branch`) has commits beyond the run base and
its tip (or, if the branch is gone, the noted sha) is an ancestor of the run
tree's HEAD -- or its tracker entry is marked closed (`RESOLVED` / `FIXED` /
`CLOSED` in the entry's heading, or a `Status: RESOLVED|Closed|Fixed` line).
A squash-merged branch is not an ancestor; the tracker covers that case.
`unpark(gap_id)` returns the parked items and moves the row to `unparked`.

Tracker writes are orchestrator-only (harden-card-authoring-pipeline tracker
hygiene): workers report gaps in their results, and only `record_gap` writes
`qa/dsl-vocab-gaps.md` / `docs/RUST_ENGINE_GAPS.md`, appending one entry in the
trackers' existing shape and committing it on the run branch (never on main):

    ## <summary>  [<gap-id>] — OPEN (<date>, card-loop run `<run>`)
    <!-- card-loop-gap:<gap-id> -->

    Surfaced: <date>, card-loop run `<run>`, attempt `<attempt>`.

    - **Gap (dsl|engine):** <summary>
    - **Blocks:** <items> (<n> core)
    - **Close:** ...

It is idempotent: an id already present anywhere in the tracker is not added.
"""
from __future__ import annotations

import json
import os
import re
import subprocess
from pathlib import Path
from typing import Any, Callable, Iterable, Mapping, Sequence

from ._jsonl import utc_now_iso
from .contracts import split_item_id
from .provenance import loop_commit_message

DSL_TRACKER = "qa/dsl-vocab-gaps.md"
ENGINE_TRACKER = "docs/RUST_ENGINE_GAPS.md"
TRACKERS = {"dsl": DSL_TRACKER, "engine": ENGINE_TRACKER}
GAPS_FILE = "gaps.json"
PROTECTED_BRANCHES = ("main", "master")

_HEADING = re.compile(r"^#{1,6}\s")
_CLOSED_TITLE = re.compile(r"\b(?:RESOLVED|FIXED|CLOSED)\b|✅")
_STATUS_CLOSED = re.compile(r"Status:\**\s*\**\s*(?:RESOLVED|Resolved|CLOSED|Closed|FIXED|Fixed)\b")


def _token(gap_id: str) -> re.Pattern:
    return re.compile(r"(?<![\w-])" + re.escape(gap_id) + r"(?![\w-])")


# ---------------------------------------------------------------------------
# plan -> clauses per card
# ---------------------------------------------------------------------------


def clause_counts_from_plan(plan: Mapping | None) -> dict[str, list[str]]:
    """card id -> its clause ids, from the plan's exam entries."""
    plan = plan or {}
    sources: list[Any] = [plan.get("exam"), (plan.get("campaign") or {}).get("exam"),
                          plan.get("exams")]
    out: dict[str, list[str]] = {}

    def add(card: str, clause: str) -> None:
        lst = out.setdefault(card, [])
        if clause not in lst:
            lst.append(clause)

    for src in sources:
        if isinstance(src, Mapping):
            for card, val in src.items():
                if isinstance(val, int) and not isinstance(val, bool):
                    for i in range(1, val + 1):
                        add(card, f"{card}#{i}")
                elif isinstance(val, (list, tuple)):
                    for v in val:
                        add(card, v if isinstance(v, str) else
                            str((v or {}).get("clause_id") or (v or {}).get("clause")))
        elif isinstance(src, (list, tuple)):
            for e in src:
                if not isinstance(e, Mapping):
                    continue
                card = e.get("card_id") or e.get("card")
                clause = e.get("clause_id") or e.get("clause") or e.get("id")
                if card and clause:
                    add(str(card), str(clause))
    return {c: sorted(v) for c, v in out.items()}


# ---------------------------------------------------------------------------
# tracker parsing
# ---------------------------------------------------------------------------


def _entries(text: str) -> list[tuple[str, str, list[str]]]:
    """(kind, title, body lines): one per heading, one per blockquote paragraph."""
    entries: list[tuple[str, str, list[str]]] = []
    heading: tuple[str, str, list[str]] | None = None
    quote: list[str] | None = None
    for line in text.splitlines():
        if line.startswith(">"):
            if quote is None:
                quote = []
                entries.append(("quote", line, quote))
            quote.append(line)
            continue
        quote = None
        if _HEADING.match(line):
            heading = ("heading", line, [])
            entries.append(heading)
        elif heading is not None:
            heading[2].append(line)
    return entries


def tracker_status(text: str, gap_id: str) -> str | None:
    """"closed", "open", or None when the tracker has no entry for `gap_id`.
    An entry is a heading naming the id, or a blockquote paragraph mentioning
    it (body cross-references do not count). Any closed entry closes it."""
    tok = _token(gap_id)
    found: list[bool] = []
    for kind, title, body in _entries(text):
        if kind == "heading" and tok.search(title):
            found.append(bool(_CLOSED_TITLE.search(title))
                         or any(_STATUS_CLOSED.search(l) for l in body))
        elif kind == "quote" and any(tok.search(l) for l in body):
            found.append(bool(_CLOSED_TITLE.search(title)))
    if not found:
        return None
    return "closed" if any(found) else "open"


# ---------------------------------------------------------------------------
# git helpers
# ---------------------------------------------------------------------------


def _git(repo: str | os.PathLike, *args: str, check: bool = False) -> subprocess.CompletedProcess:
    cp = subprocess.run(["git", *args], cwd=str(repo), capture_output=True, text=True,
                        encoding="utf-8", errors="replace")
    if check and cp.returncode != 0:
        raise RuntimeError(f"git {' '.join(args)} failed in {repo}: {cp.stderr.strip()}")
    return cp


def _rev(repo: str, ref: str) -> str | None:
    cp = _git(repo, "rev-parse", "--verify", "-q", f"{ref}^{{commit}}")
    return cp.stdout.strip() if cp.returncode == 0 and cp.stdout.strip() else None


# ---------------------------------------------------------------------------
# the lane
# ---------------------------------------------------------------------------


class RunGapLane:
    """`GapLane` for one run (see module docstring). Not thread-safe: the
    driver calls it from its scheduling thread."""

    def __init__(self, run_dir: str | os.PathLike, plan: Mapping | None, *,
                 repo: str | os.PathLike | None = None, now: Callable[[], str] | None = None,
                 clause_counts: Mapping[str, Sequence[str]] | None = None):
        self.run_dir = Path(run_dir)
        self.path = self.run_dir / GAPS_FILE
        self.plan = plan or {}
        self.now = now or utc_now_iso
        self._repo = Path(repo) if repo else None
        self.clauses = {k: list(v) for k, v in (clause_counts if clause_counts is not None
                                                 else clause_counts_from_plan(self.plan)).items()}

    # ---------------------------------------------------------------- storage

    def _load(self) -> dict:
        if not self.path.is_file():
            return {"version": 1, "gaps": [], "unparked": []}
        doc = json.loads(self.path.read_text(encoding="utf-8"))
        doc.setdefault("gaps", [])
        doc.setdefault("unparked", [])
        return doc

    def _save(self, doc: dict) -> None:
        doc["gaps"].sort(key=lambda r: r["gap_id"])
        self.run_dir.mkdir(parents=True, exist_ok=True)
        tmp = self.path.with_suffix(".json.tmp")
        tmp.write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")
        os.replace(tmp, self.path)

    def rows(self) -> list[dict]:
        return self._load()["gaps"]

    def get(self, gap_id: str) -> dict | None:
        return next((r for r in self.rows() if r["gap_id"] == gap_id), None)

    @staticmethod
    def _row(doc: dict, gap_id: str, now: str) -> dict:
        for r in doc["gaps"]:
            if r["gap_id"] == gap_id:
                return r
        row = {"gap_id": gap_id, "items": [], "core_items": [], "is_core": False, "parked_at": now}
        doc["gaps"].append(row)
        return row

    # ---------------------------------------------------------------- GapLane

    def park(self, item: str, gap_id: str, *, is_core: bool) -> None:
        split_item_id(item)  # raises on a malformed id
        if not gap_id or not str(gap_id).strip():
            raise ValueError("empty gap id")
        doc = self._load()
        row = self._row(doc, gap_id, self.now())
        if item not in row["items"]:
            row["items"].append(item)
        if is_core and item not in row["core_items"]:
            row["core_items"].append(item)
        row["is_core"] = bool(row["core_items"])
        self._save(doc)

    def unpark(self, gap_id: str) -> list[str]:
        doc = self._load()
        row = next((r for r in doc["gaps"] if r["gap_id"] == gap_id), None)
        if row is None:
            return []
        doc["gaps"].remove(row)
        doc["unparked"].append({**row, "unparked_at": self.now()})
        self._save(doc)
        return list(row["items"])

    def core_clauses(self, row: Mapping) -> int:
        units: set[str] = set()
        for item in row.get("core_items", []):
            kind, ident = split_item_id(item)
            if kind == "card":
                units.update(self.clauses.get(ident) or [f"{ident}#*"])
            elif kind == "clause":
                units.add(ident)
            else:
                units.add(item)
        return len(units)

    def ranked(self) -> list[tuple[str, int]]:
        rows = [r for r in self.rows() if r.get("items")]
        scored = [(self.core_clauses(r), r) for r in rows]
        scored.sort(key=lambda s: (-s[0], -len(s[1]["items"]), s[1].get("parked_at") or "",
                                   s[1]["gap_id"]))
        return [(r["gap_id"], n) for n, r in scored]

    def note_branch(self, gap_id: str, branch: str, sha: str | None = None) -> None:
        """Record the engine branch (and its tip) a gap's fix landed on."""
        doc = self._load()
        row = self._row(doc, gap_id, self.now())
        row["branch"] = branch
        if sha:
            row["branch_sha"] = sha
        self._save(doc)

    def closed(self, ctx: Any) -> list[str]:
        repo = str(ctx.repo)
        base = getattr(ctx, "base_sha", None)
        head = _rev(repo, "HEAD")
        texts = {kind: self._read_tracker(Path(repo) / path) for kind, path in TRACKERS.items()}
        out = []
        for row in self.rows():
            if self._branch_landed(repo, row, base, head) or any(
                    tracker_status(texts[k], row["gap_id"]) == "closed"
                    for k in ([row["kind"]] if row.get("kind") in texts else texts)):
                out.append(row["gap_id"])
        return sorted(out)

    # ---------------------------------------------------------------- internals

    @staticmethod
    def _read_tracker(path: Path) -> str:
        try:
            return path.read_text(encoding="utf-8")
        except OSError:
            return ""

    @staticmethod
    def _branch_landed(repo: str, row: Mapping, base: str | None, head: str | None) -> bool:
        if head is None:
            return False
        tip = (_rev(repo, f"refs/heads/{row['branch']}") if row.get("branch") else None) \
            or row.get("branch_sha")
        if not tip or _rev(repo, tip) is None:
            return False
        if base and _rev(repo, base):
            ahead = _git(repo, "rev-list", "--count", f"{base}..{tip}")
            if ahead.returncode != 0 or ahead.stdout.strip() == "0":
                return False  # nothing was ever committed on the branch
        return _git(repo, "merge-base", "--is-ancestor", tip, head).returncode == 0

    def repo(self) -> Path:
        if self._repo is not None:
            return self._repo
        cp = _git(self.run_dir if self.run_dir.is_dir() else Path.cwd(), "rev-parse",
                  "--show-toplevel")
        return Path(cp.stdout.strip()) if cp.returncode == 0 and cp.stdout.strip() else Path.cwd()

    # ---------------------------------------------------------------- tracker writes

    def record_gap(self, gap: Mapping | str, kind: str, *, attempt_id: str | None = None,
                   family: str | None = None, model: str | None = None,
                   commit: bool = True) -> bool:
        """Append `gap` (`{id|gap_id, summary}`) to the `kind` tracker unless its
        id is already there. Returns True when an entry was appended. Commits
        the tracker alone on the run branch (with `Loop-*` trailers when the
        surfacing attempt is given); on `main`/`master` it writes but does not
        commit."""
        if kind not in TRACKERS:
            raise ValueError(f"gap kind must be one of {sorted(TRACKERS)}, got {kind!r}")
        if isinstance(gap, str):
            gap = {"id": gap, "summary": gap}
        gap_id = str(gap.get("id") or gap.get("gap_id") or "").strip()
        if not gap_id or any(c.isspace() for c in gap_id):
            raise ValueError(f"bad gap id {gap_id!r}")
        summary = " ".join(str(gap.get("summary") or gap_id).split())
        repo = self.repo()
        rel = TRACKERS[kind]
        path = repo / rel
        text = self._read_tracker(path)
        if _token(gap_id).search(text):
            self._remember_kind(gap_id, kind)
            return False
        entry = self._entry(gap_id, summary, kind, attempt_id)
        nl = "\r\n" if "\r\n" in text else "\n"
        body = text if not text or text.endswith("\n") else text + nl
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes((body + entry.replace("\n", nl)).encode("utf-8"))
        self._remember_kind(gap_id, kind)
        if commit:
            self._commit(repo, rel, gap_id, kind, attempt_id, family, model)
        return True

    def _remember_kind(self, gap_id: str, kind: str) -> None:
        doc = self._load()
        row = next((r for r in doc["gaps"] if r["gap_id"] == gap_id), None)
        if row is not None and row.get("kind") != kind:
            row["kind"] = kind
            self._save(doc)

    def _entry(self, gap_id: str, summary: str, kind: str, attempt_id: str | None) -> str:
        date = self.now()[:10]
        run = self.plan.get("run_id") or self.run_dir.name
        row = self.get(gap_id) or {}
        items = row.get("items") or []
        surfaced = f"Surfaced: {date}, card-loop run `{run}`" + (
            f", attempt `{attempt_id}`." if attempt_id else ".")
        return (
            f"\n## {summary}  [{gap_id}] — OPEN ({date}, card-loop run `{run}`)\n"
            f"<!-- card-loop-gap:{gap_id} -->\n\n"
            f"{surfaced}\n\n"
            f"- **Gap ({kind}):** {summary}\n"
            f"- **Blocks:** {', '.join(items) or '-'} ({len(row.get('core_items') or [])} core)\n"
            f"- **Close:** mark this heading RESOLVED when the substrate lands (or merge the "
            f"gap's `card-loop/{run}/engine-{gap_id}` branch); the loop unparks the blocked "
            f"items on its next resume.\n"
        )

    def _commit(self, repo: Path, rel: str, gap_id: str, kind: str, attempt_id: str | None,
                family: str | None, model: str | None) -> None:
        branch = _git(repo, "rev-parse", "--abbrev-ref", "HEAD").stdout.strip()
        if not branch or branch in PROTECTED_BRANCHES:
            return
        run = self.plan.get("run_id") or self.run_dir.name
        subject = f"card-loop: record {kind} gap {gap_id}"
        if attempt_id and family:
            msg = loop_commit_message(subject, None, attempt_id=attempt_id, family=family,
                                      model=model, extra_trailers=[f"Loop-Run: {run}"])
        else:
            msg = f"{subject}\n\nLoop-Run: {run}\n"
        _git(repo, "add", "--", rel, check=True)
        _git(repo, "commit", "-q", "-m", msg, "--", rel, check=True)
