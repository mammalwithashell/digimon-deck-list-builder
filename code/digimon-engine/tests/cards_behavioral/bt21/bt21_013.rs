//! BT21-013 Agunimon — Digimon, Lv.4, Red, DP 5000, Cost 5.
//! Traits: Wizard, Hero.  Form: Hybrid.  Attribute: Variable.
//! Evo: Lv.3 Red / cost 2.
//!
//! # Card text (cards.json — verbatim)
//!
//! **[When Digivolving]** You may place 1 [Hybrid] or [Hero] trait Digimon card
//! from your hand or trash as this Digimon's bottom digivolution card or under
//! any of your red Tamers with inherited effects.
//!
//! **[When Attacking]** This Digimon may digivolve into a red [Hybrid] or [Hero]
//! trait Digimon card in the hand with the digivolution cost reduced by 1.
//!
//! **Inherited:** [Your Turn] This Digimon gets +2000 DP.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Red/BT21_013.cs
//!
//! # Patterns this test covers
//! - Clause 1: [When Digivolving] optional place Hybrid/Hero from hand or trash
//!   under self or red Tamer with inherited (select_own_permanent,
//!   select_effect_choice, select_hand/select_trash, place_as_bottom_source).
//!   A6 trash-to-digi-stack placement; E1 branch choice (hand vs trash).
//! - Clause 2: [When Attacking] optional effect-initiated digivolve, cost -1
//!   (select_hand trait+color filter → effect_initiated_digivolve).
//! - Clause 3: Inherited [Your Turn] +2000 DP self-aura (D4 declarative aura).
//!
//! # Gap status (verified 2026-05-21 against qa/resolved-gaps.md)
//!
//! - **G-WHEN-DIGIVOLVING-DISPATCH — RESOLVED** (Phase 2 Track D). WhenDigivolving
//!   triggered effects dispatch from the permanent via `enqueue_triggered`.
//!   The prior `#[ignore]` annotations on the 4 clause-1 tests were a FIXTURE
//!   bug: the tests placed a `BASE-LV3` filler permanent (not BT21-013/Agunimon)
//!   on the field, so firing WhenDigivolving on the filler reached no Agunimon
//!   effect. Fixed here — the tests place the real Agunimon card.
//! - **select_hand / select_trash `trait_has` filter — ENFORCED**. `install_select_hand`
//!   / `install_select_trash` evaluate the compiled filter predicate per card
//!   (`eval_predicate_with_bindings`). The 2 prior filter-eval `#[ignore]`
//!   tests are un-ignored here.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Helpers ─────────────────────────────────────────────────────────────────

const AGUNIMON_YAML: &str = include_str!("../../../cards/bt21/BT21-013.yaml");

/// A Lv.4 red Digimon card with the [Hybrid] trait — valid clause-1 source.
fn make_hybrid(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.traits = vec!["Hybrid".to_string()];
    c.level = Some(4);
    c.colors = vec![CardColor::Red];
    c
}

/// A Lv.4 red Digimon card with the [Hero] trait — valid clause-1 source.
fn make_hero(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.traits = vec!["Hero".to_string()];
    c.level = Some(4);
    c.colors = vec![CardColor::Red];
    c
}

/// A Digimon card with neither Hybrid nor Hero — invalid clause-1/2 source.
fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.colors = vec![CardColor::Red];
    c
}

/// A red [Hybrid] Lv.5 Digimon that can be digivolved INTO from a Lv.4 red
/// Agunimon — carries a matching `evo_cost { color: red, level: 4 }` so the
/// clause-2 `effect_initiated_digivolve` (ignore_requirements: false) finds a
/// legal evo path. Printed digivolution cost 3; clause 2 reduces it to 2.
fn make_red_hybrid_evo_target(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.traits = vec!["Hybrid".to_string()];
    c.level = Some(5);
    c.dp = Some(7000);
    c.colors = vec![CardColor::Red];
    c.play_cost = 8;
    // card_color 0 = Red; digivolves from a Lv.4.
    c.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 4,
        memory_cost: 3,
    }];
    c
}

/// Build a runner with Agunimon loaded from the embedded YAML pack.
fn runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(AGUNIMON_YAML)
        .expect("BT21-013 YAML parses")
        .memory(10)
        .start()
}

/// Fire the [When Digivolving] triggered batch for a permanent that carries
/// (or is) Agunimon, mirroring what the digivolution flow does.
fn fire_when_digivolving(runner: &mut DebugRunner, handle: digimon_engine::permanent::PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
}

/// Put a card already registered in `card_data` into player `p`'s trash.
fn push_to_trash(runner: &mut DebugRunner, p: digimon_engine::enums::PlayerId, card_id: &str) {
    let game = runner.game_mut();
    let idx = game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} registered"));
    let next = game.next_card_index();
    game.players[p as usize]
        .trash
        .push(CardSource::new(idx, p, next));
}

// ─── Section 1: Structural assertions ────────────────────────────────────────

#[test]
fn bt21_013_compiles_with_three_clauses() {
    let runner = runner();
    let compiled = runner
        .compiled_card("BT21-013")
        .expect("BT21-013 in compiled_cards");

    assert_eq!(
        compiled.effects.len(),
        3,
        "BT21-013 must have exactly 3 clauses: when_digivolving, when_attacking, inherited aura"
    );
}

#[test]
fn bt21_013_has_when_digivolving_clause_optional() {
    let runner = runner();
    let compiled = runner
        .compiled_card("BT21-013")
        .expect("BT21-013 in compiled_cards");

    let wd = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::WhenDigivolving))
        .expect("when_digivolving clause must exist");

    assert!(
        wd.optional,
        "WhenDigivolving clause must be optional ('you may')"
    );
    assert!(!wd.once_per_turn, "WhenDigivolving clause has no OPT");
    assert_eq!(
        wd.scope,
        CompiledScope::FaceUp,
        "WhenDigivolving clause must have default FaceUp scope (not inherited)"
    );
}

#[test]
fn bt21_013_has_when_attacking_clause_optional() {
    let runner = runner();
    let compiled = runner
        .compiled_card("BT21-013")
        .expect("BT21-013 in compiled_cards");

    let wa = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::WhenAttacking))
        .expect("when_attacking clause must exist");

    assert!(
        wa.optional,
        "WhenAttacking clause must be optional ('you may')"
    );
    assert_eq!(
        wa.scope,
        CompiledScope::FaceUp,
        "WhenAttacking clause must have default FaceUp scope (not inherited)"
    );
}

#[test]
fn bt21_013_has_inherited_aura_clause() {
    let runner = runner();
    let compiled = runner
        .compiled_card("BT21-013")
        .expect("BT21-013 in compiled_cards");

    let aura = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope, dp_modifier, ..
        }) => Some((*scope, *dp_modifier)),
        _ => None,
    });

    let (scope, dp) = aura.expect("BT21-013 must have a Declarative::Aura clause");
    assert_eq!(
        scope,
        CompiledScope::Inherited,
        "aura clause must have Inherited scope"
    );
    assert_eq!(dp, Some(2000), "aura dp_modifier must be +2000 DP");
}

// ─── Section 2: Clause 1 — [When Digivolving] place-as-source ────────────────

/// Positive: firing [When Digivolving] on Agunimon with a Hybrid Digimon in
/// hand installs a selection (the destination pick).
#[test]
fn bt21_013_when_digivolving_installs_selection_with_hybrid_in_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(AGUNIMON_YAML)
        .expect("BT21-013 YAML parses")
        .add_card(make_hybrid("HYBRID-HAND"))
        .hand(0, &["HYBRID-HAND"])
        .memory(10)
        .start();

    let agunimon = runner.place_on_field(0, "BT21-013", Some(0));

    fire_when_digivolving(&mut runner, agunimon);

    assert!(
        runner.pending_selection().is_some(),
        "a selection must install when WhenDigivolving fires with a Hybrid in hand"
    );
}

/// Negative: firing [When Digivolving] with no Agunimon on field reaches no
/// effect — no selection installs.
#[test]
fn bt21_013_when_digivolving_no_selection_without_agunimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(AGUNIMON_YAML)
        .expect("BT21-013 YAML parses")
        .add_card(make_hybrid("HYBRID-HAND"))
        .add_card(make_filler("FILLER-FIELD"))
        .hand(0, &["HYBRID-HAND"])
        .memory(10)
        .start();

    // A non-Agunimon permanent on field — firing WhenDigivolving on it
    // reaches no Agunimon clause.
    let filler = runner.place_on_field(0, "FILLER-FIELD", Some(0));

    fire_when_digivolving(&mut runner, filler);

    assert!(
        runner.pending_selection().is_none(),
        "no selection may install — the permanent is not Agunimon"
    );
}

/// Negative: when Agunimon fires [When Digivolving] but there is no eligible
/// Hybrid/Hero source in hand OR trash, the clause is optional ('you may') —
/// the destination prompt may still install but it is declinable, and after
/// declining nothing happens. With no source cards at all, the place is a
/// no-op.
#[test]
fn bt21_013_when_digivolving_no_source_card_is_a_noop() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(AGUNIMON_YAML)
        .expect("BT21-013 YAML parses")
        .add_card(make_filler("FILLER-HAND"))
        .hand(0, &["FILLER-HAND"])
        .memory(10)
        .start();

    let agunimon = runner.place_on_field(0, "BT21-013", Some(0));
    let stack_before = runner.game.players[0].battle_area[agunimon.index as usize]
        .card_sources
        .len();

    fire_when_digivolving(&mut runner, agunimon);

    // Resolve through whatever installed (the clause is optional / its
    // select steps are optional, so auto_resolve declines down the line).
    let _ = runner.auto_resolve();

    let stack_after = runner.game.players[0].battle_area[agunimon.index as usize]
        .card_sources
        .len();
    assert_eq!(
        stack_after, stack_before,
        "with no Hybrid/Hero source card, nothing is placed under Agunimon"
    );
    assert_eq!(
        runner.hand_size(0),
        1,
        "the non-Hybrid filler must remain in hand — it is not an eligible source"
    );
}

/// From hand branch: selecting Agunimon as the destination, "From hand", then
/// a Hybrid card places it as the bottom digivolution source.
#[test]
fn bt21_013_when_digivolving_from_hand_places_hybrid_as_bottom_source() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(AGUNIMON_YAML)
        .expect("BT21-013 YAML parses")
        .add_card(make_hybrid("HYBRID-SRC"))
        .hand(0, &["HYBRID-SRC"])
        .memory(10)
        .start();

    let agunimon = runner.place_on_field(0, "BT21-013", Some(0));
    let stack_before = runner.game.players[0].battle_area[agunimon.index as usize]
        .card_sources
        .len();

    fire_when_digivolving(&mut runner, agunimon);

    // Step 1: destination selection — Agunimon is the only legal destination.
    {
        let view = runner
            .pending_selection_view()
            .expect("destination selection installs");
        runner
            .execute_action(0, view.valid_action_ids[0])
            .expect("select Agunimon as destination");
    }

    // Step 2: zone branch — choose "From hand" (branch 0).
    runner.execute_branch(0).expect("choose From hand branch");

    // Step 3: select the HYBRID-SRC card from hand.
    {
        let view = runner
            .pending_selection_view()
            .expect("hand selection installs");
        runner
            .execute_action(0, view.valid_action_ids[0])
            .expect("select HYBRID-SRC from hand");
    }
    let _ = runner.auto_resolve();

    let stack_after = runner.game.players[0].battle_area[agunimon.index as usize]
        .card_sources
        .len();
    assert_eq!(
        stack_after,
        stack_before + 1,
        "placing a card as bottom source must grow Agunimon's digivolution stack by 1"
    );
    assert_eq!(
        runner.hand_size(0),
        0,
        "HYBRID-SRC must have left hand after being placed as source"
    );
}

/// From trash branch: places a Hero Digimon from trash as the bottom source.
#[test]
fn bt21_013_when_digivolving_from_trash_places_hero_as_bottom_source() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(AGUNIMON_YAML)
        .expect("BT21-013 YAML parses")
        .add_card(make_hero("HERO-TRASH"))
        .memory(10)
        .start();

    push_to_trash(&mut runner, 0, "HERO-TRASH");

    let agunimon = runner.place_on_field(0, "BT21-013", Some(0));
    let stack_before = runner.game.players[0].battle_area[agunimon.index as usize]
        .card_sources
        .len();
    let trash_before = runner.trash_size(0);

    fire_when_digivolving(&mut runner, agunimon);

    // Step 1: destination — Agunimon.
    {
        let view = runner
            .pending_selection_view()
            .expect("destination selection installs");
        runner
            .execute_action(0, view.valid_action_ids[0])
            .expect("select Agunimon as destination");
    }

    // Step 2: choose "From trash" (branch 1).
    runner.execute_branch(1).expect("choose From trash branch");

    // Step 3: select HERO-TRASH from trash.
    {
        let view = runner
            .pending_selection_view()
            .expect("trash selection installs");
        runner
            .execute_action(0, view.valid_action_ids[0])
            .expect("select HERO-TRASH from trash");
    }
    let _ = runner.auto_resolve();

    let stack_after = runner.game.players[0].battle_area[agunimon.index as usize]
        .card_sources
        .len();
    assert_eq!(
        stack_after,
        stack_before + 1,
        "placing a card from trash as bottom source must grow Agunimon's stack by 1"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before - 1,
        "HERO-TRASH must have left trash after being placed as source"
    );
}

/// Declining the destination selection (PASS — the clause is "you may")
/// leaves Agunimon's stack and the hand unchanged.
#[test]
fn bt21_013_when_digivolving_declining_does_nothing() {
    use digimon_engine::action::space::PASS;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(AGUNIMON_YAML)
        .expect("BT21-013 YAML parses")
        .add_card(make_hybrid("HYBRID-DECLINE"))
        .hand(0, &["HYBRID-DECLINE"])
        .memory(10)
        .start();

    let agunimon = runner.place_on_field(0, "BT21-013", Some(0));
    let stack_before = runner.game.players[0].battle_area[agunimon.index as usize]
        .card_sources
        .len();
    let hand_before = runner.hand_size(0);

    fire_when_digivolving(&mut runner, agunimon);

    // The destination select_own_permanent is optional — PASS declines.
    let view = runner
        .pending_selection_view()
        .expect("destination selection installs");
    assert!(
        view.is_optional,
        "the destination selection must be optional ('you may place')"
    );
    runner
        .execute_action(0, PASS)
        .expect("PASS declines the optional destination selection");

    let _ = runner.auto_resolve();

    let stack_after = runner.game.players[0].battle_area[agunimon.index as usize]
        .card_sources
        .len();
    assert_eq!(
        stack_after, stack_before,
        "declining must leave Agunimon's digivolution stack unchanged"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "declining must leave the Hybrid card in hand"
    );
}

// ─── Section 3: Clause 2 — [When Attacking] digivolve cost -1 ────────────────

/// Positive: when Agunimon attacks and a red Hybrid Digimon is in hand,
/// a hand selection prompt installs.
#[test]
fn bt21_013_when_attacking_installs_hand_selection_with_hybrid_in_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(AGUNIMON_YAML)
        .expect("BT21-013 YAML parses")
        .add_card(make_red_hybrid_evo_target("HYBRID-EVO"))
        .add_card(make_filler("DEFENDER"))
        .hand(0, &["HYBRID-EVO"])
        .memory(10)
        .start();

    let attacker = runner.place_on_field(0, "BT21-013", Some(0));
    let defender = runner.place_on_field(1, "DEFENDER", Some(0));

    runner.attack_digimon(attacker, defender, false);

    // The WhenAttacking clause is optional ("This Digimon may digivolve");
    // its body's first step is a select_hand. If an outer accept/decline
    // prompt installs first, accept it.
    if runner.pending_kind() == Some(SelectionKind::Replacement) {
        runner
            .accept_optional_trigger()
            .expect("accept the outer optional-trigger prompt");
    }

    let pending = runner
        .pending_selection()
        .expect("a selection must install for the WhenAttacking digivolve");
    assert!(
        matches!(pending.kind, SelectionKind::Hand),
        "expected Hand selection for WhenAttacking digivolve, got {:?}",
        pending.kind
    );
}

/// Negative: without a red Hybrid/Hero Digimon in hand the WhenAttacking
/// clause's `select_hand` has zero candidates — no Hand selection installs.
/// (`install_select_hand` enforces the `trait_has` + `color_is` filter.)
#[test]
fn bt21_013_when_attacking_no_hand_selection_without_eligible_card() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(AGUNIMON_YAML)
        .expect("BT21-013 YAML parses")
        .add_card(make_filler("FILLER-HAND"))
        .add_card(make_filler("DEFENDER"))
        .hand(0, &["FILLER-HAND"])
        .memory(10)
        .start();

    let attacker = runner.place_on_field(0, "BT21-013", Some(0));
    let defender = runner.place_on_field(1, "DEFENDER", Some(0));

    runner.attack_digimon(attacker, defender, false);

    // No eligible card → no Hand selection. (An outer optional-trigger
    // accept/decline may still install, but never a Hand pick of the filler.)
    if let Some(p) = runner.pending_selection() {
        assert!(
            !matches!(p.kind, SelectionKind::Hand),
            "Hand selection must not install when hand has no red Hybrid/Hero Digimon; got {:?}",
            p.kind
        );
    }
}

/// Negative: a non-red Hybrid Digimon in hand is also ineligible — the
/// `color_is: red` half of the filter excludes it.
#[test]
fn bt21_013_when_attacking_excludes_non_red_hybrid() {
    let mut blue_hybrid = make_hybrid("BLUE-HYBRID");
    blue_hybrid.colors = vec![CardColor::Blue];

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(AGUNIMON_YAML)
        .expect("BT21-013 YAML parses")
        .add_card(blue_hybrid)
        .add_card(make_filler("DEFENDER"))
        .hand(0, &["BLUE-HYBRID"])
        .memory(10)
        .start();

    let attacker = runner.place_on_field(0, "BT21-013", Some(0));
    let defender = runner.place_on_field(1, "DEFENDER", Some(0));

    runner.attack_digimon(attacker, defender, false);

    if let Some(p) = runner.pending_selection() {
        assert!(
            !matches!(p.kind, SelectionKind::Hand),
            "Hand selection must not install for a non-red Hybrid; got {:?}",
            p.kind
        );
    }
}

/// Behavioral: accepting the WhenAttacking digivolve and picking the red
/// Hybrid digivolves Agunimon into it — the card leaves hand and becomes the
/// new top card of Agunimon's stack.
#[test]
fn bt21_013_when_attacking_digivolves_into_hybrid_and_card_leaves_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(AGUNIMON_YAML)
        .expect("BT21-013 YAML parses")
        .add_card(make_red_hybrid_evo_target("HYBRID-EVO-ACT"))
        .add_card(make_filler("DEFENDER"))
        .hand(0, &["HYBRID-EVO-ACT"])
        .memory(10)
        .start();

    let attacker = runner.place_on_field(0, "BT21-013", Some(0));
    let defender = runner.place_on_field(1, "DEFENDER", Some(0));

    let hand_before = runner.hand_size(0);
    assert_eq!(hand_before, 1, "pre: hand has 1 card (HYBRID-EVO-ACT)");
    let stack_before = runner.game.players[0].battle_area[attacker.index as usize]
        .card_sources
        .len();

    runner.attack_digimon(attacker, defender, false);

    if runner.pending_kind() == Some(SelectionKind::Replacement) {
        runner
            .accept_optional_trigger()
            .expect("accept the outer optional-trigger prompt");
    }

    // Pick HYBRID-EVO-ACT from the hand selection.
    {
        let view = runner
            .pending_selection_view()
            .expect("hand selection installs for the digivolve");
        runner
            .execute_action(0, view.valid_action_ids[0])
            .expect("select HYBRID-EVO-ACT to digivolve into");
    }
    let _ = runner.auto_resolve();

    let hand_after = runner.hand_size(0);
    assert!(
        hand_after < hand_before,
        "HYBRID-EVO-ACT must leave hand after effect_initiated_digivolve; \
         hand_before={hand_before} hand_after={hand_after}"
    );

    let attacker_perm = &runner.game.players[0].battle_area[attacker.index as usize];
    assert_eq!(
        attacker_perm.card_sources.len(),
        stack_before + 1,
        "Agunimon's stack must grow by 1 — HYBRID-EVO-ACT is the new top card"
    );
    assert_eq!(
        attacker_perm.top_card().card_id(&runner.game.card_data),
        "HYBRID-EVO-ACT",
        "the digivolved-into card must be the top card of Agunimon's stack"
    );
}

/// Cost firing: the digivolution cost is reduced by 1. The evo target's
/// printed digivolution cost is 3; clause 2 charges only 2 memory.
#[test]
fn bt21_013_when_attacking_digivolve_cost_reduced_by_one() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(AGUNIMON_YAML)
        .expect("BT21-013 YAML parses")
        .add_card(make_red_hybrid_evo_target("HYBRID-EVO-COST"))
        .add_card(make_filler("DEFENDER"))
        .hand(0, &["HYBRID-EVO-COST"])
        .memory(10)
        .start();

    let attacker = runner.place_on_field(0, "BT21-013", Some(0));
    let defender = runner.place_on_field(1, "DEFENDER", Some(0));

    runner.attack_digimon(attacker, defender, false);
    if runner.pending_kind() == Some(SelectionKind::Replacement) {
        runner
            .accept_optional_trigger()
            .expect("accept the outer optional-trigger prompt");
    }

    let memory_before = runner.game.memory;

    {
        let view = runner
            .pending_selection_view()
            .expect("hand selection installs for the digivolve");
        runner
            .execute_action(0, view.valid_action_ids[0])
            .expect("select HYBRID-EVO-COST to digivolve into");
    }
    let _ = runner.auto_resolve();

    // The attacker (P0) pays digivolution memory. Memory is signed toward the
    // owner of the turn; the magnitude of the swing must be the reduced cost.
    let spent = (memory_before - runner.game.memory).abs();
    assert_eq!(
        spent, 2,
        "printed digivolution cost 3, reduced by 1 → 2 memory spent; got {spent}"
    );
}

/// Declining the WhenAttacking prompt leaves hand and field unchanged.
#[test]
fn bt21_013_when_attacking_declining_does_nothing() {
    use digimon_engine::action::space::PASS;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(AGUNIMON_YAML)
        .expect("BT21-013 YAML parses")
        .add_card(make_red_hybrid_evo_target("HYBRID-DECLINE"))
        .add_card(make_filler("DEFENDER"))
        .hand(0, &["HYBRID-DECLINE"])
        .memory(10)
        .start();

    let attacker = runner.place_on_field(0, "BT21-013", Some(0));
    let defender = runner.place_on_field(1, "DEFENDER", Some(0));

    let hand_before = runner.hand_size(0);
    let stack_before = runner.game.players[0].battle_area[attacker.index as usize]
        .card_sources
        .len();

    runner.attack_digimon(attacker, defender, false);

    // Decline whatever installed: an outer accept/decline prompt or the
    // optional select_hand both expose PASS.
    if let Some(p) = runner.pending_selection() {
        if p.is_optional {
            runner
                .execute_action(0, PASS)
                .expect("PASS on the optional WhenAttacking prompt");
        }
    }
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "hand must be unchanged after declining the WhenAttacking digivolve"
    );
    assert_eq!(
        runner.game.players[0].battle_area[attacker.index as usize]
            .card_sources
            .len(),
        stack_before,
        "Agunimon's stack must be unchanged after declining the digivolve"
    );
}

// ─── Section 4: Clause 3 — Inherited [Your Turn] +2000 DP ────────────────────

/// Place Agunimon as a digivolution source under a carrier on P0's field.
fn place_agunimon_as_source(
    runner: &mut DebugRunner,
    carrier_id: &str,
) -> digimon_engine::permanent::PermanentHandle {
    let handle = runner.place_on_field(0, carrier_id, Some(0));
    {
        let game = runner.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "BT21-013")
            .expect("BT21-013 registered in card_data");
        let next = game.next_card_index();
        let agunimon_src = CardSource::new(data_idx, 0, next);
        let perm = &mut game.players[0].battle_area[handle.index as usize];
        perm.card_sources.insert(0, agunimon_src);
    }
    handle
}

/// On the controller's turn, the inherited +2000 DP contributes for the
/// Agunimon slot (index 0).
#[test]
fn bt21_013_inherited_dp_active_on_your_turn() {
    let mut carrier = make_test_card("CARRIER-LV5", "CarrierLv5");
    carrier.level = Some(5);
    carrier.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(AGUNIMON_YAML)
        .expect("BT21-013 YAML parses")
        .add_card(carrier)
        .memory(10)
        .start();

    let carrier_handle = place_agunimon_as_source(&mut runner, "CARRIER-LV5");

    assert_eq!(runner.turn_player(), 0, "precondition: P0's turn");

    let contribution = runner.game.source_dp_contribution(carrier_handle, 0);
    assert_eq!(
        contribution, 2000,
        "Agunimon's inherited +2000 DP must contribute on the controller's turn; got {contribution}"
    );
}

/// On the opponent's turn, the [Your Turn] gate suppresses the buff.
#[test]
fn bt21_013_inherited_dp_inactive_on_opponents_turn() {
    let mut carrier = make_test_card("CARRIER-LV5-OPP", "CarrierLv5Opp");
    carrier.level = Some(5);
    carrier.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(AGUNIMON_YAML)
        .expect("BT21-013 YAML parses")
        .add_card(carrier)
        .memory(10)
        .start();

    let carrier_handle = place_agunimon_as_source(&mut runner, "CARRIER-LV5-OPP");

    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "precondition: P1's turn");

    let contribution = runner.game.source_dp_contribution(carrier_handle, 0);
    assert_eq!(
        contribution, 0,
        "Agunimon's inherited +2000 DP must NOT contribute on the opponent's turn; got {contribution}"
    );
}

/// The top-card slot (index 1 = carrier) must NOT carry Agunimon's inherited
/// buff — inherited effects fire only from a digivolution source slot.
#[test]
fn bt21_013_inherited_dp_not_applied_to_top_card_slot() {
    let mut carrier = make_test_card("CARRIER-LV5-TOP", "CarrierLv5Top");
    carrier.level = Some(5);
    carrier.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(AGUNIMON_YAML)
        .expect("BT21-013 YAML parses")
        .add_card(carrier)
        .memory(10)
        .start();

    let carrier_handle = place_agunimon_as_source(&mut runner, "CARRIER-LV5-TOP");

    let top_contribution = runner.game.source_dp_contribution(carrier_handle, 1);
    assert_eq!(
        top_contribution, 0,
        "the top-card (carrier) slot must NOT carry Agunimon's inherited buff"
    );
}
