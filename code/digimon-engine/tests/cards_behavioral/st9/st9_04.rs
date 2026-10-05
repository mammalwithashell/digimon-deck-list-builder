//! ST9-04 ExVeemon — Digimon, Lv.4, Blue, DP 4000, Cost 4.
//! Traits: Mythical Dragon. Digivolve: blue Lv.3 / 2, green Lv.3 / 2.
//!
//! # Card text (official Bandai DB, data/card_bundles/ST9-04.md; card face checked)
//!
//! **Effect:** When you would play this card from your hand, if you have a green
//! Digimon in play, reduce its play cost by 1.
//!
//! **Inherited Effect:** [When Attacking] If you have a green Digimon in play,
//! this Digimon gets +1000 DP for the turn.
//!
//! Official Q&A: if the Digimon carrying this card is itself green, the inherited
//! effect activates.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST9/Blue/ST9_04.cs — ChangeCostClass -1 (hand,
//! root = Hand, this card, owner has a green Digimon); inherited OnAllyAttack
//! ChangeDigimonDP(+1000, UntilEachTurnEnd) gated on the same green check.
//!
//! # Patterns
//! - D2 self cost_reduction (`when_playing_this`) — mirror of ST9-09 Stingmon
//! - G4 inherited [When Attacking] conditional self DP buff for the turn

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "ST9-04";

fn digimon(id: &str, color: CardColor, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(dp);
    c
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST9-04 in embedded DSL pack")
        .add_card(digimon("GREEN-DIGI", CardColor::Green, 4, 5000))
        .add_card(digimon("RED-DIGI", CardColor::Red, 4, 5000))
        .add_card(digimon("BLUE-CARRIER", CardColor::Blue, 5, 6000))
        .add_card(digimon("GREEN-CARRIER", CardColor::Green, 5, 6000))
        .add_card(make_test_card("DECK-PAD", "DECK-PAD"))
        .deck(0, &["DECK-PAD"; 5])
        .deck(1, &["DECK-PAD"; 5])
        .memory(10)
        .start()
}

fn put_exveemon_in_hand(runner: &mut DebugRunner) {
    runner.game_mut().players[0].hand.clear();
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == CARD_ID)
        .expect("ST9-04 in card_data");
    let next = runner.game.next_card_index();
    runner.game_mut().players[0]
        .hand
        .push(CardSource::new(data_idx, 0, next));
}

fn play_cost_paid(runner: &mut DebugRunner) -> i16 {
    put_exveemon_in_hand(runner);
    let before = runner.memory();
    runner.play(0, 0).expect("ST9-04 plays from hand");
    before - runner.memory()
}

/// `carrier` on P0's field with ExVeemon inserted as its bottom source.
fn carrier_with_exveemon(runner: &mut DebugRunner, carrier: &str) -> PermanentHandle {
    let handle = runner.place_on_field(0, carrier, Some(0));
    let game = runner.game_mut();
    let data_idx = game
        .card_data
        .iter()
        .position(|c| c.card_id == CARD_ID)
        .expect("ST9-04 in card_data");
    let next = game.next_card_index();
    game.players[0].battle_area[handle.index as usize]
        .card_sources
        .insert(0, CardSource::new(data_idx, 0, next));
    handle
}

fn dp_after_attack(runner: &mut DebugRunner, carrier: PermanentHandle) -> i32 {
    runner.game_mut().players[1].security.clear();
    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();
    runner.effective_dp(carrier).expect("carrier on field")
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn st9_04_metadata_and_clause_shape() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "ExVeemon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(4));
    assert_eq!(card.alt_paths.len(), 2, "blue Lv.3 and green Lv.3 circles");

    let inherited = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("inherited [When Attacking] clause");
    assert!(!inherited.optional, "no 'you may' in the printed text");
    assert!(!inherited.once_per_turn);
}

// ─── Section 2 — Play-cost reduction ─────────────────────────────────────────

#[test]
fn st9_04_costs_3_with_a_green_digimon_in_play() {
    let mut runner = runner();
    runner.place_on_field(0, "GREEN-DIGI", None);
    assert_eq!(play_cost_paid(&mut runner), 3);
}

#[test]
fn st9_04_costs_4_with_no_digimon_in_play() {
    let mut runner = runner();
    assert_eq!(play_cost_paid(&mut runner), 4);
}

#[test]
fn st9_04_costs_4_with_only_a_non_green_digimon() {
    let mut runner = runner();
    runner.place_on_field(0, "RED-DIGI", None);
    assert_eq!(play_cost_paid(&mut runner), 4);
}

#[test]
fn st9_04_reduction_does_not_apply_to_other_cards() {
    let mut other = digimon("OTHER", CardColor::Blue, 4, 4000);
    other.play_cost = 5;
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST9-04 in embedded DSL pack")
        .add_card(digimon("GREEN-DIGI", CardColor::Green, 4, 5000))
        .add_card(other)
        .hand(0, &["OTHER"])
        .memory(10)
        .start();
    runner.place_on_field(0, "GREEN-DIGI", None);
    let before = runner.memory();
    runner.play(0, 0).expect("OTHER plays");
    assert_eq!(before - runner.memory(), 5);
}

// ─── Section 3 — Inherited [When Attacking] +1000 DP ─────────────────────────

#[test]
fn st9_04_inherited_gives_plus_1000_with_another_green_digimon() {
    let mut runner = runner();
    let carrier = carrier_with_exveemon(&mut runner, "BLUE-CARRIER");
    runner.place_on_field(0, "GREEN-DIGI", None);
    assert_eq!(dp_after_attack(&mut runner, carrier), 7000);
}

#[test]
fn st9_04_inherited_counts_a_green_carrier_itself() {
    // Official Q&A: a green Digimon carrying this card satisfies the check.
    let mut runner = runner();
    let carrier = carrier_with_exveemon(&mut runner, "GREEN-CARRIER");
    assert_eq!(dp_after_attack(&mut runner, carrier), 7000);
}

#[test]
fn st9_04_inherited_does_nothing_without_a_green_digimon() {
    let mut runner = runner();
    let carrier = carrier_with_exveemon(&mut runner, "BLUE-CARRIER");
    runner.place_on_field(0, "RED-DIGI", None);
    assert_eq!(dp_after_attack(&mut runner, carrier), 6000);
}

#[test]
fn st9_04_inherited_buff_ends_with_the_turn() {
    let mut runner = runner();
    let carrier = carrier_with_exveemon(&mut runner, "BLUE-CARRIER");
    runner.place_on_field(0, "GREEN-DIGI", None);
    runner.game_mut().players[1].security = runner.game.players[1].deck.clone();
    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();
    assert_eq!(runner.effective_dp(carrier), Some(7000));
    runner.end_turn();
    assert_eq!(
        runner.effective_dp(carrier),
        Some(6000),
        "'for the turn' only"
    );
}
