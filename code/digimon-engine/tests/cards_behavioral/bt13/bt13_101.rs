//! BT13-101 Miki Kurosaki & Megumi Shirakawa — Tamer, Black/Yellow, Cost 4.
//!
//! Printed text from `data/cards.json`:
//!
//! [On Play] You may play 1 Digimon card with [PawnChessmon] in its name from
//! your hand without paying its cost.
//!
//! [All Turns] When you play a 2-color black and yellow Digimon, by suspending
//! this Tamer, <Draw 1> and gain 1 memory.
//!
//! Security Effect [Security] Play this card without paying the cost.
//!
//! Implemented YAML slices:
//! - [On Play] optional PawnChessmon hand play.
//! - [Security] play this Tamer.
//!
//! Omitted slice:
//! - [All Turns] played-Digimon observer. The engine/DSL can observe
//!   `on_enter_field_anyone`, suspend the source Tamer, draw, and gain memory,
//!   but the DSL has no event-card color predicate for "the card just played is
//!   exactly 2-color black and yellow." Name-listing PawnChessmon cards would
//!   be an approximation, so the slice is omitted until a reusable
//!   `event_card_color_only` / event-card color-count predicate exists.

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledColor, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

fn load_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT13-101")
        .expect("BT13-101 YAML loads")
        .memory(10)
        .start()
}

fn pawn_chessmon(id: &str) -> CardData {
    let mut card = make_test_card(id, "PawnChessmon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(2000);
    card.play_cost = 3;
    card.colors = vec![CardColor::Black, CardColor::Yellow];
    card
}

fn non_pawn_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, "Knightmon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(5000);
    card.play_cost = 5;
    card.colors = vec![CardColor::Black, CardColor::Yellow];
    card
}

#[test]
fn bt13_101_has_printed_metadata_and_supported_clauses() {
    let runner = load_runner();
    let compiled = runner
        .compiled_card("BT13-101")
        .expect("BT13-101 must be compiled");

    assert_eq!(compiled.name, "Miki Kurosaki & Megumi Shirakawa");
    assert_eq!(compiled.kind, CompiledCardKind::Tamer);
    assert_eq!(compiled.cost, Some(4));
    assert_eq!(
        compiled.color,
        vec![CompiledColor::Black, CompiledColor::Yellow]
    );

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .collect();

    assert!(
        triggered
            .iter()
            .any(|clause| clause.when.contains(&CompiledTiming::OnPlay)),
        "supported YAML must include the printed On Play clause"
    );
    assert!(
        triggered
            .iter()
            .any(|clause| clause.when.contains(&CompiledTiming::OnSecurity)),
        "supported YAML must include the printed Security clause"
    );
    assert!(
        triggered
            .iter()
            .all(|clause| !clause.when.contains(&CompiledTiming::OnEnterFieldAnyone)),
        "All Turns observer must be omitted until event-card color predicates exist"
    );
}

#[test]
fn bt13_101_on_play_clause_is_optional() {
    let runner = load_runner();
    let compiled = runner
        .compiled_card("BT13-101")
        .expect("BT13-101 must be compiled");
    let on_play = compiled
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnPlay) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .next()
        .expect("On Play clause must exist");

    assert!(
        on_play.optional,
        "On Play clause must be optional: 'You may'"
    );
}

#[test]
fn bt13_101_on_play_offers_only_pawnchessmon_digimon_from_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-101")
        .expect("BT13-101 YAML loads")
        .add_card(pawn_chessmon("PAWN-OK"))
        .add_card(non_pawn_digimon("NOT-PAWN"))
        .hand(0, &["BT13-101", "PAWN-OK", "NOT-PAWN"])
        .memory(10)
        .start();

    runner.play(0, 0);

    assert!(
        runner.pending_selection().is_some(),
        "playing BT13-101 should offer its optional PawnChessmon hand-play prompt"
    );
    assert!(
        runner.pending_is_optional(),
        "PawnChessmon hand-play prompt must allow PASS"
    );
    assert_eq!(
        runner.pending_action_count(),
        1,
        "only the PawnChessmon Digimon card should be selectable; non-Pawn Digimon must be filtered out"
    );
}

#[test]
fn bt13_101_on_play_can_decline_without_playing_from_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-101")
        .expect("BT13-101 YAML loads")
        .add_card(pawn_chessmon("PAWN-OK"))
        .hand(0, &["BT13-101", "PAWN-OK"])
        .memory(10)
        .start();

    runner.play(0, 0);
    let hand_before = runner.hand_size(0);
    let field_before = runner.battle_area_size(0);

    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline optional PawnChessmon play");
    runner.auto_resolve().expect("finish declined On Play");

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "declining must leave PawnChessmon in hand"
    );
    assert_eq!(
        runner.battle_area_size(0),
        field_before,
        "declining must not play another permanent"
    );
}

#[test]
fn bt13_101_on_play_plays_pawnchessmon_for_free() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-101")
        .expect("BT13-101 YAML loads")
        .add_card(pawn_chessmon("PAWN-OK"))
        .hand(0, &["BT13-101", "PAWN-OK"])
        .memory(10)
        .start();

    runner.play(0, 0);
    let memory_after_tamer_cost = runner.memory();
    let field_before = runner.battle_area_size(0);

    let action = runner
        .pending_selection_view()
        .expect("PawnChessmon selection must be pending")
        .valid_action_ids[0];
    runner
        .execute_action(0, action)
        .expect("select PawnChessmon from hand");
    runner.auto_resolve().expect("resolve PawnChessmon play");

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "PAWN-OK"),
        "selected PawnChessmon should be played to the field"
    );
    assert_eq!(
        runner.battle_area_size(0),
        field_before + 1,
        "PawnChessmon free play should add exactly one permanent"
    );
    assert_eq!(
        runner.memory(),
        memory_after_tamer_cost,
        "play_from_hand_free must not spend PawnChessmon's play cost"
    );
}

#[test]
fn bt13_101_on_play_does_not_prompt_without_pawnchessmon_in_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-101")
        .expect("BT13-101 YAML loads")
        .add_card(non_pawn_digimon("NOT-PAWN"))
        .hand(0, &["BT13-101", "NOT-PAWN"])
        .memory(10)
        .start();

    runner.play(0, 0);

    assert!(
        runner.pending_selection().is_none(),
        "On Play must not prompt when no PawnChessmon Digimon card is in hand"
    );
}

#[test]
fn bt13_101_security_plays_itself_without_paying_cost() {
    let mut attacker = make_test_card("ATTACKER", "Attacker");
    attacker.card_kind = CardKind::Digimon;
    attacker.level = Some(4);
    attacker.dp = Some(9000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-101")
        .expect("BT13-101 YAML loads")
        .add_card(attacker)
        .security(1, &["BT13-101"])
        .memory(10)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("resolve security play");

    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "BT13-101"),
        "BT13-101 should be played from the defender's security"
    );
    assert_eq!(
        runner.security_count(1),
        0,
        "security card should be consumed by the security check"
    );
}

#[test]
#[ignore = "pending: PUPPETS-G023 — OnEnterFieldAnyone observers need event-card color_only/color-count predicates and source-bound suspend-cost preflight"]
fn bt13_101_all_turns_suspends_draws_and_gains_memory_for_two_color_black_yellow_digimon() {
    // Required behavior once the reusable gap closes:
    // - trigger when controller plays a Digimon card whose colors are exactly
    //   Black + Yellow;
    // - optional activation is only available while this Tamer is unsuspended;
    // - accepting suspends this exact Tamer, draws 1, and gains 1 memory.
}

#[test]
#[ignore = "pending: PUPPETS-G023 — exact event-card color_only/color-count predicates required; name or trait filters would approximate"]
fn bt13_101_all_turns_does_not_trigger_for_single_color_or_three_color_digimon() {
    // Required negative coverage:
    // - black-only, yellow-only, and black/yellow/extra-color Digimon must not
    //   offer the activation.
}

#[test]
#[ignore = "pending: PUPPETS-G023 — observer must inspect the just-played event card and source Tamer suspend-cost availability"]
fn bt13_101_all_turns_does_not_trigger_when_tamer_is_already_suspended() {
    // Required negative coverage:
    // - even for an exact Black+Yellow two-color Digimon, a suspended
    //   BT13-101 cannot pay the suspend cost and must not install a prompt.
}
