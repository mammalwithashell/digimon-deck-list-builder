//! BT21-090 The Strongest of Brothers — Option, Red, Cost 3, no traits.
//!
//! Official text (data/card_bundles/BT21-090.md):
//!   While you have a Digimon or Tamer with [Gammamon] in its text on the
//!   field, you can ignore this card's color requirements.
//!   [Main] Reveal the top 3 cards of your deck. Add 1 card with [Gammamon] in
//!   its text among them to the hand. Return the rest to the bottom of the
//!   deck. Then, place this card in the battle area.
//!   [All Turns] When effects place digivolution cards under your Digimon,
//!   <Delay>
//!   ・1 of your Digimon may digivolve into a Digimon card with [Gammamon] in
//!     its text in the hand without paying the cost.
//!   [Security] You may play 1 card with [Gammamon] in its text and a play cost
//!   of 4 or less from your hand or trash without paying the cost. Then, place
//!   this card in the battle area.
//!
//! DCGO: BT21/Red/BT21_090.cs.
//!
//! Regression origin: a real DCGO PvP game replayed through the engine. With no
//! YAML the [Main] reveal-3 prompt never ran ("met no prompt") and the colour
//! bypass and <Delay> clauses were absent. Synthetic boards only.
#![allow(dead_code, unused_imports)]

use std::path::Path;

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{self, PASS, PLAY_HAND_START, REPLACEMENT_ACCEPT, SEL_REVEAL_START};
use digimon_engine::build_action_mask;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming};
use digimon_engine::permanent::{OptionState, PermanentHandle};
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

const CARD_ID: &str = "BT21-090";

fn yaml() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/bt21/BT21-090.yaml"))
        .expect("BT21-090 YAML must exist at cards/bt21/BT21-090.yaml")
}

/// An inline fixture: "[On Play] Place 1 card from your trash as this Digimon's
/// bottom digivolution card." An EFFECT-driven placement (fires
/// OnAddDigivolutionCards), fired manually so it can run on either turn.
const PLACER_YAML: &str = r#"
card: TEST-PLACER
name: Placermon
kind: digimon
level: 4
color: [red]
cost: 4
dp: 4000
traits: []
effects:
  - when: on_play
    summary: "[On Play] Place 1 card from your trash as this Digimon's bottom digivolution card"
    process:
      - select_trash:
          of: you
          bind_as: placed
          filter: {}
          prompt: "Place 1 card from your trash under this Digimon"
      - place_as_bottom_source:
          source: placed
          target: source
"#;

fn digimon(id: &str, name: &str, color: CardColor, level: u8, cost: u16) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = cost;
    c
}

fn with_text(mut c: CardData, text: &str) -> CardData {
    c.effect_text = text.to_string();
    c
}

fn tamer(id: &str, name: &str, color: CardColor, text: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![color];
    c.effect_text = text.to_string();
    c
}

/// A red Lv.4 with a free-to-test evo route from a red Lv.3.
fn evo(id: &str, name: &str) -> CardData {
    let mut c = digimon(id, name, CardColor::Red, 4, 5);
    c.evo_costs = vec![EvoCost {
        card_color: CardColor::Red as u8,
        level: 3,
        memory_cost: 3,
    }];
    c
}

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("BT21-090 YAML parses and compiles")
        .from_dsl_yaml(PLACER_YAML)
        .expect("placer fixture")
        .add_card(filler("FILL"))
        .add_card(filler("FILL-B"))
        .add_card(filler("FILL-C"))
        // Colour-bypass anchors (blue board => the red Option is off-colour).
        .add_card(digimon("BLUE-PLAIN", "Blue Plain", CardColor::Blue, 3, 3))
        .add_card(digimon("BLUE-NAME", "Gammamon Blue", CardColor::Blue, 3, 3))
        .add_card(with_text(
            digimon("BLUE-TEXT", "Blue Texty", CardColor::Blue, 3, 3),
            "[On Play] Add 1 [Gammamon] to your hand.",
        ))
        .add_card(tamer(
            "BLUE-TAMER-TEXT",
            "Blue Tamer",
            CardColor::Blue,
            "[Your Turn] Your [Gammamon] get +1000 DP.",
        ))
        // Reveal / security / delay cards
        .add_card(with_text(
            digimon("GAM-TXT", "Texty", CardColor::Red, 3, 3),
            "[On Play] Add 1 [Gammamon].",
        ))
        .add_card(digimon("GAM-3", "Gammamon", CardColor::Red, 3, 3))
        .add_card(digimon("GAM-4", "Gammamon X", CardColor::Red, 3, 4))
        .add_card(digimon("GAM-6", "Gammamon Big", CardColor::Red, 4, 6))
        .add_card(tamer("GAM-TAMER", "Gammamon Tamer", CardColor::Red, ""))
        .add_card(digimon("NOT-GAM", "Agumon", CardColor::Red, 3, 3))
        .add_card(digimon("BASE", "Base Digimon", CardColor::Red, 3, 3))
        .add_card(evo("GAM-EVO", "Gammamon Evolved"))
        .add_card(evo("PLAIN-EVO", "Plain Evolved"))
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

fn ids(r: &DebugRunner, cards: &[CardSource]) -> Vec<String> {
    cards
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn hand_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    ids(r, &r.game.players[p as usize].hand)
}

fn trash_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    ids(r, &r.game.players[p as usize].trash)
}

fn deck_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    ids(r, &r.game.players[p as usize].deck)
}

fn field_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&r.game.card_data).to_string())
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

fn brothers_state(r: &DebugRunner, p: u8) -> Option<OptionState> {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .find(|perm| perm.top_card().card_id(&r.game.card_data) == CARD_ID)
        .map(|perm| perm.option_state)
}

fn is_parked_on_add_sources(r: &DebugRunner, p: u8) -> bool {
    matches!(
        brothers_state(r, p),
        Some(OptionState::Delayed {
            trigger: DelayTrigger::OnEvent(EffectTiming::OnAddDigivolutionCards),
            ..
        })
    )
}

fn non_pass(r: &DebugRunner) -> Vec<u16> {
    r.pending_selection_view()
        .map(|v| {
            v.valid_action_ids
                .into_iter()
                .filter(|&a| a != PASS)
                .collect()
        })
        .unwrap_or_default()
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

fn pick_own_field(r: &mut DebugRunner, selector: u8, h: PermanentHandle) {
    let action = space::encode_attack(0, h.index as u16);
    let v = r.pending_selection_view().expect("field selection pending");
    assert!(
        v.valid_action_ids.contains(&action),
        "{h:?} not a legal pick (valid {:?})",
        v.valid_action_ids
    );
    r.execute_action(selector, action).unwrap();
}

fn pick_hand(r: &mut DebugRunner, p: u8, card_id: &str) {
    let idx = hand_index(r, p, card_id);
    let action = space::PLAY_HAND_START + idx as u16;
    let v = r.pending_selection_view().expect("hand selection pending");
    assert!(
        v.valid_action_ids.contains(&action),
        "{card_id} not a legal hand pick (valid {:?})",
        v.valid_action_ids
    );
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

fn revealed_action_for_id(r: &DebugRunner, id: &str) -> Option<u16> {
    r.game
        .revealed_cards
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&r.game.card_data) == id).then_some(SEL_REVEAL_START + idx as u16)
        })
}

// ─── Structure ──────────────────────────────────────────────────────────────

#[test]
fn bt21_090_structure() {
    let r = builder().build();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.cost, Some(3));
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
    assert_eq!(trigger, CompiledTiming::OnAddDigivolutionCards);
}

// ─── Clause 0: colour bypass ────────────────────────────────────────────────

fn color_runner(field: &[&str]) -> DebugRunner {
    let mut r = builder().hand(0, &[CARD_ID]).memory(10).start();
    r.set_first_player(0);
    r.game.enter_main_phase();
    for id in field {
        r.place_on_field(0, id, Some(0));
    }
    r
}

fn playable(r: &DebugRunner) -> bool {
    build_action_mask(&r.game, 0)[PLAY_HAND_START as usize] == 1.0
}

#[test]
fn bt21_090_off_colour_is_a_mask_miss_without_gammamon_text() {
    assert!(!playable(&color_runner(&["BLUE-PLAIN"])));
}

#[test]
fn bt21_090_digimon_named_gammamon_lifts_the_colour_requirement() {
    assert!(playable(&color_runner(&["BLUE-NAME"])));
}

#[test]
fn bt21_090_digimon_with_gammamon_in_its_effect_text_lifts_it() {
    assert!(playable(&color_runner(&["BLUE-TEXT"])));
}

#[test]
fn bt21_090_tamer_with_gammamon_in_its_text_lifts_it() {
    assert!(playable(&color_runner(&["BLUE-TAMER-TEXT"])));
}

#[test]
fn bt21_090_gammamon_in_the_breeding_area_counts_as_on_the_field() {
    let mut r = color_runner(&["BLUE-PLAIN"]);
    r.place_in_breeding(0, "BLUE-NAME");
    assert!(playable(&r), "the breeding area is part of the field");
}

#[test]
fn bt21_090_opponents_gammamon_does_not_lift_it() {
    let mut r = color_runner(&["BLUE-PLAIN"]);
    r.place_on_field(1, "BLUE-NAME", Some(0));
    assert!(!playable(&r));
}

#[test]
fn bt21_090_bypass_is_usable_end_to_end() {
    let mut r = color_runner(&["BLUE-NAME"]);
    r.game.decode_action(PLAY_HAND_START, 0);
    assert!(
        !hand_ids(&r, 0).contains(&CARD_ID.to_string()),
        "the decoder accepts the Option through the bypass"
    );
}

// ─── Clause 1: [Main] reveal 3, add 1 [Gammamon]-text card, place ────────────

/// Deck top->: FILL-B, GAM-TXT, FILL-C (builder convention: last = top).
fn main_runner() -> DebugRunner {
    let mut r = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL", "FILL-C", "GAM-TXT", "FILL-B"])
        .memory(10)
        .start();
    r.set_first_player(0);
    r.place_on_field(0, "BASE", Some(0));
    r.game.enter_main_phase();
    r
}

#[test]
fn bt21_090_main_adds_a_gammamon_text_card_and_bottoms_the_rest() {
    let mut r = main_runner();
    let res = r.game.play_option_from_hand(0, 0);
    assert_eq!(res, OptionPlayResult::Pending, "the reveal pick parks the Option");
    let action = revealed_action_for_id(&r, "GAM-TXT")
        .expect("the [Gammamon]-text card is among the revealed cards");
    assert!(
        revealed_action_for_id(&r, "FILL-B").is_some(),
        "3 cards revealed"
    );
    assert_eq!(
        non_pass(&r),
        vec![action],
        "only the card with [Gammamon] in its text is selectable"
    );
    r.execute_action(0, action).expect("add it to the hand");
    let _ = r.auto_resolve();
    assert!(hand_ids(&r, 0).contains(&"GAM-TXT".to_string()));
    let deck = deck_ids(&r, 0);
    assert_eq!(deck.len(), 5, "6 - 1 added");
    let bottom: Vec<&String> = deck.iter().take(2).collect();
    assert!(
        bottom.iter().any(|id| id.as_str() == "FILL-B")
            && bottom.iter().any(|id| id.as_str() == "FILL-C"),
        "the other two revealed cards go to the deck bottom: {deck:?}"
    );
    assert!(is_parked_on_add_sources(&r, 0), "then placed in the battle area");
    assert!(!trash_ids(&r, 0).contains(&CARD_ID.to_string()));
}

#[test]
fn bt21_090_main_without_a_gammamon_card_returns_all_three_and_still_places() {
    let mut r = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL", "FILL-C", "FILL-B", "NOT-GAM"])
        .memory(10)
        .start();
    r.set_first_player(0);
    r.place_on_field(0, "BASE", Some(0));
    r.game.enter_main_phase();
    let deck = r.deck_size(0);
    let _ = r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.deck_size(0), deck, "nothing added, all three returned");
    assert!(is_parked_on_add_sources(&r, 0));
}

// ─── Clause 2: [All Turns] effects place digivolution cards -> <Delay> ───────

/// BT21-090 parked on turn 1; advanced to player 0's next turn (`opponents_turn`
/// false) or to player 1's turn 2 (true). A PLACER Digimon with `trash_card`
/// placed beneath it by its own [On Play] has just resolved; BASE (Lv.3) and the
/// `hand` cards are staged for the Delay body.
fn stage_delay(opponents_turn: bool, hand: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 12])
        .memory(10)
        .start();
    r.set_first_player(0);
    r.place_on_field(0, "BASE", Some(0));
    r.game.enter_main_phase();
    let _ = r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve();
    assert!(is_parked_on_add_sources(&r, 0), "parked");
    for id in hand {
        push_hand(&mut r, 0, id);
    }
    r.end_turn();
    r.game.enter_main_phase();
    if !opponents_turn {
        r.end_turn();
        r.game.enter_main_phase();
        assert_eq!(r.game.turn_player(), 0);
    } else {
        assert_eq!(r.game.turn_player(), 1);
    }
    r.game.memory = 10;
    push_trash(&mut r, 0, "FILL-B");
    let placer = r.place_on_field(0, "TEST-PLACER", Some(0));
    r.game.enqueue_triggered(
        EffectTiming::OnPlay,
        digimon_engine::selection::TriggerSource::Permanent(placer),
    );
    r.game.drain_effect_queue();
    let view = r.pending_selection_view().expect("fixture trash pick");
    assert_eq!(view.kind, SelectionKind::Trash);
    r.execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("place the trash card under the Placermon");
    (r, placer)
}

#[test]
fn bt21_090_delay_digivolves_a_digimon_into_a_gammamon_text_card_for_free() {
    let (mut r, placer) = stage_delay(false, &["GAM-EVO", "PLAIN-EVO"]);
    accept_delay(&mut r);
    assert!(r.pending_is_optional(), "'may digivolve'");
    let base = find_perm(&r, 0, "BASE");
    // Only the Lv.3 BASE has a route onto [Gammamon Evolved]; the Lv.4 placer
    // does not, so it is not offered.
    let legal = non_pass(&r);
    assert!(legal.contains(&space::encode_attack(0, base.index as u16)));
    assert!(!legal.contains(&space::encode_attack(0, placer.index as u16)));
    pick_own_field(&mut r, 0, base);
    // Only the card with [Gammamon] in its text is offered from the hand.
    let hand_gam = space::PLAY_HAND_START + hand_index(&r, 0, "GAM-EVO") as u16;
    let hand_plain = space::PLAY_HAND_START + hand_index(&r, 0, "PLAIN-EVO") as u16;
    let legal = non_pass(&r);
    assert!(legal.contains(&hand_gam));
    assert!(!legal.contains(&hand_plain));
    let mem = r.memory();
    pick_hand(&mut r, 0, "GAM-EVO");
    let _ = r.auto_resolve();
    assert_eq!(
        field_ids(&r, 0)[base.index as usize],
        "GAM-EVO",
        "digivolved"
    );
    assert_eq!(r.memory(), mem, "without paying the cost");
    assert!(trash_ids(&r, 0).contains(&CARD_ID.to_string()), "Delay cost");
    assert!(brothers_state(&r, 0).is_none());
}

#[test]
fn bt21_090_delay_fires_on_the_opponents_turn_too() {
    // [All Turns]: unlike [Your Turn] clauses there is no turn gate.
    let (mut r, _) = stage_delay(true, &["GAM-EVO"]);
    accept_delay(&mut r);
    let base = find_perm(&r, 0, "BASE");
    pick_own_field(&mut r, 0, base);
    pick_hand(&mut r, 0, "GAM-EVO");
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0)[base.index as usize], "GAM-EVO");
}

#[test]
fn bt21_090_delay_declined_stays_parked() {
    let (mut r, _) = stage_delay(false, &["GAM-EVO"]);
    let v = r.pending_selection_view().expect("offered");
    assert_eq!(v.kind, SelectionKind::Replacement);
    r.execute_action(v.selecting_player, PASS).expect("decline");
    let _ = r.auto_resolve();
    assert!(is_parked_on_add_sources(&r, 0));
    assert!(hand_ids(&r, 0).contains(&"GAM-EVO".to_string()));
}

#[test]
fn bt21_090_delay_with_no_gammamon_digivolution_offers_nothing() {
    let (mut r, _) = stage_delay(false, &["PLAIN-EVO"]);
    accept_delay(&mut r);
    assert!(
        r.pending_selection_view().is_none(),
        "no Digimon has a [Gammamon]-text card to digivolve into"
    );
    assert!(
        trash_ids(&r, 0).contains(&CARD_ID.to_string()),
        "the Delay cost is still paid"
    );
}

#[test]
fn bt21_090_delay_does_not_fire_on_the_placing_turn() {
    let mut r = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 12])
        .memory(10)
        .start();
    r.set_first_player(0);
    r.place_on_field(0, "BASE", Some(0));
    r.game.enter_main_phase();
    let _ = r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve();
    push_hand(&mut r, 0, "GAM-EVO");
    push_trash(&mut r, 0, "FILL-B");
    let placer = r.place_on_field(0, "TEST-PLACER", Some(0));
    r.game.enqueue_triggered(
        EffectTiming::OnPlay,
        digimon_engine::selection::TriggerSource::Permanent(placer),
    );
    r.game.drain_effect_queue();
    let view = r.pending_selection_view().expect("fixture trash pick");
    r.execute_action(view.selecting_player, view.valid_action_ids[0])
        .unwrap();
    assert!(
        !matches!(r.pending_kind(), Some(SelectionKind::Replacement)),
        "<Delay> is only usable after the placing turn (16-17-3)"
    );
    assert!(is_parked_on_add_sources(&r, 0));
}

// ─── Clause 3: [Security] ───────────────────────────────────────────────────

fn security_runner() -> DebugRunner {
    let mut r = builder()
        .security(1, &[CARD_ID])
        .hand(1, &["GAM-3", "GAM-6", "NOT-GAM", "GAM-TAMER"])
        .memory(0)
        .start();
    push_trash(&mut r, 1, "GAM-4");
    let attacker = r.place_on_field(0, "BASE", Some(0));
    let _ = r.attack_player(attacker, 1, false);
    r
}

#[test]
fn bt21_090_security_offers_gammamon_text_cards_with_play_cost_4_or_less() {
    let r = security_runner();
    assert!(r.pending_is_optional(), "'You may play'");
    assert_eq!(
        union_offered(&r, 1),
        vec![
            (false, "GAM-3".to_string()),
            (false, "GAM-TAMER".to_string()),
            (true, "GAM-4".to_string())
        ],
        "[Gammamon] in its text and play cost <= 4, from hand or trash"
    );
}

#[test]
fn bt21_090_security_plays_the_pick_free_then_places_the_option() {
    let mut r = security_runner();
    pick_union(&mut r, 1, "GAM-4", true);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).contains(&"GAM-4".to_string()));
    assert!(is_parked_on_add_sources(&r, 1), "placed in the battle area");
    assert_eq!(r.security_count(1), 0);
    assert_eq!(r.memory(), 0, "played without paying the cost");
}

#[test]
fn bt21_090_security_decline_still_places() {
    let mut r = security_runner();
    r.execute_action(1, PASS).expect("decline");
    let _ = r.auto_resolve();
    assert!(is_parked_on_add_sources(&r, 1));
    assert!(!field_ids(&r, 1).contains(&"GAM-3".to_string()));
}

#[path = "../../support/owned_union_fixture.rs"]
mod owned_union_fixture;
