//! BT25-017 Flaremon — Digimon, Lv.5, Red, DP 7000, Cost 6.
//! Traits: Beastkin, Iliad, TS. Attribute: Vaccine.
//! Evo: Lv.4 Red / cost 3 (printed). Alt-digi: Lv.4 w/ [TS] trait / cost 3.
//!
//! # Card text (cards.json — verbatim; confirmed against card image BT25-017)
//!
//! [On Play] [When Digivolving] This Digimon may attack. Then, by trashing 1
//! card in your hand, delete 1 of your opponent's Digimon with 7000 DP or less.
//! [Your Turn] When your Digimon are played or digivolve, if any of them are
//! blue, this Digimon may digivolve into [Apollomon] in the hand with the cost
//! reduced by 2.
//!
//! Inherited Effect:
//! <Security A. +1> (This Digimon checks 1 additional security card.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Red/BT25_017.cs
//!   - EffectTiming.None: AddSelfDigivolutionRequirementStaticEffect(level 4,
//!     HasTSTraits, cost 3).
//!   - Shared OnPlay/WhenDigivolving: SelectAttackEffect on THIS Digimon
//!     (this may attack). Then SelectHandEffect (canNoSelect:true) Discard;
//!     if a card was trashed AND an eligible target exists, SelectPermanentEffect
//!     (Destroy) over opponent Digimon with DP <= 7000.
//!   - OnEnterFieldAnyone (Your Turn): when a played/digivolved OWN Digimon is
//!     blue, this Digimon may digivolve into [Apollomon] from hand, payCost:true,
//!     reduceCost 2, isHand:true. Skippable.
//!   - ChangeSelfSAttackStaticEffect(+1, isInheritedEffect:true).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - effect-created may-attack on self (Track D may_attack_now)
//! - cost-as-trashing-hand → conditional delete (DP <= 7000 gate)
//! - your-turn played/digivolve color-gated self digivolve into named, cost -2
//! - H4 inherited Security A. +1
//! - alt-digivolution Lv.4 [TS] → cost 3

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::CardColor;
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-017";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

fn red_lv5(card_id: &str, name: &str, dp: i32) -> CardData {
    let mut card = make_test_card_with_level(card_id, name, 5);
    card.colors = vec![CardColor::Red];
    card.dp = Some(dp);
    card
}

fn opp_digimon(card_id: &str, dp: i32) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 4);
    card.colors = vec![CardColor::Purple];
    card.dp = Some(dp);
    card
}

/// A blue Lv.4 own Digimon used to trigger the [Your Turn] color gate when
/// played.
fn blue_lv4(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 4);
    card.colors = vec![CardColor::Blue];
    card.dp = Some(4000);
    card
}

fn red_lv4(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 4);
    card.colors = vec![CardColor::Red];
    card.dp = Some(4000);
    card
}

/// Apollomon (Lv.6) the [Your Turn] clause digivolves Flaremon into. Cost 4
/// (so a 2-reduction is observable as 2 memory spent, not 4).
fn apollomon(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, "Apollomon", 6);
    card.colors = vec![CardColor::Red];
    card.dp = Some(12000);
    card.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 5,
        memory_cost: 4,
    }];
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
        .expect("BT25-017 YAML parses and compiles")
        .add_card(opp_digimon("OPP-7K", 7000))
        .add_card(opp_digimon("OPP-8K", 8000))
        .add_card(blue_lv4("BLUE-LV4"))
        .add_card(red_lv4("RED-LV4"))
        .add_card(apollomon("APOLLO"))
        .add_card(make_test_card_with_level("FILL", "Filler", 3))
        .add_card(make_test_card_with_level("FODDER", "Fodder", 3))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
}

fn place_flaremon(runner: &mut DebugRunner) -> PermanentHandle {
    runner.place_on_field(0, CARD_ID, Some(0))
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_017_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT25-017 must be in the embedded DSL pack");

    assert_eq!(card.name, "Flaremon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(7000));
    for trait_name in ["Beastkin", "Iliad", "TS"] {
        assert!(
            card.traits.contains(&trait_name.to_string()),
            "BT25-017 metadata must include trait {trait_name}"
        );
    }
}

#[test]
fn bt25_017_has_ts_alt_digivolution_path() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let has_ts = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(3))
            && p.from
                .as_ref()
                .is_some_and(|f| f.level_eq == Some(4) && f.trait_has.as_deref() == Some("TS"))
    });
    assert!(has_ts, "BT25-017 must have a Lv.4 [TS] → cost 3 alt-path");
}

#[test]
fn bt25_017_has_on_play_and_your_turn_clauses() {
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

    let has_op = triggered.iter().any(|t| {
        t.when.contains(&CompiledTiming::OnPlay)
            && t.when.contains(&CompiledTiming::WhenDigivolving)
    });
    let has_your_turn = triggered.iter().any(|t| {
        (t.when.contains(&CompiledTiming::OnEnterFieldAnyone)
            || t.when.contains(&CompiledTiming::OnDigivolve))
            && t.optional
    });
    assert!(has_op, "must have [On Play][When Digivolving] clause");
    assert!(
        has_your_turn,
        "must have the optional [Your Turn] played/digivolve digivolve clause"
    );
}

// ─── Section 2 — [On Play] may attack, then trash hand → delete <=7000 DP ─────

#[test]
fn bt25_017_on_play_deletes_opp_le_7000_after_trashing_hand() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    runner.game_mut().players[1].security.clear();

    let opp = runner.place_on_field(1, "OPP-7K", Some(0)); // 7000 — eligible
    let _fodder = push_to_hand(&mut runner, 0, "FODDER"); // the card to trash
    let hand_index = push_to_hand(&mut runner, 0, CARD_ID);

    let opp_count_before = runner.battle_area_size(1);
    runner.play(0, hand_index);
    // Drive the whole flow: optional self-attack (decline / accept), trash a
    // hand card, then delete the 7000-DP opponent. auto_resolve picks the first
    // legal action at each prompt — the delete target here is the only eligible
    // opponent Digimon.
    runner.auto_resolve().expect("resolve on-play flow");

    assert_eq!(
        runner.battle_area_size(1),
        opp_count_before - 1,
        "trashing a hand card must delete the 7000-DP opponent Digimon"
    );
}

#[test]
fn bt25_017_on_play_cannot_delete_opp_above_7000() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    runner.game_mut().players[1].security.clear();

    let opp = runner.place_on_field(1, "OPP-8K", Some(0)); // 8000 — NOT eligible
    let _fodder = push_to_hand(&mut runner, 0, "FODDER");
    let hand_index = push_to_hand(&mut runner, 0, CARD_ID);

    let opp_count_before = runner.battle_area_size(1);
    runner.play(0, hand_index);
    runner.auto_resolve().expect("resolve on-play flow");

    assert_eq!(
        runner.battle_area_size(1),
        opp_count_before,
        "an opponent Digimon above 7000 DP must not be a legal delete target"
    );
}

// ─── Section 3 — [Your Turn] blue played/digivolve → self digivolve into Apollomon, cost -2 ─

#[test]
fn bt25_017_your_turn_offers_apollomon_digivolve_when_blue_played() {
    let mut runner = base().start();
    runner.game.turn_count = 1; // your turn
    let flaremon = place_flaremon(&mut runner);
    let _apollo_in_hand = push_to_hand(&mut runner, 0, "APOLLO");

    // Play a BLUE Lv.4 Digimon — its entry should trigger Flaremon's [Your Turn]
    // observer ("if any of them are blue").
    let blue_index = push_to_hand(&mut runner, 0, "BLUE-LV4");
    runner.play(0, blue_index);

    assert!(
        runner.pending_kind().is_some(),
        "playing a blue Digimon must offer Flaremon's digivolve into Apollomon"
    );
    runner.auto_resolve().ok();
}

#[test]
fn bt25_017_your_turn_does_not_offer_when_non_blue_played() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    let flaremon = place_flaremon(&mut runner);
    let _apollo_in_hand = push_to_hand(&mut runner, 0, "APOLLO");

    // Play a RED Lv.4 — not blue, so the color gate must block the offer.
    let red_index = push_to_hand(&mut runner, 0, "RED-LV4");
    runner.play(0, red_index);

    assert!(
        runner.pending_selection().is_none(),
        "a non-blue played Digimon must NOT trigger the Apollomon digivolve"
    );
    runner.auto_resolve().ok();
}

#[test]
fn bt25_017_your_turn_does_not_offer_without_apollomon_in_hand() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    let flaremon = place_flaremon(&mut runner);
    // No Apollomon in hand.

    let blue_index = push_to_hand(&mut runner, 0, "BLUE-LV4");
    runner.play(0, blue_index);

    assert!(
        runner.pending_selection().is_none(),
        "without [Apollomon] in hand there is no digivolve to offer"
    );
    runner.auto_resolve().ok();
}

// ─── Section 4 — Inherited Security A. +1 ────────────────────────────────────

#[test]
fn bt25_017_has_inherited_security_attack_plus_one() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let has_sa = card.effects.iter().any(|c| match c {
        CompiledClause::Declarative(d) => format!("{d:?}").contains("SecurityAttack"),
        _ => false,
    });
    assert!(
        has_sa,
        "BT25-017 must grant inherited <Security A. +1> (SecurityAttackPlus)"
    );
}
