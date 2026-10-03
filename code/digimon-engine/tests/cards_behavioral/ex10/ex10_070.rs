//! EX10-070 God Grade Unleashed — Option, Black, Cost 2, Traits: Appmon, Leviathan.
//!
//! # Card text (official Bandai DB — data/card_bundles/EX10-070.md, card image)
//!
//! While you have a Digimon or Tamer with the [Appmon] trait on the field, you
//! can ignore this card's color requirements.
//! **[Main]** ＜Draw 1＞. Then, place this card in the battle area.
//! **[All Turns]** When effects trash any of your Digimon's link cards,
//! ＜Delay＞.
//! ・You may link 1 Digimon card with the [Appmon] trait from your trash to 1
//! of those Digimon without paying the cost.
//! **[Security]** Place this card in the battle area.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX10/Black/EX10_070.cs
//!
//! # Patterns
//! - Appmon option color bypass (use_requirement + IgnoreColorRequirement).
//! - [Main] draw then place self in the battle area.
//! - [Security] place self in the battle area.
//! - Event-gated <Delay> on "effects trash your Digimon's link cards"
//!   (`trigger: on_link_card_trashed`), linking to the event host ("those
//!   Digimon") via `host_filter: { is_event_host: true }`.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::CompiledCardKind;
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{PASS, PLAY_HAND_START, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX10-070";

fn digimon(id: &str, color: CardColor, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card.level = Some(4);
    card.dp = Some(4000);
    card.play_cost = 4;
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

fn delay_perm_state(runner: &DebugRunner, player: u8) -> Option<OptionState> {
    runner
        .game
        .player(player)
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .map(|p| p.option_state.clone())
}

fn base_builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX10-070 YAML loads")
        .add_card(digimon("APP-RED", CardColor::Red, &["Appmon"]))
        .add_card(digimon("APP-HOST", CardColor::Black, &["Appmon"]))
        .add_card(digimon("APP-OTHER", CardColor::Black, &["Appmon"]))
        .add_card(digimon("APP-LINKED", CardColor::Black, &["Appmon"]))
        .add_card(digimon("APP-LINK", CardColor::Black, &["Appmon"]))
        .add_card(digimon("RED-PLAIN", CardColor::Red, &["Beast"]))
        .add_card(digimon("PLAIN", CardColor::Black, &["Beast"]))
        .add_card(appmon_tamer("APP-TAMER"))
        .add_card(make_test_card("FILL", "Fill"))
}

fn play_from_hand(runner: &mut DebugRunner) {
    runner.game.enter_main_phase();
    let slot = hand_ids(runner, 0)
        .iter()
        .position(|c| c == CARD_ID)
        .expect("in hand");
    let _ = runner.game.play_option_from_hand(0, slot);
    runner.auto_resolve().expect("resolve main");
}

fn playable(runner: &mut DebugRunner) -> bool {
    runner.game.enter_main_phase();
    let slot = hand_ids(runner, 0)
        .iter()
        .position(|c| c == CARD_ID)
        .expect("in hand");
    build_action_mask(&runner.game, 0)[PLAY_HAND_START as usize + slot] == 1.0
}

// ── §1 Structural ──────────────────────────────────────────────────────────

#[test]
fn ex10_070_is_black_appmon_option_cost_2() {
    let runner = base_builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.cost, Some(2));
    assert!(
        card.use_requirement.is_some(),
        "Appmon color-bypass use requirement"
    );
}

// ── §2 Color requirement gating ───────────────────────────────────────────

#[test]
fn ex10_070_usable_off_color_with_appmon_digimon() {
    let mut runner = base_builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 3])
        .memory(5)
        .start();
    runner.place_on_field(0, "APP-RED", Some(0));
    assert!(playable(&mut runner));
}

#[test]
fn ex10_070_usable_off_color_with_appmon_tamer() {
    let mut runner = base_builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 3])
        .memory(5)
        .start();
    runner.place_on_field(0, "APP-TAMER", Some(0));
    assert!(playable(&mut runner));
}

#[test]
fn ex10_070_not_usable_off_color_without_appmon() {
    let mut runner = base_builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 3])
        .memory(5)
        .start();
    runner.place_on_field(0, "RED-PLAIN", Some(0));
    assert!(!playable(&mut runner));
}

// ── §3 [Main] ──────────────────────────────────────────────────────────────

#[test]
fn ex10_070_main_draws_one_then_places_self_in_battle_area() {
    let mut runner = base_builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 3])
        .deck(1, &["FILL"; 3])
        .memory(5)
        .start();
    runner.place_on_field(0, "APP-HOST", Some(0));
    play_from_hand(&mut runner);
    assert_eq!(hand_ids(&runner, 0), vec!["FILL"], "<Draw 1>");
    assert!(
        matches!(
            delay_perm_state(&runner, 0),
            Some(OptionState::Delayed { owner: 0, .. })
        ),
        "placed in the battle area as a <Delay> Option"
    );
    assert!(!trash_ids(&runner, 0).contains(&CARD_ID.to_string()));
}

// ── §4 [Security] ──────────────────────────────────────────────────────────

#[test]
fn ex10_070_security_places_self_in_battle_area() {
    let mut runner = base_builder()
        .security(1, &[CARD_ID])
        .deck(0, &["FILL"; 3])
        .deck(1, &["FILL"; 3])
        .memory(5)
        .start();
    let attacker = runner.place_on_field(0, "PLAIN", Some(0));
    let result = runner.attack_player(attacker, 1, false);
    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert!(!trash_ids(&runner, 1).contains(&CARD_ID.to_string()));
    assert!(matches!(
        delay_perm_state(&runner, 1),
        Some(OptionState::Delayed { owner: 1, .. })
    ));
}

// ── §5 <Delay> ───────────────────────────────────────────────────

/// Stage: EX10-070 placed (via [Main]) with an own Appmon host carrying a link
/// card, plus a second own Digimon; advance past the placing turn; trash the
/// host's link card BY AN EFFECT.
fn stage_link_trash() -> (DebugRunner, digimon_engine::PermanentHandle) {
    let mut runner = base_builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    let host = runner.place_on_field(0, "APP-HOST", Some(0));
    runner.place_on_field(0, "APP-OTHER", Some(0));
    let linked = runner.push_linked_owned(host, "APP-LINKED", 0);
    play_from_hand(&mut runner);
    runner.end_turn();
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.inject_trash(0, "APP-LINK");
    runner.inject_trash(0, "PLAIN");
    assert!(runner.game.trash_specific_link_card(host, linked));
    (runner, host)
}

#[test]
fn ex10_070_delay_links_appmon_from_trash_to_that_host_free() {
    let (mut runner, host) = stage_link_trash();
    let view = runner
        .pending_selection_view()
        .expect("effect-trashed link card offers the optional <Delay>");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert!(runner.pending_is_optional(), "16-16-2");
    runner
        .execute_action(view.selecting_player, REPLACEMENT_ACCEPT)
        .expect("accept");
    let view = runner.pending_selection_view().expect("trash pick");
    let picks: Vec<u16> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|a| *a != PASS)
        .collect();
    // APP-LINK and APP-LINKED (both Appmon Digimon) are in trash; PLAIN isn't.
    assert_eq!(picks.len(), 2);
    runner.execute_action(0, picks[0]).expect("pick");
    if let Some(view) = runner.pending_selection_view() {
        let hosts: Vec<u16> = view
            .valid_action_ids
            .iter()
            .copied()
            .filter(|a| *a != PASS)
            .collect();
        assert_eq!(
            hosts.len(),
            1,
            "only 'those Digimon' — the link card's host"
        );
        runner.execute_action(0, hosts[0]).expect("host");
    }
    runner.auto_resolve().expect("settle");
    let host_perm = &runner.game.player(0).battle_area[host.index as usize];
    assert_eq!(host_perm.linked_cards.len(), 1);
    assert!(trash_ids(&runner, 0).contains(&CARD_ID.to_string()));
}

#[test]
fn ex10_070_delay_offered_after_effect_trashes_link_card() {
    let (runner, _host) = stage_link_trash();
    assert!(
        runner.pending_selection_view().is_some(),
        "an effect trashing an own Digimon's link card opens the <Delay>"
    );
}

/// "YOUR Digimon's link cards": trashing an OPPONENT Digimon's link card does
/// not open the <Delay>.
#[test]
fn ex10_070_delay_not_offered_for_opponents_link_card() {
    let mut runner = base_builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    runner.place_on_field(0, "APP-HOST", Some(0));
    let opp_host = runner.place_on_field(1, "APP-OTHER", Some(0));
    let linked = runner.push_linked_owned(opp_host, "APP-LINKED", 1);
    play_from_hand(&mut runner);
    runner.end_turn();
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.inject_trash(0, "APP-LINK");
    assert!(runner.game.trash_specific_link_card(opp_host, linked));
    assert!(runner.pending_selection_view().is_none());
    assert!(delay_perm_state(&runner, 0).is_some(), "stays parked");
}

/// A host LEAVING the field sends its link cards to the trash by rule, not
/// "effects trash … link cards" — and there are no "those Digimon" to link to.
#[test]
fn ex10_070_delay_not_offered_when_host_is_deleted() {
    let mut runner = base_builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    let host = runner.place_on_field(0, "APP-HOST", Some(0));
    runner.place_on_field(0, "APP-OTHER", Some(0));
    runner.push_linked_owned(host, "APP-LINKED", 0);
    play_from_hand(&mut runner);
    runner.end_turn();
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.inject_trash(0, "APP-LINK");
    runner.game.delete_permanent_with_cause(
        host,
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );
    let _ = runner.auto_resolve();
    assert!(delay_perm_state(&runner, 0).is_some(), "stays parked");
}

/// 16-16-2: the <Delay> is optional; declining keeps EX10-070 parked.
#[test]
fn ex10_070_delay_declined_stays_parked() {
    let (mut runner, _host) = stage_link_trash();
    let view = runner.pending_selection_view().expect("activation prompt");
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline");
    let _ = runner.auto_resolve();
    assert!(delay_perm_state(&runner, 0).is_some());
}
