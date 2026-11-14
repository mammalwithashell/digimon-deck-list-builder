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
