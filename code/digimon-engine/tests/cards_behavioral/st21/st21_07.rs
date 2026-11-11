//! ST21-07 Palmon — Lv.3, Green, Vegetation / ADVENTURE, Rookie, Data,
//! DP 1000, Cost 3.
//!
//! # Card text (official Bandai DB, data/card_bundles/ST21-07.md)
//!
//! ```text
//! [On Play] By trashing 1 card with the [ADVENTURE] trait in your hand,
//! <Draw 2> (Draw 2 cards from your deck.)
//!
//! Inherited: [All Turns] This Digimon gets +1000 DP.
//!
//! [Digivolve box] Lv.2 w/[ADVENTURE] trait: Cost 0
//! [Standard circle] Green Lv.2: Cost 0
//! ```
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/ST21/Green/ST21_07.cs`
//!
//! # Patterns this test file covers
//!
//! | Clause                                   | Pattern                                         |
//! |------------------------------------------|-------------------------------------------------|
//! | alt-path (Lv.2 ADVENTURE, cost 0) + green Lv.2 | digivolve alt_paths (structural + behavioral) |
//! | [On Play] by trashing 1 [ADVENTURE] card -> <Draw 2> | optional clause, `select_hand` cost pick, decline aborts the draw, GameEvent::Trash cost event |
//! | Inherited [All Turns] +1000 DP           | inherited aura via digivolution stack, both turns |
//!
//! # Assertions the author was least able to confirm without running them
//!
//! - Whether the optional clause shows an outer accept prompt before the hand
//!   pick; every test goes through `reach_hand` (BT20-007 idiom), which
//!   accepts the outer prompt only when the hand pick is not already pending.
//! - `GameEvent::Trash { card_id, .. }` carries the card id of the trashed
//!   hand card.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

use crate::dsl_card_data::compiled;

const CARD_ID: &str = "ST21-07";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn digimon(id: &str, color: CardColor, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 3;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn tamer(id: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Red];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-07 in embedded DSL pack")
        // Hand cards: the cost pick must offer only [ADVENTURE] cards.
        .add_card(digimon("ADV-CARD", CardColor::Red, 3, 3000, &["ADVENTURE"]))
        .add_card(digimon("ADV-CARD-2", CardColor::Blue, 4, 4000, &["ADVENTURE"]))
        .add_card(tamer("ADV-TAMER", &["ADVENTURE"]))
        .add_card(digimon("PLAIN-CARD", CardColor::Red, 3, 3000, &[]))
        // Digivolution bases (Lv.2 / Lv.3).
        .add_card(digimon("GREEN-LV2", CardColor::Green, 2, 1000, &[]))
        .add_card(digimon("BLUE-LV2-ADV", CardColor::Blue, 2, 1000, &["ADVENTURE"]))
        .add_card(digimon("RED-LV2-PLAIN", CardColor::Red, 2, 1000, &[]))
        .add_card(digimon("GREEN-LV3-ADV", CardColor::Green, 3, 3000, &["ADVENTURE"]))
        // Inherited-effect carrier.
        .add_card(digimon("CARRIER", CardColor::Red, 4, 3000, &[]))
        .add_card(make_test_card("PAD", "PAD"))
        .deck(0, &["PAD"; 10])
        .deck(1, &["PAD"; 10])
        .memory(10)
}

fn hand_index(runner: &DebugRunner, id: &str) -> usize {
    runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} must be in player 0's hand"))
}

fn top_id(runner: &DebugRunner, h: PermanentHandle) -> String {
    runner.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string()
}

fn trash_has(runner: &DebugRunner, id: &str) -> bool {
    runner.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == id)
}

/// Palmon on the field plus the given hand; [On Play] fired.
fn on_play_runner(hand: &[&str]) -> DebugRunner {
    let mut runner = builder().hand(0, hand).start();
    let palmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, palmon.index as usize);
    runner
}

/// Reach the hand-pick prompt, accepting the outer optional prompt if one is
/// shown first (BT20-007 idiom).
fn reach_hand(runner: &mut DebugRunner) {
    if runner.pending_kind() != Some(SelectionKind::Hand) {
        runner
            .accept_optional_trigger()
            .expect("accept the optional [On Play]");
    }
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st21_07_identity_matches_printed_card() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.name, "Palmon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.dp, Some(1000));
    assert_eq!(card.cost, Some(3));
    let traits: Vec<String> = card.traits.iter().map(|t| t.to_lowercase()).collect();
    assert!(traits.iter().any(|t| t == "vegetation"), "{:?}", card.traits);
    assert!(traits.iter().any(|t| t == "adventure"), "{:?}", card.traits);
}

#[test]
fn st21_07_alt_paths_are_green_lv2_and_adventure_lv2_both_cost_0() {
    let card = compiled(CARD_ID);
    assert_eq!(
        card.alt_paths.len(),
        2,
        "printed green Lv.2 circle + the [Digivolve] Lv.2 w/[ADVENTURE] box"
    );
    for path in &card.alt_paths {
        assert_eq!(path.cost, Some(CompiledCost::Literal(0)));
        assert!(
            !path.ignore_requirements,
            "DCGO ignoreDigivolutionRequirement is false"
        );
    }
    let adventure_path = format!("{:?}", card.alt_paths[1].from);
    assert!(
        adventure_path.contains("ADVENTURE"),
        "second alt path gates on the ADVENTURE trait, got {adventure_path}"
    );
}

#[test]
fn st21_07_clause_shape() {
    let card = compiled(CARD_ID);
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1, "only the [On Play] clause is triggered");
    let on_play = triggered[0];
    assert_eq!(on_play.when, vec![CompiledTiming::OnPlay]);
    assert_eq!(on_play.scope, CompiledScope::FaceUp);
    assert!(on_play.optional, "'By trashing ...' is an opt-in cost (DCGO isOptional)");
    assert!(!on_play.once_per_turn, "no [Once Per Turn] is printed");

    let aura = card.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope, dp_modifier, ..
        }) => Some((*scope, *dp_modifier)),
        _ => None,
    });
    assert_eq!(
        aura,
        Some((CompiledScope::Inherited, Some(1000))),
        "inherited [All Turns] +1000 DP"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Digivolution requirements
// ═══════════════════════════════════════════════════════════════════════════

fn digivolve_into_palmon(base_id: &str) -> (DebugRunner, PermanentHandle, bool, i16) {
    let mut runner = builder().hand(0, &[CARD_ID]).start();
    let base = runner.place_on_field(0, base_id, Some(0));
    let idx = hand_index(&runner, CARD_ID);
    let before = runner.memory();
    let ok = runner
        .game
        .digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByHand);
    let paid = before - runner.memory();
    let _ = runner.auto_resolve();
    (runner, base, ok, paid)
}

#[test]
fn st21_07_digivolves_from_green_lv2_for_0() {
    let (runner, base, ok, paid) = digivolve_into_palmon("GREEN-LV2");
    assert!(ok, "printed green Lv.2 circle");
    assert_eq!(paid, 0);
    assert_eq!(top_id(&runner, base), CARD_ID);
}

#[test]
fn st21_07_digivolves_from_any_color_lv2_adventure_for_0() {
    let (runner, base, ok, paid) = digivolve_into_palmon("BLUE-LV2-ADV");
    assert!(ok, "[Digivolve] Lv.2 w/[ADVENTURE] trait accepts a blue base");
    assert_eq!(paid, 0);
    assert_eq!(top_id(&runner, base), CARD_ID);
}

#[test]
fn st21_07_cannot_digivolve_from_non_green_non_adventure_lv2() {
    let mut runner = builder().hand(0, &[CARD_ID]).start();
    let base = runner.place_on_field(0, "RED-LV2-PLAIN", Some(0));
    let card = runner.game.players[0].hand[hand_index(&runner, CARD_ID)].clone();
    let perm = runner.game.players[0].battle_area[base.index as usize].clone();
    assert!(
        !runner.game.can_digivolve(&card, &perm),
        "red Lv.2 without [ADVENTURE] matches neither route"
    );
}

#[test]
fn st21_07_cannot_digivolve_from_adventure_lv3() {
    let mut runner = builder().hand(0, &[CARD_ID]).start();
    let base = runner.place_on_field(0, "GREEN-LV3-ADV", Some(0));
    let card = runner.game.players[0].hand[hand_index(&runner, CARD_ID)].clone();
    let perm = runner.game.players[0].battle_area[base.index as usize].clone();
    assert!(
        !runner.game.can_digivolve(&card, &perm),
        "the [ADVENTURE] box needs a LEVEL 2 base"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — [On Play] by trashing 1 [ADVENTURE] card from hand, <Draw 2>
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st21_07_on_play_without_an_adventure_card_in_hand_offers_nothing() {
    let runner = on_play_runner(&["PLAIN-CARD"]);
    assert!(
        runner.pending_selection().is_none(),
        "no [ADVENTURE] card to trash -> the effect cannot be activated"
    );
    assert_eq!(runner.hand_size(0), 1);
    assert_eq!(runner.deck_size(0), 10);
}

#[test]
fn st21_07_cost_pick_offers_only_adventure_cards() {
    let mut runner = on_play_runner(&["ADV-CARD", "PLAIN-CARD", "ADV-TAMER"]);
    reach_hand(&mut runner);

    let view = runner.pending_selection_view().expect("hand pick");
    let adv = PLAY_HAND_START + hand_index(&runner, "ADV-CARD") as u16;
    let tamer = PLAY_HAND_START + hand_index(&runner, "ADV-TAMER") as u16;
    let plain = PLAY_HAND_START + hand_index(&runner, "PLAIN-CARD") as u16;
    assert!(view.valid_action_ids.contains(&adv));
    assert!(
        view.valid_action_ids.contains(&tamer),
        "any card with the [ADVENTURE] trait qualifies, not only Digimon"
    );
    assert!(
        !view.valid_action_ids.contains(&plain),
        "a card without [ADVENTURE] cannot be the cost"
    );
    assert!(view.is_optional, "the trash is an opt-in cost");
}

#[test]
fn st21_07_trashing_an_adventure_card_draws_two() {
    let mut runner = on_play_runner(&["ADV-CARD", "ADV-CARD-2", "PLAIN-CARD"]);
    reach_hand(&mut runner);
    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);
    let trash_before = runner.trash_size(0);

    let pick = PLAY_HAND_START + hand_index(&runner, "ADV-CARD") as u16;
    runner.execute_action(0, pick).expect("trash the ADV card");
    let _ = runner.auto_resolve();

    assert!(trash_has(&runner, "ADV-CARD"), "the chosen card is trashed");
    assert!(
        !trash_has(&runner, "ADV-CARD-2"),
        "only the chosen card is trashed"
    );
    assert_eq!(runner.trash_size(0), trash_before + 1);
    assert_eq!(runner.deck_size(0), deck_before - 2, "<Draw 2>");
    assert_eq!(
        runner.hand_size(0),
        hand_before - 1 + 2,
        "one card out (trashed), two cards in (drawn)"
    );
}

#[test]
fn st21_07_declining_the_cost_pick_draws_nothing() {
    let mut runner = on_play_runner(&["ADV-CARD"]);
    reach_hand(&mut runner);
    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);
    let trash_before = runner.trash_size(0);

    runner.execute_action(0, PASS).expect("decline the cost");
    let _ = runner.auto_resolve();

    assert_eq!(runner.hand_size(0), hand_before);
    assert_eq!(runner.deck_size(0), deck_before, "no <Draw 2> without the cost");
    assert_eq!(runner.trash_size(0), trash_before);
}

#[test]
fn st21_07_declining_the_outer_prompt_does_nothing() {
    let mut runner = on_play_runner(&["ADV-CARD"]);
    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);
    if runner.pending_kind() == Some(SelectionKind::Hand) {
        runner.execute_action(0, PASS).expect("decline at the pick");
    } else {
        runner
            .decline_optional_trigger()
            .expect("decline the optional [On Play]");
    }
    let _ = runner.auto_resolve();
    assert_eq!(runner.hand_size(0), hand_before);
    assert_eq!(runner.deck_size(0), deck_before);
    assert_eq!(runner.trash_size(0), 0);
}

#[test]
fn st21_07_cost_trash_emits_a_trash_event() {
    let mut runner = on_play_runner(&["ADV-CARD"]);
    reach_hand(&mut runner);
    let cp = runner.event_checkpoint();
    let pick = PLAY_HAND_START + hand_index(&runner, "ADV-CARD") as u16;
    runner.execute_action(0, pick).expect("pay the cost");
    let _ = runner.auto_resolve();

    let trashed: Vec<&GameEvent> = runner
        .events_since(cp)
        .iter()
        .filter(|e| matches!(e, GameEvent::Trash { player: 0, .. }))
        .collect();
    assert!(
        !trashed.is_empty(),
        "paying the cost must go through the normal trash path and emit GameEvent::Trash"
    );
}

#[test]
fn st21_07_playing_palmon_from_hand_runs_its_on_play() {
    let mut runner = builder().hand(0, &[CARD_ID, "ADV-CARD"]).start();
    let idx = hand_index(&runner, CARD_ID);
    runner.play(0, idx).expect("Palmon plays");
    reach_hand(&mut runner);
    let pick = PLAY_HAND_START + hand_index(&runner, "ADV-CARD") as u16;
    let deck_before = runner.deck_size(0);
    runner.execute_action(0, pick).expect("pay the cost");
    let _ = runner.auto_resolve();
    assert_eq!(runner.deck_size(0), deck_before - 2);
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Inherited [All Turns] +1000 DP
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st21_07_inherited_plus_1000_applies_to_the_carrier_on_both_turns() {
    let mut runner = builder().start();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.effective_dp(carrier),
        Some(4000),
        "3000 + 1000 on my turn"
    );

    runner.end_turn();
    let _ = runner.auto_resolve_declining_end_of_turn_attacks();
    runner.game.tick_declarative_effects();
    assert_eq!(runner.turn_player(), 1);
    assert_eq!(
        runner.effective_dp(carrier),
        Some(4000),
        "[All Turns]: still +1000 on the opponent's turn"
    );
}

#[test]
fn st21_07_inherited_bonus_is_absent_without_palmon_in_the_stack() {
    let mut runner = builder().start();
    let plain = runner.place_on_field(0, "CARRIER", Some(0));
    runner.game.tick_declarative_effects();
    assert_eq!(runner.effective_dp(plain), Some(3000));
}

#[test]
fn st21_07_inherited_bonus_does_not_apply_to_palmon_itself_as_top_card() {
    let mut runner = builder().start();
    let palmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.effective_dp(palmon),
        Some(1000),
        "an inherited effect only works while the card is a digivolution card"
    );
}
