//! Source: `data/card_bundles/BT17-015.md` (printed text and official Q&A).
//!
//! Standard digivolve cost circles:
//! Red Lv.5 / cost 3
//!
//! Special digivolution condition:
//! [Digivolve] Lv.5 w/[Greymon] in name: Cost 3
//!
//! Effect:
//! When this card would be played, if you have a Tamer with [Tai Kamiya] in its name, reduce the play cost by 3. [On Play] [When Digivolving] Activate 1 of the effects below: ・Delete 1 of your opponent's Digimon with 8000 DP or less. ・1 of your [Gabumon] may digivolve into [MetalGarurumon] in the hand, ignoring its digivolution requirements and without paying the cost.
//!
//! Inherited Effect:
//! [When Attacking] [Once Per Turn] If this Digimon has [Omnimon] in its name, trash the top card of your opponent's security stack.
//!
//! Official Q&A:
//! Yes, you can. You can activate the effect when there is no target, but the effect will end without anything happening.

use digimon_engine::action::space::{encode_attack, encode_digivolve, PASS, PLAY_HAND_START};
use digimon_engine::build_action_mask;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase};
use digimon_engine::selection::SelectionKind;

const CARD: &str = "BT17-015";

fn text_digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card(id, name);
    card.level = Some(level);
    card.dp = Some(dp);
    card
}

fn text_runner() -> DebugRunner {
    let mut wargreymon = text_digimon(CARD, "WarGreymon", 6, 11000);
    wargreymon.play_cost = 11;
    wargreymon.traits = vec!["Dragonkin".into()];
    wargreymon.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 5,
        memory_cost: 3,
    }];
    let mut greymon = text_digimon("TEXT-GREY5", "MetalGreymon", 5, 6000);
    greymon.colors = vec![CardColor::Blue];
    let mut tai = make_test_card("TEXT-TAI", "Tai Kamiya & Kari Kamiya");
    tai.card_kind = CardKind::Tamer;
    tai.level = None;
    tai.dp = None;
    let mut other_tamer = tai.clone();
    other_tamer.card_id = "TEXT-TAMER".into();
    other_tamer.card_name = "Matt Ishida".into();
    let mut named_digimon = text_digimon("TEXT-NAMED-DIGIMON", "Tai Kamiya", 3, 2000);
    named_digimon.colors = vec![CardColor::Blue];
    let mut gabumon = text_digimon("TEXT-GABU", "Gabumon", 3, 2000);
    gabumon.colors = vec![CardColor::Blue];
    let mut metal = text_digimon("TEXT-METAL", "MetalGarurumon", 6, 11000);
    metal.colors = vec![CardColor::White];
    metal.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 5,
        memory_cost: 9,
    }];
    let mut r = DebugRunner::builder()
        .add_card(wargreymon)
        .dsl_card(CARD)
        .unwrap()
        .add_card(tai)
        .add_card(other_tamer)
        .add_card(named_digimon)
        .add_card(gabumon)
        .add_card(metal)
        .add_card(text_digimon("TEXT-OTHER-HAND", "MetalGreymon", 6, 11000))
        .add_card(text_digimon("TEXT-RED5", "Test Ultimate", 5, 6000))
        .add_card(greymon)
        .add_card(text_digimon("TEXT-LOW", "Low DP", 4, 7999))
        .add_card(text_digimon("TEXT-BOUNDARY", "Boundary DP", 4, 8000))
        .add_card(text_digimon("TEXT-HIGH", "High DP", 4, 8001))
        .add_card(text_digimon("TEXT-OMNI", "Omnimon Alter-S", 7, 15000))
        .add_card(text_digimon("TEXT-NOT-OMNI", "Test Mega", 6, 15000))
        .add_card(text_digimon("TEXT-FILLER", "Filler", 3, 1000))
        .hand(0, &[CARD, "TEXT-METAL", "TEXT-OTHER-HAND"])
        .deck(0, &["TEXT-FILLER"; 30])
        .deck(1, &["TEXT-FILLER"; 30])
        .security(1, &["TEXT-LOW", "TEXT-BOUNDARY", "TEXT-HIGH"])
        .memory(10)
        .start();
    r.game.enter_main_phase();
    r
}

fn text_action(r: &mut DebugRunner, player: u8, action: u16) {
    assert!(
        build_action_mask(&r.game, player)[action as usize] > 0.5,
        "illegal action {action}, phase {:?}, prompt {:?}",
        r.current_phase(),
        r.pending_selection_view()
    );
    r.game.decode_action(action, player);
}

fn text_candidates(r: &DebugRunner) -> Vec<String> {
    let mut ids: Vec<_> = r
        .prompt_card_candidates()
        .into_iter()
        .map(|(_, id)| id)
        .collect();
    ids.sort();
    ids
}

fn text_activate(r: &mut DebugRunner, digivolving: bool) {
    if digivolving {
        let base = r.place_on_field(0, "TEXT-RED5", Some(0));
        let before = r.memory();
        text_action(r, 0, encode_digivolve(0, base.index as u16));
        assert_eq!(before - r.memory(), 3);
    } else {
        text_action(r, 0, PLAY_HAND_START);
    }
    r.assert_prompt(SelectionKind::EffectChoice, false, 0);
}

#[test]
fn when_played_tai_kamiya_tamer_reduces_play_cost_by_three() {
    for (support, paid) in [
        (None, 11),
        (Some("TEXT-TAI"), 8),
        (Some("TEXT-TAMER"), 11),
        (Some("TEXT-NAMED-DIGIMON"), 11),
    ] {
        let mut r = text_runner();
        if let Some(card) = support {
            r.place_on_field(0, card, Some(0));
        }
        let before = r.memory();
        text_activate(&mut r, false);
        assert_eq!(before - r.memory(), paid, "support {support:?}");
        r.execute_branch(0).unwrap();
        assert!(r.pending_selection().is_none());
    }
}

#[test]
fn special_digivolve_from_level_five_with_greymon_in_name_costs_three() {
    let mut r = text_runner();
    let base = r.place_on_field(0, "TEXT-GREY5", Some(0));
    let before = r.memory();
    text_action(&mut r, 0, encode_digivolve(0, base.index as u16));
    assert_eq!(before - r.memory(), 3);
    r.assert_prompt(SelectionKind::EffectChoice, false, 0);
    r.execute_branch(0).unwrap();
    assert!(r.pending_selection().is_none());
}

fn text_delete_clause(digivolving: bool) {
    let mut r = text_runner();
    r.place_on_field(0, "TEXT-TAI", Some(0));
    r.place_on_field(0, "TEXT-LOW", Some(0));
    for card in ["TEXT-LOW", "TEXT-BOUNDARY", "TEXT-HIGH", "TEXT-TAMER"] {
        r.place_on_field(1, card, Some(0));
    }
    text_activate(&mut r, digivolving);
    r.execute_branch(0).unwrap();
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(text_candidates(&r), vec!["TEXT-BOUNDARY", "TEXT-LOW"]);
    r.choose_prompt_card("TEXT-BOUNDARY", None).unwrap();
    assert!(r.pending_selection().is_none());
    assert!(r.game.players[1]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "TEXT-BOUNDARY"));
    assert_eq!(r.game.players[1].battle_area.len(), 3);
    assert!(r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "TEXT-LOW"));
}

#[test]
fn on_play_delete_one_opponents_digimon_with_8000_dp_or_less() {
    text_delete_clause(false);
}

#[test]
fn when_digivolving_delete_one_opponents_digimon_with_8000_dp_or_less() {
    text_delete_clause(true);
}

fn text_gabumon_clause(digivolving: bool, decline: bool) {
    let mut r = text_runner();
    r.place_on_field(0, "TEXT-TAI", Some(0));
    let gabumon = r.place_on_field(0, "TEXT-GABU", Some(0));
    r.place_on_field(0, "TEXT-LOW", Some(0));
    r.place_on_field(1, "TEXT-GABU", Some(0));
    text_activate(&mut r, digivolving);
    let memory = r.memory();
    let hand_size = r.hand_size(0);
    let deck_size = r.deck_size(0);
    r.execute_branch(1).unwrap();
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    assert_eq!(text_candidates(&r), vec!["TEXT-GABU"]);
    r.choose_prompt_card("TEXT-GABU", None).unwrap();
    r.assert_prompt(SelectionKind::Hand, true, 0);
    assert_eq!(text_candidates(&r), vec!["TEXT-METAL"]);
    if decline {
        r.decline_prompt().unwrap();
        assert_eq!(
            r.game.players[0].battle_area[gabumon.index as usize]
                .top_card()
                .card_id(&r.game.card_data),
            "TEXT-GABU"
        );
        assert_eq!(r.hand_size(0), hand_size);
        assert_eq!(r.deck_size(0), deck_size);
    } else {
        r.choose_prompt_card("TEXT-METAL", None).unwrap();
        assert_eq!(
            r.game.players[0].battle_area[gabumon.index as usize]
                .top_card()
                .card_id(&r.game.card_data),
            "TEXT-METAL"
        );
        assert_eq!(r.deck_size(0), deck_size - 1);
        assert_eq!(r.hand_size(0), hand_size);
    }
    assert!(r.pending_selection().is_none());
    assert_eq!(r.memory(), memory, "effect digivolution is free");
}

#[test]
fn on_play_gabumon_may_digivolve_into_metalgarurumon_ignoring_requirements_for_free() {
    text_gabumon_clause(false, false);
}

#[test]
fn when_digivolving_gabumon_may_digivolve_into_metalgarurumon_ignoring_requirements_for_free() {
    text_gabumon_clause(true, false);
}

#[test]
fn on_play_gabumon_may_decline_digivolving() {
    text_gabumon_clause(false, true);
}

#[test]
fn when_digivolving_gabumon_may_decline_digivolving() {
    text_gabumon_clause(true, true);
}

fn text_no_target(digivolving: bool, branch: usize) {
    let mut r = text_runner();
    r.place_on_field(0, "TEXT-TAI", Some(0));
    r.place_on_field(1, "TEXT-HIGH", Some(0));
    text_activate(&mut r, digivolving);
    let memory = r.memory();
    let hand = r.hand_size(0);
    let security = r.game.players[1].security.len();
    r.execute_branch(branch).unwrap();
    assert!(
        r.pending_selection().is_none(),
        "no-target effect must end: {:?}",
        r.pending_selection_view()
    );
    assert_eq!(r.memory(), memory);
    assert_eq!(r.hand_size(0), hand);
    assert_eq!(r.game.players[1].battle_area.len(), 1);
    assert_eq!(r.game.players[1].security.len(), security);
}

#[test]
fn official_qa_on_play_delete_with_no_target() {
    text_no_target(false, 0);
}

#[test]
fn official_qa_when_digivolving_delete_with_no_target() {
    text_no_target(true, 0);
}

#[test]
fn official_qa_on_play_digivolve_with_no_target() {
    text_no_target(false, 1);
}

#[test]
fn official_qa_when_digivolving_digivolve_with_no_target() {
    text_no_target(true, 1);
}

fn text_attack(r: &mut DebugRunner, attacker: u8) {
    let target = r.place_on_field(1, "TEXT-FILLER", Some(0));
    r.game.players[1].battle_area[target.index as usize].is_suspended = true;
    text_action(r, 0, encode_attack(attacker as u16, target.index as u16));
    assert!(r.pending_selection().is_none());
}

#[test]
fn inherited_when_attacking_with_omnimon_in_name_trashes_top_security_once_per_turn() {
    let mut r = text_runner();
    let attacker = r.place_stack(0, &[CARD, "TEXT-OMNI"]);
    text_attack(&mut r, attacker.index);
    assert_eq!(r.game.players[1].security.len(), 2);
    assert!(r.game.players[1]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "TEXT-HIGH"));
    // Readiness only; both attacks use the legal action surface.
    r.game.players[0].battle_area[attacker.index as usize].is_suspended = false;
    text_attack(&mut r, attacker.index);
    assert_eq!(r.game.players[1].security.len(), 2);

    text_action(&mut r, 0, PASS);
    assert_eq!(r.turn_player(), 1);
    assert_eq!(r.current_phase(), GamePhase::Main);
    text_action(&mut r, 1, PASS);
    assert_eq!(r.turn_player(), 0);
    assert_eq!(r.current_phase(), GamePhase::Main);
    text_attack(&mut r, attacker.index);
    assert_eq!(r.game.players[1].security.len(), 1);
    assert!(r.game.players[1]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "TEXT-BOUNDARY"));
}

#[test]
fn inherited_when_attacking_without_omnimon_in_name_does_not_trash_security() {
    let mut r = text_runner();
    let attacker = r.place_stack(0, &[CARD, "TEXT-NOT-OMNI"]);
    text_attack(&mut r, attacker.index);
    assert_eq!(r.game.players[1].security.len(), 3);
    assert!(!r.game.players[1]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "TEXT-HIGH"));
}
