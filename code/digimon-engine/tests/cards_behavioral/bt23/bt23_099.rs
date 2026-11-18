//! BT23-099 The Sistermon Sisters Training Gym — Option, White, Cost 2, CS.
//!
//! Official text (data/card_bundles/BT23-099.md):
//!   While you have a Digimon with [Huckmon] in its name on the field, you can
//!   ignore this card's color requirements.
//!   [Main] <Draw 1>. Then, place this card in the battle area.
//!   [Your Turn] When any of your Digimon digivolve into a Digimon with
//!   [Huckmon] or [Jesmon] in its name, <Delay>
//!   ・You may play 1 card with [Sistermon] in its name from your hand or
//!     trash without paying the cost.
//!   [Security] You may play 1 card with [Sistermon] in its name from your hand
//!   or trash without paying the cost. Then, place this card in the battle area.
//! Official Q&A: "on the field" = the battle area or the breeding area.
//!
//! DCGO: BT23/White/BT23_099.cs.
//!
//! Regression origin: a real DCGO PvP game replayed through the engine. With no
//! YAML the white Option had no colour bypass, so playing it off-colour beside a
//! [Huckmon]-named Digimon was a mask miss. Synthetic boards only.
#![allow(dead_code, unused_imports)]

use std::path::Path;

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{self, PASS, PLAY_HAND_START, REPLACEMENT_ACCEPT};
use digimon_engine::build_action_mask;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming, PlaySource};
use digimon_engine::permanent::{OptionState, PermanentHandle};
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

const CARD_ID: &str = "BT23-099";

fn yaml() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/bt23/BT23-099.yaml"))
        .expect("BT23-099 YAML must exist at cards/bt23/BT23-099.yaml")
}

fn red_digimon(id: &str, name: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = level as u16;
    c
}

/// Lv.4 evolution (free, from any colour Lv.3) so `digivolve_from_hand` succeeds.
fn free_evo(id: &str, name: &str) -> CardData {
    let mut c = red_digimon(id, name, 4);
    c.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 3,
        memory_cost: 0,
    }];
    c
}

fn tamer(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Red];
    c
}

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("BT23-099 YAML parses and compiles")
        .add_card(filler("FILL"))
        .add_card(red_digimon("BASE", "Base Digimon", 3))
        .add_card(red_digimon("HUCK-FIELD", "Super Huckmon", 3))
        .add_card(tamer("HUCK-TAMER", "Huckmon Tamer"))
        .add_card(free_evo("HUCK-EVO", "Huckmon Evolved"))
        .add_card(free_evo("JES-EVO", "Jesmon Evolved"))
        .add_card(free_evo("PLAIN-EVO", "Plain Evolved"))
        .add_card(red_digimon("SIS-HAND", "Sistermon Blanc", 3))
        .add_card(red_digimon("SIS-TRASH", "Sistermon Ciel", 3))
        .add_card(tamer("SIS-TAMER", "Sistermon Tamer"))
        .add_card(red_digimon("NOT-SIS", "Sisterhood", 3))
        .deck(0, &["FILL"; 12])
        .deck(1, &["FILL"; 12])
}

fn data_idx(r: &DebugRunner, card_id: &str) -> usize {
    r.game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"))
}

fn push_hand(r: &mut DebugRunner, p: u8, card_id: &str) {
    let idx = data_idx(r, card_id);
    let next = r.game.next_card_index();
    r.game.players[p as usize]
        .hand
        .push(CardSource::new(idx, p, next));
}

fn push_trash(r: &mut DebugRunner, p: u8, card_id: &str) {
    let idx = data_idx(r, card_id);
    let next = r.game.next_card_index();
    r.game.players[p as usize]
        .trash
        .push(CardSource::new(idx, p, next));
}

fn hand_index(r: &DebugRunner, p: u8, card_id: &str) -> usize {
    r.game.players[p as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not in hand"))
}

fn field_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn trash_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .trash
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn find_perm(r: &DebugRunner, p: u8, card_id: &str) -> PermanentHandle {
    let index = r.game.players[p as usize]
        .battle_area
        .iter()
        .position(|perm| perm.top_card().card_id(&r.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not on field"));
    PermanentHandle {
        player: p,
        index: index as u8,
    }
}

fn gym_state(r: &DebugRunner, p: u8) -> Option<OptionState> {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .find(|perm| perm.top_card().card_id(&r.game.card_data) == CARD_ID)
        .map(|perm| perm.option_state)
}

fn is_parked_on_digivolve(r: &DebugRunner, p: u8) -> bool {
    matches!(
        gym_state(r, p),
        Some(OptionState::Delayed {
            trigger: DelayTrigger::OnEvent(EffectTiming::OnDigivolve),
            ..
        })
    )
}

fn union_offered(r: &DebugRunner, owner: u8) -> Vec<(bool, String)> {
    use digimon_engine::owned_card_choice::{CardChoiceOperation, CardChoiceZone};
    let Some(pending) = r.game.pending_selection.as_ref() else {
        return Vec::new();
    };
    let Some(choice) = pending.card_choice.as_ref() else {
        return Vec::new();
    };
    let mut rows: Vec<(bool, String)> = choice
        .view(&r.game, owner)
        .rows
        .iter()
        .filter(|row| matches!(row.operation, CardChoiceOperation::CommitCard))
        .filter_map(|row| {
            let trash = matches!(row.zone, Some(CardChoiceZone::Trash));
            row.card_id.clone().map(|id| (trash, id))
        })
        .collect();
    rows.sort();
    rows
}

fn pick_union(r: &mut DebugRunner, p: u8, card_id: &str, trash: bool) {
    let action =
        owned_union_fixture::action(&r.game, p, card_id, trash).expect("eligible physical row");
    r.execute_action(p, action).unwrap();
}

fn accept_delay(r: &mut DebugRunner) {
    let v = r
        .pending_selection_view()
        .expect("<Delay> activation offered");
    assert_eq!(v.kind, SelectionKind::Replacement);
    assert!(r.pending_is_optional(), "<Delay> is optional (16-17-2)");
    r.execute_action(v.selecting_player, REPLACEMENT_ACCEPT)
        .unwrap();
}

fn advance_to_next_own_main(r: &mut DebugRunner) {
    r.end_turn();
    r.game.enter_main_phase();
    r.end_turn();
    assert_eq!(r.game.turn_player(), 0);
    r.game.enter_main_phase();
}

/// Off-colour runner: a RED permanent on the field, the WHITE Option in hand.
fn color_runner(extra_field: &[&str]) -> DebugRunner {
    let mut r = builder().hand(0, &[CARD_ID]).memory(10).start();
    r.set_first_player(0);
    r.game.enter_main_phase();
    r.place_on_field(0, "BASE", Some(0));
    for id in extra_field {
        r.place_on_field(0, id, Some(0));
    }
    r
}

/// Play the gym (via the bypass, beside a [Huckmon] Digimon), then advance to
/// the next own main phase with `hand` cards in hand.
fn parked(hand: &[&str]) -> DebugRunner {
    let mut r = builder().hand(0, &[CARD_ID]).memory(10).start();
    r.set_first_player(0);
    r.place_on_field(0, "BASE", Some(0));
    r.place_on_field(0, "HUCK-FIELD", Some(0));
    r.game.enter_main_phase();
    let _ = r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve();
    assert!(is_parked_on_digivolve(&r, 0), "gym placed as a Delay");
    for id in hand {
        push_hand(&mut r, 0, id);
    }
    advance_to_next_own_main(&mut r);
    r.game.memory = 10;
    r
}

/// Digivolve the field `BASE` into the hand card `evo_id`.
fn digivolve_base_into(r: &mut DebugRunner, evo_id: &str) {
    let base = find_perm(r, 0, "BASE");
    let hand_idx = hand_index(r, 0, evo_id);
    let ok = r
        .game
        .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByDigivolve);
    assert!(ok, "digivolve_from_hand must succeed");
    r.game.drain_effect_queue();
}

// ─── Structure ──────────────────────────────────────────────────────────────

#[test]
fn bt23_099_structure() {
    let r = builder().build();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.cost, Some(2));
    assert!(card.use_requirement.is_some(), "colour bypass is a use_requirement");
    assert_eq!(card.effects.len(), 3, "main, delay, security");
    let trigger = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Delay { trigger, .. }) => {
                Some(*trigger)
            }
            _ => None,
        })
        .expect("a <Delay> clause");
    assert_eq!(trigger, CompiledTiming::OnDigivolve);
}

// ─── Clause 0: colour bypass ────────────────────────────────────────────────

#[test]
fn bt23_099_off_colour_is_a_mask_miss_without_huckmon() {
    let r = color_runner(&[]);
    assert_eq!(
        build_action_mask(&r.game, 0)[PLAY_HAND_START as usize],
        0.0,
        "white Option, only a red Digimon: the colour requirement applies"
    );
}

#[test]
fn bt23_099_huckmon_named_digimon_lets_the_option_ignore_colour() {
    let r = color_runner(&["HUCK-FIELD"]);
    assert_eq!(
        build_action_mask(&r.game, 0)[PLAY_HAND_START as usize],
        1.0,
        "a Digimon with [Huckmon] in its name lifts the colour requirement"
    );
}

#[test]
fn bt23_099_huckmon_named_tamer_does_not_count() {
    let r = color_runner(&["HUCK-TAMER"]);
    assert_eq!(
        build_action_mask(&r.game, 0)[PLAY_HAND_START as usize],
        0.0,
        "the printed text says Digimon, not Tamer"
    );
}

#[test]
fn bt23_099_huckmon_in_the_opponents_field_does_not_count() {
    let mut r = color_runner(&[]);
    r.place_on_field(1, "HUCK-FIELD", Some(0));
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 0.0);
}

#[test]
fn bt23_099_huckmon_in_the_breeding_area_counts_as_on_the_field() {
    // Official Q&A: "on the field" = the battle area or the breeding area.
    let mut r = color_runner(&[]);
    r.place_in_breeding(0, "HUCK-FIELD");
    assert_eq!(
        build_action_mask(&r.game, 0)[PLAY_HAND_START as usize],
        1.0,
        "a [Huckmon] Digimon in the breeding area lifts the colour requirement"
    );
}

#[test]
fn bt23_099_bypass_is_usable_end_to_end() {
    let mut r = color_runner(&["HUCK-FIELD"]);
    r.game.decode_action(PLAY_HAND_START, 0);
    assert!(
        !r.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == CARD_ID),
        "the decoder accepts the Option through the bypass"
    );
}

// ─── Clause 1: [Main] ───────────────────────────────────────────────────────

#[test]
fn bt23_099_main_draws_one_and_places_as_a_delay() {
    let mut r = color_runner(&["HUCK-FIELD"]);
    let deck = r.deck_size(0);
    let hand = r.hand_size(0);
    let _ = r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.deck_size(0), deck - 1, "<Draw 1>");
    assert_eq!(r.hand_size(0), hand, "-1 (the Option) +1 (the draw)");
    assert!(is_parked_on_digivolve(&r, 0), "placed in the battle area");
    assert!(!trash_ids(&r, 0).contains(&CARD_ID.to_string()));
    assert_eq!(r.memory(), 8, "cost 2");
}

// ─── Clause 2: [Your Turn] digivolve into [Huckmon] / [Jesmon] -> <Delay> ────

#[test]
fn bt23_099_delay_plays_a_sistermon_from_hand_or_trash_for_free() {
    let mut r = parked(&["HUCK-EVO", "SIS-HAND", "SIS-TAMER", "NOT-SIS"]);
    push_trash(&mut r, 0, "SIS-TRASH");
    digivolve_base_into(&mut r, "HUCK-EVO");
    accept_delay(&mut r);
    assert!(r.pending_is_optional(), "'You may play'");
    assert_eq!(
        union_offered(&r, 0),
        vec![
            (false, "SIS-HAND".to_string()),
            (false, "SIS-TAMER".to_string()),
            (true, "SIS-TRASH".to_string())
        ],
        "[Sistermon] in the name, hand or trash"
    );
    let mem = r.memory();
    pick_union(&mut r, 0, "SIS-TRASH", true);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"SIS-TRASH".to_string()));
    assert_eq!(r.memory(), mem, "without paying the cost");
    assert!(trash_ids(&r, 0).contains(&CARD_ID.to_string()), "Delay cost");
    assert!(gym_state(&r, 0).is_none());
}

#[test]
fn bt23_099_delay_fires_for_jesmon_too() {
    let mut r = parked(&["JES-EVO", "SIS-HAND"]);
    digivolve_base_into(&mut r, "JES-EVO");
    accept_delay(&mut r);
    pick_union(&mut r, 0, "SIS-HAND", false);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"SIS-HAND".to_string()));
}

#[test]
fn bt23_099_delay_ignores_other_digivolutions() {
    let mut r = parked(&["PLAIN-EVO", "SIS-HAND"]);
    digivolve_base_into(&mut r, "PLAIN-EVO");
    assert!(
        r.pending_selection_view().is_none(),
        "only [Huckmon] / [Jesmon] digivolutions open the <Delay>"
    );
    assert!(is_parked_on_digivolve(&r, 0));
}

#[test]
fn bt23_099_delay_declined_stays_parked() {
    let mut r = parked(&["HUCK-EVO", "SIS-HAND"]);
    digivolve_base_into(&mut r, "HUCK-EVO");
    let v = r.pending_selection_view().expect("offered");
    r.execute_action(v.selecting_player, PASS).expect("decline");
    let _ = r.auto_resolve();
    assert!(is_parked_on_digivolve(&r, 0));
}

#[test]
fn bt23_099_delay_does_not_fire_on_the_placing_turn() {
    let mut r = color_runner(&["HUCK-FIELD"]);
    let _ = r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve();
    push_hand(&mut r, 0, "HUCK-EVO");
    digivolve_base_into(&mut r, "HUCK-EVO");
    assert!(
        r.pending_selection_view().is_none(),
        "<Delay> is only usable after the placing turn (16-17-3)"
    );
    assert!(is_parked_on_digivolve(&r, 0));
}

// ─── Clause 3: [Security] ───────────────────────────────────────────────────

#[test]
fn bt23_099_security_plays_a_sistermon_then_places_the_gym() {
    let mut r = builder()
        .security(1, &[CARD_ID])
        .hand(1, &["SIS-HAND"])
        .memory(0)
        .start();
    let attacker = r.place_on_field(0, "BASE", Some(0));
    let _ = r.attack_player(attacker, 1, false);
    assert!(r.pending_is_optional(), "'You may play'");
    pick_union(&mut r, 1, "SIS-HAND", false);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).contains(&"SIS-HAND".to_string()));
    assert!(is_parked_on_digivolve(&r, 1), "placed by the security effect");
    assert_eq!(r.security_count(1), 0);
}

#[test]
fn bt23_099_security_decline_still_places() {
    let mut r = builder()
        .security(1, &[CARD_ID])
        .hand(1, &["SIS-HAND"])
        .memory(0)
        .start();
    let attacker = r.place_on_field(0, "BASE", Some(0));
    let _ = r.attack_player(attacker, 1, false);
    r.execute_action(1, PASS).expect("decline the optional play");
    let _ = r.auto_resolve();
    assert!(is_parked_on_digivolve(&r, 1));
    assert!(!field_ids(&r, 1).contains(&"SIS-HAND".to_string()));
}

#[path = "../../support/owned_union_fixture.rs"]
mod owned_union_fixture;
