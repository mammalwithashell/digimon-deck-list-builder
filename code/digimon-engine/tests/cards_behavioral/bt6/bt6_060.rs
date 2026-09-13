//! BT6-060 Deputymon — Digimon, Lv.4, Black, DP 4000, Cost 5. Traits: Mutant.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/BT6-060.md`;
//! the card image is not in the local mirror)
//!
//! [On Play] Reveal the top 4 cards of your deck. Add 1 Digimon card with
//! [Three Musketeers] in its type and/or 1 Option card with a memory cost of 7
//! among them to your hand. Trash the remaining cards.
//! [Your Turn] This Digimon can digivolve into a Digimon card with
//! [Three Musketeers] in its type from your hand for a memory cost of 6,
//! ignoring its digivolution requirements.
//!
//! Official Q&A: "As long as either a Digimon card with [Three Musketeers] in
//! its type or a 7-cost Option card are among the cards revealed, you can add
//! it to your hand." (each bucket is independent: 0, 1 or both may match)
//!
//! Digivolve: Black Lv.3 / cost 2.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT6/Black/BT6_060.cs
//! - OnEnterFieldAnyone / CanTriggerOnPlay → `SimplifiedRevealDeckTopCardsAndSelect`
//!   (revealCount 4, two buckets: IsDigimon && trait "Three Musketeers";
//!   IsOption && GetCostItself == 7; `RemainingCardsPlace.Trash`), outer
//!   trigger NOT optional (`SetUpActivateClass(..., -1, false, ...)`).
//! - EffectTiming.None → `AddSelfDigivolutionRequirementStaticEffect`
//!   (permanentCondition = this permanent, cardCondition = own HAND Digimon
//!   with trait "Three Musketeers", digivolutionCost 6,
//!   ignoreDigivolutionRequirement TRUE, Condition = IsOwnerTurn &&
//!   IsExistOnBattleArea).
//!
//! # Patterns this test covers
//! - A2 two-pass reveal / two-bucket select (RevealBucket flow), remainder
//!   trashed.
//! - Alt-path `direction: into` warp digivolve (ST20-10 idiom) with a
//!   `condition: { your_turn }` gate + `ignore_requirements`.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathDirection, CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledScope, CompiledTiming,
};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{encode_digivolve, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, PlaySource};
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT6-060";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn blank(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.colors = vec![CardColor::Red];
    c.level = Some(3);
    c
}

/// A [Three Musketeers]-trait Lv.5 Digimon with EMPTY evo_costs, so the only
/// way it can digivolve onto a Lv.4 Deputymon is the warp alt-path.
fn tm_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Musketeer Five");
    c.level = Some(5);
    c.dp = Some(7000);
    c.play_cost = 8;
    c.colors = vec![CardColor::Black];
    c.traits = vec!["Three Musketeers".to_string()];
    c.evo_costs = Vec::new();
    c
}

fn non_tm_digimon(id: &str) -> CardData {
    let mut c = tm_digimon(id);
    c.card_name = "Plain Five".to_string();
    c.traits = vec!["Beast".to_string()];
    c
}

fn option_card(id: &str, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.level = None;
    c.dp = None;
    c.play_cost = cost;
    c.colors = vec![CardColor::Black];
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT6-060 YAML parses, compiles and is in the embedded pack")
        .add_card(blank("BLANK-A"))
        .add_card(blank("BLANK-B"))
        .add_card(blank("BLANK-C"))
        .add_card(blank("BLANK-D"))
        .add_card(tm_digimon("TM-L5"))
        .add_card(non_tm_digimon("PLAIN-L5"))
        .add_card(option_card("OPTION-7", 7))
        .add_card(option_card("OPTION-6", 6))
}

fn hand_index(runner: &DebugRunner, player: u8, card_id: &str) -> usize {
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be in player {player}'s hand"))
}

fn hand_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn trash_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .trash
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

/// Take the first non-PASS action of the pending prompt.
fn pick_first(runner: &mut DebugRunner, label: &str) {
    let view = runner
        .pending_selection_view()
        .unwrap_or_else(|| panic!("{label}: a prompt must be pending"));
    let id = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .unwrap_or_else(|| panic!("{label}: a non-PASS pick must exist"));
    runner
        .execute_action(view.selecting_player, id)
        .unwrap_or_else(|e| panic!("{label}: pick failed: {e:?}"));
}

fn play_deputymon(runner: &mut DebugRunner) -> usize {
    runner.skip_mulligan();
    let idx = hand_index(runner, 0, CARD_ID);
    runner.play(0, idx).expect("Deputymon plays from hand")
}

// ─── Section 1 — Structural ───────────────────────────────────────────────────

#[test]
fn bt6_060_metadata_matches_printed_card() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");
    assert_eq!(card.name, "Deputymon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(5));
    assert_eq!(card.dp, Some(4000));
    assert_eq!(card.color, vec![CompiledColor::Black]);
    assert!(card.traits.iter().any(|t| t == "Mutant"));
}

#[test]
fn bt6_060_has_one_mandatory_on_play_clause() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1, "one triggered clause ([On Play])");
    let on_play = triggered[0];
    assert_eq!(on_play.when, vec![CompiledTiming::OnPlay]);
    assert_eq!(on_play.scope, CompiledScope::FaceUp);
    assert!(!on_play.optional, "printed text has no 'you may' — mandatory");
    assert!(!on_play.once_per_turn);
}

#[test]
fn bt6_060_has_standard_circle_and_your_turn_warp_alt_path() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");
    assert_eq!(card.alt_paths.len(), 2, "printed circle + [Your Turn] warp route");

    let circle = &card.alt_paths[0];
    assert_eq!(circle.kind, CompiledAltPathKind::Digivolve);
    assert_eq!(circle.direction, CompiledAltPathDirection::From);
    assert_eq!(circle.cost, Some(CompiledCost::Literal(2)));
    assert!(!circle.ignore_requirements);
    let from = circle.from.as_ref().expect("bare gate");
    assert_eq!(from.level_eq, Some(3));

    let warp = &card.alt_paths[1];
    assert_eq!(warp.kind, CompiledAltPathKind::Digivolve);
    assert_eq!(
        warp.direction,
        CompiledAltPathDirection::Into,
        "'this Digimon can digivolve INTO ... from your hand' lives on the source"
    );
    assert_eq!(warp.cost, Some(CompiledCost::Literal(6)));
    assert!(warp.ignore_requirements, "ignoring its digivolution requirements");
    assert!(
        warp.condition.is_some(),
        "[Your Turn] gate must be authored as the alt-path condition"
    );
}

// ─── Section 3 — [On Play] reveal 4 ──────────────────────────────────────────

#[test]
fn bt6_060_on_play_adds_musketeer_digimon_and_7_cost_option_then_trashes_rest() {
    // Deck order: last = top. Reveal = TM-L5, OPTION-7, BLANK-B, BLANK-A.
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["BLANK-A", "BLANK-B", "OPTION-7", "TM-L5"])
        .memory(10)
        .start();
    play_deputymon(&mut runner);

    match runner.pending_kind() {
        Some(SelectionKind::RevealBucket { .. }) => {}
        other => panic!("expected the two-bucket reveal prompt, got {other:?}"),
    }
    pick_first(&mut runner, "bucket 1: Three Musketeers Digimon");
    pick_first(&mut runner, "bucket 2: 7-cost Option");
    let _ = runner.auto_resolve();

    let hand = hand_ids(&runner, 0);
    assert!(hand.iter().any(|c| c == "TM-L5"), "TM Digimon added to hand: {hand:?}");
    assert!(hand.iter().any(|c| c == "OPTION-7"), "7-cost Option added to hand: {hand:?}");
    assert_eq!(hand.len(), 2);

    let mut trash = trash_ids(&runner, 0);
    trash.sort();
    assert_eq!(
        trash,
        vec!["BLANK-A".to_string(), "BLANK-B".to_string()],
        "the two non-matching cards are TRASHED (not bottom-decked)"
    );
    assert_eq!(runner.deck_size(0), 0, "all 4 revealed cards left the deck");
}

#[test]
fn bt6_060_on_play_with_no_matching_cards_trashes_all_four_without_prompt() {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["BLANK-A", "BLANK-B", "BLANK-C", "BLANK-D"])
        .memory(10)
        .start();
    play_deputymon(&mut runner);

    assert!(
        runner.pending_selection().is_none(),
        "nothing matches either bucket — no prompt may install"
    );
    assert_eq!(runner.hand_size(0), 0);
    assert_eq!(runner.trash_size(0), 4, "all 4 revealed cards are trashed");
    assert_eq!(runner.deck_size(0), 0);
}

#[test]
fn bt6_060_on_play_6_cost_option_is_not_eligible() {
    // NEGATIVE: the Option bucket requires a memory cost of exactly 7.
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["BLANK-A", "BLANK-B", "BLANK-C", "OPTION-6"])
        .memory(10)
        .start();
    play_deputymon(&mut runner);

    assert!(runner.pending_selection().is_none(), "a 6-cost Option must not be offered");
    assert_eq!(runner.hand_size(0), 0);
    assert!(trash_ids(&runner, 0).iter().any(|c| c == "OPTION-6"));
}

#[test]
fn bt6_060_on_play_non_musketeer_digimon_is_not_eligible() {
    // NEGATIVE: the Digimon bucket requires the [Three Musketeers] trait.
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["BLANK-A", "BLANK-B", "BLANK-C", "PLAIN-L5"])
        .memory(10)
        .start();
    play_deputymon(&mut runner);

    assert!(runner.pending_selection().is_none());
    assert_eq!(runner.hand_size(0), 0);
    assert!(trash_ids(&runner, 0).iter().any(|c| c == "PLAIN-L5"));
}

#[test]
fn bt6_060_on_play_only_option_matches_offers_only_that_bucket() {
    // Q&A: "and/or" — a lone match is still added.
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["BLANK-A", "BLANK-B", "OPTION-7", "BLANK-C"])
        .memory(10)
        .start();
    play_deputymon(&mut runner);

    let view = runner.pending_selection_view().expect("Option bucket prompt");
    let non_pass = view.valid_action_ids.iter().filter(|&&a| a != PASS).count();
    assert_eq!(non_pass, 1, "exactly the 7-cost Option is selectable");
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "a present match is mandatory (no 'you may' on the card)"
    );
    pick_first(&mut runner, "Option bucket");
    let _ = runner.auto_resolve();

    assert_eq!(hand_ids(&runner, 0), vec!["OPTION-7".to_string()]);
    assert_eq!(runner.trash_size(0), 3);
}

#[test]
fn bt6_060_on_play_only_musketeer_matches_offers_only_that_bucket() {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["TM-L5", "BLANK-A", "BLANK-B", "BLANK-C"])
        .memory(10)
        .start();
    play_deputymon(&mut runner);

    let view = runner.pending_selection_view().expect("TM Digimon bucket prompt");
    let non_pass = view.valid_action_ids.iter().filter(|&&a| a != PASS).count();
    assert_eq!(non_pass, 1);
    pick_first(&mut runner, "TM bucket");
    let _ = runner.auto_resolve();

    assert_eq!(hand_ids(&runner, 0), vec!["TM-L5".to_string()]);
    assert_eq!(runner.trash_size(0), 3);
}

#[test]
fn bt6_060_on_play_reveals_from_the_top_of_the_deck() {
    // Only the top 4 are revealed; the 5th (bottom) card stays in the deck.
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["TM-L5", "BLANK-A", "BLANK-B", "BLANK-C", "BLANK-D"])
        .memory(10)
        .start();
    play_deputymon(&mut runner);

    assert!(
        runner.pending_selection().is_none(),
        "TM-L5 sits at the deck BOTTOM (5th card) and must not be revealed"
    );
    assert_eq!(runner.deck_size(0), 1);
    assert_eq!(
        runner.game.players[0].deck[0].card_id(&runner.game.card_data),
        "TM-L5"
    );
    assert_eq!(runner.trash_size(0), 4);
}

// ─── Section 3 — [Your Turn] warp digivolve into a [Three Musketeers] Digimon ─

#[test]
fn bt6_060_warps_into_musketeer_digimon_from_hand_for_6_ignoring_requirements() {
    let mut runner = builder().hand(0, &["TM-L5"]).memory(10).start();
    let deputy = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.current_phase = GamePhase::Main;
    let memory_before = runner.game.memory;

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[encode_digivolve(0, deputy.index as u16) as usize],
        1.0,
        "the warp route must be exposed in the action mask"
    );

    assert!(
        runner
            .game
            .digivolve_from_hand(0, 0, deputy.index as usize, PlaySource::ByHand),
        "Lv.4 Deputymon → Lv.5 TM Digimon with no evo_costs proves the warp route"
    );
    let top = runner.game.players[0].battle_area[deputy.index as usize].top_card();
    assert_eq!(top.card_id(&runner.game.card_data), "TM-L5");
    assert_eq!(memory_before - runner.game.memory, 6, "warp costs exactly 6");
}

#[test]
fn bt6_060_warp_route_rejects_non_musketeer_hand_digimon() {
    // NEGATIVE: the destination must carry the [Three Musketeers] trait.
    let mut runner = builder().hand(0, &["PLAIN-L5"]).memory(10).start();
    let deputy = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.current_phase = GamePhase::Main;

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[encode_digivolve(0, deputy.index as u16) as usize], 0.0);
    assert!(!runner
        .game
        .digivolve_from_hand(0, 0, deputy.index as usize, PlaySource::ByHand));
}

#[test]
fn bt6_060_warp_route_is_gated_to_your_turn() {
    // NEGATIVE: [Your Turn] — on the opponent's turn the route does not exist.
    let mut runner = builder().hand(0, &["TM-L5"]).memory(10).start();
    let deputy = runner.place_on_field(0, CARD_ID, Some(0));
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1);
    runner.game.current_phase = GamePhase::Main;

    assert!(
        !runner
            .game
            .digivolve_from_hand(0, 0, deputy.index as usize, PlaySource::ByHand),
        "the warp route is unavailable on the opponent's turn"
    );
    assert_eq!(runner.hand_size(0), 1, "TM-L5 stays in hand");
}
