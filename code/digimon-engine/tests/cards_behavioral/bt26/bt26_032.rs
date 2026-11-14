//! BT26-032 Ceresmon // Famis — DUAL (Lv.6 Digimon // Option).
//!
//! <Alliance> <Succession ([Ceresmon])>
//! [When Digivolving] All of your opponent's suspended Digimon get -5000 DP
//! until their turn ends. Then, by suspending 1 Digimon, if it's your turn,
//! you may play or use 1 [Vegetation] or [TS] trait card from your hand with
//! the cost reduced by 5. (Rule) Trait: Has [Vegetation] Type.
//! Option: <Use Req. ([TS] trait)> [Main] You may suspend 2 of your
//! opponent's Digimon or Tamers. Then, 3 of their Digimon or Tamers can't
//! unsuspend until their turn ends. <Arts Digivolve>.
//!
//! <Succession ([Ceresmon])>: G-ENGINE-SUCCESSION-KEYWORD.
//!
//! DCGO: BT26/Yellow/BT26_032.cs.

use super::support::*;
use digimon_dsl::compiled::CompiledCardKind;
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, Keyword, ModifierType};

const CARD_ID: &str = "BT26-032";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-032")
        .dsl_card("BT25-059")
        .expect("BT25-059 Ceresmon")
        .add_card(filler("FILLER"))
        .add_card(digimon("OPP", "Opp", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("BIG", "Big", CardColor::Red, 9, 9, &[]))
        .add_card(tamer("OPP-T", "Opp Tamer", CardColor::Red, &[]))
        .add_card(digimon("MINE", "Mine", CardColor::Green, 4, 5, &[]))
        .add_card(digimon("TS-8", "TS Eight", CardColor::Green, 5, 8, &["TS"]))
        .add_card(digimon(
            "VEG-8",
            "Veg Eight",
            CardColor::Green,
            5,
            8,
            &["Vegetation"],
        ))
        .add_card(digimon(
            "NOPE-8",
            "Nope",
            CardColor::Green,
            5,
            8,
            &["Beast"],
        ))
        .add_card(tamer("TS-TAMER", "TS Tamer", CardColor::Green, &["TS"]))
        // Play cost 21 / 22 reduced by 5 at memory 6: 16 → -10 (payable),
        // 17 → -11 (not payable).
        .add_card(digimon("TS-21", "TS 21", CardColor::Green, 7, 21, &["TS"]))
        .add_card(digimon("TS-22", "TS 22", CardColor::Green, 7, 22, &["TS"]))
        // Budmon (BT26-002): a [Vegetation] Digi-Egg.
        .dsl_card("BT26-002")
        .expect("BT26-002 Budmon (Digi-Egg)")
        // [Vegetation] Options: green (requirement met by Ceresmon) and purple
        // (unmet on a yellow/green board, CR 4.3 4-22).
        .add_card({
            let mut o = gd_option("VEG-OPT", 3);
            o.traits = vec!["Vegetation".to_string()];
            o.colors = vec![CardColor::Green];
            o
        })
        .add_card({
            let mut o = gd_option("VEG-OPT-PURPLE", 3);
            o.traits = vec!["Vegetation".to_string()];
            o.colors = vec![CardColor::Purple];
            o
        })
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(6)
        .start();
    r.set_first_player(0);
    r
}

fn picks(r: &DebugRunner) -> usize {
    r.pending_selection_view()
        .map(|v| v.valid_action_ids.iter().filter(|&&a| a != PASS).count())
        .unwrap_or(0)
}

#[test]
fn bt26_032_metadata() {
    let r = setup();
    let c = r.compiled_card(CARD_ID).unwrap();
    assert_eq!(c.kind, CompiledCardKind::Dual);
    for t in ["Shaman", "Olympos XII", "Iliad", "TS", "Vegetation"] {
        assert!(c.traits.contains(&t.to_string()), "{t}");
    }
    let alt = format!("{:?}", c.alt_paths);
    assert!(alt.contains("Ceresmon"), "{alt}");
}

#[test]
fn bt26_032_has_alliance() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    assert!(r.game.has_keyword(h, Keyword::Alliance));
}

#[test]
fn bt26_032_wd_debuffs_suspended_opponents_continuously() {
    let mut r = setup();
    let c = r.place_on_field(0, CARD_ID, Some(0));
    // 9000 DP so -5000 does not delete them at the rule check.
    let s = r.place_on_field(1, "BIG", Some(0));
    let u = r.place_on_field(1, "BIG", Some(0));
    r.game.players[1].battle_area[s.index as usize].is_suspended = true;
    let base = r.effective_dp(u).unwrap();
    fire(&mut r, EffectTiming::WhenDigivolving, c);
    // Decline the suspend leg.
    if r.pending_selection_view().is_some() {
        pass(&mut r, 0);
    }
    let _ = r.auto_resolve();
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(s).unwrap(), base - 5000);
    assert_eq!(r.effective_dp(u).unwrap(), base);
    // Continuous: an opponent Digimon suspended later in the window.
    r.game.players[1].battle_area[u.index as usize].is_suspended = true;
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(u).unwrap(), base - 5000);
}

#[test]
fn bt26_032_wd_suspend_then_play_ts_from_hand_reduced() {
    let mut r = setup();
    let c = r.place_on_field(0, CARD_ID, Some(0));
    let mine = r.place_on_field(0, "MINE", Some(0));
    push_hand(&mut r, 0, "TS-8");
    push_hand(&mut r, 0, "VEG-8");
    push_hand(&mut r, 0, "NOPE-8");
    fire(&mut r, EffectTiming::WhenDigivolving, c);
    let v = r.pending_selection_view().expect("suspend pick");
    assert!(v.is_optional, "by suspending — optional");
    assert_eq!(picks(&r), 2, "Ceresmon + MINE (any unsuspended Digimon)");
    // Suspend MINE (second candidate).
    let a = *v
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .last()
        .unwrap();
    r.execute_action(0, a).unwrap();
    assert!(r.game.players[0].battle_area[mine.index as usize].is_suspended);
    let v = r.pending_selection_view().expect("hand pick");
    assert!(v.is_optional);
    assert_eq!(picks(&r), 2, "TS-8 and VEG-8 only");
    pick_hand(&mut r, 0, "TS-8");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"TS-8".to_string()));
    assert_eq!(r.memory(), 3, "cost 8 reduced by 5");
}

#[test]
fn bt26_032_wd_declined_suspend_skips_play() {
    let mut r = setup();
    let c = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "TS-8");
    fire(&mut r, EffectTiming::WhenDigivolving, c);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
    assert_eq!(hand_ids(&r, 0), vec!["TS-8".to_string()]);
}

// ─── Hand-pick candidate legality (select-hand playability audit) ────────────
// DCGO BT26_032.cs:62-64 offers only `(EqualsTraits("Vegetation") ||
// HasTSTraits) && CanPlayOrUse(card, activateClass, fixedCost: max(0,
// GetCostItself - 5))` (PlayEffects.cs:286-293): an Option (a DUAL counts) with
// `!CanNotPlayThisOption` (no "can't use Option" lock, colour requirement or
// <Use Req.> met — CR 4.3 4-22, Q1500 / Q2785) and `MaxMemoryCost >= cost`, or
// a card with a play cost (never a Digi-Egg, CR 4.3 7-1-1) that
// `CanPlayAsNewPermanent` (empty frame, no play lock, payable). A card the
// play / use would refuse is never offered (CR 4.3 1-3-2).

/// Fire [When Digivolving] on Ceresmon, suspend Ceresmon itself, and return
/// the hand-pick candidates' card ids (empty when no hand pick installs).
fn offered_after_suspend(r: &mut DebugRunner, hand: &[&str]) -> Vec<String> {
    use digimon_engine::action::space::PLAY_HAND_START;
    let c = r.place_on_field(0, CARD_ID, Some(0));
    for id in hand {
        push_hand(r, 0, id);
    }
    fire(r, EffectTiming::WhenDigivolving, c);
    pick_first(r, 0); // suspend Ceresmon
    assert!(r.game.players[0].battle_area[c.index as usize].is_suspended);
    let Some(v) = r.pending_selection_view() else {
        return Vec::new();
    };
    assert!(
        r.game
            .pending_selection
            .as_ref()
            .is_none_or(|p| p.card_choice.is_none()),
        "hand pick uses PLAY_HAND_START action ids"
    );
    let ids = hand_ids(r, 0);
    v.valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS && a >= PLAY_HAND_START && ((a - PLAY_HAND_START) as usize) < ids.len())
        .map(|a| ids[(a - PLAY_HAND_START) as usize].clone())
        .collect()
}

/// Budmon (BT26-002) is a [Vegetation] Digi-Egg: no play cost, never played.
#[test]
fn bt26_032_hand_pick_excludes_vegetation_digi_egg() {
    let mut r = setup();
    let offered = offered_after_suspend(&mut r, &["BT26-002", "VEG-8"]);
    assert_eq!(offered, vec!["VEG-8".to_string()]);
}

/// A purple [Vegetation] Option's colour requirement is unmet: not offered;
/// the green one is.
#[test]
fn bt26_032_hand_pick_excludes_option_whose_colour_is_unmet() {
    let mut r = setup();
    let offered = offered_after_suspend(&mut r, &["VEG-OPT-PURPLE", "VEG-OPT"]);
    assert_eq!(offered, vec!["VEG-OPT".to_string()]);
}

/// "You can't use Option cards": the Option is not offered, the Digimon is.
#[test]
fn bt26_032_hand_pick_excludes_option_when_options_cannot_be_used() {
    use digimon_engine::enums::Expiry;
    use digimon_engine::modifiers::PlayerModifierEntry;
    let mut r = setup();
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotUseOptionCards, 0, Expiry::Permanent, None, 1),
    );
    let offered = offered_after_suspend(&mut r, &["VEG-OPT", "VEG-8"]);
    assert_eq!(offered, vec!["VEG-8".to_string()]);
}

/// "You can't play Digimon by effects": the Digimon is not offered, the Option
/// still is.
#[test]
fn bt26_032_hand_pick_excludes_digimon_when_effect_play_is_prohibited() {
    use digimon_engine::enums::Expiry;
    use digimon_engine::modifiers::PlayerModifierEntry;
    let mut r = setup();
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1),
    );
    let offered = offered_after_suspend(&mut r, &["VEG-OPT", "VEG-8"]);
    assert_eq!(offered, vec!["VEG-OPT".to_string()]);
}

/// A full battle area: the Digimon and the Tamer are not offered, the Option
/// still is.
#[test]
fn bt26_032_hand_pick_excludes_digimon_and_tamer_when_battle_area_full() {
    let mut r = setup();
    r.game.rules.field_slots = 1; // Ceresmon alone fills it
    let offered = offered_after_suspend(&mut r, &["VEG-OPT", "VEG-8", "TS-TAMER"]);
    assert_eq!(offered, vec!["VEG-OPT".to_string()]);
}

/// The play pays the cost reduced by 5: a card whose reduced cost would take
/// memory past -10 is not offered.
#[test]
fn bt26_032_hand_pick_excludes_unaffordable_digimon() {
    let mut r = setup();
    let offered = offered_after_suspend(&mut r, &["TS-21", "TS-22"]);
    assert_eq!(offered, vec!["TS-21".to_string()]);
}

#[test]
fn bt26_032_wd_opponents_turn_suspends_but_no_play() {
    let mut r = setup();
    let c = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "TS-8");
    r.game.turn_player_idx = 1;
    fire(&mut r, EffectTiming::WhenDigivolving, c);
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(r.game.players[0].battle_area[c.index as usize].is_suspended);
    assert!(
        r.pending_selection_view().is_none(),
        "not your turn → no play"
    );
    assert_eq!(hand_ids(&r, 0), vec!["TS-8".to_string()]);
}

#[test]
fn bt26_032_option_main_suspend_optional_lock_three() {
    let mut r = setup();
    r.place_on_field(0, "TS-TAMER", Some(0));
    let a = r.place_on_field(1, "OPP", Some(0));
    let b = r.place_on_field(1, "OPP", Some(0));
    let t = r.place_on_field(1, "OPP-T", Some(0));
    push_hand(&mut r, 0, CARD_ID);
    let idx = r.game.players[0].hand.len() - 1;
    let _ = r.game.play_option_from_hand(0, idx);
    let v = r.pending_selection_view().expect("suspend pick");
    assert!(v.is_optional, "may suspend");
    pass(&mut r, 0);
    // Lock leg: mandatory, all 3 of theirs.
    drain_first(&mut r);
    for h in [a, b, t] {
        assert!(!r.game.players[1].battle_area[h.index as usize].is_suspended);
        assert!(r.modifiers().has(h, ModifierType::CannotUnsuspend));
    }
}

#[test]
fn bt26_032_option_main_suspends_exactly_two() {
    let mut r = setup();
    r.place_on_field(0, "TS-TAMER", Some(0));
    let hs: Vec<_> = (0..3)
        .map(|_| r.place_on_field(1, "OPP", Some(0)))
        .collect();
    push_hand(&mut r, 0, CARD_ID);
    let idx = r.game.players[0].hand.len() - 1;
    let _ = r.game.play_option_from_hand(0, idx);
    pick_first(&mut r, 0);
    let v = r.pending_selection_view().expect("2nd pick");
    assert!(!v.is_optional, "0 or exactly 2");
    pick_first(&mut r, 0);
    drain_first(&mut r);
    let n = hs
        .iter()
        .filter(|h| r.game.players[1].battle_area[h.index as usize].is_suspended)
        .count();
    assert_eq!(n, 2);
}

// ─── <Succession ([Ceresmon])> ──────────────────────────────────────────────

#[test]
fn bt26_032_succession_adopts_ceresmon_all_turns_suspend_observer() {
    let mut r = setup();
    let c = r.place_stack(0, &["BT25-059", CARD_ID]);
    let opp = r.place_on_field(1, "OPP", Some(0));
    let before = r.effective_dp(opp).unwrap();
    // -3000 per suspended Digimon: suspend the carrier, as its own suspend would.
    r.game.players[0].battle_area[c.index as usize].is_suspended = true;
    fire(&mut r, EffectTiming::OnSuspend, c);
    assert!(
        r.pending_selection_view().is_some(),
        "BT25-059's [All Turns] OnSuspend debuff is the carrier's own now"
    );
    while r.pending_selection_view().is_some() {
        pick_first(&mut r, 0);
    }
    let _ = r.auto_resolve();
    assert!(r.effective_dp(opp).unwrap() < before);
}

#[test]
fn bt26_032_succession_copy_has_its_own_once_per_turn() {
    // DCGO builds a fresh ActivateClass per copy: Ceresmon (BT25-059) using
    // its [Once Per Turn] while it was the top card does not lock out the copy
    // the same permanent gains once BT26-032 is put on top.
    let mut r = setup();
    let h = r.place_on_field(0, "BT25-059", Some(0));
    r.place_on_field(1, "OPP", Some(0));
    fire(&mut r, EffectTiming::OnSuspend, h);
    assert!(r.pending_selection_view().is_some(), "BT25-059's own OPT");
    while r.pending_selection_view().is_some() {
        pick_first(&mut r, 0);
    }
    let _ = r.auto_resolve();
    fire(&mut r, EffectTiming::OnSuspend, h);
    assert!(r.pending_selection_view().is_none(), "own OPT spent");

    put_on_top(&mut r, h, CARD_ID);
    fire(&mut r, EffectTiming::OnSuspend, h);
    assert!(
        r.pending_selection_view().is_some(),
        "the copy has its own counter"
    );
    while r.pending_selection_view().is_some() {
        pick_first(&mut r, 0);
    }
    let _ = r.auto_resolve();
    fire(&mut r, EffectTiming::OnSuspend, h);
    assert!(r.pending_selection_view().is_none(), "copy's OPT spent");
}

#[test]
fn bt26_032_without_a_ceresmon_source_adopts_nothing() {
    let mut r = setup();
    let c = r.place_stack(0, &["MINE", CARD_ID]);
    r.place_on_field(1, "OPP", Some(0));
    assert!(r.game.succession_source_indices(c).is_empty());
    fire(&mut r, EffectTiming::OnSuspend, c);
    assert!(r.pending_selection_view().is_none());
}
