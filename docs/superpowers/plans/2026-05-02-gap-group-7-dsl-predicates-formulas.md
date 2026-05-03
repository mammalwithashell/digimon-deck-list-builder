# Gap Group 7: DSL Predicate, Formula, and Lowering Coverage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the Rust card DSL so predicate, formula, and lowering-only blockers can be expressed declaratively without raw-Rust escape hatches.

**Architecture:** Add narrow parser, compiler, compiled-IR, evaluator, lowerer, and DSL infra tests for each missing reusable vocabulary item. Pure predicate and formula gaps land first because they do not resize action space; hybrid slices are gated behind the relevant engine primitives and must prove behavior with DSL-driven tests, not just parse tests. Current `main` already landed the adjacent Group 5, Group 6, and Group 8 primitives, so this plan treats those as baseline coverage and keeps only residual predicate/formula and inherited alt-path registration work in Group 7.

**Tech Stack:** Rust (`code/digimon-dsl`, `code/digimon-engine`), YAML card DSL, Cargo integration tests, markdown gap trackers.

---

## Scope Note

This plan owns DSL predicate/formula/lowering vocabulary only. Do not run tasks that mutate shared action-space, pending-selection, replacement-window, Option/Delay state, aura-query storage, or `CardData` shape in parallel with implementation plans for Groups 2, 3, 5, 6, or 8 touching the same surfaces.

Already-landed Group 7 slices must be protected, not reimplemented:

- `dp_lte` / `dp_gte` runtime evaluation works for literal thresholds and existing `FormulaSpec` values.
- `not_in_binding` parses, compiles, evaluates, and threads through `for_each` / modifier filters.
- Static `play_cost_lte` works for card and permanent predicates and selection filters.
- Basic `event_card_trait_has`, `event_target_trait_has`, and replacement cause/controller predicates exist for the event/replacement contexts already supplied by the engine.

Mainline updates after this plan was first drafted must also be protected:

- Group 5 closes reusable Option/Delay/Link state: Delay windows, replacement-aware Delay, Link registration/action DSL, linked-card scope dispatch, inherited-security placement, `OnOptionPlaced` fan-out, Training scope, and transient Standard Option replay. Do not reopen these generic primitives from this plan; only migrate stale card YAML/tests once their Group 7 predicate/formula blockers are also gone.
- Group 6 closes dynamic formula-backed aura fields. `kind: aura` now accepts `dp_modifier_fn` and `security_attack_fn`, and runtime DP/security queries recompute those formulas live. Keep regression coverage, but do not implement another Group 7 aura storage path.
- Group 8 closes top-level printed DNA authoring through `alt_paths: [{ kind: dna_digivolve, ... }]` into `CardData.dna_costs`, and existing DNA action masks observe that metadata. The remaining Group 7-adjacent DNA item is inherited/end-of-turn `alt_path_registration` lowering, not top-level `dna_costs` authoring.
- Group 8 also closes scoped DigiXros aliases, ACE Overflow metadata, token card data, and reveal overlays. Leave those out of Group 7 unless a card-specific predicate/formula blocker still depends on them.

This plan must not add new raw-Rust callbacks as the final implementation state. If a slice cannot be implemented faithfully because an engine primitive is missing, leave the tracker open with that dependency named and keep affected card YAML on an explicit blocked path.

## File Structure

Likely DSL schema and compiler files:

- Modify: `code/digimon-dsl/src/predicate.rs`
- Modify: `code/digimon-dsl/src/formula.rs`
- Modify: `code/digimon-dsl/src/clause.rs`
- Modify: `code/digimon-dsl/src/compiled.rs`
- Modify: `code/digimon-dsl/src/compile.rs`
- Modify: `code/digimon-dsl/src/step.rs`
- Modify: `code/digimon-dsl/src/validator.rs`

Likely Rust lowering and evaluator files:

- Modify: `code/digimon-engine/src/dsl_cards/predicate.rs`
- Modify: `code/digimon-engine/src/dsl_cards/formula_eval.rs`
- Modify: `code/digimon-engine/src/dsl_cards/lower_triggered.rs`
- Modify: `code/digimon-engine/src/dsl_cards/lower_replacement.rs`
- Modify: `code/digimon-engine/src/dsl_cards/lower_aura.rs`
- Modify: `code/digimon-engine/src/dsl_cards/lower_cost_reduction.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/permanent_scan.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/iteration.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs`
- Modify: `code/digimon-engine/src/dsl_registry.rs`

Likely files for the residual inherited alt-path registration slice:

- Modify: `code/digimon-engine/src/dsl_cards/mod.rs`
- Create or modify: `code/digimon-engine/src/dsl_cards/lower_alt_path_registration.rs`
- Modify: `code/digimon-engine/src/effect.rs` only if a new runtime registration effect is required.
- Modify: `code/digimon-engine/src/action/mask.rs` and `code/digimon-engine/src/game_actions.rs` only if inherited registration changes how existing DNA action IDs become legal.
- Modify: `code/digimon-engine-py/src/lib.rs` only if exposed action or card-data constants change.

Likely tests:

- Modify: `code/digimon-engine/tests/dsl/group7_predicate_batch.rs`
- Modify: `code/digimon-engine/tests/dsl/parse_predicates.rs`
- Modify: `code/digimon-engine/tests/dsl/parse_formulas.rs`
- Modify: `code/digimon-engine/tests/dsl/phase3d_event_context.rs`
- Modify: `code/digimon-engine/tests/dsl/phase3d_formula_zone_count.rs`
- Modify: `code/digimon-engine/tests/dsl/phase2f2_formula_eval.rs`
- Modify: `code/digimon-engine/tests/dsl/phase2f2_modifier_formula.rs`
- Create or modify: `code/digimon-engine/tests/dsl/group7_formula_batch.rs`
- Create or modify: `code/digimon-engine/tests/dsl/group7_alt_path_registration.rs`
- Modify: `code/digimon-engine/tests/dsl/group6_dynamic_formulas.rs` only as regression evidence if aura behavior is accidentally touched.
- Modify: `code/digimon-engine/tests/dna_digivolve_user_action.rs`
- Modify production YAML only after a reusable slice passes, for example `code/digimon-engine/cards/p/P-206.yaml`, `code/digimon-engine/cards/bt8/BT8-097.yaml`, or `code/digimon-engine/cards/bt24/BT24-080.yaml` when present.

Tracker files:

- Modify: `docs/RUST_ENGINE_GAPS.md`
- Modify: `qa/archetype-qa/engine-gaps.md`
- Modify: `qa/dsl-vocab-gaps.md`
- Modify relevant archetype docs under `qa/archetype-qa/` and `qa/archetype-qa/dsl/` only when a real archetype blocker is narrowed or closed.

## Global Verification Rules

- [ ] Run the specific failing test before implementation and confirm it fails for the expected missing vocabulary, evaluator, or lowerer path.
- [ ] Keep `ACTION_SPACE_SIZE = 2168` unless a slice explicitly introduces a new player-visible choice. Most Group 7 slices should reuse existing selection/action surfaces and therefore require no action-space, tensor, PyO3, RL, or frontend constant changes.
- [ ] If any implementation does add or change a player-visible choice, update `docs/ACTION_SPEC.md`, `docs/TENSOR_SPEC.md` when tensor semantics change, `code/digimon-engine-py/src/lib.rs`, `code/digimon_gym/digimon_gym.py`, and relevant frontend constants in the same slice.
- [ ] Close a tracker entry only after schema, compiler, evaluator/lowerer, runtime behavior, and docs all match the evidence. Otherwise narrow the remaining blocker.
- [ ] Commit after each task with only that task's source, tests, and tracker edits staged.

## Task 1: Baseline Existing Group 7 Coverage and Tracker Corrections

**Files:**
- Modify: `qa/dsl-vocab-gaps.md`
- Modify: `docs/RUST_ENGINE_GAPS.md`
- Modify: `qa/archetype-qa/engine-gaps.md`
- Test: `code/digimon-engine/tests/dsl/group7_predicate_batch.rs`

- [ ] **Step 1: Run the existing Group 7 predicate regression tests**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_predicate_batch
```

Expected: PASS. This proves the already-landed `dp_lte` / `dp_gte`, `not_in_binding`, and static `play_cost_lte` slices remain alive.

- [ ] **Step 2: Run the supporting formula and event-context regressions**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase3d_formula_zone_count
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase3d_event_context
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- replacement
cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- group6_dynamic_formulas
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- dsl_dna_alt_path_enriches_card_data_dna_costs
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dna_digivolve_user_action -- authored_dna_alt_path_makes_dna_action_legal_for_bt20_016
```

Expected: PASS or a targeted failure that identifies a regression in an already-claimed slice. Fix regressions before continuing to new vocabulary. The Group 6 dynamic aura and Group 8 DNA-alt-path commands are baseline evidence from current `main`; failures there mean the plan should be paused for regression repair rather than re-implementing those features inside Group 7.

- [ ] **Step 3: Update tracker wording for partial closures**

Edit `qa/dsl-vocab-gaps.md` so these entries do not read as fully open:

```text
- EX4-011 shared trash: keep open only for `shared_trash_count` / bucket formula.
- BT20-102 not_in_binding: keep closed for predicate/filter support; leave card-specific raw-rust retirement to the card's remaining blockers.
- P-189 / play-cost filters if still mentioned in archetype notes: state that static `play_cost_lte` selection filters are closed.
- G-AURA-DP-FORMULA: keep closed under Group 6 for `dp_modifier_fn` and `security_attack_fn`; do not carry it as a Group 7 implementation task.
- BG Imperial top-level DNA authoring: keep closed for `alt_paths: kind: dna_digivolve` populating `CardData.dna_costs`; keep inherited/end-of-turn `alt_path_registration` lowering open.
```

Do not mark `G-COLOR-MATCH-AGAINST-BOARD`, `G-FORMULA-KIND-FILTER`, same-level pair formulas, shared-trash formulas, binding-DP formulas, or inherited `alt_path_registration` lowering as closed in this step.

- [ ] **Step 4: Verify tracker edits**

Run:

```bash
git diff --check -- qa/dsl-vocab-gaps.md docs/RUST_ENGINE_GAPS.md qa/archetype-qa/engine-gaps.md
```

Expected: no output.

- [ ] **Step 5: Commit**

```bash
git add qa/dsl-vocab-gaps.md docs/RUST_ENGINE_GAPS.md qa/archetype-qa/engine-gaps.md
git commit -m "docs: refresh group 7 dsl gap status"
```

## Task 2: Board-Color Cross-Reference Predicate

**Files:**
- Modify: `code/digimon-dsl/src/predicate.rs`
- Modify: `code/digimon-dsl/src/compiled.rs`
- Modify: `code/digimon-dsl/src/compile.rs`
- Modify: `code/digimon-engine/src/dsl_cards/predicate.rs`
- Modify: `code/digimon-engine/tests/dsl/group7_predicate_batch.rs`
- Docs: `qa/dsl-vocab-gaps.md`

- [ ] **Step 1: Write the failing parse/compile test**

Add to `code/digimon-engine/tests/dsl/group7_predicate_batch.rs`:

```rust
#[test]
fn color_matches_any_field_digimon_compiles() {
    let yaml = r#"
card: T-G7-COLOR
name: Board Color Predicate
kind: option
color: [white]
cost: 0
effects:
  - when: main_from_hand
    process:
      - select_hand:
          of: you
          bind_as: tamer
          prompt: Pick matching Tamer
          filter:
            all_of:
              - kind: tamer
              - color_matches_any_field_digimon: { of: you }
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse yaml");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile yaml");
    let digimon_dsl::compiled::CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let digimon_dsl::compiled::CompiledStep::SelectHand { filter, .. } = &triggered.process[0] else {
        panic!("expected select_hand");
    };
    assert!(filter
        .all_of
        .iter()
        .any(|p| p.color_matches_any_field_digimon == Some(digimon_dsl::compiled::CompiledPlayerRef::You)));
}
```

- [ ] **Step 2: Run the parse/compile test to verify it fails**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- color_matches_any_field_digimon_compiles
```

Expected: FAIL because `CompiledPredicate` has no `color_matches_any_field_digimon` field, or because the parsed field is captured in `extra` and never compiled.

- [ ] **Step 3: Write the failing runtime selection test**

Add to `code/digimon-engine/tests/dsl/group7_predicate_batch.rs`:

```rust
#[test]
fn select_hand_color_matches_any_field_digimon_filters_by_live_board_colors() {
    use digimon_dsl::compiled::{CompiledCardKind, CompiledPlayerRef, CompiledPredicate, CompiledStep};
    use digimon_engine::action::space::PLAY_HAND_START;
    use digimon_engine::debug_runner::{make_test_card, DebugRunner};
    use digimon_engine::dsl_cards::bindings::Bindings;
    use digimon_engine::dsl_cards::step::run_steps;
    use digimon_engine::effect_context::EffectContext;
    use digimon_engine::enums::{CardColor, CardKind};

    let mut red_digimon = make_test_card("RED-DIGI", "Red Digimon");
    red_digimon.colors = vec![CardColor::Red];
    red_digimon.card_kind = CardKind::Digimon;
    let mut red_tamer = make_test_card("RED-TAMER", "Red Tamer");
    red_tamer.colors = vec![CardColor::Red];
    red_tamer.card_kind = CardKind::Tamer;
    let mut blue_tamer = make_test_card("BLUE-TAMER", "Blue Tamer");
    blue_tamer.colors = vec![CardColor::Blue];
    blue_tamer.card_kind = CardKind::Tamer;

    let mut runner = DebugRunner::builder()
        .add_card(red_digimon)
        .add_card(red_tamer)
        .add_card(blue_tamer)
        .hand(0, &["RED-DIGI", "RED-TAMER", "BLUE-TAMER"])
        .build();
    let source = runner.game.players[0].hand[0].handle();
    runner.place_on_field(0, "RED-DIGI", None);

    let steps = vec![CompiledStep::SelectHand {
        of: CompiledPlayerRef::You,
        filter: CompiledPredicate {
            kind: Some(CompiledCardKind::Tamer),
            color_matches_any_field_digimon: Some(CompiledPlayerRef::You),
            ..Default::default()
        },
        bind_as: Some("pick".to_string()),
        prompt: "Pick matching Tamer".to_string(),
        prompt_key: None,
        optional: false,
    }];
    let mut ctx = EffectContext::new(&mut runner.game, source, None, 0);
    let mut bindings = Bindings::new();
    run_steps(&steps, &mut ctx, &mut bindings);

    let pending = runner.game.pending_selection.as_ref().expect("selection");
    assert_eq!(pending.valid_action_ids, vec![PLAY_HAND_START + 1]);
}
```

- [ ] **Step 4: Run the runtime test to verify it fails**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- select_hand_color_matches_any_field_digimon_filters_by_live_board_colors
```

Expected: FAIL because the compiled/evaluator field does not exist or because both Tamers remain legal.

- [ ] **Step 5: Implement the predicate**

Make these changes:

```text
1. Add `color_matches_any_field_digimon: Option<PlayerRef>` to `PredicateSpec` in `code/digimon-dsl/src/predicate.rs`.
2. Add `color_matches_any_field_digimon: Option<CompiledPlayerRef>` to `CompiledPredicate` in `code/digimon-dsl/src/compiled.rs`.
3. In `compile_predicate` in `code/digimon-dsl/src/compile.rs`, map the new field through `compile_player_ref`.
4. In `eval_card_fields` in `code/digimon-engine/src/dsl_cards/predicate.rs`, collect colors from Digimon top cards in the requested player's battle area and return false unless the candidate card shares at least one color.
5. Keep empty-board behavior false: a card cannot match "same color as any of your Digimon" when no Digimon are present.
```

- [ ] **Step 6: Run tests to verify they pass**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- color_matches_any_field_digimon_compiles
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- select_hand_color_matches_any_field_digimon_filters_by_live_board_colors
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_predicate_batch
```

Expected: PASS.

- [ ] **Step 7: Update tracker**

In `qa/dsl-vocab-gaps.md`, mark `G-COLOR-MATCH-AGAINST-BOARD` resolved only for dynamic board-color card predicates. Leave unrelated P-206 Delay, Option, or action-flow blockers open.

- [ ] **Step 8: Commit**

```bash
git add code/digimon-dsl/src/predicate.rs code/digimon-dsl/src/compiled.rs code/digimon-dsl/src/compile.rs code/digimon-engine/src/dsl_cards/predicate.rs code/digimon-engine/tests/dsl/group7_predicate_batch.rs qa/dsl-vocab-gaps.md
git commit -m "feat: add board color dsl predicate"
```

## Task 3: Formula Sources for Shared Trash, Filtered Zone Counts, and Binding DP

**Files:**
- Modify: `code/digimon-dsl/src/formula.rs`
- Modify: `code/digimon-dsl/src/predicate.rs`
- Modify: `code/digimon-dsl/src/compiled.rs`
- Modify: `code/digimon-dsl/src/compile.rs`
- Modify: `code/digimon-engine/src/dsl_cards/formula_eval.rs`
- Modify: `code/digimon-engine/src/dsl_cards/predicate.rs`
- Create or modify: `code/digimon-engine/tests/dsl/group7_formula_batch.rs`
- Docs: `qa/dsl-vocab-gaps.md`

- [ ] **Step 1: Write failing parse/compile tests for the three formula shapes**

Create `code/digimon-engine/tests/dsl/group7_formula_batch.rs` or add to it:

```rust
use digimon_dsl::compiled::{CompiledFormula, CompiledPerSelector};

#[test]
fn shared_trash_count_formula_compiles() {
    let yaml = r#"
card: T-G7-SHARED-TRASH
name: Shared Trash Formula
kind: option
color: [white]
cost: 0
effects:
  - when: main_from_hand
    process:
      - select_opponent_permanent:
          bind_as: target
          prompt: Pick
          filter:
            dp_lte:
              formula:
                base: 7000
                per:
                  shared_trash_count: {}
                bucket: 10
                delta: 2000
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse yaml");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile yaml");
    let digimon_dsl::compiled::CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let digimon_dsl::compiled::CompiledStep::SelectOpponentPermanent { filter, .. } =
        &triggered.process[0]
    else {
        panic!("expected select_opponent_permanent");
    };
    let Some(digimon_dsl::compiled::CompiledDpConstraint::Formula(CompiledFormula::BasePerDelta { per, .. })) =
        &filter.dp_lte
    else {
        panic!("expected formula dp_lte");
    };
    assert!(matches!(per, CompiledPerSelector::SharedTrashCount { bucket: Some(10) }));
}

#[test]
fn binding_dp_formula_compiles() {
    let yaml = r#"
card: T-G7-BINDING-DP
name: Binding DP Formula
kind: option
color: [white]
cost: 0
effects:
  - when: main_from_hand
    process:
      - select_own_permanent:
          bind_as: ally
          prompt: Ally
          filter: { kind: digimon }
      - select_opponent_permanent:
          bind_as: target
          prompt: Target
          filter:
            dp_lte:
              formula:
                binding_dp: ally
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse yaml");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile yaml");
    assert_eq!(compiled.effects.len(), 1);
}

#[test]
fn card_count_in_zone_formula_accepts_filter() {
    let yaml = r#"
card: T-G7-FILTERED-COUNT
name: Filtered Count
kind: option
color: [white]
cost: 0
effects:
  - kind: cost_reduction
    amount_fn:
      base: 0
      per:
        card_count_in_zone:
          zone: battle_area
          of: opponent
          filter: { kind: digimon }
      delta: 1
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse yaml");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile yaml");
    assert_eq!(compiled.effects.len(), 1);
}
```

- [ ] **Step 2: Register the test module**

If creating a new file, add it to `code/digimon-engine/tests/dsl/main.rs`:

```rust
mod group7_formula_batch;
```

- [ ] **Step 3: Run parse/compile tests to verify they fail**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_formula_batch
```

Expected: FAIL because the formula schema lacks `shared_trash_count`, `binding_dp`, and filtered `card_count_in_zone` payloads.

- [ ] **Step 4: Add runtime evaluator tests**

Extend `group7_formula_batch.rs` with evaluator tests:

```rust
#[test]
fn shared_trash_count_bucket_formula_evaluates() {
    use digimon_dsl::compiled::{CompiledFormula, CompiledPerSelector};
    use digimon_engine::card_source::CardSource;
    use digimon_engine::debug_runner::{make_test_card, DebugRunner};
    use digimon_engine::dsl_cards::formula_eval;
    use digimon_engine::effect_context::EffectContext;

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("FILL", "Filler"))
        .build();
    let target = runner.place_on_field(0, "SRC", None);
    let data_idx = runner.game.card_data.iter().position(|c| c.card_id == "FILL").unwrap();
    for owner in [0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1] {
        let next = runner.game.next_card_index();
        runner.game.players[owner].trash.push(CardSource::new(data_idx, owner as u8, next));
    }
    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, Some(target), 0);
    let formula = CompiledFormula::BasePerDelta {
        base: 7000,
        per: CompiledPerSelector::SharedTrashCount { bucket: Some(10) },
        delta: 2000,
    };
    assert_eq!(formula_eval::evaluate(&formula, &ctx, target), 9000);
}

#[test]
fn binding_dp_formula_reads_bound_permanent_effective_dp() {
    use digimon_dsl::compiled::CompiledFormula;
    use digimon_engine::debug_runner::{make_test_card, DebugRunner};
    use digimon_engine::dsl_cards::bindings::Bindings;
    use digimon_engine::dsl_cards::formula_eval;
    use digimon_engine::effect_context::EffectContext;
    use digimon_engine::enums::Expiry;

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("ALLY", "Ally"))
        .build();
    let src = runner.place_on_field(0, "SRC", None);
    let ally = runner.place_on_field(0, "ALLY", None);
    runner.game.add_dp_modifier(ally, 3000, Expiry::EndOfTurn);
    let mut bindings = Bindings::new();
    bindings.insert_permanent("ally", ally);
    let src_card = runner.game.players[0].battle_area[src.index as usize].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, Some(src), 0);
    let formula = CompiledFormula::BindingDp("ally".to_string());
    assert_eq!(formula_eval::evaluate_with_bindings(&formula, &ctx, src, &bindings), 6000);
}
```

The exact `add_dp_modifier` helper call may need to use the repository's current DebugRunner or Game method name. Keep the assertion on effective DP, not base DP.

- [ ] **Step 5: Run runtime tests to verify they fail**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- shared_trash_count_bucket_formula_evaluates
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- binding_dp_formula_reads_bound_permanent_effective_dp
```

Expected: FAIL because compiled formula variants and binding-aware formula evaluation are missing.

- [ ] **Step 6: Implement formula schema, IR, compiler, and evaluator**

Make these changes:

```text
1. In `formula.rs`, add:
   - `PerSelector::SharedTrashCount { bucket: Option<u32> }`
   - a filtered `CardCountInZoneSpec { zone, of, filter: Option<Box<PredicateSpec>> }`
   - `FormulaSpec::BindingDp(String)` or an equivalent unambiguous YAML shape.
2. In `compiled.rs`, add:
   - `CompiledPerSelector::SharedTrashCount { bucket: Option<u32> }`
   - `CompiledPerSelector::CardCountInZoneFiltered { zone, of, filter }`
   - `CompiledFormula::BindingDp(String)`
3. In `compile.rs`, compile all three forms and recursively compile filtered-count predicates.
4. In `formula_eval.rs`, add binding-aware functions:
   - `evaluate_with_bindings`
   - `evaluate_read_with_bindings`
   that preserve existing `evaluate` / `evaluate_read` behavior by calling the binding-aware path with `None`.
5. In `predicate.rs`, pass current `Bindings` into `eval_dp_constraint` and formula evaluation so `dp_lte: { formula: { binding_dp: ally } }` works inside selection filters.
6. For filtered zone counts, count only battle-area permanents or zone cards matching the supplied predicate. For zones that cannot be represented as a `PredicateSubject`, return false for subject-only predicates rather than counting everything.
```

- [ ] **Step 7: Run tests to verify they pass**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_formula_batch
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase3d_formula_zone_count
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_predicate_batch
```

Expected: PASS.

- [ ] **Step 8: Update tracker**

In `qa/dsl-vocab-gaps.md`, update:

```text
- EX4-011 shared-trash formula: resolved if shared trash + bucket evaluates in dp_lte filters.
- ST22-08 Binding DP formula: resolved if formulas can read a named permanent binding's effective DP.
- G-FORMULA-KIND-FILTER: resolved if `card_count_in_zone` can apply `filter: { kind: digimon }` for battle-area counts.
```

Do not close card YAML blockers that also need Option, Link, lowest-DP, or effect-initiated digivolve behavior.

- [ ] **Step 9: Commit**

```bash
git add code/digimon-dsl/src/formula.rs code/digimon-dsl/src/predicate.rs code/digimon-dsl/src/compiled.rs code/digimon-dsl/src/compile.rs code/digimon-engine/src/dsl_cards/formula_eval.rs code/digimon-engine/src/dsl_cards/predicate.rs code/digimon-engine/tests/dsl/main.rs code/digimon-engine/tests/dsl/group7_formula_batch.rs qa/dsl-vocab-gaps.md
git commit -m "feat: add group 7 formula vocabulary"
```

## Task 4: Aggregate Permanent Predicates for Lowest Level and Same-Level Pairs

**Files:**
- Modify: `code/digimon-dsl/src/predicate.rs`
- Modify: `code/digimon-dsl/src/formula.rs`
- Modify: `code/digimon-dsl/src/compiled.rs`
- Modify: `code/digimon-dsl/src/compile.rs`
- Modify: `code/digimon-engine/src/dsl_cards/predicate.rs`
- Modify: `code/digimon-engine/src/dsl_cards/formula_eval.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/permanent_scan.rs`
- Create or modify: `code/digimon-engine/tests/dsl/group7_formula_batch.rs`
- Docs: `qa/dsl-vocab-gaps.md`

- [ ] **Step 1: Write failing tests for lowest-level permanent filter**

Add:

```rust
#[test]
fn level_is_lowest_among_opponent_digimon_filters_only_lowest_level_digimon() {
    use digimon_dsl::compiled::{CompiledAggregateSelector, CompiledCardKind, CompiledPlayerRef, CompiledPredicate};
    use digimon_engine::debug_runner::{make_test_card, DebugRunner};
    use digimon_engine::dsl_cards::predicate::{eval_predicate_with_bindings, PredicateSubject};
    use digimon_engine::effect_context::EffectReadContext;
    use digimon_engine::enums::CardKind;

    let mut low = make_test_card("LOW-LV", "Low Level");
    low.card_kind = CardKind::Digimon;
    low.level = Some(3);
    let mut high = make_test_card("HIGH-LV", "High Level");
    high.card_kind = CardKind::Digimon;
    high.level = Some(6);
    let mut tamer = make_test_card("TAMER", "Tamer");
    tamer.card_kind = CardKind::Tamer;
    tamer.level = None;
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(low)
        .add_card(high)
        .add_card(tamer)
        .hand(0, &["SRC"])
        .hand(1, &["LOW-LV", "HIGH-LV", "TAMER"])
        .build();
    let src = runner.game.players[0].hand[0].handle();
    let low_h = runner.place_on_field(1, "LOW-LV", None);
    let high_h = runner.place_on_field(1, "HIGH-LV", None);
    runner.place_on_field(1, "TAMER", None);
    let rctx = EffectReadContext::new(&runner.game, src, None, 0);
    let pred = CompiledPredicate {
        kind: Some(CompiledCardKind::Digimon),
        level_matches_aggregate: Some((CompiledAggregateSelector::LowestLevel, CompiledPlayerRef::Opponent)),
        ..Default::default()
    };

    assert!(eval_predicate_with_bindings(&pred, &rctx, PredicateSubject::Permanent(low_h), None));
    assert!(!eval_predicate_with_bindings(&pred, &rctx, PredicateSubject::Permanent(high_h), None));
}
```

- [ ] **Step 2: Write failing tests for same-level pair formula**

Add:

```rust
#[test]
fn same_level_pair_count_formula_reads_source_stack_levels() {
    use digimon_dsl::compiled::{CompiledFormula, CompiledPerSelector};
    use digimon_engine::card_source::CardSource;
    use digimon_engine::debug_runner::{make_test_card, DebugRunner};
    use digimon_engine::dsl_cards::formula_eval;
    use digimon_engine::effect_context::EffectContext;
    use digimon_engine::enums::CardKind;

    let mut lv4a = make_test_card("LV4-A", "Lv4 A");
    lv4a.card_kind = CardKind::Digimon;
    lv4a.level = Some(4);
    let mut lv4b = make_test_card("LV4-B", "Lv4 B");
    lv4b.card_kind = CardKind::Digimon;
    lv4b.level = Some(4);
    let mut lv5 = make_test_card("LV5", "Lv5");
    lv5.card_kind = CardKind::Digimon;
    lv5.level = Some(5);
    let mut runner = DebugRunner::builder()
        .add_card(lv4a)
        .add_card(lv4b)
        .add_card(lv5)
        .build();
    let target = runner.place_on_field(0, "LV5", None);
    for id in ["LV4-A", "LV4-B", "LV5"] {
        let data_idx = runner.game.card_data.iter().position(|c| c.card_id == id).unwrap();
        let next = runner.game.next_card_index();
        runner.game.players[0].battle_area[target.index as usize]
            .card_sources
            .push(CardSource::new(data_idx, 0, next));
    }
    let src_card = runner.game.players[0].battle_area[target.index as usize].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, Some(target), 0);
    let formula = CompiledFormula::BasePerDelta {
        base: 0,
        per: CompiledPerSelector::SameLevelPairsInSources,
        delta: 1,
    };
    assert_eq!(formula_eval::evaluate(&formula, &ctx, target), 1);
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- level_is_lowest_among_opponent_digimon_filters_only_lowest_level_digimon
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- same_level_pair_count_formula_reads_source_stack_levels
```

Expected: FAIL because the compiled fields and formula selector do not exist.

- [ ] **Step 4: Implement aggregate predicate and formula**

Make these changes:

```text
1. Add a DSL predicate leaf for aggregate level matching, using a concrete name such as `level_is: { aggregate: lowest, over: opponent_battle_area }` or `level_matches_aggregate`.
2. Compile it to a typed `CompiledPredicate` field using `CompiledAggregateSelector::LowestLevel` and a `CompiledPlayerRef`.
3. Evaluate it only for permanent subjects whose top card has a level. Tamers and Option permanents must not participate in the aggregate.
4. Add `SameLevelPairsInSources` to formula selectors.
5. Evaluate same-level pairs by counting source cards below the top card by level and summing `count / 2` for each level bucket.
6. If printed text requires including the top card for a later card, add a distinct selector name rather than changing this source-only selector.
```

- [ ] **Step 5: Run tests to verify they pass**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- level_is_lowest_among_opponent_digimon_filters_only_lowest_level_digimon
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- same_level_pair_count_formula_reads_source_stack_levels
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_formula_batch
```

Expected: PASS.

- [ ] **Step 6: Update tracker**

In `qa/dsl-vocab-gaps.md`, update:

```text
- BT24-080 lowest-level predicate: resolved for deleting/filtering all opponent Digimon whose level equals the opponent-side minimum.
- BT22-015 same-level pair formula: resolved for source-stack pair counting. Leave repeat target-selection blockers open unless repeat loops are also implemented and tested.
```

- [ ] **Step 7: Commit**

```bash
git add code/digimon-dsl/src/predicate.rs code/digimon-dsl/src/formula.rs code/digimon-dsl/src/compiled.rs code/digimon-dsl/src/compile.rs code/digimon-engine/src/dsl_cards/predicate.rs code/digimon-engine/src/dsl_cards/formula_eval.rs code/digimon-engine/src/dsl_cards/step/permanent_scan.rs code/digimon-engine/tests/dsl/group7_formula_batch.rs qa/dsl-vocab-gaps.md
git commit -m "feat: add aggregate dsl predicates and formulas"
```

## Task 5: Event, Replacement, and Source-Subject Predicate Completeness

**Files:**
- Modify: `code/digimon-dsl/src/predicate.rs`
- Modify: `code/digimon-dsl/src/compiled.rs`
- Modify: `code/digimon-dsl/src/compile.rs`
- Modify: `code/digimon-engine/src/dsl_cards/predicate.rs`
- Modify: `code/digimon-engine/src/dsl_cards/lower_triggered.rs`
- Modify: `code/digimon-engine/src/dsl_cards/lower_replacement.rs`
- Modify: `code/digimon-engine/tests/dsl/phase3d_event_context.rs`
- Modify: `code/digimon-engine/tests/dsl/group7_predicate_batch.rs`
- Docs: `qa/dsl-vocab-gaps.md`

- [ ] **Step 1: Write failing tests for event owner and source-card predicates**

Add to `code/digimon-engine/tests/dsl/phase3d_event_context.rs`:

```rust
#[test]
fn event_target_owner_predicate_matches_controller() {
    let yaml = r#"
card: T-G7-EVENT-OWNER
name: Event Owner
kind: tamer
color: [white]
cost: 3
effects:
  - when: on_enter_field_anyone
    condition:
      all_of:
        - event_target_owner: opponent
        - event_target_kind: digimon
    process:
      - gain_memory: 1
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse yaml");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile yaml");
    assert_eq!(compiled.effects.len(), 1);
}

#[test]
fn host_and_trashed_source_predicates_compile_for_source_trash_events() {
    let yaml = r#"
card: T-G7-SOURCE-EVENT
name: Source Event
kind: tamer
color: [white]
cost: 3
effects:
  - when: on_digivolution_card_trashed
    condition:
      all_of:
        - host_permanent_trait_has: Mineral
        - trashed_source_trait_has: Rock
    process:
      - gain_memory: 1
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse yaml");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile yaml");
    assert_eq!(compiled.effects.len(), 1);
}
```

- [ ] **Step 2: Write failing test for self-digivolution-stack name predicate**

Add to `group7_predicate_batch.rs`:

```rust
#[test]
fn self_digivolution_contains_name_uses_source_permanent_subject() {
    let yaml = r#"
card: T-G7-STACK-NAME
name: Stack Name
kind: digimon
level: 7
color: [white]
cost: 14
dp: 15000
effects:
  - when: when_digivolving
    condition:
      any_of:
        - self_digivolution_contains_name: Omnimon
        - self_digivolution_contains_name: X Antibody
    process:
      - gain_memory: 1
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse yaml");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile yaml");
    assert_eq!(compiled.effects.len(), 1);
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- event_target_owner_predicate_matches_controller
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- host_and_trashed_source_predicates_compile_for_source_trash_events
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- self_digivolution_contains_name_uses_source_permanent_subject
```

Expected: FAIL because one or more predicate leaves are absent or `lower_triggered.rs` still evaluates conditions with `PredicateSubject::None`.

- [ ] **Step 4: Implement predicate leaves and subject threading**

Make these changes:

```text
1. Add predicate leaves:
   - `event_target_owner: PlayerRef`
   - `host_permanent_trait_has: String`
   - `trashed_source_trait_has: String`
   - `trashed_source_card_id_is: String`
   - `self_digivolution_contains_name: String`
2. Compile each leaf into `CompiledPredicate`.
3. In `eval_event_fields`, resolve event target owner from event permanent, target permanent, event card owner, or source player in that priority order.
4. Resolve host permanent and trashed source predicates from `current_trigger_context.event_host_permanent`, `event_host_card`, `event_source_card`, or equivalent current fields. If the engine does not yet populate one of these fields for a producer path, return false and leave that producer path tracked as an engine dependency.
5. In `lower_triggered.rs`, when evaluating `active_when` and `condition`, pass `PredicateSubject::Permanent(source_h)` when the effect is tied to a live source permanent. Use `PredicateSubject::BreedingPermanent(player)` for breeding source handles when applicable.
6. Implement `self_digivolution_contains_name` by scanning the source permanent's full stack, not by matching the top card's name.
7. In `lower_replacement.rs`, preserve the existing replacement cause/controller predicates and ensure `active_when` is evaluated before offering replacement choices.
```

- [ ] **Step 5: Add runtime tests for at least one predicate path**

Extend `code/digimon-engine/tests/dsl/phase3d_event_context.rs` with a concrete runtime test beside the existing `event_card_trait_predicate_matches_trashed_digivolution_card` coverage. Use the same `DebugRunner::builder().from_dsl_yaml(...)`, `place_stack_on_field(...)`, `enqueue_triggered(...)`, and `drain_effect_queue()` style already used in that file:

```rust
#[test]
fn source_trash_event_host_and_trashed_source_predicates_evaluate() {
    let yaml = r#"
card: DSL-SOURCE-TRASH-HOST
name: Source Trash Host
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_digivolution_card_trashed
    condition:
      all:
        - host_permanent_trait_has: Mineral
        - trashed_source_trait_has: Rock
    process:
      - gain_memory: 1
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(digimon_card("HOST", "Mineral Host", &["Mineral"], 5000))
        .add_card(digimon_card("SRC", "Rock Source", &["Rock"], 1000))
        .build();
    let host = runner.place_stack_on_field(0, &["SRC", "HOST"]);
    let source = runner.card_handle("SRC");

    runner.game.enqueue_triggered(
        EffectTiming::OnDigivolutionCardTrashed,
        TriggerSource::DigivolutionCardTrashed {
            player: host.player,
            host,
            trashed_card: source,
        },
    );
    runner.game.drain_effect_queue();

    assert_eq!(runner.memory(), 1);
}
```

If `TriggerSource::DigivolutionCardTrashed` uses different field names when this task is executed, adjust the struct literal to the current enum definition in `code/digimon-engine/src/selection.rs` and keep the assertions unchanged.

- [ ] **Step 6: Run tests to verify they pass**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase3d_event_context
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_predicate_batch
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- context_predicates
```

Expected: PASS.

- [ ] **Step 7: Update tracker**

In `qa/dsl-vocab-gaps.md`, update:

```text
- G-ROCKS-EVENT-CARD-PREDICATES: close only the predicate leaves whose event payload is tested. Keep untested producer paths open.
- G-SELF-DIGIVOLUTION-CONTAINS-NAME: close when source-subject threading and stack-name evaluation pass.
- Replacement cause predicate entry: close only if `active_when` is applied before replacement choices and covered by replacement tests.
```

- [ ] **Step 8: Commit**

```bash
git add code/digimon-dsl/src/predicate.rs code/digimon-dsl/src/compiled.rs code/digimon-dsl/src/compile.rs code/digimon-engine/src/dsl_cards/predicate.rs code/digimon-engine/src/dsl_cards/lower_triggered.rs code/digimon-engine/src/dsl_cards/lower_replacement.rs code/digimon-engine/tests/dsl/phase3d_event_context.rs code/digimon-engine/tests/dsl/group7_predicate_batch.rs qa/dsl-vocab-gaps.md
git commit -m "feat: add event subject dsl predicates"
```

## Task 6: Protect Main-Landed Dynamic Aura Formula Coverage

**Files:**
- Verify: `code/digimon-dsl/src/clause.rs`
- Verify: `code/digimon-dsl/src/compiled.rs`
- Verify: `code/digimon-engine/src/dsl_cards/lower_aura.rs`
- Verify: `code/digimon-engine/src/effect.rs`
- Verify: `code/digimon-engine/src/game.rs`
- Verify: `code/digimon-engine/tests/dsl/group6_dynamic_formulas.rs`
- Docs: `qa/dsl-vocab-gaps.md`
- Docs: `docs/RUST_ENGINE_GAPS.md`

- [ ] **Step 1: Confirm the schema and runtime fields already exist**

Run:

```bash
git grep -n "dp_modifier_fn\|security_attack_fn" -- code/digimon-dsl/src/clause.rs code/digimon-dsl/src/compiled.rs code/digimon-engine/src/dsl_cards/lower_aura.rs code/digimon-engine/src/effect.rs code/digimon-engine/src/game.rs
```

Expected: output shows `AuraBody.dp_modifier_fn`, `AuraBody.security_attack_fn`, compiled aura fields, `lower_aura` formula lowering, and runtime `Effect` / `Game` query usage.

- [ ] **Step 2: Run the current mainline aura regression tests**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- group6_dynamic_formulas phase2f2_formula_eval phase2f2_modifier_formula --nocapture
```

Expected: PASS. This is not a failing-test step because current `main` already implemented the behavior. A failure means a regression must be fixed before continuing, not that Group 7 should add a second aura implementation.

- [ ] **Step 3: Keep tracker wording aligned with Group 6 ownership**

Edit `qa/dsl-vocab-gaps.md` and `docs/RUST_ENGINE_GAPS.md` only if they drift from this baseline:

```text
G-AURA-DP-FORMULA is closed by Group 6 for `dp_modifier_fn`.
Security Attack formula auras are closed by Group 6 for `security_attack_fn`.
Residual formula selectors that `dp_modifier_fn` may consume remain tracked under their own Group 7 formula entries.
```

- [ ] **Step 4: Verify docs only if edited**

Run:

```bash
git diff --check -- qa/dsl-vocab-gaps.md docs/RUST_ENGINE_GAPS.md
```

Expected: no output.

- [ ] **Step 5: Commit only if tracker wording changed**

```bash
git add qa/dsl-vocab-gaps.md docs/RUST_ENGINE_GAPS.md
git commit -m "docs: preserve dynamic aura formula baseline"
```

## Task 7: Inherited Alt-Path Registration Lowering

**Files:**
- Modify: `code/digimon-engine/src/dsl_cards/mod.rs`
- Create or modify: `code/digimon-engine/src/dsl_cards/lower_alt_path_registration.rs`
- Modify: `code/digimon-engine/src/effect.rs` if a runtime registration payload must live on `Effect`.
- Modify: `code/digimon-engine/src/action/mask.rs` only if the registration changes existing DNA action-mask legality.
- Modify: `code/digimon-engine/src/game_actions.rs` only if executing the registered DNA path needs a new resume branch.
- Create: `code/digimon-engine/tests/dsl/group7_alt_path_registration.rs`
- Modify: `code/digimon-engine/tests/dsl/main.rs`
- Test: `code/digimon-engine/tests/dna_digivolve_user_action.rs`
- Docs: `qa/dsl-vocab-gaps.md`
- Docs: `qa/archetype-qa/dsl/bg-imperial.md`

- [ ] **Step 1: Run the already-closed top-level DNA authoring baselines**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- dsl_dna_alt_path_enriches_card_data_dna_costs
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dna_digivolve_user_action -- authored_dna_alt_path_makes_dna_action_legal_for_bt20_016
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt20_016_has_dna_digivolve_alt_path
```

Expected: PASS. These tests prove top-level `alt_paths: kind: dna_digivolve` already populates `CardData.dna_costs`; do not add a second `dna_costs:` top-level schema.

- [ ] **Step 2: Write the failing parse/compile guard for inherited registration**

Create `code/digimon-engine/tests/dsl/group7_alt_path_registration.rs`:

```rust
use digimon_dsl::{compile, CardSpec};

fn compile_yaml(yaml: &str) -> digimon_dsl::compiled::CompiledCard {
    let spec: CardSpec = serde_yml::from_str(yaml).expect("parse card yaml");
    compile::compile(&spec).expect("compile card yaml")
}

#[test]
fn inherited_end_of_turn_alt_path_registration_compiles() {
    let yaml = r#"
card: T-G7-ALT
name: Alt Registration
kind: digimon
level: 3
color: [blue]
cost: 3
dp: 1000
effects:
  - kind: alt_path_registration
    scope: inherited
    trigger: end_of_your_turn
    applies_to: { kind: digimon }
    registers:
      kind: dna_digivolve
      materials:
        - filter: { color_is: blue, level_eq: 4 }
        - filter: { color_is: green, level_eq: 4 }
      cost: 0
"#;
    let compiled = compile_yaml(yaml);
    let digimon_dsl::compiled::CompiledClause::Declarative(
        digimon_dsl::compiled::CompiledDeclarativeClause::AltPathRegistration {
            trigger,
            registers,
            ..
        },
    ) = &compiled.effects[0]
    else {
        panic!("expected alt_path_registration clause");
    };
    assert_eq!(*trigger, digimon_dsl::compiled::CompiledTiming::EndOfYourTurn);
    assert_eq!(registers.materials.len(), 2);
}
```

Register it in `code/digimon-engine/tests/dsl/main.rs`:

```rust
mod group7_alt_path_registration;
```

- [ ] **Step 3: Run the parse/compile guard**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- inherited_end_of_turn_alt_path_registration_compiles
```

Expected: PASS. The DSL syntax and compiled IR already exist; the open gap is engine lowering/runtime behavior.

- [ ] **Step 4: Write the failing lowering test**

Add to `code/digimon-engine/tests/dsl/group7_alt_path_registration.rs`:

```rust
#[test]
fn inherited_alt_path_registration_lowers_to_runtime_effect() {
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::dsl_cards::DslCardEffect;
    use digimon_engine::effect::CardEffect;
    use digimon_engine::enums::EffectTiming;
    use std::sync::Arc;

    let compiled = compile_yaml(r#"
card: T-G7-ALT
name: Alt Registration
kind: digimon
level: 3
color: [blue]
cost: 3
dp: 1000
effects:
  - kind: alt_path_registration
    scope: inherited
    trigger: end_of_your_turn
    applies_to: { kind: digimon }
    registers:
      kind: dna_digivolve
      materials:
        - filter: { color_is: blue, level_eq: 4 }
        - filter: { color_is: green, level_eq: 4 }
      cost: 0
"#);
    let dsl = DslCardEffect::new(Arc::new(compiled));
    let effects = dsl.effects(CardHandle(0));
    assert!(
        effects.iter().any(|effect| {
            effect.timing == EffectTiming::EndOfYourTurn
                && effect.alt_path_registration.is_some()
                && effect.inherited
        }),
        "inherited alt_path_registration must lower into an end-of-turn runtime effect"
    );
}
```

- [ ] **Step 5: Run the lowering test to verify it fails**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- inherited_alt_path_registration_lowers_to_runtime_effect
```

Expected: FAIL to compile or fail the assertion because `Effect` has no `alt_path_registration` payload and `DslCardEffect::effects()` currently drops `CompiledDeclarativeClause::AltPathRegistration`.

- [ ] **Step 6: Implement inherited registration without new action IDs**

Make these changes:

```text
1. Add a small runtime payload, for example `AltPathRegistrationEffect { applies_to: Option<CompiledPredicate>, registers: CompiledAltPath }`, in `code/digimon-engine/src/effect.rs`.
2. Add `alt_path_registration: Option<AltPathRegistrationEffect>` to `Effect`.
3. Add `lower_alt_path_registration.rs` that maps `CompiledDeclarativeClause::AltPathRegistration` to an `Effect` at the compiled trigger timing, preserving `scope`, `active_when`, `applies_to`, and `registers`.
4. Call the lowerer from `DslCardEffect::effects()` instead of allowing the declarative clause to fall through.
5. Reuse existing DNA action IDs (`DNA_DIGIVOLVE_START..DNA_DIGIVOLVE_END`) when a registered path makes a hand card legal. Do not add hidden automatic DNA digivolve resolution.
6. If no engine surface can consult the registered path during action-mask construction, add the narrow query hook there and cover both mask legality and execution validation in `dna_digivolve_user_action.rs`.
```

- [ ] **Step 7: Run tests to verify they pass**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_alt_path_registration
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dna_digivolve_user_action
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor -- dna
```

Expected: PASS. `mask_and_tensor` is the current tensor/mask target name on `main`.

- [ ] **Step 8: Contract review**

Review these files and confirm no constant changes were required:

```text
docs/ACTION_SPEC.md
docs/TENSOR_SPEC.md
code/digimon-engine-py/src/lib.rs
code/digimon_gym/digimon_gym.py
```

Expected: no edits unless implementation changed action-space or tensor semantics. If edits are required, add targeted tests proving masks and PyO3 constants remain synchronized.

- [ ] **Step 9: Update tracker**

In `qa/dsl-vocab-gaps.md` and `qa/archetype-qa/dsl/bg-imperial.md`, keep top-level DNA alt-path authoring marked resolved, and mark inherited/end-of-turn `alt_path_registration` resolved only when the registered path can make an existing DNA action legal and executing that action uses the registered materials/cost.

- [ ] **Step 10: Commit**

```bash
git add code/digimon-engine/src/effect.rs code/digimon-engine/src/dsl_cards/mod.rs code/digimon-engine/src/dsl_cards/lower_alt_path_registration.rs code/digimon-engine/src/action/mask.rs code/digimon-engine/src/game_actions.rs code/digimon-engine/tests/dsl/main.rs code/digimon-engine/tests/dsl/group7_alt_path_registration.rs code/digimon-engine/tests/dna_digivolve_user_action.rs qa/dsl-vocab-gaps.md qa/archetype-qa/dsl/bg-imperial.md
git commit -m "feat: lower inherited dsl alt path registration"
```

## Task 8: Raw-Rust Retirement Guard and Representative YAML Migration

**Files:**
- Modify: `code/digimon-engine/tests/dsl/phase4_retirement_guard.rs`
- Modify representative YAML files that only depended on completed Group 7 blockers.
- Modify: `qa/dsl-vocab-gaps.md`
- Modify relevant `qa/archetype-qa/dsl/*.md`

- [ ] **Step 1: Identify raw-rust placeholders made obsolete by Tasks 2-5 and 7**

Run:

```bash
Select-String -Path 'code/digimon-engine/cards/**/*.yaml' -Pattern 'raw_rust|process: \[\]' -Context 2,4
```

Expected: output lists any remaining raw-Rust or empty-process placeholders. For this task, only edit cards whose remaining blockers are fully covered by Group 7 tasks in this plan plus the already-landed Group 5/6/8 baselines.

- [ ] **Step 2: Add or update a retirement guard test**

In `code/digimon-engine/tests/dsl/phase4_retirement_guard.rs`, add assertions for migrated card IDs:

```rust
#[test]
fn group7_migrated_cards_do_not_use_raw_rust() {
    let card_ids = ["P-206", "BT8-097"];
    for card_id in card_ids {
        let yaml = digimon_engine::dsl_registry::embedded_yaml_for_card(card_id)
            .unwrap_or_else(|| panic!("missing embedded yaml for {card_id}"));
        assert!(
            !yaml.contains("raw_rust"),
            "{card_id} should not need raw_rust after Group 7 vocabulary landed"
        );
    }
}
```

Use the actual embedded-YAML helper name available in the repository. If no helper exists, load the YAML file from the known `code/digimon-engine/cards/<set>/<card>.yaml` path inside the test.

- [ ] **Step 3: Run the guard to verify it fails before migration**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_migrated_cards_do_not_use_raw_rust
```

Expected: FAIL for cards still using `raw_rust` or empty placeholder processes.

- [ ] **Step 4: Migrate only fully unblocked YAML**

Examples of acceptable migrations after prior tasks pass:

```yaml
filter:
  all_of:
    - kind: tamer
    - color_matches_any_field_digimon: { of: you }
```

```yaml
amount_fn:
  base: 0
  per:
    card_count_in_zone:
      zone: battle_area
      of: opponent
      filter: { kind: digimon }
  delta: 1
```

Do not migrate card clauses that still require effect-initiated digivolve from non-hand zones, repeat target selection, inherited alt-path registration before Task 7 lands, or player-visible choices not covered by the current engine. Delay/Link/Option primitives from Group 5 and dynamic aura fields from Group 6 are no longer generic blockers on current `main`; if a stale card note still names them, retag it to the actual remaining card-level gap before migrating YAML.

- [ ] **Step 5: Run card and DSL tests**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_migrated_cards_do_not_use_raw_rust
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- p_206
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt8_097
```

Expected: DSL tests pass. Card behavioral tests pass only for cards whose remaining blockers were fully removed; if a card still has a non-Group-7 blocker, keep that card out of the migration list and document why.

- [ ] **Step 6: Update trackers and archetype docs**

Update:

```text
qa/dsl-vocab-gaps.md
qa/archetype-qa/dsl/*.md for migrated archetypes
qa/archetype-qa/engine-gaps.md only if an engine blocker was narrowed by tested behavior
```

Each edit must include the exact passing command that proves the blocker moved.

- [ ] **Step 7: Commit**

```bash
git add code/digimon-engine/tests/dsl/phase4_retirement_guard.rs code/digimon-engine/cards qa/dsl-vocab-gaps.md qa/archetype-qa/dsl qa/archetype-qa/engine-gaps.md
git commit -m "feat: retire group 7 raw rust placeholders"
```

## Final Verification

- [ ] **Step 1: Run targeted Group 7 DSL suite**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase3d_formula_zone_count
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase3d_event_context
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase4_retirement_guard
```

Expected: PASS.

- [ ] **Step 2: Run broader DSL tests**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl
```

Expected: PASS.

- [ ] **Step 3: Run mask/action contract checks when applicable**

Run if any slice changes player-visible choices, DNA masks, or action decoding:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dna_digivolve_user_action
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor
```

Expected: PASS. If `mask_and_tensor` is not a valid target, list tests and run the repository's current mask/tensor target:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml -- --list
```

- [ ] **Step 4: Search for forbidden placeholder language in changed YAML and trackers**

Run:

```bash
$patterns = @(
  'raw_rust',
  'process: \[\]',
  [string]::new([char[]](84,66,68)),
  [string]::new([char[]](84,79,68,79)),
  ('implement' + ' later'),
  ('fill ' + 'in ' + 'details')
)
Select-String -Path 'code/digimon-engine/cards/**/*.yaml','qa/dsl-vocab-gaps.md','docs/RUST_ENGINE_GAPS.md','qa/archetype-qa/engine-gaps.md' -Pattern $patterns
```

Expected: no output for newly migrated card files. Existing unrelated tracker entries may still mention `raw_rust`; any such output must be reviewed and confirmed unrelated to completed Group 7 slices.

- [ ] **Step 5: Check markdown and whitespace**

Run:

```bash
git diff --check
```

Expected: no output.

## Self-Review Checklist

- [ ] Every new predicate names its subject explicitly: candidate card, permanent, event payload, source permanent, replacement context, or binding.
- [ ] Formula evaluation accepts current bindings wherever formulas can appear inside selection predicates or modifier/filter scans.
- [ ] The compiler rejects unsupported shapes through validation or compile errors; it does not silently lower unknown fields into no-ops.
- [ ] No action-space or tensor contract changed unless the same commit updates specs, PyO3, RL, and mask tests.
- [ ] Every closed tracker entry includes a passing test command.
- [ ] No production card behavior was approximated through auto-selection, empty process bodies, UI-only handling, or new raw-Rust escape hatches.
