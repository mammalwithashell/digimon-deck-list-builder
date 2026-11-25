use digimon_engine::enums::{CardColor, Keyword};

use super::support::{hand_index, select_hand_card, top_card_id, yellow_sw, DebugRunner};

const CARD_ID: &str = "EX12-043";

#[test]
fn ex12_043_main_plays_sw_from_hand_with_cost_reduced_by_2() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-043 YAML loads")
        .add_card(yellow_sw("SW-PLAY", 3))
        .hand(0, &["SW-PLAY"])
        .memory(10)
        .start();
    let hakubamon = runner.place_on_field(0, CARD_ID, Some(0));
    let field_before = runner.battle_area_size(0);
    let memory_before = runner.memory();

    assert!(
        runner.game.activate_field_main(0, hakubamon.index as usize),
        "Hakubamon's [Main] effect should activate from field"
    );
    select_hand_card(&mut runner, 0, "SW-PLAY");
    runner.auto_resolve().expect("finish Hakubamon [Main]");

    assert_eq!(runner.battle_area_size(0), field_before + 1);
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "SW-PLAY"),
        "the selected [SW] Digimon is played"
    );
    assert_eq!(
        memory_before - runner.memory(),
        1,
        "play cost 3 reduced by 2 spends 1 memory"
    );
}

#[test]
fn ex12_043_inherited_grants_barrier() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-043 YAML loads")
        .add_card(super::support::plain_digimon(
            "CARRIER",
            CardColor::Yellow,
            3,
            3000,
        ))
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);

    assert!(
        runner.game.has_keyword(carrier, Keyword::Barrier),
        "Hakubamon inherited effect grants Barrier"
    );
    assert_eq!(top_card_id(&runner, 0, carrier.index as usize), "CARRIER");
}

// --- Hand-pick candidate legality (select-hand playability audit) ---
// DCGO EX12_043.cs:51-58 `CanSelectCardCondition`: `EqualsTraits("SW") &&
// ((IsOption && !CanNotPlayThisOption) || (HasPlayCost &&
// CanPlayAsNewPermanent(cardSource, payCost: true, activateClass, fixedCost:
// GetCostItself - 2)))`, and the [Main] is only usable while the hand holds
// such a card (EX12_043.cs:45-49 `HasMatchConditionOwnersHand`). Never a
// Digi-Egg (Mococomon EX12-002: no play cost, CR 4.3 7-1-1), never an Option
// whose colour requirement is unmet (CR 4.3 4-22-1..3) or a Digimon under a
// play prohibition / with no free frame, never a card whose reduced cost
// can't be paid (CR 4.3 1-3-11-1 — this also covers Options, which DCGO's
// `CanNotPlayThisOption` does not price; the rule governs).

fn ex12_043_option(id: &str, color: CardColor, cost: u16) -> digimon_engine::card_data::CardData {
    let mut c = yellow_sw(id, 3);
    c.card_kind = digimon_engine::enums::CardKind::Option;
    c.colors = vec![color];
    c.level = None;
    c.dp = None;
    c.play_cost = cost;
    c
}

/// Hakubamon on the field with `hand`; returns `None` when its [Main] can't
/// be activated, else the hand cards the play/use pick offers (sorted).
fn ex12_043_offered(
    hand: &[&str],
    memory: i16,
    tweak: impl FnOnce(&mut DebugRunner),
) -> Option<Vec<String>> {
    use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_START};
    use digimon_engine::selection::SelectionKind;
    let mut big = yellow_sw("SW-BIG", 6);
    big.play_cost = 13;
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-043 YAML loads")
        .dsl_card("EX12-002")
        .expect("Mococomon EX12-002 YAML loads")
        .add_card(yellow_sw("SW-PLAY", 3))
        .add_card(big)
        .add_card(ex12_043_option("SW-OPT-YELLOW", CardColor::Yellow, 2))
        .add_card(ex12_043_option("SW-OPT-BLUE", CardColor::Blue, 2))
        .add_card(ex12_043_option("SW-OPT-BIG", CardColor::Yellow, 13))
        .hand(0, hand)
        .memory(10)
        .start();
    let hakubamon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.enter_main_phase();
    runner.game.memory = memory;
    tweak(&mut runner);
    if !runner.game.activate_field_main(0, hakubamon.index as usize) {
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

/// Mococomon (EX12-002) is an [SW] Digi-Egg: no play cost, not offered.
#[test]
fn ex12_043_pick_excludes_sw_digi_egg() {
    assert_eq!(
        ex12_043_offered(&["EX12-002", "SW-PLAY"], 10, |_| {}),
        Some(vec!["SW-PLAY".to_string()])
    );
}

/// With only Mococomon (EX12-002) in hand there is nothing to play or use, so
/// the [Main] can't be activated (DCGO `CanUseCondition`).
#[test]
fn ex12_043_main_not_activatable_with_only_unplayable_candidates() {
    assert_eq!(ex12_043_offered(&["EX12-002"], 10, |_| {}), None);
}

/// Hakubamon is yellow: a blue [SW] Option's colour requirement is unmet, so
/// it can't be used and isn't offered; the yellow one is.
#[test]
fn ex12_043_pick_excludes_option_whose_colour_requirement_is_unmet() {
    assert_eq!(
        ex12_043_offered(&["SW-OPT-BLUE", "SW-OPT-YELLOW", "SW-PLAY"], 10, |_| {}),
        Some(vec!["SW-OPT-YELLOW".to_string(), "SW-PLAY".to_string()])
    );
}

/// At 0 memory a cost-13 card still costs 11 after the reduction, which can't
/// be paid (CR 4.3 1-3-11-1): neither the cost-13 Digimon nor the cost-13
/// Option is offered; the cost-3 Digimon (pays 1) is.
#[test]
fn ex12_043_pick_excludes_unaffordable_cards() {
    assert_eq!(
        ex12_043_offered(&["SW-BIG", "SW-OPT-BIG", "SW-PLAY"], 0, |_| {}),
        Some(vec!["SW-PLAY".to_string()])
    );
}

/// A full battle area: the Digimon can't be played, the Option can still be
/// used.
#[test]
fn ex12_043_pick_excludes_digimon_when_battle_area_full() {
    assert_eq!(
        ex12_043_offered(&["SW-PLAY", "SW-OPT-YELLOW"], 10, |r| {
            r.game.rules.field_slots = r.game.players[0].battle_area.len() as u8;
        }),
        Some(vec!["SW-OPT-YELLOW".to_string()])
    );
}

/// "You can't play Digimon with 4000 DP or less": the 3000 DP [SW] Digimon is
/// not offered.
#[test]
fn ex12_043_pick_excludes_digimon_under_play_prohibition() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    assert_eq!(
        ex12_043_offered(&["SW-PLAY", "SW-OPT-YELLOW"], 10, |r| {
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
        Some(vec!["SW-OPT-YELLOW".to_string()])
    );
}

// --- [Main] use accounting: owner ruling 2026-11-24 "Match dcgo on the main use" ---
// DCGO EX12_043.cs:179 `if (!isUsed) activateClass.RemoveUse();`, with isUsed
// set only when a card is picked (:95): a passed pick gives the use back,
// a picked card spends it whatever the play then does. The owner chose DCGO
// parity over the CR 4.3 15-14-1-4 / 15-8-4-1/-2 reading that a declared
// [Main] spends. A kept use on an unchanged state is withheld by the
// main-declaration loop guard (docs/RUST_ENGINE_API.md "Main-declaration
// guard") until the state changes.

fn ex12_043_main_bit(field_index: u8) -> usize {
    use digimon_engine::action::space::{EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START};
    (FIELD_EFFECT_START + field_index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN) as usize
}

fn ex12_043_main_offered(r: &DebugRunner, field_index: u8) -> bool {
    digimon_engine::action::build_action_mask(&r.game, 0)[ex12_043_main_bit(field_index)] > 0.5
}

/// Once-per-turn uses recorded on the permanent (a refund leaves no entry).
fn ex12_043_uses(r: &DebugRunner, field_index: u8) -> u32 {
    r.game.players[0].battle_area[field_index as usize]
        .effect_activations
        .values()
        .map(|&n| n as u32)
        .sum()
}

fn ex12_043_use_board() -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-043 YAML loads")
        .add_card(yellow_sw("SW-PLAY", 3))
        .hand(0, &["SW-PLAY"])
        .memory(10)
        .start();
    runner.set_first_player(0);
    runner.game.enter_main_phase();
    runner
}

fn ex12_043_hand(r: &DebugRunner) -> Vec<String> {
    r.game.players[0].hand.iter().map(|c| c.card_id(&r.game.card_data).to_string()).collect()
}

#[test]
fn ex12_043_main_declared_then_pick_passed_keeps_the_once_per_turn() {
    use digimon_engine::action::space::PASS;
    let mut r = ex12_043_use_board();
    let m = r.place_on_field(0, CARD_ID, Some(0));
    let other = r.place_on_field(0, CARD_ID, Some(0));
    let memory = r.memory();
    let field = r.battle_area_size(0);
    let trash = r.game.players[0].trash.len();
    assert!(ex12_043_main_offered(&r, m.index));
    r.game.decode_action(ex12_043_main_bit(m.index) as u16, 0);
    assert_eq!(ex12_043_uses(&r, m.index), 1, "the declaration records the use");
    assert!(r.pending_selection_view().expect("the hand pick").is_optional);
    r.execute_action(0, PASS).expect("pass the pick");
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none(), "the effect ended");
    assert_eq!(ex12_043_hand(&r), vec!["SW-PLAY"], "nothing was played");
    assert_eq!(r.memory(), memory, "no cost was paid");
    assert_eq!(r.battle_area_size(0), field);
    assert_eq!(r.game.players[0].trash.len(), trash);
    assert_eq!(ex12_043_uses(&r, m.index), 0, "DCGO RemoveUse: the passed pick keeps the use");
    assert!(!ex12_043_main_offered(&r, m.index), "loop guard: withheld while nothing changed");
    assert!(ex12_043_main_offered(&r, other.index), "the other copy has its own use");
}

#[test]
fn ex12_043_main_pass_then_state_change_redeclares_and_spends() {
    use digimon_engine::action::space::PASS;
    let mut r = ex12_043_use_board();
    let m = r.place_on_field(0, CARD_ID, Some(0));
    r.game.decode_action(ex12_043_main_bit(m.index) as u16, 0);
    r.execute_action(0, PASS).expect("pass the pick");
    let _ = r.auto_resolve();
    assert!(!ex12_043_main_offered(&r, m.index));
    r.game.memory -= 1;
    assert!(ex12_043_main_offered(&r, m.index), "the state changed: the kept use is offered again");
    let memory = r.memory();
    r.game.decode_action(ex12_043_main_bit(m.index) as u16, 0);
    select_hand_card(&mut r, 0, "SW-PLAY");
    let _ = r.auto_resolve();
    assert!(ex12_043_hand(&r).is_empty(), "the picked card was played");
    assert_eq!(memory - r.memory(), 1, "play cost 3 reduced by 2");
    assert_eq!(ex12_043_uses(&r, m.index), 1, "a picked card spends the use");
    r.game.memory -= 1;
    assert!(!ex12_043_main_offered(&r, m.index), "spent for the turn, whatever else changes");
}
