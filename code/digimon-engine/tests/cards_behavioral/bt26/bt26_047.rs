//! BT26-047 TyrantKabuterimon — Lv.6 Green, Insectoid/Titan/TS.
//!
//! [On Play] [When Digivolving] This Digimon may battle 1 of your opponent's
//! Digimon.
//! [Start of Your Main Phase] [On Play] [When Digivolving] By suspending 1
//! Digimon, until your opponent's turn ends, none of your suspended
//! [Insectoid] or [Titan] trait Digimon are affected by your opponent's Option
//! effects, and they get +3000 DP.
//! Assembly -6: 4 [Larva]/[Insectoid]/[Titan] trait Digimon cards w/different
//! levels.
//!
//! DCGO: BT26/Green/BT26_047.cs.

use super::support::*;
use digimon_engine::action::space::{PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectSourceKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT26-047";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-047 in embedded DSL pack")
        .add_card(filler("FILLER"))
        .add_card(digimon(
            "LARVA2",
            "Larva Two",
            CardColor::Green,
            2,
            0,
            &["Larva"],
        ))
        .add_card(digimon(
            "BUG3",
            "Bug Three",
            CardColor::Green,
            3,
            3,
            &["Insectoid"],
        ))
        .add_card(digimon(
            "BUG3B",
            "Bug Three B",
            CardColor::Green,
            3,
            3,
            &["Insectoid"],
        ))
        .add_card(digimon(
            "TITAN4",
            "Titan Four",
            CardColor::Purple,
            4,
            5,
            &["Titan"],
        ))
        .add_card(digimon(
            "BUG5",
            "Bug Five",
            CardColor::Green,
            5,
            7,
            &["Insectoid"],
        ))
        .add_card(digimon(
            "BEAST4",
            "Beast Four",
            CardColor::Green,
            4,
            5,
            &["Beast"],
        ))
        .add_card(digimon("OPP", "Opp", CardColor::Red, 3, 3, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(5)
        .start();
    r.set_first_player(0);
    r.game.turn_player_idx = 0;
    r.game.turn_count = 1;
    r
}

fn suspend(r: &mut DebugRunner, h: PermanentHandle) {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended = true;
}

#[test]
fn bt26_047_on_play_may_battle_and_delete() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    // No unsuspended Digimon anywhere → only the battle clause fires.
    suspend(&mut r, h);
    suspend(&mut r, opp);
    fire(&mut r, EffectTiming::OnPlay, h);
    let v = r.pending_selection_view().expect("optional battle pick");
    assert!(v.is_optional);
    pick_side_field(&mut r, 0, opp);
    let _ = r.auto_resolve();
    assert!(r.game.players[1].battle_area.is_empty(), "13000 beats 3000");
    assert_eq!(field_ids(&r, 0), vec![CARD_ID.to_string()]);
}

#[test]
fn bt26_047_battle_can_be_declined() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    suspend(&mut r, h);
    suspend(&mut r, opp);
    fire(&mut r, EffectTiming::WhenDigivolving, h);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.game.players[1].battle_area.len(), 1);
}

#[test]
fn bt26_047_somp_suspend_buffs_suspended_insectoids_continuously() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    let bug = r.place_on_field(0, "BUG5", Some(0));
    let beast = r.place_on_field(0, "BEAST4", Some(0));
    suspend(&mut r, beast);
    fire(&mut r, EffectTiming::StartOfYourMainPhase, h);
    let v = r.pending_selection_view().expect("optional suspend cost");
    assert!(v.is_optional);
    pick_any_field(&mut r, 0, h);
    let _ = r.auto_resolve();
    r.game.tick_declarative_effects();
    assert!(r.game.players[0].battle_area[h.index as usize].is_suspended);
    assert_eq!(r.effective_dp(h), Some(16000));
    assert_eq!(r.effective_dp(bug), Some(5000), "unsuspended: no buff");
    assert_eq!(r.effective_dp(beast), Some(4000), "not Insectoid/Titan");
    assert!(r
        .game
        .permanent_is_unaffected_by_effect(h, 1, EffectSourceKind::Option));
    assert!(!r
        .game
        .permanent_is_unaffected_by_effect(h, 1, EffectSourceKind::Digimon));
    assert!(!r
        .game
        .permanent_is_unaffected_by_effect(bug, 1, EffectSourceKind::Option));
    // Re-evaluated live: the [Insectoid] that suspends later is covered.
    suspend(&mut r, bug);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(bug), Some(8000));
    assert!(r
        .game
        .permanent_is_unaffected_by_effect(bug, 1, EffectSourceKind::Option));
    // Lasts through the opponent's turn.
    r.end_turn();
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(h), Some(16000));
}

#[test]
fn bt26_047_declined_suspend_applies_nothing() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    let bug = r.place_on_field(0, "BUG5", Some(0));
    suspend(&mut r, bug);
    fire(&mut r, EffectTiming::StartOfYourMainPhase, h);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(bug), Some(5000));
    assert!(!r
        .game
        .permanent_is_unaffected_by_effect(bug, 1, EffectSourceKind::Option));
}

#[test]
fn bt26_047_suspend_may_target_an_opponent_digimon() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    suspend(&mut r, h);
    let opp = r.place_on_field(1, "OPP", Some(0));
    fire(&mut r, EffectTiming::StartOfYourMainPhase, h);
    pick_any_field(&mut r, 0, opp);
    let _ = r.auto_resolve();
    r.game.tick_declarative_effects();
    assert!(r.game.players[1].battle_area[opp.index as usize].is_suspended);
    assert_eq!(r.effective_dp(h), Some(16000));
    assert_eq!(r.effective_dp(opp), Some(3000), "only YOUR Digimon");
}

#[test]
fn bt26_047_assembly_four_different_levels_reduces_cost_by_six() {
    let mut r = setup();
    r.skip_mulligan();
    push_hand(&mut r, 0, CARD_ID);
    for id in ["LARVA2", "BUG3", "BUG3B", "TITAN4", "BUG5", "BEAST4"] {
        push_trash(&mut r, 0, id);
    }
    let mem = r.memory();
    let idx = r.game.players[0].hand.len() - 1;
    r.game.decode_action(PLAY_HAND_START + idx as u16, 0);
    assert!(r.game.pending_selection.is_some(), "assembly gate");
    let v = r.pending_selection_view().expect("assembly pick");
    assert!(
        !v.valid_action_ids.contains(&(TRASH_EFFECT_START + 5)),
        "[Beast] is not an Assembly material"
    );
    r.game.decode_action(TRASH_EFFECT_START, 0); // LARVA2
    r.game.decode_action(TRASH_EFFECT_START + 1, 0); // BUG3
    let v = r.pending_selection_view().expect("still selecting");
    assert!(
        !v.valid_action_ids.contains(&(TRASH_EFFECT_START + 2)),
        "same level as an already-picked card"
    );
    r.game.decode_action(TRASH_EFFECT_START + 3, 0); // TITAN4
    r.game.decode_action(TRASH_EFFECT_START + 4, 0); // BUG5
    assert_eq!(r.memory(), mem - 7, "13 - 6");
    let field = field_ids(&r, 0);
    assert_eq!(field, vec![CARD_ID.to_string()]);
    assert_eq!(
        sources(
            &r,
            PermanentHandle {
                player: 0,
                index: 0
            }
        ),
        4
    );
    assert_eq!(
        trash_ids(&r, 0),
        vec!["BUG3B".to_string(), "BEAST4".to_string()]
    );
}
