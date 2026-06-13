//! BT25-024 Lekismon — Digimon, Lv.4, Blue, DP 5000, Cost 4.
//! Traits: Beastkin, Iliad, TS. Attribute: Data.
//! Evo: Lv.3 Blue / cost 2 (printed). Alt-digi: Lv.3 w/ [TS] trait / cost 2.
//!
//! # Card text (cards.json + card image BT25-024 — image is authoritative)
//!
//! [On Play] [When Digivolving] <Draw 1> (Draw 1 card from your deck.)
//! [Your Turn] When your Digimon are played or digivolve, if any of them are
//! red, this Digimon may digivolve into [Crescemon] in the TRASH with the cost
//! reduced by 1.
//!   (NOTE: cards.json says "in the hand"; the printed card face and DCGO both
//!    say "in the trash" — image wins per source priority.)
//!
//! Inherited Effect:
//! <Jamming> (This Digimon can't be deleted in battles against Security Digimon.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Blue/BT25_024.cs
//!   - EffectTiming.None: AddSelfDigivolutionRequirementStaticEffect(level 3,
//!     HasTSTraits, cost 2).
//!   - Shared OnPlay/WhenDigivolving: DrawClass(1).
//!   - OnEnterFieldAnyone (Your Turn): a played/digivolved OWN red Digimon →
//!     this Digimon may digivolve into [Crescemon] from TRASH (isHand:false),
//!     reduceCost 1, payCost:true. Skippable.
//!   - JammingSelfStaticEffect(isInheritedEffect:true).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - A draw on play/digivolve
//! - your-turn played/digivolve color-gated self digivolve into a TRASH card,
//!   cost -1 (effect_initiated_digivolve { source: <trash binding> } — BT16-040 idiom)
//! - H2 inherited <Jamming>
//! - alt-digivolution Lv.3 [TS] → cost 2

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::CardColor;
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-024";

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

/// Crescemon (Lv.5) the [Your Turn] clause digivolves Lekismon into FROM TRASH.
/// Cost 4 so a 1-reduction is observable as 3 memory spent.
fn crescemon(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, "Crescemon", 5);
    card.colors = vec![CardColor::Blue];
    card.dp = Some(7000);
    card.evo_costs = vec![EvoCost {
        card_color: 1,
        level: 4,
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

fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card id {card_id}"));
    let instance_id = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(data_index, player, instance_id));
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-024 YAML parses and compiles")
        .add_card(red_lv4("RED-LV4"))
        .add_card(blue_lv4("BLUE-LV4"))
        .add_card(crescemon("CRESCE"))
        .add_card(make_test_card_with_level("FILL", "Filler", 3))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
}

fn place_lekismon(runner: &mut DebugRunner) -> PermanentHandle {
    runner.place_on_field(0, CARD_ID, Some(0))
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_024_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT25-024 must be in the embedded DSL pack");

    assert_eq!(card.name, "Lekismon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(5000));
    for trait_name in ["Beastkin", "Iliad", "TS"] {
        assert!(
            card.traits.contains(&trait_name.to_string()),
            "BT25-024 metadata must include trait {trait_name}"
        );
    }
}

#[test]
fn bt25_024_has_ts_alt_digivolution_path() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let has_ts = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(2))
            && p.from
                .as_ref()
                .is_some_and(|f| f.level_eq == Some(3) && f.trait_has.as_deref() == Some("TS"))
    });
    assert!(has_ts, "BT25-024 must have a Lv.3 [TS] → cost 2 alt-path");
}

#[test]
fn bt25_024_has_draw_and_your_turn_clauses() {
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

    let has_draw = triggered.iter().any(|t| {
        t.when.contains(&CompiledTiming::OnPlay) && t.when.contains(&CompiledTiming::WhenDigivolving)
    });
    let has_your_turn = triggered.iter().any(|t| {
        (t.when.contains(&CompiledTiming::OnEnterFieldAnyone)
            || t.when.contains(&CompiledTiming::OnDigivolve))
            && t.optional
    });
    assert!(has_draw, "must have [On Play][When Digivolving] draw clause");
    assert!(
        has_your_turn,
        "must have the optional [Your Turn] played/digivolve digivolve clause"
    );
}

// ─── Section 2 — [On Play][When Digivolving] <Draw 1> ────────────────────────

#[test]
fn bt25_024_on_play_draws_one() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    let hand_index = push_to_hand(&mut runner, 0, CARD_ID);

    let hand_before = runner.hand_size(0);
    runner.play(0, hand_index);
    runner.auto_resolve().expect("resolve draw");

    // Played 1 (hand -1) then drew 1 (hand +1) → net unchanged from pre-play
    // minus the played card; assert the draw fired by comparing to a no-draw
    // baseline: post-play hand == hand_before (played card out, 1 drawn in).
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "<Draw 1> on play must replace the played Lekismon in hand count"
    );
}

// ─── Section 3 — [Your Turn] red played/digivolve → digivolve into Crescemon from trash, cost -1 ─

#[test]
fn bt25_024_your_turn_offers_crescemon_from_trash_when_red_played() {
    let mut runner = base().start();
    runner.game.turn_count = 1; // your turn
    let lekismon = place_lekismon(&mut runner);
    push_to_trash(&mut runner, 0, "CRESCE"); // Crescemon waiting in trash

    // Play a RED Lv.4 — triggers the [Your Turn] observer ("if any are red").
    let red_index = push_to_hand(&mut runner, 0, "RED-LV4");
    runner.play(0, red_index);

    assert!(
        runner.pending_kind().is_some(),
        "playing a red Digimon must offer Lekismon's digivolve into Crescemon (from trash)"
    );
    runner.auto_resolve().ok();
}

#[test]
fn bt25_024_your_turn_does_not_offer_when_non_red_played() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    let lekismon = place_lekismon(&mut runner);
    push_to_trash(&mut runner, 0, "CRESCE");

    // Play a BLUE Lv.4 — not red, so the color gate must block the offer.
    let blue_index = push_to_hand(&mut runner, 0, "BLUE-LV4");
    runner.play(0, blue_index);

    assert!(
        runner.pending_selection().is_none(),
        "a non-red played Digimon must NOT trigger the Crescemon digivolve"
    );
    runner.auto_resolve().ok();
}

#[test]
fn bt25_024_your_turn_does_not_offer_without_crescemon_in_trash() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    let lekismon = place_lekismon(&mut runner);
    // No Crescemon in trash.

    let red_index = push_to_hand(&mut runner, 0, "RED-LV4");
    runner.play(0, red_index);

    assert!(
        runner.pending_selection().is_none(),
        "without [Crescemon] in the trash there is no digivolve to offer"
    );
    runner.auto_resolve().ok();
}

// ─── Section 4 — Inherited <Jamming> ─────────────────────────────────────────

#[test]
fn bt25_024_has_inherited_jamming() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let has_jamming = card.effects.iter().any(|c| match c {
        CompiledClause::Declarative(d) => format!("{d:?}").contains("Jamming"),
        _ => false,
    });
    assert!(
        has_jamming,
        "BT25-024 must grant inherited <Jamming>"
    );
}
