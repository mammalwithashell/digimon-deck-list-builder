# Oracle Readiness Plan 4 — Scenario Library, Lean Skills, Cost Reporting, and the Readiness Batch

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make each exam cheaper by reusing proven deck books and line shapes, rewrite the exam skill around the new tools, add a `/readiness-batch` dispatch skill driven by the readiness plan, and measure every run's cost so savings are proven, not assumed.

**Architecture:** A stdlib Python tool reports per-phase token cost from subagent transcripts. A per-archetype library (`qa/dcgo-exams/_library/<slug>/`) holds deck books and a `LINES.md` of proven shapes. `/dcgo-exam` gains a tool-first fast path; `/readiness-batch` takes cards from `readiness --plan`, groups them by archetype, runs `/dcgo-exam` per card on Sonnet 5.5 subagents, records cost, and regenerates readiness.

**Tech Stack:** Python 3.11+ stdlib, pytest, Markdown skills under `.claude/skills/`.

**Spec:** `docs/superpowers/specs/2026-10-04-dcgo-oracle-readiness-design.md` §4.5, §4.8 (dispatch unit), §4.9, §4.10, §6. Depends on Plans 1–3 (tools the skills call) and Plan 2 (`--plan`).

## Global Constraints

- Worktree only; verify `git rev-parse --show-toplevel`.
- Subagent transcripts live at `C:/Users/james/.claude/projects/C--Users-james-Documents-digimon-deck-list-builder-1/<session-id>/subagents/agent-<id>.jsonl`; the task `.output` file is empty and must not be used.
- Price table default (USD per million tokens, Claude Sonnet 5.5): input 2.00, cache write 2.50, cache read 0.20, output 10.00. Configurable; never hard-code elsewhere.
- Success targets (spec §6): median ≤ $0.75 per adjudicated clause; ≤ 1.3 oracle round-trips per confirmed clause. Track A baseline: $1.72 and 2.5.
- Worker model for exam subagents: Claude Sonnet 5.5 (`model: "sonnet"` in the Agent tool).
- Honesty rules carry over unchanged: `confirmed` only from an oracle diff; the full denominator is always printed.
- Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## File Map

| File | Change | Responsibility |
|---|---|---|
| `code/tools/exam_cost_report.py` | Create | Per-phase turns/tools/tokens/USD from a transcript |
| `code/tests/tools/test_exam_cost_report.py` | Create | Parser tests on a synthetic transcript |
| `qa/dcgo-exams/_library/README.md` | Create | Library contract |
| `qa/dcgo-exams/_library/rocks/decks.json` | Create (moved) | Rocks deck book (superset of the two Track A books) |
| `qa/dcgo-exams/_library/rocks/LINES.md` | Create | Proven Rocks line shapes |
| `qa/dcgo-exams/EX10/rocks_pool*.json` | Delete (after move) | Replaced by the library |
| `qa/dcgo-exams/EX10/NOTES-*.md`, `EX8/NOTES-EX8-067.md` | Modify | Point at the library path |
| `.claude/skills/dcgo-exam/SKILL.md` | Modify | Tool-first fast path |
| `.claude/skills/readiness-batch/SKILL.md` | Create | The dispatch unit |
| `qa/qa-reports/readiness-batch-log.md` | Create | Per-run cost and outcome log |

---

### Task 1: `exam_cost_report.py`

**Files:**
- Create: `code/tools/exam_cost_report.py`
- Create: `code/tests/tools/test_exam_cost_report.py`

**Interfaces:**
- Produces:
  - `PRICES_SONNET_5_5 = {"input": 2.0, "cache_write": 2.5, "cache_read": 0.2, "output": 10.0}`
  - `report(path: Path, prices: dict = PRICES_SONNET_5_5) -> dict` → `{"phases": [{"phase", "turns", "tool_calls", "input", "cache_write", "cache_read", "output", "usd"}], "total": {...same keys minus phase...}, "model": str | None}`
  - CLI: `python code/tools/exam_cost_report.py <transcript.jsonl> [--json] [--clauses N]` (with `--clauses`, also prints USD per clause).

- [ ] **Step 1: Write the failing tests**

Create `code/tests/tools/test_exam_cost_report.py`:

```python
import json

from tools.exam_cost_report import PRICES_SONNET_5_5, report


def _assistant(mid, usage, tools=(), model="claude-sonnet-5-5"):
    content = [{"type": "tool_use", "name": name, "input": inp} for name, inp in tools]
    return {"message": {"role": "assistant", "id": mid, "model": model,
                        "usage": usage, "content": content}}


def _write(tmp_path, rows):
    p = tmp_path / "agent.jsonl"
    p.write_text("\n".join(json.dumps(r) for r in rows) + "\n", encoding="utf-8")
    return p


def test_phases_split_on_markers_and_streamed_output_is_counted_once(tmp_path):
    u = {"input_tokens": 10, "cache_creation_input_tokens": 100,
         "cache_read_input_tokens": 1000, "output_tokens": 5}
    rows = [
        _assistant("m1", u, [("Bash", {"command": 'echo "TRACKA-PHASE P0-DENOMINATOR"'})]),
        _assistant("m2", u, [("Read", {"file_path": "x"})]),
        # streaming: same id, growing output count -> counted as 50 total, one turn
        _assistant("m2", {**u, "output_tokens": 50}),
        _assistant("m3", u, [("Bash", {"command": 'echo "TRACKA-PHASE P4-ORACLE"'})]),
        {"message": {"role": "user", "content": "tool result"}},
    ]
    r = report(_write(tmp_path, rows))
    phases = {p["phase"]: p for p in r["phases"]}
    assert phases["P0-DENOMINATOR"]["turns"] == 2
    assert phases["P0-DENOMINATOR"]["output"] == 5 + 50
    assert phases["P0-DENOMINATOR"]["tool_calls"] == 2
    assert phases["P4-ORACLE"]["turns"] == 1
    assert r["total"]["turns"] == 3
    assert r["model"] == "claude-sonnet-5-5"


def test_usd_uses_the_price_table(tmp_path):
    u = {"input_tokens": 1_000_000, "cache_creation_input_tokens": 0,
         "cache_read_input_tokens": 1_000_000, "output_tokens": 1_000_000}
    r = report(_write(tmp_path, [_assistant("m1", u)]))
    expected = PRICES_SONNET_5_5["input"] + PRICES_SONNET_5_5["cache_read"] + PRICES_SONNET_5_5["output"]
    assert abs(r["total"]["usd"] - expected) < 1e-9
```

- [ ] **Step 2: Run to verify failure**

Run: `python -m pytest code/tests/tools/test_exam_cost_report.py -v`
Expected: FAIL with `ModuleNotFoundError: No module named 'tools.exam_cost_report'`.

- [ ] **Step 3: Implement**

Create `code/tools/exam_cost_report.py`:

```python
"""Per-phase cost of an exam subagent run, from its transcript.

Phases are delimited by Bash tool calls that echo `TRACKA-PHASE <NAME>`.
Streaming writes several rows per assistant message id; input/cache usage is
taken from the first row, output from the largest count seen. Message content
is never printed.

    python code/tools/exam_cost_report.py <agent-xxxx.jsonl> [--json] [--clauses N]
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

PRICES_SONNET_5_5 = {"input": 2.0, "cache_write": 2.5, "cache_read": 0.2, "output": 10.0}
_MARK = re.compile(r"TRACKA-PHASE\s+([A-Z0-9-]+)")
_KEYS = ("turns", "tool_calls", "input", "cache_write", "cache_read", "output")


def _blank() -> dict:
    return {k: 0 for k in _KEYS}


def _usd(s: dict, prices: dict) -> float:
    return sum(s[k] * prices[k] for k in ("input", "cache_write", "cache_read", "output")) / 1e6


def report(path: Path, prices: dict = PRICES_SONNET_5_5) -> dict:
    phase, order, stats = "PRE", ["PRE"], {"PRE": _blank()}
    seen_out: dict[str, tuple[str, int]] = {}
    model = None
    for line in Path(path).read_text(encoding="utf-8").splitlines():
        try:
            row = json.loads(line)
        except json.JSONDecodeError:
            continue
        msg = row.get("message") or {}
        if msg.get("role") != "assistant":
            continue
        model = model or msg.get("model")
        content = msg.get("content") if isinstance(msg.get("content"), list) else []
        for c in content:
            if c.get("type") == "tool_use":
                m = _MARK.search(json.dumps(c.get("input", {})))
                if m:
                    phase = m.group(1)
                    if phase not in stats:
                        stats[phase] = _blank()
                        order.append(phase)
        usage = msg.get("usage") or {}
        mid = msg.get("id")
        out = usage.get("output_tokens", 0) or 0
        if mid not in seen_out:
            s = stats[phase]
            s["turns"] += 1
            s["input"] += usage.get("input_tokens", 0) or 0
            s["cache_write"] += usage.get("cache_creation_input_tokens", 0) or 0
            s["cache_read"] += usage.get("cache_read_input_tokens", 0) or 0
            s["output"] += out
            seen_out[mid] = (phase, out)
        else:
            ph, prev = seen_out[mid]
            if out > prev:
                stats[ph]["output"] += out - prev
                seen_out[mid] = (ph, out)
        stats[phase]["tool_calls"] += sum(1 for c in content if c.get("type") == "tool_use")

    phases = []
    total = _blank()
    for p in order:
        s = stats[p]
        if p == "PRE" and not any(s.values()):
            continue
        phases.append({"phase": p, **s, "usd": round(_usd(s, prices), 6)})
        for k in _KEYS:
            total[k] += s[k]
    total["usd"] = round(_usd(total, prices), 6)
    return {"phases": phases, "total": total, "model": model}


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("transcript", type=Path)
    ap.add_argument("--json", action="store_true")
    ap.add_argument("--clauses", type=int, default=0, help="adjudicated clauses, for $/clause")
    args = ap.parse_args(argv)
    r = report(args.transcript)
    if args.json:
        print(json.dumps(r, indent=2))
        return 0
    print(f"model: {r['model']}")
    print(f"{'phase':<16}{'turns':>6}{'tools':>6}{'c_read':>12}{'out':>9}{'USD':>8}")
    for p in r["phases"]:
        print(f"{p['phase']:<16}{p['turns']:>6}{p['tool_calls']:>6}{p['cache_read']:>12}"
              f"{p['output']:>9}{p['usd']:>8.2f}")
    t = r["total"]
    print(f"{'TOTAL':<16}{t['turns']:>6}{t['tool_calls']:>6}{t['cache_read']:>12}{t['output']:>9}{t['usd']:>8.2f}")
    if args.clauses:
        print(f"USD per clause: {t['usd'] / args.clauses:.2f}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
```

- [ ] **Step 4: Run tests and a real transcript**

Run: `python -m pytest code/tests/tools/test_exam_cost_report.py -v`
Expected: PASS.

Run: `python code/tools/exam_cost_report.py C:/Users/james/.claude/projects/C--Users-james-Documents-digimon-deck-list-builder-1/d8944ebc-1a72-440f-8dbf-2e44a5bc4445/subagents/agent-acd1e79c5a3569309.jsonl --clauses 2`
Expected: TOTAL `$4.38`, `USD per clause: 2.19` (the Track A Sunarizamon pilot). If that transcript has been cleaned up, skip this check and say so in the commit message.

- [ ] **Step 5: Commit**

```bash
git add code/tools/exam_cost_report.py code/tests/tools/test_exam_cost_report.py
git commit -m "tools: exam_cost_report -- per-phase cost of an exam subagent transcript" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: The Rocks scenario library

**Files:**
- Create: `qa/dcgo-exams/_library/README.md`, `qa/dcgo-exams/_library/rocks/decks.json`, `qa/dcgo-exams/_library/rocks/LINES.md`
- Delete: `qa/dcgo-exams/EX10/rocks_pool.json`, `qa/dcgo-exams/EX10/rocks_pool_ex10_028.json`
- Modify: NOTES files and plans that reference the old paths

- [ ] **Step 1: Confirm the larger book is a superset**

Run: `python -c "import json;a=json.load(open('qa/dcgo-exams/EX10/rocks_pool.json',encoding='utf-8'))['decks'];b=json.load(open('qa/dcgo-exams/EX10/rocks_pool_ex10_028.json',encoding='utf-8'))['decks'];bn={d['name']:d for d in b};print(all(bn.get(d['name'])==d for d in a), sorted(bn))"`
Expected: `True ['rocks-exam', 'rocks-exam-tumble']`. If `False`, merge by hand so both deck names exist with the entries each Track A scenario used, and note the difference in `LINES.md`.

- [ ] **Step 2: Move into the library**

```bash
mkdir -p qa/dcgo-exams/_library/rocks
git mv qa/dcgo-exams/EX10/rocks_pool_ex10_028.json qa/dcgo-exams/_library/rocks/decks.json
git rm qa/dcgo-exams/EX10/rocks_pool.json
```

Then replace every reference: `grep -rln "rocks_pool" qa docs .claude` and change `qa/dcgo-exams/EX10/rocks_pool.json` and `qa/dcgo-exams/EX10/rocks_pool_ex10_028.json` to `qa/dcgo-exams/_library/rocks/decks.json` (in NOTES files, plan files, and the spec).

Run: `D:/cargo-target/dcgo-effect-translation/debug/dcgo-harness.exe exam --scenario qa/dcgo-exams/EX10 --sim-only --cards-json data/cards.json --decks qa/dcgo-exams/_library/rocks/decks.json`
Run: `D:/cargo-target/dcgo-effect-translation/debug/dcgo-harness.exe exam --scenario qa/dcgo-exams/EX8 --sim-only --cards-json data/cards.json --decks qa/dcgo-exams/_library/rocks/decks.json`
Expected: every scenario lowers and runs (same results as before the move).

- [ ] **Step 3: Write the library contract**

Create `qa/dcgo-exams/_library/README.md`:

```markdown
# DCGO exam scenario library

One directory per archetype slug: `decks.json` (a deck book for `--decks`)
and `LINES.md` (proven line shapes). Read it BEFORE authoring an exam for a
card in that archetype; reusing a proven shape cut the second Rocks card's
exam cost by ~37% in Track A (2026-10-04).

Rules:
- Every shape in `LINES.md` cites the scenario that proved it against the oracle.
- A run that proves a new shape appends it (with the citation) in the same commit.
- Deck names are shared across scenarios; add a NEW named deck rather than
  editing one an existing scenario uses.
```

- [ ] **Step 4: Write `LINES.md` for Rocks**

Create `qa/dcgo-exams/_library/rocks/LINES.md`:

```markdown
# Rocks — proven exam line shapes

Deck book: `decks.json` — `rocks-exam` (base) and `rocks-exam-tumble`
(adds Tumblemon EX10-003 as a trash source with no "when trashed" trigger).

## Eggs keep the base off the Mineral/Rock list
Use ST5-01 eggs. A Mineral/Rock Lv.2 under the line makes "trash 1 Mineral/Rock
source" prompts ambiguous. Proven: `qa/dcgo-exams/EX10/EX10-025-effect0.yaml`.

## Fill the trash with exactly N Mineral/Rock cards
Play Sunarizamon EX11-038 from hand; its cost trashes one Mineral/Rock hand card.
DCGO first asks a zone menu that we do not model: author it as a `dcgo_only`
row `{ select: { value: 0 } }` before the hand pick. Repeat for N cards.
Proven: `qa/dcgo-exams/EX10/EX10-025-effect0.yaml`, `qa/dcgo-exams/EX8/EX8-067-effect1.yaml`.

## Reach a Lv.4 Rocks Digimon by turn 3
Sunarizamon digivolves from the breeding area at cost 0; keep the T1/T3/T5
memory timeline from the EX10-025 line. Proven: `qa/dcgo-exams/EX10/EX10-025-effect0.yaml`.

## Trash a source without firing an inherited trigger
Use Tumblemon EX10-003 (deck `rocks-exam-tumble`) as the source to trash; it
has no "when trashed from digivolution cards" effect, so the line measures the
trashing card's own clause. Proven: `qa/dcgo-exams/EX10/EX10-028-effect0.yaml`.

## Put a card on top of security
`stack[9]` is the top security card at game start. Proven:
`qa/dcgo-exams/EX8/EX8-067-effect2.yaml` (shape from EX12-070).

## Start a turn at exactly 2 memory
Opponent plays two cost-3 Ukkomon BT16-082 on its turn. Proven:
`qa/dcgo-exams/EX8/EX8-067-effect0.yaml`.

## DCGO batches a second [When Digivolving] we do not
When Close EX8-067's digivolve trigger and Landramon EX8-048's [When Digivolving]
fire together, DCGO asks `MultipleSkills`; author a `dcgo_only` row choosing
Close, and a `dcgo_only` decline for Landramon afterwards. Proven:
`qa/dcgo-exams/EX8/EX8-067-effect1.yaml` (row shape from BT24-017).

## Known engine divergence (until Plan 5 lands)
Source-trash triggers resolve mid-effect in our engine (§15-8-3-2 says they
wait). Lines through an inherited "when trashed from sources" clause need
paired `sim_only`/`dcgo_only` rows today; remove them once
`G-ENGINE-INLINE-DRAIN-SIBLING-TRIGGERS` is resolved.
Proven: `qa/dcgo-exams/EX10/EX10-025-inherited0.yaml`, `EX10-028-inherited0.yaml`.
```

- [ ] **Step 5: Commit**

```bash
git add qa/dcgo-exams/_library qa/dcgo-exams/EX10 qa/dcgo-exams/EX8 docs/superpowers
git commit -m "exam: per-archetype scenario library, seeded with the Rocks Track A shapes" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Tool-first fast path in `/dcgo-exam`

**Files:**
- Modify: `.claude/skills/dcgo-exam/SKILL.md` (insert after the frontmatter and title, before `## Non-negotiables — read before acting`)

- [ ] **Step 1: Insert the section**

```markdown
## Fast path (2026-10, use this first)

Every turn re-reads your whole context, so cost is turns × context. Use the
tools; do not reconstruct by hand what they return.

1. **Denominator:** `exam_status { card }` — extracts the card into the clause
   book if it is new (`extracted_now`). Note every clause id.
2. **Library:** read `qa/dcgo-exams/_library/<archetype>/LINES.md` and use its
   `decks.json` for `--decks`. Reuse a proven shape before inventing one.
3. **Unavailable?** No `DCGO/Assets/Scripts/CardEffect/<SET>/*/<ID_>.cs` in the
   BASE repo → every clause `unavailable`; record and stop.
4. **Author** one scenario per clause under `qa/dcgo-exams/<SET>/`.
5. **Lint:** `exam_validate { yaml, decks }` until clean (deck budget, verbs,
   prompt kinds, unstacked cards).
6. **Sim:** `exam_probe { yaml }`; when a row misbehaves,
   `exam_probe { yaml, inspect_step: N }` shows the live prompt and board.
7. **Oracle (one call):** `exam_probe { yaml, sim_only: false }` or
   `dcgo-harness exam --oracle ...` — submits, waits for THIS job's result,
   diffs, records the verdict, backfills asserts. Max 3 oracle round-trips per
   clause; then record `unmeasured` with the exact mismatch.
8. **Divergence:** triage with a citation and record it:
   `dcgo-harness verdict-triage --clause <ID> --triage ours_wrong|dcgo_quirk|undetermined --citation "<rule §, DCGO file:line, or gap id>"`.
   Never fix the engine from this skill.
9. **Library update:** if you proved a new shape, append it to `LINES.md` with
   the scenario path.
10. **Phase markers:** when run inside `/readiness-batch`, `echo "TRACKA-PHASE <NAME>"`
    at each step boundary (P0-DENOMINATOR, P1-RESEARCH, P2-AUTHOR, P3-SIM,
    P4-ORACLE, P5-TRIAGE, P6-RECORD) so `exam_cost_report.py` can attribute cost.

The phases below remain the reference for WHY each step exists.
```

- [ ] **Step 2: Commit**

```bash
git add .claude/skills/dcgo-exam/SKILL.md
git commit -m "skill: /dcgo-exam tool-first fast path" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: `/readiness-batch` skill

**Files:**
- Create: `.claude/skills/readiness-batch/SKILL.md`
- Create: `qa/qa-reports/readiness-batch-log.md`

- [ ] **Step 1: Write the skill**

Create `.claude/skills/readiness-batch/SKILL.md`:

```markdown
---
name: readiness-batch
description: Grow the oracle-ready training pool. Takes the top N cards from `readiness --plan` (ranked by how many decklists each unblocks), groups them by archetype, examines each against the DCGO oracle with /dcgo-exam on Sonnet subagents, records per-card cost, triages divergences, and regenerates data/oracle_readiness.json. Triggers on "run a readiness batch", "unblock more training decks", "examine the next N cards", "grow the training pool". Composes /dcgo-exam; never re-implements it.
---

# Readiness batch

Training only uses oracle-ready cards (CLAUDE.md rule 34). This skill turns the
readiness plan into exams, one bounded batch at a time.

## Non-negotiables
- Preflight the oracle FIRST: `node_health`. NO-GO → stop; never author "in the meantime".
- `confirmed` only from an oracle diff. Print every card's full denominator.
- Divergences are triaged with a citation (`verdict-triage`), not fixed here.
  Engine fixes go through the campaign fix gate on their own branch.
- One worker per card, Sonnet 5.5 (`model: "sonnet"`), dispatched in waves of
  at most 3 (the oracle is one queue; more workers only wait).
- Workers verify `git rev-parse --show-toplevel` ends in this worktree.

## Steps
1. `node_health` → GO.
2. `PYTHONPATH=code python -m tools.clause_coverage.readiness --plan --limit <N> --json`
   (default N = 6). Keep the picks in plan order.
3. Group picks by archetype (`python -m tools.clause_coverage.campaign --archetype <A> --json`
   resolves membership); a card with a YAML gap goes to `/batch-implement-cards-rust-dsl` first.
4. `claim { cards, job_id: "readiness-<date>-<n>", archetype }`.
5. For each card, dispatch a worker with: the card id and name, its outstanding
   clause ids from the plan, the library path `qa/dcgo-exams/_library/<slug>/`,
   the instruction to follow `/dcgo-exam`'s **Fast path** with phase markers,
   and the harness binary path + root. Collect each worker's denominator line.
6. For each finished worker, run
   `python code/tools/exam_cost_report.py <transcript> --clauses <adjudicated>`
   (transcripts: `~/.claude/projects/<project>/<session>/subagents/agent-<id>.jsonl`).
7. `PYTHONPATH=code python -m tools.clause_coverage.readiness` then `--check`.
   Report decklists ready before → after.
8. Append one row per card to `qa/qa-reports/readiness-batch-log.md`.
9. `release { cards, job_id }`. Commit scenarios, verdicts, library updates,
   the readiness artifact and the log together.

## Report
- Per card: denominator line, oracle round-trips, $/clause.
- Batch: median $/clause vs the $0.75 target, round-trips per confirmed clause
  vs 1.3, decklists ready before → after, divergences with their triage.
```

Create `qa/qa-reports/readiness-batch-log.md`:

```markdown
# Readiness batch log

One row per examined card. Cost from `code/tools/exam_cost_report.py`
(Sonnet 5.5 list price). Targets: ≤ $0.75 per adjudicated clause,
≤ 1.3 oracle round-trips per confirmed clause.

| Date | Batch | Card | Clauses | Confirmed | Diverged | Unreachable | Unmeasured | Round-trips | USD | USD/clause |
|---|---|---|---|---|---|---|---|---|---|---|
| 2026-10-04 | track-a | EX10-025 Sunarizamon | 2 | 1 | 1 | 0 | 0 | 3 | 4.38 | 2.19 |
| 2026-10-04 | track-a | EX10-028 Landramon | 2 | 1 | 1 | 0 | 0 | 2 | 2.76 | 1.38 |
| 2026-10-04 | track-a | EX8-067 Close | 3 | 2 | 0 | 0 | 1 | 5 | 4.90 | 1.63 |
```

- [ ] **Step 2: Commit**

```bash
git add .claude/skills/readiness-batch/SKILL.md qa/qa-reports/readiness-batch-log.md
git commit -m "skill: /readiness-batch -- examine the cards that unblock the most training decks" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Pilot batch and measurement

Requires Plans 1–3 landed and the oracle node GO.

- [ ] **Step 1: Run one batch**

Invoke `/readiness-batch` with N = 3.
Expected: three cards examined, three rows appended to the log, the readiness artifact regenerated and `--check` clean.

- [ ] **Step 2: Compare against targets**

From the log, compute median USD/clause and round-trips per confirmed clause for the batch.
Expected: both reported next to the Track A baseline. If median USD/clause > $0.75, list the top two phases by cost from the cost reports and open one follow-up task per phase in the spec's §9 table ("revisit when").

- [ ] **Step 3: Commit**

```bash
git add qa/ data/oracle_readiness.json
git commit -m "readiness: first measured batch (<median $/clause>, <round-trips/confirmed>)" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
