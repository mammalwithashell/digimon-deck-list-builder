//! ST21-10 Gabumon — Lv.3, Purple, Reptile / ADVENTURE, Rookie, Data,
//! DP 1000, Cost 3.
//!
//! # Card text (official Bandai DB, data/card_bundles/ST21-10.md)
//!
//! ```text
//! [Your Turn] While your opponent has a Digimon with 10000 DP or more, or your
//! Tamers have 3 or more total colors, this Digimon can digivolve into
//! [MetalGarurumon] in the hand for a digivolution cost of 4, ignoring
//! digivolution requirements.
//!
//! Inherited: [When Attacking] [Once Per Turn] <Draw 1> (Draw 1 card from your
//! deck.) Then, trash 1 card in your hand.
//!
//! [Digivolve box] Lv.2 w/[ADVENTURE] trait: Cost 0
//! [Standard circles] Purple Lv.2: Cost 0 / Blue Lv.2: Cost 0
//! Official Q&A: "3 or more total colors" = as many or more colors than the
//! number specified.
//! ```
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/ST21/Purple/ST21_10.cs` (copied from the
//! ST20-10 Agumon implementation; its alt requirement also admits `HERO`,
//! which the PRINTED box does not -- see
//! `st21_10_does_not_digivolve_from_a_hero_only_lv2`).
//!
//! # Patterns this test file covers
//!
//! | Clause                                             | Pattern                                    |
//! |----------------------------------------------------|--------------------------------------------|
//! | standard circles + [Digivolve] Lv.2 ADVENTURE box  | digivolve alt_paths (structural + behavioral) |
//! | [Your Turn] warp into [MetalGarurumon], cost 4, ignoring requirements | `direction: into` alt-path with `condition:` (ST20-10 exemplar) |
//! | Inherited [When Attacking][OPT] draw 1, trash 1    | inherited when_attacking, OPT lockout/reset, mandatory trash pick, GameEvent::Trash |
//!
//! # Assertions the author was least able to confirm without running them
//!
//! - The inherited attack tests drive `attack_player` and expect the effect's
//!   hand-pick to be the FIRST pending selection (EX12-015 idiom).
//! - The OPT-reset test relies on two `end_turn()` calls re-unsuspending the
//!   carrier.
//! - The Hero-only negative encodes the PRINTED box (ADVENTURE only), which
//!   deliberately differs from DCGO's ST20-10-derived `|| HERO`.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathDirection, CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope,
    CompiledTiming,
};
use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

use crate::dsl_card_data::compiled;

const CARD_ID: &str = "ST21-10";

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

/// A named Lv.6 Digimon with EMPTY evo_costs: the standard rules route cannot
/// apply, so a successful digivolve proves the `direction: into` warp path.
fn named_lv6(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(12000);
    c.play_cost = 7;
    c.colors = vec![CardColor::Blue];
    c.evo_costs = Vec::new();
    c
}

fn tamer(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![color];
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-10 in embedded DSL pack")
        // Warp targets.
        .add_card(named_lv6("MG", "MetalGarurumon"))
        .add_card(named_lv6("MG-X", "MetalGarurumon (X Antibody)"))
        .add_card(named_lv6("WG", "WarGreymon"))
        // Opponent Digimon.
        .add_card(digimon("OPP-BIG", CardColor::Black, 5, 10000, &[]))
        .add_card(digimon("OPP-NEARLY", CardColor::Black, 5, 9999, &[]))
        .add_card(digimon("OPP-SMALL", CardColor::Black, 4, 5000, &[]))
        // Tamers.
        .add_card(tamer("T-RED", CardColor::Red))
        .add_card(tamer("T-RED-2", CardColor::Red))
        .add_card(tamer("T-BLUE", CardColor::Blue))
        .add_card(tamer("T-GREEN", CardColor::Green))
        // Digivolution bases for Gabumon (Lv.2) and warp-base negatives.
        .add_card(digimon("PURPLE-LV2", CardColor::Purple, 2, 1000, &[]))
        .add_card(digimon("BLUE-LV2", CardColor::Blue, 2, 1000, &[]))
        .add_card(digimon("RED-LV2-ADV", CardColor::Red, 2, 1000, &["ADVENTURE"]))
        .add_card(digimon("RED-LV2-HERO", CardColor::Red, 2, 1000, &["Hero"]))
        .add_card(digimon("RED-LV2-PLAIN", CardColor::Red, 2, 1000, &[]))
        .add_card(digimon("PURPLE-LV3-ADV", CardColor::Purple, 3, 3000, &["ADVENTURE"]))
        .add_card(digimon("PLAIN-LV5", CardColor::Purple, 5, 9000, &[]))
        // Inherited-effect carrier and hand fodder.
        .add_card(digimon("CARRIER", CardColor::Red, 4, 5000, &[]))
        .add_card(digimon("SEC", CardColor::Red, 3, 1000, &[]))
        .add_card(make_test_card("HAND-A", "HAND-A"))
        .add_card(make_test_card("HAND-B", "HAND-B"))
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

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st21_10_identity_matches_printed_card() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.name, "Gabumon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.dp, Some(1000));
    assert_eq!(card.cost, Some(3));
    let traits: Vec<String> = card.traits.iter().map(|t| t.to_lowercase()).collect();
    assert!(traits.iter().any(|t| t == "reptile"), "{:?}", card.traits);
    assert!(traits.iter().any(|t| t == "adventure"), "{:?}", card.traits);
}

#[test]
fn st21_10_has_four_alt_paths() {
    let card = compiled(CARD_ID);
    assert_eq!(
        card.alt_paths.len(),
        4,
        "purple Lv.2 + blue Lv.2 circles, the ADVENTURE Lv.2 box, and the MetalGarurumon warp"
    );
    for path in &card.alt_paths[..3] {
        assert_eq!(path.kind, CompiledAltPathKind::Digivolve);
        assert_eq!(path.cost, Some(CompiledCost::Literal(0)));
        assert!(!path.ignore_requirements);
    }
    let adventure_path = format!("{:?}", card.alt_paths[2].from);
    assert!(adventure_path.contains("ADVENTURE"), "{adventure_path}");
}

#[test]
fn st21_10_fourth_alt_path_is_the_metalgarurumon_warp() {
    let card = compiled(CARD_ID);
    let path = &card.alt_paths[3];
    assert_eq!(path.kind, CompiledAltPathKind::Digivolve);
    assert_eq!(
        path.direction,
        CompiledAltPathDirection::Into,
        "the alt-path lives on the source (this Digimon) and filters the hand card"
    );
    assert_eq!(path.cost, Some(CompiledCost::Literal(4)));
    assert!(path.ignore_requirements, "'ignoring digivolution requirements'");
    let from = path.from.as_ref().expect("hand-card filter");
    assert_eq!(from.name_is.as_deref(), Some("MetalGarurumon"));
    assert!(path.condition.is_some(), "the [Your Turn] + board-state gate");
}

#[test]
fn st21_10_clause_shape() {
    let card = compiled(CARD_ID);
    assert_eq!(card.effects.len(), 1, "only the inherited attack effect");
    let clause = match &card.effects[0] {
        CompiledClause::Triggered(t) => t,
        other => panic!("expected a triggered clause, got {other:?}"),
    };
    assert_eq!(clause.when, vec![CompiledTiming::WhenAttacking]);
    assert_eq!(clause.scope, CompiledScope::Inherited);
    assert!(clause.once_per_turn, "[Once Per Turn]");
    assert!(!clause.optional, "draw and trash are mandatory");
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Digivolution requirements
// ═══════════════════════════════════════════════════════════════════════════

fn digivolve_into_gabumon(base_id: &str) -> (DebugRunner, PermanentHandle, bool, i16) {
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

fn cannot_digivolve_from(base_id: &str) -> bool {
    let mut runner = builder().hand(0, &[CARD_ID]).start();
    let base = runner.place_on_field(0, base_id, Some(0));
    let card = runner.game.players[0].hand[hand_index(&runner, CARD_ID)].clone();
    let perm = runner.game.players[0].battle_area[base.index as usize].clone();
    !runner.game.can_digivolve(&card, &perm)
}

#[test]
fn st21_10_digivolves_from_purple_lv2_for_0() {
    let (runner, base, ok, paid) = digivolve_into_gabumon("PURPLE-LV2");
    assert!(ok);
    assert_eq!(paid, 0);
    assert_eq!(top_id(&runner, base), CARD_ID);
}

#[test]
fn st21_10_digivolves_from_blue_lv2_for_0() {
    let (runner, base, ok, paid) = digivolve_into_gabumon("BLUE-LV2");
    assert!(ok, "the second printed circle is Blue Lv.2");
    assert_eq!(paid, 0);
    assert_eq!(top_id(&runner, base), CARD_ID);
}

#[test]
fn st21_10_digivolves_from_any_color_lv2_adventure_for_0() {
    let (runner, base, ok, paid) = digivolve_into_gabumon("RED-LV2-ADV");
    assert!(ok, "[Digivolve] Lv.2 w/[ADVENTURE] trait accepts a red base");
    assert_eq!(paid, 0);
    assert_eq!(top_id(&runner, base), CARD_ID);
}

#[test]
fn st21_10_cannot_digivolve_from_unrelated_lv2() {
    assert!(cannot_digivolve_from("RED-LV2-PLAIN"));
}

#[test]
fn st21_10_does_not_digivolve_from_a_hero_only_lv2() {
    // The printed box is "w/[ADVENTURE] trait" only (official DB); DCGO's
    // ST20-10-derived code also admits Hero. Printed data wins.
    assert!(cannot_digivolve_from("RED-LV2-HERO"));
}

#[test]
fn st21_10_cannot_digivolve_from_adventure_lv3() {
    assert!(cannot_digivolve_from("PURPLE-LV3-ADV"));
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — [Your Turn] warp digivolve into [MetalGarurumon]
// ═══════════════════════════════════════════════════════════════════════════

/// Gabumon on the field, `hand` in hand, then `opp`/`tamers` placed.
fn warp_setup(
    hand: &[&str],
    opp: &[&str],
    tamers: &[&str],
) -> (DebugRunner, PermanentHandle) {
    let mut runner = builder().hand(0, hand).start();
    let gabumon = runner.place_on_field(0, CARD_ID, Some(0));
    for id in opp {
        runner.place_on_field(1, id, Some(0));
    }
    for id in tamers {
        runner.place_on_field(0, id, None);
    }
    (runner, gabumon)
}

fn try_warp(runner: &mut DebugRunner, base: PermanentHandle, hand_id: &str) -> bool {
    let idx = hand_index(runner, hand_id);
    runner
        .game
        .digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByHand)
}

#[test]
fn st21_10_warp_via_opponent_digimon_with_10000_dp() {
    let (mut runner, gabumon) = warp_setup(&["MG"], &["OPP-BIG"], &[]);
    let before = runner.memory();
    assert!(try_warp(&mut runner, gabumon, "MG"));
    assert_eq!(top_id(&runner, gabumon), "MG");
    assert_eq!(before - runner.memory(), 4, "digivolution cost 4");
}

#[test]
fn st21_10_warp_blocked_when_opponents_biggest_digimon_is_9999_dp() {
    let (mut runner, gabumon) = warp_setup(&["MG"], &["OPP-NEARLY"], &[]);
    assert!(!try_warp(&mut runner, gabumon, "MG"));
    assert_eq!(top_id(&runner, gabumon), CARD_ID);
}

#[test]
fn st21_10_warp_via_three_tamer_colors() {
    let (mut runner, gabumon) =
        warp_setup(&["MG"], &["OPP-SMALL"], &["T-RED", "T-BLUE", "T-GREEN"]);
    let before = runner.memory();
    assert!(try_warp(&mut runner, gabumon, "MG"));
    assert_eq!(top_id(&runner, gabumon), "MG");
    assert_eq!(before - runner.memory(), 4);
}

#[test]
fn st21_10_warp_blocked_with_only_two_tamer_colors() {
    let (mut runner, gabumon) = warp_setup(&["MG"], &["OPP-SMALL"], &["T-RED", "T-BLUE"]);
    assert!(!try_warp(&mut runner, gabumon, "MG"));
}

#[test]
fn st21_10_warp_blocked_when_tamer_colors_repeat() {
    let (mut runner, gabumon) =
        warp_setup(&["MG"], &["OPP-SMALL"], &["T-RED", "T-RED-2", "T-BLUE"]);
    assert!(
        !try_warp(&mut runner, gabumon, "MG"),
        "red, red, blue = 2 distinct colors"
    );
}

#[test]
fn st21_10_warp_blocked_on_the_opponents_turn() {
    let (mut runner, gabumon) = warp_setup(&["MG"], &["OPP-BIG"], &[]);
    runner.game.turn_player_idx = 1;
    assert!(
        !try_warp(&mut runner, gabumon, "MG"),
        "[Your Turn] only"
    );
}

#[test]
fn st21_10_warp_only_accepts_a_card_named_exactly_metalgarurumon() {
    let (mut runner, gabumon) = warp_setup(&["WG", "MG-X"], &["OPP-BIG"], &[]);
    assert!(
        !try_warp(&mut runner, gabumon, "WG"),
        "a differently named hand card is not a legal warp target"
    );
    assert!(
        !try_warp(&mut runner, gabumon, "MG-X"),
        "[MetalGarurumon (X Antibody)] is a different card name"
    );
    assert_eq!(top_id(&runner, gabumon), CARD_ID);
}

#[test]
fn st21_10_warp_is_registered_on_gabumon_only() {
    let (mut runner, _gabumon) = warp_setup(&["MG"], &["OPP-BIG"], &[]);
    let other = runner.place_on_field(0, "PLAIN-LV5", Some(0));
    assert!(
        !try_warp(&mut runner, other, "MG"),
        "another Digimon cannot use Gabumon's warp (MetalGarurumon has no printed route here)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Inherited [When Attacking][OPT] <Draw 1>, then trash 1
// ═══════════════════════════════════════════════════════════════════════════

/// A red Lv.4 carrier with Gabumon as its digivolution card, opponent with
/// three security cards, plus the given hand.
fn attack_setup(hand: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut runner = builder()
        .hand(0, hand)
        .security(1, &["SEC", "SEC", "SEC"])
        .start();
    runner.game.turn_count = 1;
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    (runner, carrier)
}

#[test]
fn st21_10_inherited_attack_draws_one_then_forces_a_trash() {
    let (mut runner, carrier) = attack_setup(&["HAND-A", "HAND-B"]);
    let deck_before = runner.deck_size(0);
    runner.attack_player(carrier, 1, false);

    let view = runner
        .pending_selection_view()
        .expect("trash pick follows the draw");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(!view.is_optional, "the trash is mandatory");
    assert!(!view.valid_action_ids.contains(&PASS));
    assert_eq!(runner.deck_size(0), deck_before - 1, "<Draw 1> happened first");
    assert_eq!(
        view.valid_action_ids.len(),
        3,
        "the whole hand (2 + the drawn card) is eligible"
    );

    let pick = PLAY_HAND_START + hand_index(&runner, "HAND-A") as u16;
    runner.execute_action(0, pick).expect("trash HAND-A");
    let _ = runner.auto_resolve();

    assert!(trash_has(&runner, "HAND-A"), "the chosen card is trashed");
    assert!(!trash_has(&runner, "HAND-B"));
    assert_eq!(runner.hand_size(0), 2, "2 in hand + 1 drawn - 1 trashed");
}

#[test]
fn st21_10_the_card_just_drawn_may_be_trashed() {
    // Empty hand: the only card to trash is the one just drawn.
    let (mut runner, carrier) = attack_setup(&[]);
    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();
    assert_eq!(runner.hand_size(0), 0, "drew 1, trashed 1");
    assert_eq!(runner.trash_size(0), 1);
}

#[test]
fn st21_10_trash_cost_emits_a_trash_event() {
    let (mut runner, carrier) = attack_setup(&["HAND-A"]);
    let cp = runner.event_checkpoint();
    runner.attack_player(carrier, 1, false);
    let pick = PLAY_HAND_START + hand_index(&runner, "HAND-A") as u16;
    runner.execute_action(0, pick).expect("trash HAND-A");
    let _ = runner.auto_resolve();
    let trashed = runner
        .events_since(cp)
        .iter()
        .filter(|e| matches!(e, GameEvent::Trash { player: 0, .. }))
        .count();
    assert!(trashed >= 1, "the discard must go through the normal trash path");
}

#[test]
fn st21_10_inherited_effect_is_once_per_turn() {
    let (mut runner, carrier) = attack_setup(&["HAND-A", "HAND-B"]);
    runner.attack_player(carrier, 1, false);
    let pick = PLAY_HAND_START + hand_index(&runner, "HAND-A") as u16;
    runner.execute_action(0, pick).expect("trash HAND-A");
    let _ = runner.auto_resolve();

    // Ready the carrier again and attack a second time this turn.
    runner.game.players[0].battle_area[carrier.index as usize].is_suspended = false;
    let deck_before = runner.deck_size(0);
    runner.attack_player(carrier, 1, false);
    assert_ne!(
        runner.pending_kind(),
        Some(SelectionKind::Hand),
        "[Once Per Turn]: no second draw/trash"
    );
    let _ = runner.auto_resolve();
    assert_eq!(runner.deck_size(0), deck_before, "no second <Draw 1>");
}

#[test]
fn st21_10_inherited_effect_resets_next_turn() {
    let (mut runner, carrier) = attack_setup(&["HAND-A", "HAND-B"]);
    runner.attack_player(carrier, 1, false);
    let pick = PLAY_HAND_START + hand_index(&runner, "HAND-A") as u16;
    runner.execute_action(0, pick).expect("trash HAND-A");
    let _ = runner.auto_resolve();

    runner.end_turn();
    let _ = runner.auto_resolve_declining_end_of_turn_attacks();
    runner.end_turn();
    let _ = runner.auto_resolve_declining_end_of_turn_attacks();
    assert_eq!(runner.turn_player(), 0);
    runner.game.players[0].battle_area[carrier.index as usize].is_suspended = false;

    runner.attack_player(carrier, 1, false);
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Hand),
        "the lockout clears on my next turn"
    );
}

#[test]
fn st21_10_effect_is_inherited_only_and_does_not_fire_from_the_top_card() {
    let mut runner = builder()
        .hand(0, &["HAND-A"])
        .security(1, &["SEC", "SEC", "SEC"])
        .start();
    runner.game.turn_count = 1;
    let gabumon = runner.place_on_field(0, CARD_ID, Some(0));
    let deck_before = runner.deck_size(0);
    runner.attack_player(gabumon, 1, false);
    assert_ne!(
        runner.pending_kind(),
        Some(SelectionKind::Hand),
        "an inherited effect does not work while Gabumon is the top card"
    );
    let _ = runner.auto_resolve();
    assert_eq!(runner.deck_size(0), deck_before, "no <Draw 1> from the top card");
}

#[test]
fn st21_10_carrier_without_gabumon_has_no_attack_draw() {
    let mut runner = builder()
        .hand(0, &["HAND-A"])
        .security(1, &["SEC", "SEC", "SEC"])
        .start();
    runner.game.turn_count = 1;
    let carrier = runner.place_on_field(0, "CARRIER", Some(0));
    let deck_before = runner.deck_size(0);
    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();
    assert_eq!(runner.deck_size(0), deck_before);
}
