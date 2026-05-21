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
//! - [All Turns] 2-color black/yellow Digimon played observer with
//!   `activation_cost: { suspend_self: true }` → Draw 1 + gain 1 memory.
//!   Uses PUPPETS-G023 `event_card_color_only` / `event_card_color_count`
//!   predicates.
//! - [Security] play this Tamer.

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

/// A 2-color Black+Yellow Digimon that should trigger BT13-101.
fn two_color_black_yellow_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, "BlackYellowDigimon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(5000);
    card.play_cost = 4;
    card.colors = vec![CardColor::Black, CardColor::Yellow];
    card
}

/// A mono-color Black Digimon — should NOT trigger BT13-101.
fn mono_black_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, "MonoBlackDigimon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(5000);
    card.play_cost = 4;
    card.colors = vec![CardColor::Black];
    card
}

/// A mono-color Yellow Digimon — should NOT trigger BT13-101.
fn mono_yellow_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, "MonoYellowDigimon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(5000);
    card.play_cost = 4;
    card.colors = vec![CardColor::Yellow];
    card
}

/// A 3-color Black+Yellow+Red Digimon — should NOT trigger BT13-101.
fn three_color_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, "ThreeColorDigimon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(5000);
    card.play_cost = 5;
    card.colors = vec![CardColor::Black, CardColor::Yellow, CardColor::Red];
    card
}

/// A draw-filler card for the deck in observer tests.
fn draw_card(id: &str) -> CardData {
    let mut card = make_test_card(id, "DrawFiller");
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(1000);
    card.play_cost = 0;
    card
}

/// Build a runner for BT13-101 [All Turns] observer tests.
/// BT13-101 is placed on the field via `place_on_field` after start.
/// `hand_cards` go into P0's hand (index 0 = first card to play).
/// `deck_cards` go into P0's deck (for Draw 1 to consume).
fn observer_runner(hand_cards: &[CardData], deck_cards: &[CardData]) -> DebugRunner {
    let mut builder = DebugRunner::builder()
        .dsl_card("BT13-101")
        .expect("BT13-101 YAML loads")
        .memory(10);
    for card in hand_cards {
        builder = builder.add_card(card.clone());
    }
    for card in deck_cards {
        builder = builder.add_card(card.clone());
    }
    let hand_ids: Vec<&str> = hand_cards.iter().map(|c| c.card_id.as_str()).collect();
    let deck_ids: Vec<&str> = deck_cards.iter().map(|c| c.card_id.as_str()).collect();
    if !hand_ids.is_empty() {
        builder = builder.hand(0, &hand_ids);
    }
    if !deck_ids.is_empty() {
        builder = builder.deck(0, &deck_ids);
    }
    builder.start()
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
            .any(|clause| clause.when.contains(&CompiledTiming::OnAnyDigimonPlayed)),
        "All Turns observer must be present after PUPPETS-G023 event-card color predicates landed"
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
    // PawnChessmon is a 2-color Black+Yellow Digimon, so the [All Turns] observer
    // fires: BT13-101 suspends itself and gains +1 memory. play_from_hand_free
    // correctly does not spend PawnChessmon's play cost; the only memory delta
    // beyond the Tamer's own cost is the observer's +1 gain.
    assert_eq!(
        runner.memory(),
        memory_after_tamer_cost + 1,
        "play_from_hand_free must not spend PawnChessmon's play cost; observer adds +1 memory"
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
fn bt13_101_all_turns_suspends_draws_and_gains_memory_for_two_color_black_yellow_digimon() {
    // PUPPETS-G023: positive path — exactly Black+Yellow 2-color Digimon triggers.
    let digimon = two_color_black_yellow_digimon("BY-DIGIMON-G023");
    let draw_filler = draw_card("DRAW-G023");
    let mut runner = observer_runner(&[digimon], &[draw_filler]);
    let tamer = runner.place_on_field(0, "BT13-101", Some(0));
    let memory_before = runner.memory();

    runner.play(0, 0).expect("2-color B/Y Digimon plays from hand");
    runner.auto_resolve().expect("activation_cost + Draw 1 + gain 1 memory resolve");

    assert!(
        runner.game.player(0).battle_area[tamer.index as usize].is_suspended,
        "BT13-101 must be suspended as the activation cost"
    );
    assert_eq!(runner.hand_size(0), 1, "Draw 1 must put the deck card into hand");
    assert_eq!(runner.deck_size(0), 0, "Draw 1 must consume the deck card");
    assert_eq!(
        runner.memory(),
        memory_before - 4 + 1, // -4 for the Digimon's play cost, +1 for gain_memory
        "playing the Digimon costs 4 memory, observer adds 1"
    );
}

#[test]
fn bt13_101_all_turns_does_not_trigger_for_single_color_or_three_color_digimon() {
    // PUPPETS-G023: negative paths — mono-color and 3-color must not trigger.
    // Provide draw filler in deck to distinguish: if draw happened, hand size grows.
    let mut runner = observer_runner(
        &[
            mono_black_digimon("MONO-B-G023"),
            mono_yellow_digimon("MONO-Y-G023"),
            three_color_digimon("THREE-C-G023"),
        ],
        &[
            draw_card("DRAW1-G023"),
            draw_card("DRAW2-G023"),
            draw_card("DRAW3-G023"),
        ],
    );
    runner.place_on_field(0, "BT13-101", Some(0));

    // Play mono-black — should not trigger.
    runner.play(0, 0).expect("mono-black plays");
    runner.auto_resolve().expect("settle mono-black play");
    assert_eq!(
        runner.hand_size(0),
        2,
        "mono-black Digimon must not trigger observer (hand = remaining 2 cards)"
    );

    // Play mono-yellow — should not trigger.
    runner.play(0, 0).expect("mono-yellow plays");
    runner.auto_resolve().expect("settle mono-yellow play");
    assert_eq!(
        runner.hand_size(0),
        1,
        "mono-yellow Digimon must not trigger observer"
    );

    // Play 3-color B/Y/R — should not trigger.
    runner.play(0, 0).expect("3-color plays");
    runner.auto_resolve().expect("settle 3-color play");
    assert_eq!(
        runner.hand_size(0),
        0,
        "3-color B/Y/R Digimon must not trigger observer"
    );
    assert_eq!(
        runner.deck_size(0),
        3,
        "none of the non-qualifying Digimon must trigger Draw 1"
    );
}

#[test]
fn bt13_101_all_turns_does_not_trigger_when_tamer_is_already_suspended() {
    // PUPPETS-G023: negative path — suspended BT13-101 cannot pay the suspend
    // cost, so the observer body must not execute (no draw, no memory gain).
    let digimon = two_color_black_yellow_digimon("BY-DIGIMON-SUSP-G023");
    let draw_filler = draw_card("DRAW-SUSP-G023");
    let mut runner = observer_runner(&[digimon], &[draw_filler]);
    let tamer = runner.place_on_field(0, "BT13-101", Some(0));
    // Pre-suspend the Tamer so it cannot pay the suspend cost.
    runner.game.player_mut(0).battle_area[tamer.index as usize].is_suspended = true;
    let memory_before = runner.memory();

    runner.play(0, 0).expect("2-color B/Y Digimon plays");
    runner.auto_resolve().expect("settle with pre-suspended Tamer");

    assert!(
        runner.game.player(0).battle_area[tamer.index as usize].is_suspended,
        "pre-suspended Tamer must remain suspended"
    );
    assert_eq!(runner.hand_size(0), 0, "cost failure must not draw");
    assert_eq!(runner.deck_size(0), 1, "cost failure must not consume deck card");
    assert_eq!(
        runner.memory(),
        memory_before - 4, // only the Digimon's play cost, no gain_memory
        "cost failure must not grant the +1 memory"
    );
}
