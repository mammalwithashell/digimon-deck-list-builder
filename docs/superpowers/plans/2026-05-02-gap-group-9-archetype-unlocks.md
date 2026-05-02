# Gap Group 9 Archetype Unlock Passes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Re-run archetype readiness gates in roadmap order, prove which real deck shells are unlocked by capability groups 1-8, retire raw-Rust/no-op placeholders only when behavior is complete, and update QA trackers with test-backed evidence.

**Architecture:** Treat Group 9 as a validation and card-readiness layer over completed reusable capabilities, not as a new shared primitive group. Each checkpoint reads printed card text, re-runs the `assess-rust-engine-archetype` workflow, adds representative failing behavioral tests for newly-unblocked cards, migrates only those card clauses whose engine and DSL support is proven, and updates tracker files with exact commands.

**Tech Stack:** Rust `digimon-engine`, Rust `digimon-dsl`, YAML card specs in `code/digimon-engine/cards/`, Cargo integration tests, Codex `assess-rust-engine-archetype` skill, markdown QA reports and gap trackers.

---

## Scope Notes

Group 9 must run after the relevant capability groups have landed in the target branch:

- Medusamon depends on Groups 1, 5, 6, and 7 for inherited dispatch, security-removed observers, option/security disposition, and DP predicates.
- Rocks depends on Groups 2, 3, 5, 6, and 7 for source selections, source-trash events, pay-cost ordering, Collision/source immunity, and Delay/Option state.
- Royal Knights depends on Groups 1, 2, 4, 5, and 7 for breeding dispatch, breeding selection, option placement, option trait predicates, and stack placement.
- Puppets depends on Groups 3, 5, 6, 7, and 8 for Overclock predicates, Familiar Token effects, event-gated Delay, and replacement cause gates.
- BG Imperial depends on Groups 3, 5, 7, and 8 for DNA cost data, end-of-turn DNA registration, Partition, and Delay replacement.
- Chaos Control / DNA Omnimon depends on Groups 2, 4, 7, and 8 for non-hand digivolve zones, self-stack predicates, branch choices, and DNA metadata.
- Dark Masters and remaining audits depend on all observer and zone-movement follow-ups needed by their deletion and enter-field loops.

Do not run this plan in parallel with a capability implementation that edits the same card YAML, card behavioral tests, or tracker rows. It is safe to run different archetype checkpoints in parallel only when their write sets are disjoint; the shared files `docs/RUST_ENGINE_GAPS.md`, `qa/archetype-qa/engine-gaps.md`, and `qa/dsl-vocab-gaps.md` must be merged serially.

Do not change `ACTION_SPACE_SIZE` or `TENSOR_SIZE` in Group 9. If an unlock checkpoint discovers a missing player-visible choice, stop that checkpoint and route the blocker back to Group 2, Group 5, or Group 10 rather than expanding contracts here.

## File Structure

Read before editing:

- `AGENTS.md`
- `CLAUDE.md`
- `docs/RUST_ENGINE_API.md`
- `docs/RUST_DSL_TEST_API.md`
- `docs/ACTION_SPEC.md`
- `docs/TENSOR_SPEC.md`
- `docs/RUST_ENGINE_GAPS.md`
- `qa/archetype-qa/engine-gaps.md`
- `qa/dsl-vocab-gaps.md`
- `docs/superpowers/specs/2026-04-29-archetype-engine-dsl-gap-roadmap-design.md`
- `docs/superpowers/plans/2026-04-29-archetype-engine-dsl-gap-roadmap.md`
- `.codex/skills/assess-rust-engine-archetype/SKILL.md`

Expected QA files:

- `qa/archetype-qa/INDEX.md`
- `qa/archetype-qa/medusa.md`
- `qa/archetype-qa/rocks.md`
- `qa/archetype-qa/royal-knights.md`
- `qa/archetype-qa/Puppets.md`
- `qa/archetype-qa/bg-imperial.md`
- `qa/archetype-qa/chaos_control.md`
- `qa/archetype-qa/DNA_Omnimon.md`
- `qa/archetype-qa/Dark_Masters.md`
- `qa/archetype-qa/dsl/medusamon.md`
- `qa/archetype-qa/dsl/medusamon-final-report.md`
- `qa/archetype-qa/dsl/bg-imperial.md`
- Create as needed: `qa/archetype-qa/dsl/rocks.md`
- Create as needed: `qa/archetype-qa/dsl/royal-knights.md`
- Create as needed: `qa/archetype-qa/dsl/puppets.md`
- Create as needed: `qa/archetype-qa/dsl/chaos-control.md`
- Create as needed: `qa/archetype-qa/dsl/dna-omnimon.md`
- Create as needed: `qa/archetype-qa/dsl/dark-masters.md`

Expected card YAML and test areas:

- `code/digimon-engine/cards/bt13/`
- `code/digimon-engine/cards/bt16/`
- `code/digimon-engine/cards/bt17/`
- `code/digimon-engine/cards/bt20/`
- `code/digimon-engine/cards/bt21/`
- `code/digimon-engine/cards/bt22/`
- `code/digimon-engine/cards/bt24/`
- `code/digimon-engine/cards/ex7/`
- `code/digimon-engine/cards/ex8/`
- `code/digimon-engine/cards/ex9/`
- `code/digimon-engine/cards/ex10/`
- `code/digimon-engine/cards/ex11/`
- `code/digimon-engine/cards/lm/`
- `code/digimon-engine/cards/p/`
- `code/digimon-engine/tests/cards_behavioral/bt13/`
- `code/digimon-engine/tests/cards_behavioral/bt16/`
- `code/digimon-engine/tests/cards_behavioral/bt17/`
- `code/digimon-engine/tests/cards_behavioral/bt20/`
- `code/digimon-engine/tests/cards_behavioral/bt21/`
- `code/digimon-engine/tests/cards_behavioral/bt22/`
- `code/digimon-engine/tests/cards_behavioral/bt24/`
- `code/digimon-engine/tests/cards_behavioral/ex7/`
- `code/digimon-engine/tests/cards_behavioral/ex8/`
- `code/digimon-engine/tests/cards_behavioral/ex9/`
- `code/digimon-engine/tests/cards_behavioral/ex10/`
- `code/digimon-engine/tests/cards_behavioral/ex11/`
- `code/digimon-engine/tests/cards_behavioral/lm/`
- `code/digimon-engine/tests/cards_behavioral/p/`
- `code/digimon-engine/tests/effects/dsl_archetype_slice.rs`
- `code/digimon-engine/tests/dsl/group7_predicate_batch.rs`
- `code/digimon-engine/tests/dsl/delay.rs`
- `code/digimon-engine/tests/dsl/partition.rs`
- `code/digimon-engine/tests/dna_digivolve_user_action.rs`
- `code/digimon-engine/tests/cards_behavioral/tokens.rs`

Tracker files touched by every checkpoint:

- `docs/RUST_ENGINE_GAPS.md`
- `qa/archetype-qa/engine-gaps.md`
- `qa/dsl-vocab-gaps.md`

## Readiness Report Template

Each checkpoint report must include this exact section shape:

```markdown
# <Archetype> Rust DSL Unlock Check

**Date:** 2026-05-02
**Assessment target:** <deck_library archetype name, card pool source, and card count>
**Verdict:** ready | mostly-ready | blocked

## Capability Dependencies Checked

| Capability | Evidence command | Result | Residual limit |
|---|---|---|---|
| <capability name> | `<command>` | pass | <remaining blocker or `none`> |

## Card Readiness

| Card | Status | Evidence | Remaining blocker |
|---|---|---|---|
| `<CARD-ID>` | ready | `<test command>` | none |
| `<CARD-ID>` | blocked | tracker link | `<gap id>` |

## Raw-Rust and Placeholder Retirement

| Card | Raw/placeholder symbol | Replacement | Evidence |
|---|---|---|---|
| `<CARD-ID>` | `<symbol>` | DSL or hand-written Rust behavior | `<test command>` |

## Tracker Updates

- `docs/RUST_ENGINE_GAPS.md`: <exact entry names changed>
- `qa/archetype-qa/engine-gaps.md`: <exact gap ids changed>
- `qa/dsl-vocab-gaps.md`: <exact entries changed>
```

Use `none` for empty residual-limit cells. Do not leave blank cells.

## Task 1: Medusamon Unlock Checkpoint

**Files:**

- Modify: `qa/archetype-qa/dsl/medusamon.md`
- Modify: `qa/archetype-qa/dsl/medusamon-final-report.md`
- Modify: `qa/archetype-qa/medusa.md`
- Modify: `docs/RUST_ENGINE_GAPS.md`
- Modify: `qa/archetype-qa/engine-gaps.md`
- Modify: `qa/dsl-vocab-gaps.md`
- Modify: `code/digimon-engine/cards/bt21/BT21-008.yaml`
- Modify: `code/digimon-engine/cards/lm/LM-027.yaml`
- Test: `code/digimon-engine/tests/cards_behavioral/bt21/bt21_008.rs`
- Test: `code/digimon-engine/tests/cards_behavioral/lm/lm_027.rs`

- [ ] **Step 1: Re-run the readiness workflow**

Use `.codex/skills/assess-rust-engine-archetype/SKILL.md` and assess target `Medusamon`. The report must verify these known Medusamon rows from `qa/archetype-qa/dsl/medusamon.md`:

```text
BT21-008 inherited observer
BT21-017 inherited observer
BT24-012 inherited observer and replacement arm
ST22-08 Plug-In / Link
LM-027 Red Scramble
P-206 Digital Gate Open
EX7-074 Vortex Resonance
BT21-029 Medusamon deletion arm
BT24-017 Medusamon DP predicates
```

Record the findings in `qa/archetype-qa/dsl/medusamon.md` using the readiness report template.

- [ ] **Step 2: Add the failing inherited-observer unlock test**

If `code/digimon-engine/tests/cards_behavioral/bt21/bt21_008.rs` does not already contain an unignored test proving the inherited security-removed observer, add:

```rust
#[test]
fn bt21_008_unlock_inherited_security_removed_observer_is_runtime_ready() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-008")
        .dsl_card("BT21-017")
        .memory(0)
        .build();

    let carrier = runner.place_stack(0, &["BT21-008", "BT21-017"]);
    runner.trash_top_security_by_effect(1);
    runner.auto_resolve();

    assert_eq!(runner.game.memory, 1);
    assert!(
        runner
            .game
            .player(0)
            .battle_area
            .get(carrier.index as usize)
            .expect("carrier remains")
            .has_source_card_id("BT21-008", &runner.game.card_data)
    );
}
```

If helper names differ, use the existing `DebugRunner` helpers with the same setup semantics. Do not skip the test and do not mark it ignored.

Run to fail before migration:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt21_008_unlock_inherited_security_removed_observer_is_runtime_ready
```

Expected before the relevant capability is present: FAIL because the inherited observer does not fire, the YAML is still placeholder-backed, or the test helper must be adjusted.

- [ ] **Step 3: Add the failing Red Scramble start-of-turn Delay unlock test**

If `code/digimon-engine/tests/cards_behavioral/lm/lm_027.rs` does not already contain an unignored start-of-turn Delay test, add:

```rust
#[test]
fn lm_027_unlock_start_of_turn_delay_activates_after_placement_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("LM-027")
        .dsl_card("BT21-008")
        .hand(0, &["LM-027"])
        .hand(1, &["BT21-008"])
        .memory(5)
        .build();

    runner.play(1, 0).expect("opponent has a Digimon");
    runner.play(0, 0).expect("Red Scramble plays and places itself");

    assert!(
        runner.available_delay_actions(0).is_empty(),
        "Delay cannot activate on the turn the option was placed"
    );

    runner.end_turn();
    runner.end_turn();

    let actions = runner.available_delay_actions(0);
    assert_eq!(actions.len(), 1);
    runner.execute_action(actions[0]);
    runner.auto_resolve();

    assert!(
        runner.trash_contains(0, "LM-027"),
        "Delay activation trashes the placed option as its cost"
    );
}
```

Run to fail:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- lm_027_unlock_start_of_turn_delay_activates_after_placement_turn
```

- [ ] **Step 4: Retire only proven placeholders**

For every Medusamon card currently using `raw_rust` no-op or placeholder clauses, search:

```bash
Get-ChildItem -Path code/digimon-engine/cards -Recurse -Filter *.yaml | Select-String -Pattern 'raw_rust|noop|no-op' | Select-String -Pattern 'BT21-008|BT21-017|BT24-012|ST22-08|LM-027|P-206|EX7-074|BT21-029|BT24-017'
```

Replace a placeholder only when the new YAML has a passing card behavioral test. Keep unproven placeholders listed in the report under `Raw-Rust and Placeholder Retirement`.

- [ ] **Step 5: Run Medusamon verification**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt21_008
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- lm_027
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_predicate_batch
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- delay
```

Expected after migration: PASS for every newly-unignored test.

- [ ] **Step 6: Update trackers and commit**

Update only the rows proven by the commands above:

- `docs/RUST_ENGINE_GAPS.md`: inherited dispatch follow-up entries, Delay start-of-turn entry, DP predicate entries, and Option security disposition entries if closed.
- `qa/archetype-qa/engine-gaps.md`: `G-INHERITED-DISPATCH`, `G-OPT-TRIGGERED`, `G-DELAY-START-OF-TURN`, `G-ADD-OPTION-SELF-TO-HAND`, and any residual Medusamon gap ids.
- `qa/dsl-vocab-gaps.md`: Medusamon predicate and formula entries whose tests now pass.

Commit:

```bash
git add qa/archetype-qa/dsl/medusamon.md qa/archetype-qa/dsl/medusamon-final-report.md qa/archetype-qa/medusa.md docs/RUST_ENGINE_GAPS.md qa/archetype-qa/engine-gaps.md qa/dsl-vocab-gaps.md code/digimon-engine/cards code/digimon-engine/tests/cards_behavioral code/digimon-engine/tests/dsl
git commit -m "qa: unlock medusamon rust dsl readiness"
```

## Task 2: Rocks Unlock Checkpoint

**Files:**

- Create: `qa/archetype-qa/dsl/rocks.md`
- Modify: `qa/archetype-qa/rocks.md`
- Modify: `docs/RUST_ENGINE_GAPS.md`
- Modify: `qa/archetype-qa/engine-gaps.md`
- Modify: `qa/dsl-vocab-gaps.md`
- Modify: `code/digimon-engine/cards/ex10/EX10-032.yaml`
- Modify: `code/digimon-engine/cards/p/P-167.yaml`
- Modify: `code/digimon-engine/cards/ex10/EX10-003.yaml`
- Test: `code/digimon-engine/tests/cards_behavioral/ex10/ex10_032.rs`
- Test: `code/digimon-engine/tests/cards_behavioral/p/p_167.rs`
- Test: `code/digimon-engine/tests/cards_behavioral/ex10/ex10_003.rs`

- [ ] **Step 1: Re-run the readiness workflow**

Use `assess-rust-engine-archetype` on target `Rocks`. Include the core cards named by the roadmap and trackers:

```text
EX10-032 Proganomon
P-167 Landramon
EX10-036 Magneticdramon
EX10-033 Mineral/Rock payoff
EX8-067 Close
P-169 Close
EX11-065 Close
EX10-003 Tumblemon
BT14-009 Gotsumon
BT16-082 Ukkomon
P-206 Digital Gate Open
EX7-074 Vortex Resonance
```

Save the report to `qa/archetype-qa/dsl/rocks.md`.

- [ ] **Step 2: Add the failing source-selection unlock test**

Create `code/digimon-engine/tests/cards_behavioral/ex10/ex10_032.rs` if missing, and register it in `code/digimon-engine/tests/cards_behavioral/ex10/mod.rs`.

Add:

```rust
#[test]
fn ex10_032_unlock_trashes_chosen_mineral_or_rock_source_and_fires_source_trigger() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-032")
        .dsl_card("P-167")
        .dsl_card("EX8-047")
        .memory(0)
        .build();

    let first = runner.place_stack(0, &["P-167", "EX10-032"]);
    let _second = runner.place_stack(0, &["EX8-047", "EX10-032"]);

    runner.fire_when_digivolving(first);
    let view = runner
        .pending_selection_view()
        .expect("source selection must be visible to the action mask");

    assert!(
        view.valid_action_ids.len() >= 2,
        "Rocks source selection must see legal sources across own stacks"
    );

    runner.execute_action(view.valid_action_ids[0]);
    runner.auto_resolve();

    assert_eq!(
        runner.trash_count_card(0, "P-167") + runner.trash_count_card(0, "EX8-047"),
        1,
        "exactly the selected Mineral/Rock source is trashed"
    );
}
```

Run to fail:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex10_032_unlock_trashes_chosen_mineral_or_rock_source_and_fires_source_trigger
```

- [ ] **Step 3: Add the failing attack-cancel unlock test**

Create or extend `code/digimon-engine/tests/cards_behavioral/ex10/ex10_003.rs`:

```rust
#[test]
fn ex10_003_unlock_pays_three_rock_sources_then_ends_opponent_attack() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-003")
        .dsl_card("EX10-032")
        .dsl_card("P-167")
        .dsl_card("EX8-047")
        .memory(0)
        .build();

    let defender_stack = runner.place_stack(0, &["EX10-032", "P-167", "EX8-047", "EX10-003"]);
    let attacker = runner.place_on_field(1, "EX10-032", Some(0));

    runner.attack_player(attacker, 0);
    let view = runner
        .pending_selection_view()
        .expect("cost source selection is required before attack cancellation");
    assert_eq!(view.valid_action_ids.len(), 3);

    for action in view.valid_action_ids.clone() {
        runner.execute_action(action);
    }
    runner.auto_resolve();

    assert!(
        !runner.has_pending_attack(),
        "printed effect ends the attack after the source cost is paid"
    );
    assert_eq!(runner.source_count(defender_stack), 1);
}
```

Run to fail:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex10_003_unlock_pays_three_rock_sources_then_ends_opponent_attack
```

- [ ] **Step 4: Migrate ready Rocks YAML only**

Author or update YAML only for cards whose dependencies are proven by the failing tests and existing Group 2/3/5/6 tests. Keep a card blocked when it still needs source-scoped immunity, Plug-In/Link, board-color predicates, or option disposition not yet covered by tests.

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test selection -- source_multi
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cost_hooks -- pay_cost_selection
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- attack_cancel
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex10_032
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex10_003
```

Expected after migration: PASS.

- [ ] **Step 5: Update trackers and commit**

Update:

- `docs/RUST_ENGINE_GAPS.md`: Rocks refresh notes, source-selection residual limits, attack cancellation residual limits, Collision/source-immunity if proven.
- `qa/archetype-qa/engine-gaps.md`: Rocks `G-ROCKS-SOURCE-SELECTION-DSL` and attack-cancel references.
- `qa/dsl-vocab-gaps.md`: event-card predicate and Rocks source-selection entries.
- `qa/archetype-qa/dsl/rocks.md`: full card readiness table.

Commit:

```bash
git add qa/archetype-qa/dsl/rocks.md qa/archetype-qa/rocks.md docs/RUST_ENGINE_GAPS.md qa/archetype-qa/engine-gaps.md qa/dsl-vocab-gaps.md code/digimon-engine/cards/ex10 code/digimon-engine/cards/ex8 code/digimon-engine/cards/p code/digimon-engine/tests/cards_behavioral/ex10 code/digimon-engine/tests/cards_behavioral/p
git commit -m "qa: unlock rocks rust dsl readiness"
```

## Task 3: Royal Knights Unlock Checkpoint

**Files:**

- Create: `qa/archetype-qa/dsl/royal-knights.md`
- Modify: `qa/archetype-qa/royal-knights.md`
- Modify: `docs/RUST_ENGINE_GAPS.md`
- Modify: `qa/archetype-qa/engine-gaps.md`
- Modify: `qa/dsl-vocab-gaps.md`
- Modify: `code/digimon-engine/cards/bt13/BT13-007.yaml`
- Modify: `code/digimon-engine/cards/bt13/BT13-110.yaml`
- Modify: `code/digimon-engine/cards/bt20/BT20-083.yaml`
- Test: `code/digimon-engine/tests/cards_behavioral/bt13/bt13_007.rs`
- Test: `code/digimon-engine/tests/cards_behavioral/bt13/bt13_110.rs`
- Test: `code/digimon-engine/tests/cards_behavioral/bt20/bt20_083.rs`

- [ ] **Step 1: Re-run the readiness workflow**

Use `assess-rust-engine-archetype` on target `Royal Knights`. The report must check:

```text
BT13-007 King Drasil_7D6 breeding start-main trigger
BT13-110 Royal Knights of the Purge option placement
BT20-083 Omekamon breeding permanent selection
BT13-093 Omekamon
BT13-112 Omnimon
EX11-053 Omekamon
BT23-072 King Drasil_7D6
```

Save to `qa/archetype-qa/dsl/royal-knights.md`.

- [ ] **Step 2: Add the failing breeding-dispatch unlock test**

Create or extend `code/digimon-engine/tests/cards_behavioral/bt13/bt13_007.rs`:

```rust
#[test]
fn bt13_007_unlock_breeding_start_main_places_digitama_and_royal_knights_under_it() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-007")
        .dsl_card("BT13-110")
        .dsl_card("BT13-112")
        .digitama_deck(0, &["BT13-007"])
        .memory(0)
        .build();

    runner.place_in_breeding(0, "BT13-007");
    let rk = runner.place_on_field(0, "BT13-112", Some(0));

    runner.enter_main_phase(0);
    runner.auto_resolve();

    let breeding = runner
        .game
        .player(0)
        .breeding_area
        .as_ref()
        .expect("King Drasil remains in breeding");
    assert!(breeding.has_source_card_id("BT13-112", &runner.game.card_data));
    assert!(!runner.permanent_exists(rk));
}
```

Run to fail:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt13_007_unlock_breeding_start_main_places_digitama_and_royal_knights_under_it
```

- [ ] **Step 3: Add the failing option-placement observer test**

Extend `code/digimon-engine/tests/cards_behavioral/bt13/bt13_110.rs`:

```rust
#[test]
fn bt13_110_unlock_royal_knight_option_placement_triggers_king_drasil_inherited() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-007")
        .dsl_card("BT13-110")
        .hand(0, &["BT13-110"])
        .memory(5)
        .build();

    runner.place_stack_in_breeding(0, &["BT13-007", "BT13-007"]);
    runner.play(0, 0).expect("Royal Knights of the Purge plays");
    runner.auto_resolve();

    assert_eq!(
        runner.game.memory,
        6,
        "BT13-007 inherited on_option_placed should gain 1 memory once"
    );
}
```

Run to fail:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt13_110_unlock_royal_knight_option_placement_triggers_king_drasil_inherited
```

- [ ] **Step 4: Verify breeding selection and option placement contracts**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test selection -- breeding
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor -- breeding
cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_option_placed
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_option_placed
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt13_007
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt13_110
```

Expected after migration: PASS.

- [ ] **Step 5: Update trackers and commit**

Update Royal Knights report and tracker rows for `G-BREEDING-TRIGGER-DISPATCH`, `G-BREEDING-PERMANENT-SELECTION`, and `G-OPTION-PLACED-TIMING`. Keep stack-placement or material-play blockers open unless the card tests pass.

Commit:

```bash
git add qa/archetype-qa/dsl/royal-knights.md qa/archetype-qa/royal-knights.md docs/RUST_ENGINE_GAPS.md qa/archetype-qa/engine-gaps.md qa/dsl-vocab-gaps.md code/digimon-engine/cards/bt13 code/digimon-engine/cards/bt20 code/digimon-engine/tests/cards_behavioral/bt13 code/digimon-engine/tests/cards_behavioral/bt20
git commit -m "qa: unlock royal knights rust dsl readiness"
```

## Task 4: Puppets Unlock Checkpoint

**Files:**

- Create: `qa/archetype-qa/dsl/puppets.md`
- Modify: `qa/archetype-qa/Puppets.md`
- Modify: `docs/RUST_ENGINE_GAPS.md`
- Modify: `qa/archetype-qa/engine-gaps.md`
- Modify: `qa/dsl-vocab-gaps.md`
- Modify: `code/digimon-engine/cards/bt22/BT22-042.yaml`
- Modify: `code/digimon-engine/cards/bt22/BT22-098.yaml`
- Modify: `code/digimon-engine/cards/ex7/EX7-027.yaml`
- Modify: `code/digimon-engine/cards/ex11/EX11-024.yaml`
- Test: `code/digimon-engine/tests/cards_behavioral/bt22/bt22_042.rs`
- Test: `code/digimon-engine/tests/cards_behavioral/bt22/bt22_098.rs`
- Test: `code/digimon-engine/tests/cards_behavioral/tokens.rs`
- Test: `code/digimon-engine/tests/replacements/context_predicates.rs`

- [ ] **Step 1: Re-run the readiness workflow**

Use `assess-rust-engine-archetype` on target `Puppets`. The report must check:

```text
BT22-042 Nyabootmon
EX7-027 Chaperomon
BT22-036 Kazuchimon
EX7-030 Cendrillmon
EX11-024 Cendrillmon
BT22-098 Unique Emblem: Fable Waltz
P-229 Unique Emblem: Narrative Ronde
TOKEN_FAMILIAR
```

Save to `qa/archetype-qa/dsl/puppets.md`.

- [ ] **Step 2: Add the failing Familiar Token unlock test**

If Group 8 did not already add an unignored test in `code/digimon-engine/tests/cards_behavioral/tokens.rs`, add:

```rust
#[test]
fn familiar_token_unlock_on_deletion_selects_opponent_digimon_for_minus_3000() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-024")
        .memory(5)
        .build();

    let token = runner.play_token_for_test(0, "familiar").expect("token");
    let target = runner.place_on_field(1, "EX11-024", Some(0));

    runner.delete_permanent(token);
    let view = runner
        .pending_selection_view()
        .expect("Familiar token deletion must create target selection");
    assert_eq!(view.valid_action_ids.len(), 1);

    runner.execute_action(view.valid_action_ids[0]);
    runner.auto_resolve();

    assert_eq!(runner.game.effective_dp(target), Some(0));
}
```

Run to fail:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- familiar_token_unlock_on_deletion_selects_opponent_digimon_for_minus_3000
```

- [ ] **Step 3: Add the failing event-gated Delay unlock test**

Create or extend `code/digimon-engine/tests/cards_behavioral/bt22/bt22_098.rs`:

```rust
#[test]
fn bt22_098_unlock_delay_activates_when_arisa_suspends_after_placement_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-098")
        .dsl_card("BT22-088")
        .dsl_card("EX7-027")
        .hand(0, &["BT22-098", "EX7-027"])
        .memory(6)
        .build();

    runner.play(0, 0).expect("Unique Emblem places Delay option");
    runner.place_on_field(0, "BT22-088", Some(0));
    runner.end_turn();
    runner.end_turn();

    runner.suspend_named_permanent(0, "Arisa Kinosaki");
    let actions = runner.available_delay_actions(0);
    assert_eq!(actions.len(), 1);

    runner.execute_action(actions[0]);
    runner.auto_resolve();

    assert!(
        runner.trash_contains(0, "BT22-098"),
        "event-gated Delay activation trashes the option after firing"
    );
}
```

Run to fail:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt22_098_unlock_delay_activates_when_arisa_suspends_after_placement_turn
```

- [ ] **Step 4: Verify Overclock and replacement gates**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- familiar_token
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt22_098
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- context_predicates
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- replacement
cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- overclock
```

Expected after migration: PASS for the newly-unignored Puppet tests and the existing replacement predicate tests.

- [ ] **Step 5: Update trackers and commit**

Update `G-FAMILIAR-TOKEN-ON-DELETION`, `G-DELAY-EVENT-GATED`, `G-OVERCLOCK-TRAIT-FILTER`, and replacement cause predicate entries only when matching tests pass.

Commit:

```bash
git add qa/archetype-qa/dsl/puppets.md qa/archetype-qa/Puppets.md docs/RUST_ENGINE_GAPS.md qa/archetype-qa/engine-gaps.md qa/dsl-vocab-gaps.md code/digimon-engine/cards/bt22 code/digimon-engine/cards/ex7 code/digimon-engine/cards/ex11 code/digimon-engine/tests/cards_behavioral/bt22 code/digimon-engine/tests/cards_behavioral/tokens.rs code/digimon-engine/tests/replacements
git commit -m "qa: unlock puppets rust dsl readiness"
```

## Task 5: BG Imperial Unlock Checkpoint

**Files:**

- Modify: `qa/archetype-qa/dsl/bg-imperial.md`
- Modify: `qa/archetype-qa/bg-imperial.md`
- Modify: `docs/RUST_ENGINE_GAPS.md`
- Modify: `qa/archetype-qa/engine-gaps.md`
- Modify: `qa/dsl-vocab-gaps.md`
- Modify: `code/digimon-engine/cards/bt12/BT12-021.yaml`
- Modify: `code/digimon-engine/cards/bt12/BT12-047.yaml`
- Modify: `code/digimon-engine/cards/bt16/BT16-025.yaml`
- Modify: `code/digimon-engine/cards/lm/LM-030.yaml`
- Test: `code/digimon-engine/tests/cards_behavioral/bt12/bt12_021.rs`
- Test: `code/digimon-engine/tests/cards_behavioral/bt12/bt12_047.rs`
- Test: `code/digimon-engine/tests/cards_behavioral/bt16/bt16_025.rs`
- Test: `code/digimon-engine/tests/cards_behavioral/lm/lm_030.rs`
- Test: `code/digimon-engine/tests/dna_digivolve_user_action.rs`

- [ ] **Step 1: Re-run the readiness workflow**

Use `assess-rust-engine-archetype` on target `BG Imperial`. Check the existing report rows in `qa/archetype-qa/dsl/bg-imperial.md`, especially:

```text
BT12-021 Veemon
BT12-047 Wormmon
BT12-022 ExVeemon
BT12-050 Stingmon
ST9-05 Paildramon
BT12-028 Paildramon
BT16-025 Paildramon
BT16-027 Imperialdramon: Fighter Mode
BT16-028 Imperialdramon: Dragon Mode
LM-030 Green Scramble
BT17-097 Return to the Primogenitor
```

Update `qa/archetype-qa/dsl/bg-imperial.md` using the readiness report template.

- [ ] **Step 2: Add the failing inherited end-of-turn DNA registration test**

Create or extend `code/digimon-engine/tests/cards_behavioral/bt12/bt12_021.rs`:

```rust
#[test]
fn bt12_021_unlock_inherited_end_of_turn_dna_registers_player_action() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-021")
        .dsl_card("BT12-047")
        .dsl_card("BT16-025")
        .hand(0, &["BT16-025"])
        .memory(3)
        .build();

    runner.place_stack(0, &["BT12-021", "BT12-022"]);
    runner.place_stack(0, &["BT12-047", "BT12-050"]);
    runner.enter_end_of_turn_action_window(0);

    let mask = runner.game.get_action_mask(0);
    assert!(
        mask[digimon_engine::action::space::DNA_DIGIVOLVE_START as usize] > 0.5,
        "inherited end-of-turn DNA must be visible as a legal action"
    );
}
```

Run to fail:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt12_021_unlock_inherited_end_of_turn_dna_registers_player_action
```

- [ ] **Step 3: Add the failing Partition unlock test**

Create or extend `code/digimon-engine/tests/cards_behavioral/bt16/bt16_025.rs`:

```rust
#[test]
fn bt16_025_unlock_partition_requires_printed_blue_and_green_sources() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-025")
        .dsl_card("BT12-021")
        .dsl_card("BT12-047")
        .memory(0)
        .build();

    let paildramon = runner.place_stack(0, &["BT12-021", "BT12-047", "BT16-025"]);
    runner.delete_permanent(paildramon);
    let view = runner
        .pending_selection_view()
        .expect("Partition source choices must be exposed");
    assert_eq!(view.valid_action_ids.len(), 2);

    for action in view.valid_action_ids.clone() {
        runner.execute_action(action);
    }
    runner.auto_resolve();

    assert!(runner.battle_area_contains(0, "BT12-021"));
    assert!(runner.battle_area_contains(0, "BT12-047"));
    assert!(!runner.battle_area_contains(0, "BT16-025"));
}
```

Run to fail:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt16_025_unlock_partition_requires_printed_blue_and_green_sources
```

- [ ] **Step 4: Verify BG Imperial contracts**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dna_digivolve_user_action
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase3e_on_dna_digivolve
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- partition
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- partition
cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- replacement_integration
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt16_025
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- lm_030
```

Expected after migration: PASS.

- [ ] **Step 5: Update trackers and commit**

Update DNA metadata, inherited end-of-turn DNA, Partition, Green Scramble start-of-turn Delay, and Return to the Primogenitor replacement rows with passing commands.

Commit:

```bash
git add qa/archetype-qa/dsl/bg-imperial.md qa/archetype-qa/bg-imperial.md docs/RUST_ENGINE_GAPS.md qa/archetype-qa/engine-gaps.md qa/dsl-vocab-gaps.md code/digimon-engine/cards/bt12 code/digimon-engine/cards/bt16 code/digimon-engine/cards/lm code/digimon-engine/tests/cards_behavioral/bt12 code/digimon-engine/tests/cards_behavioral/bt16 code/digimon-engine/tests/cards_behavioral/lm code/digimon-engine/tests/dna_digivolve_user_action.rs
git commit -m "qa: unlock bg imperial rust dsl readiness"
```

## Task 6: Chaos Control and DNA Omnimon Unlock Checkpoint

**Files:**

- Create: `qa/archetype-qa/dsl/chaos-control.md`
- Create: `qa/archetype-qa/dsl/dna-omnimon.md`
- Modify: `qa/archetype-qa/chaos_control.md`
- Modify: `qa/archetype-qa/DNA_Omnimon.md`
- Modify: `docs/RUST_ENGINE_GAPS.md`
- Modify: `qa/archetype-qa/engine-gaps.md`
- Modify: `qa/dsl-vocab-gaps.md`
- Modify: `code/digimon-engine/cards/ex11/EX11-005.yaml`
- Modify: `code/digimon-engine/cards/bt24/BT24-080.yaml`
- Modify: `code/digimon-engine/cards/bt20/BT20-102.yaml`
- Modify: `code/digimon-engine/cards/bt17/BT17-078.yaml`
- Test: `code/digimon-engine/tests/cards_behavioral/ex11/ex11_005.rs`
- Test: `code/digimon-engine/tests/cards_behavioral/bt24/bt24_080.rs`
- Test: `code/digimon-engine/tests/cards_behavioral/bt20/bt20_102.rs`
- Test: `code/digimon-engine/tests/cards_behavioral/bt17/bt17_078.rs`

- [ ] **Step 1: Re-run both readiness workflows**

Use `assess-rust-engine-archetype` twice:

```text
Chaos Control
DNA Omnimon
```

Save reports to `qa/archetype-qa/dsl/chaos-control.md` and `qa/archetype-qa/dsl/dna-omnimon.md`.

The reports must cover:

```text
EX11-005 Yaamon
EX11-069 Yuuki
BT21-100 The Digimon I Designed
BT24-080 Megidramon
BT20-102 Omnimon (X Antibody)
BT17-078 Omnimon
BT22-013 WarGreymon
EX4-073 Omnimon Alter-B
```

- [ ] **Step 2: Add the failing non-hand digivolve unlock test**

Create or extend `code/digimon-engine/tests/cards_behavioral/ex11/ex11_005.rs`:

```rust
#[test]
fn ex11_005_unlock_effect_digivolves_from_trash_with_reduced_cost() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-005")
        .dsl_card("BT24-080")
        .trash(0, &["BT24-080"])
        .memory(5)
        .build();

    let base = runner.place_on_field(0, "EX11-005", Some(0));
    runner.fire_on_play(0, base.index as usize);
    let view = runner
        .pending_selection_view()
        .expect("trash digivolve must select the target card or base");

    runner.execute_action(view.valid_action_ids[0]);
    runner.auto_resolve();

    assert_eq!(runner.top_card_id(base), Some("BT24-080"));
}
```

Run to fail:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex11_005_unlock_effect_digivolves_from_trash_with_reduced_cost
```

- [ ] **Step 3: Add the failing self-stack predicate unlock test**

Create or extend `code/digimon-engine/tests/cards_behavioral/bt20/bt20_102.rs`:

```rust
#[test]
fn bt20_102_unlock_self_stack_name_condition_requires_omnimon_or_x_antibody_source() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-102")
        .dsl_card("BT5-086")
        .memory(10)
        .build();

    let without_source = runner.place_on_field(0, "BT20-102", Some(0));
    runner.fire_when_digivolving(without_source);
    runner.auto_resolve();
    assert!(
        !runner.boardwipe_happened(),
        "standalone Omnimon X Antibody must not satisfy its own source condition"
    );

    let with_source = runner.place_stack(0, &["BT5-086", "BT20-102"]);
    runner.fire_when_digivolving(with_source);
    runner.auto_resolve();
    assert!(runner.boardwipe_happened());
}
```

Run to fail:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt20_102_unlock_self_stack_name_condition_requires_omnimon_or_x_antibody_source
```

- [ ] **Step 4: Verify branch-choice and non-hand digivolve contracts**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase2e_select_effect_choice
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase3d_event_context
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_predicate_batch
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex11_005
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt20_102
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt17_078
```

Expected after migration: PASS.

- [ ] **Step 5: Update trackers and commit**

Update non-hand digivolve, self-stack predicates, branch-choice, and DNA Omnimon source/option predicate entries with exact commands.

Commit:

```bash
git add qa/archetype-qa/dsl/chaos-control.md qa/archetype-qa/dsl/dna-omnimon.md qa/archetype-qa/chaos_control.md qa/archetype-qa/DNA_Omnimon.md docs/RUST_ENGINE_GAPS.md qa/archetype-qa/engine-gaps.md qa/dsl-vocab-gaps.md code/digimon-engine/cards/ex11 code/digimon-engine/cards/bt24 code/digimon-engine/cards/bt20 code/digimon-engine/cards/bt17 code/digimon-engine/tests/cards_behavioral/ex11 code/digimon-engine/tests/cards_behavioral/bt24 code/digimon-engine/tests/cards_behavioral/bt20 code/digimon-engine/tests/cards_behavioral/bt17
git commit -m "qa: unlock chaos control and dna omnimon readiness"
```

## Task 7: Dark Masters and Remaining Audits Checkpoint

**Files:**

- Create: `qa/archetype-qa/dsl/dark-masters.md`
- Modify: `qa/archetype-qa/Dark_Masters.md`
- Modify: `qa/archetype-qa/INDEX.md`
- Modify: `docs/RUST_ENGINE_GAPS.md`
- Modify: `qa/archetype-qa/engine-gaps.md`
- Modify: `qa/dsl-vocab-gaps.md`
- Modify: `code/digimon-engine/cards/ex10/EX10-012.yaml`
- Modify: `code/digimon-engine/cards/ex10/EX10-020.yaml`
- Modify: `code/digimon-engine/cards/ex10/EX10-035.yaml`
- Modify: `code/digimon-engine/cards/ex10/EX10-057.yaml`
- Modify: `code/digimon-engine/cards/ex10/EX10-061.yaml`
- Test: `code/digimon-engine/tests/cards_behavioral/ex10/ex10_012.rs`
- Test: `code/digimon-engine/tests/cards_behavioral/ex10/ex10_020.rs`
- Test: `code/digimon-engine/tests/cards_behavioral/ex10/ex10_035.rs`
- Test: `code/digimon-engine/tests/cards_behavioral/ex10/ex10_057.rs`
- Test: `code/digimon-engine/tests/cards_behavioral/ex10/ex10_061.rs`

- [ ] **Step 1: Re-run the Dark Masters readiness workflow**

Use `assess-rust-engine-archetype` on target `Dark Masters`. Save the report to `qa/archetype-qa/dsl/dark-masters.md`.

The report must cover:

```text
EX10-012 MetalSeadramon
EX10-020 Puppetmon
EX10-035 Machinedramon
EX10-057 Piedmon
EX10-061 Apocalymon
BT19-075 MoonMillenniummon
BT4-097 Kari Kamiya
ST6-14 Matt Ishida
BT8-094 Digimon Emperor
```

- [ ] **Step 2: Add the failing deletion/enter-field observer unlock test**

Create or extend `code/digimon-engine/tests/cards_behavioral/ex10/ex10_061.rs`:

```rust
#[test]
fn ex10_061_unlock_global_deletion_and_enter_field_observers_do_not_auto_select() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-061")
        .dsl_card("EX10-012")
        .dsl_card("EX10-020")
        .memory(10)
        .build();

    let observer = runner.place_on_field(0, "EX10-061", Some(0));
    let victim = runner.place_on_field(1, "EX10-012", Some(0));

    runner.delete_permanent(victim);
    let view = runner
        .pending_selection_view()
        .expect("global deletion observer must expose its printed branch");

    assert!(
        !view.valid_action_ids.is_empty(),
        "observer branch must be visible through the action mask"
    );

    runner.execute_action(view.valid_action_ids[0]);
    runner.auto_resolve();

    assert!(runner.permanent_exists(observer));
}
```

Run to fail:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex10_061_unlock_global_deletion_and_enter_field_observers_do_not_auto_select
```

- [ ] **Step 3: Run remaining audit sweep**

Run a repository search for unresolved audit files and update `qa/archetype-qa/INDEX.md` with Rust DSL report links:

```bash
Get-ChildItem -Path qa/archetype-qa -File -Filter *.md | Select-String -Pattern 'BLOCKED|engine-gap|dsl-gap|raw_rust|no-op'
Get-ChildItem -Path qa/archetype-qa\\dsl -File -Filter *.md | Select-Object -ExpandProperty Name
```

Every launch archetype that has a Rust DSL readiness report should be linked from `qa/archetype-qa/INDEX.md`. Archetypes without reports stay linked only in the legacy table and must be listed under a `Rust DSL reports not yet generated` heading.

- [ ] **Step 4: Verify Dark Masters and observer contracts**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_any
cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_enter
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex10_061
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex10_012
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex10_020
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex10_035
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex10_057
```

Expected after migration: PASS.

- [ ] **Step 5: Update trackers and commit**

Update global observer coverage, deletion/enter-field observer rows, Dark Masters report, and `qa/archetype-qa/INDEX.md`.

Commit:

```bash
git add qa/archetype-qa/dsl/dark-masters.md qa/archetype-qa/Dark_Masters.md qa/archetype-qa/INDEX.md docs/RUST_ENGINE_GAPS.md qa/archetype-qa/engine-gaps.md qa/dsl-vocab-gaps.md code/digimon-engine/cards/ex10 code/digimon-engine/tests/cards_behavioral/ex10
git commit -m "qa: unlock dark masters rust dsl readiness"
```

## Task 8: Cross-Archetype Acceptance Sweep

**Files:**

- Modify: `qa/archetype-qa/INDEX.md`
- Modify: `docs/RUST_ENGINE_GAPS.md`
- Modify: `qa/archetype-qa/engine-gaps.md`
- Modify: `qa/dsl-vocab-gaps.md`
- Modify: every `qa/archetype-qa/dsl/*.md` file created or updated by Tasks 1-7

- [ ] **Step 1: Run targeted regression suite**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt21_008
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex10_032
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt13_007
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt22_098
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt16_025
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex11_005
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex10_061
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_predicate_batch
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- delay
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- partition
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dna_digivolve_user_action
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor
```

Expected: every command passes. If a command fails because a card remains blocked, keep that card's status as `blocked` and reference the exact failing command in the report.

- [ ] **Step 2: Run full Rust and contract smoke tests**

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml
DIGIMON_BACKEND=rust python -m pytest code/engine_py_legacy/tests/engine/test_rust_backend_parity.py -v
python -m pytest code/tests/rl -v
```

If PyO3 bindings are missing, run:

```bash
cd code/digimon-engine-py
maturin develop
cd ../..
DIGIMON_BACKEND=rust python -m pytest code/engine_py_legacy/tests/engine/test_rust_backend_parity.py -v
```

Record any unavailable Python dependency or build-tool failure in the final Group 9 report.

- [ ] **Step 3: Verify placeholder and raw-Rust retirement state**

```bash
Get-ChildItem -Path code/digimon-engine/cards -Recurse -Filter *.yaml | Select-String -Pattern 'raw_rust|noop|no-op'
Get-ChildItem -Path code/digimon-engine/tests -Recurse -Filter *.rs | Select-String -Pattern '#\\[ignore|pending:'
```

For every remaining match, ensure one of these is true:

- A tracker row names the exact remaining capability blocker.
- The matching card is outside the Group 9 assessed archetype set.
- The test is an intentional negative or fixture test that does not mask a card behavior.

- [ ] **Step 4: Markdown and contract self-review**

```bash
$patterns = @(
  [string]::new([char[]](84,66,68)),
  [string]::new([char[]](84,79,68,79)),
  ('implement' + ' later'),
  ('fill in ' + 'details')
)
Select-String -Path 'docs/superpowers/plans/2026-05-02-gap-group-9-archetype-unlocks.md','qa/archetype-qa/dsl/*.md' -Pattern $patterns
git diff --check
git status --short
```

Expected: no placeholder-pattern output from the plan or QA reports, no whitespace errors, and only Group 9 files in `git status`.

- [ ] **Step 5: Final tracker closure commit**

```bash
git add qa/archetype-qa/INDEX.md qa/archetype-qa/dsl docs/RUST_ENGINE_GAPS.md qa/archetype-qa/engine-gaps.md qa/dsl-vocab-gaps.md
git commit -m "qa: close archetype unlock checkpoint sweep"
```

## Completion Criteria

Group 9 is complete when:

- Every checkpoint report exists and uses the readiness report template.
- Medusamon, Rocks, Royal Knights, Puppets, BG Imperial, Chaos Control, DNA Omnimon, and Dark Masters each have a current Rust DSL verdict.
- Every `ready` card has a passing behavioral or DSL test command in its report.
- Every `blocked` card cites an exact engine, DSL, data, rules, or test gap.
- Raw-Rust and no-op placeholders are retired only where a passing card-level test proves the replacement behavior.
- `docs/RUST_ENGINE_GAPS.md`, `qa/archetype-qa/engine-gaps.md`, and `qa/dsl-vocab-gaps.md` agree on which reusable gaps are resolved and which remain.
- Action masks and pending selections expose every new player-visible choice exercised by the unlock tests.
- `ACTION_SPACE_SIZE = 2168` and `TENSOR_SIZE = 1375` remain unchanged unless a separate contract-changing plan updates all Rust, PyO3, RL, frontend, and docs surfaces in the same change.
