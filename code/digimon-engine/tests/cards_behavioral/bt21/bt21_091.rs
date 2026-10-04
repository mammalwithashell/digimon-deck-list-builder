//! BT21-091 Spirit Evolution! — Option, Red, cost 3, Ten Warriors.
//!
//! Official text (data/card_bundles/BT21-091.md):
//!   While you have a Tamer with inherited effects, you can ignore this card's
//!   color requirements.
//!   [Main] By trashing 1 card with the [Hybrid] trait from your hand,
//!   <Draw 2>. Then, place this card in the battle area.
//!   [All Turns] When any of your Tamers with inherited effects are played,
//!   <Delay>.
//!   ・1 of your Tamers may digivolve into a Digimon card with the [Hybrid]
//!     trait in the hand without paying the cost.
//!   [Security] You may play 1 Tamer card with inherited effects from your
//!   hand or trash without paying the cost. Then, add this card to the hand.
//!
//! DCGO: BT21/Red/BT21_091.cs.
//!
//! Pattern tags: option-ignore-color-flood-gate, main-optional-trash-cost-draw,
//! place-self-as-delay-option, event-gated-delay-on-ally-played,
//! event-target-has-inherited, tamer-digivolve-into-hybrid-free,
//! security-union-zone-play-then-add-self.
#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{self, PASS, PLAY_HAND_START, REPLACEMENT_ACCEPT};
use digimon_engine::build_action_mask;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming};
use digimon_engine::permanent::{OptionState, PermanentHandle};
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

const CARD_ID: &str = "BT21-091";

/// Red [Hybrid] Lv.4 Digimon that can digivolve onto a red Tamer (cost 4) —
/// the Frontier "Tamer → Hybrid" route the Delay body needs.
const HYBRID_EVO_YAML: &str = r#"
card: T-HYB-EVO
name: "Hybrid Evo"
kind: digimon
level: 4
color: [red]
cost: 6
dp: 6000
traits: [Hybrid]
alt_paths:
  - kind: digivolve
    from: { kind: tamer, color_is: red }
    cost: 4
effects: []
"#;

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .add_card(filler("FILLER"))
        .add_card(digimon("RED3", "Red Three", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("BLUE3", "Blue Three", CardColor::Blue, 3, 3, &[]))
        .add_card(digimon(
            "HYB-DISCARD",
            "Hybrid Discard",
            CardColor::Red,
            3,
            3,
            &["Hybrid"],
        ))
        .add_card(digimon(
            "HERO-CARD",
            "Hero Card",
            CardColor::Red,
            3,
            3,
            &["Hero"],
        ))
        .add_card(digimon("ATK", "Attacker", CardColor::Blue, 3, 3, &[]))
        // Plain (no-route) Hybrid — can't digivolve onto a Tamer.
        .add_card(digimon(
            "HYB-NOROUTE",
            "Hybrid No Route",
            CardColor::Red,
            4,
            5,
            &["Hybrid"],
        ))
        .add_card(tamer(
            "RED-TAMER-INH",
            "Red Spirit Tamer",
            CardColor::Red,
            true,
        ))
        .add_card(tamer(
            "RED-TAMER-PLAIN",
            "Red Plain Tamer",
            CardColor::Red,
            false,
        ))
        .add_card(tamer(
            "BLUE-TAMER-INH",
            "Blue Spirit Tamer",
            CardColor::Blue,
            true,
        ))
        .add_card(tamer(
            "BLUE-TAMER-PLAIN",
            "Blue Plain Tamer",
            CardColor::Blue,
            false,
        ))
        .from_dsl_yaml(HYBRID_EVO_YAML)
        .expect("hybrid fixture")
        .dsl_card(CARD_ID)
        .expect("BT21-091")
        .deck(0, &["FILLER"; 12])
        .deck(1, &["FILLER"; 12])
}

fn setup() -> DebugRunner {
    let mut r = builder().memory(10).start();
    r.set_first_player(0);
    r
}

fn delayed_state(r: &DebugRunner) -> Option<OptionState> {
    r.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == CARD_ID)
        .map(|p| p.option_state)
}

/// Use the Option ([Main]) from hand with a red Digimon on board. If
/// `discard` is `Some`, trash that card for the cost; otherwise decline.
fn use_main(r: &mut DebugRunner, discard: Option<&str>) {
    r.place_on_field(0, "RED3", Some(0));
    r.game.enter_main_phase();
    let idx = hand_index(r, 0, CARD_ID);
    let res = r.game.play_option_from_hand(0, idx);
    assert!(!matches!(res, OptionPlayResult::Invalid), "usable with red");
    match discard {
        Some(id) => {
            let v = r.pending_selection_view().expect("optional trash cost");
            assert!(v.is_optional, "'By trashing' cost is declinable");
            pick_hand(r, 0, id);
        }
        None => {
            if r.pending_selection_view().is_some() {
                pass(r, 0);
            }
        }
    }
    let _ = r.auto_resolve();
}

fn advance_to_next_own_main(r: &mut DebugRunner) {
    r.end_turn();
    r.game.enter_main_phase();
    r.end_turn();
    assert_eq!(r.game.turn_player(), 0);
    r.game.enter_main_phase();
}

fn play_from_hand_by_id(r: &mut DebugRunner, card_id: &str) {
    let idx = hand_index(r, 0, card_id);
    assert!(r.play(0, idx).is_some(), "{card_id} playable");
}

/// Park BT21-091 as a Delay Option and advance to the next own main phase
/// with `tamer_id` + `evo_ids` in hand and 10 memory.
fn parked_with(tamer_id: &str, evo_ids: &[&str]) -> DebugRunner {
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    push_hand(&mut r, 0, tamer_id);
    for id in evo_ids {
        push_hand(&mut r, 0, id);
    }
    use_main(&mut r, None);
    advance_to_next_own_main(&mut r);
    r.game.memory = 10;
    r
}

// ─── Structure ──────────────────────────────────────────────────────────────

#[test]
fn bt21_091_structure() {
    let r = setup();
    let c = r.compiled_card(CARD_ID).unwrap();
    assert_eq!(c.cost, Some(3));
    assert_eq!(c.traits, vec!["Ten Warriors".to_string()]);
    let req = c
        .use_requirement
        .as_ref()
        .expect("color-bypass use requirement");
    let any = req.any_permanent.as_ref().expect("any_permanent gate");
    assert!(
        any.predicate.has_inherited.is_some(),
        "Tamer with inherited effects"
    );
    let main = c
        .effects
        .iter()
        .find_map(|cl| match cl {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand) => {
                Some(t)
            }
            _ => None,
        })
        .expect("[Main]");
    assert!(!main.optional);
    assert_eq!(
        main.process.last(),
        Some(&CompiledStep::PlaceSelfAsDelayOption)
    );
    let delay = c
        .effects
        .iter()
        .find_map(|cl| match cl {
            CompiledClause::Declarative(CompiledDeclarativeClause::Delay { trigger, .. }) => {
                Some(trigger)
            }
            _ => None,
        })
        .expect("<Delay>");
    assert_eq!(*delay, CompiledTiming::OnAllyPlayed);
    let sec = c
        .effects
        .iter()
        .find_map(|cl| match cl {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
            _ => None,
        })
        .expect("[Security]");
    assert_eq!(sec.scope, CompiledScope::Inherited);
    assert_eq!(sec.process.last(), Some(&CompiledStep::AddThisOptionToHand));
}

// ─── Clause 0: color bypass ─────────────────────────────────────────────────

fn color_runner(board_tamer: &str) -> DebugRunner {
    let mut r = builder().hand(0, &[CARD_ID]).memory(10).start();
    r.set_first_player(0);
    r.game.enter_main_phase();
    r.place_on_field(0, board_tamer, Some(0));
    r
}

#[test]
fn bt21_091_color_bypass_with_tamer_with_inherited_effects() {
    let r = color_runner("BLUE-TAMER-INH");
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 1.0,
        "a (blue) Tamer with inherited effects lets the red Option ignore color"
    );
}

#[test]
fn bt21_091_no_color_bypass_without_inherited_tamer() {
    let r = color_runner("BLUE-TAMER-PLAIN");
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 0.0,
        "no red source and no Tamer with inherited effects → color requirement applies"
    );
}

#[test]
fn bt21_091_color_bypass_uses_option_end_to_end() {
    let mut r = color_runner("BLUE-TAMER-INH");
    r.game.decode_action(PLAY_HAND_START, 0);
    assert!(
        !r.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == CARD_ID),
        "decoder accepts the Option via the bypass"
    );
}

// ─── Clause 1: [Main] ───────────────────────────────────────────────────────

#[test]
fn bt21_091_main_trashes_hybrid_draws_two_and_places() {
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    push_hand(&mut r, 0, "HYB-DISCARD");
    push_hand(&mut r, 0, "HERO-CARD");
    let deck = r.deck_size(0);
    use_main(&mut r, Some("HYB-DISCARD"));
    assert_eq!(r.deck_size(0), deck - 2, "<Draw 2>");
    assert!(trash_ids(&r, 0).contains(&"HYB-DISCARD".to_string()));
    assert_eq!(r.hand_size(0), 3, "HERO + 2 drawn");
    assert!(
        matches!(
            delayed_state(&r),
            Some(OptionState::Delayed {
                trigger: DelayTrigger::OnEvent(EffectTiming::OnAllyPlayed),
                ..
            })
        ),
        "placed as an event-gated Delay"
    );
}

#[test]
fn bt21_091_main_only_hybrid_cards_are_trashable() {
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    push_hand(&mut r, 0, "HYB-DISCARD");
    push_hand(&mut r, 0, "HERO-CARD");
    r.place_on_field(0, "RED3", Some(0));
    r.game.enter_main_phase();
    let idx = hand_index(&r, 0, CARD_ID);
    let _ = r.game.play_option_from_hand(0, idx);
    assert!(hand_pickable(&r, 0, "HYB-DISCARD"));
    assert!(!hand_pickable(&r, 0, "HERO-CARD"), "[Hero] is not [Hybrid]");
}

#[test]
fn bt21_091_main_declining_cost_skips_draw_but_still_places() {
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    push_hand(&mut r, 0, "HYB-DISCARD");
    let deck = r.deck_size(0);
    use_main(&mut r, None);
    assert_eq!(r.deck_size(0), deck, "no draw without the trash");
    assert_eq!(hand_ids(&r, 0), vec!["HYB-DISCARD".to_string()]);
    assert!(delayed_state(&r).is_some(), "placement is unconditional");
}

#[test]
fn bt21_091_main_without_hybrid_in_hand_still_places() {
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    push_hand(&mut r, 0, "HERO-CARD");
    let deck = r.deck_size(0);
    use_main(&mut r, None);
    assert_eq!(r.deck_size(0), deck);
    assert!(delayed_state(&r).is_some());
}

// ─── Clause 2: <Delay> ──────────────────────────────────────────────────────

fn accept_delay(r: &mut DebugRunner) {
    let v = r
        .pending_selection_view()
        .expect("<Delay> activation offered");
    assert_eq!(v.kind, SelectionKind::Replacement);
    assert!(r.pending_is_optional(), "<Delay> is optional");
    r.execute_action(v.selecting_player, REPLACEMENT_ACCEPT)
        .unwrap();
}

#[test]
fn bt21_091_delay_tamer_digivolves_into_hybrid_for_free() {
    let mut r = parked_with("RED-TAMER-INH", &["T-HYB-EVO", "HERO-CARD"]);
    play_from_hand_by_id(&mut r, "RED-TAMER-INH");
    let mem_after_play = r.memory();
    assert_eq!(mem_after_play, 7, "Tamer cost 3");
    accept_delay(&mut r);
    let v = r.pending_selection_view().expect("Tamer pick");
    assert!(v.is_optional, "'may digivolve'");
    let tamer = find_perm(&r, 0, "RED-TAMER-INH");
    pick_own_field(&mut r, 0, tamer);
    assert!(hand_pickable(&r, 0, "T-HYB-EVO"));
    assert!(!hand_pickable(&r, 0, "HERO-CARD"), "only [Hybrid]");
    pick_hand(&mut r, 0, "T-HYB-EVO");
    let _ = r.auto_resolve();
    let perm = &r.game.players[0].battle_area[tamer.index as usize];
    assert_eq!(perm.top_card().card_id(&r.game.card_data), "T-HYB-EVO");
    assert!(
        perm.card_sources
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "RED-TAMER-INH"),
        "the Tamer becomes a digivolution card"
    );
    assert_eq!(r.memory(), mem_after_play, "without paying the cost");
    assert!(
        trash_ids(&r, 0).contains(&CARD_ID.to_string()),
        "Delay trashes this card"
    );
    assert!(delayed_state(&r).is_none());
}

#[test]
fn bt21_091_delay_tamer_without_route_is_not_offered() {
    let mut r = parked_with("RED-TAMER-INH", &["HYB-NOROUTE"]);
    play_from_hand_by_id(&mut r, "RED-TAMER-INH");
    accept_delay(&mut r);
    assert!(
        r.pending_selection_view().is_none(),
        "no Tamer has a legal [Hybrid] digivolution → nothing to choose"
    );
    assert!(
        trash_ids(&r, 0).contains(&CARD_ID.to_string()),
        "Delay still paid"
    );
}

#[test]
fn bt21_091_delay_digivolve_is_optional() {
    let mut r = parked_with("RED-TAMER-INH", &["T-HYB-EVO"]);
    play_from_hand_by_id(&mut r, "RED-TAMER-INH");
    accept_delay(&mut r);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(hand_ids(&r, 0).contains(&"T-HYB-EVO".to_string()));
    let tamer = find_perm(&r, 0, "RED-TAMER-INH");
    assert_eq!(
        r.game.players[0].battle_area[tamer.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "RED-TAMER-INH"
    );
    assert!(trash_ids(&r, 0).contains(&CARD_ID.to_string()));
}

#[test]
fn bt21_091_declining_delay_keeps_it_parked() {
    let mut r = parked_with("RED-TAMER-INH", &["T-HYB-EVO"]);
    play_from_hand_by_id(&mut r, "RED-TAMER-INH");
    let v = r
        .pending_selection_view()
        .expect("<Delay> activation offered");
    pass(&mut r, v.selecting_player);
    let _ = r.auto_resolve();
    assert!(delayed_state(&r).is_some(), "declined <Delay> stays");
    assert!(hand_ids(&r, 0).contains(&"T-HYB-EVO".to_string()));
}

#[test]
fn bt21_091_delay_ignores_tamer_without_inherited_effects() {
    let mut r = parked_with("RED-TAMER-PLAIN", &["T-HYB-EVO"]);
    play_from_hand_by_id(&mut r, "RED-TAMER-PLAIN");
    assert!(
        r.pending_selection_view().is_none(),
        "Tamer without inherited effects does not trigger"
    );
    assert!(delayed_state(&r).is_some());
}

#[test]
fn bt21_091_delay_ignores_digimon_play() {
    let mut r = parked_with("RED3", &["T-HYB-EVO"]);
    play_from_hand_by_id(&mut r, "RED3");
    assert!(r.pending_selection_view().is_none(), "not a Tamer");
    assert!(delayed_state(&r).is_some());
}

#[test]
fn bt21_091_delay_not_on_placing_turn() {
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    push_hand(&mut r, 0, "RED-TAMER-INH");
    push_hand(&mut r, 0, "T-HYB-EVO");
    use_main(&mut r, None);
    r.game.memory = 10;
    play_from_hand_by_id(&mut r, "RED-TAMER-INH");
    assert!(r.pending_selection_view().is_none(), "placing turn");
    assert!(delayed_state(&r).is_some());
}

#[test]
fn bt21_091_delay_fires_on_opponents_turn_all_turns() {
    // [All Turns]: our Tamer with inherited effects is played on the
    // opponent's turn by a second Spirit Evolution!'s [Security] effect.
    let mut r = builder().security(0, &[CARD_ID]).memory(10).start();
    r.set_first_player(0);
    push_hand(&mut r, 0, CARD_ID);
    push_hand(&mut r, 0, "RED-TAMER-INH");
    push_hand(&mut r, 0, "T-HYB-EVO");
    use_main(&mut r, None);
    assert!(delayed_state(&r).is_some());
    r.end_turn();
    assert_eq!(r.game.turn_player(), 1);
    r.game.enter_main_phase();
    let atk = r.place_on_field(1, "ATK", Some(0));
    r.attack_player(atk, 0, false);
    let v = r.pending_selection_view().expect("security play prompt");
    assert!(v.is_optional);
    pick_hand(&mut r, 0, "RED-TAMER-INH");
    // The parked copy's <Delay> now offers activation on the opponent's turn.
    accept_delay(&mut r);
    let tamer = find_perm(&r, 0, "RED-TAMER-INH");
    pick_own_field(&mut r, 0, tamer);
    pick_hand(&mut r, 0, "T-HYB-EVO");
    let _ = r.auto_resolve();
    assert_eq!(
        r.game.players[0].battle_area[tamer.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "T-HYB-EVO"
    );
}

// ─── Clause 3: [Security] ───────────────────────────────────────────────────

fn security_runner() -> DebugRunner {
    let mut r = builder().security(1, &[CARD_ID]).memory(3).start();
    r.set_first_player(0);
    r
}

#[test]
fn bt21_091_security_plays_inherited_tamer_from_hand_then_adds_self() {
    let mut r = security_runner();
    push_hand(&mut r, 1, "RED-TAMER-INH");
    push_hand(&mut r, 1, "RED-TAMER-PLAIN");
    let atk = r.place_on_field(0, "ATK", Some(0));
    r.attack_player(atk, 1, false);
    let v = r.pending_selection_view().expect("security play prompt");
    assert!(v.is_optional, "'you may play'");
    assert_eq!(
        non_pass(&r).len(),
        1,
        "only the Tamer with inherited effects"
    );
    pick_hand(&mut r, 1, "RED-TAMER-INH");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).contains(&"RED-TAMER-INH".to_string()));
    assert!(
        hand_ids(&r, 1).contains(&CARD_ID.to_string()),
        "then add this card"
    );
    assert_eq!(r.memory(), 3, "without paying the cost");
}

#[test]
fn bt21_091_security_plays_inherited_tamer_from_trash() {
    let mut r = security_runner();
    push_trash(&mut r, 1, "BLUE-TAMER-INH");
    let atk = r.place_on_field(0, "ATK", Some(0));
    r.attack_player(atk, 1, false);
    pick_trash(&mut r, 1, "BLUE-TAMER-INH");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).contains(&"BLUE-TAMER-INH".to_string()));
    assert!(hand_ids(&r, 1).contains(&CARD_ID.to_string()));
}

#[test]
fn bt21_091_security_decline_still_adds_self() {
    let mut r = security_runner();
    push_hand(&mut r, 1, "RED-TAMER-INH");
    let atk = r.place_on_field(0, "ATK", Some(0));
    r.attack_player(atk, 1, false);
    pass(&mut r, 1);
    let _ = r.auto_resolve();
    assert!(!field_ids(&r, 1).contains(&"RED-TAMER-INH".to_string()));
    assert!(hand_ids(&r, 1).contains(&CARD_ID.to_string()));
}

#[test]
fn bt21_091_security_without_candidates_adds_self() {
    let mut r = security_runner();
    push_hand(&mut r, 1, "RED-TAMER-PLAIN");
    let atk = r.place_on_field(0, "ATK", Some(0));
    r.attack_player(atk, 1, false);
    let _ = r.auto_resolve();
    assert!(!field_ids(&r, 1).contains(&"RED-TAMER-PLAIN".to_string()));
    assert!(hand_ids(&r, 1).contains(&CARD_ID.to_string()));
}

// ─── Local fixtures ─────────────────────────────────────────────────────────

fn digimon(
    id: &str,
    name: &str,
    color: CardColor,
    level: u8,
    cost: u16,
    traits: &[&str],
) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = cost;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn tamer(id: &str, name: &str, color: CardColor, inherited: bool) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![color];
    if inherited {
        c.inherited_text = "[Your Turn] This Digimon gets +1000 DP.".to_string();
    }
    c
}

fn filler(id: &str) -> CardData {
    digimon(id, id, CardColor::Red, 3, 3, &[])
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

fn hand_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    ids(r, &r.game.players[p as usize].hand)
}

fn trash_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    ids(r, &r.game.players[p as usize].trash)
}

fn pass(r: &mut DebugRunner, p: u8) {
    r.execute_action(p, PASS).expect("PASS legal");
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

fn hand_pickable(r: &DebugRunner, p: u8, card_id: &str) -> bool {
    let Some(idx) = r.game.players[p as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == card_id)
    else {
        return false;
    };
    non_pass(r).contains(&(space::PLAY_HAND_START + idx as u16))
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

fn pick_trash(r: &mut DebugRunner, p: u8, card_id: &str) {
    let idx = r.game.players[p as usize]
        .trash
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not in trash"));
    let action = space::TRASH_EFFECT_START + idx as u16;
    let v = r.pending_selection_view().expect("trash selection pending");
    assert!(
        v.valid_action_ids.contains(&action),
        "{card_id} not a legal trash pick (valid {:?})",
        v.valid_action_ids
    );
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
