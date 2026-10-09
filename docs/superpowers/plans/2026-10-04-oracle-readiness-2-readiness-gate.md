# Oracle Readiness Plan 2 — Readiness Artifact, Training Gate, and Exam Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** RL training admits a decklist only when every card in it is adjudicated against the DCGO oracle, and an exam plan ranks which cards to examine next to grow the pool fastest.

**Architecture:** A deterministic Python generator (`tools.clause_coverage.readiness`) joins the printed clause denominator (`exam_binding.bind`) with the verdict store and writes the committed artifact `data/oracle_readiness.json`. `gauntlet.py` reads only that artifact and rejects any decklist with a non-ready card, for every library path and every pool snapshot, failing fast when nothing survives. `--plan` ranks not-ready cards by greedy decklist completion.

**Tech Stack:** Python 3.11+ (stdlib only in the generator), pytest, existing `digimon_gym` gauntlet.

**Spec:** `docs/superpowers/specs/2026-10-04-dcgo-oracle-readiness-design.md` §4.7, §4.8 (plan part), §6, §7, §8. Depends on Plan 1 Task 5 (`triage`/`citation` passed through `bind()`). The gate is ON by default with no override — the user accepted the pool reduction.

## Global Constraints

- Work only in the worktree `C:\Users\james\Documents\digimon-deck-list-builder-1\.claude\worktrees\dcgo-effect-translation`; verify with `git rev-parse --show-toplevel` first.
- The gate is ON by default. Do not add a CLI flag, env var, or config switch that turns it off. Tests may inject their own ready set (same pattern as the ledger gate's `not_ready_card_ids`).
- A card is `ready` iff every printed clause is individually adjudicated: `confirmed`; `unreachable` or `unavailable` with a non-empty `reason`; or `diverged` with `triage == "dcgo_quirk"` and a non-empty `citation`. A clause with no verdict row is `unmeasured` and blocks. A card with zero clauses is `ready`.
- The generator runs extraction with `DIGIMON_DCGO_ROOT=""` so the artifact is identical with or without the base-repo DCGO checkout.
- The artifact is byte-deterministic: `json.dumps(data, indent=2, sort_keys=True) + "\n"`, UTF-8, no timestamps.
- Python runs from the worktree root: `PYTHONPATH=code python -m ...`; tests: `python -m pytest <path>`.
- Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## File Map

| File | Change | Responsibility |
|---|---|---|
| `code/data_paths.py` | Modify | `ORACLE_READINESS` path constant |
| `code/tools/clause_coverage/readiness.py` | Create | Status rules, artifact build/check, exam plan ranking |
| `code/tests/tools/test_clause_coverage_readiness.py` | Create | Generator + plan tests |
| `data/oracle_readiness.json` | Create (generated) | The committed readiness artifact |
| `scripts/verify` | Modify | Tier-0 `--check` suite + test file |
| `.github/workflows/verification-ladder.yml` | Modify | Trigger on verdict / library / artifact changes |
| `code/digimon_gym/agents/gauntlet.py` | Modify | Load artifact, gate every library + snapshot, fail fast |
| `code/tests/conftest.py` | Create | Permissive oracle gate for tests that don't opt in |
| `pyproject.toml` | Modify | Register the `oracle_gate` marker |
| `code/tests/rl/test_gauntlet.py` | Modify | Gate tests |
| `docs/TRAINING_RUNBOOK.md` | Modify | Operator note: regenerate, read the fail-fast message |

---

### Task 1: Readiness status rules and artifact builder

**Files:**
- Modify: `code/data_paths.py`
- Create: `code/tools/clause_coverage/readiness.py`
- Create: `code/tests/tools/test_clause_coverage_readiness.py`

**Interfaces:**
- Consumes: `tools.clause_coverage.exam_binding.bind(card_ids, scenarios_dir, verdicts_path, *, source_desc=None) -> dict` whose `["cards"][cid]` is `{"card_id", "total_clauses", "by_verdict", "clauses": [{"clause_id", "verdict", "reason", "triage", "citation", ...}]}`.
- Produces:
  - `clause_is_adjudicated(clause: dict) -> bool`
  - `card_status(card_report: dict) -> dict` → `{"status": "ready"|"not_ready", "total_clauses": int, "by_verdict": dict, "blocking": list[str]}`
  - `library_decklists(library_path: Path) -> list[list[str]]`
  - `build_readiness(card_ids: Iterable[str], *, scenarios_dir: Path, verdicts_dir: Path, bind_fn=None) -> dict` → `{"version": 1, "cards": {cid: card_status}, "summary": {"ready": int, "not_ready": int}}`
  - `render(data: dict) -> str`
  - `data_paths.ORACLE_READINESS: Path`

- [ ] **Step 1: Add the path constant**

In `code/data_paths.py`, after `TESTED_CARDS`:

```python
ORACLE_READINESS: Path = _resolve("DIGIMON_ORACLE_READINESS", "oracle_readiness.json")
```

- [ ] **Step 2: Write the failing tests**

Create `code/tests/tools/test_clause_coverage_readiness.py`:

```python
import json

import pytest

from tools.clause_coverage.readiness import (
    build_readiness,
    card_status,
    clause_is_adjudicated,
    library_decklists,
    render,
)


def _clause(cid, verdict, **extra):
    row = {"clause_id": cid, "verdict": verdict, "reason": None, "triage": None, "citation": None}
    row.update(extra)
    return row


@pytest.mark.parametrize(
    "clause, expected",
    [
        (_clause("A#effect#0", "confirmed"), True),
        (_clause("A#effect#0", "unreachable", reason="background-process card"), True),
        (_clause("A#effect#0", "unreachable", reason="  "), False),
        (_clause("A#effect#0", "unavailable", reason="no DCGO script"), True),
        (_clause("A#effect#0", "unavailable"), False),
        (_clause("A#effect#0", "diverged", triage="dcgo_quirk", citation="G-EXAM-X"), True),
        (_clause("A#effect#0", "diverged", triage="dcgo_quirk"), False),
        (_clause("A#effect#0", "diverged", triage="ours_wrong", citation="15-8-3-2"), False),
        (_clause("A#effect#0", "diverged", triage="undetermined"), False),
        (_clause("A#effect#0", "diverged"), False),
        (_clause("A#effect#0", "unmeasured"), False),
    ],
)
def test_clause_rules(clause, expected):
    assert clause_is_adjudicated(clause) is expected


def test_card_ready_only_when_every_clause_is_adjudicated():
    report = {
        "card_id": "A",
        "total_clauses": 2,
        "by_verdict": {"confirmed": 1, "unmeasured": 1},
        "clauses": [_clause("A#effect#0", "confirmed"), _clause("A#inherited#0", "unmeasured")],
    }
    status = card_status(report)
    assert status["status"] == "not_ready"
    assert status["blocking"] == ["A#inherited#0"]


def test_zero_clause_card_is_ready():
    report = {"card_id": "V", "total_clauses": 0, "by_verdict": {}, "clauses": []}
    assert card_status(report)["status"] == "ready"


def test_build_readiness_is_deterministic_and_summarised(tmp_path):
    def fake_bind(card_ids, scenarios_dir, verdicts_path, *, source_desc=None):
        cards = {
            "B": {"card_id": "B", "total_clauses": 1, "by_verdict": {"confirmed": 1},
                  "clauses": [_clause("B#effect#0", "confirmed")]},
            "A": {"card_id": "A", "total_clauses": 1, "by_verdict": {"unmeasured": 1},
                  "clauses": [_clause("A#effect#0", "unmeasured")]},
        }
        return {"cards": {c: cards[c] for c in card_ids}}

    data = build_readiness(["B", "A", "B"], scenarios_dir=tmp_path, verdicts_dir=tmp_path,
                           bind_fn=fake_bind)
    assert list(data["cards"]) == ["A", "B"]
    assert data["summary"] == {"not_ready": 1, "ready": 1}
    assert render(data) == render(json.loads(render(data)))
    assert render(data).endswith("}\n")


def test_library_decklists_reads_json_and_card_ids(tmp_path):
    lib = tmp_path / "lib.json"
    lib.write_text(json.dumps({"archetypes": {
        "X": {"decklists": [{"decklist": json.dumps(["BT1-001", "BT1-001"])},
                            {"card_ids": ["BT1-002"]},
                            {"decklist": "not json"}]},
    }}), encoding="utf-8")
    assert library_decklists(lib) == [["BT1-001", "BT1-001"], ["BT1-002"]]
```

- [ ] **Step 3: Run to verify failure**

Run: `python -m pytest code/tests/tools/test_clause_coverage_readiness.py -v`
Expected: FAIL with `ModuleNotFoundError: No module named 'tools.clause_coverage.readiness'`.

- [ ] **Step 4: Implement `readiness.py` (builder half)**

```python
"""Oracle readiness: which cards RL training may draw.

A card is READY when every printed clause is adjudicated against the DCGO
oracle: confirmed; unreachable/unavailable with a stated reason; or diverged
and triaged as a cited DCGO quirk. Everything else blocks. Training reads only
the committed artifact this module writes (`data/oracle_readiness.json`).

Usage (from the repo root)::

    PYTHONPATH=code python -m tools.clause_coverage.readiness            # write
    PYTHONPATH=code python -m tools.clause_coverage.readiness --check    # CI drift
    PYTHONPATH=code python -m tools.clause_coverage.readiness --plan --limit 25
"""
from __future__ import annotations

import argparse
import json
import os
import sys
from collections import Counter
from pathlib import Path
from typing import Callable, Iterable

_CODE = Path(__file__).resolve().parents[2]
if str(_CODE) not in sys.path:
    sys.path.insert(0, str(_CODE))
_REPO = _CODE.parent

DEFAULT_LIBRARY = _REPO / "data" / "deck_library.json"
DEFAULT_OUT = _REPO / "data" / "oracle_readiness.json"
DEFAULT_TESTED = _REPO / "data" / "tested_cards.json"
DEFAULT_LEDGER = _REPO / "qa" / "qa-reports" / "validated_cards_dsl.json"
DEFAULT_VERDICTS = _REPO / "qa" / "qa-reports" / "exam-verdicts"
DEFAULT_SCENARIOS = _REPO / "qa" / "dcgo-exams"

READY = "ready"
NOT_READY = "not_ready"
# Mirror of gauntlet._NOT_READY_DSL_STATUSES (gauntlet needs the engine binding,
# which this stdlib tool must not import).
LEDGER_NOT_READY = frozenset({"PARTIAL", "BLOCKED", "AUDITED-DRIFT", "AUDITED-MISSING-TESTS"})


def _has_text(value) -> bool:
    return bool((value or "").strip())


def clause_is_adjudicated(clause: dict) -> bool:
    verdict = clause.get("verdict")
    if verdict == "confirmed":
        return True
    if verdict in ("unreachable", "unavailable"):
        return _has_text(clause.get("reason"))
    if verdict == "diverged":
        return clause.get("triage") == "dcgo_quirk" and _has_text(clause.get("citation"))
    return False


def card_status(card_report: dict) -> dict:
    blocking = sorted(
        c["clause_id"] for c in card_report.get("clauses", []) if not clause_is_adjudicated(c)
    )
    return {
        "status": READY if not blocking else NOT_READY,
        "total_clauses": int(card_report.get("total_clauses", 0)),
        "by_verdict": dict(sorted((card_report.get("by_verdict") or {}).items())),
        "blocking": blocking,
    }


def library_decklists(library_path: Path) -> list[list[str]]:
    lib = json.loads(Path(library_path).read_text(encoding="utf-8"))["archetypes"]
    out: list[list[str]] = []
    for entry in lib.values():
        for dl in entry.get("decklists", []):
            raw = dl.get("decklist")
            if raw:
                try:
                    ids = json.loads(raw)
                except (json.JSONDecodeError, TypeError):
                    continue
            else:
                ids = list(dl.get("card_ids", []))
            if ids:
                out.append([str(x) for x in ids])
    return out


def build_readiness(
    card_ids: Iterable[str],
    *,
    scenarios_dir: Path,
    verdicts_dir: Path,
    bind_fn: Callable | None = None,
) -> dict:
    if bind_fn is None:
        from tools.clause_coverage.exam_binding import bind as bind_fn
    ids = sorted(set(card_ids))
    report = bind_fn(ids, scenarios_dir, verdicts_dir, source_desc="oracle readiness")
    cards = {cid: card_status(report["cards"][cid]) for cid in sorted(report["cards"])}
    summary = Counter(s["status"] for s in cards.values())
    return {
        "version": 1,
        "cards": cards,
        "summary": {READY: summary.get(READY, 0), NOT_READY: summary.get(NOT_READY, 0)},
    }


def render(data: dict) -> str:
    return json.dumps(data, indent=2, sort_keys=True) + "\n"
```

- [ ] **Step 5: Run tests**

Run: `python -m pytest code/tests/tools/test_clause_coverage_readiness.py -v`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add code/data_paths.py code/tools/clause_coverage/readiness.py code/tests/tools/test_clause_coverage_readiness.py
git commit -m "readiness: per-card oracle readiness status and artifact builder" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Exam plan ranking and the CLI

**Files:**
- Modify: `code/tools/clause_coverage/readiness.py`
- Modify: `code/tests/tools/test_clause_coverage_readiness.py`

**Interfaces:**
- Produces:
  - `rank_plan(decklists: list[list[str]], cards: dict[str, dict], eligible: Callable[[str], bool], limit: int) -> dict` → `{"decklists_ready_now": int, "decklists_considered": int, "picks": [{"card_id", "decklists_completed", "score", "decklists_containing", "blocking"}]}`
  - `main(argv=None) -> int` with `--library --verdicts --scenarios --out --tested --ledger --check --plan --limit --json`.

- [ ] **Step 1: Write the failing tests**

Append:

```python
from tools.clause_coverage.readiness import rank_plan


def _cards(ready, not_ready):
    cards = {c: {"status": "ready", "total_clauses": 1, "by_verdict": {}, "blocking": []} for c in ready}
    cards.update({c: {"status": "not_ready", "total_clauses": 1, "by_verdict": {},
                      "blocking": [f"{c}#effect#0"]} for c in not_ready})
    return cards


def test_plan_prefers_the_card_that_completes_decklists():
    decks = [["R", "X"], ["R", "X"], ["R", "Y", "Z"]]
    plan = rank_plan(decks, _cards(["R"], ["X", "Y", "Z"]), lambda c: True, limit=3)
    assert plan["decklists_ready_now"] == 0
    assert [p["card_id"] for p in plan["picks"]][0] == "X"
    assert plan["picks"][0]["decklists_completed"] == 2


def test_plan_skips_decklists_the_other_gates_reject():
    decks = [["R", "X"], ["R", "UNREG"]]
    plan = rank_plan(decks, _cards(["R"], ["X", "UNREG"]), lambda c: c != "UNREG", limit=5)
    assert plan["decklists_considered"] == 1
    assert [p["card_id"] for p in plan["picks"]] == ["X"]


def test_plan_ties_break_by_card_id_for_determinism():
    decks = [["A1"], ["B1"]]
    plan = rank_plan(decks, _cards([], ["B1", "A1"]), lambda c: True, limit=2)
    assert [p["card_id"] for p in plan["picks"]] == ["A1", "B1"]
```

- [ ] **Step 2: Run to verify failure**

Run: `python -m pytest code/tests/tools/test_clause_coverage_readiness.py -v -k plan`
Expected: FAIL with `ImportError: cannot import name 'rank_plan'`.

- [ ] **Step 3: Implement `rank_plan` and `main`**

Append to `readiness.py`:

```python
def rank_plan(
    decklists: list[list[str]],
    cards: dict[str, dict],
    eligible: Callable[[str], bool],
    limit: int,
) -> dict:
    """Greedy decklist completion.

    Only decklists every card of which passes the OTHER gates (registered,
    no not-ready ledger verdict) are considered. Each round picks the card that
    completes the most decklists; ties fall to fractional progress
    (sum of 1/|missing| over decklists containing it), then to how many
    considered decklists contain it, then to card id.
    """
    considered: list[set[str]] = []
    for deck in decklists:
        unique = set(deck)
        if all(eligible(c) for c in unique):
            considered.append(unique)
    contains = Counter(c for deck in considered for c in deck)
    missing = [
        {c for c in deck if cards.get(c, {}).get("status") != READY} for deck in considered
    ]
    ready_now = sum(1 for m in missing if not m)
    missing = [m for m in missing if m]
    picks: list[dict] = []
    while missing and len(picks) < limit:
        score: Counter = Counter()
        completes: Counter = Counter()
        for m in missing:
            for c in m:
                score[c] += 1.0 / len(m)
            if len(m) == 1:
                completes[next(iter(m))] += 1
        best = sorted(score, key=lambda c: (-completes[c], -score[c], -contains[c], c))[0]
        picks.append(
            {
                "card_id": best,
                "decklists_completed": completes[best],
                "score": round(score[best], 4),
                "decklists_containing": contains[best],
                "blocking": cards.get(best, {}).get("blocking", [f"{best} (not in artifact)"]),
            }
        )
        missing = [m - {best} for m in missing]
        missing = [m for m in missing if m]
    return {"decklists_ready_now": ready_now, "decklists_considered": len(considered), "picks": picks}


def _eligibility(tested_path: Path, ledger_path: Path, cards: dict[str, dict]) -> Callable[[str], bool]:
    tested = set(json.loads(tested_path.read_text(encoding="utf-8")).get("card_ids", []))
    ledger = json.loads(ledger_path.read_text(encoding="utf-8")).get("cards", {})
    not_ready = {
        cid for cid, e in ledger.items()
        if str(e.get("status") or "").strip().upper() in LEDGER_NOT_READY
    }

    def eligible(card_id: str) -> bool:
        playable = card_id in tested or cards.get(card_id, {}).get("total_clauses", 1) == 0
        return playable and card_id not in not_ready

    return eligible


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Oracle readiness artifact and exam plan")
    parser.add_argument("--library", type=Path, default=DEFAULT_LIBRARY)
    parser.add_argument("--verdicts", type=Path, default=DEFAULT_VERDICTS)
    parser.add_argument("--scenarios", type=Path, default=DEFAULT_SCENARIOS)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    parser.add_argument("--tested", type=Path, default=DEFAULT_TESTED)
    parser.add_argument("--ledger", type=Path, default=DEFAULT_LEDGER)
    parser.add_argument("--check", action="store_true", help="fail if --out is missing or stale")
    parser.add_argument("--plan", action="store_true", help="rank not-ready cards to examine next")
    parser.add_argument("--limit", type=int, default=25)
    parser.add_argument("--json", action="store_true", help="--plan output as JSON")
    args = parser.parse_args(argv)

    # Determinism: never consult a local DCGO checkout during extraction.
    os.environ["DIGIMON_DCGO_ROOT"] = ""

    decklists = library_decklists(args.library)
    card_ids = {c for deck in decklists for c in deck}

    if args.plan:
        data = json.loads(args.out.read_text(encoding="utf-8"))
        plan = rank_plan(decklists, data["cards"], _eligibility(args.tested, args.ledger, data["cards"]),
                         args.limit)
        if args.json:
            print(json.dumps(plan, indent=2, sort_keys=True))
        else:
            print(f"decklists considered: {plan['decklists_considered']}, "
                  f"oracle-ready now: {plan['decklists_ready_now']}")
            for i, p in enumerate(plan["picks"], 1):
                print(f"{i:>3}. {p['card_id']:<10} completes {p['decklists_completed']:>4}  "
                      f"in {p['decklists_containing']:>4} decklists  "
                      f"outstanding: {', '.join(p['blocking'])}")
        return 0

    text = render(build_readiness(card_ids, scenarios_dir=args.scenarios, verdicts_dir=args.verdicts))
    if args.check:
        if not args.out.exists():
            print(f"error: {args.out} is missing; run "
                  "`PYTHONPATH=code python -m tools.clause_coverage.readiness`", file=sys.stderr)
            return 1
        if args.out.read_text(encoding="utf-8") != text:
            print(f"error: {args.out} is stale; run "
                  "`PYTHONPATH=code python -m tools.clause_coverage.readiness`", file=sys.stderr)
            return 1
        print(f"ok: {args.out} is up to date")
        return 0
    args.out.write_text(text, encoding="utf-8", newline="\n")
    data = json.loads(text)
    print(f"wrote {args.out}: {data['summary']['ready']} ready, "
          f"{data['summary']['not_ready']} not ready")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
```

- [ ] **Step 4: Run tests**

Run: `python -m pytest code/tests/tools/test_clause_coverage_readiness.py -v`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add code/tools/clause_coverage/readiness.py code/tests/tools/test_clause_coverage_readiness.py
git commit -m "readiness: --check drift mode and --plan greedy decklist-completion ranking" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Generate the artifact and wire the CI drift check

**Files:**
- Create: `data/oracle_readiness.json` (generated)
- Modify: `scripts/verify` (`tier0_suites()`)
- Modify: `.github/workflows/verification-ladder.yml` (both `paths:` lists)

- [ ] **Step 1: Generate**

Run: `PYTHONPATH=code python -m tools.clause_coverage.readiness`
Expected: `wrote ...data/oracle_readiness.json: N ready, M not ready`. Record N/M and the wall time; if extraction over the whole library takes over ~5 minutes, note it in the commit message (CI budget).

Run: `PYTHONPATH=code python -m tools.clause_coverage.readiness --check`
Expected: `ok: ... is up to date`.

- [ ] **Step 2: Sanity-check the Track A cards**

Run: `python -c "import json;d=json.load(open('data/oracle_readiness.json',encoding='utf-8'))['cards'];[print(c,d[c]['status'],d[c]['blocking']) for c in ['EX10-025','EX10-028','EX8-067','BT11-089']]"`
Expected: EX10-025 and EX10-028 `not_ready` (their inherited clauses are `ours_wrong`); EX8-067 `ready` only if Plan 1 Task 9 confirmed effect#1, else `not_ready` listing `EX8-067#effect#1`.

- [ ] **Step 3: Add the tier-0 suite**

In `scripts/verify`, inside `tier0_suites()`, after the `authoring_guide_check` entry add:

```python
        py_suite(
            0,
            "oracle_readiness_check",
            "code/tools/clause_coverage/readiness.py",
            "--check",
        ),
```

and add `"code/tests/tools/test_clause_coverage_readiness.py",` to the `verification_tool_tests` pytest list (after `test_clause_coverage_authoring_guide.py`).

Run: `python -m pytest code/tests/tools/test_verification_tier_manifest.py code/tests/tools/test_verify_script.py -q`
Expected: PASS. If a manifest test enumerates tier-0 suite names, add `oracle_readiness_check` to its expected list.

- [ ] **Step 4: Trigger CI on the inputs**

In `.github/workflows/verification-ladder.yml`, add to BOTH the `pull_request.paths` and `push.paths` lists:

```yaml
      - 'qa/qa-reports/exam-verdicts/**'
      - 'qa/exam-clause-text.json'
      - 'data/deck_library.json'
      - 'data/oracle_readiness.json'
      - 'data/cards.json'
      - 'data/card_official.json'
```

- [ ] **Step 5: Run tier 0 locally**

Run: `python scripts/verify --tier 0`
Expected: all tier-0 suites pass, including `oracle_readiness_check`. (If `--tier 0` is not a valid flag, run `python scripts/verify --tier 1`, which always includes tier 0.)

- [ ] **Step 6: Commit**

```bash
git add data/oracle_readiness.json scripts/verify .github/workflows/verification-ladder.yml
git commit -m "readiness: commit the oracle readiness artifact and gate drift in CI" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Gate every library load in the gauntlet and fail fast

**Files:**
- Modify: `code/digimon_gym/agents/gauntlet.py`
- Create: `code/tests/conftest.py`
- Modify: `pyproject.toml` (`markers`)
- Modify: `code/tests/rl/test_gauntlet.py`

**Interfaces:**
- Consumes: `data/oracle_readiness.json` (`{"cards": {cid: {"status": ...}}}`), `data_paths.ORACLE_READINESS`.
- Produces:
  - `class OracleReadinessMissingError(RuntimeError)`
  - `class EmptyTrainingPoolError(RuntimeError)`
  - `def _load_oracle_ready_card_ids(path: Optional[str | Path] = None) -> Set[str]` (raises `OracleReadinessMissingError` when the file is missing)
  - `MetaGauntlet.__init__(..., oracle_ready_card_ids: Optional[Set[str]] = None)` and `load_generalist_deck_pool(..., oracle_ready_card_ids: Optional[Set[str]] = None)`
  - Rejection reason key `"not_oracle_ready"` in the per-archetype `rejected` counter.

- [ ] **Step 1: Register the marker and the permissive default for unrelated tests**

In `pyproject.toml`, extend `markers`:

```toml
markers = [
    "ai_pipeline: AI pipeline tests (server.ai.* and DB deps)",
    "slow: Tests that take >10s (greedy baselines)",
    "oracle_gate: test exercises the real oracle readiness gate (no permissive default)",
]
```

Create `code/tests/conftest.py`:

```python
"""Shared test configuration.

The oracle readiness gate (gauntlet.py) is ON in production with no override.
Tests that build synthetic deck libraries are not about readiness, so by
default they see a ready set that contains every card. A test that exercises
the gate itself is marked `@pytest.mark.oracle_gate` and injects its own set.
"""
import pytest


class _EveryCard(frozenset):
    def __contains__(self, item):  # every card is "ready"
        return True


@pytest.fixture(autouse=True)
def _oracle_gate_permissive_unless_marked(request, monkeypatch):
    if request.node.get_closest_marker("oracle_gate"):
        return
    try:
        import digimon_gym.agents.gauntlet as gauntlet
    except ImportError:  # engine binding not built in this environment
        return
    monkeypatch.setattr(gauntlet, "_load_oracle_ready_card_ids", lambda path=None: _EveryCard())
```

- [ ] **Step 2: Write the failing gate tests**

Append to `code/tests/rl/test_gauntlet.py` (it already defines `_make_deck_library`, `_make_archetype`, `_with_card`, `MetaGauntlet`, `GeneralistDeckPool`, `gauntlet_module`):

```python
@pytest.mark.oracle_gate
class TestOracleReadinessGate:
    REGISTERED = {"BT12-002", "BT12-022", "BT12-031"}

    def _lib(self, tmp_path):
        lib = _make_deck_library({"Mixed": _make_archetype("Mixed", n_decks=2)})
        lib["archetypes"]["Mixed"]["decklists"][1]["decklist"] = _with_card("BT12-031")
        path = tmp_path / "lib.json"
        path.write_text(json.dumps(lib))
        return path

    def test_list_with_a_not_ready_card_is_rejected(self, tmp_path):
        g = MetaGauntlet(
            implemented_card_ids=self.REGISTERED,
            not_ready_card_ids=set(),
            oracle_ready_card_ids={"BT12-002", "BT12-022"},
        )
        g.load(str(self._lib(tmp_path)))
        assert g.deck_count == 1
        assert g._deck_pool[0].source_deck_id == "mixed_000"

    def test_gate_applies_to_a_non_default_library_path(self, tmp_path):
        # The ledger gate only auto-loads for the default path; the oracle gate
        # must not inherit that hole. Nothing is injected here: the loader runs.
        readiness = tmp_path / "oracle_readiness.json"
        readiness.write_text(json.dumps({"version": 1, "cards": {
            "BT12-002": {"status": "ready"}, "BT12-022": {"status": "ready"},
            "BT12-031": {"status": "not_ready"},
        }}))
        g = MetaGauntlet(implemented_card_ids=self.REGISTERED, not_ready_card_ids=set())
        original = gauntlet_module._ORACLE_READINESS_PATH
        gauntlet_module._ORACLE_READINESS_PATH = readiness
        try:
            g.load(str(self._lib(tmp_path)))
        finally:
            gauntlet_module._ORACLE_READINESS_PATH = original
        assert g.deck_count == 1

    def test_missing_artifact_fails_at_load(self, tmp_path):
        g = MetaGauntlet(implemented_card_ids=self.REGISTERED, not_ready_card_ids=set())
        original = gauntlet_module._ORACLE_READINESS_PATH
        gauntlet_module._ORACLE_READINESS_PATH = tmp_path / "absent.json"
        try:
            with pytest.raises(gauntlet_module.OracleReadinessMissingError,
                               match="tools.clause_coverage.readiness"):
                g.load(str(self._lib(tmp_path)))
        finally:
            gauntlet_module._ORACLE_READINESS_PATH = original

    def test_empty_pool_fails_fast_naming_the_blockers(self, tmp_path):
        g = MetaGauntlet(
            implemented_card_ids=self.REGISTERED,
            not_ready_card_ids=set(),
            oracle_ready_card_ids={"BT12-002"},
        )
        with pytest.raises(gauntlet_module.EmptyTrainingPoolError) as exc:
            g.load(str(self._lib(tmp_path)))
        assert "BT12-022" in str(exc.value)
        assert "--plan" in str(exc.value)

    def test_allowed_archetypes_empty_set_still_produces_an_empty_pool_quietly(self, tmp_path):
        # No decklist reached the oracle gate, so this is not an oracle failure.
        g = MetaGauntlet(
            implemented_card_ids=self.REGISTERED,
            not_ready_card_ids=set(),
            oracle_ready_card_ids=set(),
            allowed_archetypes=set(),
        )
        g.load(str(self._lib(tmp_path)))
        assert g.deck_count == 0
```

- [ ] **Step 3: Run to verify failure**

Run: `python -m pytest code/tests/rl/test_gauntlet.py -v -k OracleReadinessGate`
Expected: FAIL — `TypeError: __init__() got an unexpected keyword argument 'oracle_ready_card_ids'` / missing attributes.

- [ ] **Step 4: Implement the gate**

In `gauntlet.py`:

1. Extend the `data_paths` import:

```python
from data_paths import (
    ARCHETYPE_ALIASES as _ARCHETYPE_ALIASES_PATH,
    DECK_LIBRARY as _DECK_LIBRARY_PATH,
    ORACLE_READINESS as _ORACLE_READINESS_PATH,
)
```

2. After `_load_not_ready_card_ids`, add:

```python
class OracleReadinessMissingError(RuntimeError):
    """`data/oracle_readiness.json` is missing; training refuses to guess."""


class EmptyTrainingPoolError(RuntimeError):
    """No decklist survived the oracle readiness gate."""


_REGENERATE_READINESS = "PYTHONPATH=code python -m tools.clause_coverage.readiness"


def _load_oracle_ready_card_ids(path: Optional[str | Path] = None) -> Set[str]:
    """Card IDs whose every printed clause is adjudicated against the DCGO oracle.

    Reads the committed artifact written by `tools.clause_coverage.readiness`.
    There is deliberately no fallback: a missing artifact stops training.
    """
    path = Path(_ORACLE_READINESS_PATH if path is None else path)
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        raise OracleReadinessMissingError(
            f"{path} is missing; regenerate it with `{_REGENERATE_READINESS}`"
        ) from None
    return {
        str(cid) for cid, entry in raw.get("cards", {}).items()
        if entry.get("status") == "ready"
    }


def _describe_oracle_blockers(blockers: Counter, top: int = 10) -> str:
    worst = ", ".join(f"{cid} ({n})" for cid, n in blockers.most_common(top))
    return (
        "No training decklist is oracle-ready. Cards blocking the most decklists "
        f"(decklists blocked): {worst}. Next exams: `{_REGENERATE_READINESS} --plan`."
    )
```

(`Counter` and `Path` are already imported in this module; if not, add them.)

3. `load_generalist_deck_pool`: add parameter `oracle_ready_card_ids: Optional[Set[str]] = None` and pass `oracle_ready_card_ids=oracle_ready_card_ids` to `MetaGauntlet(...)`.

4. `MetaGauntlet.__init__`: add parameter `oracle_ready_card_ids: Optional[Set[str]] = None` after `not_ready_card_ids`, and store `self._oracle_ready_card_ids = oracle_ready_card_ids`.

5. In `load`, after the `not_ready_card_ids` block (the one ending with `not_ready_card_ids = _load_not_ready_card_ids()`), add:

```python
        # The oracle gate applies to EVERY library path (no default-path-only
        # hole like the ledger gate's). Tests inject; production loads the artifact.
        oracle_ready = (
            self._oracle_ready_card_ids
            if self._oracle_ready_card_ids is not None
            else _load_oracle_ready_card_ids()
        )
        oracle_blockers: Counter = Counter()
```

6. In the admission loop, immediately after the `if not_ready_card_ids is not None: ... continue` block and before `decks.append(DeckEntry(`, add:

```python
                not_oracle = sorted({cid for cid in card_ids if cid not in oracle_ready})
                if not_oracle:
                    logger.debug(
                        "Skipping not-oracle-ready decklist %s for %s: %s",
                        dl.get("deck_id", "?"), arch_name, ", ".join(not_oracle),
                    )
                    rejected[arch_name]["not_oracle_ready"] += 1
                    oracle_blockers.update(not_oracle)
                    continue
```

7. In the `if canonical_allowed is not None:` summary log, change the message to include the new count:

```python
                logger.info(
                    "allowed_archetypes entry %r excluded: no training-ready decklist "
                    "(%d with unregistered cards, %d with not-ready ledger verdicts, "
                    "%d with not-oracle-ready cards)",
                    name,
                    rejected[name]["unregistered"],
                    rejected[name]["not_ready"],
                    rejected[name]["not_oracle_ready"],
                )
```

8. Directly after that `if canonical_allowed is not None:` block and before `self._compute_threat_indices()`, add:

```python
        if not self.archetypes and sum(r["not_oracle_ready"] for r in rejected.values()) > 0:
            raise EmptyTrainingPoolError(_describe_oracle_blockers(oracle_blockers))
```

- [ ] **Step 5: Run the gate tests and the whole gauntlet file**

Run: `python -m pytest code/tests/rl/test_gauntlet.py -v`
Expected: PASS — existing tests see the permissive default from `code/tests/conftest.py`.

Run: `python -m pytest code/tests/rl -q`
Expected: PASS. Any failure in a test this change does not touch must be reproduced on unmodified `origin/main` (a separate checkout, e.g. `git worktree add ../rl-main-check origin/main`) before you debug it. Never use `git stash` here — the stash stack is shared across worktrees.

- [ ] **Step 6: Commit**

```bash
git add code/digimon_gym/agents/gauntlet.py code/tests/conftest.py pyproject.toml code/tests/rl/test_gauntlet.py
git commit -m "training: gate every deck library on oracle readiness and fail fast on an empty pool" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Gate pool snapshots

`GeneralistDeckPool.from_snapshot` (used by `--curriculum-pool` and the eval/Elo/exploiter CLIs) bypasses the library, so a snapshot taken before the gate would smuggle non-ready decks back in.

**Files:**
- Modify: `code/digimon_gym/agents/gauntlet.py` (`from_snapshot`, ~L320)
- Modify: `code/tests/rl/test_gauntlet.py`

**Interfaces:**
- Produces: `class NotOracleReadyDeckError(ValueError)`; `from_snapshot(path, *, implemented_card_ids=None, oracle_ready_card_ids: Optional[Set[str]] = None)`.

- [ ] **Step 1: Write the failing test**

Add to `TestOracleReadinessGate`:

```python
    def test_snapshot_with_a_not_ready_card_is_refused(self, tmp_path):
        g = MetaGauntlet(
            implemented_card_ids=self.REGISTERED,
            not_ready_card_ids=set(),
            oracle_ready_card_ids=set(self.REGISTERED),
        )
        g.load(str(self._lib(tmp_path)))
        snapshot = tmp_path / "pool.json"
        g.as_generalist_pool().write_snapshot(snapshot)
        with pytest.raises(gauntlet_module.NotOracleReadyDeckError, match="BT12-031"):
            GeneralistDeckPool.from_snapshot(
                snapshot,
                implemented_card_ids=self.REGISTERED,
                oracle_ready_card_ids={"BT12-002", "BT12-022"},
            )

    def test_snapshot_of_ready_decks_still_loads(self, tmp_path):
        g = MetaGauntlet(
            implemented_card_ids=self.REGISTERED,
            not_ready_card_ids=set(),
            oracle_ready_card_ids=set(self.REGISTERED),
        )
        g.load(str(self._lib(tmp_path)))
        snapshot = tmp_path / "pool.json"
        written = g.as_generalist_pool().write_snapshot(snapshot)
        loaded = GeneralistDeckPool.from_snapshot(
            snapshot, implemented_card_ids=self.REGISTERED,
            oracle_ready_card_ids=set(self.REGISTERED),
        )
        assert loaded.snapshot_hash == written
```

- [ ] **Step 2: Run to verify failure**

Run: `python -m pytest code/tests/rl/test_gauntlet.py -v -k snapshot`
Expected: FAIL — unexpected keyword `oracle_ready_card_ids` / missing `NotOracleReadyDeckError`.

- [ ] **Step 3: Implement**

Next to `UnimplementedDeckError`:

```python
class NotOracleReadyDeckError(ValueError):
    """A pool snapshot contains a card that is not oracle-ready."""
```

In `from_snapshot`, add the keyword parameter `oracle_ready_card_ids: Optional[Set[str]] = None`; before the `archetypes: Dict[...] = {}` loop add:

```python
        oracle_ready = (
            oracle_ready_card_ids
            if oracle_ready_card_ids is not None
            else _load_oracle_ready_card_ids()
        )
```

and inside the loop, right after the `validate_implemented_deck(...)` block:

```python
            not_ready = sorted({cid for cid in card_ids if cid not in oracle_ready})
            if not_ready:
                raise NotOracleReadyDeckError(
                    f"snapshot deck {record.get('deck_id', '?')} contains cards that are not "
                    f"oracle-ready: {', '.join(not_ready)}"
                )
```

- [ ] **Step 4: Run tests**

Run: `python -m pytest code/tests/rl -q`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add code/digimon_gym/agents/gauntlet.py code/tests/rl/test_gauntlet.py
git commit -m "training: refuse pool snapshots that contain non-oracle-ready cards" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Operator documentation and an end-to-end check

**Files:**
- Modify: `docs/TRAINING_RUNBOOK.md`
- Modify: `CLAUDE.md` (Working Rules: one new rule)

- [ ] **Step 1: Runbook section**

Append to `docs/TRAINING_RUNBOOK.md`:

```markdown
## Oracle readiness gate (2026-10-04)

Training admits a decklist only when every card in it is oracle-ready:
every printed clause is confirmed against DCGO, or unreachable/unavailable
with a reason, or a diverged clause triaged as a cited DCGO quirk. The gate
is always on — there is no flag to disable it. Source of truth:
`data/oracle_readiness.json`, regenerated with

    PYTHONPATH=code python -m tools.clause_coverage.readiness

and checked in CI (`scripts/verify` tier 0). If training stops with
`EmptyTrainingPoolError`, the message names the cards blocking the most
decklists; the exam queue is

    PYTHONPATH=code python -m tools.clause_coverage.readiness --plan --limit 25

Pool snapshots (`--curriculum-pool`, eval CLIs) are refused if they contain a
non-ready card (`NotOracleReadyDeckError`). Spec:
`docs/superpowers/specs/2026-10-04-dcgo-oracle-readiness-design.md`.
```

- [ ] **Step 2: CLAUDE.md rule**

Append to the Working Rules list in `CLAUDE.md`:

```markdown
34. **Training is gated on DCGO oracle readiness (2026-10-04).** `gauntlet.py` admits a decklist only if every card is `ready` in `data/oracle_readiness.json` (every printed clause confirmed against the oracle, reasoned unreachable/unavailable, or a cited `dcgo_quirk`). It applies to every library path and every pool snapshot, has no override, and fails fast with `EmptyTrainingPoolError` naming the top blockers. Regenerate the artifact after any verdict change (`PYTHONPATH=code python -m tools.clause_coverage.readiness`); pick exams with `--plan`. Spec: `docs/superpowers/specs/2026-10-04-dcgo-oracle-readiness-design.md`.
```

- [ ] **Step 3: End-to-end check with the real artifact**

Run: `python -c "from digimon_gym.agents.gauntlet import load_generalist_deck_pool as l; p=l(); print(p.archetype_count, p.deck_count)"`
Expected: either a small pool (archetype and deck counts printed) or `EmptyTrainingPoolError: No training decklist is oracle-ready. Cards blocking the most decklists ...` — both are correct outcomes of the gate; record which one in the commit message.

Run: `PYTHONPATH=code python -m tools.clause_coverage.readiness --plan --limit 10`
Expected: a ranked list of 10 cards with decklist counts.

- [ ] **Step 4: Commit**

```bash
git add docs/TRAINING_RUNBOOK.md CLAUDE.md
git commit -m "docs: the oracle readiness training gate" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
