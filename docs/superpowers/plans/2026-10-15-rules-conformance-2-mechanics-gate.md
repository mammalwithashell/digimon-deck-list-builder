# Rules Conformance 2 — Mechanics Training Gate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the per-clause-plus-interaction DCGO readiness gate on training with a two-layer gate: a global mechanics-conformance gate derived from `qa/rules/cr_obligations.json`, plus a light per-card admissibility check.

**Architecture:** A generator writes `data/mechanics_readiness.json` from the obligation ledger (Plan 1, Task 3). `gauntlet.py` learns `_load_mechanics_readiness()` and `_card_light_admissible()`, keeps the fail-fast `EmptyTrainingPoolError` and blocker report, and gains `--legacy-clause-gate` for a transition comparison. `oracle_readiness.json` keeps being generated but becomes a report.

**Tech Stack:** Python 3.13, pytest (`code/tests/rl/`), existing `tools.clause_coverage.readiness`, `data/card_official.json`, `qa/qa-reports/validated_cards_dsl.json`, `qa/qa-reports/exam-verdicts/`.

**Spec:** `docs/superpowers/specs/2026-10-15-engine-rules-conformance-validation-design.md` §5b (gate), §0 (steer). Depends on Plan 1 Task 3's `qa/rules/cr_obligations.json` schema.

## Global Constraints

- Mechanics set (verbatim from spec §5b): `effects-and-ordering, keywords, attack-and-timing, security, battle, blocking, rule-checks, turn-flow, digivolution-and-costs, play-and-costs, card-use-and-costs, link`.
- Training is admitted only if `mechanics_readiness.ready == true` **and** every card in a decklist passes the light gate. `unmeasured` clauses never block; a `diverged` exam verdict without `triage: dcgo_quirk` always blocks that card.
- The artifact prints its denominator; no bare "ready" string without counts.
- No CLI override that admits a non-admissible deck; `--legacy-clause-gate` only *adds* the old per-card requirement for comparison runs.
- Tests inject their own readiness via `code/tests/conftest.py` as today (`@pytest.mark.oracle_gate` keeps the real artifacts).
- `CLAUDE.md` rule 34 is rewritten in the same commit that flips the default (Task 4).

## Review Focus

1. **A card whose authored text drifted from the official bundle after its last verdict** — the person expects it to drop out of the pool. Pinned by Task 2 `test_text_drift_blocks_card`.
2. **An artifact generated from a stale `cr_obligations.json`** — pinned by Task 1 `--check` and `test_source_sha_mismatch_fails_check`.
3. **A pool snapshot taken under the old gate that contains a now-blocked card** — pinned by Task 3 `test_snapshot_load_applies_light_gate`.
4. **An empty mechanics set (all `not-testable`) reading as ready** — pinned by Task 1 `test_zero_proven_is_not_ready`.
5. **The blocker message naming no actionable next step** — pinned by Task 3 `test_blocker_message_names_gap_ids_and_regenerate_command`.

---

### Task 1: `mechanics_readiness.json` generator

**Files:**
- Create: `code/tools/rules/mechanics_readiness.py`, `data/mechanics_readiness.json`
- Test: `code/tests/tools/test_rules_mechanics_readiness.py`

**Interfaces:**
- Consumes: `qa/rules/cr_obligations.json` (`obligations[id].{mechanisms,status,blocks_training,gap_id}`, `source.sha256`).
- Produces: `build(obligations: dict) -> dict` → `{ "version": 1, "source": {"obligations_sha256": str, "rules_version": "4.3"}, "mechanics_set": [...], "counts": {"proven": n, "not_testable": n, "gap_blocking": n, "gap_nonblocking": n, "total": n}, "blocking": [{"rule": id, "gap_id": str, "mechanisms": [...]}], "ready": bool }`; `ready = counts.gap_blocking == 0 and counts.proven > 0`. CLI `python -m tools.rules.mechanics_readiness [--check]`.

- [ ] **Step 1: Failing tests** — `test_ready_when_no_blocking_gaps`, `test_blocking_gap_sets_not_ready_and_lists_rule`, `test_zero_proven_is_not_ready`, `test_only_mechanics_set_rules_counted`, `test_source_sha_mismatch_fails_check`.
- [ ] **Step 2: Run → FAIL.**
- [ ] **Step 3: Implement** (stdlib only; `--check` compares a fresh build to the committed file).
- [ ] **Step 4: Run tests; generate; `--check` exits 0.** Expected today: `ready: false` with the Plan 1 gaps listed.
- [ ] **Step 5: Commit** — `"data: mechanics_readiness.json generator (global mechanics gate artifact)"`.

---

### Task 2: Light per-card admissibility

**Files:**
- Create: `code/digimon_gym/agents/card_admissibility.py`
- Test: `code/tests/rl/test_card_admissibility.py`

**Interfaces:**
- Produces: `CardAdmissibility.load(*, registry_ids: set[str], verdicts_dsl: dict, verdicts_rust: dict, official: dict, exam_verdicts: Iterable[dict]) -> CardAdmissibility`; `.admissible(card_id: str) -> bool`; `.reason(card_id: str) -> str | None` returning one of `"unregistered" | "failed_verdict" | "text_drift" | "diverged_no_quirk"`; `.from_repo(paths: DataPaths = default) -> CardAdmissibility`.
- Rules: registered ⇐ `code/tools/build_registry.py`'s registry ids (reuse the loader `gauntlet.py` already uses for `unregistered`); failed verdict ⇐ `validated_cards_dsl.json` / `validated_cards.json` status `FAILED`; text drift ⇐ `text_sha256` recorded in the card's verdict ≠ sha256 of the official bundle text (`data/card_official.json` effect/inherited/security text); diverged ⇐ any verdict row with `verdict == "diverged"` and `triage != "dcgo_quirk"`.

- [ ] **Step 1: Failing tests** — `test_unregistered_blocks`, `test_failed_verdict_blocks`, `test_text_drift_blocks_card`, `test_diverged_without_quirk_blocks`, `test_diverged_with_quirk_citation_admits`, `test_unmeasured_clauses_admit`.
- [ ] **Step 2: Run → FAIL.**  **Step 3: Implement.**  **Step 4: Run → PASS.**
- [ ] **Step 5: Commit** — `"gym: light per-card admissibility (registry, verdicts, text parity, diverged-without-quirk)"`.

---

### Task 3: Two-layer admission in `gauntlet.py`

**Files:**
- Modify: `code/digimon_gym/agents/gauntlet.py` (`_load_oracle_ready_card_ids` ~168, `validate_oracle_ready_deck` ~189, the rejection counters and `EmptyTrainingPoolError` site ~765, `GeneralistDeckPool.from_snapshot`), `code/tests/conftest.py` (permissive mechanics artifact + admissibility for non-`oracle_gate` tests)
- Test: `code/tests/rl/test_gauntlet_mechanics_gate.py`

**Interfaces:**
- Produces: `MechanicsNotReadyError(RuntimeError)` raised at library load when `mechanics_readiness.ready` is false, message = blocking gap ids + `PYTHONPATH=code python -m tools.rules.mechanics_readiness`; rejection reason `not_admissible` replaces `not_oracle_ready` in counters and logs; `validate_admissible_deck(card_ids, admissibility=None, *, label)` replaces `validate_oracle_ready_deck` (old name kept as a deprecated alias for one release); `MetaGauntlet.load(..., legacy_clause_gate: bool = False)`.

- [ ] **Step 1: Failing tests** — `test_mechanics_not_ready_fails_fast_before_any_deck`, `test_admissible_deck_admitted_with_unmeasured_clauses`, `test_inadmissible_card_rejects_deck_with_reason`, `test_snapshot_load_applies_light_gate`, `test_legacy_clause_gate_restores_old_rejections`, `test_blocker_message_names_gap_ids_and_regenerate_command`.
- [ ] **Step 2: Run → FAIL.**  **Step 3: Implement.**  **Step 4:** `python -m pytest code/tests/rl -q -k "gauntlet or admissib"` → PASS; `python -m pytest code/tests -q -m "not slow"` → no new failures.
- [ ] **Step 5: Commit** — `"gauntlet: two-layer admission (mechanics gate + light per-card), legacy clause gate behind a flag"`.

---

### Task 4: Flip the default, rewrite rule 34, demote `oracle_readiness` to a report

**Files:**
- Modify: `CLAUDE.md` rule 34; `docs/superpowers/specs/2026-10-04-dcgo-oracle-readiness-design.md` (status note at §4.7 pointing here); `code/tools/clause_coverage/readiness.py` (docstring: report, not gate); `docs/TRAINING_RUNBOOK.md` (admission section); `.github/workflows/*` where `oracle_readiness --check` runs (keep it; add `mechanics_readiness --check` beside it).

- [ ] **Step 1:** Edit the docs; add the `--check` to CI next to the existing readiness check.
- [ ] **Step 2:** `python -m pytest code/tests -q -m "not slow"`; `scripts/verify --tier 0`.
- [ ] **Step 3:** Commit — `"docs: rule 34 → mechanics gate + light per-card admission; oracle readiness is a report"`.

---

## Self-review notes

- Spec §5b covered end to end (artifact → admissibility → gauntlet → docs). The gate flips to admitting decks only when Plan 1's chapters have no `blocks_training` gaps open; until then Task 3's `MechanicsNotReadyError` is the honest state and the message names what to fix.
- Names consistent with Plan 1: `cr_obligations.json` fields, mechanics-set list.
