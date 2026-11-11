//! ST20-02 Biyomon — Digimon, Lv.3, Red, DP 1000, Cost 3.
//! Traits: Bird / ADVENTURE. Form: Rookie. Attribute: Vaccine.
//!
//! # Card text (official bundle `data/card_bundles/ST20-02.md`)
//!
//! ```text
//! Digivolve: Red Lv.2 / cost 0.
//! [Digivolve] Lv.2 w/[ADVENTURE] trait: Cost 0
//!
//! [On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with the
//! [ADVENTURE] trait and 1 such Tamer card or Option card among them to the
//! hand. Return the rest to the bottom of the deck.
//!
//! Inherited: [Your Turn] This Digimon gets +2000 DP.
//! ```
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/ST20/Red/ST20_02.cs` —
//! `SimplifiedRevealDeckTopCardsAndSelect(3, [AddHand ADVENTURE Digimon max 1,
//! AddHand ADVENTURE Option-or-Tamer max 1], DeckBottom)`; inherited
//! `ChangeSelfDPStaticEffect(+2000, IsOwnerTurn)`.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{encode_digivolve, PASS, SEL_REVEAL_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "ST20-02";

fn kind_card(id: &str, kind: CardKind, adventure: bool) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = kind;
    if kind != CardKind::Digimon {
        c.level = None;
        c.dp = None;
    }
    if adventure {
        c.traits = vec!["ADVENTURE".to_string()];
    }
    c
}

fn builder() -> DebugRunnerBuilder {
    let mut adv_lv2 = make_test_card("ADV-LV2", "Adv Baby");
    adv_lv2.level = Some(2);
    adv_lv2.colors = vec![CardColor::Blue];
    adv_lv2.traits = vec!["ADVENTURE".to_string()];
    let mut plain_lv2 = make_test_card("PLAIN-LV2", "Plain Baby");
    plain_lv2.level = Some(2);
    plain_lv2.colors = vec![CardColor::Blue];
    let mut carrier = make_test_card("CARRIER", "Carrier");
    carrier.level = Some(4);
    carrier.dp = Some(4000);
    carrier.evo_costs = vec![EvoCost { card_color: 0, level: 3, memory_cost: 2 }];
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-02 YAML parses, compiles and is in the embedded pack")
        .add_card(kind_card("ADV-DIGI", CardKind::Digimon, true))
        .add_card(kind_card("ADV-DIGI-2", CardKind::Digimon, true))
        .add_card(kind_card("ADV-TAMER", CardKind::Tamer, true))
        .add_card(kind_card("ADV-OPTION", CardKind::Option, true))
        .add_card(kind_card("PLAIN-DIGI", CardKind::Digimon, false))
        .add_card(kind_card("PLAIN-TAMER", CardKind::Tamer, false))
        .add_card(adv_lv2)
        .add_card(plain_lv2)
        .add_card(carrier)
        .add_card(make_test_card("FILL", "Fill"))
}

fn ids(runner: &DebugRunner, cards: &[digimon_engine::card_source::CardSource]) -> Vec<String> {
    cards
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn hand_ids(runner: &DebugRunner) -> Vec<String> {
    ids(runner, &runner.game.players[0].hand)
}

fn reveal_action(runner: &DebugRunner, id: &str) -> Option<u16> {
    runner
        .game
        .revealed_cards
        .iter()
        .enumerate()
        .find_map(|(i, c)| (c.card_id(&runner.game.card_data) == id).then_some(SEL_REVEAL_START + i as u16))
}

/// Play Biyomon from hand with `deck` (last = top of deck).
fn play_biyomon(deck: &[&str]) -> DebugRunner {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, deck)
        .deck(1, &["FILL", "FILL"])
        .memory(10)
        .start();
    runner.skip_mulligan();
    runner.play(0, 0).expect("Biyomon plays");
    runner
}

/// Pick `id` at the current reveal prompt. Panics if it is not legal.
fn pick(runner: &mut DebugRunner, id: &str) {
    let view = runner.pending_selection_view().expect("a reveal prompt must be pending");
    let action = reveal_action(runner, id).unwrap_or_else(|| panic!("{id} not revealed"));
    assert!(view.valid_action_ids.contains(&action), "{id} must be selectable; view={view:?}");
    runner.execute_action(view.selecting_player, action).expect("pick revealed card");
}

/// True when `id` is offered at the current reveal prompt.
fn offered(runner: &DebugRunner, id: &str) -> bool {
    let Some(view) = runner.pending_selection_view() else { return false };
    reveal_action(runner, id).is_some_and(|a| view.valid_action_ids.contains(&a))
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn st20_02_identity_and_digivolve_paths() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.dp, Some(1000));
    assert_eq!(card.cost, Some(3));
    assert!(card.traits.iter().any(|t| t == "ADVENTURE"));
    let paths = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .count();
    assert_eq!(paths, 2, "Red Lv.2/0 circle + Lv.2 w/[ADVENTURE]/0 box");
}

#[test]
fn st20_02_clause_shape() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let on_play: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(on_play.len(), 1);
    assert!(!on_play[0].optional, "no 'may' in the printed [On Play]");
    let auras: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope, dp_modifier, active_when, ..
            }) => Some((*scope, *dp_modifier, active_when.is_some())),
            _ => None,
        })
        .collect();
    assert_eq!(auras, vec![(CompiledScope::Inherited, Some(2000), true)]);
}

// ─── Section 2/3 — [On Play] reveal 3 ────────────────────────────────────────

#[test]
fn st20_02_on_play_adds_one_adventure_digimon_and_one_adventure_tamer() {
    let mut runner = play_biyomon(&["FILL", "FILL", "PLAIN-DIGI", "ADV-TAMER", "ADV-DIGI"]);
    assert!(offered(&runner, "ADV-DIGI"), "Digimon bucket offers the ADVENTURE Digimon");
    assert!(!offered(&runner, "PLAIN-DIGI"), "non-ADVENTURE Digimon is not offered");
    assert!(!offered(&runner, "ADV-TAMER"), "Tamer is not a Digimon-bucket candidate");
    assert!(
        !runner.pending_selection_view().unwrap().valid_action_ids.contains(&PASS),
        "a matching Digimon is revealed: the add is mandatory (no PASS)"
    );
    pick(&mut runner, "ADV-DIGI");
    assert!(offered(&runner, "ADV-TAMER"), "Tamer/Option bucket offers the ADVENTURE Tamer");
    pick(&mut runner, "ADV-TAMER");
    let _ = runner.auto_resolve();
    let hand = hand_ids(&runner);
    assert!(hand.contains(&"ADV-DIGI".to_string()));
    assert!(hand.contains(&"ADV-TAMER".to_string()));
    assert!(!hand.contains(&"PLAIN-DIGI".to_string()));
    let deck = ids(&runner, &runner.game.players[0].deck);
    assert_eq!(deck.first().map(String::as_str), Some("PLAIN-DIGI"), "rest to the bottom");
    assert!(runner.game.revealed_cards.is_empty());
}

#[test]
fn st20_02_option_card_fills_the_second_bucket() {
    let mut runner = play_biyomon(&["FILL", "ADV-OPTION", "PLAIN-TAMER", "ADV-DIGI"]);
    pick(&mut runner, "ADV-DIGI");
    assert!(!offered(&runner, "PLAIN-TAMER"), "Tamer without [ADVENTURE] is not offered");
    pick(&mut runner, "ADV-OPTION");
    let _ = runner.auto_resolve();
    let hand = hand_ids(&runner);
    assert!(hand.contains(&"ADV-OPTION".to_string()));
    assert!(!hand.contains(&"PLAIN-TAMER".to_string()));
}

#[test]
fn st20_02_only_the_tamer_option_slot_has_a_candidate() {
    let mut runner = play_biyomon(&["FILL", "FILL", "PLAIN-DIGI", "ADV-TAMER"]);
    let view = runner.pending_selection_view().expect("Tamer/Option bucket prompt");
    assert!(offered(&runner, "ADV-TAMER"));
    assert!(!offered(&runner, "PLAIN-DIGI"));
    assert!(!view.valid_action_ids.contains(&PASS), "matching Tamer revealed: mandatory add");
    pick(&mut runner, "ADV-TAMER");
    let _ = runner.auto_resolve();
    assert_eq!(hand_ids(&runner), vec!["ADV-TAMER"]);
    assert!(runner.pending_selection().is_none());
    assert_eq!(runner.deck_size(0), 3, "the other two revealed cards return to the bottom");
}

#[test]
fn st20_02_only_one_adventure_digimon_may_be_added() {
    let mut runner = play_biyomon(&["FILL", "ADV-DIGI-2", "ADV-DIGI", "ADV-TAMER"]);
    pick(&mut runner, "ADV-DIGI");
    assert!(!offered(&runner, "ADV-DIGI-2"), "the second Digimon cannot fill the Tamer/Option bucket");
    pick(&mut runner, "ADV-TAMER");
    let _ = runner.auto_resolve();
    let hand = hand_ids(&runner);
    assert!(hand.contains(&"ADV-DIGI".to_string()));
    assert!(!hand.contains(&"ADV-DIGI-2".to_string()));
    let deck = ids(&runner, &runner.game.players[0].deck);
    assert_eq!(deck.first().map(String::as_str), Some("ADV-DIGI-2"));
}

#[test]
fn st20_02_no_matching_cards_bottoms_all_three() {
    let runner = {
        let mut r = play_biyomon(&["FILL", "PLAIN-DIGI", "PLAIN-TAMER", "FILL"]);
        let _ = r.auto_resolve();
        r
    };
    assert!(hand_ids(&runner).is_empty(), "nothing added");
    assert_eq!(runner.deck_size(0), 4, "all revealed cards returned");
}

// ─── Section 4 — Digivolve requirement box ───────────────────────────────────

#[test]
fn st20_02_digivolves_from_a_non_red_adventure_lv2_for_0() {
    let mut runner = builder().hand(0, &[CARD_ID]).deck(0, &["FILL", "FILL"]).memory(5).start();
    runner.skip_mulligan();
    let base = runner.place_on_field(0, "ADV-LV2", Some(0));
    let before = runner.memory();
    runner.game.decode_action(encode_digivolve(0, base.index as u16), 0);
    let top = runner.game.players[0].battle_area[base.index as usize].top_card();
    assert_eq!(top.card_id(&runner.game.card_data), CARD_ID);
    assert_eq!(before - runner.memory(), 0);
}

#[test]
fn st20_02_cannot_digivolve_from_a_non_red_non_adventure_lv2() {
    let mut runner = builder().hand(0, &[CARD_ID]).deck(0, &["FILL", "FILL"]).memory(5).start();
    runner.skip_mulligan();
    let base = runner.place_on_field(0, "PLAIN-LV2", Some(0));
    runner.game.decode_action(encode_digivolve(0, base.index as u16), 0);
    let top = runner.game.players[0].battle_area[base.index as usize].top_card();
    assert_eq!(top.card_id(&runner.game.card_data), "PLAIN-LV2");
}

// ─── Section 5 — inherited [Your Turn] +2000 DP ──────────────────────────────

#[test]
fn st20_02_inherited_plus_2000_on_your_turn_only() {
    let mut runner = builder()
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .start();
    runner.skip_mulligan();
    let carrier: PermanentHandle = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    assert_eq!(runner.effective_dp(carrier), Some(4000 + 2000));
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.effective_dp(carrier), Some(4000));
}
