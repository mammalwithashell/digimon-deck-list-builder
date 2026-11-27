#![allow(dead_code, unused_imports)]
//! EX12 [ME] / Machine / Puppet slice: Hagurumon (EX12-053), Guardromon
//! (EX12-054), ToyAgumon (EX12-008), Kokuwamon (EX12-038), Thundermon
//! (EX12-041), and the Kokuwamon (EX13-046) reveal that must reach Thundermon
//! (EX12-041) through its `(Rule) Name: Treated as including [Mamemon]`.
//!
//! All boards are synthetic; card behaviour is checked against the printed text
//! (data/cards.json), DCGO `EX12_0xx.cs` and CR 4.3.

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledCost, CompiledPredicate};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS, SEL_REVEAL_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::enums::{CardColor, Keyword, PlaySource};
use digimon_engine::selection::SelectionKind;

use super::support::{
    digimon, hand_contains, hand_ids, select_hand_card, trash_contains, DebugRunner,
};

pub(super) fn pred_any<F: Fn(&CompiledPredicate) -> bool + Copy>(p: &CompiledPredicate, f: F) -> bool {
    f(p)
        || p.all_of.iter().any(|q| pred_any(q, f))
        || p.any_of.iter().any(|q| pred_any(q, f))
        || p.none_of.iter().any(|q| pred_any(q, f))
        || p.not.as_ref().is_some_and(|q| pred_any(q, f))
}

pub(super) fn me(id: &str, color: CardColor, level: u8, traits: &[&str]) -> CardData {
    digimon(id, color, level, if level <= 3 { 3 } else { 5 }, 3000, traits)
}

pub(super) fn hand_slot(r: &DebugRunner, id: &str) -> usize {
    r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} must be in P0's hand"))
}

/// Digivolve hand card `id` onto field slot `field_index`; returns the memory
/// it cost.
pub(super) fn digivolve_cost(r: &mut DebugRunner, id: &str, field_index: usize) -> Option<i16> {
    let slot = hand_slot(r, id);
    let before = r.memory();
    r.game
        .digivolve_from_hand(0, slot, field_index, PlaySource::ByHand)
        .then(|| before - r.memory())
}

pub(super) fn deck_with_top<'a>(top: &[&'a str], filler: &'a str, n: usize) -> Vec<&'a str> {
    let mut deck: Vec<&str> = vec![filler; n];
    for c in top.iter().rev() {
        deck.push(c);
    }
    deck
}

pub(super) fn offered_reveal_ids(r: &DebugRunner) -> Vec<String> {
    let view = r.pending_selection_view().expect("reveal prompt pending");
    let mut out: Vec<String> = view
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .filter_map(|&a| a.checked_sub(SEL_REVEAL_START))
        .filter_map(|i| r.game.revealed_cards.get(i as usize))
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    out.sort();
    out
}

pub(super) fn pick_revealed(r: &mut DebugRunner, id: &str) {
    let view = r.pending_selection_view().expect("reveal prompt pending");
    let want = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .find(|&a| {
            a.checked_sub(SEL_REVEAL_START)
                .and_then(|i| r.game.revealed_cards.get(i as usize))
                .is_some_and(|c| c.card_id(&r.game.card_data) == id)
        })
        .unwrap_or_else(|| panic!("{id} must be a legal reveal pick: {view:?}"));
    r.execute_action(view.selecting_player, want)
        .expect("pick revealed card");
}

pub(super) fn deck_bottom_ids(r: &DebugRunner, n: usize) -> Vec<String> {
    r.game.players[0].deck[..n]
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

pub(super) fn assert_me_alt_path(r: &DebugRunner, card_id: &str, level: u8, cost: i32) {
    let card = r.compiled_card(card_id).expect("compiled card");
    let path = card
        .alt_paths
        .iter()
        .find(|p| {
            p.kind == CompiledAltPathKind::Digivolve
                && p.cost == Some(CompiledCost::Literal(cost))
                && p.from.as_ref().is_some_and(|f| {
                    pred_any(f, |q| q.trait_has.as_deref() == Some("ME"))
                        && pred_any(f, |q| q.level_eq == Some(level))
                })
        })
        .unwrap_or_else(|| panic!("{card_id}: printed `Lv.{level} w/[ME] trait: Cost {cost}` path"));
    assert!(path.from.is_some());
}

