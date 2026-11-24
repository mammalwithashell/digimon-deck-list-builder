use digimon_engine::enums::{CardColor, Keyword};

use super::support::{select_hand_card, vb_digimon, DebugRunner};

const CARD_ID: &str = "EX12-013";

#[test]
fn ex12_013_main_plays_vb_card_from_hand_with_cost_reduced_by_two() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-013 YAML loads")
        .add_card(vb_digimon("VB-PLAY", CardColor::Red, 3, 2000))
        .hand(0, &["VB-PLAY"])
        .memory(10)
        .start();
    let betel = runner.place_on_field(0, CARD_ID, Some(0));
    let field_before = runner.battle_area_size(0);
    let memory_before = runner.memory();

    assert!(
        runner.game.activate_field_main(0, betel.index as usize),
        "BetelGammamon [Main] should activate from field"
    );
    select_hand_card(&mut runner, 0, "VB-PLAY");
    runner.auto_resolve().expect("resolve EX12-013 Main");

    assert_eq!(runner.battle_area_size(0), field_before + 1);
    assert_eq!(
        memory_before - runner.memory(),
        1,
        "play cost 3 reduced by 2 spends 1 memory"
    );
}

#[test]
fn ex12_013_inherited_grants_barrier() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-013 YAML loads")
        .add_card(vb_digimon("CARRIER", CardColor::Red, 4, 6000))
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);

    assert!(
        runner.game.has_keyword(carrier, Keyword::Barrier),
        "BetelGammamon inherited effect grants Barrier"
    );
}

// --- Hand-pick candidate legality (select-hand playability audit) ---
// DCGO EX12_013.cs:52-60 `CanSelectCardCondition`: `(HasText("Gammamon") ||
// EqualsTraits("VB")) && ((IsOption && !CanNotPlayThisOption) || (HasPlayCost
// && CanPlayAsNewPermanent(cardSource, payCost: true, activateClass,
// fixedCost: GetCostItself - 2)))`, and the [Main] is only usable while the
// hand holds such a card (EX12_013.cs:46-50 `HasMatchConditionOwnersHand`).
// Never a Digi-Egg (no play cost, CR 4.3 7-1-1), never an Option whose colour
// requirement is unmet (CR 4.3 4-22-1..3) or a Digimon under a play
// prohibition / with no free frame, never a card whose reduced cost can't be
// paid (CR 4.3 1-3-11-1 — this also covers Options, which DCGO's
// `CanNotPlayThisOption` does not price; the rule governs).

fn ex12_013_card(
    id: &str,
    kind: digimon_engine::enums::CardKind,
    color: CardColor,
    cost: u16,
) -> digimon_engine::card_data::CardData {
    use digimon_engine::enums::CardKind;
    let mut c = vb_digimon(id, color, 3, 3000);
    c.card_kind = kind;
    c.play_cost = cost;
    if kind != CardKind::Digimon {
        c.dp = None;
    }
    if kind == CardKind::DigiEgg {
        c.level = Some(2);
    }
    if kind == CardKind::Option {
        c.level = None;
    }
    c
}

/// BetelGammamon on the field with `hand`; returns `None` when its [Main]
/// can't be activated, else the hand cards the play/use pick offers (sorted).
fn ex12_013_offered(
    hand: &[&str],
    memory: i16,
    tweak: impl FnOnce(&mut DebugRunner),
) -> Option<Vec<String>> {
    use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_START};
    use digimon_engine::enums::CardKind;
    use digimon_engine::selection::SelectionKind;
    let mut big = vb_digimon("VB-BIG", CardColor::Red, 6, 13000);
    big.play_cost = 13;
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-013 YAML loads")
        .add_card(vb_digimon("VB-PLAY", CardColor::Red, 3, 3000))
        .add_card(big)
        .add_card(ex12_013_card("VB-EGG", CardKind::DigiEgg, CardColor::Red, 0))
        .add_card(ex12_013_card("VB-OPT-RED", CardKind::Option, CardColor::Red, 2))
        .add_card(ex12_013_card("VB-OPT-GREEN", CardKind::Option, CardColor::Green, 2))
        .add_card(ex12_013_card("VB-OPT-BIG", CardKind::Option, CardColor::Red, 13))
        .hand(0, hand)
        .memory(10)
        .start();
    let betel = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.enter_main_phase();
    runner.game.memory = memory;
    tweak(&mut runner);
    if !runner.game.activate_field_main(0, betel.index as usize) {
        return None;
    }
    let Some(view) = runner.pending_selection_view() else {
        return Some(Vec::new());
    };
    if view.kind != SelectionKind::Hand {
        return Some(Vec::new());
    }
    let hand_ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    let mut offered: Vec<String> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&id| id != PASS)
        .map(|id| {
            let slot = if id >= HAND_EFFECT_START { id - HAND_EFFECT_START } else { id - PLAY_HAND_START };
            hand_ids[slot as usize].clone()
        })
        .collect();
    offered.sort();
    Some(offered)
}

/// A [VB] Digi-Egg (e.g. Gurimon BT21-002, [Gammamon] in its text) has no
/// play cost and can't be played: not offered.
#[test]
fn ex12_013_pick_excludes_digi_egg() {
    assert_eq!(
        ex12_013_offered(&["VB-EGG", "VB-PLAY"], 10, |_| {}),
        Some(vec!["VB-PLAY".to_string()])
    );
}

/// With only a Digi-Egg in hand there is nothing to play or use, so the
/// [Main] can't be activated (DCGO `CanUseCondition`).
#[test]
fn ex12_013_main_not_activatable_with_only_unplayable_candidates() {
    assert_eq!(ex12_013_offered(&["VB-EGG"], 10, |_| {}), None);
}

/// BetelGammamon is red/yellow: a green [VB] Option's colour requirement is
/// unmet, so it can't be used and isn't offered; the red one is.
#[test]
fn ex12_013_pick_excludes_option_whose_colour_requirement_is_unmet() {
    assert_eq!(
        ex12_013_offered(&["VB-OPT-GREEN", "VB-OPT-RED", "VB-PLAY"], 10, |_| {}),
        Some(vec!["VB-OPT-RED".to_string(), "VB-PLAY".to_string()])
    );
}

/// At 0 memory a cost-13 card still costs 11 after the reduction, which can't
/// be paid (CR 4.3 1-3-11-1): neither the cost-13 Digimon nor the cost-13
/// Option is offered; the cost-3 Digimon (pays 1) is.
#[test]
fn ex12_013_pick_excludes_unaffordable_cards() {
    assert_eq!(
        ex12_013_offered(&["VB-BIG", "VB-OPT-BIG", "VB-PLAY"], 0, |_| {}),
        Some(vec!["VB-PLAY".to_string()])
    );
}

/// A full battle area: the Digimon can't be played, the Option can still be
/// used.
#[test]
fn ex12_013_pick_excludes_digimon_when_battle_area_full() {
    assert_eq!(
        ex12_013_offered(&["VB-PLAY", "VB-OPT-RED"], 10, |r| {
            r.game.rules.field_slots = r.game.players[0].battle_area.len() as u8;
        }),
        Some(vec!["VB-OPT-RED".to_string()])
    );
}

/// "You can't play Digimon with 4000 DP or less": the 3000 DP [VB] Digimon is
/// not offered.
#[test]
fn ex12_013_pick_excludes_digimon_under_play_prohibition() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    assert_eq!(
        ex12_013_offered(&["VB-PLAY", "VB-OPT-RED"], 10, |r| {
            r.game.modifiers.add_player_modifier(
                0,
                PlayerModifierEntry::simple(
                    ModifierType::CannotPlayDigimonWithDPAtMost,
                    4000,
                    Expiry::Permanent,
                    None,
                    1,
                ),
            );
        }),
        Some(vec!["VB-OPT-RED".to_string()])
    );
}

// --- [Main] use accounting: owner ruling 2026-11-24 "Match dcgo on the main use" ---
// DCGO EX12_013.cs:181 `if (!isUsed) activateClass.RemoveUse();`, with isUsed
// set only when a card is picked (:97): a passed pick gives the use back,
// a picked card spends it whatever the play then does. The owner chose DCGO
// parity over the CR 4.3 15-14-1-4 / 15-8-4-1/-2 reading that a declared
// [Main] spends. A kept use on an unchanged state is withheld by the
// main-declaration loop guard (docs/RUST_ENGINE_API.md "Main-declaration
// guard") until the state changes.

fn ex12_013_main_bit(field_index: u8) -> usize {
    use digimon_engine::action::space::{EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START};
    (FIELD_EFFECT_START + field_index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN) as usize
}

fn ex12_013_main_offered(r: &DebugRunner, field_index: u8) -> bool {
    digimon_engine::action::build_action_mask(&r.game, 0)[ex12_013_main_bit(field_index)] > 0.5
}

/// Once-per-turn uses recorded on the permanent (a refund leaves no entry).
fn ex12_013_uses(r: &DebugRunner, field_index: u8) -> u32 {
    r.game.players[0].battle_area[field_index as usize]
        .effect_activations
        .values()
        .map(|&n| n as u32)
        .sum()
}

fn ex12_013_use_board() -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-013 YAML loads")
        .add_card(vb_digimon("VB-PLAY", CardColor::Red, 3, 2000))
        .hand(0, &["VB-PLAY"])
        .memory(10)
        .start();
    runner.set_first_player(0);
    runner.game.enter_main_phase();
    runner
}

fn ex12_013_hand(r: &DebugRunner) -> Vec<String> {
    r.game.players[0].hand.iter().map(|c| c.card_id(&r.game.card_data).to_string()).collect()
}

#[test]
fn ex12_013_main_declared_then_pick_passed_keeps_the_once_per_turn() {
    use digimon_engine::action::space::PASS;
    let mut r = ex12_013_use_board();
    let m = r.place_on_field(0, CARD_ID, Some(0));
    let other = r.place_on_field(0, CARD_ID, Some(0));
    let memory = r.memory();
    let field = r.battle_area_size(0);
    let trash = r.game.players[0].trash.len();
    assert!(ex12_013_main_offered(&r, m.index));
    r.game.decode_action(ex12_013_main_bit(m.index) as u16, 0);
    assert_eq!(ex12_013_uses(&r, m.index), 1, "the declaration records the use");
    assert!(r.pending_selection_view().expect("the hand pick").is_optional);
    r.execute_action(0, PASS).expect("pass the pick");
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none(), "the effect ended");
    assert_eq!(ex12_013_hand(&r), vec!["VB-PLAY"], "nothing was played");
    assert_eq!(r.memory(), memory, "no cost was paid");
    assert_eq!(r.battle_area_size(0), field);
    assert_eq!(r.game.players[0].trash.len(), trash);
    assert_eq!(ex12_013_uses(&r, m.index), 0, "DCGO RemoveUse: the passed pick keeps the use");
    assert!(!ex12_013_main_offered(&r, m.index), "loop guard: withheld while nothing changed");
    assert!(ex12_013_main_offered(&r, other.index), "the other copy has its own use");
}

#[test]
fn ex12_013_main_pass_then_state_change_redeclares_and_spends() {
    use digimon_engine::action::space::PASS;
    let mut r = ex12_013_use_board();
    let m = r.place_on_field(0, CARD_ID, Some(0));
    r.game.decode_action(ex12_013_main_bit(m.index) as u16, 0);
    r.execute_action(0, PASS).expect("pass the pick");
    let _ = r.auto_resolve();
    assert!(!ex12_013_main_offered(&r, m.index));
    r.game.memory -= 1;
    assert!(ex12_013_main_offered(&r, m.index), "the state changed: the kept use is offered again");
    let memory = r.memory();
    r.game.decode_action(ex12_013_main_bit(m.index) as u16, 0);
    select_hand_card(&mut r, 0, "VB-PLAY");
    let _ = r.auto_resolve();
    assert!(ex12_013_hand(&r).is_empty(), "the picked card was played");
    assert_eq!(memory - r.memory(), 1, "play cost 3 reduced by 2");
    assert_eq!(ex12_013_uses(&r, m.index), 1, "a picked card spends the use");
    r.game.memory -= 1;
    assert!(!ex12_013_main_offered(&r, m.index), "spent for the turn, whatever else changes");
}
