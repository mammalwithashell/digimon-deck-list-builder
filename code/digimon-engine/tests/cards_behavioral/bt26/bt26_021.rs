//! BT26-021 Gekomon — Lv.4 Blue/Purple Digimon (Amphibian / Titan / TS).
//!
//! [On Play] [When Digivolving] 1 of your [TS] trait Digimon's attack target
//! can't change for the turn.
//! [Main] [Once Per Turn] You may play 1 [TS] trait Tamer card from your trash
//! with the cost reduced by 2.
//! Inherited: [All Turns] [Once Per Turn] When a Digimon attacks, by trashing
//! 1 card in your hand, trash the bottom 2 digivolution cards of 1 of your
//! opponent's Digimon.
//!
//! DCGO: BT26/Blue/BT26_021.cs.

use super::support::*;
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START,
};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, ModifierType};

const CARD_ID: &str = "BT26-021";

fn setup() -> DebugRunner {
    let mut ts_tamer = tamer("TS-T", "TS Tamer", CardColor::Purple, &["TS"]);
    ts_tamer.play_cost = 5;
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-021")
        .add_card(filler("FILLER"))
        .add_card(ts_tamer)
        .add_card(tamer("OTHER-T", "Other Tamer", CardColor::Purple, &[]))
        .add_card(digimon("TS-D", "TS Digi", CardColor::Blue, 4, 5, &["TS"]))
        .add_card(digimon("NON-TS", "Plain", CardColor::Blue, 4, 5, &[]))
        .add_card(digimon("OPP", "Opp", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("CARRIER", "Carrier", CardColor::Blue, 5, 7, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(5)
        .start();
    r.set_first_player(0);
    r
}

fn main_bit(field_index: usize) -> u16 {
    FIELD_EFFECT_START + field_index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN
}

fn main_legal(r: &DebugRunner) -> bool {
    build_action_mask(&r.game, 0)[main_bit(0) as usize] > 0.0
}

#[test]
fn bt26_021_traits_include_rule_titan() {
    let r = setup();
    let c = r.compiled_card(CARD_ID).unwrap();
    for t in ["Amphibian", "Titan", "TS"] {
        assert!(c.traits.contains(&t.to_string()), "{t}");
    }
}

#[test]
fn bt26_021_on_play_locks_own_ts_attack_target() {
    let mut r = setup();
    let g = r.place_on_field(0, CARD_ID, Some(0));
    let plain = r.place_on_field(0, "NON-TS", Some(0));
    let ts = r.place_on_field(0, "TS-D", Some(0));
    fire(&mut r, EffectTiming::OnPlay, g);
    // Mandatory pick over own [TS] Digimon only: Gekomon itself + TS-D.
    let v = r.pending_selection_view().expect("own [TS] pick");
    assert!(!v.is_optional);
    let picks = v
        .valid_action_ids
        .iter()
        .filter(|&&a| a != digimon_engine::action::space::PASS)
        .count();
    assert_eq!(picks, 2, "only [TS] Digimon");
    // Pick the TS-D Digimon (last candidate).
    let a = *v
        .valid_action_ids
        .iter()
        .filter(|&&a| a != digimon_engine::action::space::PASS)
        .last()
        .unwrap();
    r.execute_action(0, a).unwrap();
    let _ = r.auto_resolve();
    let locked: Vec<_> = [g, plain, ts]
        .into_iter()
        .filter(|h| {
            r.modifiers()
                .has(*h, ModifierType::CannotSwitchAttackTarget)
        })
        .collect();
    assert_eq!(locked.len(), 1);
    assert_ne!(locked[0], plain);
}

#[test]
fn bt26_021_main_plays_ts_tamer_from_trash_reduced_once_per_turn() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "TS-T");
    push_trash(&mut r, 0, "TS-T");
    push_trash(&mut r, 0, "OTHER-T");
    r.game.enter_main_phase();
    let _ = r.auto_resolve();
    let mem = r.memory();
    assert!(main_legal(&r), "[Main] offered");
    r.game.decode_action(main_bit(0), 0);
    let v = r.pending_selection_view().expect("trash pick");
    assert!(v.is_optional, "may");
    assert_eq!(
        v.valid_action_ids
            .iter()
            .filter(|&&a| a != digimon_engine::action::space::PASS)
            .count(),
        2,
        "only [TS] Tamers"
    );
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"TS-T".to_string()));
    assert_eq!(r.memory(), mem - 3, "cost 5 reduced by 2");
    // Once per turn: the [Main] is no longer offered.
    assert!(!main_legal(&r), "[Main] already used this turn");
    assert_eq!(
        trash_ids(&r, 0).iter().filter(|c| *c == "TS-T").count(),
        1,
        "second TS Tamer stays in trash"
    );
}

#[test]
fn bt26_021_main_needs_ts_tamer_in_trash() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "OTHER-T");
    r.game.enter_main_phase();
    let _ = r.auto_resolve();
    assert!(!main_legal(&r), "no [TS] Tamer in trash → not activatable");
    assert!(field_ids(&r, 0).len() == 1);
}

fn opp_with_sources(r: &mut DebugRunner) -> digimon_engine::permanent::PermanentHandle {
    r.place_stack(1, &["FILLER", "FILLER", "FILLER", "OPP"])
}

#[test]
fn bt26_021_inherited_any_attack_trash_hand_strips_bottom_two() {
    let mut r = setup();
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let opp = opp_with_sources(&mut r);
    push_hand(&mut r, 0, "FILLER");
    // The OPPONENT's Digimon attacks → observed via on_opponent_attack.
    r.game.enqueue_triggered(
        EffectTiming::OnOpponentAttack,
        digimon_engine::selection::TriggerSource::PlayerBattleAreaAttack {
            player: 0,
            attacker: opp,
            card: r.game.players[1].battle_area[opp.index as usize]
                .top_card()
                .handle(),
        },
    );
    r.game.drain_effect_queue();
    assert!(r.pending_is_optional(), "by trashing — optional cost");
    r.accept_optional_trigger().unwrap();
    drain_first(&mut r);
    assert_eq!(r.hand_size(0), 0, "hand card trashed as the cost");
    assert_eq!(sources(&r, opp), 1, "bottom 2 of 3 sources trashed");
    let _ = carrier;
}

#[test]
fn bt26_021_inherited_once_per_turn_shared_across_attackers() {
    let mut r = setup();
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let opp = opp_with_sources(&mut r);
    push_hand(&mut r, 0, "FILLER");
    push_hand(&mut r, 0, "FILLER");
    // The carrier itself attacks.
    fire(&mut r, EffectTiming::OnAttack, carrier);
    r.accept_optional_trigger().unwrap();
    drain_first(&mut r);
    assert_eq!(sources(&r, opp), 1);
    // A second attack (by anything) the same turn: no trigger.
    fire(&mut r, EffectTiming::OnAttack, carrier);
    assert!(r.pending_selection_view().is_none(), "once per turn");
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn bt26_021_inherited_decline_keeps_hand_and_use() {
    let mut r = setup();
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let opp = opp_with_sources(&mut r);
    push_hand(&mut r, 0, "FILLER");
    fire(&mut r, EffectTiming::OnAttack, carrier);
    r.decline_optional_trigger().unwrap();
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(sources(&r, opp), 3);
    fire(&mut r, EffectTiming::OnAttack, carrier);
    assert!(
        r.pending_selection_view().is_some(),
        "not consumed by a decline"
    );
}

#[test]
fn bt26_021_inherited_needs_a_hand_card() {
    let mut r = setup();
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let _opp = opp_with_sources(&mut r);
    fire(&mut r, EffectTiming::OnAttack, carrier);
    assert!(r.pending_selection_view().is_none());
}

#[test]
fn bt26_021_main_decline_refunds_once_per_turn() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "TS-T");
    r.game.enter_main_phase();
    let _ = r.auto_resolve();
    r.game.decode_action(main_bit(0), 0);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0).len(), 1, "nothing played");
    // DCGO RemoveUse: declining the pick does not spend the [Once Per Turn].
    assert!(main_legal(&r), "[Main] still available after declining");
}
