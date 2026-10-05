# Oracle Readiness Plan 5 — Source-Trash Triggers Wait for the Trashing Effect (Rocks Unblock)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A "when this card is trashed from digivolution cards" trigger (and any sibling trigger already queued) waits until the effect that trashed the source finishes, per `general_rule.pdf` §15-8-3-2 — so every Rocks inherited clause stops diverging from DCGO.

**Architecture:** Mirror the already-landed `G-ENGINE-SECURITY-REMOVED-OBSERVER-MID-EFFECT` fix: at each source-trash fire site in `game_actions/mod.rs`, replace the deliberate inline `drain_effect_queue()` with `maybe_drain_effect_queue()`, which defers while `draining_deferred > 0` (inside a queued effect body or a selection callback) and drains immediately otherwise. Rewrite the sim tests that pinned the mid-effect order, then confirm on the DCGO oracle.

**Tech Stack:** Rust (`digimon-engine`), DebugRunner behavioral tests, `dcgo-harness` exam.

**Spec:** `docs/superpowers/specs/2026-10-04-dcgo-oracle-readiness-design.md` §4.11; gap `G-ENGINE-INLINE-DRAIN-SIBLING-TRIGGERS` in `docs/RUST_ENGINE_GAPS.md`. Independent of Plans 2–4; Task 6 uses Plan 1's `--backfill` and `verdict-triage` if they have landed.

## Global Constraints

- **Fix gate (campaign rule):** an engine fix needs a rules citation (§15-8-3-2, `general_rule.pdf` p.25), a test that fails before and passes after, and `cards_behavioral` green. Engine fixes land on their own branch, flagged for human review — create it first: `git switch -c engine/source-trash-trigger-deferral`.
- Work in the worktree `C:\Users\james\Documents\digimon-deck-list-builder-1\.claude\worktrees\dcgo-effect-translation`; verify `git rev-parse --show-toplevel`.
- Per-worktree cargo target (rule 31): `CARGO_TARGET_DIR="D:/cargo-target/dcgo-effect-translation"` if unset.
- Full `cards_behavioral` runs use rule 33's invocation: `RUST_MIN_STACK=268435456 cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- --test-threads=8` (~5 min). A run that dies with no `test result:` line and exit 1 is the environmental abort, not a failure; kill stray `cards_behavioral*`/`cargo` processes before re-running.
- No closure-based `pending_selection` may be introduced (rule 28 clone-safety); this plan only changes drain calls and tests.
- Rule 25 is unaffected (OnDeletion handlers already fire post-trash).
- Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## File Map

| File | Change |
|---|---|
| `code/digimon-engine/tests/cards_behavioral/ex10/ex10_028.rs` | New failing ordering test |
| `code/digimon-engine/src/game_actions/mod.rs` | `drain_effect_queue` → `maybe_drain_effect_queue` at the source-trash fire sites; comments |
| `code/digimon-engine/tests/cards_behavioral/ex10/ex10_036.rs` | Rewrite the test that pinned Clause B running mid-Clause A |
| Other tests that flip (Task 3) | Rewrite to the §15-8-3-2 order, each with a citation |
| `code/digimon-engine/src/dsl_cards/step/mod.rs` | Remove the tail-clobber park only if Task 4 proves it dead |
| `docs/RUST_ENGINE_GAPS.md` | Resolve `G-ENGINE-INLINE-DRAIN-SIBLING-TRIGGERS` |
| `qa/dcgo-exams/EX10/EX10-025-inherited0.yaml`, `EX10-028-inherited0.yaml`, NOTES, verdicts | Oracle re-measurement |

---

### Task 1: Failing test — Landramon finishes its effect before the trashed Sunarizamon's trigger

DCGO and §15-8-3-2: EX10-028's [When Digivolving] trashes EX10-025 from its sources as a cost, then grants <Reboot>/<Blocker>/+3000 DP; only after that does EX10-025's inherited "delete 1 of your opponent's Digimon with a play cost of 4 or less" activate. Ours activates the delete first (oracle exam `EX10-028#inherited#0`, `p0.field[1].dp: ours=4000 dcgo=7000`).

**Files:**
- Modify: `code/digimon-engine/tests/cards_behavioral/ex10/ex10_028.rs`

- [ ] **Step 1: Write the failing test**

Append to `ex10_028.rs` (the file already imports `encode_source_select`, `make_test_card`, `DebugRunner`, `EffectTiming`, `Keyword`, `SelectionKind`, `TriggerSource`, and defines `accept_optional_cost`):

```rust
/// §15-8-3-2 (general_rule.pdf p.25): "Trigger-type effects can't activate during
/// the processing for a rule or effect." EX10-028's [When Digivolving] trashes
/// EX10-025 from its digivolution cards as its cost; EX10-025's inherited delete
/// TRIGGERS then, but must not ACTIVATE until EX10-028's grant has resolved.
/// DCGO stacks it the same way (`ITrashDigivolutionCards.TrashDigivolutionCards`
/// → `AutoProcessing.StackSkillInfos`). Oracle evidence: exam
/// `qa/dcgo-exams/EX10/EX10-028-inherited0.yaml`, DCGO build scripted-v17.
#[test]
fn ex10_028_trashed_sunarizamon_trigger_waits_for_the_grant_to_resolve() {
    let mut opp = make_test_card("OPP-COST3", "Opp Cost 3");
    opp.play_cost = 3;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-028")
        .expect("EX10-028 YAML parses and compiles")
        .dsl_card("EX10-025")
        .expect("EX10-025 YAML parses and compiles")
        .add_card(opp)
        .memory(10)
        .start();

    // EX10-028 (Mineral) on the field with EX10-025 (Mineral) as its only source.
    let golemon = runner.place_on_field(0, "EX10-028", None);
    runner.push_source(golemon, "EX10-025");
    runner.place_on_field(1, "OPP-COST3", None);
    let dp_before = runner.dp_of(golemon).unwrap_or(0);

    // A queued [When Digivolving] body: inside it, triggers must wait (15-8-3-2).
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(golemon));
    runner.game.drain_effect_queue();
    accept_optional_cost(&mut runner);

    // Cost: trash EX10-025 (source index 0 under field slot 0).
    assert!(
        matches!(runner.pending_kind(), Some(SelectionKind::SourceMulti { .. })),
        "the cost asks which source to trash; got {:?}",
        runner.pending_kind()
    );
    runner
        .game
        .resolve_selection(0, encode_source_select(0, 0).unwrap())
        .expect("trashing EX10-025 as the cost resolves");

    // EX10-028's own effect continues: pick the Digimon that gains the buffs.
    // EX10-025's delete has TRIGGERED but must still be pending, not prompting.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField),
        "EX10-028's grant target pick must come BEFORE EX10-025's inherited delete (15-8-3-2)"
    );
    assert_eq!(
        runner.game.player(1).battle_area.len(),
        1,
        "the opponent's cost-3 Digimon must still be on the field while EX10-028's effect resolves"
    );
    let view = runner.pending_selection_view().expect("OwnField view");
    runner
        .game
        .resolve_selection(view.selecting_player, view.valid_action_ids[0])
        .expect("the grant target pick resolves");
    assert_eq!(runner.dp_of(golemon).unwrap_or(0) - dp_before, 3000);
    assert!(runner.game.has_keyword(golemon, Keyword::Reboot));

    // Only now does the inherited delete activate.
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    runner.auto_resolve().expect("the inherited delete resolves");
    assert!(
        runner.game.player(1).battle_area.is_empty(),
        "EX10-025's inherited delete removes the opponent's cost-3 Digimon after the grant"
    );
}
```

- [ ] **Step 2: Run to verify it fails for the right reason**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex10_028_trashed_sunarizamon_trigger_waits`
Expected: FAIL at the `OwnField` assertion with `left: Some(OppField)` — the inherited delete prompts mid-effect. If it fails EARLIER (e.g., at the SourceMulti assertion or because the source index differs), fix the setup (print `runner.pending_selection_view()`) until the only failure is the OppField-before-OwnField one; that is the defect under test.

- [ ] **Step 3: Commit the failing test**

```bash
git add code/digimon-engine/tests/cards_behavioral/ex10/ex10_028.rs
git commit -m "test: EX10-028 grant must resolve before the trashed EX10-025's trigger (15-8-3-2, failing)" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Defer source-trash observers

**Files:**
- Modify: `code/digimon-engine/src/game_actions/mod.rs` — `fire_digivolution_card_trashed` (~L1986-2027), `place_permanent_on_security_observed` (~L3019-3053, two sites), `place_permanent_on_security_without_leave_replacement` (~L3359-3377)

- [ ] **Step 1: `fire_digivolution_card_trashed`**

Replace

```rust
        // Intentionally NOT routed through maybe_drain: EX10-036 (and
        // similar multi-source trash chains) rely on observers firing
        // synchronously between source trashes so secondary clauses can
        // pick up the just-trashed cards mid-resolution. Behavioral test
        // `ex10_036_clause_a_after_source_trash_prompts_opp_field_delete`
        // documents the expected interleaving. Other observer fires
        // (place_security, leave_field, link, attack, play) are deferred.
        self.drain_effect_queue();
```

with

```rust
        // 15-8-3-2 (general_rule.pdf p.25): "trigger-type effects can't activate
        // during the processing for a rule or effect". Inside a resolving effect
        // body or selection callback (the deferred scope) the source-trashed
        // observers -- and any sibling trigger already queued -- wait until the
        // trashing effect finishes; DCGO stacks them the same way
        // (`ITrashDigivolutionCards` -> `AutoProcessing.StackSkillInfos`).
        // Outside any effect this still drains immediately.
        // G-ENGINE-INLINE-DRAIN-SIBLING-TRIGGERS; mirrors
        // G-ENGINE-SECURITY-REMOVED-OBSERVER-MID-EFFECT.
        self.maybe_drain_effect_queue();
```

- [ ] **Step 2: `place_permanent_on_security_observed`**

Replace

```rust
            // Intentionally inline-drain (see `fire_digivolution_card_trashed`):
            // EX10-036's behavioral test depends on synchronous between-source
            // observer firing for chained trash-pickup clauses.
            self.drain_effect_queue();
```

with

```rust
            // 15-8-3-2: deferred inside an effect (see `fire_digivolution_card_trashed`).
            self.maybe_drain_effect_queue();
```

and replace

```rust
            // Intentionally inline-drain — same rationale as above.
            self.drain_effect_queue();
```

with

```rust
            // 15-8-3-2: deferred inside an effect (see `fire_digivolution_card_trashed`).
            self.maybe_drain_effect_queue();
```

- [ ] **Step 3: `place_permanent_on_security_without_leave_replacement`**

In its `for card in permanent.card_sources { ... }` loop, replace the trailing `self.drain_effect_queue();` (right after the `enqueue_triggered(EffectTiming::OnDigivolutionCardTrashed, ...)` call) with:

```rust
            // 15-8-3-2: deferred inside an effect (see `fire_digivolution_card_trashed`).
            self.maybe_drain_effect_queue();
```

- [ ] **Step 4: Run the new test**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex10_028_trashed_sunarizamon_trigger_waits`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add code/digimon-engine/src/game_actions/mod.rs
git commit -m "engine: source-trash triggers wait for the trashing effect (15-8-3-2)" -m "Mirrors G-ENGINE-SECURITY-REMOVED-OBSERVER-MID-EFFECT: drain_effect_queue -> maybe_drain_effect_queue at the source-trash fire sites. Fixes the order divergence measured on EX10-025#inherited#0 / EX10-028#inherited#0 (DCGO scripted-v17)." -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Rewrite the tests that pinned the mid-effect order

**Files:**
- Modify: `code/digimon-engine/tests/cards_behavioral/ex10/ex10_036.rs`
- Modify: whichever of the listed tests flip

- [ ] **Step 1: Run the suites most likely to flip**

Run each (separately):

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex10:: bt26:: ex7:: ex8:: p::
```
```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test archetypes -- rocks magneticdra
```
```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test judge_quiz
```
```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch
```

Expected failures (from the pre-change survey): `ex10_036_clause_a_after_source_trash_prompts_opp_field_delete`; likely `bt26_091_effect_trash_from_under_this_tamer_triggers`; possibly `bt26_076_self_caused_tamer_trash_prompt_survives_the_discard`, the four `bt26_075` tests using `decline_ascension_if_pending`, `q23_inherited_trash_memory_gated_on_remaining_in_trash`, and the Rocks/Magneticdramon archetype C1/C5 tests. Write down the exact failing list.

- [ ] **Step 2: Rewrite the EX10-036 test to the rules order**

Magneticdramon EX10-036's two [When Digivolving] clauses trigger together; the player orders them (TriggerOrder) and each resolves completely before the next (§15-8-3-2). In `ex10_036.rs`, replace the whole test `ex10_036_clause_a_after_source_trash_prompts_opp_field_delete` (doc comment and body) with:

```rust
/// Clause A resolves COMPLETELY before Clause B activates (general_rule.pdf
/// §15-8-3-2: trigger-type effects can't activate during the processing of an
/// effect). Both [When Digivolving] clauses trigger together; after the player
/// orders Clause A first, its 3-source cost, opponent pick, delete and
/// security trash all happen before Clause B's first trash pick appears.
#[test]
fn ex10_036_clause_a_resolves_fully_before_clause_b_activates() {
    let src_a = make_mineral_source("BEH-SRC-A1");
    let src_b = make_mineral_source("BEH-SRC-A2");
    let src_c = make_mineral_source("BEH-SRC-A3");
    let carrier = make_test_card("BEH-CARRIER-A", "Beh Carrier A");
    let opp1 = make_opponent_digimon("BEH-OPP-A1");
    let opp2 = make_opponent_digimon("BEH-OPP-A2");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-036")
        .expect("EX10-036 YAML loads")
        .add_card(src_a)
        .add_card(src_b)
        .add_card(src_c)
        .add_card(carrier)
        .add_card(opp1)
        .add_card(opp2)
        .memory(10)
        .start();

    let carrier_h = runner.place_on_field(0, "BEH-CARRIER-A", None);
    runner.push_source(carrier_h, "BEH-SRC-A1");
    runner.push_source(carrier_h, "BEH-SRC-A2");
    runner.push_source(carrier_h, "BEH-SRC-A3");
    runner.place_on_field(1, "BEH-OPP-A1", None);
    runner.place_on_field(1, "BEH-OPP-A2", None);

    let sec_filler = make_test_card("SEC-BEH-A", "Sec Beh A");
    runner.game.card_data.push(sec_filler);
    let fi = runner.game.card_data.len() - 1;
    let cs = CardSource::new(fi, 1, runner.game.next_card_index());
    runner.game.players[1].security.push(cs);

    let magnet = runner.place_on_field(0, "EX10-036", None);
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(magnet));
    runner.game.drain_effect_queue();
    drain_trigger_order_if_any(&mut runner); // orders Clause A first

    for pick in 1..=3 {
        assert!(
            matches!(runner.pending_kind(), Some(SelectionKind::SourceMulti { .. })),
            "Clause A source pick {pick}/3 must be pending; got {:?}",
            runner.pending_kind()
        );
        let view = runner.pending_selection_view().expect("SourceMulti view");
        runner
            .game
            .resolve_selection(view.selecting_player, view.valid_action_ids[0])
            .expect("source pick resolves");
    }

    // Clause A continues straight to its opponent pick: Clause B (and any
    // source-trashed observer) is still waiting.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "Clause A's delete target pick comes before Clause B's trash pick (15-8-3-2); got {:?}",
        runner.pending_kind()
    );
    let view = runner.pending_selection_view().expect("OppField view");
    runner
        .game
        .resolve_selection(view.selecting_player, view.valid_action_ids[0])
        .expect("delete target resolves");
    assert_eq!(runner.game.player(1).battle_area.len(), 1, "Clause A deleted one opponent Digimon");
    assert_eq!(runner.security_count(1), 0, "Clause A trashed the opponent's top security card");

    // Only now does Clause B activate (optional first trash pick).
    assert!(
        matches!(runner.pending_kind(), Some(SelectionKind::Trash)),
        "Clause B activates after Clause A finished; got {:?}",
        runner.pending_kind()
    );
}
```

Also update the stale comment in `ex10_036_clause_a_trashes_the_3_selected_sources` (~L712-716) that says Clause B fires mid-body: replace it with `// Clause B now waits until Clause A finishes (15-8-3-2); the trash check below is unaffected.`

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex10::ex10_036`
Expected: PASS. If `security_count(1)` is wrong because Clause A's security trash also waits behind a park, assert it after `auto_resolve()` instead — but the OppField-before-Trash ordering assertion must stay.

- [ ] **Step 3: Rewrite every other flipped test**

For each test in Step 1's failing list (other than EX10-036):

1. Read it and confirm the old assertion requires a trigger to resolve **during** another effect (an observer prompt appearing before the rest of the trashing effect, or state read mid-effect).
2. If yes: change the expected order so the trashing effect finishes first, and add a one-line comment citing `general_rule.pdf §15-8-3-2 (p.25)`. Keep every other assertion.
3. If the failure is NOT about trigger order (a panic, a lost prompt, wrong final state), STOP: that is a real regression from deferral (cf. `G-PENDING-SECURITY-ADD-WRONG-CARD` after the security fix). Write a failing test for it, fix the root cause in the engine, and log it under the gap entry in Task 5.

Re-run the four commands from Step 1 until all pass.

- [ ] **Step 4: Commit**

```bash
git add code/digimon-engine/tests
git commit -m "test: rewrite tests that pinned mid-effect source-trash triggers to the 15-8-3-2 order" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Retire the tail-clobber park if nothing needs it

`run_steps_with_runtime_inner` (`dsl_cards/step/mod.rs` ~L543-570) parks a tail behind a selection an inline-draining observer installed (`G-DSL-TAIL-CLOBBERS-INLINE-OBSERVER-SELECTION`). With deferral, an observer can still drain inline when a source is trashed OUTSIDE any deferred scope, so the guard may still be live.

- [ ] **Step 1: Try removing it**

Delete the block that begins `if !steps.is_empty() && ctx.game.pending_selection.is_some() {` and ends with its `return RunOutcome::Parked; }` (plus its comment).

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt26::bt26_076 bt26::bt26_091`
Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl`

- [ ] **Step 2: Decide**

- All pass → keep the removal and change the gap entry's last sentence to "the tail-clobber park was removed (no inline drain remains inside effects)".
- Any fail → restore the block exactly (`git checkout -- code/digimon-engine/src/dsl_cards/step/mod.rs`) and update its comment's first sentence to: `// A selection is ALREADY pending before the first step runs: a source trashed OUTSIDE any deferred scope still drains its observers inline.`

- [ ] **Step 3: Commit**

```bash
git add code/digimon-engine/src/dsl_cards/step/mod.rs
git commit -m "engine: tail-clobber park after source-trash deferral (removed or re-justified)" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Full verification and gap tracker

- [ ] **Step 1: CI gate set (rule 33)**

Run each:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --lib
```
```bash
RUST_MIN_STACK=268435456 cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- --test-threads=8
```
```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl
```
```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test archetypes
```
```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test judge_quiz
```
```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test selection
```
```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context
```
```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test keyword_phase_d
```

Expected: every run ends `test result: ok`. A failure in a suite this change could not affect must be checked against unmodified `origin/main` (separate checkout) before debugging (rule 33).

- [ ] **Step 2: Resolve the gap entry**

In `docs/RUST_ENGINE_GAPS.md`, change the heading `## G-ENGINE-INLINE-DRAIN-SIBLING-TRIGGERS — OPEN (observed 2026-10-03, BT26-075 ScourgeChiropmon)` to `## G-ENGINE-INLINE-DRAIN-SIBLING-TRIGGERS — RESOLVED (2026-10-04)` and append:

```markdown
**Resolved (2026-10-04).** `fire_digivolution_card_trashed` and the two security-placement
batch paths now call `maybe_drain_effect_queue()` (deferred inside an effect body or selection
callback; immediate otherwise), mirroring G-ENGINE-SECURITY-REMOVED-OBSERVER-MID-EFFECT.
Pinned by `ex10_028_trashed_sunarizamon_trigger_waits_for_the_grant_to_resolve` (verified failing
with the fix reverted) and `ex10_036_clause_a_resolves_fully_before_clause_b_activates`.
Oracle acceptance: EX10-025#inherited#0 and EX10-028#inherited#0 re-run in
`qa/dcgo-exams/EX10/` (see their NOTES). Not in scope, same pattern, still inline:
`fire_digivolution_card_returned_to_deck_bottom` (`game_actions/mod.rs` ~L2060).
```

Run: `python code/tools/check_resolved_gap_citations.py`
Expected: no error for this entry (it cites real tests).

- [ ] **Step 3: Commit**

```bash
git add docs/RUST_ENGINE_GAPS.md
git commit -m "docs: resolve G-ENGINE-INLINE-DRAIN-SIBLING-TRIGGERS" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Oracle acceptance — re-run the two Rocks inherited clauses

Requires the oracle node and a harness built from this branch.

- [ ] **Step 1: Rebuild and preflight**

```bash
cargo build -p dcgo-harness
```
```bash
D:/cargo-target/dcgo-effect-translation/debug/dcgo-harness.exe --root "C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness" node status --build D:/dcgo-build/scripted-v17
```
Expected: `GO` (start the player with `node up` if the only warning is "player not running").

- [ ] **Step 2: Remove the order-split rows from both scenarios**

Both scenarios carry paired `sim_only` / `dcgo_only` rows that existed only to keep the traces aligned across the order divergence (see `qa/dcgo-exams/EX10/NOTES-EX10-025.md` and `NOTES-EX10-028.md`). Replace each such pair with one ordinary row answered in DCGO's order (the grant target pick, then the inherited delete target), then:

Run: `D:/cargo-target/dcgo-effect-translation/debug/dcgo-harness.exe exam --scenario qa/dcgo-exams/EX10/EX10-025-inherited0.yaml --sim-only --cards-json data/cards.json --decks qa/dcgo-exams/EX10/rocks_pool.json --emit-job C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness/jobs`
Run: `D:/cargo-target/dcgo-effect-translation/debug/dcgo-harness.exe exam --scenario qa/dcgo-exams/EX10/EX10-028-inherited0.yaml --sim-only --cards-json data/cards.json --decks qa/dcgo-exams/EX10/rocks_pool_ex10_028.json --emit-job C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness/jobs`
Expected: both lines complete sim-only.

- [ ] **Step 3: Oracle diff**

For each, wait for `done/exam-EX10-025-inherited0.result.json` / `done/exam-EX10-028-inherited0.result.json` to be newer than the emitted job, read `recording_path`, and run (substituting the `.state.jsonl` sidecar):

```bash
D:/cargo-target/dcgo-effect-translation/debug/dcgo-harness.exe exam --scenario qa/dcgo-exams/EX10/EX10-025-inherited0.yaml --sidecar "<sidecar>" --cards-json data/cards.json --verdicts --clause-text-json qa/exam-clause-text.json
```
(and the same for EX10-028; add `--backfill` if Plan 1 Task 6 has landed.)

Expected: `CLEAN` → `confirmed` replaces the `diverged`/`ours_wrong` row. If either still diverges, the fix is incomplete: record the new first divergence in the NOTES file and do not mark the gap resolved until it is explained.

- [ ] **Step 4: Regenerate readiness (if Plan 2 has landed) and commit**

Run (only if `code/tools/clause_coverage/readiness.py` exists): `PYTHONPATH=code python -m tools.clause_coverage.readiness`

```bash
git add qa/dcgo-exams/EX10 qa/qa-reports/exam-verdicts/EX10-025.json qa/qa-reports/exam-verdicts/EX10-028.json
git add data/oracle_readiness.json 2>/dev/null; git commit -m "exam: EX10-025/028 inherited clauses confirmed after the source-trash deferral fix" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

- [ ] **Step 5: Hand off for review**

Write the PR description (title `engine: source-trash triggers wait for the trashing effect (15-8-3-2)`; body citing §15-8-3-2, the two oracle verdicts, and the list of rewritten tests). Ask the user before pushing the branch or opening the PR — both are outward-facing. Never merge it yourself (engine fixes are human-reviewed).
