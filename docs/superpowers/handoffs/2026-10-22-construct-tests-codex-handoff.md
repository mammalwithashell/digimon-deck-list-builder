# Hand-off: construct-test campaign 1 (top-10 meta decks) — for Codex

Written 2026-10-22 by the Claude controller session that planned this work. The owner is out of
Claude usage; Codex takes over execution from here. Everything below is on the branch
`claude/p3-construct-tests`; nothing is implemented yet.

## 1. What this is, in one paragraph

RL training admits a deck only when every card is *admissible*; a card is admissible only when
every DSL construct its YAML references (verb / predicate / `when:` timing / keyword) has a
**behavioural mechanics test** (`qa/impact/lowering_coverage.json`, built by
`code/tools/lowering_coverage.py`). At HEAD the DigiLab tournament 11087 top-10 decks reference
**51 distinct constructs with no such test**, so 0/10 decks can train. This campaign writes those
tests — testing the DSL lowering with synthetic fixture cards, not the production cards — and fixes
the engine defects the research surfaced. Exit: 9/10 decks admissible (Rebootmon waits on a
separate substrate gap). The gate itself does not change.

## 2. Where everything is

| Thing | Path |
|---|---|
| Branch / base | `claude/p3-construct-tests`, cut from `claude/p2-mechanics-gate` @ f24f26dfa — the gate's content, which is on `claude/ex13-meta-cards` (PR #743 merged there) and reaches `main` via the follow-up PR described in §7 |
| Worktree used so far | `D:/wt/p3-constructs` (cargo target `E:/cargo-target-wt/p3-constructs`) — reuse it or make your own from the branch |
| **The plan (read first)** | `docs/superpowers/plans/2026-10-22-construct-tests-1-top10.md` — 9 tasks; Tasks 2–7 are the six construct families |
| Spec | `docs/superpowers/specs/2026-10-15-engine-rules-conformance-validation-design.md` **§5d** (what a construct test is) and §5b (the gate it feeds) |
| Semantics sheets (per family) | `docs/superpowers/research/2026-10-22-construct-tests/semantics/*.md` — lowering sites with line numbers, exact YAML shapes, CR 4.3 citations, production users, proposed fixture + positive/negative/selection assertions, suspected defects. **Each task's implementer must read its family's sheet.** |
| Research brief that produced the sheets | `docs/superpowers/research/2026-10-22-construct-tests/SEMANTICS-BRIEF.md` (regenerate a sheet with it if one is wrong) |
| The 51 constructs, with the decks each blocks | `docs/superpowers/research/2026-10-22-construct-tests/top10_untested_union.json` |
| Baseline measurement | `docs/superpowers/research/2026-10-22-construct-tests/baseline-report.md` (0/10) |
| Deck pool for measurement | `qa/impact/decks/digilab-11087-top10.json` |
| Test home (to create) | `code/digimon-engine/tests/dsl/constructs/<family>.rs` + `mod.rs`; `mod constructs;` in `tests/dsl/main.rs` |
| Example of the test style | `code/digimon-engine/tests/dsl/effect_battle.rs`, `tests/dsl/play_or_use_from_sources.rs`, `tests/dsl/folded_may_opt_refund.rs`; DebugRunner surface in `docs/RUST_DSL_TEST_API.md` §3 |
| Gap trackers | `docs/RUST_ENGINE_GAPS.md` (engine), `qa/dsl-vocab-gaps.md` (DSL) |
| Rules text | `docs/digimon-rules/cr-4.3.txt` (grep by rule number; cite 4.3 numbers — §16 keywords shifted +1 and §15-7-4 REVERSED vs 3.6, see `docs/digimon-rules/changelog-3.6-to-4.3.md`) |
| DCGO (behaviour tiebreak) | base repo only: `C:/Users/james/Documents/digimon-deck-list-builder-1/DCGO/Assets/Scripts/CardEffect/<SET>/<COLOR>/<ID_with_underscore>.cs`. Never `git submodule update --init` in a worktree. |

## 3. Rules that are not negotiable (owner-set)

1. **Test the mechanic, not the card.** Fixture = same-file `const X: &str = r#"…"#` inline YAML. No `format!`
   placeholder may touch a construct key or a `when:` value (the coverage scanner credits a test by finding the YAML
   key in a plain string literal; it follows same-file consts by name, never cross-file helpers or `include_str!`).
2. **Real play.** The construct must be reached through a clause resolution driven by `DebugRunner` (a trigger
   firing, a `[Main]` activation, an attack, a play) and asserted on game state. Calling `eval_predicate_with_bindings`
   / `run_steps` directly is allowed only as an extra assertion. A test that only inspects compiled IR counts for
   nothing (`parsed_only`).
3. **Positive + negative + selection surface** per construct. Negative boards differ from positive ones in exactly one
   fact. Timings also assert the adjacent timing does NOT fire. Every choice is a real `pending_selection` resolved
   with `execute_action`, built with ≥2 candidates; `auto_resolve` is never the only path through a choice. No
   auto-selections anywhere (CLAUDE.md rule 17).
4. **No `#[ignore]`, no stubs, never weaken an assertion to match the engine.** A defect found while writing a test is
   fixed in the same task when it is a small lowering/engine fix with its own test; otherwise the construct stays
   untested, a gap is filed, and the cards stay inadmissible — that is the gate working. The plan already says, per
   task, which defects to fix and which to file.
5. **Cite the rule.** Module doc comments cite CR 4.3 § numbers, or say "card-text idiom" and name two printing cards.
6. **Smoke tests are not validation** (owner feedback). Tests assert behaviour, not "it didn't panic".
7. **Build isolation:** `CARGO_TARGET_DIR=E:/cargo-target-wt/p3-constructs` (or your own per-worktree dir under
   `E:/cargo-target-wt/`) on every cargo command. Iterate with the `dsl` binary only:
   `cargo test -p digimon-engine --test dsl constructs::<family>`.
8. **Python env:** `PYTHONIOENCODING=utf-8 PYTHONUTF8=1 PYTHONPATH=code` on every python command.
9. **Git hygiene:** never bare `git stash` (shared stash across worktrees); never touch or commit
   `qa/dcgo-exams/P/P-092-tail-*.yaml` (CRLF churn); never commit `qa/impact/lowering_coverage.json` or
   `data/card_admissibility.json` mid-family — Task 8 regenerates and commits them once; small focused commits per task.
10. **Never init the worktree DCGO submodule**; read DCGO from the base repo path above.

## 4. Execution

- Order by deck unblock: **Task 1** (scaffold) → **5** (identity_state: `card_number_is` + `can_attack` free seven
  decks' largest blockers) → **2** (security_stack) → **3** (play_digivolve_link) → **4** (cost_target_event) →
  **6** (zone_moves) → **7** (protection, slower rubric) → **8** (regenerate + measure) → **9** (verify + docs).
- Tasks 2–7 are independent (disjoint test files). Tasks 3 and 7 both edit `src/dsl_cards/lower_triggered.rs` in
  different functions — do 7 after 3 or rebase. Parallel workers need separate worktrees; two writers never touch the
  same file.
- Each task: write the named tests red → run → fix the plan-named defects → run green → run the `cards_behavioral`
  modules the task lists (`cargo test -p digimon-engine --test cards_behavioral <module> ` with
  `RUST_MIN_STACK=268435456`) → regenerate the coverage index locally to confirm the family's constructs flip to
  `tested_behaviourally: true` → `git checkout -- qa/impact/lowering_coverage.json` → file gaps → commit.
- Reviewer per task (a fresh context): checks spec §5d items 3–5 against the diff — the scanner cannot tell a
  vacuous test from a real one, the reviewer is the real gate.

Useful commands:

```bash
# list the family's tests
CARGO_TARGET_DIR=E:/cargo-target-wt/p3-constructs cargo test -p digimon-engine --test dsl constructs:: -- --list
# regenerate + check coverage
PYTHONIOENCODING=utf-8 PYTHONUTF8=1 PYTHONPATH=code python -m tools.lowering_coverage
PYTHONIOENCODING=utf-8 PYTHONUTF8=1 PYTHONPATH=code python -m tools.lowering_coverage --check
# deck report
PYTHONIOENCODING=utf-8 PYTHONUTF8=1 PYTHONPATH=code python -m tools.lowering_coverage --report out.md --decks qa/impact/decks/digilab-11087-top10.json
# admissibility artifact (Task 8 only)
PYTHONIOENCODING=utf-8 PYTHONUTF8=1 PYTHONPATH=code python -m digimon_gym.agents.card_admissibility
# tier 0 (13 checks)
PYTHONIOENCODING=utf-8 PYTHONUTF8=1 python scripts/verify --tier 0
```

Known pre-existing reds you may hit and must not "fix": `dsl` binary `official_rule_grants` and `trait_parity`;
`cards_behavioral` `bt24_085`; `lib` `security_compaction_rekeys`; `archetypes` `ma3_gazimon`; one `invariant_fuzz`
seed; 14 `code/tests/tools` failures. Confirm a red is pre-existing on `f24f26dfa` before attributing it to your change.
Cargo stalls → kill stale `dsl-*.exe` / `cards_behavioral-*.exe` holding the target lock (CLAUDE.md rule 33).

## 5. Owner decisions still open (do NOT guess; leave the affected test out and report)

1. `can_attack` with `without_suspending` on a `CannotSuspend` Digimon: official Q&A Q3185 (BT19-101) + DCGO allow,
   engine refuses citing §11-2-5. Task 5 row 8 stays untested.
2. Bandai's Language Standardization "in its name" exclusions (BurningGreymon ≠ [Greymon] etc.): only Indramon ≠
   [Dramon] ships. Task 4 tests pin only that one + case-insensitivity.
3. `source_count` and face-down sources (§4-7-9): fix unless DCGO counts them — Task 5 (c) says check DCGO first.
4. App Fusion link-card scope (Task 3 gap): report only.

## 6. What "done" looks like

- `tests/dsl/constructs/` holds six modules; `cargo test -p digimon-engine --test dsl` green except the two
  pre-existing reds.
- All 50 constructs (51 minus `verb:app_fuse`'s refund signal, whose existing behaviour is still covered by the
  ported suite) read `tested_behaviourally: true` in the regenerated `qa/impact/lowering_coverage.json`.
- `--report --decks qa/impact/decks/digilab-11087-top10.json` prints `Decks fully lowering-covered: 9/10`
  (only `2-Rebootmon` no).
- `scripts/verify --tier 0` 13/13; the regenerated `data/card_admissibility.json` admits every card of those 9 decks.
- Every defect the plan says to fix has a test; every defect it says to file has a gap entry; the task reports list
  the owner findings (Section 5 + the per-task "Findings" steps).
- A PR (draft) targeting `claude/p2-mechanics-gate` with a body listing per-family construct counts, deck flips, gaps
  filed, verification counts. Do not push to `claude/p2-mechanics-gate` or main directly.

## 7. Context you would otherwise have to rediscover

- The admissibility gate (`code/digimon_gym/agents/card_admissibility.py`) reads only the two committed `data/` /
  `qa/impact/` artifacts; `scripts/verify --tier 0` has `--check`s for both, so a stale artifact goes red in CI.
- `data/card_admissibility.json` reasons: `unregistered > untested_lowering > untested_card > failed_verdict >
  diverged_no_quirk`. Rebootmon's BT25-089 is `failed_verdict` (PARTIAL) for the app-fuse result signal regardless of
  construct coverage.
- The hybrid rule (card with ≥4 clauses or a protection construct also needs its own `cards_behavioral` test) is
  already satisfied for every top-10 card; nothing to do there.
- **Base-branch state (2026-10-22 09:10Z):** PR #742 (37 EX13 meta cards) is on `main`. PR #743 (the two-layer
  mechanics gate) merged 46 s later into its *stacked* base `claude/ex13-meta-cards`, so **`main` does not have the
  gate yet**; PR #744 (`claude/ex13-meta-cards` → `main`) carries it (diff = #743's merge commit
  88e3cb841). This branch's base f24f26dfa is contained in that follow-up, so once it merges, rebase this branch onto
  `main` (expect a clean rebase: only docs on this branch so far) and target the campaign PR at `main`. Until then
  the campaign PR targets `claude/ex13-meta-cards`. Check with `git ls-tree origin/main data/card_admissibility.json`
  — when it prints, main has the gate.
- Other open follow-ups (not this campaign): app-fuse result signal (BT25-089); would-link window for effect links
  (BT25-004 / BT25-045 vs BT25-089); `card_number_is` self-proxy sweep (~25 cards, Task 5 gap); BT16-055
  De-Digivolve scope; oracle re-exams for rows triaged `stale`; `meta_coverage.py` / `candidates.py` still on the old
  ledger rule.
