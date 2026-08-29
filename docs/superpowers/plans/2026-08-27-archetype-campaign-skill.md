# Archetype Campaign Skill Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make "do Hunters" a thing you can hand to a warm node. One dispatch resolves the archetype to its cards, skips what is already confirmed, implements what has no YAML, exams what does, triages what diverges, and leaves a ledger entry that stops anyone repeating the work.

**Architecture:** Python resolves an archetype to its card pool and its competitive core from `data/deck_library.json`; that feeds the existing clause binding to produce a work plan of outstanding clauses only. A `/archetype-campaign` skill drives the phases, using the MCP surface for its inner loop and the existing card-implementation and exam skills for the heavy lifting. It re-implements neither.

**Tech Stack:** Python 3 stdlib (`code/tools/clause_coverage/`), Markdown (the skill).

**Spec:** `docs/superpowers/specs/2026-08-27-archetype-campaign-fleet-design.md` §2.

**Prerequisites:** the ledger plan (verdicts, log, claims, index), the MCP plan (`exam_plan`, `exam_probe`, `exam_validate`, `exam_keyword_brief`), and the node plan (`node_health`).

## Global Constraints

- Python tools are **standard-library only**, matching `code/tools/clause_coverage/`.
- **Clause identity is never invented** — `{card_id}#{zone}#{idx}` from `clause_coverage`.
- **`unmeasured` is a real outcome.** Every report prints the full denominator; a card never reads as "passed".
- **DCGO is source priority #2.** `general_rule.pdf` outranks it; `diverged` is a finding to triage, never proof our engine is wrong.
- **The campaign does not re-implement cards.** It composes `/batch-implement-cards-rust-dsl` and `/dcgo-exam`.
- **No approximations** (CLAUDE.md rule 17): every choice must reach the RL action space. A campaign may not "simplify" a card to make a scenario pass.

## Verified data facts this plan depends on

Measured directly, not assumed:

- `data/deck_library.json` is `{version, generated_at, total_entries, archetypes}` where `archetypes` is a **dict of 432** keyed by archetype name.
- Each archetype entry has keys `archetype_name`, `decklists`, `display_card_id`, `format_stats`, `primary_color`, `secondary_color`, `stats`.
- `decklists` is a list (Toho Braves: **45**) of entries with `deck_id`, `decklist`, `event_date`, `is_top_cut`, `placement`, `format`, …
- **`decklist` is a JSON-encoded *string*** holding a list of card-id strings with duplicates for copies — e.g. `'["EX12-075", "EX12-075", ...]'`. It must be `json.loads`-ed, not iterated directly.
- Parsing Toho Braves yields **42 distinct cards**, matching the published "42-card tournament pool".
- **Cards in ≥70% of its 45 lists = 18**, exactly matching the published "18-card competitive core (≥33 of 45 lists)". So the spec's 0.7 default reproduces the report.

## File Structure

| File | Responsibility |
|---|---|
| `code/tools/clause_coverage/archetype.py` (create) | Archetype → card pool, per-card list frequency, core set. |
| `code/tools/clause_coverage/campaign.py` (create) | Work plan: outstanding clauses, split into implement/exam, keyword-tagged. |
| `code/tools/clause_coverage/exam_index.py` (modify) | `main()` renders real archetypes instead of an empty index. |
| `.claude/skills/archetype-campaign/SKILL.md` (create) | The dispatchable workflow. |
| `code/tests/tools/test_clause_coverage_archetype.py` (create) | Resolution + core threshold. |
| `code/tests/tools/test_clause_coverage_campaign.py` (create) | Work-plan shape and skipping. |
| `CLAUDE.md`, `docs/DCGO_EXAM.md` (modify) | Register the skill. |

---

### Task 1: Archetype resolution and the competitive core

**Files:**
- Create: `code/tools/clause_coverage/archetype.py`, `code/tests/tools/test_clause_coverage_archetype.py`

**Interfaces:**
- Produces:
  ```python
  DEFAULT_CORE_FRACTION = 0.7

  def load_archetypes(path: Path) -> dict          # name -> entry
  def resolve(library: dict, name: str) -> str     # canonical name; raises LookupError with near-misses
  def card_frequency(entry: dict) -> dict[str, int]   # card_id -> how many lists contain it
  def pool(entry: dict) -> list[str]                  # distinct card ids, sorted
  def core(entry: dict, fraction: float = DEFAULT_CORE_FRACTION) -> dict
      # {"cards": [...], "threshold": int, "list_count": int, "fraction": float}
  ```

**Why `core` returns the threshold and list count, not just cards:** the report must print "≥33 of 45 lists", and a caller that only gets a card list has to recompute the denominator or invent it. Returning it removes the chance of a report quoting a fraction it did not use.

- [ ] **Step 1: Write the failing tests**

Create `code/tests/tools/test_clause_coverage_archetype.py`:

```python
"""Archetype -> card pool and competitive core.

The core threshold is a FRACTION of the archetype's recorded lists, not a raw
count: a raw 33 would silently redefine the core for an archetype with a
different corpus size.
"""

import json
from pathlib import Path

import pytest

from tools.clause_coverage.archetype import (
    DEFAULT_CORE_FRACTION,
    card_frequency,
    core,
    load_archetypes,
    pool,
    resolve,
)

LIBRARY = Path("data/deck_library.json")


def _fixture_entry(lists: list[list[str]]) -> dict:
    """Build an archetype entry the way deck_library.json really stores one:
    `decklist` is a JSON-encoded STRING, not a list."""
    return {
        "archetype_name": "Fixture",
        "decklists": [{"deck_id": str(i), "decklist": json.dumps(cards)}
                      for i, cards in enumerate(lists)],
    }


def test_decklist_is_parsed_from_its_json_string():
    entry = _fixture_entry([["A-001", "A-001", "B-002"], ["A-001"]])
    freq = card_frequency(entry)
    assert freq["A-001"] == 2, "counted per LIST, not per copy"
    assert freq["B-002"] == 1


def test_pool_is_distinct_and_sorted():
    entry = _fixture_entry([["B-002", "A-001", "A-001"]])
    assert pool(entry) == ["A-001", "B-002"]


def test_core_threshold_is_a_fraction_of_the_list_count():
    # 10 lists, 0.7 -> a card must appear in >= 7 of them.
    entry = _fixture_entry([["A-001"]] * 7 + [["B-002"]] * 3)
    c = core(entry, 0.7)
    assert c["list_count"] == 10
    assert c["threshold"] == 7
    assert c["cards"] == ["A-001"]


def test_core_reports_the_threshold_it_used():
    """A report must be able to print '>=N of M lists' without recomputing it."""
    entry = _fixture_entry([["A-001"]] * 4)
    c = core(entry, DEFAULT_CORE_FRACTION)
    assert set(c) == {"cards", "threshold", "list_count", "fraction"}


def test_resolve_is_case_insensitive_and_suggests_near_misses():
    lib = {"Toho Braves": {}, "Hunters": {}}
    assert resolve(lib, "toho braves") == "Toho Braves"
    with pytest.raises(LookupError) as e:
        resolve(lib, "Toho Brave")
    assert "Toho Braves" in str(e.value), "an unknown name must suggest, not just fail"


def test_real_library_reproduces_the_published_toho_figures():
    """Guards the 0.7 default against the published report: 42-card pool,
    18-card core, 45 lists. If deck_library.json is re-scraped and these move,
    this fails loudly rather than letting a report quote stale figures."""
    lib = load_archetypes(LIBRARY)
    entry = lib[resolve(lib, "Toho Braves")]
    assert len(entry["decklists"]) == 45
    assert len(pool(entry)) == 42
    c = core(entry, DEFAULT_CORE_FRACTION)
    assert len(c["cards"]) == 18, f"expected the published 18-card core, got {len(c['cards'])}"
    assert c["threshold"] == 31
```

- [ ] **Step 2: Run to verify failure**

Run: `python -m pytest code/tests/tools/test_clause_coverage_archetype.py -v`

Expected: FAIL — `ModuleNotFoundError: tools.clause_coverage.archetype`.

- [ ] **Step 3: Implement**

Create `code/tools/clause_coverage/archetype.py`:

```python
"""Resolve an archetype to the cards a campaign is about.

`data/deck_library.json` stores `archetypes` as a dict keyed by archetype name.
Each entry's `decklists` is a list of tournament entries, and each entry's
`decklist` field is a **JSON-encoded string** of card ids with duplicates for
copies -- so it must be `json.loads`-ed, not iterated.

The competitive core is a FRACTION of the archetype's recorded lists, never a
raw count. The published Toho Braves report describes its core as ">=33 of 45
lists"; hardcoding 33 would silently redefine the core for an archetype with a
different corpus size. 0.7 reproduces that figure exactly (31 of 45 -> the same
18 cards), which is what `test_real_library_reproduces_the_published_toho_figures`
pins.

Standard library only.
"""

from __future__ import annotations

import json
from collections import Counter
from difflib import get_close_matches
from pathlib import Path

#: A card is "core" if it appears in at least this fraction of the lists.
DEFAULT_CORE_FRACTION = 0.7


def load_archetypes(path: Path | str) -> dict:
    """Load `deck_library.json` -> ``{archetype_name: entry}``."""
    data = json.loads(Path(path).read_text(encoding="utf-8"))
    archetypes = data.get("archetypes")
    if not isinstance(archetypes, dict):
        raise ValueError(f"{path}: expected an 'archetypes' object")
    return archetypes


def resolve(library: dict, name: str) -> str:
    """Canonical archetype name, case-insensitively.

    An unknown name raises ``LookupError`` **with near-misses**: a bare "not
    found" makes a caller guess, and a campaign dispatched at a misspelled
    archetype would otherwise resolve to nothing and report an empty plan as
    though the work were done.
    """
    if name in library:
        return name
    lowered = {k.lower(): k for k in library}
    if name.lower() in lowered:
        return lowered[name.lower()]
    close = get_close_matches(name, list(library), n=5, cutoff=0.6)
    hint = f" Did you mean: {', '.join(close)}?" if close else ""
    raise LookupError(f"no archetype named {name!r}.{hint}")


def _lists(entry: dict) -> list[list[str]]:
    """Every decklist as a list of card ids (duplicates preserved)."""
    out: list[list[str]] = []
    for dl in entry.get("decklists") or []:
        raw = dl.get("decklist")
        if not raw:
            continue
        if isinstance(raw, str):
            try:
                cards = json.loads(raw)
            except json.JSONDecodeError:
                continue
        else:
            cards = raw
        if isinstance(cards, list):
            out.append([c for c in cards if isinstance(c, str)])
    return out


def card_frequency(entry: dict) -> dict[str, int]:
    """``card_id -> how many LISTS contain it`` (copies within a list count once)."""
    counts: Counter[str] = Counter()
    for cards in _lists(entry):
        counts.update(set(cards))
    return dict(counts)


def pool(entry: dict) -> list[str]:
    """Every distinct card the archetype has played, sorted."""
    return sorted(card_frequency(entry))


def core(entry: dict, fraction: float = DEFAULT_CORE_FRACTION) -> dict:
    """The competitive core, plus the threshold and denominator it used.

    Returning the threshold and list count is not decoration: a report has to
    print ">=N of M lists", and a caller given only a card list would have to
    recompute them -- which is how a report ends up quoting a fraction it did
    not actually apply.
    """
    lists = _lists(entry)
    list_count = len(lists)
    threshold = int(-(-list_count * fraction // 1)) if list_count else 0  # ceil
    freq = card_frequency(entry)
    return {
        "cards": sorted(c for c, n in freq.items() if n >= threshold) if list_count else [],
        "threshold": threshold,
        "list_count": list_count,
        "fraction": fraction,
    }
```

- [ ] **Step 4: Run the tests**

Run: `python -m pytest code/tests/tools/test_clause_coverage_archetype.py -v`

Expected: PASS — 6 tests, including the real-library guard.

**If the Toho guard fails**, do not adjust the expected numbers to match. Either `deck_library.json` was re-scraped (in which case update the published report too, in the same commit, and say so) or the parsing is wrong. Silently re-baselining that test destroys the only link between this code and the published figures.

- [ ] **Step 5: Commit**

```bash
git add code/tools/clause_coverage/archetype.py code/tests/tools/test_clause_coverage_archetype.py
git commit -m "clause_coverage: resolve an archetype to its pool and competitive core

The core is a FRACTION of the archetype's recorded lists, not a raw count: the
published Toho report says '>=33 of 45', and hardcoding 33 would redefine the
core for any archetype with a different corpus size. 0.7 reproduces that figure
exactly -- 42-card pool, 18-card core, 45 lists -- and a test pins it against
the real library so a re-scrape fails loudly instead of letting a report quote
stale numbers.

decklist is a JSON-encoded STRING in this file, which is the thing that bites
anyone reading it for the first time."
```

---

### Task 2: The work plan

**Files:**
- Create: `code/tools/clause_coverage/campaign.py`, `code/tests/tools/test_clause_coverage_campaign.py`

**Interfaces:**
- Consumes: `archetype.{load_archetypes, resolve, pool, core}` (Task 1); `exam_binding.bind`; `card_sources.extract_card_clauses`.
- Produces:
  ```python
  def build_plan(archetype: str, *, library_path, cards_dir, scenarios_dir,
                 verdicts_path, core_fraction=0.7, limit=None) -> dict
  # {"archetype", "pool", "core", "implement", "exam", "skipped", "denominator"}
  def main(argv=None) -> int   # --archetype NAME [--json]
  ```

**The plan's job is to be honest about three different kinds of "not done":**
- `implement` — no YAML spec exists; the card cannot be examined yet.
- `exam` — YAML exists, clauses outstanding.
- `skipped` — confirmed (text unchanged), or `unavailable` because DCGO has no script. **Reported, never hidden**, with the reason.

- [ ] **Step 1: Write the failing tests**

Create `code/tests/tools/test_clause_coverage_campaign.py`:

```python
"""The work plan splits three kinds of 'not done' and hides none of them."""

import json

import pytest

from tools.clause_coverage.campaign import build_plan


@pytest.fixture
def workspace(tmp_path):
    """A miniature repo: one archetype, two cards, one with a YAML spec."""
    lib = tmp_path / "deck_library.json"
    lib.write_text(
        json.dumps(
            {
                "version": 1,
                "archetypes": {
                    "Fixtures": {
                        "archetype_name": "Fixtures",
                        "decklists": [
                            {"deck_id": "1", "decklist": json.dumps(["EX12-004", "BT8-084"])},
                            {"deck_id": "2", "decklist": json.dumps(["EX12-004"])},
                        ],
                    }
                },
            }
        ),
        encoding="utf-8",
    )

    cards = tmp_path / "cards" / "ex12"
    cards.mkdir(parents=True)
    (cards / "EX12-004.yaml").write_text("card: EX12-004\n", encoding="utf-8")
    # BT8-084 deliberately has NO yaml -> it must land in `implement`.

    verdicts = tmp_path / "exam-verdicts"
    verdicts.mkdir()

    scenarios = tmp_path / "scenarios"
    scenarios.mkdir()
    return {"library": lib, "cards": cards.parent, "verdicts": verdicts, "scenarios": scenarios}


def test_a_card_without_yaml_lands_in_implement(workspace):
    plan = build_plan(
        "Fixtures",
        library_path=workspace["library"],
        cards_dir=workspace["cards"],
        scenarios_dir=workspace["scenarios"],
        verdicts_path=workspace["verdicts"],
    )
    assert "BT8-084" in plan["implement"], "a card with no spec cannot be examined yet"
    assert "BT8-084" not in [c["card_id"] for c in plan["exam"]]


def test_the_plan_reports_its_denominator(workspace):
    plan = build_plan(
        "Fixtures",
        library_path=workspace["library"],
        cards_dir=workspace["cards"],
        scenarios_dir=workspace["scenarios"],
        verdicts_path=workspace["verdicts"],
    )
    d = plan["denominator"]
    for cls in ("confirmed", "diverged", "unreachable", "unavailable", "unmeasured"):
        assert cls in d["by_verdict"], f"missing {cls}: a card must never read as 'passed'"


def test_core_is_reported_with_its_threshold(workspace):
    plan = build_plan(
        "Fixtures",
        library_path=workspace["library"],
        cards_dir=workspace["cards"],
        scenarios_dir=workspace["scenarios"],
        verdicts_path=workspace["verdicts"],
    )
    assert plan["core"]["list_count"] == 2
    assert plan["core"]["threshold"] == 2  # ceil(2 * 0.7)
    assert plan["core"]["cards"] == ["EX12-004"]


def test_an_unknown_archetype_raises_with_suggestions(workspace):
    with pytest.raises(LookupError) as e:
        build_plan(
            "Fixture",  # near-miss
            library_path=workspace["library"],
            cards_dir=workspace["cards"],
            scenarios_dir=workspace["scenarios"],
            verdicts_path=workspace["verdicts"],
        )
    assert "Fixtures" in str(e.value)
```

- [ ] **Step 2: Run to verify failure**

Run: `python -m pytest code/tests/tools/test_clause_coverage_campaign.py -v`

Expected: FAIL — `ModuleNotFoundError: tools.clause_coverage.campaign`.

- [ ] **Step 3: Implement**

**Read `exam_binding.bind`'s signature and return shape first**, and `card_sources.extract_card_clauses`. Reuse them; do not re-derive a denominator here — one denominator, one producer.

Create `code/tools/clause_coverage/campaign.py`:

```python
"""Turn an archetype into a unit of dispatchable work.

Three kinds of "not done", kept apart because they cost different things:

- ``implement`` -- no YAML spec exists, so the card cannot be examined at all
  and the work is card authoring.
- ``exam`` -- a spec exists and clauses are outstanding; the work is scenario
  authoring against the oracle.
- ``skipped`` -- confirmed (with unchanged text) or ``unavailable`` because DCGO
  has no script for the card. **Reported with its reason, never hidden**: a plan
  that silently omits skipped work reads as though the work does not exist.

The denominator comes from `exam_binding.bind`, not from a second computation
here. One denominator, one producer.

Standard library only.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from tools.clause_coverage import archetype as archetype_mod
from tools.clause_coverage.exam_binding import bind


def _has_yaml(cards_dir: Path, card_id: str) -> bool:
    """Does a DSL spec exist for this card, in any set directory?"""
    return any(cards_dir.rglob(f"{card_id}.yaml"))


def build_plan(
    archetype: str,
    *,
    library_path: Path | str,
    cards_dir: Path | str,
    scenarios_dir: Path | str,
    verdicts_path: Path | str,
    core_fraction: float = archetype_mod.DEFAULT_CORE_FRACTION,
    limit: int | None = None,
) -> dict:
    """Build the work plan for one archetype."""
    library = archetype_mod.load_archetypes(library_path)
    canonical = archetype_mod.resolve(library, archetype)
    entry = library[canonical]

    cards_dir = Path(cards_dir)
    card_pool = archetype_mod.pool(entry)
    core = archetype_mod.core(entry, core_fraction)

    implement = [c for c in card_pool if not _has_yaml(cards_dir, c)]
    examinable = [c for c in card_pool if c not in implement]

    binding = bind(card_pool, scenarios_dir, verdicts_path)

    exam: list[dict] = []
    skipped: list[dict] = []
    for clause in binding.get("clauses", []):
        card_id = clause.get("card_id") or clause["clause_id"].split("#")[0]
        verdict = clause.get("verdict", "unmeasured")
        if card_id in implement:
            # Its clauses are real, but the card must be written first.
            continue
        if verdict == "confirmed":
            skipped.append({"clause_id": clause["clause_id"], "reason": "confirmed"})
            continue
        if verdict == "unavailable":
            skipped.append({
                "clause_id": clause["clause_id"],
                "reason": clause.get("reason") or "DCGO has no script for this card",
            })
            continue
        exam.append({
            "clause_id": clause["clause_id"],
            "card_id": card_id,
            "label": clause.get("label", ""),
            "verdict": verdict,
            "is_core": card_id in core["cards"],
        })

    # Core clauses first: the campaign's done-criterion is defined on the core,
    # so working the tail first would delay the only gate that matters.
    exam.sort(key=lambda c: (not c["is_core"], c["clause_id"]))
    exam_total = len(exam)
    if limit is not None:
        exam = exam[:limit]

    return {
        "archetype": canonical,
        "pool": card_pool,
        "examinable": examinable,
        "core": core,
        "implement": implement,
        "exam": exam,
        "exam_total": exam_total,
        "elided": exam_total - len(exam),
        "skipped": skipped,
        "denominator": binding.get("denominator", {}),
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archetype", required=True)
    parser.add_argument("--library", type=Path, default=Path("data/deck_library.json"))
    parser.add_argument("--cards-dir", type=Path, default=Path("code/digimon-engine/cards"))
    parser.add_argument("--scenarios-dir", type=Path, default=Path("qa/dcgo-exams"))
    parser.add_argument("--verdicts", type=Path, default=Path("qa/qa-reports/exam-verdicts"))
    parser.add_argument("--core-fraction", type=float, default=archetype_mod.DEFAULT_CORE_FRACTION)
    parser.add_argument("--limit", type=int, default=None)
    parser.add_argument("--json", action="store_true", help="machine-readable output")
    args = parser.parse_args(argv)

    plan = build_plan(
        args.archetype,
        library_path=args.library,
        cards_dir=args.cards_dir,
        scenarios_dir=args.scenarios_dir,
        verdicts_path=args.verdicts,
        core_fraction=args.core_fraction,
        limit=args.limit,
    )

    if args.json:
        print(json.dumps(plan, indent=2, sort_keys=True))
        return 0

    core = plan["core"]
    d = plan["denominator"].get("by_verdict", {})
    print(f"{plan['archetype']} — {len(plan['pool'])} cards, "
          f"core {len(core['cards'])} (>={core['threshold']} of {core['list_count']} lists)")
    print(f"  implement : {len(plan['implement'])} cards with no YAML spec")
    print(f"  exam      : {plan['exam_total']} outstanding clauses "
          f"({sum(1 for c in plan['exam'] if c['is_core'])} shown are core)")
    print(f"  skipped   : {len(plan['skipped'])} (confirmed or unavailable)")
    if d:
        print("  denominator: " + ", ".join(f"{k} {v}" for k, v in sorted(d.items())))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
```

- [ ] **Step 4: Run the tests**

Run: `python -m pytest code/tests/tools/test_clause_coverage_campaign.py -v`

Expected: PASS — 4 tests.

- [ ] **Step 5: Run it against the real library**

```bash
PYTHONPATH=code python -m tools.clause_coverage.campaign --archetype "Toho Braves"
PYTHONPATH=code python -m tools.clause_coverage.campaign --archetype "Hunters"
```

Expected: Toho Braves reports 42 cards and an 18-card core with a small `implement` list (the campaign already ran); Hunters reports a large `implement` list (~42 cards with no spec, per the prioritisation aid). Paste both outputs in your report.

**If Toho Braves shows a large `implement` list, stop** — `_has_yaml`'s glob is not finding specs that exist, and the plan would send an agent to re-author implemented cards.

- [ ] **Step 6: Commit**

```bash
git add code/tools/clause_coverage/campaign.py code/tests/tools/test_clause_coverage_campaign.py
git commit -m "clause_coverage: the archetype work plan

Three kinds of 'not done' kept apart because they cost different things: cards
with no spec (write them), clauses with a spec and no verdict (exam them), and
work skipped because it is confirmed or because DCGO has no script. Skipped
work is REPORTED with its reason -- a plan that omits it reads as though the
work does not exist.

Core clauses sort first, because the campaign's done-criterion is defined on
the core and working the tail first delays the only gate that matters."
```

---

### Task 3: Wire the index to real archetypes

**Files:**
- Modify: `code/tools/clause_coverage/exam_index.py`, `code/tests/tools/test_clause_coverage_exam_index.py`

**Interfaces:**
- Consumes: `archetype.load_archetypes`, `campaign.build_plan`.
- Produces: `main()` renders every archetype that has any exam activity, instead of an empty index.

**Context:** the ledger plan shipped `exam_index.main()` deliberately rendering an empty index, with a comment saying archetype resolution belonged to a later plan. This is that plan. Remove the placeholder comment along with the placeholder behaviour.

- [ ] **Step 1: Write the failing test**

Add to `code/tests/tools/test_clause_coverage_exam_index.py`:

```python
def test_main_renders_archetypes_that_have_exam_activity(tmp_path, monkeypatch):
    """The index exists to answer 'what should I dispatch next', which it
    cannot do while it renders nothing."""
    import json

    from tools.clause_coverage import exam_index

    lib = tmp_path / "deck_library.json"
    lib.write_text(json.dumps({
        "version": 1,
        "archetypes": {
            "Fixtures": {"archetype_name": "Fixtures", "decklists": [
                {"deck_id": "1", "decklist": json.dumps(["EX12-004"])}]}
        },
    }), encoding="utf-8")

    cards = tmp_path / "cards"
    cards.mkdir()
    verdicts = tmp_path / "exam-verdicts"
    verdicts.mkdir()
    scenarios = tmp_path / "scenarios"
    scenarios.mkdir()
    out = tmp_path / "exam-index.md"

    rc = exam_index.main([
        "--out", str(out), "--library", str(lib), "--cards-dir", str(cards),
        "--scenarios-dir", str(scenarios), "--verdicts", str(verdicts),
    ])
    assert rc == 0
    text = out.read_text(encoding="utf-8")
    assert "Fixtures" in text, "an archetype with a pool must appear in the index"
```

- [ ] **Step 2: Run to verify failure**

Run: `python -m pytest code/tests/tools/test_clause_coverage_exam_index.py -v`

Expected: FAIL — `main()` does not accept `--library` / renders no rows.

- [ ] **Step 3: Implement**

In `exam_index.py`, replace `main()`'s placeholder block (the comment beginning "Archetype -> card list resolution lands with the campaign skill") with real assembly:

```python
def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, default=Path("qa/qa-reports/exam-index.md"))
    parser.add_argument("--verdicts", type=Path, default=Path("qa/qa-reports/exam-verdicts"))
    parser.add_argument("--library", type=Path, default=Path("data/deck_library.json"))
    parser.add_argument("--cards-dir", type=Path, default=Path("code/digimon-engine/cards"))
    parser.add_argument("--scenarios-dir", type=Path, default=Path("qa/dcgo-exams"))
    parser.add_argument(
        "--min-clauses",
        type=int,
        default=1,
        help="skip archetypes whose pool yields fewer clauses than this",
    )
    args = parser.parse_args(argv)

    from tools.clause_coverage import archetype as archetype_mod
    from tools.clause_coverage.campaign import build_plan

    library = archetype_mod.load_archetypes(args.library)
    rows = []
    for name in sorted(library):
        try:
            plan = build_plan(
                name,
                library_path=args.library,
                cards_dir=args.cards_dir,
                scenarios_dir=args.scenarios_dir,
                verdicts_path=args.verdicts,
            )
        except (LookupError, ValueError):
            # An archetype the library lists but cannot be resolved into a pool
            # is skipped rather than rendered as zeros -- a row of zeros reads
            # as "measured and empty", which is the opposite of the truth.
            continue
        total = plan["denominator"].get("total_clauses", 0)
        if total < args.min_clauses:
            continue
        rows.append({"archetype": plan["archetype"], "cards": plan["pool"],
                     "binding": {"denominator": plan["denominator"],
                                 "total_clauses": total}})

    text = render_index(rows, generated_from=str(args.verdicts))
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(text, encoding="utf-8")
    print(f"wrote {args.out} ({len(rows)} archetypes)")
    return 0
```

**Note the shape passed to `render_index`:** `binding["denominator"]["total_clauses"]` — matching what `render_index` reads after the ledger plan's final fix. Do not reintroduce a top-level `total_clauses` as the only key.

- [ ] **Step 4: Run the tests and generate the real index**

```bash
python -m pytest code/tests/tools/test_clause_coverage_exam_index.py -v
PYTHONPATH=code python -m tools.clause_coverage.exam_index
head -20 qa/qa-reports/exam-index.md
```

Expected: tests PASS; the index lists real archetypes sorted by `unmeasured` descending. Paste the head in your report.

This walks 432 archetypes and binds each — if it is unusably slow, add `--only` to filter and say so, rather than leaving a generator nobody will run.

- [ ] **Step 5: Commit**

```bash
git add code/tools/clause_coverage/exam_index.py code/tests/tools/test_clause_coverage_exam_index.py qa/qa-reports/exam-index.md
git commit -m "clause_coverage: the index renders real archetypes

It shipped rendering an empty index with a comment saying archetype resolution
belonged to a later plan. This is that plan.

An archetype that cannot be resolved into a pool is skipped rather than
rendered as zeros: a row of zeros reads as 'measured and empty', which is the
opposite of the truth."
```

---

### Task 4: The `/archetype-campaign` skill

**Files:**
- Create: `.claude/skills/archetype-campaign/SKILL.md`

**Interfaces:**
- Consumes: everything above, plus `/batch-implement-cards-rust-dsl`, `/dcgo-exam`, and the MCP tools.
- Produces: the dispatchable workflow. No code.

**Read first:** `.claude/skills/dcgo-exam/SKILL.md`. This skill composes it and must not contradict it — especially its non-negotiables about `diverged`, the denominator, and `unavailable` being per-card.

- [ ] **Step 1: Write the skill**

Create `.claude/skills/archetype-campaign/SKILL.md`:

```markdown
---
name: archetype-campaign
description: Run a full archetype campaign on an oracle node — resolve the archetype to its card pool, implement the cards that have no YAML spec, exam the clauses that have no verdict against the DCGO oracle, triage divergences, and leave a ledger entry so no other node repeats the work. Triggers on "run a campaign on <archetype>", "do <archetype> end to end", "implement and exam <archetype>", dispatching an archetype as a job, or resuming a crashed campaign. Composes /batch-implement-cards-rust-dsl and /dcgo-exam; re-implements neither.
argument-hint: <archetype> [--exam-only] [--core-fraction 0.7]
---

# Archetype Campaign

You take **one archetype** and drive it to a stated finish line on a node that
has its own DCGO oracle. The output is a per-clause verdict table over the full
denominator plus a ledger entry — never "archetype done".

This is the dispatch unit the fleet is built around. Everything it needs is
resumable from the ledger, so a crashed campaign is re-dispatched, not restarted.

## Non-negotiables — read before acting

- **Preflight first.** Run `node_health` before authoring anything. Authoring
  costs real money; discovering afterwards that the oracle was never going to
  answer wastes all of it. A NO-GO stops the campaign — report it, do not
  author "in the meantime".
- **`--sim-only` is not confirmation.** It proves a line is legal in our engine
  and says nothing about DCGO's prompt sequence. Six sim-green scenarios were
  put to the oracle in the first campaign and **all six failed, every one on
  prompt sequence.** Only an oracle pass moves a clause to `confirmed`.
- **Always print the full denominator.** An archetype reads as
  `Hunters — 166 clauses: 107 confirmed, 0 diverged, 5 unreachable, 54 unmeasured`,
  never "Hunters passed". If anything is `unmeasured`, `unavailable`, or
  `unreachable`, say so in the **first sentence** of your summary.
- **`diverged` is a finding to TRIAGE, not proof our engine is wrong.**
  `general_rule.pdf` outranks DCGO. Read the printed card text and the rule
  before concluding anything.
- **Claim before you author.** Archetype pools overlap — one archetype's cards
  can be a strict subset of another's. Claiming is what stops two nodes doing
  the same card twice.
- **Never re-implement a card that already has a spec.** The plan's `implement`
  list is the authority; if it looks wrong, stop and check the resolver rather
  than authoring over existing work.

## Phase 0 — Preflight

```
node_health(build=<player dir>)
```

Every check must be `ok` or `warn`. On any `fail`, report the check and its
remedy and **stop**. The most common `fail` is `action_space`: the player
encodes against a dead action space and its recordings would read as engine
divergence. That needs a rebuild on the build machine, not a retry here.

## Phase 1 — Resolve and bind

```bash
PYTHONPATH=code python -m tools.clause_coverage.campaign --archetype "<NAME>" --json
```

Read four things before doing anything else:

- `core` — `{cards, threshold, list_count}`. **This is your finish line.**
- `implement` — cards with no YAML spec. They cannot be examined yet.
- `exam` — outstanding clauses, core-first.
- `skipped` — confirmed or `unavailable`, with reasons. Report these; never
  silently drop them.

An unknown archetype raises with near-misses. Use one of them; do not invent a
name.

## Phase 2 — Claim

```
claim(cards=[...the plan's implement + examinable cards...], job_id="<archetype>-<n>",
      archetype="<NAME>", node="<node name>")
```

Work only what was `granted`. For anything `held_by_others`, say who holds it in
your report — that is the fleet coordinating, not an error.

Claims are **advisory**: simultaneous pushes can both claim. If you find a
duplicate at merge, that is the known trade, not a bug.

## Phase 3 — Implement the missing cards

For the `implement` list, use **`/batch-implement-cards-rust-dsl`**. Do not
author cards inline here — that skill owns the TDD flow, the DSL-first rule, and
the verdict tracking, and duplicating it would drift.

Gaps route to their existing trackers: DSL vocabulary to `qa/dsl-vocab-gaps.md`,
engine primitives to `docs/RUST_ENGINE_GAPS.md`. **Widen the substrate rather
than routing around it** (CLAUDE.md rule 28) — a card implemented by
approximation fails the exam anyway, and now silently.

## Phase 4 — Exam, with the oracle in the loop

For each outstanding clause, core first:

1. `exam_keyword_brief(keyword)` for each keyword the clause's text prints. The
   **kind predicts the prompt shape**: `Opt-cost→Mand` means DCGO asks (the line
   needs an `expect:` row); `Mandatory` means no prompt at all (an `expect:` row
   there desynchronizes everything after it).
2. Compose the line. `exam_authoring_guide(topic)` for the part you need —
   `format`, `steps`, `prompts`, `decks`, `assert`, `verdicts`.
3. `exam_validate(yaml)` — milliseconds, catches the orphan clause id, an
   unstacked card, a prompt kind outside the 13.
4. `exam_probe(yaml, sim_only=true)` — does it lower?
5. `exam_probe(yaml, sim_only=false)` — **what does DCGO actually do?** This is
   the step that finds prompt-sequence mismatches, and it is why the oracle is
   on this machine.
6. Fix and repeat. Only commit the scenario file once the oracle agrees.

Then run the committed scenario through `/dcgo-exam` to record the verdict and
backfill the confirmed assertions.

## Phase 5 — Triage divergences

For each `diverged`, in this order:

1. The **printed card text** — the image / official bundle (`/digimon-card-lookup`).
2. The governing rule in `general_rule.pdf` (`/digimon-rules`). **The PDF
   outranks DCGO.**
3. The DCGO C# at `$BASE_DCGO/Assets/Scripts/CardEffect/<SET>/<COLOR>/<CARD_ID>.cs`
   (underscored filename).

Classify: **our bug** / **DCGO quirk** / **rules-ambiguous**.

A fix may land autonomously **only** with all three of: a citation to the rule §
or DCGO C# it rests on; a test that fails before and passes after; and
`cards_behavioral` green. **Card/YAML fixes** proceed under that gate. **Engine
fixes** proceed under it but land on their own branch and are flagged for human
review. Anything you cannot justify by citation is a **logged finding, not a
fix**.

## Phase 6 — Done, reported, and released

The finish line is **not** "every clause confirmed":

> **Every core clause is adjudicated** — `confirmed`, or carrying a named,
> *measured* reason — **and zero untriaged `diverged`.**

Pool coverage is reported, not gated. The tail of 1-of tech cards is real work
but it is not this campaign's gate; grinding it at $8+/clause is how a campaign
never ends.

Then:
- Append one line per attempt to `qa/qa-reports/exam-log.jsonl`.
- Regenerate the index: `python -m tools.clause_coverage.exam_index`.
- `release(cards=[...], job_id=...)`.
- Report the table, denominator first.

## Red flags — STOP

- About to author before `node_health` returns GO → STOP.
- About to treat a sim-green scenario as confirmation → STOP. Only the oracle confirms.
- About to report the archetype without printing `unmeasured` → STOP.
- About to call a `diverged` clause an engine bug before reading `general_rule.pdf` → STOP.
- About to re-implement a card that already has a YAML spec → STOP; check the resolver.
- About to fix the engine without a rule citation and a failing-then-passing test → STOP.
- About to grind the support-card tail while a core clause is unmeasured → STOP. Core first.

## Reference

- Fleet design: `docs/superpowers/specs/2026-08-27-archetype-campaign-fleet-design.md`
- Exam manual: `docs/DCGO_EXAM.md`; node runbook: `docs/runbooks/oracle-node.md`
- Composes: `/batch-implement-cards-rust-dsl`, `/dcgo-exam`
- What to dispatch next: `qa/qa-reports/exam-index.md`, and the ranked shortlist
  in `docs/superpowers/specs/2026-08-22-unimplemented-winning-decks.md`
```

- [ ] **Step 2: Verify the skill registers**

The skill list is refreshed by the harness on change. Confirm the file parses as
frontmatter + body and that `name:` matches its directory:

```bash
head -5 .claude/skills/archetype-campaign/SKILL.md
ls .claude/skills/archetype-campaign/
```

Expected: `name: archetype-campaign` on line 2, directory `archetype-campaign`.

- [ ] **Step 3: Check it against the skill it composes**

Read `.claude/skills/dcgo-exam/SKILL.md` and confirm this skill contradicts none
of its non-negotiables — particularly: `diverged` is a finding, the denominator
is always printed, and `unavailable` is per card rather than per set. Report any
contradiction you found and fixed.

- [ ] **Step 4: Commit**

```bash
git add .claude/skills/archetype-campaign/SKILL.md
git commit -m "skill: /archetype-campaign -- an archetype as a dispatchable job

The finish line is defined on the competitive core, not the pool: an archetype
always has a tail of 1-of tech cards, and without a stated gate an agent grinds
it at \$8+/clause and never finishes.

Phase 4 puts the oracle in the authoring loop rather than after it, because
sim-green is not confirmation -- six sim-green scenarios went to the oracle in
the first campaign and all six failed on prompt sequence."
```

---

### Task 5: Register the workflow

**Files:**
- Modify: `CLAUDE.md`, `docs/DCGO_EXAM.md`, `docs/INDEX.md`

- [ ] **Step 1: Add the campaign to CLAUDE.md**

In the section that lists the DCGO exam (around line 406), add a sentence naming
`/archetype-campaign` as the dispatch unit that composes `/batch-implement-cards-rust-dsl`
and `/dcgo-exam`, and pointing at `qa/qa-reports/exam-index.md` for what to
dispatch next. Match the surrounding voice; do not restate the exam's own rules.

- [ ] **Step 2: Add a "Running a campaign" section to `docs/DCGO_EXAM.md`**

Cover: the one-line dispatch, the three-way split of the work plan, the
core-based done-criterion with Toho's 69/74 as the worked example, and the
fix gate. Link the node runbook.

- [ ] **Step 3: Index the new plans**

Add the three plan documents to `docs/INDEX.md` beside the existing planning
docs, so the four-plan arc is discoverable from one place.

- [ ] **Step 4: Verify no doc contradicts the shipped behaviour**

```bash
grep -rn 'archetype-campaign' --include=*.md . | grep -v '\.superpowers/'
```

Every hit must describe behaviour that now exists. Report anything aspirational
you found and either implemented or reworded.

- [ ] **Step 5: Commit**

```bash
git add CLAUDE.md docs/DCGO_EXAM.md docs/INDEX.md
git commit -m "docs: register /archetype-campaign as the dispatch unit"
```

---

## Self-Review

**Spec coverage** (`2026-08-27-archetype-campaign-fleet-design.md` §2):

| Spec requirement | Task |
|---|---|
| §2 Phase 0 preflight | 4 (skill), node plan (tool) |
| §2 Phase 1 resolve + bind, two kinds of work | 1, 2 |
| §2 Phase 1 skip confirmed by construction | 2 |
| §2 Phase 2 claim | 4 (skill), MCP plan (tool) |
| §2 Phase 3 implement wave via the existing skill | 4 |
| §2 Phase 4 exam with the oracle in the loop | 4 |
| §2 Phase 5 triage, §4 fix gate | 4 |
| §2 Phase 6 report / log / index / push | 3, 4 |
| §2.1 done-criterion on the core, fraction not raw count | 1, 4 |
| §1.4 index generated from the ledger | 3 |

**Deliberately not built:** keyword *tagging* inside `exam_plan`'s payload (MCP plan Task 2 left this to the campaign). The skill has the agent call `exam_keyword_brief` per clause instead — same routing, one fewer join, and it keeps the plan payload small, which was the point of that tool. If per-clause tagging later proves worth the join, `campaign.build_plan` is where it goes.

**Type consistency:** `core()` returns `{cards, threshold, list_count, fraction}` in Task 1 and is consumed with those exact keys in Task 2's `build_plan` and its tests. `build_plan`'s return keys (`archetype`, `pool`, `examinable`, `core`, `implement`, `exam`, `exam_total`, `elided`, `skipped`, `denominator`) are used unchanged by Task 3's index wiring. Task 3 passes `binding["denominator"]["total_clauses"]`, matching `render_index` after the ledger plan's final fix — not the top-level key that fix removed.

**Ordering:** Task 1 → Task 2 (imports it) → Task 3 (imports both). Task 4 needs 1–3 to exist for its commands to work. Task 5 last.
