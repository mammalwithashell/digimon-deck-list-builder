//! EX7-043 Tankmon — Digimon, Lv.4, Black, DP 5000, Cost 5.
//! Traits: Cyborg. Form: Champion. Attribute: Data.
//!
//! # Card text (data/card_bundles/EX7-043.md — official Bandai DB, verbatim)
//!
//! [On Play] [When Digivolving] By returning 3 cards with the [Three
//! Musketeers] trait from your hand or trash to the top of the deck,
//! ＜De-Digivolve 1＞ 1 of your opponent's Digimon.
//!
//! Inherited Effect: ＜Reboot＞ (This Digimon also unsuspends in your
//! opponent's unsuspend phase.)
//!
//! Official Q&A: "No, you can choose 3 cards with the [Three Musketeers]
//! trait to return to the top of the deck from your hand, trash, or both.
//! For example, you can choose 3 cards from your hand or choose a combination
//! of 1 card from your hand and 2 cards from your trash."
//!
//! Digivolution requirements (official Bandai DB):
//!   - Standard circle: Black Lv.3 / cost 2
//!   - xros_req: "[Digivolve] Lv.3 w/[Three Musketeers] in text: Cost 2"
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX7/Black/EX7_043.cs
//!
//! # DCGO crosscheck
//! - Alt-digivolve: `IsLevel3 && HasText("Three Musketeers")`, cost 2.
//! - Shared OP/WD: two ActivateClass entries (isOptional TRUE);
//!   SharedCanActivateCondition = `hand.Count(TM) + trash.Count(TM) >= 3`.
//!   Body: SelectHandEffect (up to 3, canNoSelect, canEndNotMax) then
//!   SelectCardEffect(Root.Trash, up to 3 - picked); `if (selected == 3)`:
//!   the player ORDERS the 3 cards, they go to the deck top, then (if an
//!   opponent Digimon exists) mandatory SelectPermanentEffect →
//!   `IDegeneration(permanent, 1)`.
//!   Authored as three sequential `select_union_zone` picks each immediately
//!   returned to the deck TOP — the pick order IS the deck order (the last
//!   pick ends on top), so every ordering DCGO's "specify the order" prompt
//!   can reach is reachable here too (G-DSL-RETURN-UNION-BOUND-TO-DECK).
//! - ESS: RebootSelfStaticEffect(isInheritedEffect: true).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - E2: declinable 3-card union (hand ∪ trash) return-to-deck-top cost
//! - De-Digivolve 1 on an opponent Digimon
//! - H7: inherited <Reboot>

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::action::space::{PASS, PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Keyword, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX7-043";

fn digimon(id: &str, level: u8, color: CardColor, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.colors = vec![CardColor::Green];
    c
}

fn zone_ids(cards: &[digimon_engine::card_source::CardSource], data: &[CardData]) -> Vec<String> {
    cards.iter().map(|c| c.card_id(data).to_string()).collect()
}

fn sources_of(runner: &DebugRunner, h: PermanentHandle) -> usize {
    runner.game.players[h.player as usize].battle_area[h.index as usize]
        .card_sources
        .len()
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX7-043 YAML loads from the embedded pack")
        .add_card(digimon("TM-1", 3, CardColor::Black, &["Three Musketeers"]))
        .add_card(digimon("TM-2", 3, CardColor::Black, &["Three Musketeers"]))
        .add_card(digimon("TM-3", 3, CardColor::Black, &["Three Musketeers"]))
        .add_card(digimon("TM-4", 3, CardColor::Black, &["Three Musketeers"]))
        .add_card(digimon("PLAIN", 3, CardColor::Black, &["Cyborg"]))
        .add_card(digimon("OPP-LV3", 3, CardColor::Blue, &[]))
        .add_card(digimon("OPP-LV4", 4, CardColor::Blue, &[]))
        .add_card(digimon("OPP-LV5", 5, CardColor::Blue, &[]))
        .add_card(digimon("BLK-LV3", 3, CardColor::Black, &[]))
        .add_card(filler("FILL"))
        .add_card(digimon("CARRIER", 5, CardColor::Black, &[]))
        .deck(0, &["FILL"; 4])
        .memory(10)
}

fn hand_pick(runner: &DebugRunner, card_id: &str) -> u16 {
    let idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not in hand"));
    PLAY_HAND_START + idx as u16
}

fn trash_pick(runner: &DebugRunner, card_id: &str) -> u16 {
    let idx = runner.game.players[0]
        .trash
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not in trash"));
    TRASH_EFFECT_START + idx as u16
}

fn assert_union_prompt(runner: &DebugRunner, optional: bool) {
    let view = runner.pending_selection_view().expect("union prompt");
    assert!(matches!(view.kind, SelectionKind::UnionZone { .. }), "kind={:?}", view.kind);
    assert_eq!(runner.pending_is_optional(), optional, "PASS legality");
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_043_structure_matches_printed_text() {
    let runner = base().start();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled");
    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1, "one shared [On Play][When Digivolving] clause");
    assert_eq!(triggered[0].when, vec![CompiledTiming::OnPlay, CompiledTiming::WhenDigivolving]);
    assert!(triggered[0].optional, "'By returning ...' is declinable");
    assert!(!triggered[0].once_per_turn);

    let reboot = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { scope, keyword, .. })
                if *scope == CompiledScope::Inherited && keyword == "Reboot"
        )
    });
    assert!(reboot, "inherited <Reboot>");
    let paths = compiled
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .count();
    assert_eq!(paths, 2);
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — Condition gating
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_043_on_play_no_prompt_with_only_two_tm_cards() {
    let mut runner = base().hand(0, &[CARD_ID, "TM-1"]).start();
    runner.inject_trash(0, "TM-2");
    runner.place_on_field(1, "OPP-LV4", Some(0));
    runner.play(0, 0).expect("play Tankmon");
    assert!(
        runner.pending_selection().is_none(),
        "the cost needs 3 [Three Musketeers] cards across hand+trash; 2 → unpayable"
    );
}

#[test]
fn ex7_043_on_play_prompts_with_three_tm_cards_across_hand_and_trash() {
    let mut runner = base().hand(0, &[CARD_ID, "TM-1"]).start();
    runner.inject_trash(0, "TM-2");
    runner.inject_trash(0, "TM-3");
    runner.play(0, 0).expect("play Tankmon");
    assert_union_prompt(&runner, true);
}

#[test]
fn ex7_043_non_tm_cards_are_not_selectable() {
    let mut runner = base().hand(0, &[CARD_ID, "TM-1", "PLAIN"]).start();
    runner.inject_trash(0, "TM-2");
    runner.inject_trash(0, "PLAIN");
    runner.inject_trash(0, "TM-3");
    runner.play(0, 0).expect("play Tankmon");
    let view = runner.pending_selection_view().expect("union prompt");
    let picks: Vec<u16> = view.valid_action_ids.iter().copied().filter(|&a| a != PASS).collect();
    assert_eq!(picks.len(), 3, "TM-1 (hand) + TM-2/TM-3 (trash); PLAIN excluded in both zones");
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — Behavioral outcome
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_043_returns_three_to_deck_top_in_pick_order_then_de_digivolves_opponent() {
    let mut runner = base().hand(0, &[CARD_ID, "TM-1", "TM-2"]).start();
    runner.inject_trash(0, "TM-3");
    let opp = runner.place_stack(1, &["OPP-LV3", "OPP-LV4"]);
    let deck_before = runner.deck_size(0);
    runner.play(0, 0).expect("play Tankmon");

    // Pick 1 (declinable), pick 2 and 3 (mandatory once engaged).
    assert_union_prompt(&runner, true);
    runner.execute_action(0, hand_pick(&runner, "TM-2")).expect("pick TM-2");
    assert_union_prompt(&runner, false);
    runner.execute_action(0, trash_pick(&runner, "TM-3")).expect("pick TM-3");
    assert_union_prompt(&runner, false);
    runner.execute_action(0, hand_pick(&runner, "TM-1")).expect("pick TM-1");

    let view = runner.pending_selection_view().expect("De-Digivolve target prompt");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(!view.valid_action_ids.contains(&PASS), "mandatory once the cost is paid");
    runner.execute_action(0, view.valid_action_ids[0]).expect("choose OPP");
    runner.auto_resolve().expect("finish");

    assert_eq!(runner.deck_size(0), deck_before + 3, "3 cards returned to the deck");
    let deck = zone_ids(&runner.game.players[0].deck, &runner.game.card_data);
    let n = deck.len();
    assert_eq!(&deck[n - 3..], &["TM-2", "TM-3", "TM-1"], "deck top (last) = last pick; order = pick order");
    assert_eq!(runner.hand_size(0), 0);
    assert_eq!(runner.trash_size(0), 0, "TM-3 left the trash");
    assert_eq!(sources_of(&runner, opp), 1, "De-Digivolve 1: OPP-LV4 trashed, OPP-LV3 is the new top");
    let top = runner.game.players[1].battle_area[opp.index as usize]
        .top_card()
        .card_id(&runner.game.card_data);
    assert_eq!(top, "OPP-LV3");
    assert!(zone_ids(&runner.game.players[1].trash, &runner.game.card_data).contains(&"OPP-LV4".to_string()));
}

#[test]
fn ex7_043_decline_first_pick_returns_nothing_and_skips_de_digivolve() {
    let mut runner = base().hand(0, &[CARD_ID, "TM-1", "TM-2", "TM-3"]).start();
    let opp = runner.place_stack(1, &["OPP-LV3", "OPP-LV4"]);
    let deck_before = runner.deck_size(0);
    runner.play(0, 0).expect("play Tankmon");
    runner.execute_action(0, PASS).expect("decline");
    assert!(runner.pending_selection().is_none());
    assert_eq!(runner.deck_size(0), deck_before);
    assert_eq!(runner.hand_size(0), 3, "all TM cards stay in hand");
    assert_eq!(sources_of(&runner, opp), 2, "no De-Digivolve");
}

#[test]
fn ex7_043_cost_paid_with_no_opponent_digimon_still_returns_cards() {
    let mut runner = base().hand(0, &[CARD_ID, "TM-1", "TM-2", "TM-3"]).start();
    let deck_before = runner.deck_size(0);
    runner.play(0, 0).expect("play Tankmon");
    runner.execute_action(0, hand_pick(&runner, "TM-1")).expect("pick 1");
    runner.execute_action(0, hand_pick(&runner, "TM-2")).expect("pick 2");
    runner.execute_action(0, hand_pick(&runner, "TM-3")).expect("pick 3");
    assert!(runner.pending_selection().is_none(), "no opponent Digimon → no De-Digivolve prompt");
    assert_eq!(runner.deck_size(0), deck_before + 3);
}

#[test]
fn ex7_043_de_digivolve_stops_at_level_three() {
    let mut runner = base().hand(0, &[CARD_ID, "TM-1", "TM-2", "TM-3"]).start();
    let opp = runner.place_on_field(1, "OPP-LV3", Some(0));
    runner.play(0, 0).expect("play Tankmon");
    runner.execute_action(0, hand_pick(&runner, "TM-1")).expect("pick 1");
    runner.execute_action(0, hand_pick(&runner, "TM-2")).expect("pick 2");
    runner.execute_action(0, hand_pick(&runner, "TM-3")).expect("pick 3");
    let view = runner.pending_selection_view().expect("target prompt (a Lv.3 is still a legal target)");
    runner.execute_action(0, view.valid_action_ids[0]).expect("choose OPP-LV3");
    runner.auto_resolve().expect("finish");
    assert_eq!(runner.battle_area_size(1), 1, "a lone Lv.3 cannot be de-digivolved past level 3");
}

#[test]
fn ex7_043_when_digivolving_offers_the_same_cost() {
    let mut runner = base().hand(0, &[CARD_ID, "TM-1", "TM-2"]).start();
    runner.inject_trash(0, "TM-3");
    let base_perm = runner.place_on_field(0, "BLK-LV3", Some(0));
    let opp = runner.place_stack(1, &["OPP-LV4", "OPP-LV5"]);
    let memory_before = runner.memory();
    assert!(runner.game.digivolve_from_hand(0, 0, base_perm.index as usize, PlaySource::ByHand));
    assert_eq!(runner.memory(), memory_before - 2, "Black Lv.3 / cost 2");
    assert_union_prompt(&runner, true);
    runner.execute_action(0, hand_pick(&runner, "TM-1")).expect("pick 1");
    runner.execute_action(0, hand_pick(&runner, "TM-2")).expect("pick 2");
    runner.execute_action(0, trash_pick(&runner, "TM-3")).expect("pick 3");
    let view = runner.pending_selection_view().expect("target prompt");
    runner.execute_action(0, view.valid_action_ids[0]).expect("choose OPP");
    runner.auto_resolve().expect("finish");
    assert_eq!(sources_of(&runner, opp), 1, "OPP-LV5 trashed → OPP-LV4 on top");
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 4 — Inherited <Reboot> + digivolution paths
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_043_inherited_reboot_grants_keyword_to_carrier() {
    let mut runner = base().start();
    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    assert!(runner.game.has_keyword(stack, Keyword::Reboot));
    let alone = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(!runner.game.has_keyword(alone, Keyword::Reboot), "inherited only");
}

#[test]
fn ex7_043_digivolves_from_red_lv3_with_tm_in_text_for_two() {
    let mut red = digimon("RED-TM-LV3", 3, CardColor::Red, &[]);
    red.effect_text = "... [Three Musketeers] ...".to_string();
    let mut runner = base().add_card(red).hand(0, &[CARD_ID]).start();
    let base_perm = runner.place_on_field(0, "RED-TM-LV3", Some(0));
    let memory_before = runner.memory();
    assert!(runner.game.digivolve_from_hand(0, 0, base_perm.index as usize, PlaySource::ByHand));
    assert_eq!(runner.memory(), memory_before - 2);
}

#[test]
fn ex7_043_cannot_digivolve_from_plain_red_lv3() {
    let mut runner = base()
        .add_card(digimon("RED-LV3", 3, CardColor::Red, &[]))
        .hand(0, &[CARD_ID])
        .start();
    let base_perm = runner.place_on_field(0, "RED-LV3", Some(0));
    assert!(!runner.game.digivolve_from_hand(0, 0, base_perm.index as usize, PlaySource::ByHand));
}
