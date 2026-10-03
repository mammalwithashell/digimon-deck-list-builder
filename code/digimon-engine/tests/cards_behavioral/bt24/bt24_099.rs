//! BT24-099 Super Hacking — Option, Purple, Cost 3, Traits: Appmon.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT24-099.md)
//!
//! While you have an [Appmon] trait Digimon or Tamer on the field, you can
//! ignore this card's color requirements.
//! **[Main]** By trashing 1 [Appmon] trait card from your hand, ＜Draw 2＞.
//! Then, place this card in the battle area.
//! **[All Turns]** When Digimon are deleted, ＜Delay＞.
//! ・You may link 1 [Appmon] trait Digimon card from your trash to 1 of your
//! Digimon without paying the cost.
//! **[Security]** Place this card in the battle area.
//!
//! Official Q&A: "If you don't trash 1 [Appmon] trait card from your hand, you
//! can't process the part after 'then' in this card's [Main] effect."
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT24/Purple/BT24_099.cs (places the card
//! unconditionally — diverges from the official Q&A, which governs here).
//!
//! # Patterns
//! - Appmon option color bypass (use_requirement + IgnoreColorRequirement).
//! - Cost-gated [Main] ("By trashing 1 ... from your hand") gating draw AND
//!   placement (EX12-071 idiom).
//! - Event-gated <Delay> on any Digimon deletion (`trigger: on_any_deletion`),
//!   optional activation (16-16-2), placing-turn lockout (16-16-3).
//! - Delay body: optional free link from trash (`link_cards from: [trash]`).
//! - [Security] place self as a Delay Option.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledDeclarativeClause};
use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_START, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming};
use digimon_engine::permanent::OptionState;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT24-099";

fn digimon(id: &str, level: u8, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Purple];
    card.level = Some(level);
    card.dp = Some(i32::from(level) * 1000);
    card.play_cost = u16::from(level);
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn appmon_tamer(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![CardColor::Red];
    card.play_cost = 3;
    card.traits = vec!["Appmon".to_string()];
    card
}

fn ids(cards: &[CardSource], runner: &DebugRunner) -> Vec<String> {
    cards
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn hand_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    ids(&runner.game.player(player).hand, runner)
}

fn trash_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    ids(&runner.game.player(player).trash, runner)
}

fn hand_index(runner: &DebugRunner, player: u8, card_id: &str) -> usize {
    hand_ids(runner, player)
        .iter()
        .position(|c| c == card_id)
        .unwrap_or_else(|| panic!("{card_id} in hand"))
}

fn delay_perm_state(runner: &DebugRunner, player: u8) -> Option<OptionState> {
    runner
        .game
        .player(player)
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .map(|p| p.option_state.clone())
}

fn non_pass(ids: &[u16]) -> Vec<u16> {
    ids.iter().copied().filter(|a| *a != PASS).collect()
}

fn hand_action(runner: &DebugRunner, ids: &[u16], slot: usize) -> u16 {
    let slot = slot as u16;
    [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
        .into_iter()
        .find(|a| ids.contains(a))
        .expect("hand slot selectable")
}

fn base_builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-099 YAML loads")
        .add_card(digimon("APP-FIELD", 5, &["Appmon"]))
        .add_card(appmon_tamer("APP-COST"))
        .add_card(digimon("APP-LINK", 4, &["Appmon"]))
        .add_card(digimon("PLAIN", 3, &["Beast"]))
        .add_card(digimon("OPP-VICTIM", 3, &["Beast"]))
        .add_card(appmon_tamer("APP-TAMER"))
        .add_card(make_test_card("FILL", "Fill"))
}

/// Play BT24-099 from hand paying the Appmon trash cost with APP-COST.
fn play_and_pay(runner: &mut DebugRunner) {
    runner.game.enter_main_phase();
    let slot = hand_index(runner, 0, CARD_ID);
    let _ = runner.game.play_option_from_hand(0, slot);
    let view = runner.pending_selection_view().expect("cost prompt");
    assert_eq!(view.kind, SelectionKind::Hand);
    let cost_slot = hand_index(runner, 0, "APP-COST");
    let action = hand_action(runner, &view.valid_action_ids, cost_slot);
    runner.execute_action(0, action).expect("pay cost");
    runner.auto_resolve().expect("resolve main");
}

/// Seat a placed Delay in the battle area (as [Main] would) and advance to
/// the owner's next turn so the placing-turn lockout is over.
fn seat_and_advance(runner: &mut DebugRunner) {
    play_and_pay(runner);
    assert!(delay_perm_state(runner, 0).is_some());
    runner.end_turn();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();
}

// ── §1 Structural ──────────────────────────────────────────────────────────

#[test]
fn bt24_099_is_purple_appmon_option_cost_3_with_color_bypass() {
    let runner = base_builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.cost, Some(3));
    assert!(card.use_requirement.is_some(), "Appmon color-bypass use requirement");
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Delay { .. })
        )),
        "carries an event-gated <Delay> clause"
    );
}

// ── §2 Color requirement gating ───────────────────────────────────────────

#[test]
fn bt24_099_usable_off_color_with_appmon_tamer() {
    let mut runner = base_builder()
        .hand(0, &[CARD_ID, "APP-COST"])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .memory(5)
        .start();
    runner.place_on_field(0, "APP-TAMER", Some(0));
    runner.game.enter_main_phase();
    let mask = digimon_engine::action::mask::build_action_mask(&runner.game, 0);
    let slot = hand_index(&runner, 0, CARD_ID);
    assert_eq!(mask[PLAY_HAND_START as usize + slot], 1.0);
}

#[test]
fn bt24_099_not_usable_off_color_without_appmon() {
    let mut runner = base_builder()
        .hand(0, &[CARD_ID, "APP-COST"])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .memory(5)
        .start();
    // Red non-Appmon Digimon only — neither purple nor Appmon.
    let mut red = digimon("RED", 3, &["Beast"]);
    red.colors = vec![CardColor::Red];
    runner.game.card_data.push(red);
    runner.place_on_field(0, "RED", Some(0));
    runner.game.enter_main_phase();
    let mask = digimon_engine::action::mask::build_action_mask(&runner.game, 0);
    let slot = hand_index(&runner, 0, CARD_ID);
    assert_eq!(mask[PLAY_HAND_START as usize + slot], 0.0);
}

// ── §3 [Main] ──────────────────────────────────────────────────────────────

#[test]
fn bt24_099_main_pays_appmon_cost_draws_two_and_places_delay() {
    let mut runner = base_builder()
        .hand(0, &[CARD_ID, "APP-COST"])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .memory(5)
        .start();
    runner.place_on_field(0, "APP-FIELD", Some(0));
    play_and_pay(&mut runner);

    assert!(trash_ids(&runner, 0).contains(&"APP-COST".to_string()));
    assert_eq!(hand_ids(&runner, 0), vec!["FILL", "FILL"], "<Draw 2>");
    match delay_perm_state(&runner, 0) {
        Some(OptionState::Delayed { trigger, .. }) => assert_eq!(
            trigger,
            DelayTrigger::OnEvent(EffectTiming::OnAnyDeletion),
            "event-gated on Digimon deletion"
        ),
        other => panic!("BT24-099 must be placed as a Delay option: {other:?}"),
    }
}

#[test]
fn bt24_099_main_cost_only_accepts_appmon_cards() {
    let mut runner = base_builder()
        .hand(0, &[CARD_ID, "APP-COST", "PLAIN"])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .memory(5)
        .start();
    runner.place_on_field(0, "APP-FIELD", Some(0));
    runner.game.enter_main_phase();
    let slot = hand_index(&runner, 0, CARD_ID);
    let _ = runner.game.play_option_from_hand(0, slot);
    let view = runner.pending_selection_view().expect("cost prompt");
    assert!(runner.pending_is_optional(), "the cost may be declined");
    assert_eq!(
        non_pass(&view.valid_action_ids).len(),
        1,
        "only the [Appmon] card is a legal cost"
    );
}

/// Official Q&A: without the trash cost, nothing after "then" happens — no
/// draw and no placement; the Option is trashed.
#[test]
fn bt24_099_main_declined_cost_no_draw() {
    let mut runner = base_builder()
        .hand(0, &[CARD_ID, "APP-COST"])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .memory(5)
        .start();
    runner.place_on_field(0, "APP-FIELD", Some(0));
    runner.game.enter_main_phase();
    let slot = hand_index(&runner, 0, CARD_ID);
    let _ = runner.game.play_option_from_hand(0, slot);
    runner.execute_action(0, PASS).expect("decline cost");
    runner.auto_resolve().expect("resolve");

    assert_eq!(hand_ids(&runner, 0), vec!["APP-COST"], "no draw");
}

#[test]
fn bt24_099_main_declined_cost_not_placed_and_option_trashed() {
    let mut runner = base_builder()
        .hand(0, &[CARD_ID, "APP-COST"])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .memory(5)
        .start();
    runner.place_on_field(0, "APP-FIELD", Some(0));
    runner.game.enter_main_phase();
    let slot = hand_index(&runner, 0, CARD_ID);
    let _ = runner.game.play_option_from_hand(0, slot);
    runner.execute_action(0, PASS).expect("decline cost");
    runner.auto_resolve().expect("resolve");

    assert_eq!(hand_ids(&runner, 0), vec!["APP-COST"], "no draw");
    assert!(delay_perm_state(&runner, 0).is_none(), "not placed (Q&A)");
    assert!(trash_ids(&runner, 0).contains(&CARD_ID.to_string()));
}

#[test]
fn bt24_099_main_without_appmon_in_hand_no_draw() {
    let mut runner = base_builder()
        .hand(0, &[CARD_ID, "PLAIN"])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .memory(5)
        .start();
    runner.place_on_field(0, "APP-FIELD", Some(0));
    runner.game.enter_main_phase();
    let slot = hand_index(&runner, 0, CARD_ID);
    let _ = runner.game.play_option_from_hand(0, slot);
    runner.auto_resolve().expect("resolve");

    assert_eq!(hand_ids(&runner, 0), vec!["PLAIN"], "no draw");
}

#[test]
fn bt24_099_main_without_appmon_in_hand_not_placed_and_option_trashed() {
    let mut runner = base_builder()
        .hand(0, &[CARD_ID, "PLAIN"])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .memory(5)
        .start();
    runner.place_on_field(0, "APP-FIELD", Some(0));
    runner.game.enter_main_phase();
    let slot = hand_index(&runner, 0, CARD_ID);
    let _ = runner.game.play_option_from_hand(0, slot);
    runner.auto_resolve().expect("resolve");

    assert_eq!(hand_ids(&runner, 0), vec!["PLAIN"], "no draw");
    assert!(delay_perm_state(&runner, 0).is_none());
    assert!(trash_ids(&runner, 0).contains(&CARD_ID.to_string()));
}

// ── §4 <Delay> ─────────────────────────────────────────────────────────────

#[test]
fn bt24_099_delay_not_offered_on_placing_turn() {
    let mut runner = base_builder()
        .hand(0, &[CARD_ID, "APP-COST"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    runner.place_on_field(0, "APP-FIELD", Some(0));
    runner.inject_trash(0, "APP-LINK");
    let victim = runner.place_on_field(1, "OPP-VICTIM", Some(1));
    play_and_pay(&mut runner);

    runner
        .game
        .delete_permanent_with_cause(victim, ReplacementCause::OwnEffect);
    assert!(
        runner.game.pending_selection.is_none(),
        "<Delay> can't activate the turn it was placed (16-16-3)"
    );
    assert!(delay_perm_state(&runner, 0).is_some());
}

#[test]
fn bt24_099_delay_on_deletion_links_appmon_from_trash_free() {
    let mut runner = base_builder()
        .hand(0, &[CARD_ID, "APP-COST"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    let host = runner.place_on_field(0, "APP-FIELD", Some(0));
    seat_and_advance(&mut runner);
    runner.inject_trash(0, "APP-LINK");
    runner.inject_trash(0, "PLAIN");
    let victim = runner.place_on_field(1, "OPP-VICTIM", Some(1));
    let memory_before = runner.game.memory;

    runner
        .game
        .delete_permanent_with_cause(victim, ReplacementCause::OwnEffect);

    let view = runner
        .pending_selection_view()
        .expect("deletion offers the optional <Delay> activation");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert!(runner.pending_is_optional(), "16-16-2");
    runner
        .execute_action(view.selecting_player, REPLACEMENT_ACCEPT)
        .expect("accept");

    let view = runner.pending_selection_view().expect("trash link pick");
    assert!(runner.pending_is_optional(), "'You may link'");
    let picks = non_pass(&view.valid_action_ids);
    assert_eq!(
        picks.len(),
        1,
        "only the [Appmon] Digimon card in trash (not the [Appmon] Tamer cost card, not the non-Appmon Digimon)"
    );
    runner.execute_action(0, picks[0]).expect("pick APP-LINK");

    if let Some(view) = runner.pending_selection_view() {
        let picks = non_pass(&view.valid_action_ids);
        assert_eq!(picks.len(), 1, "own Digimon host only");
        runner.execute_action(0, picks[0]).expect("pick host");
    }
    runner.auto_resolve().expect("settle");

    let host_perm = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "APP-FIELD")
        .expect("host");
    assert_eq!(ids(&host_perm.linked_cards, &runner), vec!["APP-LINK"]);
    let _ = host;
    assert!(trash_ids(&runner, 0).contains(&CARD_ID.to_string()), "Delay cost");
    assert!(delay_perm_state(&runner, 0).is_none());
    assert_eq!(runner.game.memory, memory_before, "without paying the cost");
}

#[test]
fn bt24_099_delay_fires_on_own_digimon_deletion_too() {
    let mut runner = base_builder()
        .hand(0, &[CARD_ID, "APP-COST"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    runner.place_on_field(0, "APP-FIELD", Some(0));
    seat_and_advance(&mut runner);
    runner.inject_trash(0, "APP-LINK");
    let own = runner.place_on_field(0, "PLAIN", Some(0));
    runner
        .game
        .delete_permanent_with_cause(own, ReplacementCause::OwnEffect);
    let view = runner
        .pending_selection_view()
        .expect("any Digimon deletion opens the <Delay>");
    assert_eq!(view.kind, SelectionKind::Replacement);
}

#[test]
fn bt24_099_delay_not_offered_on_tamer_deletion() {
    let mut runner = base_builder()
        .hand(0, &[CARD_ID, "APP-COST"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    runner.place_on_field(0, "APP-FIELD", Some(0));
    seat_and_advance(&mut runner);
    runner.inject_trash(0, "APP-LINK");
    let tamer = runner.place_on_field(1, "APP-TAMER", Some(1));
    runner
        .game
        .delete_permanent_with_cause(tamer, ReplacementCause::OwnEffect);
    assert!(
        runner.game.pending_selection.is_none(),
        "only Digimon deletions are the printed trigger"
    );
    assert!(delay_perm_state(&runner, 0).is_some());
}

#[test]
fn bt24_099_delay_declined_stays_parked() {
    let mut runner = base_builder()
        .hand(0, &[CARD_ID, "APP-COST"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    runner.place_on_field(0, "APP-FIELD", Some(0));
    seat_and_advance(&mut runner);
    runner.inject_trash(0, "APP-LINK");
    let victim = runner.place_on_field(1, "OPP-VICTIM", Some(1));
    runner
        .game
        .delete_permanent_with_cause(victim, ReplacementCause::OwnEffect);
    let view = runner.pending_selection_view().expect("activation prompt");
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline");
    runner.auto_resolve().expect("settle");
    assert!(delay_perm_state(&runner, 0).is_some(), "declined Delay stays");
    assert!(trash_ids(&runner, 0).contains(&"APP-LINK".to_string()));
}

// ── §5 [Security] ──────────────────────────────────────────────────────────

#[test]
fn bt24_099_security_places_self_in_battle_area() {
    let mut runner = base_builder()
        .security(1, &[CARD_ID])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .memory(5)
        .start();
    let attacker = runner.place_on_field(0, "PLAIN", Some(0));
    let result = runner.attack_player(attacker, 1, false);
    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert!(!trash_ids(&runner, 1).contains(&CARD_ID.to_string()));
    match delay_perm_state(&runner, 1) {
        Some(OptionState::Delayed { owner, .. }) => assert_eq!(owner, 1),
        other => panic!("placed as Delay option: {other:?}"),
    }
}
