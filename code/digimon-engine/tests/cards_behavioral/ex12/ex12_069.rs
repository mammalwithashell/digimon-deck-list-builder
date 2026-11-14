use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_END, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::enums::{CardColor, Expiry, ModifierType};
use digimon_engine::modifiers::PlayerModifierEntry;
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

use super::support::{
    digimon, field_contains, hand_contains, hand_index, plain_digimon, select_first_non_pass,
    select_hand_card, vb_digimon, DebugRunner,
};

const CARD_ID: &str = "EX12-069";

/// Walk the prompts (accepting each non-hand prompt with its first non-PASS
/// action) to the hand pick and return the card ids it offers, sorted. Empty
/// when no hand pick installs.
fn offered_hand_ids(runner: &mut DebugRunner, player: u8) -> Vec<String> {
    for _ in 0..8 {
        let Some(view) = runner.pending_selection_view() else {
            return Vec::new();
        };
        if view.kind == SelectionKind::Hand {
            let hand: Vec<String> = runner.game.players[player as usize]
                .hand
                .iter()
                .map(|c| c.card_id(&runner.game.card_data).to_string())
                .collect();
            let mut ids: Vec<String> = view
                .valid_action_ids
                .iter()
                .copied()
                .filter(|&a| a != PASS)
                .map(|a| {
                    let slot = if a < PLAY_HAND_END { a - PLAY_HAND_START } else { a - HAND_EFFECT_START };
                    hand[slot as usize].clone()
                })
                .collect();
            ids.sort();
            return ids;
        }
        let Some(action) = view.valid_action_ids.iter().copied().find(|&a| a != PASS) else {
            return Vec::new();
        };
        runner.execute_action(view.selecting_player, action).expect("advance prompt");
    }
    panic!("never reached the hand pick");
}

/// Face-up Virus Busters in player 0's security, `attacker` (a [VB] Digimon) on
/// the field, `hand` in hand and `memory` memory; returns the runner and the
/// attacker's handle, ready to attack.
fn face_up_runner(
    extra: Vec<CardData>,
    attacker: &str,
    hand: &[&str],
    memory: i16,
) -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut builder = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-069 YAML loads")
        .dsl_card("EX12-018")
        .expect("EX12-018 YAML loads")
        .add_card(plain_digimon("SEC", CardColor::Purple, 3, 2000));
    for card in extra {
        builder = builder.add_card(card);
    }
    let mut runner = builder
        .hand(0, hand)
        .security(0, &[CARD_ID])
        .security(1, &["SEC"])
        .memory(memory)
        .start();
    runner.set_turn(1);
    let security_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0].face_up_security.insert(security_index);
    let attacker = runner.place_on_field(0, attacker, Some(0));
    (runner, attacker)
}

// ── Face-up [Security][Your Turn] play with the cost reduced by 3 ────────────
// DCGO EX12_069.cs:58-62 `CanSelectCardCondition`: [VB], has a level equal to
// the attacker's, AND `CardEffectCommons.CanPlayAsNewPermanent(cardSource, true,
// activateClass, fixedCost: GetCostItself - 3)` (GameContextDeterminarion.cs:
// 419-432): not an Option — so never a DUAL, whose kinds include Option (CR 4.3
// 4-6-2: a DUAL is digivolved or used, never played) — an empty battle-area
// frame, no play prohibition, and the reduced cost payable (7-1-3-2). A card the
// play would refuse is not a legal choice (CR 4.3 1-3-2).

/// A Lv.4 [VB] costing 15 (12 after the -3) can't be paid from 0 memory: not
/// offered; the payable same-level card is.
#[test]
fn ex12_069_face_up_security_pick_excludes_unaffordable_card() {
    let pricey = digimon("PRICEY-LV4", CardColor::Yellow, 4, 15, 5000, &["VB"]);
    let (mut runner, attacker) = face_up_runner(
        vec![
            vb_digimon("ATTACKER", CardColor::Yellow, 4, 5000),
            vb_digimon("SAME-LV4", CardColor::Yellow, 4, 5000),
            pricey,
        ],
        "ATTACKER",
        &["SAME-LV4", "PRICEY-LV4"],
        0,
    );
    runner.attack_player(attacker, 1, false);
    assert_eq!(offered_hand_ids(&mut runner, 0), vec!["SAME-LV4".to_string()]);
}

/// Siriusmon (EX12-018) is a Lv.6 [VB] DUAL card: it can't be played (CR 4.3
/// 4-6-2; DCGO `CanPlayAsNewPermanent` refuses `IsOption`), so a Lv.6 [VB]
/// attacker's trigger offers only the playable Lv.6 [VB] Digimon.
#[test]
fn ex12_069_face_up_security_pick_excludes_dual_siriusmon() {
    let (mut runner, attacker) = face_up_runner(
        vec![
            vb_digimon("ATTACKER-LV6", CardColor::Yellow, 6, 11000),
            vb_digimon("SAME-LV6", CardColor::Yellow, 6, 11000),
        ],
        "ATTACKER-LV6",
        &["EX12-018", "SAME-LV6"],
        10,
    );
    runner.attack_player(attacker, 1, false);
    assert_eq!(offered_hand_ids(&mut runner, 0), vec!["SAME-LV6".to_string()]);
}

/// A full battle area: no same-level [VB] Digimon can be played, so none is
/// offered.
#[test]
fn ex12_069_face_up_security_pick_offers_nothing_when_battle_area_full() {
    let (mut runner, attacker) = face_up_runner(
        vec![
            vb_digimon("ATTACKER", CardColor::Yellow, 4, 5000),
            vb_digimon("SAME-LV4", CardColor::Yellow, 4, 5000),
        ],
        "ATTACKER",
        &["SAME-LV4"],
        10,
    );
    runner.game.rules.field_slots = runner.game.players[0].battle_area.len() as u8;
    runner.attack_player(attacker, 1, false);
    assert!(offered_hand_ids(&mut runner, 0).is_empty());
    assert!(hand_contains(&runner, 0, "SAME-LV4"));
}

// ── [Security] free play of a Lv.5 or lower [VB] Digimon ─────────────────────
// DCGO EX12_069.cs:120-127 `CanPlayCondition`: IsDigimon, Lv.5 or lower, [VB],
// AND `CanPlayAsNewPermanent(cardSource, payCost: false, activateClass)`: not
// an Option (never a DUAL), an empty frame, no play prohibition (CR 4.3 1-3-2).

/// Player 1 can't play Digimon by effects: the [VB] Digimon is not offered.
#[test]
fn ex12_069_security_pick_offers_nothing_under_effect_play_prohibition() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-069 YAML loads")
        .add_card(plain_digimon("ATTACKER", CardColor::Purple, 4, 6000))
        .add_card(vb_digimon("SEC-PLAY", CardColor::Yellow, 5, 7000))
        .hand(1, &["SEC-PLAY"])
        .security(1, &[CARD_ID])
        .memory(0)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    runner.game.modifiers.add_player_modifier(
        1,
        PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 0),
    );
    runner.attack_player(attacker, 1, false);
    assert!(offered_hand_ids(&mut runner, 1).is_empty());
    assert!(hand_contains(&runner, 1, "SEC-PLAY"));
}

/// Player 1's battle area is full: the [VB] Digimon is not offered.
#[test]
fn ex12_069_security_pick_offers_nothing_when_battle_area_full() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-069 YAML loads")
        .add_card(plain_digimon("ATTACKER", CardColor::Purple, 4, 6000))
        .add_card(plain_digimon("BLOCKER-LESS", CardColor::Purple, 3, 2000))
        .add_card(vb_digimon("SEC-PLAY", CardColor::Yellow, 5, 7000))
        .hand(1, &["SEC-PLAY"])
        .security(1, &[CARD_ID])
        .memory(0)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    runner.place_on_field(1, "BLOCKER-LESS", Some(0));
    runner.game.rules.field_slots = 1;
    runner.attack_player(attacker, 1, false);
    assert!(offered_hand_ids(&mut runner, 1).is_empty());
    assert!(hand_contains(&runner, 1, "SEC-PLAY"));
}

#[test]
fn ex12_069_face_up_security_attack_trigger_is_not_once_per_turn() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-069 YAML loads")
        .start();
    let card = runner.compiled_card(CARD_ID).expect("compiled EX12-069");
    let effect = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.scope == CompiledScope::Security
                    && triggered.when.contains(&CompiledTiming::OnAllyAttack) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("printed face-up security attack trigger");

    assert!(
        !effect.once_per_turn,
        "printed face-up security trigger has no [Once Per Turn] tag"
    );
}

#[test]
fn ex12_069_use_requirement_requires_vb_field_card() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-069 YAML loads")
        .hand(0, &[CARD_ID])
        .memory(5)
        .start();

    let result = runner.game.play_option_from_hand(0, 0);
    assert_eq!(
        result,
        OptionPlayResult::Invalid,
        "Use Req. should not unconditionally ignore color requirements without a [VB] field card"
    );
}

#[test]
fn ex12_069_main_adds_bottom_security_to_hand_and_places_self_face_up_bottom_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-069 YAML loads")
        .add_card(vb_digimon("VB-ENABLER", CardColor::Yellow, 4, 5000))
        .add_card(plain_digimon("BOTTOM", CardColor::Yellow, 3, 2000))
        .hand(0, &[CARD_ID])
        .security(0, &["BOTTOM"])
        .memory(5)
        .start();

    runner.place_on_field(0, "VB-ENABLER", Some(0));
    let option_slot = hand_index(&runner, 0, CARD_ID);
    let result = runner.game.play_option_from_hand(0, option_slot);
    assert_ne!(result, OptionPlayResult::Invalid);
    runner.auto_resolve().expect("resolve Virus Busters main");

    assert!(hand_contains(&runner, 0, "BOTTOM"));
    assert_eq!(
        runner.game.players[0].security[0].card_id(&runner.game.card_data),
        CARD_ID
    );
    let security_index = runner.game.players[0].security[0].card_index;
    assert!(
        runner.game.players[0]
            .face_up_security
            .contains(&security_index),
        "Virus Busters should be face-up in security"
    );
}

#[test]
fn ex12_069_face_up_security_plays_same_level_vb_from_hand_with_cost_reduced() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-069 YAML loads")
        .add_card(vb_digimon("ATTACKER", CardColor::Yellow, 4, 5000))
        .add_card(vb_digimon("SAME-LV4", CardColor::Yellow, 4, 5000))
        .add_card(vb_digimon("WRONG-LV5", CardColor::Yellow, 5, 7000))
        .add_card(plain_digimon("SEC", CardColor::Purple, 3, 2000))
        .hand(0, &["SAME-LV4", "WRONG-LV5"])
        .security(0, &[CARD_ID])
        .security(1, &["SEC"])
        .memory(5)
        .start();
    runner.set_turn(1);

    let security_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0]
        .face_up_security
        .insert(security_index);
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let memory_before = runner.memory();

    runner.attack_player(attacker, 1, false);
    let offer = runner
        .pending_selection_view()
        .expect("face-up Virus Busters should offer play trigger");
    assert!(offer.is_optional);
    select_first_non_pass(&mut runner);

    let hand = runner
        .pending_selection_view()
        .expect("accepting should expose same-level VB hand cards");
    assert_eq!(hand.kind, SelectionKind::Hand);
    select_hand_card(&mut runner, 0, "SAME-LV4");
    runner
        .auto_resolve()
        .expect("resolve face-up security play");

    assert!(field_contains(&runner, 0, "SAME-LV4"));
    assert!(
        !field_contains(&runner, 0, "WRONG-LV5"),
        "different-level VB card must not be selectable"
    );
    assert_eq!(memory_before - runner.memory(), 2, "cost 5 reduced by 3");
}

#[test]
fn ex12_069_security_plays_level_5_or_lower_vb_from_hand_without_play_cost_cap() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-069 YAML loads")
        .add_card(plain_digimon("ATTACKER", CardColor::Purple, 4, 6000))
        .add_card(vb_digimon("SEC-PLAY", CardColor::Yellow, 5, 7000))
        .hand(1, &["SEC-PLAY"])
        .security(1, &[CARD_ID])
        .memory(0)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let memory_before = runner.memory();

    runner.attack_player(attacker, 1, false);
    let view = runner
        .pending_selection_view()
        .expect("Virus Busters security effect should prompt");
    assert_eq!(
        view.kind,
        SelectionKind::Hand,
        "printed security effect plays a level 5 or lower [VB] Digimon from hand, not hand/trash"
    );
    select_hand_card(&mut runner, 1, "SEC-PLAY");
    runner
        .auto_resolve()
        .expect("resolve Virus Busters security free play");

    assert!(field_contains(&runner, 1, "SEC-PLAY"));
    assert_eq!(
        runner.memory(),
        memory_before,
        "the level-5 cost-7 [VB] Digimon is played without paying its cost"
    );
}
