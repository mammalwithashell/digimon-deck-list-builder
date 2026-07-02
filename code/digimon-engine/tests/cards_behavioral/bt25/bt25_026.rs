//! BT25-026 Crescemon — Digimon, Lv.5, Blue, DP 7000, Cost 6.
//! Traits: Wizard, Iliad, TS.  Attribute: Data.
//! Evo: Lv.4 Blue / cost 3 (printed evo_costs).
//!
//! # Card text (card image BT25-026 — authoritative for printed text)
//!
//! [On Play] [When Digivolving] Trash the bottom 3 digivolution cards of 1 of
//!   your opponent's Digimon. Then, 1 of their Digimon with no digivolution
//!   cards can't suspend until their turn ends.
//! [Your Turn] When your Digimon are played or digivolve, if any of them are
//!   red, this Digimon may digivolve into [Dianamon] in the trash with the cost
//!   reduced by 2.
//!
//! Inherited Effect:
//! [Your Turn] This Digimon's attack target can't change.
//!   (NOTE: cards.json says inherited = "<Security A. +1>" — that is WRONG. The
//!    printed card FACE and DCGO C# both say "attack target can't change"
//!    (CanNotSwitchAttackTargetClass). Image wins per CLAUDE.md source priority.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Blue/BT25_026.cs
//!   - None: AddSelfDigivolutionRequirementStaticEffect(level 4, HasTSTraits, cost 3).
//!   - Shared OnPlay/WhenDigivolving (mandatory): select opp Digimon → trash
//!     bottom 3 digivolution cards; then select opp Digimon with 0 sources →
//!     GainCanNotSuspend until end of their (opp's) turn.
//!   - OnEnterFieldAnyone (Your Turn): own red played/digivolved → may digivolve
//!     into [Dianamon] in TRASH, reduceCost 2.
//!   - None (inherited): CanNotSwitchAttackTargetClass on self ([Your Turn]).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - F: [On Play][When Digivolving] trash_bottom_sources(3) + CannotSuspend lock
//! - A5: your-turn red played/digivolve → self digivolve into TRASH card, cost -2
//! - inherited self-aura CanNotSwitchAttackTarget ([Your Turn])
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

const CARD_ID: &str = "BT25-026";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

fn red_lv4(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 4);
    card.colors = vec![CardColor::Red];
    card.dp = Some(5000);
    card
}

fn blue_lv4(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 4);
    card.colors = vec![CardColor::Blue];
    card.dp = Some(5000);
    card
}

fn opp_lv4(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 4);
    card.colors = vec![CardColor::Blue];
    card.dp = Some(6000);
    card
}

/// Dianamon (Lv.6) — the [Your Turn] clause digivolves Crescemon into it FROM TRASH.
fn dianamon(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, "Dianamon", 6);
    card.colors = vec![CardColor::Blue];
    card.dp = Some(11000);
    card.evo_costs = vec![EvoCost {
        card_color: 1,
        level: 5,
        memory_cost: 5,
    }];
    card
}

fn push_to_hand(runner: &mut DebugRunner, player: u8, card_id: &str) -> usize {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card id {card_id}"));
    let instance_id = runner.game.next_card_index();
    runner.game.players[player as usize]
        .hand
        .push(CardSource::new(data_index, player, instance_id));
    runner.game.players[player as usize].hand.len() - 1
}

fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card id {card_id}"));
    let instance_id = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(data_index, player, instance_id));
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-026 YAML parses and compiles")
        .add_card(red_lv4("RED-LV4"))
        .add_card(blue_lv4("BLUE-LV4"))
        .add_card(opp_lv4("OPP-LV4"))
        .add_card(dianamon("DIANA"))
        .add_card(make_test_card_with_level("FILL", "Filler", 3))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(12)
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_026_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT25-026 must be in the embedded DSL pack");

    assert_eq!(card.name, "Crescemon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(7000));
    for trait_name in ["Wizard", "Iliad", "TS"] {
        assert!(
            card.traits.contains(&trait_name.to_string()),
            "BT25-026 metadata must include trait {trait_name}"
        );
    }
}

#[test]
fn bt25_026_has_ts_alt_digivolution_path() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let has_ts = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(3))
            && p.from
                .as_ref()
                .is_some_and(|f| f.level_eq == Some(4) && f.trait_has.as_deref() == Some("TS"))
    });
    assert!(has_ts, "BT25-026 must have a Lv.4 [TS] → cost 3 alt-path");
}

#[test]
fn bt25_026_has_op_wd_and_your_turn_clauses() {
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

    let has_op_wd = triggered.iter().any(|t| {
        t.when.contains(&CompiledTiming::OnPlay)
            && t.when.contains(&CompiledTiming::WhenDigivolving)
    });
    let has_your_turn = triggered.iter().any(|t| {
        (t.when.contains(&CompiledTiming::OnEnterFieldAnyone)
            || t.when.contains(&CompiledTiming::OnDigivolve))
            && t.optional
    });
    assert!(
        has_op_wd,
        "must have [On Play][When Digivolving] sources/lock clause"
    );
    assert!(
        has_your_turn,
        "must have the optional [Your Turn] red played/digivolve → Dianamon digivolve clause"
    );
}

#[test]
fn bt25_026_has_inherited_cannot_switch_attack_target_aura() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let has_inherited_aura = card.effects.iter().any(|c| match c {
        CompiledClause::Declarative(d) => {
            let s = format!("{d:?}");
            // Inherited aura granting CanNotSwitchAttackTarget (NOT Security A.+1).
            s.contains("SwitchAttackTarget")
        }
        _ => false,
    });
    assert!(
        has_inherited_aura,
        "BT25-026 inherited effect must be CanNotSwitchAttackTarget (image-authoritative), not Security A.+1"
    );
}

// ─── Section 2 — [On Play][When Digivolving] trash bottom 3 + lock ───────────

#[test]
fn bt25_026_on_play_offers_source_trash_against_opp_digimon() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    // Opponent Digimon present (has 0 visible sources via place_on_field, but is
    // a valid target for the bottom-3 trash pick).
    let _opp = runner.place_on_field(1, "OPP-LV4", Some(0));
    let cres = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, cres.index as usize);

    assert!(
        runner.pending_kind().is_some(),
        "OnPlay must offer a pick of an opponent Digimon to trash bottom 3 sources from"
    );
    runner.auto_resolve().ok();
}

#[test]
fn bt25_026_on_play_does_nothing_without_opp_digimon() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    let cres = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, cres.index as usize);

    assert!(
        runner.pending_selection().is_none(),
        "with no opponent Digimon there is no source-trash target — clause no-ops"
    );
    runner.auto_resolve().ok();
}

// ─── Section 3 — [Your Turn] red played → digivolve into Dianamon from trash ─

#[test]
fn bt25_026_your_turn_offers_dianamon_from_trash_when_red_played() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    let cres = runner.place_on_field(0, CARD_ID, Some(0));
    push_to_trash(&mut runner, 0, "DIANA"); // Dianamon waiting in trash

    let red = push_to_hand(&mut runner, 0, "RED-LV4");
    runner.play(0, red);

    assert!(
        runner.pending_kind().is_some(),
        "playing a red Digimon must offer Crescemon's digivolve into Dianamon (from trash)"
    );
    runner.auto_resolve().ok();
}

#[test]
fn bt25_026_your_turn_does_not_offer_when_non_red_played() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    let cres = runner.place_on_field(0, CARD_ID, Some(0));
    push_to_trash(&mut runner, 0, "DIANA");

    let blue = push_to_hand(&mut runner, 0, "BLUE-LV4");
    runner.play(0, blue);

    assert!(
        runner.pending_selection().is_none(),
        "a non-red played Digimon must NOT trigger the Dianamon digivolve"
    );
    runner.auto_resolve().ok();
}

#[test]
fn bt25_026_your_turn_does_not_offer_without_dianamon_in_trash() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    let cres = runner.place_on_field(0, CARD_ID, Some(0));
    // No Dianamon in trash.

    let red = push_to_hand(&mut runner, 0, "RED-LV4");
    runner.play(0, red);

    assert!(
        runner.pending_selection().is_none(),
        "without [Dianamon] in the trash there is no digivolve to offer"
    );
    runner.auto_resolve().ok();
}
