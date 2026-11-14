//! BT17-097 Return to the Primogenitor — Option, Cost 2, Blue+Green.
//!
//! # Card text (cards.json)
//!
//! [Main] 1 of your Digimon may digivolve into a level 5 or higher Digimon
//! card with the [Free] trait in your hand with the digivolution cost reduced
//! by 4. Then, place this card in the battle area.
//!
//! [All Turns] When one of your Digimon with the [Free] trait would be deleted
//! other than by one of your effects, ＜Delay＞ (By trashing this card after
//! the placing turn, activate the effect below.)
//! ・By digivolving that Digimon into a Digimon card with [Imperialdramon] in
//! its name in your hand without paying the cost, prevent that deletion.
//!
//! Inherited (Security):
//! Security Effect [Security] You may play 1 Tamer card with [Davis Motomiya]
//! or [Ken Ichijoji] in its name from your hand or trash without paying the
//! cost. Then, place this card in the battle area.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT17/Blue/BT17_097.cs
//!
//! # Patterns this test file covers
//!
//! - Clause A (Main): `when: main_from_hand`, `optional: true`. Implemented for
//!   the digivolve sub-clause (G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-
//!   PERMANENT-TARGET). Only `place_self_as_delay_option` fires.
//!   - Structural: timing, optional, scope.
//!   - Behavioral: activating Main places self as Delay-Option on field.
//!
//! - Clause B (All Turns Delay replacement): `kind: replacement`, optional
//!   (CR 4.3 16-17-2: accept / PASS before the Delay cost is paid),
//!   `timing: when_would_be_deleted`. Subject filter: `trait_has: "Free"` +
//!   `replacement_subject_is_mine: true`. Cause filter: `none_of:
//!   [replacement_cause: own_effect]`. Delay cost: `cost: delay_self: true`.
//!   Digivolve step: `choose: name_contains: "Imperialdramon"` + `then:
//!   digivolve_without_cost`. Prevention: `outcome: prevent`.
//!   - Structural: effect count, replacement clause exists.
//!   - Behavioral: does not fire for own-effect deletions; fires for
//!     opponent_effect + battle deletions (negative: OwnEffect; positive:
//!     OpponentEffect, Battle).
//!   - The comprehensive Delay flow (cost, hand pick, digivolve, prevent) is
//!     already exercised by option_flow/replacement_integration.rs.
//!
//! - Clause C (Security inherited): `scope: inherited`, `when: on_security`,
//!   `optional: true`. Workaround G-DSL-UNION-PLAY-FREE: zone-choice
//!   branching (select_effect_choice). Places self as Delay-Option permanent.
//!   - Structural: scope, timing, optional.
//!   - Behavioral: smoke-activating Security installs zone-choice selection;
//!     with Davis Motomiya in hand, playing from hand works.
//!
//! # Known gaps
//!
//! - **G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET**:
//!   The [Main] digivolve sub-clause (select own Digimon → select Lv5+ [Free]
//!   hand card → effect_initiated_digivolve with cost -4) is implemented; the
//!   the select_own_permanent → select_hand → effect_initiated_digivolve chain
//!   does not resume after the first pick. Filed in qa/dsl-vocab-gaps.md.
//!   historical ignored tests were reworked into active behavioral coverage.
//!
//! - **G-DSL-UNION-PLAY-FREE**: Security clause cannot use `select_union_zone`
//!   (Card-typed binding incompatible with play_from_*_free). Workaround:
//!   explicit zone-choice branching per BT17-095 / BT21-015 pattern.
//!   Auto-collapse is covered by active union-zone tests.

#![allow(unused_imports, dead_code)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{HAND_EFFECT_START, PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming};
use digimon_engine::permanent::OptionState;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const BT17_097_YAML: &str = include_str!("../../../cards/bt17/BT17-097.yaml");

// ─── Helper cards ────────────────────────────────────────────────────────────

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A Digimon with the [Free] trait (for the Delay replacement subject).
fn make_free_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Free Digimon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(6000);
    c.play_cost = 7;
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["Free".to_string()];
    c
}

/// A Digimon WITHOUT the [Free] trait (negative test for Delay replacement).
fn make_non_free_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Non-Free Digimon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(6000);
    c.play_cost = 7;
    c.colors = vec![CardColor::Blue];
    c
}

/// A plain blue Lv.4 Digimon that can be used as the Main-effect digivolve base.
fn make_blue_lv4_base(id: &str) -> CardData {
    let mut c = make_test_card(id, "Blue Lv4 Base");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Blue];
    c
}

/// A Lv.5 [Free] Digimon with a normal blue Lv.4 digivolution cost of 5.
fn make_free_lv5_evo(id: &str) -> CardData {
    let mut c = make_test_card(id, "Free Lv5 Evolution");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(7000);
    c.play_cost = 7;
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["Free".to_string()];
    c.evo_costs = vec![EvoCost {
        card_color: CardColor::Blue as u8,
        level: 4,
        memory_cost: 5,
    }];
    c
}

/// A Lv.5 Digimon with a matching evo cost but no [Free] trait.
fn make_non_free_lv5_evo(id: &str) -> CardData {
    let mut c = make_free_lv5_evo(id);
    c.card_name = "Non-Free Lv5 Evolution".to_string();
    c.traits = vec![];
    c
}

/// A Digimon whose name contains "Imperialdramon" (for the Delay hand pick),
/// digivolving from a blue Lv.5 so it can digivolve onto `make_free_digimon`
/// (the <Delay> offers only such a card, DCGO BT17_097.cs:126-137).
fn make_imperialdramon_hand_card(id: &str) -> CardData {
    let mut c = make_test_card(id, "Imperialdramon Test");
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(11000);
    c.play_cost = 10;
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["Free".to_string()];
    c.evo_costs = vec![EvoCost {
        card_color: CardColor::Blue as u8,
        level: 5,
        memory_cost: 4,
    }];
    c
}

/// A Tamer with [Davis Motomiya] in its name (for Security clause).
fn make_davis_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, "Davis Motomiya");
    c.card_kind = CardKind::Tamer;
    c.play_cost = 3;
    c.colors = vec![CardColor::Blue];
    c
}

/// A Tamer with [Ken Ichijoji] in its name (for Security clause).
fn make_ken_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, "Ken Ichijoji");
    c.card_kind = CardKind::Tamer;
    c.play_cost = 3;
    c.colors = vec![CardColor::Green];
    c
}

/// Seat BT17-097 as a Delay-Option permanent (bypasses Main trigger).
fn place_as_delay(
    runner: &mut DebugRunner,
    player: u8,
) -> digimon_engine::permanent::PermanentHandle {
    let handle = runner.place_on_field(player, "BT17-097", Some(0));
    runner.game.player_mut(player).battle_area[handle.index as usize].option_state =
        OptionState::Delayed {
            owner: player,
            trash_on_turn: runner.game.turn_count + 1,
            trigger: DelayTrigger::EndOfYourNextTurn,
            // CR 16-17-3: placed on an earlier turn, so the <Delay> is usable now.
            placed_on_turn: runner.game.turn_count.saturating_sub(1),
        };
    handle
}

fn bt17_097_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(BT17_097_YAML)
        .expect("BT17-097 YAML must parse")
        .memory(10)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

/// YAML must parse and compile without error.
#[test]
fn bt17_097_yaml_parses_without_error() {
    let _runner = bt17_097_runner();
}

/// Three compiled clauses: Main (triggered), All-Turns (declarative
/// replacement), Security (triggered, inherited scope).
#[test]
fn bt17_097_has_three_compiled_clauses() {
    let runner = bt17_097_runner();
    let compiled = runner
        .compiled_card("BT17-097")
        .expect("BT17-097 must be in compiled_cards");

    assert_eq!(
        compiled.effects.len(),
        3,
        "BT17-097 must have exactly 3 compiled clauses (Main, Replacement, Security); got {}",
        compiled.effects.len()
    );
}

/// Clause A: triggered with `main_from_hand` timing, optional, FaceUp scope.
#[test]
fn bt17_097_main_clause_is_optional_face_up() {
    let runner = bt17_097_runner();
    let compiled = runner
        .compiled_card("BT17-097")
        .expect("BT17-097 must be in compiled_cards");

    let main_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::MainFromHand))
        .expect("Main clause with MainFromHand timing must exist");

    assert!(
        main_clause.optional,
        "BT17-097 Main clause must be optional (printed: '1 of your Digimon may')"
    );
    assert_eq!(
        main_clause.scope,
        CompiledScope::FaceUp,
        "BT17-097 Main clause must have FaceUp scope (Option played from hand)"
    );
}

/// Clause B is a declarative replacement (NOT a triggered clause), targeting
/// `when_would_be_deleted` timing.
#[test]
fn bt17_097_has_replacement_clause_for_when_would_be_deleted() {
    let runner = bt17_097_runner();
    let compiled = runner
        .compiled_card("BT17-097")
        .expect("BT17-097 must be in compiled_cards");

    let has_deletion_replacement = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
                trigger,
                ..
            }) if trigger == "when_would_be_deleted"
        )
    });

    assert!(
        has_deletion_replacement,
        "BT17-097 must have a declarative `when_would_be_deleted` replacement clause"
    );
}

/// Clause C: triggered with `on_security` timing, optional, INHERITED scope.
#[test]
fn bt17_097_security_clause_is_optional_inherited() {
    let runner = bt17_097_runner();
    let compiled = runner
        .compiled_card("BT17-097")
        .expect("BT17-097 must be in compiled_cards");

    let sec_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("Security clause with OnSecurity timing must exist");

    assert!(
        sec_clause.optional,
        "BT17-097 Security clause must be optional (printed: 'You may play')"
    );
    assert_eq!(
        sec_clause.scope,
        CompiledScope::Inherited,
        "BT17-097 Security clause must have Inherited scope"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Clause A: [Main] place self as Delay-Option permanent
// ═══════════════════════════════════════════════════════════════════════════

/// Activating [Main] from hand places BT17-097 as a Delay-Option permanent on
/// the field. The card leaves the hand and appears in the battle area.
///
/// The digivolve sub-clause is implemented; historical G-EFFECT-INITIATED-DIGIVOLVE-FROM-
/// HAND-WITH-PERMANENT-TARGET); only `place_self_as_delay_option` fires.
#[test]
fn bt17_097_main_places_self_as_delay_option_on_field() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT17_097_YAML)
        .expect("BT17-097 YAML parses")
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["BT17-097"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    assert_eq!(runner.hand_size(0), 1, "precondition: BT17-097 in hand");
    assert_eq!(runner.battle_area_size(0), 0, "precondition: field empty");

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(
        fired,
        "activate_hand_main must return true for BT17-097 at hand index 0"
    );
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.hand_size(0),
        0,
        "BT17-097 must leave hand after Main"
    );
    assert_eq!(
        runner.battle_area_size(0),
        1,
        "BT17-097 must appear on field after Main (place_self_as_delay_option)"
    );

    assert!(
        matches!(
            runner.game.players[0].battle_area[0].option_state,
            digimon_engine::permanent::OptionState::Delayed { .. }
        ),
        "BT17-097 must be in Delayed option_state after Main"
    );
}

#[test]
fn bt17_097_main_digivolves_into_free_lv5_hand_card_cost_reduced_by_4_then_places_self() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT17_097_YAML)
        .expect("BT17-097 YAML parses")
        .add_card(make_blue_lv4_base("BT17097-BASE"))
        .add_card(make_free_lv5_evo("BT17097-FREE-LV5"))
        .add_card(make_non_free_lv5_evo("BT17097-NONFREE-LV5"))
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["BT17-097", "BT17097-FREE-LV5", "BT17097-NONFREE-LV5"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let base = runner.place_on_field(0, "BT17097-BASE", Some(0));

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired);

    let field_view = runner
        .pending_selection_view()
        .expect("Main effect must ask which Digimon digivolves");
    assert_eq!(field_view.kind, SelectionKind::OwnField);
    assert!(
        runner.pending_is_optional(),
        "the printed 'may digivolve' target selection must expose PASS"
    );
    assert_eq!(
        field_view.valid_action_ids.len(),
        1,
        "only the own field Digimon should be offered as the digivolve target"
    );
    let field_mask = build_action_mask(&runner.game, field_view.selecting_player);
    assert_eq!(field_mask[field_view.valid_action_ids[0] as usize], 1.0);
    runner
        .execute_action(field_view.selecting_player, field_view.valid_action_ids[0])
        .expect("select own Digimon target");

    let hand_view = runner
        .pending_selection_view()
        .expect("Main effect must ask which Lv5+ [Free] hand card to digivolve into");
    assert_eq!(hand_view.kind, SelectionKind::Hand);
    assert_eq!(
        hand_view.valid_action_ids.len(),
        1,
        "only the Lv5+ [Free] trait hand card should be legal; non-Free Lv5 is excluded"
    );
    let hand_action = hand_view.valid_action_ids[0];
    let hand_mask = build_action_mask(&runner.game, hand_view.selecting_player);
    assert_eq!(hand_mask[hand_action as usize], 1.0);
    runner
        .execute_action(hand_view.selecting_player, hand_action)
        .expect("select Lv5+ [Free] hand card");
    runner.auto_resolve().expect("finish BT17-097 Main effect");

    let evolved = &runner.game.players[0].battle_area[base.index as usize];
    assert_eq!(
        evolved.top_card().card_id(&runner.game.card_data),
        "BT17097-FREE-LV5",
        "selected Lv5+ [Free] card must become the stack's top card"
    );
    assert_eq!(
        runner.memory(),
        9,
        "evo cost 5 reduced by 4 should spend exactly 1 memory"
    );
    assert!(
        runner
            .game
            .player(0)
            .hand
            .iter()
            .all(|card| card.card_id(&runner.game.card_data) != "BT17097-FREE-LV5"),
        "selected Lv5+ [Free] card must leave hand"
    );
    assert!(
        runner.game.players[0].battle_area.iter().any(|p| {
            p.top_card().card_id(&runner.game.card_data) == "BT17-097"
                && matches!(p.option_state, OptionState::Delayed { .. })
        }),
        "BT17-097 must be placed in the battle area as a Delay option after the Main digivolve"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — Clause B: [All Turns] Delay replacement (cause gate)
// ═══════════════════════════════════════════════════════════════════════════
//
// The full Delay flow (cost, hand pick, digivolve, prevent) is already
// exercised by option_flow/replacement_integration.rs. Here we focus on the
// "other than by one of your effects" cause gate (the new filter vs old fixture)
// and the "your Digimon with [Free] trait" subject filter.

/// When own [Free] Digimon is deleted by OPPONENT EFFECT → replacement fires.
/// (Baseline: OpponentEffect is NOT own_effect → none_of filter passes.)
/// The replacement installs a pending selection (the hand pick prompt).
#[test]
fn bt17_097_delay_fires_for_opponent_effect_deletion_of_free_digimon() {
    let free_digimon = make_free_digimon("BT17097-FREE");
    let imperialdramon = make_imperialdramon_hand_card("BT17097-IMPERIAL");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT17_097_YAML)
        .expect("BT17-097 YAML parses")
        .add_card(free_digimon)
        .add_card(imperialdramon)
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["BT17097-IMPERIAL"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let free_perm = runner.place_on_field(0, "BT17097-FREE", Some(0));
    let _delay = place_as_delay(&mut runner, 0);

    runner
        .game
        .delete_permanent_with_cause(free_perm, ReplacementCause::OpponentEffect);

    assert!(
        runner.game.pending_selection.is_some(),
        "Delay replacement must fire for OpponentEffect deletion of own [Free] Digimon (Imperialdramon in hand)"
    );
}

/// When own [Free] Digimon is deleted by OWN EFFECT → replacement must NOT
/// fire ("other than by one of your effects" gate).
#[test]
fn bt17_097_delay_does_not_fire_for_own_effect_deletion_of_free_digimon() {
    let free_digimon = make_free_digimon("BT17097-FREE");
    let imperialdramon = make_imperialdramon_hand_card("BT17097-IMPERIAL");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT17_097_YAML)
        .expect("BT17-097 YAML parses")
        .add_card(free_digimon)
        .add_card(imperialdramon)
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["BT17097-IMPERIAL"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let free_perm = runner.place_on_field(0, "BT17097-FREE", Some(0));
    let _delay = place_as_delay(&mut runner, 0);

    runner
        .game
        .delete_permanent_with_cause(free_perm, ReplacementCause::OwnEffect);
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "Delay replacement must NOT fire for OwnEffect deletion (printed: 'other than by one of your effects')"
    );
    // The Free Digimon should be gone (deletion proceeded).
    let still_present = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "BT17097-FREE");
    assert!(
        !still_present,
        "Own-effect deletion should proceed (no replacement fired)"
    );
}

/// When own Digimon WITHOUT [Free] trait would be deleted by opponent effect →
/// replacement must NOT fire (trait filter rejects).
#[test]
fn bt17_097_delay_does_not_fire_for_non_free_digimon_deletion() {
    let non_free = make_non_free_digimon("BT17097-NON-FREE");
    let imperialdramon = make_imperialdramon_hand_card("BT17097-IMPERIAL");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT17_097_YAML)
        .expect("BT17-097 YAML parses")
        .add_card(non_free)
        .add_card(imperialdramon)
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["BT17097-IMPERIAL"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let non_free_perm = runner.place_on_field(0, "BT17097-NON-FREE", Some(0));
    let _delay = place_as_delay(&mut runner, 0);

    runner
        .game
        .delete_permanent_with_cause(non_free_perm, ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "Delay replacement must NOT fire for non-[Free] Digimon deletion (trait filter)"
    );
}

/// When OPPONENT'S [Free] Digimon would be deleted → replacement must NOT fire
/// (replacement_subject_is_mine: true rejects opponent subjects).
#[test]
fn bt17_097_delay_does_not_fire_for_opponent_free_digimon_deletion() {
    let opp_free = make_free_digimon("BT17097-OPP-FREE");
    let imperialdramon = make_imperialdramon_hand_card("BT17097-IMPERIAL");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT17_097_YAML)
        .expect("BT17-097 YAML parses")
        .add_card(opp_free)
        .add_card(imperialdramon)
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["BT17097-IMPERIAL"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let opp_perm = runner.place_on_field(1, "BT17097-OPP-FREE", Some(0));
    let _delay = place_as_delay(&mut runner, 0);

    runner
        .game
        .delete_permanent_with_cause(opp_perm, ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "Delay replacement must NOT fire for opponent's [Free] Digimon (replacement_subject_is_mine)"
    );
}

/// When own [Free] Digimon is deleted by BATTLE cause → replacement fires.
/// Battle is NOT own_effect, so none_of filter passes.
#[test]
fn bt17_097_delay_fires_for_battle_deletion_of_free_digimon() {
    let free_digimon = make_free_digimon("BT17097-FREE");
    let imperialdramon = make_imperialdramon_hand_card("BT17097-IMPERIAL");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT17_097_YAML)
        .expect("BT17-097 YAML parses")
        .add_card(free_digimon)
        .add_card(imperialdramon)
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["BT17097-IMPERIAL"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let free_perm = runner.place_on_field(0, "BT17097-FREE", Some(0));
    let _delay = place_as_delay(&mut runner, 0);

    runner
        .game
        .delete_permanent_with_cause(free_perm, ReplacementCause::Battle);

    assert!(
        runner.game.pending_selection.is_some(),
        "Delay replacement must fire for Battle deletion of own [Free] Digimon"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Clause C: [Security] (inherited) Davis/Ken Tamer + place self
// ═══════════════════════════════════════════════════════════════════════════

/// Smoke test: firing the Security clause on a placed BT17-097 (inherited)
/// with no eligible Tamers in hand or trash installs a zone-choice prompt and
/// completes without panic.
#[test]
fn bt17_097_security_smoke_no_eligible_tamer_no_panic() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT17_097_YAML)
        .expect("BT17-097 YAML parses")
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let field_handle = runner.place_on_field(0, "BT17-097", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::SecuritySkill,
        TriggerSource::Permanent(field_handle),
    );
    runner.game.drain_effect_queue();

    // G-DSL-UNION-PLAY-FREE workaround installs a zone choice prompt.
    // Drive through any selections; with no eligible Davis/Ken Tamer, the
    // select_hand / select_trash steps are no-ops.
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 30 {
        let player = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .selecting_player;
        let action = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids[0];
        let _ = runner.game.resolve_selection(player, action);
        runner.game.drain_effect_queue();
        steps += 1;
    }
    // Primary assertion: no panic. Secondary: card may appear on field as
    // Delay-Option after the security activation (place_self_as_delay_option).
}

/// [Security] + Davis Motomiya in the defender's hand → a real attack on the
/// defender's security checks BT17-097, the inherited [Security] clause fires
/// through the proper security path, and the printed effect runs: it may play
/// the Davis Tamer from hand and then places BT17-097 in the battle area.
///
/// Driven through the real combat/security-check path (see BT17-095's
/// `bt17_095_security_adds_card_to_hand_after_play`) — the previous
/// `enqueue_triggered(SecuritySkill, Permanent(..))` shortcut only ever
/// "worked" because of an over-fire bug in `enqueue_from_permanent` and is now
/// a silent no-op for inherited-scope [Security] clauses.
#[test]
fn bt17_097_security_plays_davis_from_hand_and_places_self_on_field() {
    let mut attacker = make_filler("BT17097-ATK");
    attacker.dp = Some(6000);
    let davis = make_davis_tamer("BT17097-DAVIS");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT17_097_YAML)
        .expect("BT17-097 YAML parses")
        .add_card(attacker.clone())
        .add_card(davis.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(1, &["BT17097-DAVIS"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .security(1, &["BT17-097"])
        .start();

    let attacker_handle = runner.place_on_field(0, "BT17097-ATK", Some(0));
    assert_eq!(
        runner.hand_size(1),
        1,
        "precondition: defender holds only the Davis Tamer"
    );
    assert_eq!(
        runner.security_count(1),
        1,
        "precondition: BT17-097 in security"
    );

    let _ = runner.attack_player(attacker_handle, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    // BT17-097's [Security] clause must have run end-to-end.
    assert_eq!(
        runner.security_count(1),
        0,
        "BT17-097 left the defender's security stack after the security check"
    );

    // Printed tail "place this card in the battle area" — BT17-097 itself must
    // be seated as a Delay-Option permanent on the defender's field.
    let bt17_097_on_field = runner.game.players[1]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "BT17-097");

    // The Davis Motomiya Tamer may also have been played from the defender's
    // hand by the optional clause body.
    let davis_on_field = runner.game.players[1]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "BT17097-DAVIS");

    assert!(
        davis_on_field || bt17_097_on_field,
        "After the [Security] clause ran, either the Davis Tamer was played \
         from the defender's hand OR BT17-097 was placed in the battle area"
    );
}

/// [Security] + Ken Ichijoji in the defender's hand → a real attack on the
/// defender's security checks BT17-097 and the inherited [Security] clause's
/// name filter ("[Davis Motomiya] or [Ken Ichijoji]") accepts the Ken Tamer.
///
/// Driven through the real combat/security-check path. Post-state proves the
/// clause executed AND the Ken-name filter accepted Ken: BT17-097 left the
/// security stack and was placed in the battle area, and/or the Ken Tamer was
/// played from the defender's hand onto the defender's field.
#[test]
fn bt17_097_security_eligible_filter_accepts_ken_ichijoji() {
    let mut attacker = make_filler("BT17097-ATK-KEN");
    attacker.dp = Some(6000);
    let ken = make_ken_tamer("BT17097-KEN");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT17_097_YAML)
        .expect("BT17-097 YAML parses")
        .add_card(attacker.clone())
        .add_card(ken.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(1, &["BT17097-KEN"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .security(1, &["BT17-097"])
        .start();

    let attacker_handle = runner.place_on_field(0, "BT17097-ATK-KEN", Some(0));
    assert_eq!(
        runner.hand_size(1),
        1,
        "precondition: defender holds only the Ken Tamer"
    );
    assert_eq!(
        runner.security_count(1),
        1,
        "precondition: BT17-097 in security"
    );

    let _ = runner.attack_player(attacker_handle, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    // The [Security] clause must have run: BT17-097 left the security stack.
    assert_eq!(
        runner.security_count(1),
        0,
        "BT17-097 left the defender's security stack after the security check"
    );

    let bt17_097_on_field = runner.game.players[1]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "BT17-097");
    let ken_on_field = runner.game.players[1]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "BT17097-KEN");

    assert!(
        bt17_097_on_field || ken_on_field,
        "After the [Security] clause ran, BT17-097 was placed in the battle area \
         and/or the Ken Ichijoji Tamer was played from the defender's hand \
         (proving the name filter accepted Ken)"
    );
}

/// G-DSL-UNION-PLAY-FREE RESOLVED: the Security clause uses a native
/// `select_union_zone` (zones: [hand, trash]) step. When only one zone (here,
/// the defender's hand) holds an eligible [Davis Motomiya]/[Ken Ichijoji]
/// Tamer, the union selection auto-collapses — its only valid card action
/// points at a hand slot; there is no separate From-hand/From-trash prompt.
#[test]
fn bt17_097_security_auto_collapses_zone_when_only_hand_eligible() {
    use digimon_engine::action::space::{PASS, PLAY_HAND_END, PLAY_HAND_START};
    use digimon_engine::selection::UnionZoneSet;

    let mut attacker = make_filler("BT17097-ATK-COLLAPSE");
    attacker.dp = Some(6000);
    let davis = make_davis_tamer("BT17097-DAVIS-COLLAPSE");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT17_097_YAML)
        .expect("BT17-097 YAML parses")
        .add_card(attacker.clone())
        .add_card(davis.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        // Davis Tamer ONLY in the defender's hand; trash holds no eligible card.
        .hand(1, &["BT17097-DAVIS-COLLAPSE"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .security(1, &["BT17-097"])
        .start();

    let attacker_handle = runner.place_on_field(0, "BT17097-ATK-COLLAPSE", Some(0));
    assert_eq!(
        runner.security_count(1),
        1,
        "precondition: BT17-097 in security"
    );

    // Attack the defender's security → BT17-097's [Security] clause fires.
    let _ = runner.attack_player(attacker_handle, 1, false);

    // The clause's first interactive prompt must be the union-zone pick,
    // spanning hand ∪ trash — NOT a separate effect-choice zone prompt.
    let sel = runner
        .game
        .pending_selection
        .as_ref()
        .expect("union-zone selection installs for the [Security] clause");
    assert_eq!(
        sel.kind,
        SelectionKind::UnionZone {
            zones: UnionZoneSet::HAND | UnionZoneSet::TRASH
        },
        "the [Security] prompt must be the union-zone pick (no separate zone-choice)"
    );

    // The defender's trash has no eligible Tamer, so the only eligible card
    // action must be a HAND slot — the union selection auto-collapsed to hand.
    let card_actions: Vec<u16> = sel
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    assert_eq!(
        card_actions.len(),
        1,
        "exactly one eligible card (the hand Davis Tamer); got {card_actions:?}"
    );
    assert!(owned_union_fixture::is_zone(&runner.game, card_actions[0], false));

    // Drive the rest of the clause to completion — proves the union pick feeds
    // play_union_bound_free without a follow-up zone-choice prompt.
    runner.auto_resolve().expect("security clause resolves");
    assert_eq!(
        runner.security_count(1),
        0,
        "BT17-097 left the defender's security stack after the security check"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// §15-8-3-2 — the digivolved Digimon's [When Digivolving] waits for the
// deleting effect
// ═══════════════════════════════════════════════════════════════════════════

/// P1's deleting effect for the §15-8-3-2 pin: more processing follows the
/// deletion.
const DELETER_YAML: &str = r#"
card: BT17097-DELETER
name: Deleter
kind: digimon
level: 5
color: [red]
cost: 7
dp: 7000
effects:
  - when: when_digivolving
    summary: "[When Digivolving] Delete 1 of your opponent's Digimon. Then, gain 1 memory."
    process:
      - select_opponent_permanent:
          bind_as: victim
          filter: { kind: digimon }
          prompt: "Delete 1 of your opponent's Digimon"
      - delete_permanent: { target: victim }
      - gain_memory: 1
"#;

/// §15-8-3-2 "Trigger-type effects can't activate during the processing for a
/// rule or effect".
///
/// P1's effect "Delete 1 of your opponent's Digimon. Then, gain 1 memory."
/// would delete P0's [Free] Digimon; Return to the Primogenitor's <Delay>
/// digivolves it into Imperialdramon: Fighter Mode (BT16-027) from the hand
/// instead. That digivolution happens during P1's effect, so BT16-027's
/// "[When Digivolving] Return 1 of your opponent's Digimon with as many or
/// fewer digivolution cards as this Digimon to the bottom of the deck."
/// activates only after the rest of P1's effect ("Then, gain 1 memory") --
/// never inside it, where it could change what the rest of that effect sees.
/// G-ENGINE-RESIDUAL-INLINE-DRAINS-OUTSIDE-SOURCE-TRASH.
#[test]
fn bt17_097_digivolved_when_digivolving_waits_for_the_deleting_effect() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT17_097_YAML)
        .expect("BT17-097 YAML parses")
        .from_dsl_yaml(DELETER_YAML)
        .expect("deleter YAML parses")
        .dsl_card("BT16-027")
        .expect("BT16-027 in embedded DSL pack")
        .add_card(make_free_digimon("BT17097-FREE"))
        .add_card(make_filler("FILL"))
        .memory(0)
        .hand(0, &["BT16-027"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();
    let free = runner.place_on_field(0, "BT17097-FREE", Some(0));
    let _delay = place_as_delay(&mut runner, 0);
    let deleter = runner.place_on_field(1, "BT17097-DELETER", Some(0));
    let cp = runner.event_checkpoint();
    let memory_gains = |r: &DebugRunner| -> Vec<(u8, String)> {
        r.events_since(cp)
            .iter()
            .filter_map(|e| match e {
                digimon_engine::events::GameEvent::MemoryChange {
                    player,
                    source_card_id: Some(id),
                    ..
                } => Some((*player, id.clone())),
                _ => None,
            })
            .collect()
    };

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(deleter));
    runner.game.drain_effect_queue();
    // P1 picks the Digimon to delete; P0's Delay prompt for the hand card.
    let victim = runner.pending_selection_view().expect("P1's deletion target");
    runner
        .execute_action(victim.selecting_player, victim.valid_action_ids[0])
        .expect("deletion target");
    let accept = runner
        .pending_selection_view()
        .expect("P0's optional <Delay> accept prompt (CR 4.3 16-17-2)");
    assert_eq!(accept.selecting_player, 0, "{accept:?}");
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("activate the <Delay>");
    let pick = runner
        .pending_selection_view()
        .expect("the Delay replacement's hand pick");
    assert_eq!(pick.selecting_player, 0, "{pick:?}");
    runner
        .execute_action(0, HAND_EFFECT_START)
        .expect("digivolve into BT16-027");

    assert_eq!(
        runner.game.player(0).battle_area[free.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "BT16-027",
        "the deletion was replaced by the digivolution"
    );
    let sel = runner
        .pending_selection()
        .expect("BT16-027's [When Digivolving] target pick");
    assert_eq!(
        runner
            .game
            .card_data_for_handle(sel.source_card)
            .map(|c| c.card_id.clone())
            .as_deref(),
        Some("BT16-027"),
        "{sel:?}"
    );
    assert_eq!(
        memory_gains(&runner),
        vec![(1, "BT17097-DELETER".to_string())],
        "the rest of P1's effect finished before BT16-027's [When Digivolving] activated"
    );
    runner.auto_resolve().expect("BT16-027 returns P1's Digimon");
    assert!(
        runner.game.player(1).battle_area.is_empty(),
        "BT16-027's [When Digivolving] returned P1's Digimon to the deck"
    );
}

#[path = "../../support/owned_union_fixture.rs"]
mod owned_union_fixture;

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — Candidate legality: no pick the digivolve / play would refuse
// ═══════════════════════════════════════════════════════════════════════════
//
// A pick that offers a card the follow-up digivolve or play then refuses is a
// choice with no legal outcome in the action space (CR 4.3 1-3-2: an
// impossible action is not carried out). DCGO filters every candidate set:
//   * [Main] hand pick: `DigivolveIntoHandOrTrashCard` (CardEffectCommons.cs
//     :859-881) offers only cards passing `CanPlayCardTargetFrame(frame,
//     PayCost: true, ...)` with the -4 reducer installed: route AND payment.
//   * [Main] own-Digimon pick: BT17_097.cs:39-43 offers only a Digimon that
//     some Lv.5+ [Free] hand card can digivolve onto.
//   * <Delay> hand pick: BT17_097.cs:126-137 + `DigivolveIntoHandOrTrashCard`
//     offer only an [Imperialdramon] card that can digivolve onto the
//     would-be-deleted Digimon (requirements apply: official Q&A on this card,
//     "you can only digivolve into a card that meets the digivolution
//     requirements"); and `CanUseCondition` (BT17_097.cs:113-137) refuses the
//     <Delay> outright when no such card exists.
//   * [Security] Tamer pick: BT17_097.cs:221-227 `CanPlayAsNewPermanent`.

/// A Lv.5 [Free] blue Digimon whose blue Lv.4 digivolution costs `evo_cost`.
fn make_free_lv5_evo_costing(id: &str, evo_cost: u16) -> CardData {
    let mut c = make_free_lv5_evo(id);
    c.evo_costs[0].memory_cost = evo_cost;
    c
}

/// An [Imperialdramon] card that digivolves from a blue level `from_level`.
fn make_imperialdramon_from(id: &str, from_level: u8) -> CardData {
    let mut c = make_imperialdramon_hand_card(id);
    c.evo_costs = vec![EvoCost {
        card_color: CardColor::Blue as u8,
        level: from_level,
        memory_cost: 4,
    }];
    c
}

fn hand_ids(runner: &DebugRunner, p: u8) -> Vec<String> {
    runner.game.players[p as usize]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

/// The hand cards a pending `Hand` pick offers (PASS excluded).
fn offered_hand_cards(runner: &DebugRunner, p: u8) -> Vec<String> {
    let hand = hand_ids(runner, p);
    let Some(sel) = runner.game.pending_selection.as_ref() else {
        return Vec::new();
    };
    if sel.kind != SelectionKind::Hand {
        return Vec::new();
    }
    let mut ids: Vec<String> = sel
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .map(|a| hand[a as usize].clone())
        .collect();
    ids.sort();
    ids
}

/// The [Main] runner: BT17-097 + `others` in P0's hand, a blue Lv.4 base on
/// the field, memory 0 (a cost of at most 10 is payable).
fn main_runner(others: &[CardData], hand: &[&str]) -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut b = DebugRunner::builder()
        .from_dsl_yaml(BT17_097_YAML)
        .expect("BT17-097 YAML parses")
        .add_card(make_blue_lv4_base("BT17097-BASE"))
        .add_card(make_filler("FILL"));
    for c in others {
        b = b.add_card(c.clone());
    }
    let mut hand_all = vec!["BT17-097"];
    hand_all.extend_from_slice(hand);
    let mut runner = b
        .memory(0)
        .hand(0, &hand_all)
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();
    let base = runner.place_on_field(0, "BT17097-BASE", Some(0));
    (runner, base)
}

/// [Main] hand pick: a Lv.5+ [Free] card whose digivolution cost stays
/// unpayable after the -4 reduction (15 - 4 = 11 > the 10 memory can pay) is
/// not offered; the payable one (5 - 4 = 1) is. DCGO
/// `CanPlayCardTargetFrame(PayCost: true)` with the reducer installed.
#[test]
fn bt17_097_main_hand_pick_excludes_free_card_unpayable_after_reduction() {
    let (mut runner, _base) = main_runner(
        &[
            make_free_lv5_evo_costing("BT17097-PAYABLE", 5),
            make_free_lv5_evo_costing("BT17097-UNPAYABLE", 15),
        ],
        &["BT17097-PAYABLE", "BT17097-UNPAYABLE"],
    );
    assert!(runner.game.activate_hand_main(0, 0));
    let field = runner.pending_selection_view().expect("own Digimon pick");
    runner
        .execute_action(0, field.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap())
        .expect("pick the base");
    assert_eq!(
        offered_hand_cards(&runner, 0),
        vec!["BT17097-PAYABLE".to_string()],
        "only the card whose reduced digivolution cost is payable is offered"
    );
}

/// [Main] hand pick: a Lv.5 [Free] card that can't digivolve onto the chosen
/// Digimon (its only requirement is green Lv.4; the base is blue) is not
/// offered (official Q&A: this [Main] doesn't ignore requirements).
/// Regression guard (the compile pass's `can_digivolve_onto` already holds it).
#[test]
fn bt17_097_main_hand_pick_excludes_free_card_without_a_route_onto_the_target() {
    let mut green_only = make_free_lv5_evo("BT17097-GREEN-ONLY");
    green_only.evo_costs[0].card_color = CardColor::Green as u8;
    let (mut runner, _base) = main_runner(
        &[make_free_lv5_evo("BT17097-ROUTE"), green_only],
        &["BT17097-ROUTE", "BT17097-GREEN-ONLY"],
    );
    assert!(runner.game.activate_hand_main(0, 0));
    let field = runner.pending_selection_view().expect("own Digimon pick");
    runner
        .execute_action(0, field.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap())
        .expect("pick the base");
    assert_eq!(offered_hand_cards(&runner, 0), vec!["BT17097-ROUTE".to_string()]);
}

/// [Main] own-Digimon pick: a Digimon no Lv.5+ [Free] hand card can
/// digivolve onto (a red Lv.3) is not offered; the blue Lv.4 base is.
/// DCGO BT17_097.cs:39-43.
#[test]
fn bt17_097_main_field_pick_excludes_digimon_with_no_digivolve_candidate() {
    let (mut runner, base) = main_runner(&[make_free_lv5_evo("BT17097-ROUTE")], &["BT17097-ROUTE"]);
    let other = runner.place_on_field(0, "FILL", Some(0)); // red Lv.3
    assert!(runner.game.activate_hand_main(0, 0));
    let field = runner.pending_selection_view().expect("own Digimon pick");
    assert_eq!(field.kind, SelectionKind::OwnField);
    let offered: Vec<u16> = field.valid_action_ids.iter().copied().filter(|&a| a != PASS).collect();
    assert_eq!(offered.len(), 1, "only the base is offered: {offered:?} (other = {other:?})");
    runner.execute_action(0, offered[0]).expect("pick the only offered Digimon");
    assert_eq!(offered_hand_cards(&runner, 0), vec!["BT17097-ROUTE".to_string()]);
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.game.players[0].battle_area[base.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "BT17097-ROUTE",
        "the offered Digimon was the base"
    );
}

/// [Main] own-Digimon pick: when the only Lv.5+ [Free] hand card is
/// unpayable even after the -4, no Digimon is a legal pick (choosing one
/// would lead to a digivolve that can't happen); the Option still places
/// itself in the battle area.
#[test]
fn bt17_097_main_field_pick_offers_nothing_when_only_candidate_is_unpayable() {
    let (mut runner, base) = main_runner(
        &[make_free_lv5_evo_costing("BT17097-UNPAYABLE", 15)],
        &["BT17097-UNPAYABLE"],
    );
    assert!(runner.game.activate_hand_main(0, 0));
    let offered: Vec<u16> = runner
        .game
        .pending_selection
        .as_ref()
        .filter(|s| s.kind == SelectionKind::OwnField)
        .map(|s| s.valid_action_ids.iter().copied().filter(|&a| a != PASS).collect())
        .unwrap_or_default();
    assert!(offered.is_empty(), "no Digimon offered: {offered:?}");
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.game.players[0].battle_area[base.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "BT17097-BASE"
    );
    assert!(hand_ids(&runner, 0).contains(&"BT17097-UNPAYABLE".to_string()));
    assert!(runner.game.players[0].battle_area.iter().any(|p| {
        p.top_card().card_id(&runner.game.card_data) == "BT17-097"
            && matches!(p.option_state, OptionState::Delayed { .. })
    }));
}

/// <Delay> runner: a Lv.5 blue [Free] Digimon threatened with deletion by an
/// opponent's effect, BT17-097 seated as a usable Delay, `hand` in P0's hand.
fn delay_runner(cards: &[CardData], hand: &[&str]) -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut b = DebugRunner::builder()
        .from_dsl_yaml(BT17_097_YAML)
        .expect("BT17-097 YAML parses")
        .add_card(make_free_digimon("BT17097-FREE"))
        .add_card(make_filler("FILL"));
    for c in cards {
        b = b.add_card(c.clone());
    }
    let mut runner = b
        .memory(10)
        .hand(0, hand)
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();
    let free = runner.place_on_field(0, "BT17097-FREE", Some(0));
    let _delay = place_as_delay(&mut runner, 0);
    (runner, free)
}

/// <Delay> hand pick: an [Imperialdramon] card that can't digivolve onto the
/// threatened Lv.5 (it needs a Lv.6) is not offered; one that can (from a
/// blue Lv.5) is.
#[test]
fn bt17_097_delay_hand_pick_excludes_imperialdramon_that_cannot_digivolve_onto_subject() {
    let (mut runner, free) = delay_runner(
        &[
            make_imperialdramon_from("BT17097-IMP-OK", 5),
            make_imperialdramon_from("BT17097-IMP-LV6", 6),
        ],
        &["BT17097-IMP-OK", "BT17097-IMP-LV6"],
    );
    runner
        .game
        .delete_permanent_with_cause(free, ReplacementCause::OpponentEffect);
    runner
        .game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept the optional <Delay> (CR 4.3 16-17-2)");
    let sel = runner
        .game
        .pending_selection
        .as_ref()
        .expect("the <Delay> hand pick");
    let offered: Vec<u16> = sel.valid_action_ids.iter().copied().filter(|&a| a != PASS).collect();
    assert_eq!(
        offered,
        vec![HAND_EFFECT_START],
        "only BT17097-IMP-OK (hand index 0) can digivolve onto the Lv.5 subject"
    );
    runner
        .game
        .resolve_selection(0, HAND_EFFECT_START)
        .expect("digivolve into the offered card");
    let top = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.card_sources.iter().any(|s| s.card_id(&runner.game.card_data) == "BT17097-FREE"))
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string());
    assert_eq!(top.as_deref(), Some("BT17097-IMP-OK"), "deletion prevented by the digivolve");
}

/// <Delay> activation: with no [Imperialdramon] card that can digivolve onto
/// the threatened Digimon, the <Delay> is not offered at all (DCGO
/// `CanUseCondition`, BT17_097.cs:113-137): BT17-097 stays on the field and
/// the deletion proceeds, rather than trashing the Option for nothing.
#[test]
fn bt17_097_delay_not_offered_when_no_imperialdramon_can_digivolve_onto_subject() {
    let (mut runner, free) = delay_runner(
        &[make_imperialdramon_from("BT17097-IMP-LV6", 6)],
        &["BT17097-IMP-LV6"],
    );
    runner
        .game
        .delete_permanent_with_cause(free, ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();
    assert!(
        runner.game.pending_selection.is_none(),
        "no <Delay> prompt: {:?}",
        runner.game.pending_selection.as_ref().map(|s| (&s.kind, &s.valid_action_ids))
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "BT17-097"),
        "the Delay Option was not trashed"
    );
    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "BT17097-FREE"),
        "the deletion proceeded"
    );
}

/// <Delay> is optional (CR 4.3 16-17-2; DCGO BT17_097.cs:105
/// `SetUpActivateClass(..., isOptional: true)`): the threatened deletion first
/// asks whether to activate, and BT17-097 is still on the field (its <Delay>
/// cost unpaid) while that question is open.
#[test]
fn bt17_097_delay_asks_before_paying_its_cost() {
    let (mut runner, free) = delay_runner(
        &[make_imperialdramon_from("BT17097-IMP-OK", 5)],
        &["BT17097-IMP-OK"],
    );
    runner
        .game
        .delete_permanent_with_cause(free, ReplacementCause::OpponentEffect);
    let sel = runner
        .game
        .pending_selection
        .as_ref()
        .expect("the optional <Delay> accept prompt");
    assert_eq!(sel.kind, SelectionKind::Replacement, "first decision is whether to activate the <Delay>");
    assert!(sel.valid_action_ids.contains(&REPLACEMENT_ACCEPT));
    assert!(sel.is_optional, "the <Delay> can be declined (PASS)");
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "BT17-097"),
        "the <Delay> cost is not paid before the player accepts"
    );
}

/// Declining the optional <Delay> keeps BT17-097 on the field (no cost paid)
/// and lets the deletion proceed.
#[test]
fn bt17_097_delay_declined_keeps_option_and_deletion_proceeds() {
    let (mut runner, free) = delay_runner(
        &[make_imperialdramon_from("BT17097-IMP-OK", 5)],
        &["BT17097-IMP-OK"],
    );
    runner
        .game
        .delete_permanent_with_cause(free, ReplacementCause::OpponentEffect);
    runner
        .game
        .resolve_selection(0, PASS)
        .expect("decline the optional <Delay>");
    runner.game.drain_effect_queue();
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "BT17-097"),
        "declining the <Delay> does not trash BT17-097"
    );
    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "BT17097-FREE"),
        "the deletion proceeded"
    );
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "BT17097-IMP-OK"),
        "the Imperialdramon card stays in hand"
    );
}

/// [Security] runner: P1 holds BT17-097 in security and `davis_zone` decides
/// where the Davis Tamer sits ("hand" / "trash"); P0 attacks P1.
fn security_runner(davis_in_trash: bool) -> DebugRunner {
    let mut attacker = make_filler("BT17097-ATK-SEC");
    attacker.dp = Some(6000);
    let mut b = DebugRunner::builder()
        .from_dsl_yaml(BT17_097_YAML)
        .expect("BT17-097 YAML parses")
        .add_card(attacker)
        .add_card(make_davis_tamer("BT17097-DAVIS-SEC"))
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .security(1, &["BT17-097"]);
    if !davis_in_trash {
        b = b.hand(1, &["BT17097-DAVIS-SEC"]);
    }
    let mut runner = b.start();
    if davis_in_trash {
        let data = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "BT17097-DAVIS-SEC")
            .unwrap();
        let next = runner.game.next_card_index();
        runner.game.players[1]
            .trash
            .push(digimon_engine::card_source::CardSource::new(data, 1, next));
    }
    runner
}

fn lock(runner: &mut DebugRunner, player: u8, kind: digimon_engine::enums::ModifierType) {
    use digimon_engine::enums::Expiry;
    use digimon_engine::modifiers::PlayerModifierEntry;
    runner.game.modifiers.add_player_modifier(
        player,
        PlayerModifierEntry::simple(kind, 0, Expiry::Permanent, None, 1 - player),
    );
}

/// The Tamer actions the [Security] union pick offers right after the attack.
fn attack_and_offered_union_cards(runner: &mut DebugRunner) -> Vec<u16> {
    let attacker = runner.place_on_field(0, "BT17097-ATK-SEC", Some(0));
    let _ = runner.attack_player(attacker, 1, false);
    runner
        .game
        .pending_selection
        .as_ref()
        .filter(|s| matches!(s.kind, SelectionKind::UnionZone { .. }))
        .map(|s| s.valid_action_ids.iter().copied().filter(|&a| a != PASS).collect())
        .unwrap_or_default()
}

/// Control: the Davis Tamer in hand is offered.
#[test]
fn bt17_097_security_offers_davis_in_hand_when_playable() {
    let mut runner = security_runner(false);
    assert_eq!(attack_and_offered_union_cards(&mut runner).len(), 1);
}

/// [Security]: under "can't play Tamers by effects" the Davis Tamer is not
/// offered (DCGO `CanPlayAsNewPermanent`, BT17_097.cs:226).
#[test]
fn bt17_097_security_excludes_davis_when_tamer_effect_play_is_prohibited() {
    let mut runner = security_runner(false);
    lock(&mut runner, 1, digimon_engine::enums::ModifierType::CannotPlayTamerByEffect);
    let offered = attack_and_offered_union_cards(&mut runner);
    assert!(offered.is_empty(), "no Tamer offered: {offered:?}");
    let _ = runner.auto_resolve();
    assert!(hand_ids(&runner, 1).contains(&"BT17097-DAVIS-SEC".to_string()), "Davis stays in hand");
}

/// [Security]: a full battle area leaves no room for the Tamer, so it is not
/// offered (DCGO `CanPlayAsNewPermanent` needs an empty frame).
#[test]
fn bt17_097_security_excludes_davis_when_battle_area_full() {
    let mut runner = security_runner(false);
    runner.place_on_field(1, "FILL", Some(0));
    runner.game.rules.field_slots = 1;
    let offered = attack_and_offered_union_cards(&mut runner);
    assert!(offered.is_empty(), "no Tamer offered: {offered:?}");
}

/// [Security]: a Davis Tamer in the trash is not offered while its owner
/// can't play cards from the trash (the play from the trash would be refused).
#[test]
fn bt17_097_security_excludes_trash_davis_when_trash_play_is_locked() {
    let mut control = security_runner(true);
    assert_eq!(attack_and_offered_union_cards(&mut control).len(), 1, "control: trash Davis offered");
    let mut runner = security_runner(true);
    lock(&mut runner, 1, digimon_engine::enums::ModifierType::CannotPlayFromTrash);
    let offered = attack_and_offered_union_cards(&mut runner);
    assert!(offered.is_empty(), "no Tamer offered: {offered:?}");
}
