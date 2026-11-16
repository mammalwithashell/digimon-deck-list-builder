//! BT26-012 Manekimon — Lv.4 Red/Yellow (Puppet / Shambala / TB).
//!
//! [Main] [Once Per Turn] You may play or use 1 [TB] trait card from your
//! hand with the cost reduced by 2.
//! Inherited: [When Attacking] [Once Per Turn] 1 of your opponent's Digimon
//! gets -2000 DP for the turn.
//!
//! DCGO: BT26/Red/BT26_012.cs.

use super::support::*;
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PLAY_HAND_START,
};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, GamePhase};

const CARD_ID: &str = "BT26-012";

fn main_action(field_index: u8) -> u16 {
    FIELD_EFFECT_START + field_index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN
}

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-012")
        .add_card(filler("FILLER"))
        .add_card(digimon("TBMON", "TBmon", CardColor::Red, 4, 5, &["TB"]))
        .add_card(tamer("TBTAMER", "TB Tamer", CardColor::Red, &["TB"]))
        .add_card(digimon("PLAIN", "Plainmon", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("HOST", "Host", CardColor::Red, 5, 7, &[]))
        .add_card(digimon("OPP4", "Opp4", CardColor::Blue, 4, 5, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(0, &["FILLER"; 3])
        .security(1, &["FILLER"; 3])
        .memory(5)
        .start();
    r.set_first_player(0);
    r.set_phase(GamePhase::Main);
    r
}

fn mask_has(r: &DebugRunner, a: u16) -> bool {
    build_action_mask(&r.game, 0)[a as usize] == 1.0
}

#[test]
fn bt26_012_alt_path_shambala() {
    let r = setup();
    let alt = format!("{:?}", r.compiled_card(CARD_ID).unwrap().alt_paths);
    assert!(
        alt.contains("Shambala") && alt.contains("level_eq: Some(3)"),
        "{alt}"
    );
}

#[test]
fn bt26_012_main_plays_tb_digimon_with_cost_reduced_by_2() {
    let mut r = setup();
    push_hand(&mut r, 0, "PLAIN");
    push_hand(&mut r, 0, "TBMON");
    let m = r.place_on_field(0, CARD_ID, Some(0));
    let a = main_action(m.index);
    assert!(mask_has(&r, a));
    let mem = r.memory();
    r.game.decode_action(a, 0);
    let v = r.pending_selection_view().expect("hand pick");
    assert!(v.is_optional);
    assert!(
        !v.valid_action_ids.contains(&PLAY_HAND_START),
        "PLAIN is not [TB]"
    );
    pick_hand(&mut r, 0, "TBMON");
    assert_eq!(field_ids(&r, 0), vec![CARD_ID, "TBMON"]);
    assert_eq!((mem - r.memory()).abs(), 3, "cost 5 - 2");
    // Once per turn.
    push_hand(&mut r, 0, "TBTAMER");
    assert!(!mask_has(&r, a));
}

#[test]
fn bt26_012_main_plays_tb_tamer() {
    let mut r = setup();
    push_hand(&mut r, 0, "TBTAMER");
    let m = r.place_on_field(0, CARD_ID, Some(0));
    let mem = r.memory();
    r.game.decode_action(main_action(m.index), 0);
    pick_hand(&mut r, 0, "TBTAMER");
    assert_eq!(field_ids(&r, 0), vec![CARD_ID, "TBTAMER"]);
    assert_eq!((mem - r.memory()).abs(), 1, "cost 3 - 2");
}

#[test]
fn bt26_012_declining_after_activation_spends_once_per_turn() {
    let mut r = setup();
    push_hand(&mut r, 0, "TBMON");
    let m = r.place_on_field(0, CARD_ID, Some(0));
    let other = r.place_on_field(0, CARD_ID, Some(0));
    let a = main_action(m.index);
    let memory = r.memory();
    assert!(mask_has(&r, a));
    r.game.decode_action(a, 0);
    pass(&mut r, 0);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(hand_ids(&r, 0), vec!["TBMON"]);
    assert_eq!(
        r.memory(),
        memory,
        "declining plays nothing and pays no memory"
    );
    assert!(
        !mask_has(&r, a),
        "CR 4.3 15-14-1-4: activation spends the use even when processing is declined"
    );
    assert!(
        mask_has(&r, main_action(other.index)),
        "each copy has its own use"
    );
}

#[test]
fn bt26_012_main_unavailable_without_tb_card() {
    let mut r = setup();
    push_hand(&mut r, 0, "PLAIN");
    let m = r.place_on_field(0, CARD_ID, Some(0));
    assert!(!mask_has(&r, main_action(m.index)));
}

#[test]
fn bt26_012_inherited_when_attacking_minus_2000_once_per_turn() {
    let mut r = setup();
    let opp = r.place_on_field(1, "OPP4", Some(0));
    let c = r.place_stack(0, &[CARD_ID, "HOST"]);
    fire(&mut r, EffectTiming::WhenAttacking, c);
    if r.pending_selection_view().is_some() {
        pick_first(&mut r, 0);
    }
    assert_eq!(r.effective_dp(opp), Some(2000));
    fire(&mut r, EffectTiming::WhenAttacking, c);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.effective_dp(opp), Some(2000), "once per turn");
    r.end_turn();
    assert_eq!(r.effective_dp(opp), Some(4000), "for the turn");
}

// ─── Pick legality (select-hand audit 2026-11-13) ────────────────────────────
//
// DCGO BT26_012.cs:42-44 `CanSelectCardCondition`: `EqualsTraits("TB") &&
// CanPlayOrUse(card, activateClass, fixedCost: Math.Max(0, GetCostItself - 2))`,
// and the [Main] is only declarable with such a card in hand (`CanUseCondition`,
// BT26_012.cs:38-40). `CanPlayOrUse` (PlayEffects.cs:286-294) = an Option that
// can be used (`!CanNotPlayThisOption`: no "can't use Options" lock and its
// colour requirement met, CR 4.3 4-22-1..3; Q1500, Q2785) and whose reduced
// cost is payable, or a card with a play cost (never a Digi-Egg, CR 4.3 7-1-1)
// that `CanPlayAsNewPermanent` (empty frame, no prohibition, reduced cost
// payable). A card that can't be played or used stays in hand (CR 4.3 7-1-2-3,
// 1-3-2). YAML: `can_play_or_use_from_effect` + `affordable_with_cost: {reduce:
// 2}` on the pick and on the activation condition.

fn tb_option(id: &str, color: CardColor, cost: u16) -> digimon_engine::card_data::CardData {
    let mut c = digimon_engine::debug_runner::make_test_card(id, id);
    c.card_kind = digimon_engine::enums::CardKind::Option;
    c.colors = vec![color];
    c.level = None;
    c.dp = None;
    c.play_cost = cost;
    c.traits = vec!["TB".to_string()];
    c
}

fn audit_setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-012")
        .dsl_card("EX12-004")
        .expect("Onibimon (EX12-004)")
        .add_card(filler("FILLER"))
        .add_card(digimon("TBMON", "TBmon", CardColor::Red, 4, 5, &["TB"]))
        .add_card(digimon("TB-17", "TB17", CardColor::Red, 6, 17, &["TB"]))
        .add_card(digimon("TB-18", "TB18", CardColor::Red, 6, 18, &["TB"]))
        .add_card(tamer("TBTAMER", "TB Tamer", CardColor::Red, &["TB"]))
        .add_card(tb_option("TBOPT-RED", CardColor::Red, 2))
        .add_card(tb_option("TBOPT-BLUE", CardColor::Blue, 2))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(0, &["FILLER"; 3])
        .security(1, &["FILLER"; 3])
        .memory(5)
        .start();
    r.set_first_player(0);
    r.set_phase(GamePhase::Main);
    r
}

/// The non-PASS hand candidates of the pending pick, as sorted card ids; empty
/// when no hand pick is pending.
fn offered_hand_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    use digimon_engine::action::space::PASS;
    use digimon_engine::owned_card_choice::{CardChoiceOperation, CardChoiceZone};
    let Some(view) = r.pending_selection_view() else {
        return Vec::new();
    };
    let hand = &r.game.players[p as usize].hand;
    let id_at = |i: usize| hand.get(i).map(|c| c.card_id(&r.game.card_data).to_string());
    let mut ids: Vec<String> = match view.card_choice {
        Some(choice) => choice
            .rows
            .iter()
            .filter(|row| {
                matches!(row.operation, CardChoiceOperation::CommitCard)
                    && matches!(row.zone, Some(CardChoiceZone::Hand))
            })
            .filter_map(|row| row.position.as_deref()?.parse::<usize>().ok())
            .filter_map(id_at)
            .collect(),
        None => view
            .valid_action_ids
            .iter()
            .copied()
            .filter(|&a| a != PASS)
            .filter_map(|a| a.checked_sub(PLAY_HAND_START))
            .filter_map(|i| id_at(i as usize))
            .collect(),
    };
    ids.sort();
    ids
}

/// Seat Manekimon, put `hand` in hand, run `setup`, activate the [Main] and
/// return the candidates.
fn main_offered(hand: &[&str], setup: impl FnOnce(&mut DebugRunner)) -> Vec<String> {
    let mut r = audit_setup();
    for id in hand {
        push_hand(&mut r, 0, id);
    }
    let m = r.place_on_field(0, CARD_ID, Some(0));
    setup(&mut r);
    let a = main_action(m.index);
    assert!(mask_has(&r, a), "the [Main] is declarable");
    r.game.decode_action(a, 0);
    offered_hand_ids(&r, 0)
}

/// Onibimon (EX12-004) is a [TB] Digi-Egg: no play cost, never offered.
#[test]
fn bt26_012_main_pick_excludes_tb_digi_egg() {
    assert_eq!(main_offered(&["EX12-004", "TBMON"], |_| {}), vec!["TBMON".to_string()]);
}

/// Only a Digi-Egg [TB] card in hand: the [Main] is not declarable (DCGO
/// `CanUseCondition` needs a selectable card).
#[test]
fn bt26_012_main_not_declarable_with_only_a_tb_digi_egg() {
    let mut r = audit_setup();
    push_hand(&mut r, 0, "EX12-004");
    let m = r.place_on_field(0, CARD_ID, Some(0));
    assert!(!mask_has(&r, main_action(m.index)));
}

/// A blue [TB] Option with only red/yellow Manekimon on the field can't be
/// used (CR 4.3 4-22-2): only the red one is offered.
#[test]
fn bt26_012_main_pick_excludes_option_whose_colour_requirement_is_unmet() {
    assert_eq!(
        main_offered(&["TBOPT-BLUE", "TBOPT-RED"], |_| {}),
        vec!["TBOPT-RED".to_string()]
    );
}

/// "You can't use Option cards": the Option is not offered, the Digimon is.
#[test]
fn bt26_012_main_pick_excludes_option_when_options_cannot_be_used() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let offered = main_offered(&["TBOPT-RED", "TBMON"], |r| {
        r.game.modifiers.add_player_modifier(
            0,
            PlayerModifierEntry::simple(ModifierType::CannotUseOptionCards, 0, Expiry::Permanent, None, 1),
        );
    });
    assert_eq!(offered, vec!["TBMON".to_string()]);
}

/// Memory 5: the reduced cost must be payable down to -10. TB-17 (15) yes,
/// TB-18 (16) no.
#[test]
fn bt26_012_main_pick_excludes_unaffordable_card() {
    assert_eq!(main_offered(&["TB-17", "TB-18"], |_| {}), vec!["TB-17".to_string()]);
}

/// A full battle area: the Digimon is not offered, the Option still is.
#[test]
fn bt26_012_main_pick_excludes_digimon_when_battle_area_full() {
    let offered = main_offered(&["TBMON", "TBOPT-RED"], |r| {
        r.game.rules.field_slots = r.game.players[0].battle_area.len() as u8;
    });
    assert_eq!(offered, vec!["TBOPT-RED".to_string()]);
}

/// "You can't play Digimon by effects": the Digimon is not offered, the Tamer
/// still is.
#[test]
fn bt26_012_main_pick_excludes_digimon_under_effect_play_prohibition() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let offered = main_offered(&["TBMON", "TBTAMER"], |r| {
        r.game.modifiers.add_player_modifier(
            0,
            PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1),
        );
    });
    assert_eq!(offered, vec!["TBTAMER".to_string()]);
}
