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
