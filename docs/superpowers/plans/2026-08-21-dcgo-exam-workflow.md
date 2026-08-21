# DCGO Exam Workflow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn scenario runs into a measurable per-(card, clause) verdict, backfill confirmed oracle state into scenario assertions so they survive into CI, and put the whole thing behind an agent surface.

**Architecture:** A verdict store keyed by `clause_coverage` clause id, fed by the differ from plan 2. Confirmed verdicts backfill `assert:` blocks, which a Unity-free GitHub workflow then gates every PR on. The agent surface is a subcommand of the existing `dcgo-harness` binary, so every behavior stays unit-testable with no MCP client.

**Tech Stack:** Rust 2021 (verdict store, backfill, drafter, MCP subcommand), Python 3 (clause binding, reusing `code/tools/clause_coverage/`), GitHub Actions.

## Global Constraints

- **Prerequisite:** plans 1 and 2. Tasks 1 and 6 can start early; Tasks 2–5 need plan 2's `DiffReport`.
- **Per-worktree `CARGO_TARGET_DIR`** (rule 31). Prefix cargo commands with `CARGO_TARGET_DIR='D:\cargo-target-wt\bold-bassi-d34dc7'` if the harness inherited a stale env.
- **Clause ids are `{card_id}#{zone}#{idx}`** from `clause_coverage.models.Clause.id`. The exam never invents its own clause identity.
- **DCGO is source-priority #2, below `general_rule.pdf`.** A `diverged` verdict is a finding to triage, never proof we are wrong; a drafted test records evidence, never truth.
- **Always print the full denominator.** Every report states `unmeasured` alongside `confirmed`.
- `dcgo-harness` is dev/test tooling: never imported by `server.*` or `digimon_gym.*`, never bundled into a production build.
- Python tools are standard-library only, matching the rest of `code/tools/`.

## File Structure

| File | Responsibility |
|---|---|
| `code/tools/dcgo-harness/src/exam/verdict.rs` (create) | The verdict store: read, write, invalidate on clause drift. |
| `code/tools/dcgo-harness/src/exam/backfill.rs` (create) | Write confirmed DCGO state into a scenario's `assert:` block. |
| `code/tools/dcgo-harness/src/exam/drafter.rs` (create) | Emit a provenance-headed draft `#[test]`. |
| `code/tools/dcgo-harness/src/exam/mcp.rs` (create) | `exam_card` / `run_scenario` / `exam_status` tools. |
| `code/tools/clause_coverage/exam_binding.py` (create) | Bind scenarios to the clause denominator; report `unmeasured`. |
| `.github/workflows/dcgo-exam-sim.yml` (create) | The Unity-free PR gate. |
| `qa/qa-reports/dcgo_exam_verdicts.json` (create) | The store itself. |
| `.claude/skills/dcgo-exam/SKILL.md` (create) | The agent-facing workflow. |
| `docs/DCGO_EXAM.md` (create) | Operating manual. |

---

### Task 1: The verdict store

**Files:**
- Create: `code/tools/dcgo-harness/src/exam/verdict.rs`
- Modify: `code/tools/dcgo-harness/src/exam/mod.rs`
- Test: inline tests

**Interfaces:**
- Consumes: nothing from plan 2 (deliberately — this task can start immediately).
- Produces:
  ```rust
  pub enum Verdict { Confirmed, Diverged, Unreachable, Unavailable, Unmeasured }
  pub struct ClauseVerdict {
      pub clause_id: String, pub card_id: String, pub verdict: Verdict,
      pub label: String, pub text_sha256: String,
      pub scenario_path: Option<String>, pub reason: Option<String>,
      pub dcgo_build: Option<String>, pub job_id: Option<String>,
      pub recorded_at: String,
  }
  pub struct VerdictStore { /* … */ }
  impl VerdictStore {
      pub fn load(path: &Path) -> Result<VerdictStore, String>;
      pub fn save(&self, path: &Path) -> Result<(), String>;
      pub fn record(&mut self, v: ClauseVerdict);
      pub fn get(&self, clause_id: &str) -> Option<&ClauseVerdict>;
      pub fn get_validated(&self, clause_id: &str, current_text_sha256: &str) -> Option<&ClauseVerdict>;
      pub fn summary(&self, all_clause_ids: &[String]) -> VerdictSummary;
  }
  pub struct VerdictSummary { pub confirmed: usize, pub diverged: usize,
                              pub unreachable: usize, pub unavailable: usize,
                              pub unmeasured: usize, pub total: usize,
                              pub invalidated: usize }
  ```

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn v(clause: &str, verdict: Verdict, sha: &str) -> ClauseVerdict {
        ClauseVerdict {
            clause_id: clause.to_string(),
            card_id: clause.split('#').next().unwrap().to_string(),
            verdict,
            label: "[On Play]".to_string(),
            text_sha256: sha.to_string(),
            scenario_path: None, reason: None,
            dcgo_build: None, job_id: None,
            recorded_at: "2026-08-21T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn records_and_reads_back() {
        let mut s = VerdictStore::default();
        s.record(v("EX12-035#effect#0", Verdict::Confirmed, "abc"));
        assert_eq!(s.get("EX12-035#effect#0").unwrap().verdict, Verdict::Confirmed);
    }

    #[test]
    fn a_clause_absent_from_the_store_is_unmeasured_not_missing() {
        // The denominator is the point: an unauthored clause must appear in the
        // report as `unmeasured`, never be silently omitted.
        let s = VerdictStore::default();
        let sum = s.summary(&["EX12-035#effect#0".to_string(), "EX12-035#security#0".to_string()]);
        assert_eq!(sum.unmeasured, 2);
        assert_eq!(sum.total, 2);
        assert_eq!(sum.confirmed, 0);
    }

    #[test]
    fn summary_totals_every_class() {
        let mut s = VerdictStore::default();
        s.record(v("A#effect#0", Verdict::Confirmed, "x"));
        s.record(v("B#effect#0", Verdict::Diverged, "x"));
        s.record(v("C#effect#0", Verdict::Unavailable, "x"));
        let ids: Vec<String> = ["A#effect#0","B#effect#0","C#effect#0","D#effect#0"]
            .iter().map(|s| s.to_string()).collect();
        let sum = s.summary(&ids);
        assert_eq!((sum.confirmed, sum.diverged, sum.unavailable, sum.unmeasured), (1, 1, 1, 1));
        assert_eq!(sum.total, 4);
    }

    #[test]
    fn a_verdict_whose_clause_text_changed_is_invalidated() {
        // Clause ids are positional within a zone, so an override or re-scrape
        // that changes a card's text silently re-points every later id at a
        // DIFFERENT clause. A stale `confirmed` would then vouch for a clause
        // nobody examined.
        let mut s = VerdictStore::default();
        s.record(v("EX12-035#effect#0", Verdict::Confirmed, "old-sha"));
        assert!(s.get_validated("EX12-035#effect#0", "old-sha").is_some());
        assert!(s.get_validated("EX12-035#effect#0", "new-sha").is_none(),
                "text drift must invalidate the verdict");
    }

    #[test]
    fn invalidated_verdicts_count_as_unmeasured_in_the_summary() {
        let mut s = VerdictStore::default();
        s.record(v("EX12-035#effect#0", Verdict::Confirmed, "old-sha"));
        s.set_current_text_sha("EX12-035#effect#0", "new-sha");
        let sum = s.summary(&["EX12-035#effect#0".to_string()]);
        assert_eq!(sum.confirmed, 0);
        assert_eq!(sum.unmeasured, 1);
        assert_eq!(sum.invalidated, 1);
    }

    #[test]
    fn round_trips_through_json() {
        let mut s = VerdictStore::default();
        s.record(v("EX12-035#effect#0", Verdict::Confirmed, "abc"));
        let text = s.to_json().unwrap();
        let back = VerdictStore::from_json(&text).unwrap();
        assert_eq!(back.get("EX12-035#effect#0").unwrap().verdict, Verdict::Confirmed);
    }

    #[test]
    fn unavailable_carries_a_reason() {
        // "DCGO has no script for this card" must be distinguishable from
        // "we never got around to it" in the stored data, not just in prose.
        let mut cv = v("BT27-001#effect#0", Verdict::Unavailable, "x");
        cv.reason = Some("no DCGO script at BT27/Red/BT27_001.cs".to_string());
        let mut s = VerdictStore::default();
        s.record(cv);
        assert!(s.get("BT27-001#effect#0").unwrap().reason.as_ref().unwrap().contains("BT27_001.cs"));
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cargo test -p dcgo-harness --lib exam::verdict
```

Expected: FAIL — `cannot find type 'VerdictStore'`.

- [ ] **Step 3: Write the implementation**

Serialize as `{"version": 1, "last_updated": "...", "clauses": {<clause_id>: {...}}}`, matching the shape of `qa/qa-reports/validated_cards_dsl.json` (`version` / `last_updated` / `cards`) so the QA artifacts read alike.

`get_validated` returns `None` when the stored `text_sha256` differs from the caller's current one. `summary` counts an invalidated verdict as `unmeasured` **and** increments `invalidated`, so drift is visible rather than merely absorbed.

- [ ] **Step 4: Run the test to verify it passes**

```bash
cargo test -p dcgo-harness --lib exam::verdict
```

Expected: `test result: ok. 7 passed`.

- [ ] **Step 5: Commit**

```bash
git add code/tools/dcgo-harness/src/exam/
git commit -m "exam: verdict store with clause-text drift invalidation"
```

---

### Task 2: Clause binding — the denominator

**Files:**
- Create: `code/tools/clause_coverage/exam_binding.py`
- Test: `code/tests/tools/test_clause_coverage_exam_binding.py`

**Interfaces:**
- Consumes: `clause_coverage.extract.run`, `clause_coverage.models.Clause`.
- Produces:
  ```python
  def bind(card_ids: list[str], scenarios_dir: Path, verdicts_path: Path) -> dict
  # -> {"cards": {...}, "denominator": {"total_clauses": N, "by_verdict": {...}},
  #     "unmeasured_clause_ids": [...], "orphan_scenarios": [...]}
  ```

- [ ] **Step 1: Write the failing test**

```python
"""Exam binding: scenarios <-> the clause_coverage denominator."""
import json
from pathlib import Path

import pytest

from tools.clause_coverage.exam_binding import bind


def _write_scenario(d: Path, name: str, card: str, clause: str) -> Path:
    p = d / f"{name}.yaml"
    p.write_text(
        f"card: {card}\nclause: {clause}\nseed: 1\n"
        "decks:\n  p0: {stack: [], rest: x}\n  p1: {stack: [], rest: x}\n"
        "steps:\n  - actor: 0\n    do: {pass: {}}\n",
        encoding="utf-8",
    )
    return p


def test_clause_with_no_scenario_is_unmeasured(tmp_path):
    result = bind(["EX12-073"], tmp_path, tmp_path / "verdicts.json")
    assert result["denominator"]["total_clauses"] > 0
    # Every clause is unmeasured: nothing has been authored.
    assert result["denominator"]["by_verdict"]["unmeasured"] == result["denominator"]["total_clauses"]
    assert result["unmeasured_clause_ids"]


def test_a_scenario_naming_an_unknown_clause_is_an_orphan_not_a_pass(tmp_path):
    # This is the invisible-sixth-class failure the whole binding exists to
    # prevent: a typo'd clause id would otherwise pass its own assertions while
    # covering nothing in the denominator.
    _write_scenario(tmp_path, "typo", "EX12-073", "EX12-073#effct#0")
    result = bind(["EX12-073"], tmp_path, tmp_path / "verdicts.json")
    assert result["orphan_scenarios"], "a scenario keyed to no real clause must be reported"
    assert "EX12-073#effct#0" in json.dumps(result["orphan_scenarios"])


def test_denominator_always_sums_to_total(tmp_path):
    result = bind(["EX12-073", "EX12-035"], tmp_path, tmp_path / "verdicts.json")
    by = result["denominator"]["by_verdict"]
    assert sum(by.values()) == result["denominator"]["total_clauses"]


def test_verdicts_file_absent_is_not_an_error(tmp_path):
    # First run on a fresh checkout must work, reporting everything unmeasured.
    result = bind(["EX12-073"], tmp_path, tmp_path / "does-not-exist.json")
    assert result["denominator"]["by_verdict"]["unmeasured"] > 0
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
PYTHONPATH=code python -m pytest code/tests/tools/test_clause_coverage_exam_binding.py -v
```

Expected: `ModuleNotFoundError: No module named 'tools.clause_coverage.exam_binding'`.

- [ ] **Step 3: Write the implementation**

`bind` runs `extract` for the given cards, globs `scenarios_dir/**/*.yaml`, maps each scenario's `clause:` onto the extracted clause ids, loads the verdict store if present, and produces the joined report. A scenario whose clause id is not in the extracted set goes to `orphan_scenarios` — never silently ignored.

- [ ] **Step 4: Run the test to verify it passes**

```bash
PYTHONPATH=code python -m pytest code/tests/tools/test_clause_coverage_exam_binding.py -v
```

Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
git add code/tools/clause_coverage/exam_binding.py code/tests/tools/test_clause_coverage_exam_binding.py
git commit -m "exam: bind scenarios to the clause_coverage denominator"
```

---

### Task 3: `unavailable` — does DCGO even implement this card?

**Files:**
- Create: `code/tools/dcgo-harness/src/exam/dcgo_pool.rs`
- Test: inline tests

**Interfaces:**
- Consumes: the DCGO checkout path.
- Produces: `pub fn has_dcgo_script(dcgo_root: &Path, card_id: &str) -> bool;`

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fake_checkout(tmp: &std::path::Path) {
        // Mirrors the real layout: CardEffect/<SET>/<COLOR>/<CARD_ID>.cs, with
        // UNDERSCORED filenames (BT17-001 -> BT17_001.cs).
        let dir = tmp.join("Assets/Scripts/CardEffect/EX12/Red");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("EX12_035.cs"), "// stub").unwrap();
        fs::create_dir_all(tmp.join("Assets/Scripts/CardEffect/BT26/Blue")).unwrap();
    }

    #[test]
    fn finds_a_card_dcgo_implements() {
        let tmp = tempdir();
        fake_checkout(tmp.path());
        assert!(has_dcgo_script(tmp.path(), "EX12-035"));
    }

    #[test]
    fn a_card_in_an_existing_set_but_with_no_script_is_absent() {
        // The per-card, not per-set, rule: a set directory can exist while an
        // individual card has none. "Newer than DCGO" is the WRONG test --
        // DCGO spans BT1-BT26, EX1-EX12, ST1-ST24, AD1, LM, P, RB1.
        let tmp = tempdir();
        fake_checkout(tmp.path());
        assert!(!has_dcgo_script(tmp.path(), "BT26-001"));
    }

    #[test]
    fn a_card_from_a_set_dcgo_does_not_have_is_absent() {
        let tmp = tempdir();
        fake_checkout(tmp.path());
        assert!(!has_dcgo_script(tmp.path(), "BT27-001"));
    }

    #[test]
    fn the_underscore_naming_convention_is_honored() {
        let tmp = tempdir();
        fake_checkout(tmp.path());
        // The hyphenated filename must NOT match -- DCGO uses underscores.
        assert!(!tmp.path().join("Assets/Scripts/CardEffect/EX12/Red/EX12-035.cs").exists());
        assert!(has_dcgo_script(tmp.path(), "EX12-035"));
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cargo test -p dcgo-harness --lib exam::dcgo_pool
```

Expected: FAIL — `cannot find function 'has_dcgo_script'`.

- [ ] **Step 3: Write the implementation**

Search `<dcgo_root>/Assets/Scripts/CardEffect/<SET>/*/<CARD_ID with '-' → '_'>.cs`. The colour subdirectory is not known from the card id alone, so glob across colours.

Resolve `dcgo_root` per rule 29: `$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO`. **In a worktree the local `./DCGO` is an intentionally-empty placeholder** — a checker pointed there would report every card `unavailable` and quietly turn the whole exam into a no-op that reads as "nothing to verify". Fail loudly if `dcgo_root/Assets/Scripts/CardEffect` does not exist.

- [ ] **Step 4: Run the test to verify it passes**

```bash
cargo test -p dcgo-harness --lib exam::dcgo_pool
```

Expected: `test result: ok. 4 passed`.

- [ ] **Step 5: Commit**

```bash
git add code/tools/dcgo-harness/src/exam/
git commit -m "exam: per-card DCGO script presence drives the unavailable verdict"
```

---

### Task 4: Assertion backfill

**Files:**
- Create: `code/tools/dcgo-harness/src/exam/backfill.rs`
- Test: inline tests

**Interfaces:**
- Consumes: `Scenario` (plan 2 Task 1), `StateProjection` (plan 2 Task 4), `DiffReport` (plan 2 Task 5).
- Produces: `pub fn backfill(scenario_yaml: &str, confirmed: &[StateProjection]) -> Result<String, String>;`

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backfill_writes_assertions_for_every_step() { /* … */ }

    #[test]
    fn backfill_refuses_when_the_diff_was_not_clean() {
        // Backfilling a diverged run would bake DCGO's disagreement in as our
        // expected value and make the scenario pass forever after.
        /* … */
    }

    #[test]
    fn backfill_preserves_hand_authored_assertions_it_did_not_generate() { /* … */ }

    #[test]
    fn backfill_is_idempotent() {
        // Running it twice must not accumulate duplicate `at:` entries.
        /* … */
    }

    #[test]
    fn backfill_does_not_assert_security_contents() {
        // Security is a COUNT in the projection precisely because contents are
        // hidden information; an assertion over them would encode knowledge no
        // player has.
        /* … */
    }
}
```

Fill each body against the real `Scenario` type before running.

- [ ] **Step 2: Run, implement, re-run**

```bash
cargo test -p dcgo-harness --lib exam::backfill
```

Backfill re-serializes the scenario with a generated `assert:` block. Generated entries carry a marker key so `backfill_preserves_hand_authored_assertions_it_did_not_generate` and idempotency are both mechanically decidable rather than heuristic.

- [ ] **Step 3: Commit**

```bash
git add code/tools/dcgo-harness/src/exam/
git commit -m "exam: backfill confirmed oracle state into scenario assertions"
```

---

### Task 5: The test drafter

**Files:**
- Create: `code/tools/dcgo-harness/src/exam/drafter.rs`
- Test: inline tests

**Interfaces:**
- Produces: `pub fn draft_test(scenario: &Scenario, confirmed: &[StateProjection], provenance: &Provenance) -> String;`
  with `pub struct Provenance { pub dcgo_build: String, pub job_id: String, pub scenario_path: String }`

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_header_records_provenance_and_does_not_claim_correctness() {
        let out = draft_test(&scenario(), &[], &prov());
        assert!(out.contains("DCGO build"));
        assert!(out.contains("job "));
        assert!(out.contains("observed:"));
        // DCGO is source-priority #2, below general_rule.pdf. A drafted test
        // encodes strong evidence, not truth -- and a generated test asserting
        // a behavior nobody read would launder a DCGO quirk into a permanent
        // guard, which under the no-approximations policy is worse than no test.
        assert!(!out.to_lowercase().contains("correct"),
                "the header must not assert correctness");
    }

    #[test]
    fn output_is_a_compilable_test_shape() {
        let out = draft_test(&scenario(), &[], &prov());
        assert!(out.contains("#[test]"));
        assert!(out.contains("fn "));
    }

    #[test]
    fn the_draft_is_returned_never_written_to_disk() {
        // The drafter must never auto-commit; returning a String is what makes
        // that structural rather than a rule someone has to remember.
        let out = draft_test(&scenario(), &[], &prov());
        assert!(!out.is_empty());
    }
}
```

- [ ] **Step 2: Run, implement, re-run**

```bash
cargo test -p dcgo-harness --lib exam::drafter
```

The function returns a `String`. Writing it to `code/digimon-engine/tests/cards_behavioral/<set>/` is the CLI's job, behind an explicit `--write-draft` flag, and it never runs `git add`.

- [ ] **Step 3: Commit**

```bash
git add code/tools/dcgo-harness/src/exam/
git commit -m "exam: draft cards_behavioral tests with provenance, never auto-commit"
```

---

### Task 6: The Unity-free CI gate

**Files:**
- Create: `.github/workflows/dcgo-exam-sim.yml`

**Interfaces:**
- Consumes: `dcgo-harness exam --sim-only` (plan 2 Task 6).

- [ ] **Step 1: Write the workflow**

```yaml
name: DCGO Exam (sim-only)

# The half of the exam that GitHub CAN run. The DCGO oracle itself cannot gate
# PRs here: DCGO's AI mode connects to Photon (a live third-party service), the
# build is a multi-GB licensed Unity LFS checkout, headless -batchmode PLAY is
# out of scope, and the artifact contains redistributable-restricted card art.
#
# So this job asserts only that our engine still agrees with what the oracle
# PREVIOUSLY confirmed -- the `assert:` blocks that `exam backfill` wrote. It
# cannot find a new divergence, and must never be described as if it could.

on:
  pull_request:
    paths:
      - 'qa/dcgo-exams/**'
      - 'code/digimon-engine/**'
      - 'code/digimon-dsl/**'
      - 'code/tools/dcgo-harness/**'
      - '.github/workflows/dcgo-exam-sim.yml'
  push:
    branches: ["main"]
    paths:
      - 'qa/dcgo-exams/**'
      - 'code/digimon-engine/**'
      - 'code/digimon-dsl/**'
      - 'code/tools/dcgo-harness/**'
      - '.github/workflows/dcgo-exam-sim.yml'

jobs:
  exam-sim:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        # DCGO is a multi-GB LFS submodule and this job must never need it.
        with:
          submodules: false

      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Run every exam scenario sim-only
        run: |
          cargo run -p dcgo-harness --release -- exam \
            --scenario qa/dcgo-exams/ \
            --sim-only \
            --cards-json data/cards.json

      - name: Differ unit + golden tests
        run: cargo test -p dcgo-harness
```

- [ ] **Step 2: Verify it does not need the DCGO submodule**

```bash
git grep -n "DCGO" .github/workflows/dcgo-exam-sim.yml
```

Expected: matches only in the explanatory comment and `submodules: false`. **If the job ever needs the submodule, the design has drifted** — the whole point is that the only thing needing Unity is Unity.

- [ ] **Step 3: Verify the gate actually fails on a regression**

Locally break one committed scenario's assertion and confirm a non-zero exit:

```bash
cargo run -p dcgo-harness -- exam --scenario qa/dcgo-exams/ --sim-only --cards-json data/cards.json
echo "exit=$?"
```

Expected: `exit=1` with the failing scenario named. **A gate that cannot fail is not a gate** — do not commit until this is demonstrated, then restore the scenario.

- [ ] **Step 4: Commit**

```bash
git checkout qa/dcgo-exams/
git add .github/workflows/dcgo-exam-sim.yml
git commit -m "ci: Unity-free exam gate over scenario assertions"
```

---

### Task 7: Agent surface — MCP subcommand and skill

**Files:**
- Create: `code/tools/dcgo-harness/src/exam/mcp.rs`, `.claude/skills/dcgo-exam/SKILL.md`, `docs/DCGO_EXAM.md`
- Modify: `code/tools/dcgo-harness/src/main.rs`, `docs/INDEX.md`, `CLAUDE.md`

**Interfaces:**
- Produces: `dcgo-harness mcp` exposing `exam_card`, `run_scenario`, `exam_status`.

- [ ] **Step 1: Implement the tools over the library API**

Each tool is a thin wrapper over functions already unit-tested in Tasks 1–5 — that is the reason the MCP is a subcommand of this binary rather than a separate crate, matching how `dcgo-replay` and `digimon-engine-mcp` share one core.

- `exam_status(card_id)` → the `VerdictSummary`, **always including `unmeasured`**.
- `run_scenario(path, sim_only)` → the `DiffReport`.
- `exam_card(card_id)` → binds the denominator, runs authored scenarios, returns per-clause verdicts.

- [ ] **Step 2: Write the skill**

`.claude/skills/dcgo-exam/SKILL.md` must state, in the skill body so an agent reads it before acting:

- A `diverged` verdict is a **finding to triage, not proof our engine is wrong** — `general_rule.pdf` outranks DCGO.
- `unavailable` and `unmeasured` are **real outcomes** and must be reported alongside `confirmed`. Never report a card as verified on a partial denominator.
- The drafter's output is **evidence, not truth**, and is never committed without a human reading it.
- 25 cards route through `SetIsBackgroundProcess(true)` and bypass the `effect_activation` hook (rule 27), so their clauses are **structurally unmeasurable** by activation matching and get `unreachable` with that reason — not a silent pass.
- DCGO work happens in the **base repo** (rule 29).

- [ ] **Step 3: Write the operating manual**

`docs/DCGO_EXAM.md`: the scenario format, the two run modes, the verdict classes, the CI split and *why* DCGO cannot gate PRs, and the known gaps (`job.first_player` unhonored; background-process cards unmeasurable).

- [ ] **Step 4: Register the docs**

Add `docs/DCGO_EXAM.md` to `docs/INDEX.md`, and a line to CLAUDE.md's documentation list beside the existing DCGO entries.

- [ ] **Step 5: Verify the MCP starts**

```bash
cargo build -p dcgo-harness
target/debug/dcgo-harness mcp --help
```

Expected: usage text, exit 0.

- [ ] **Step 6: Commit**

```bash
git add code/tools/dcgo-harness/ .claude/skills/dcgo-exam/ docs/DCGO_EXAM.md docs/INDEX.md CLAUDE.md
git commit -m "exam: MCP subcommand, /dcgo-exam skill, operating manual"
```

---

### Task 8: End-to-end — exam one real card

**Files:**
- Create: `qa/dcgo-exams/EX12/EX12-035.yaml` (and siblings per clause)
- Modify: `qa/qa-reports/dcgo_exam_verdicts.json`

- [ ] **Step 1: Extract the denominator for one card**

```bash
PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids EX12-035
```

Record the clause count and every clause id. **This number is the denominator every later report must print.**

- [ ] **Step 2: Author one scenario per clause**

One file per clause under `qa/dcgo-exams/EX12/`. Where a clause cannot be reached by a legal line, do not skip it silently — record `unreachable` with the reason.

- [ ] **Step 3: Run sim-only first**

```bash
cargo run -p dcgo-harness -- exam --scenario qa/dcgo-exams/EX12/ --sim-only --cards-json data/cards.json
```

Every scenario must lower and run before any Unity time is spent.

- [ ] **Step 4: Run the oracle pass**

```bash
cargo run -p dcgo-harness -- exam --scenario qa/dcgo-exams/EX12/ --cards-json data/cards.json
```

- [ ] **Step 5: Report honestly**

The output must read like `EX12-035: 8 clauses — 5 confirmed, 1 diverged, 2 unmeasured`, never `EX12-035: passed`.

- [ ] **Step 6: Triage, do not auto-fix**

For each `diverged`, check the printed card text and `general_rule.pdf` before concluding our engine is wrong. **Ask before fixing** — the operating agreement is triage and report; fixes stay a decision.

- [ ] **Step 7: Commit**

```bash
git add qa/dcgo-exams/ qa/qa-reports/dcgo_exam_verdicts.json
git commit -m "qa: first real card exam (EX12-035) with full clause denominator"
```

---

## Self-Review

**Spec coverage.** Implements the spec's workflow layer: verdict store with the five classes (Task 1), clause binding and the `unmeasured` denominator (Task 2), the per-card `unavailable` determination (Task 3), assertion backfill (Task 4), the provenance-headed drafter (Task 5), the Unity-free CI gate (Task 6), the MCP subcommand and skill (Task 7), and a real end-to-end card (Task 8).

**One spec requirement deliberately deferred.** The spec's `unreachable` reason for the 25 `SetIsBackgroundProcess(true)` cards is documented in the skill (Task 7 Step 2) but not *detected automatically* — no task enumerates those cards from the DCGO source. Doing so needs a C# scan that belongs with the Unity work, and hard-coding a list of 25 card ids would rot silently. Until then those clauses surface as ordinary `unreachable` entries whose reason is written by hand.

**Placeholder scan.** Tasks 4 and 5 carry test bodies as `/* … */` because they depend on plan 2's `Scenario` and `StateProjection` types being finalized; each test's *name and comment* fully specify the behavior, and the bodies are to be filled against the real types before running. This is a genuine sequencing dependency, not vagueness — the assertions are stated, only the constructor calls are pending. Tasks 1, 2, 3, and 6 have complete, runnable tests.

**Type consistency.** `Verdict` and `ClauseVerdict` are defined in Task 1 and consumed in Tasks 2, 3, 7, and 8. Clause ids are `{card_id}#{zone}#{idx}` everywhere — Task 1's tests, Task 2's binding, plan 2's `Scenario::validate`. `has_dcgo_script` (Task 3) is what produces `Verdict::Unavailable`. `StateProjection` and `DiffReport` come from plan 2 and are consumed unchanged in Tasks 4, 5, and 7.
