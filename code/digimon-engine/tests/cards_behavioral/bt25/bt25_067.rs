//! BT25-067 Sealsdramon — Digimon, Lv.4, Black/Purple, DP 5000, Cost 4.
//! Traits: Cyborg, D-Brigade, ACCEL.  Attribute: Virus.
//! Evo: Lv.3 Black / cost 3 (printed evo_costs).
//!
//! # Card text (cards.json — verbatim)
//!
//! [Your Turn] When any of your [D-Brigade] or [ACCEL] trait Digimon are
//! played, this Digimon may digivolve into a [D-Brigade] or [ACCEL] trait
//! Digimon card in the hand with the cost reduced by 2.
//!
//! Inherited Effect:
//! [All Turns] This Digimon gets +1000 DP.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Black/BT25_067.cs
//!   - EffectTiming.None: AddSelfDigivolutionRequirementStaticEffect(level 3,
//!     TopCard.EqualsTraits("D-Brigade") || EqualsTraits("ACCEL"), cost 2).
//!     (Printed alt-digivolve box "Lv.3 w/[D-Brigade]/[ACCEL]: Cost 2".)
//!   - OnEnterFieldAnyone (Your Turn): CanTriggerOnPermanentPlay where the
//!     played OWN permanent has the D-Brigade or ACCEL trait → this Digimon
//!     MAY digivolve into a D-Brigade/ACCEL Digimon card in HAND (isHand:true,
//!     payCost:true, reduceCost 2). Skippable (optional).
//!   - EffectTiming.None: ChangeSelfDPStaticEffect(+1000, isInheritedEffect:true).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - A5-adjacent: your-turn play-triggered self-digivolve into a HAND card, cost -2
//!   (effect_initiated_digivolve { target: source, from_hand: <hand binding> })
//! - D4 inherited declarative +1000 DP aura (all turns)
//! - alt-digivolution Lv.3 [D-Brigade]/[ACCEL] → cost 2

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::CardColor;
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-067";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

/// A D-Brigade Lv.4 Digimon used as the played trigger AND as a hand digivolve
/// target. Cost 5 so a 2-reduction is observable.
fn dbrigade_lv4(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 4);
    card.colors = vec![CardColor::Black];
    card.dp = Some(5000);
    card.traits = vec!["D-Brigade".to_string()];
    card.play_cost = 5;
    card.evo_costs = vec![EvoCost {
        card_color: 5,
        level: 4,
        memory_cost: 4,
    }];
    card
}

/// A plain Lv.4 with NO D-Brigade / ACCEL trait — must NOT trigger the clause.
fn plain_lv4(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 4);
    card.colors = vec![CardColor::Black];
    card.dp = Some(5000);
    card.traits = vec!["Cyborg".to_string()];
    card.play_cost = 4;
    card
}

fn push_to_hand(runner: &mut DebugRunner, player: u8, card_id: &str) -> usize {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card id {card_id}"));
    let instance_id = runner.game.next_card_index();
    runner.game.players[player as usize]
        .hand
        .push(CardSource::new(data_index, player, instance_id));
    runner.game.players[player as usize].hand.len() - 1
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-067 YAML parses and compiles")
        .add_card(dbrigade_lv4("DB-LV4"))
        .add_card(plain_lv4("PLAIN-LV4"))
        .add_card(make_test_card_with_level("FILL", "Filler", 3))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
}

fn place_sealsdramon(runner: &mut DebugRunner) -> PermanentHandle {
    runner.place_on_field(0, CARD_ID, Some(0))
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_067_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT25-067 must be in the embedded DSL pack");

    assert_eq!(card.name, "Sealsdramon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(5000));
    for trait_name in ["Cyborg", "D-Brigade", "ACCEL"] {
        assert!(
            card.traits.contains(&trait_name.to_string()),
            "BT25-067 metadata must include trait {trait_name}"
        );
    }
}

#[test]
fn bt25_067_has_dbrigade_accel_alt_digivolution_path() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let has_alt = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(2))
    });
    assert!(
        has_alt,
        "BT25-067 must have a Lv.3 [D-Brigade]/[ACCEL] → cost 2 alt-path"
    );
}

#[test]
fn bt25_067_has_optional_your_turn_play_triggered_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let has_your_turn = triggered.iter().any(|t| {
        (t.when.contains(&CompiledTiming::OnAllyPlayed)
            || t.when.contains(&CompiledTiming::OnEnterFieldAnyone))
            && t.optional
    });
    assert!(
        has_your_turn,
        "must have the optional [Your Turn] play-triggered self-digivolve clause"
    );
}

#[test]
fn bt25_067_has_inherited_dp_buff() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let has_inherited_aura = card.effects.iter().any(|c| match c {
        CompiledClause::Declarative(d) => {
            let s = format!("{d:?}");
            s.contains("Aura") && s.contains("1000")
        }
        _ => false,
    });
    assert!(
        has_inherited_aura,
        "BT25-067 must grant inherited +1000 DP aura"
    );
}

// ─── Section 2 — [Your Turn] play-triggered digivolve gating ─────────────────

#[test]
fn bt25_067_offers_digivolve_when_dbrigade_played_with_hand_target() {
    let mut runner = base().start();
    runner.game.turn_count = 1; // your turn
    let sealsdramon = place_sealsdramon(&mut runner);
    // A D-Brigade Digimon in hand is the digivolve target.
    let _evo = push_to_hand(&mut runner, 0, "DB-LV4");

    // Play a D-Brigade Lv.4 — triggers the [Your Turn] observer.
    let played = push_to_hand(&mut runner, 0, "DB-LV4");
    runner.play(0, played);

    assert!(
        runner.pending_kind().is_some(),
        "playing a D-Brigade Digimon must offer Sealsdramon's digivolve into a hand D-Brigade/ACCEL card"
    );
    runner.auto_resolve().ok();
}

#[test]
fn bt25_067_does_not_offer_when_non_dbrigade_non_accel_played() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    let sealsdramon = place_sealsdramon(&mut runner);
    let _evo = push_to_hand(&mut runner, 0, "DB-LV4");

    // Play a plain Cyborg (no D-Brigade / ACCEL) — gate must block.
    let played = push_to_hand(&mut runner, 0, "PLAIN-LV4");
    runner.play(0, played);

    assert!(
        runner.pending_selection().is_none(),
        "a non-D-Brigade/ACCEL play must NOT trigger the digivolve"
    );
    runner.auto_resolve().ok();
}

#[test]
fn bt25_067_does_not_offer_without_hand_digivolve_target() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    let sealsdramon = place_sealsdramon(&mut runner);
    // No D-Brigade/ACCEL card in hand to digivolve into (only the played one,
    // which leaves the hand on play).

    let played = push_to_hand(&mut runner, 0, "DB-LV4");
    runner.play(0, played);

    assert!(
        runner.pending_selection().is_none(),
        "without a hand D-Brigade/ACCEL digivolve target there is nothing to offer"
    );
    runner.auto_resolve().ok();
}

#[test]
fn bt25_067_does_not_offer_on_opponents_turn() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    let sealsdramon = place_sealsdramon(&mut runner);
    let _evo = push_to_hand(&mut runner, 0, "DB-LV4");

    // Switch to the opponent's turn — Sealsdramon's [Your Turn] gate is now closed.
    runner.end_turn();
    assert_ne!(
        runner.game.turn_player(),
        0,
        "precondition: it must be the opponent's turn"
    );

    // The opponent (P1) plays a D-Brigade Digimon. Sealsdramon belongs to P0 and
    // its clause is [Your Turn] (P0's turn), so it must NOT fire here.
    let played = push_to_hand(&mut runner, 1, "DB-LV4");
    runner.play(1, played);

    assert!(
        runner.pending_selection().is_none(),
        "[Your Turn] clause must not fire on the opponent's turn"
    );
    runner.auto_resolve().ok();
}

// ─── Section 3 — inherited [All Turns] +1000 DP behavioral check ─────────────
//
// The inherited "[All Turns] This Digimon gets +1000 DP" is a declarative
// self-aura: it applies only when Sealsdramon is a digivolution SOURCE under a
// carrier (DCGO ChangeSelfDPStaticEffect isInheritedEffect:true). It contributes
// +1000 to the carrier on BOTH turns ([All Turns], no your_turn gate).

/// Lv.5 carrier hosting Sealsdramon as a digivolution source.
fn carrier_lv5(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 5);
    card.colors = vec![CardColor::Black];
    card.dp = Some(7000);
    card
}

#[test]
fn bt25_067_inherited_aura_grants_1000_dp_on_your_turn() {
    let mut runner = base().add_card(carrier_lv5("CARRIER-LV5")).start();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER-LV5"]);
    assert_eq!(
        runner.game.turn_player(),
        0,
        "precondition: P0 is turn player"
    );

    let contribution = runner.game.source_dp_contribution(carrier, 0);
    assert_eq!(
        contribution, 1000,
        "Sealsdramon inherited [All Turns] +1000 DP must contribute on your turn"
    );
}

#[test]
fn bt25_067_inherited_aura_grants_1000_dp_on_opponents_turn() {
    let mut runner = base().add_card(carrier_lv5("CARRIER-LV5")).start();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER-LV5"]);
    runner.end_turn();
    assert_ne!(
        runner.game.turn_player(),
        0,
        "precondition: P0 not turn player"
    );

    let contribution = runner.game.source_dp_contribution(carrier, 0);
    assert_eq!(
        contribution, 1000,
        "Sealsdramon inherited [All Turns] +1000 DP must ALSO contribute on the opponent's turn"
    );
}
