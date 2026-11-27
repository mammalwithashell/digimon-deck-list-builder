#![allow(dead_code, unused_imports)]
//! EX12 [ME] / Machine big-Digimon slice: Metal Empire (EX12-072), Andromon
//! (EX12-055), HiAndromon (EX12-058), Megadramon (EX12-064), Machinedramon
//! (EX12-059), Chaosdramon (EX12-060) and Kapurimon (EX12-003).
//!
//! All boards are synthetic; behaviour is checked against the printed text
//! (card faces), DCGO `EX12_0xx.cs` and CR 4.3.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::enums::{CardColor, Keyword, ModifierType};
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

use super::support::{
    digimon, field_contains, hand_contains, hand_index, push_to_trash, select_hand_card,
    source_count, trash_contains, DebugRunner,
};

pub(super) fn me(id: &str, color: CardColor, level: u8, cost: u16, traits: &[&str]) -> CardData {
    digimon(id, color, level, cost, 3000, traits)
}

pub(super) fn triggered_clauses(r: &DebugRunner, card: &str) -> Vec<(CompiledScope, Vec<CompiledTiming>)> {
    r.compiled_card(card)
        .expect("compiled card")
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Triggered(t) => Some((t.scope, t.when.clone())),
            _ => None,
        })
        .collect()
}

pub(super) fn reveal_offered(r: &DebugRunner) -> Vec<String> {
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

pub(super) fn reveal_pick(r: &mut DebugRunner, id: &str) {
    let view = r.pending_selection_view().expect("reveal prompt pending");
    let a = view
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
    r.execute_action(view.selecting_player, a).expect("pick revealed card");
}

pub(super) fn deck_top(top: &[&'static str]) -> Vec<&'static str> {
    let mut deck: Vec<&str> = vec!["FILL"; 4];
    for c in top.iter().rev() {
        deck.push(c);
    }
    deck
}


// ═════════════════════════ Kapurimon (EX12-003) ═════════════════════════

#[test]
fn ex12_003_is_an_inherited_would_leave_replacement() {
    let r = DebugRunner::builder()
        .dsl_card("EX12-003")
        .expect("EX12-003 YAML loads")
        .start();
    let card = r.compiled_card("EX12-003").expect("compiled");
    assert!(
        card.effects.iter().any(|e| matches!(e, CompiledClause::Declarative(CompiledDeclarativeClause::Replacement { .. }))),
        "[All Turns] 'would leave the battle area' is a replacement-window clause"
    );
}
