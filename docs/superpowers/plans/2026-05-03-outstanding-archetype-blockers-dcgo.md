# Outstanding Archetype Blockers DCGO Reference Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the remaining archetype blockers left after the 2026-05-03 gap-closure batch, using the pinned DCGO submodule as the behavior reference for BT17-097, BT17-009, BT24-031, BT24-040, and BT24-101.

**Architecture:** Keep this as Rust engine plus DSL work only. First fix reusable engine/DSL primitives that block faithful card authoring, then add production YAML and card-level behavioral tests. No action-space, tensor, PyO3, RL wrapper, or frontend contract changes are expected.

**Tech Stack:** Rust, `digimon-engine`, `digimon-dsl`, YAML card specs, Cargo integration tests, DCGO C# reference under `DCGO/Assets/Scripts/CardEffect`.

---

## Scope Check

This is one plan because every item is in the Rust card-effect/DSL lane and shares the same replacement, selection, security movement, and production YAML surfaces. If execution reveals an action/tensor contract change, stop and write a separate action/tensor plan before continuing.

Use these DCGO reference files while implementing:

- `DCGO/Assets/Scripts/CardEffect/BT17/Blue/BT17_097.cs`: Return to the Primogenitor. Key behavior: Delay replacement deletes the option as cost, then chooses an Imperialdramon hand card; the original deletion is prevented only after the digivolve succeeds.
- `DCGO/Assets/Scripts/CardEffect/BT17/Red/BT17_009.cs`: Flamemon. Key behavior: On Play multi-bucket reveal; inherited On Deletion optional free play of a Tamer with inherited effects from hand.
- `DCGO/Assets/Scripts/CardEffect/BT24/Yellow/BT24_031.cs`: Elecmon. Key behavior: On Play multi-bucket reveal; inherited When Attacking OPT optional top-security-to-hand, then Recovery +1 if security is 0.
- `DCGO/Assets/Scripts/CardEffect/BT24/Yellow/BT24_040.cs`: Venusmon. Key behavior: play-cost reduction at <=3 security, shared On Play/When Digivolving trash-all-sources and two-card freeze/WD suppression, OPT cross-permanent protection by placing another sourceless Digimon as bottom security.
- `DCGO/Assets/Scripts/CardEffect/BT24/Yellow/BT24_101.cs`: Jupitermon. Key behavior: shared On Play/When Digivolving trash own top security, -13000 DP, Recovery +2 at <=1 security; OnLoseSecurity OPT trash opponent top security; OPT TS/Tamer leave prevention by trashing own top security.

Known current failure cluster:

- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- replacement_integration::bt17_097 --nocapture` should currently expose the BT17-097 Delay prompt/continuation failures.
- Production YAML for `BT17-009`, `BT24-031`, `BT24-040`, and `BT24-101` is deferred until the primitives below are closed.

## File Structure

- Modify `code/digimon-engine/src/dsl_cards/lower_replacement.rs`: replacement active-when subject semantics and BT17-097 Delay hand-digivolve continuation.
- Modify `code/digimon-engine/src/effect_context/mod.rs`: reusable security movement helpers and replacement-cost helper methods.
- Modify `code/digimon-engine/src/enums.rs`: add a permanent-top card source ref only if the replacement-cost security placement task chooses that representation.
- Modify `code/digimon-engine/src/game_actions.rs`: support moving a sourceless permanent to security as a cost, preserving existing security observer behavior.
- Modify `code/digimon-dsl/src/step.rs`, `code/digimon-dsl/src/compile.rs`, and `code/digimon-dsl/src/compiled.rs`: add missing step verbs.
- Modify `code/digimon-engine/src/dsl_cards/step/zone_moves.rs` and `code/digimon-engine/src/dsl_cards/step/replacement_outcome.rs`: lower the new verbs into engine helpers.
- Add tests under `code/digimon-engine/tests/dsl/`: primitive-level regressions.
- Add tests under `code/digimon-engine/tests/effect_context/`: security helper regressions.
- Add card specs under `code/digimon-engine/cards/bt17/` and `code/digimon-engine/cards/bt24/`: production YAML.
- Add card tests under `code/digimon-engine/tests/cards_behavioral/bt17/` and `code/digimon-engine/tests/cards_behavioral/bt24/`: card-level behavior.
- Modify `code/digimon-engine/tests/cards_behavioral/bt17/mod.rs` and `code/digimon-engine/tests/cards_behavioral/bt24/mod.rs`: include new card modules.
- Modify `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md`, `qa/archetype-qa/dsl/red-hybrid-ancientgreymon-2026-05-03-dsl-engine-gaps.md`, `qa/archetype-qa/dsl/ts-olympos-2026-05-03-dsl-engine-gaps.md`, and `qa/archetype-qa/dsl/bg-imperial-cross-archetype-gaps-2026-05-03.md`: close or narrow resolved rows.
- Modify `qa/qa-reports/validated_cards_dsl.json`: mark only fully tested production cards as implemented or audited.

---

## Task 1: Fix BT17-097 Delay Replacement Subject Matching

**Files:**
- Modify: `code/digimon-engine/src/dsl_cards/lower_replacement.rs`
- Test: `code/digimon-engine/tests/dsl/replacement_context.rs`
- Test: `code/digimon-engine/tests/option_flow/replacement_integration.rs`

- [ ] **Step 1: Add a replacement predicate regression for subject trait matching**

Add this test to `code/digimon-engine/tests/dsl/replacement_context.rs`:

```rust
use digimon_engine::card_source::CardHandle;
use digimon_engine::cards::CardEffect;
use digimon_engine::debug_runner::{digimon_card, DebugRunner};
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::enums::CardColor;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use std::sync::Arc;

#[test]
fn replacement_active_when_trait_matches_replacement_subject_not_source() {
    let yaml = r#"
card: TEST-SUBJECT-TRAIT-PROTECTOR
name: Subject Trait Protector
kind: digimon
color: [yellow]
level: 6
cost: 11
dp: 11000
traits: [Protector]
effects:
  - kind: replacement
    timing: when_would_be_deleted
    active_when:
      all_of:
        - trait_has: Free
        - replacement_cause: opponent_effect
    process:
      - cancel_replacement: {}
"#;
    let spec: digimon_dsl::spec::CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    let compiled = digimon_dsl::compile::compile(&spec).expect("replacement compiles");

    let mut runner = DebugRunner::builder()
        .add_card(digimon_card("TEST-SUBJECT-TRAIT-PROTECTOR", CardColor::Yellow))
        .add_card(digimon_card("FREE-TARGET", CardColor::Blue).with_traits(["Free"]))
        .add_card(digimon_card("PLAIN-TARGET", CardColor::Blue))
        .memory(0)
        .start();
    runner.register_effect(
        "TEST-SUBJECT-TRAIT-PROTECTOR",
        Arc::new(DslCardEffect::new(Arc::new(compiled))),
    );

    runner.place_on_field(0, "TEST-SUBJECT-TRAIT-PROTECTOR", Some(0));
    let free = runner.place_on_field(0, "FREE-TARGET", Some(0));
    let plain = runner.place_on_field(0, "PLAIN-TARGET", Some(0));

    runner
        .game
        .delete_permanent_with_cause(free, ReplacementCause::OpponentEffect);
    assert!(
        find_permanent(&runner, 0, "FREE-TARGET").is_some(),
        "cross-permanent replacement must match the replacement subject's Free trait"
    );

    runner
        .game
        .delete_permanent_with_cause(plain, ReplacementCause::OpponentEffect);
    assert!(
        find_permanent(&runner, 0, "PLAIN-TARGET").is_none(),
        "replacement must not fire for an unqualified subject"
    );
}

fn find_permanent(runner: &DebugRunner, player: u8, card_id: &str) -> Option<PermanentHandle> {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .enumerate()
        .find(|(_, perm)| perm.top_card().card_id(&runner.game.card_data) == card_id)
        .map(|(index, _)| PermanentHandle {
            player,
            index: index as u8,
        })
}
```

If `digimon_card(...).with_traits(...)` does not exist in the test helper, copy the local helper shape used in `code/digimon-engine/tests/option_flow/replacement_integration.rs` instead of adding a new production API.

- [ ] **Step 2: Run the focused failing tests**

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- replacement_active_when_trait_matches_replacement_subject_not_source --nocapture
cargo test --manifest-path code\digimon-engine\Cargo.toml --test option_flow -- replacement_integration::bt17_097 --nocapture
```

Expected before the fix:

- The new DSL regression fails because the replacement condition still requires the replacement subject to be the source permanent unless `replacement_subject_is_mine` is present.
- The existing BT17-097 tests fail with no Delay hand-choice prompt or missing pending cost prompt.

- [ ] **Step 3: Expand replacement active-when subject detection**

In `code/digimon-engine/src/dsl_cards/lower_replacement.rs`, replace `predicate_requires_replacement_subject` with a broader subject-reader detector. The intent is: a replacement clause may match a non-source subject when `active_when` contains subject predicates such as `trait_has`, `kind`, `level_*`, `name_*`, `owner`, `materials_count_*`, or `replacement_subject_is_mine`; replacement-global predicates such as only `replacement_cause` do not by themselves authorize cross-permanent matching.

Use this implementation shape:

```rust
fn predicate_reads_replacement_subject(pred: &CompiledPredicate) -> bool {
    let reads_direct_subject =
        pred.kind.is_some()
            || pred.level_eq.is_some()
            || pred.level_lte.is_some()
            || pred.level_gte.is_some()
            || pred.level_matches_aggregate.is_some()
            || pred.color_is.is_some()
            || !pred.color_only.is_empty()
            || pred.color_matches_any_field_digimon.is_some()
            || pred.trait_has.is_some()
            || pred.form_is.is_some()
            || pred.attribute_is.is_some()
            || pred.name_is.is_some()
            || pred.name_contains.is_some()
            || pred.name_in.is_some()
            || pred.card_number_is.is_some()
            || pred.play_cost_lte.is_some()
            || pred.dp_eq.is_some()
            || pred.dp_lte.is_some()
            || pred.dp_gte.is_some()
            || pred.stack_size_lte.is_some()
            || pred.stack_size_gte.is_some()
            || pred.materials_count_lte.is_some()
            || pred.materials_count_gte.is_some()
            || pred.has_inherited.is_some()
            || pred.is_suspended.is_some()
            || pred.is_unsuspended.is_some()
            || pred.has_keyword.is_some()
            || !pred.zone.is_empty()
            || pred.owner.is_some()
            || pred.other.is_some()
            || pred.of_permanent.is_some()
            || pred.not_in_binding.is_some()
            || pred.replacement_subject_is_mine.is_some();

    reads_direct_subject
        || pred.all_of.iter().any(predicate_reads_replacement_subject)
        || pred.any_of.iter().any(predicate_reads_replacement_subject)
        || pred.none_of.iter().any(predicate_reads_replacement_subject)
        || pred
            .not
            .as_deref()
            .is_some_and(predicate_reads_replacement_subject)
}
```

Then update the caller:

```rust
let can_match_cross_permanent_subject = active_when
    .as_ref()
    .is_some_and(predicate_reads_replacement_subject);
```

Do not include event-only predicates (`event_*`, `host_permanent_*`, `trashed_source_*`) in this detector; those read trigger payloads, not the replacement subject.

- [ ] **Step 4: Run focused verification**

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- replacement_active_when_trait_matches_replacement_subject_not_source replacement_subject_and_source_predicates_compile_together --nocapture
cargo test --manifest-path code\digimon-engine\Cargo.toml --test option_flow -- replacement_integration::bt17_097 --nocapture
```

Expected after the fix:

- The new subject-trait regression passes.
- The five known BT17-097 Delay replacement tests pass or reduce to a narrower continuation bug. If any BT17-097 test still fails, do not move on; inspect `continue_delay_cost_after_selection` and `install_delay_hand_digivolve_after_paid` before changing tests.

- [ ] **Step 5: Commit**

```powershell
git add code\digimon-engine\src\dsl_cards\lower_replacement.rs code\digimon-engine\tests\dsl\replacement_context.rs
git commit -m "fix: match replacement active_when against subjects"
```

---

## Task 2: Add Top-Security-To-Hand and Recovery DSL Steps

**Files:**
- Modify: `code/digimon-dsl/src/step.rs`
- Modify: `code/digimon-dsl/src/compiled.rs`
- Modify: `code/digimon-dsl/src/compile.rs`
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/zone_moves.rs`
- Modify: `code/digimon-engine/tests/dsl/main.rs`
- Create: `code/digimon-engine/tests/dsl/security_stack_steps.rs`
- Test: `code/digimon-engine/tests/effect_context/security_stack_operations.rs`

- [ ] **Step 1: Add failing effect-context tests**

Append these tests to `code/digimon-engine/tests/effect_context/security_stack_operations.rs`:

```rust
#[test]
fn add_top_security_to_hand_moves_top_card_and_fires_loss_observer() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("OBS", "Observer"))
        .add_card(make_test_card("BOTTOM", "Bottom"))
        .add_card(make_test_card("TOP", "Top"))
        .security(0, &["BOTTOM", "TOP"])
        .memory(0)
        .start();
    runner.register_effect("OBS", Arc::new(OppSecurityRemovedGainOne));
    runner.place_on_field(1, "OBS", Some(0));

    let source_card = runner.game.players[1].battle_area[0].top_card().handle();
    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 0);
        assert!(ctx.add_top_security_to_hand(0));
    }

    assert_eq!(runner.security_count(0), 1);
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "TOP"),
        "top security card moved to hand"
    );
    assert_eq!(
        runner.memory(),
        1,
        "security removal observers fire for security-to-hand"
    );
}

#[test]
fn recover_from_deck_places_deck_top_on_security_without_loss_observer() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("OBS", "Observer"))
        .add_card(make_test_card("RECOVER-A", "Recover A"))
        .add_card(make_test_card("RECOVER-B", "Recover B"))
        .deck(0, &["RECOVER-A", "RECOVER-B"])
        .memory(0)
        .start();
    runner.register_effect("OBS", Arc::new(OppSecurityRemovedGainOne));
    runner.place_on_field(1, "OBS", Some(0));

    let source_card = runner.game.players[1].battle_area[0].top_card().handle();
    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 0);
        assert_eq!(ctx.recover_from_deck(0, 1), 1);
    }

    assert_eq!(runner.security_count(0), 1);
    assert_eq!(runner.memory(), 0, "Recovery does not fire security-loss observers");
    assert_eq!(
        runner.game.players[0].security.last().unwrap().card_id(&runner.game.card_data),
        "RECOVER-B",
        "deck top becomes top security"
    );
}
```

- [ ] **Step 2: Run effect-context tests to verify they fail**

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test effect_context -- add_top_security_to_hand_moves_top_card_and_fires_loss_observer recover_from_deck_places_deck_top_on_security_without_loss_observer --nocapture
```

Expected before implementation: compile fails because `add_top_security_to_hand` and `recover_from_deck` do not exist.

- [ ] **Step 3: Implement EffectContext helpers**

Add these methods near the existing security helpers in `code/digimon-engine/src/effect_context/mod.rs`:

```rust
pub fn add_top_security_to_hand(&mut self, player: PlayerId) -> bool {
    let Some(card) = self
        .game
        .player(player)
        .security
        .last()
        .map(|card| card.handle())
    else {
        return false;
    };
    self.add_to_hand_from_security(player, card)
}

pub fn recover_from_deck(&mut self, player: PlayerId, count: u8) -> u8 {
    let mut recovered = 0;
    for _ in 0..count {
        if self.place_on_security(
            player,
            crate::enums::CardSourceRef::DeckTop(player),
            crate::enums::StackPosition::Top,
            false,
        ) {
            recovered += 1;
        } else {
            break;
        }
    }
    recovered
}
```

This intentionally uses `place_on_security`, not a raw push, so Recovery respects `CannotAddSecurityByEffect` and any `WhenWouldPlaceInSecurity` replacement rules.

- [ ] **Step 4: Add DSL verb parsing and compilation**

In `code/digimon-dsl/src/step.rs`, add enum variants:

```rust
AddTopSecurityToHand(PlayerArg),
Recover(DrawArgs),
```

Add serializer arms:

```rust
StepSpec::AddTopSecurityToHand(v) => kv!(s, "add_top_security_to_hand", v),
StepSpec::Recover(v) => kv!(s, "recover", v),
```

Add deserializer arms:

```rust
"add_top_security_to_hand" => StepSpec::AddTopSecurityToHand(map.next_value()?),
"recover" => StepSpec::Recover(map.next_value()?),
```

Add both verb names to the semantic validator's known step list in the same file.

In `code/digimon-dsl/src/compiled.rs`, add:

```rust
AddTopSecurityToHand {
    of: CompiledPlayerRef,
},
Recover {
    of: CompiledPlayerRef,
    count: u8,
},
```

In `code/digimon-dsl/src/compile.rs`, add match arms:

```rust
S::AddTopSecurityToHand(v) => CompiledStep::AddTopSecurityToHand {
    of: compile_player(v.of),
},
S::Recover(v) => CompiledStep::Recover {
    of: compile_player(v.of),
    count: v.count,
},
```

- [ ] **Step 5: Lower the new steps**

In `code/digimon-engine/src/dsl_cards/step/zone_moves.rs`, add arms:

```rust
CompiledStep::AddTopSecurityToHand { of } => {
    let p = resolve_player(ctx, *of);
    ctx.add_top_security_to_hand(p);
    true
}
CompiledStep::Recover { of, count } => {
    let p = resolve_player(ctx, *of);
    ctx.recover_from_deck(p, *count);
    true
}
```

- [ ] **Step 6: Add DSL integration tests**

Create `code/digimon-engine/tests/dsl/security_stack_steps.rs`:

```rust
use digimon_engine::action::space::SEL_REVEAL_START;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};

#[test]
fn dsl_add_top_security_to_hand_then_recover_models_bt24_031_inherited_shape() {
    let yaml = r#"
card: TEST-SECURITY-STEPS
name: Security Steps
kind: digimon
color: [yellow]
level: 3
cost: 3
dp: 1000
effects:
  - when: when_attacking
    optional: true
    process:
      - add_top_security_to_hand: { of: you }
      - if:
          condition: { security_count_lte: 0 }
          then:
            - recover: { of: you, count: 1 }
"#;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("test YAML loads")
        .add_card(make_test_card("SECURITY", "Security"))
        .add_card(make_test_card("RECOVER", "Recover"))
        .security(0, &["SECURITY"])
        .deck(0, &["RECOVER"])
        .memory(0)
        .start();
    let carrier = runner.place_on_field(0, "TEST-SECURITY-STEPS", Some(0));

    runner
        .game
        .fire_timing_for_permanent(digimon_engine::enums::EffectTiming::WhenAttacking, carrier);
    runner.execute_branch(0).expect("accept optional inherited-like effect");

    assert_eq!(runner.security_count(0), 1, "security removed then recovered");
    assert!(
        runner
            .game
            .players[0]
            .hand
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "SECURITY")
    );
    assert_eq!(
        runner.game.players[0].security.last().unwrap().card_id(&runner.game.card_data),
        "RECOVER"
    );
}
```

Add this line to `code/digimon-engine/tests/dsl/main.rs`:

```rust
mod security_stack_steps;
```

`DebugRunner::builder().from_dsl_yaml(...)` is available behind the test harness DSL loader feature and is the expected pattern for this integration test.

- [ ] **Step 7: Run focused verification**

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test effect_context -- add_top_security_to_hand_moves_top_card_and_fires_loss_observer recover_from_deck_places_deck_top_on_security_without_loss_observer --nocapture
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- security_stack_steps --nocapture
```

Expected: all tests pass.

- [ ] **Step 8: Commit**

```powershell
git add code\digimon-dsl\src\step.rs code\digimon-dsl\src\compiled.rs code\digimon-dsl\src\compile.rs code\digimon-engine\src\effect_context\mod.rs code\digimon-engine\src\dsl_cards\step\zone_moves.rs code\digimon-engine\tests\dsl\main.rs code\digimon-engine\tests\dsl\security_stack_steps.rs code\digimon-engine\tests\effect_context\security_stack_operations.rs
git commit -m "feat: add security hand and recovery DSL steps"
```

---

## Task 3: Author BT17-009 and BT24-031 Production YAML

**Files:**
- Create: `code/digimon-engine/cards/bt17/BT17-009.yaml`
- Create: `code/digimon-engine/cards/bt24/BT24-031.yaml`
- Create: `code/digimon-engine/tests/cards_behavioral/bt17/bt17_009.rs`
- Create: `code/digimon-engine/tests/cards_behavioral/bt24/bt24_031.rs`
- Modify: `code/digimon-engine/tests/cards_behavioral/bt17/mod.rs`
- Modify: `code/digimon-engine/tests/cards_behavioral/bt24/mod.rs`

- [ ] **Step 1: Write BT17-009 failing card tests**

Create `code/digimon-engine/tests/cards_behavioral/bt17/bt17_009.rs`:

```rust
//! BT17-009 Flamemon.
//! DCGO reference: DCGO/Assets/Scripts/CardEffect/BT17/Red/BT17_009.cs
//! Covers On Play multi-bucket reveal and inherited On Deletion free Tamer play.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;

#[test]
fn bt17_009_on_play_adds_hybrid_and_inherited_tamer_then_bottoms_rest() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-009")
        .expect("BT17-009 YAML loads")
        .add_card(make_trait_card("HYBRID", &["Hybrid"]))
        .add_card(make_inherited_tamer("TAMER-INHERITED"))
        .add_card(make_test_card("BLANK", "Blank"))
        .deck(0, &["BLANK", "TAMER-INHERITED", "HYBRID"])
        .hand(0, &["BT17-009"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Flamemon");
    pick_first_pending(&mut runner, "pick Hybrid bucket");
    pick_first_pending(&mut runner, "pick Tamer bucket");
    runner.auto_resolve().expect("bottom remainder");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand_ids.contains(&"HYBRID".to_string()));
    assert!(hand_ids.contains(&"TAMER-INHERITED".to_string()));
    assert_eq!(
        runner.game.players[0].deck.first().unwrap().card_id(&runner.game.card_data),
        "BLANK",
        "unchosen reveal card returned to deck bottom"
    );
}

#[test]
fn bt17_009_inherited_on_deletion_may_play_tamer_with_inherited_effect_from_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-009")
        .expect("BT17-009 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_inherited_tamer("TAMER-INHERITED"))
        .hand(0, &["TAMER-INHERITED"])
        .memory(0)
        .start();

    let carrier = runner.place_stack(0, &["BT17-009", "CARRIER"]);
    runner
        .game
        .delete_permanent_with_cause(carrier, digimon_engine::replacement::ReplacementCause::OpponentEffect);

    runner.execute_branch(0).expect("accept inherited On Deletion");
    pick_first_pending(&mut runner, "choose Tamer");

    assert!(
        runner
            .game
            .players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "TAMER-INHERITED"),
        "Tamer with inherited effect was played for free"
    );
}

fn make_trait_card(id: &str, traits: &[&str]) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn make_inherited_tamer(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = digimon_engine::enums::CardKind::Tamer;
    card.inherited_text = "[Your Turn] This Digimon gets +1000 DP.".to_string();
    card
}

fn zone_ids(cards: &[digimon_engine::card_source::CardSource], data: &[digimon_engine::card_data::CardData]) -> Vec<String> {
    cards.iter().map(|card| card.card_id(data).to_string()).collect()
}

fn pick_first_pending(runner: &mut DebugRunner, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect(label);
}
```

Adjust helper names only to match existing `DebugRunner` APIs; keep the assertions and behavior unchanged.

- [ ] **Step 2: Write BT24-031 failing card tests**

Create `code/digimon-engine/tests/cards_behavioral/bt24/bt24_031.rs`:

```rust
//! BT24-031 Elecmon.
//! DCGO reference: DCGO/Assets/Scripts/CardEffect/BT24/Yellow/BT24_031.cs
//! Covers On Play multi-bucket reveal and inherited When Attacking security-to-hand plus Recovery.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;

#[test]
fn bt24_031_on_play_adds_iliad_and_ts_without_double_picking_dual_match() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-031")
        .expect("BT24-031 YAML loads")
        .add_card(make_trait_card("ILIAD-ONLY", &["Iliad"]))
        .add_card(make_trait_card("TS-ONLY", &["TS"]))
        .add_card(make_trait_card("DUAL", &["Iliad", "TS"]))
        .deck(0, &["DUAL", "TS-ONLY", "ILIAD-ONLY"])
        .hand(0, &["BT24-031"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Elecmon");
    pick_revealed_by_id(&mut runner, "ILIAD-ONLY", "pick Iliad");
    pick_revealed_by_id(&mut runner, "TS-ONLY", "pick TS");
    runner.auto_resolve().expect("finish reveal");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert_eq!(hand_ids.iter().filter(|id| *id == "DUAL").count(), 0);
    assert!(hand_ids.contains(&"ILIAD-ONLY".to_string()));
    assert!(hand_ids.contains(&"TS-ONLY".to_string()));
}

#[test]
fn bt24_031_inherited_when_attacking_adds_top_security_then_recovers_at_zero() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-031")
        .expect("BT24-031 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("SECURITY", "Security"))
        .add_card(make_test_card("RECOVER", "Recover"))
        .security(0, &["SECURITY"])
        .deck(0, &["RECOVER"])
        .memory(0)
        .start();

    let carrier = runner.place_stack(0, &["BT24-031", "CARRIER"]);
    runner.game.fire_timing_for_permanent(EffectTiming::WhenAttacking, carrier);
    runner.execute_branch(0).expect("accept optional inherited effect");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand_ids.contains(&"SECURITY".to_string()));
    assert_eq!(runner.security_count(0), 1);
    assert_eq!(
        runner.game.players[0].security.last().unwrap().card_id(&runner.game.card_data),
        "RECOVER"
    );
}

fn make_trait_card(id: &str, traits: &[&str]) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn zone_ids(cards: &[digimon_engine::card_source::CardSource], data: &[digimon_engine::card_data::CardData]) -> Vec<String> {
    cards.iter().map(|card| card.card_id(data).to_string()).collect()
}

fn pick_revealed_by_id(runner: &mut DebugRunner, id: &str, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    let action = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|action| {
            let idx = (*action - SEL_REVEAL_START) as usize;
            runner.game.revealed_cards[idx].card_id(&runner.game.card_data) == id
        })
        .expect(label);
    runner
        .execute_action(0, action)
        .expect(label);
}
```

- [ ] **Step 3: Register card test modules and run failing tests**

Add to `code/digimon-engine/tests/cards_behavioral/bt17/mod.rs`:

```rust
mod bt17_009;
```

Add to `code/digimon-engine/tests/cards_behavioral/bt24/mod.rs`:

```rust
mod bt24_031;
```

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test cards_behavioral -- bt17_009 bt24_031 --nocapture
```

Expected before YAML exists: tests fail because the DSL cards are missing.

- [ ] **Step 4: Author BT17-009 YAML**

Create `code/digimon-engine/cards/bt17/BT17-009.yaml`:

```yaml
card: BT17-009
name: Flamemon
kind: digimon
level: 3
color: [red]
cost: 3
dp: 1000
traits: [Wizard, Hybrid]
alt_paths:
  - kind: digivolve
    from: { level_eq: 2 }
    cost: 0

effects:
  - when: on_play
    summary: "[On Play] Reveal 3; add one Hybrid/Ten Warriors and one inherited-effect Tamer"
    process:
      - reveal_top_deck: { of: you, count: 3, bind_as: revealed }
      - select_reveal_buckets:
          from: revealed
          buckets:
            - bind_as: hybrid_pick
              min: 0
              max: 1
              filter:
                any_of:
                  - trait_has: Hybrid
                  - trait_has: Ten Warriors
            - bind_as: tamer_pick
              min: 0
              max: 1
              filter:
                all_of:
                  - kind: tamer
                  - has_inherited: {}
          no_duplicate_cards: true
          prompt: "Select 1 [Hybrid]/[Ten Warriors] and 1 inherited-effect Tamer"
      - add_to_hand_from_reveal: { of: you, card: hybrid_pick }
      - add_to_hand_from_reveal: { of: you, card: tamer_pick }
      - place_remainder_on_deck: { of: you, position: bottom }

  - scope: inherited
    when: on_deletion
    optional: true
    summary: "[On Deletion] You may play 1 Tamer with inherited effects from hand free"
    process:
      - select_hand:
          of: you
          bind_as: tamer
          optional: true
          filter:
            all_of:
              - kind: tamer
              - has_inherited: {}
          prompt: "Play 1 Tamer card with inherited effects"
      - play_from_hand_free: { of: you, hand_index: tamer }
```

- [ ] **Step 5: Author BT24-031 YAML**

Create `code/digimon-engine/cards/bt24/BT24-031.yaml`:

```yaml
card: BT24-031
name: Elecmon
kind: digimon
level: 3
color: [yellow]
cost: 3
dp: 1000
traits: [Mammal, Iliad, TS]
alt_paths:
  - kind: digivolve
    from:
      all_of:
        - level_eq: 2
        - trait_has: TS
    cost: 0

effects:
  - when: on_play
    summary: "[On Play] Reveal 3; add one Iliad and one TS card"
    process:
      - reveal_top_deck: { of: you, count: 3, bind_as: revealed }
      - select_reveal_buckets:
          from: revealed
          buckets:
            - bind_as: iliad_pick
              min: 0
              max: 1
              filter: { trait_has: Iliad }
            - bind_as: ts_pick
              min: 0
              max: 1
              filter: { trait_has: TS }
          no_duplicate_cards: true
          prompt: "Select 1 [Iliad] and 1 [TS] trait card"
      - add_to_hand_from_reveal: { of: you, card: iliad_pick }
      - add_to_hand_from_reveal: { of: you, card: ts_pick }
      - place_remainder_on_deck: { of: you, position: bottom }

  - scope: inherited
    when: when_attacking
    once_per_turn: true
    optional: true
    summary: "[When Attacking] Add top security to hand; if you have 0 security, Recovery +1"
    process:
      - add_top_security_to_hand: { of: you }
      - if:
          condition: { security_count_lte: 0 }
          then:
            - recover: { of: you, count: 1 }
```

- [ ] **Step 6: Run card and primitive verification**

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test cards_behavioral -- bt17_009 bt24_031 --nocapture
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- reveal_buckets security_stack_steps hybrid_tamer_digivolve --nocapture
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```powershell
git add code\digimon-engine\cards\bt17\BT17-009.yaml code\digimon-engine\cards\bt24\BT24-031.yaml code\digimon-engine\tests\cards_behavioral\bt17\bt17_009.rs code\digimon-engine\tests\cards_behavioral\bt24\bt24_031.rs code\digimon-engine\tests\cards_behavioral\bt17\mod.rs code\digimon-engine\tests\cards_behavioral\bt24\mod.rs
git commit -m "feat: author Flamemon and Elecmon DSL cards"
```

---

## Task 4: Add Replacement-Cost Success Helpers for TS Protection

**Files:**
- Modify: `code/digimon-dsl/src/step.rs`
- Modify: `code/digimon-dsl/src/compiled.rs`
- Modify: `code/digimon-dsl/src/compile.rs`
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Modify: `code/digimon-engine/src/enums.rs`
- Modify: `code/digimon-engine/src/game_actions.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/replacement_outcome.rs`
- Test: `code/digimon-engine/tests/dsl/phase3b_replacement_process.rs`
- Test: `code/digimon-engine/tests/replacements/context_predicates.rs`

- [ ] **Step 1: Add failing DSL tests for cost-paid replacement cancellation**

Append to `code/digimon-engine/tests/dsl/phase3b_replacement_process.rs`:

```rust
#[test]
fn replacement_cancels_only_when_top_security_cost_is_paid() {
    let yaml = r#"
card: TEST-TRASH-SEC-PROTECT
name: Trash Security Protect
kind: digimon
color: [yellow]
level: 6
cost: 12
dp: 12000
traits: [TS]
effects:
  - kind: replacement
    timing: when_would_leave_battle_area
    active_when:
      all_of:
        - replacement_subject_is_mine: true
        - trait_has: TS
        - security_count_gte: 1
    process:
      - trash_top_security_and_cancel_replacement: { of: you }
"#;
    let compiled = compile_test_card(yaml);
    let mut runner = runner_with_dsl(compiled)
        .add_card(make_trait_card("TARGET", &["TS"]))
        .add_card(make_test_card("SEC", "Security"))
        .security(0, &["SEC"])
        .start();
    let protector = runner.place_on_field(0, "TEST-TRASH-SEC-PROTECT", Some(0));
    let target = runner.place_on_field(0, "TARGET", Some(0));

    runner
        .game
        .return_to_hand_with_cause(target, digimon_engine::replacement::ReplacementCause::OpponentEffect);

    assert!(permanent_exists(&runner, 0, "TARGET"));
    assert_eq!(runner.security_count(0), 0, "security cost was paid");
    assert!(permanent_exists(&runner, 0, "TEST-TRASH-SEC-PROTECT"));
}

#[test]
fn replacement_cancels_only_when_other_sourceless_digimon_is_placed_in_security() {
    let yaml = r#"
card: TEST-PLACE-SEC-PROTECT
name: Place Security Protect
kind: digimon
color: [yellow]
level: 6
cost: 12
dp: 12000
traits: [TS]
effects:
  - kind: replacement
    timing: when_would_leave_battle_area
    active_when:
      all_of:
        - replacement_subject_is_mine: true
        - trait_has: TS
        - replacement_cause: opponent_effect
    process:
      - select_own_permanent:
          bind_as: cost_body
          filter:
            all_of:
              - kind: digimon
              - materials_count_lte: 0
              - not_in_binding: replacement_subject
          prompt: "Place another sourceless Digimon as bottom security"
      - place_permanent_bottom_security_and_cancel_replacement:
          of: you
          target: cost_body
"#;
    let compiled = compile_test_card(yaml);
    let mut runner = runner_with_dsl(compiled)
        .add_card(make_trait_card("TARGET", &["TS"]))
        .add_card(make_test_card("COST-BODY", "Cost Body"))
        .start();
    runner.place_on_field(0, "TEST-PLACE-SEC-PROTECT", Some(0));
    let target = runner.place_on_field(0, "TARGET", Some(0));
    runner.place_on_field(0, "COST-BODY", Some(0));

    runner
        .game
        .return_to_hand_with_cause(target, digimon_engine::replacement::ReplacementCause::OpponentEffect);
    let view = runner
        .pending_selection_view()
        .expect("choose cost body selection");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose cost body");

    assert!(permanent_exists(&runner, 0, "TARGET"));
    assert!(!permanent_exists(&runner, 0, "COST-BODY"));
    assert_eq!(
        runner.game.players[0].security.first().unwrap().card_id(&runner.game.card_data),
        "COST-BODY",
        "cost body was placed as bottom security"
    );
}
```

Use local helper names from the existing file. If this file does not already expose `compile_test_card`, `runner_with_dsl`, or `permanent_exists`, add private helpers at the bottom of the test module rather than changing production APIs.

- [ ] **Step 2: Run failing tests**

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- replacement_cancels_only_when_top_security_cost_is_paid replacement_cancels_only_when_other_sourceless_digimon_is_placed_in_security --nocapture
```

Expected before implementation: compile fails because the two DSL verbs are unknown.

- [ ] **Step 3: Add compiled step variants and parser arms**

Add to `code/digimon-dsl/src/step.rs`:

```rust
TrashTopSecurityAndCancelReplacement(PlayerArg),
PlacePermanentBottomSecurityAndCancelReplacement(PlacePermanentSecurityReplacementArgs),
```

Define the args:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PlacePermanentSecurityReplacementArgs {
    #[serde(default)]
    pub of: PlayerRef,
    pub target: crate::common::BindingRef,
}
```

Add serializer/deserializer arms for:

```rust
"trash_top_security_and_cancel_replacement"
"place_permanent_bottom_security_and_cancel_replacement"
```

In `code/digimon-dsl/src/compiled.rs`, add:

```rust
TrashTopSecurityAndCancelReplacement {
    of: CompiledPlayerRef,
},
PlacePermanentBottomSecurityAndCancelReplacement {
    of: CompiledPlayerRef,
    target: CompiledBindingRef,
},
```

In `code/digimon-dsl/src/compile.rs`, compile both variants using the existing player and binding-ref helpers.

- [ ] **Step 4: Add engine helpers**

In `code/digimon-engine/src/effect_context/mod.rs`, add:

```rust
pub fn trash_top_security_and_cancel_current_replacement(&mut self, player: PlayerId) -> bool {
    if self.trash_top_security(player) {
        self.cancel_current_replacement();
        true
    } else {
        false
    }
}

pub fn place_sourceless_permanent_bottom_security_and_cancel_current_replacement(
    &mut self,
    player: PlayerId,
    target: PermanentHandle,
) -> bool {
    if self.game.place_sourceless_permanent_on_security_bottom(player, target, self.player) {
        self.cancel_current_replacement();
        true
    } else {
        false
    }
}
```

In `code/digimon-engine/src/game_actions.rs`, implement `place_sourceless_permanent_on_security_bottom`. The method must:

1. Reject invalid handles.
2. Reject non-sourceless permanents with `card_sources.len() != 1`.
3. Fire `WhenWouldPlaceInSecurity` via the same replacement path used by `place_on_security_observed`.
4. Return `false` and leave state unchanged if a replacement cancels or parks.
5. Remove the permanent from battle area, expire attached modifiers for that permanent, and insert the top card at `security[0]` for bottom security.
6. Return `true` only after the security placement has actually happened.

Use this shape, keeping imports local to match nearby style:

```rust
pub(crate) fn place_sourceless_permanent_on_security_bottom(
    &mut self,
    player_id: PlayerId,
    target: PermanentHandle,
    observer_player: PlayerId,
) -> bool {
    use crate::enums::{StackPosition, Zone};
    use crate::replacement::{ReplacementOutcome, ReplacementSubject};

    let Some(permanent) = self
        .player(target.player)
        .battle_area
        .get(target.index as usize)
    else {
        return false;
    };
    if permanent.card_sources.len() != 1 {
        return false;
    }
    let source_card = permanent.top_card().handle();
    let subject = ReplacementSubject::Card(source_card, Zone::BattleArea);
    let cause = self.infer_effect_cause(player_id);

    let outcome = self.try_replace(
        crate::enums::EffectTiming::WhenWouldPlaceInSecurity,
        subject,
        cause,
        Some(Zone::Security),
    );
    if self.pending_selection.is_some() {
        return false;
    }
    if !matches!(outcome, ReplacementOutcome::None) {
        return false;
    }

    let permanent = self.player_mut(target.player).battle_area.remove(target.index as usize);
    self.modifiers.expire_on_permanent_leave(target);
    let card = permanent.top_card().clone();
    self.player_mut(player_id).security.insert(0, card);
    true
}
```

If `top_card().clone()` cannot move the card source out safely, remove the permanent first and then `pop()` from its `card_sources`. Do not duplicate the card.

- [ ] **Step 5: Lower replacement-cost steps**

In `code/digimon-engine/src/dsl_cards/step/replacement_outcome.rs`, add arms:

```rust
CompiledStep::TrashTopSecurityAndCancelReplacement { of } => {
    let player = crate::dsl_cards::step::resolve_player(ctx, *of);
    ctx.trash_top_security_and_cancel_current_replacement(player);
    true
}
CompiledStep::PlacePermanentBottomSecurityAndCancelReplacement { of, target } => {
    let player = crate::dsl_cards::step::resolve_player(ctx, *of);
    if let Some(ResolvedBinding::Permanent(handle)) = resolve_binding_ref(target, ctx, bindings) {
        ctx.place_sourceless_permanent_bottom_security_and_cancel_current_replacement(player, handle);
    }
    true
}
```

Import `resolve_binding_ref` and `ResolvedBinding` from `crate::dsl_cards::binding_ref`.

- [ ] **Step 6: Run focused verification**

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- replacement_cancels_only_when_top_security_cost_is_paid replacement_cancels_only_when_other_sourceless_digimon_is_placed_in_security --nocapture
cargo test --manifest-path code\digimon-engine\Cargo.toml --test replacements -- cross_permanent context_predicates --nocapture
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```powershell
git add code\digimon-dsl\src\step.rs code\digimon-dsl\src\compiled.rs code\digimon-dsl\src\compile.rs code\digimon-engine\src\effect_context\mod.rs code\digimon-engine\src\game_actions.rs code\digimon-engine\src\dsl_cards\step\replacement_outcome.rs code\digimon-engine\tests\dsl\phase3b_replacement_process.rs
git commit -m "feat: add replacement cost success helpers"
```

---

## Task 5: Author BT24-101 Jupitermon Production YAML

**Files:**
- Create: `code/digimon-engine/cards/bt24/BT24-101.yaml`
- Create: `code/digimon-engine/tests/cards_behavioral/bt24/bt24_101.rs`
- Modify: `code/digimon-engine/tests/cards_behavioral/bt24/mod.rs`

- [ ] **Step 1: Write failing BT24-101 tests**

Create `code/digimon-engine/tests/cards_behavioral/bt24/bt24_101.rs`:

```rust
//! BT24-101 Jupitermon.
//! DCGO reference: DCGO/Assets/Scripts/CardEffect/BT24/Yellow/BT24_101.cs
//! Covers security-cost body, OnLoseSecurity retaliation, and TS protection.

use digimon_engine::action::space::REPLACEMENT_ACCEPT;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::CardKind;
use digimon_engine::replacement::ReplacementCause;

#[test]
fn bt24_101_on_play_trashes_own_top_security_minus_13000_and_recovers_at_one_or_less() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-101")
        .expect("BT24-101 YAML loads")
        .add_card(make_test_card("SECURITY", "Security"))
        .add_card(make_test_card("RECOVER-A", "Recover A"))
        .add_card(make_test_card("RECOVER-B", "Recover B"))
        .add_card(make_trait_digimon("OPPONENT", &[]))
        .hand(0, &["BT24-101"])
        .security(0, &["SECURITY"])
        .deck(0, &["RECOVER-A", "RECOVER-B"])
        .memory(20)
        .start();
    let opponent = runner.place_on_field(1, "OPPONENT", Some(0));

    runner.play(0, 0).expect("play Jupitermon");
    pick_first_pending(&mut runner, "choose -13000 DP target");
    runner.auto_resolve().expect("resolve Recovery +2");

    assert!(trash_contains(&runner, 0, "SECURITY"));
    assert_eq!(runner.security_count(0), 2, "trashed one security, then recovered two");
    assert!(
        runner.dp_of(opponent).unwrap() <= -11000,
        "2000 DP test Digimon should have received -13000 DP"
    );
    assert_eq!(security_ids(&runner, 0), vec!["RECOVER-A", "RECOVER-B"]);
}

#[test]
fn bt24_101_on_lose_security_trashes_opponent_top_security_once_per_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-101")
        .expect("BT24-101 YAML loads")
        .add_card(make_test_card("OWN-SEC-A", "Own Security A"))
        .add_card(make_test_card("OWN-SEC-B", "Own Security B"))
        .add_card(make_test_card("OPP-SEC-A", "Opp Security A"))
        .add_card(make_test_card("OPP-SEC-B", "Opp Security B"))
        .security(0, &["OWN-SEC-A", "OWN-SEC-B"])
        .security(1, &["OPP-SEC-A", "OPP-SEC-B"])
        .start();
    let jupitermon = runner.place_on_field(0, "BT24-101", Some(0));
    let source = runner.top_card(jupitermon);

    {
        let mut ctx = EffectContext::new(&mut runner.game, source, Some(jupitermon), 0);
        assert!(ctx.trash_top_security(0));
    }
    runner.auto_resolve().expect("resolve first OnLoseSecurity retaliation");
    assert_eq!(runner.security_count(1), 1);

    {
        let mut ctx = EffectContext::new(&mut runner.game, source, Some(jupitermon), 0);
        assert!(ctx.trash_top_security(0));
    }
    runner.auto_resolve().expect("drain second same-turn removal");
    assert_eq!(
        runner.security_count(1),
        1,
        "once-per-turn OnLoseSecurity retaliation must not fire twice"
    );
}

#[test]
fn bt24_101_protects_ts_digimon_or_tamer_by_trashing_top_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-101")
        .expect("BT24-101 YAML loads")
        .add_card(make_trait_digimon("TS-TARGET", &["TS"]))
        .add_card(make_test_card("SECURITY-COST", "Security Cost"))
        .security(0, &["SECURITY-COST"])
        .start();
    runner.place_on_field(0, "BT24-101", Some(0));
    let target = runner.place_on_field(0, "TS-TARGET", Some(0));

    runner
        .game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);
    runner
        .game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Jupitermon protection");

    assert!(permanent_exists(&runner, 0, "TS-TARGET"));
    assert!(trash_contains(&runner, 0, "SECURITY-COST"));
}

fn make_trait_digimon(id: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn pick_first_pending(runner: &mut DebugRunner, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect(label);
}

fn trash_contains(runner: &DebugRunner, player: usize, id: &str) -> bool {
    runner.game.players[player]
        .trash
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == id)
}

fn security_ids(runner: &DebugRunner, player: usize) -> Vec<&str> {
    runner.game.players[player]
        .security
        .iter()
        .map(|card| card.card_id(&runner.game.card_data))
        .collect()
}

fn permanent_exists(runner: &DebugRunner, player: usize, id: &str) -> bool {
    runner.game.players[player]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == id)
}
```

- [ ] **Step 2: Register and run failing tests**

Add to `code/digimon-engine/tests/cards_behavioral/bt24/mod.rs`:

```rust
mod bt24_101;
```

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test cards_behavioral -- bt24_101 --nocapture
```

Expected before YAML exists: tests fail at DSL card load.

- [ ] **Step 3: Author BT24-101 YAML**

Create `code/digimon-engine/cards/bt24/BT24-101.yaml`:

```yaml
card: BT24-101
name: Jupitermon
kind: digimon
level: 6
color: [yellow]
cost: 12
dp: 13000
traits: [Shaman, Olympos XII, Iliad, TS]
alt_paths:
  - kind: digivolve
    from:
      all_of:
        - level_eq: 5
        - trait_has: TS
    cost: 5
  - kind: digivolve
    from:
      all_of:
        - level_eq: 5
        - name_contains: Aegiochusmon
    cost: 5

effects:
  - when: [on_play, when_digivolving]
    summary: "Trash own top security, -13000 DP, then Recovery +2 at <=1 security"
    process:
      - trash_top_security: { of: you }
      - select_opponent_permanent:
          bind_as: dp_target
          filter: { kind: digimon }
          prompt: "Select 1 opponent Digimon to get -13000 DP"
      - add_dp_modifier:
          target: dp_target
          value: -13000
          expiry: until_opponent_turn_end
      - if:
          condition: { security_count_lte: 1 }
          then:
            - recover: { of: you, count: 2 }

  - when: on_lose_security
    once_per_turn: true
    active_when: { all_turns: true }
    summary: "[All Turns][OPT] When your security stack is removed from, trash opponent top security"
    process:
      - trash_top_security: { of: opponent }

  - kind: replacement
    timing: when_would_leave_battle_area
    once_per_turn: true
    active_when:
      all_of:
        - replacement_subject_is_mine: true
        - any_of:
            - kind: digimon
            - kind: tamer
        - trait_has: TS
        - security_count_gte: 1
    summary: "[All Turns][OPT] By trashing top security, a TS Digimon or Tamer does not leave"
    process:
      - trash_top_security_and_cancel_replacement: { of: you }
```

If the alt path with dynamic Aegiochusmon cost equal to security count is already expressible in the current DSL, replace the second `cost: 5` with the dynamic formula in this task. If it is not expressible, keep the fixed-cost path out of the YAML and leave a tracker note; do not knowingly author wrong dynamic cost.

- [ ] **Step 4: Run verification**

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test cards_behavioral -- bt24_101 --nocapture
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- security_stack_steps replacement_cancels_only_when_top_security_cost_is_paid effect_refiring --nocapture
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```powershell
git add code\digimon-engine\cards\bt24\BT24-101.yaml code\digimon-engine\tests\cards_behavioral\bt24\bt24_101.rs code\digimon-engine\tests\cards_behavioral\bt24\mod.rs
git commit -m "feat: author Jupitermon DSL card"
```

---

## Task 6: Author BT24-040 Venusmon Production YAML

**Files:**
- Create: `code/digimon-engine/cards/bt24/BT24-040.yaml`
- Create: `code/digimon-engine/tests/cards_behavioral/bt24/bt24_040.rs`
- Modify: `code/digimon-engine/tests/cards_behavioral/bt24/mod.rs`

- [ ] **Step 1: Write failing BT24-040 tests**

Create `code/digimon-engine/tests/cards_behavioral/bt24/bt24_040.rs`:

```rust
//! BT24-040 Venusmon.
//! DCGO reference: DCGO/Assets/Scripts/CardEffect/BT24/Yellow/BT24_040.cs
//! Covers source trash, WD suppression locks, and TS protection cost.

use digimon_engine::action::space::REPLACEMENT_ACCEPT;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;

#[test]
fn bt24_040_on_play_trashes_all_sources_then_locks_two_opponent_cards() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-040")
        .expect("BT24-040 YAML loads")
        .add_card(make_trait_digimon("SRC-A", &[]))
        .add_card(make_trait_digimon("SRC-B", &[]))
        .add_card(make_trait_digimon("SOURCE-HOST", &[]))
        .add_card(make_trait_digimon("LOCK-DIGIMON", &[]))
        .add_card(make_tamer("LOCK-TAMER"))
        .hand(0, &["BT24-040"])
        .memory(20)
        .start();
    let source_stack = runner.place_stack(1, &["SRC-A", "SRC-B", "SOURCE-HOST"]);
    let lock_digimon = runner.place_on_field(1, "LOCK-DIGIMON", Some(0));
    let lock_tamer = runner.place_on_field(1, "LOCK-TAMER", Some(0));

    runner.play(0, 0).expect("play Venusmon");
    pick_first_pending(&mut runner, "choose stack to strip");
    pick_first_pending(&mut runner, "choose first lock target");
    pick_first_pending(&mut runner, "choose second lock target");
    runner.auto_resolve().expect("finish Venusmon On Play");

    assert_eq!(source_count(&runner, source_stack), 1, "only the top card remains");
    assert!(trash_contains(&runner, 1, "SRC-A"));
    assert!(trash_contains(&runner, 1, "SRC-B"));
    assert_lock_modifiers(&runner, lock_digimon);
    assert_lock_modifiers(&runner, lock_tamer);
}

#[test]
fn bt24_040_when_digivolving_shares_the_same_effect_body() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-040")
        .expect("BT24-040 YAML loads")
        .add_card(make_trait_digimon("LV5-TS", &["TS"]))
        .add_card(make_trait_digimon("SRC-A", &[]))
        .add_card(make_trait_digimon("SOURCE-HOST", &[]))
        .add_card(make_trait_digimon("LOCK-DIGIMON", &[]))
        .hand(0, &["BT24-040"])
        .memory(20)
        .start();
    let base = runner.place_on_field(0, "LV5-TS", Some(0));
    let source_stack = runner.place_stack(1, &["SRC-A", "SOURCE-HOST"]);
    let lock_digimon = runner.place_on_field(1, "LOCK-DIGIMON", Some(0));
    let evo_card = runner.game.players[0].hand.remove(0);
    assert!(runner.game.digivolve_onto(0, base.index as usize, evo_card));

    runner.game.fire_timing_for_permanent(EffectTiming::WhenDigivolving, base);
    pick_first_pending(&mut runner, "choose stack to strip");
    pick_first_pending(&mut runner, "choose lock target");
    runner.auto_resolve().expect("finish Venusmon When Digivolving");

    assert_eq!(source_count(&runner, source_stack), 1);
    assert!(trash_contains(&runner, 1, "SRC-A"));
    assert_lock_modifiers(&runner, lock_digimon);
}

#[test]
fn bt24_040_protects_ts_digimon_by_placing_other_sourceless_digimon_bottom_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-040")
        .expect("BT24-040 YAML loads")
        .add_card(make_trait_digimon("TS-TARGET", &["TS"]))
        .add_card(make_trait_digimon("COST-BODY", &[]))
        .start();
    runner.place_on_field(0, "BT24-040", Some(0));
    let target = runner.place_on_field(0, "TS-TARGET", Some(0));
    runner.place_on_field(0, "COST-BODY", Some(0));

    runner
        .game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);
    runner
        .game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Venusmon protection");
    pick_first_pending(&mut runner, "choose sourceless cost Digimon");

    assert!(permanent_exists(&runner, 0, "TS-TARGET"));
    assert!(!permanent_exists(&runner, 0, "COST-BODY"));
    assert_eq!(
        runner.game.players[0].security.first().unwrap().card_id(&runner.game.card_data),
        "COST-BODY",
        "cost body was placed at security bottom"
    );
}

fn make_trait_digimon(id: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn make_tamer(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card
}

fn pick_first_pending(runner: &mut DebugRunner, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect(label);
}

fn source_count(runner: &DebugRunner, handle: PermanentHandle) -> usize {
    runner.game.players[handle.player as usize].battle_area[handle.index as usize]
        .card_sources
        .len()
}

fn trash_contains(runner: &DebugRunner, player: usize, id: &str) -> bool {
    runner.game.players[player]
        .trash
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == id)
}

fn permanent_exists(runner: &DebugRunner, player: usize, id: &str) -> bool {
    runner.game.players[player]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == id)
}

fn assert_lock_modifiers(runner: &DebugRunner, handle: PermanentHandle) {
    assert!(runner.modifiers().has(handle, ModifierType::CannotSuspend));
    assert!(
        runner
            .modifiers()
            .has(handle, ModifierType::CannotActivateWhenDigivolvingEffects)
    );
}
```

- [ ] **Step 2: Register and run failing tests**

Add to `code/digimon-engine/tests/cards_behavioral/bt24/mod.rs`:

```rust
mod bt24_040;
```

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test cards_behavioral -- bt24_040 --nocapture
```

Expected before YAML exists: tests fail at DSL card load.

- [ ] **Step 3: Author BT24-040 YAML**

Create `code/digimon-engine/cards/bt24/BT24-040.yaml`:

```yaml
card: BT24-040
name: Venusmon
kind: digimon
level: 6
color: [yellow, blue]
cost: 12
dp: 12000
traits: [Shaman, Olympos XII, Iliad, TS]
alt_paths:
  - kind: digivolve
    from:
      all_of:
        - level_eq: 5
        - trait_has: TS
    cost: 3

effects:
  - kind: cost_reduction
    when_playing_this: true
    amount: 5
    condition: { security_count_lte: 3 }
    summary: "When this card would be played, if you have 3 or fewer security, reduce play cost by 5"

  - when: [on_play, when_digivolving]
    summary: "Trash all sources of one opponent Digimon, then lock two opponent Digimon or Tamers"
    process:
      - select_opponent_permanent:
          bind_as: source_stack
          filter: { kind: digimon }
          prompt: "Trash all digivolution cards of 1 opponent Digimon"
      - trash_all_sources: { target: source_stack }
      - select_opponent_permanent:
          bind_as: lock_1
          optional: true
          filter:
            any_of:
              - kind: digimon
              - kind: tamer
          prompt: "Select the first opponent Digimon or Tamer"
      - add_modifier:
          target: lock_1
          modifier: CannotSuspend
          value: 1
          expiry: until_opponent_turn_end
      - add_modifier:
          target: lock_1
          modifier: CannotActivateWhenDigivolvingEffects
          value: 1
          expiry: until_opponent_turn_end
      - select_opponent_permanent:
          bind_as: lock_2
          optional: true
          filter:
            all_of:
              - any_of:
                  - kind: digimon
                  - kind: tamer
              - not_in_binding: lock_1
          prompt: "Select the second opponent Digimon or Tamer"
      - add_modifier:
          target: lock_2
          modifier: CannotSuspend
          value: 1
          expiry: until_opponent_turn_end
      - add_modifier:
          target: lock_2
          modifier: CannotActivateWhenDigivolvingEffects
          value: 1
          expiry: until_opponent_turn_end

  - kind: replacement
    timing: when_would_leave_battle_area
    once_per_turn: true
    active_when:
      all_of:
        - replacement_subject_is_mine: true
        - kind: digimon
        - trait_has: TS
        - replacement_cause: opponent_effect
    summary: "[All Turns][OPT] Place another sourceless Digimon as bottom security to prevent a TS Digimon leaving"
    process:
      - select_own_permanent:
          bind_as: cost_body
          filter:
            all_of:
              - kind: digimon
              - materials_count_lte: 0
              - not_in_binding: replacement_subject
          prompt: "Place 1 other Digimon with no digivolution cards as bottom security"
      - place_permanent_bottom_security_and_cancel_replacement:
          of: you
          target: cost_body
```

This deliberately uses two `select_opponent_permanent` steps instead of `select_count_capped_multi`, because the current count-capped helper is card-zone based and does not select field permanents.

- [ ] **Step 4: Run verification**

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test cards_behavioral -- bt24_040 --nocapture
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- source_stack_aggregates replacement_cancels_only_when_other_sourceless_digimon_is_placed_in_security --nocapture
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```powershell
git add code\digimon-engine\cards\bt24\BT24-040.yaml code\digimon-engine\tests\cards_behavioral\bt24\bt24_040.rs code\digimon-engine\tests\cards_behavioral\bt24\mod.rs
git commit -m "feat: author Venusmon DSL card"
```

---

## Task 7: Tracker Cleanup and Validation

**Files:**
- Modify: `docs/RUST_ENGINE_GAPS.md`
- Modify: `qa/dsl-vocab-gaps.md`
- Modify: `qa/archetype-qa/dsl/red-hybrid-ancientgreymon-2026-05-03-dsl-engine-gaps.md`
- Modify: `qa/archetype-qa/dsl/ts-olympos-2026-05-03-dsl-engine-gaps.md`
- Modify: `qa/archetype-qa/dsl/bg-imperial-cross-archetype-gaps-2026-05-03.md`
- Modify: `qa/qa-reports/validated_cards_dsl.json`

- [ ] **Step 1: Update tracker rows**

Make these specific tracker changes:

- `BT17-097`: mark Delay replacement prompt/continuation fixed only if `replacement_integration::bt17_097*` passes. Keep security Tamer play open unless production YAML includes it and has tests.
- `BT17-009`: mark multi-bucket reveal plus inherited free Tamer play fixed after `bt17_009` behavioral tests pass.
- `BT24-031`: mark On Play reveal and inherited top-security-to-hand/Recovery fixed after `bt24_031` behavioral tests pass.
- `BT24-040`: mark trash-all-sources, WD suppression lock, and placement-cost protection fixed after `bt24_040` behavioral tests pass.
- `BT24-101`: mark top-security trash/Recovery, OnLoseSecurity observer, and trash-security protection fixed after `bt24_101` behavioral tests pass.

Do not mark an entire archetype ready unless every card in that archetype pool has faithful YAML or a documented non-blocking exclusion.

- [ ] **Step 2: Update validated card tracker**

In `qa/qa-reports/validated_cards_dsl.json`, add or update entries for cards that now have full production YAML and behavioral tests:

```json
"BT17-009": {
  "verdict": "IMPLEMENTED",
  "date": "2026-05-03",
  "source": "rust-dsl",
  "tests": ["cards_behavioral::bt17::bt17_009"]
}
```

Repeat for `BT24-031`, `BT24-040`, and `BT24-101` only if their tests pass. If a card is still partial, use `"verdict": "BLOCKED"` or `"PARTIAL"` with the exact blocker instead of claiming implemented.

- [ ] **Step 3: Run full focused suite**

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test option_flow -- replacement_integration::bt17_097 --nocapture
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- reveal_buckets security_stack_steps source_stack_aggregates replacement_context effect_refiring hybrid_tamer_digivolve --nocapture
cargo test --manifest-path code\digimon-engine\Cargo.toml --test effect_context -- security_stack_operations effect_digivolve_union_zones effect_refiring --nocapture
cargo test --manifest-path code\digimon-engine\Cargo.toml --test replacements -- cross_permanent context_predicates route_replacements nested_select_substrate --nocapture
cargo test --manifest-path code\digimon-engine\Cargo.toml --test cards_behavioral -- bt17_009 bt24_031 bt24_040 bt24_101 --nocapture
```

Expected: all pass. The `replacements` suite may print expected `#[should_panic]` output while still exiting successfully.

- [ ] **Step 4: Run broad regression check**

Run:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test option_flow -- --nocapture
```

Expected: the previous five BT17-097 failures are gone. If new failures appear, record them in the tracker update and do not claim broad option-flow green.

- [ ] **Step 5: Commit**

```powershell
git add docs\RUST_ENGINE_GAPS.md qa\dsl-vocab-gaps.md qa\archetype-qa\dsl\red-hybrid-ancientgreymon-2026-05-03-dsl-engine-gaps.md qa\archetype-qa\dsl\ts-olympos-2026-05-03-dsl-engine-gaps.md qa\archetype-qa\dsl\bg-imperial-cross-archetype-gaps-2026-05-03.md qa\qa-reports\validated_cards_dsl.json
git commit -m "docs: close outstanding archetype blockers"
```

---

## Final Verification

Run this final matrix before declaring the plan complete:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- reveal_buckets security_stack_steps source_stack_aggregates replacement_context effect_refiring hybrid_tamer_digivolve --nocapture
cargo test --manifest-path code\digimon-engine\Cargo.toml --test effect_context -- security_stack_operations effect_digivolve_union_zones effect_refiring --nocapture
cargo test --manifest-path code\digimon-engine\Cargo.toml --test replacements -- cross_permanent context_predicates route_replacements nested_select_substrate --nocapture
cargo test --manifest-path code\digimon-engine\Cargo.toml --test option_flow -- --nocapture
cargo test --manifest-path code\digimon-engine\Cargo.toml --test cards_behavioral -- bt17_009 bt24_031 bt24_040 bt24_101 --nocapture
```

Expected: every command exits 0.

Also run:

```powershell
git status --short
```

Expected: no uncommitted files from this plan except pre-existing unrelated dirty files explicitly called out by the orchestrator.

## Self-Review

- Spec coverage: Covers the open BT17-097 Delay failures, BT17-009 inherited Tamer play, BT24-031 top-security-to-hand plus Recovery, BT24-040 source/lock/protection blockers, BT24-101 security/recovery/observer/protection blockers, tracker updates, and production YAML gates.
- DCGO coverage: Each card task names the exact pinned submodule file to compare.
- Completeness check: No intentional stubs remain; BT24-040 and BT24-101 include concrete behavioral test modules with assertions and helper functions.
- Contract guard: No task changes `ACTION_SPACE_SIZE`, tensor layout, PyO3 exports, frontend constants, or RL wrappers.
