//! BT26-049 Rosemon — Lv.6 Green, Fairy/DATA SQUAD.
//!
//! [When Digivolving] [When Attacking] [Once Per Turn] Suspend 2 of your
//! opponent's Digimon or Tamers.
//! [All Turns] [Once Per Turn] When any of your opponent's Digimon or Tamers
//! suspend, or effects trash cards from under your Tamers, you may play or use
//! 1 play or use cost 3 or lower [DATA SQUAD] trait card from your hand without
//! paying the cost. For each suspended Digimon or Tamer, add 1 to the cost
//! maximum.
//!
//! DCGO: BT26/Green/BT26_049.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT26-049";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-049")
        .add_card(filler("FILLER"))
        .add_card(digimon("OPP", "Opp", CardColor::Red, 4, 5, &[]))
        .add_card(tamer("OPP-T", "Opp Tamer", CardColor::Red, &[]))
        .add_card(digimon(
            "DS3",
            "DS three",
            CardColor::Green,
            3,
            3,
            &["DATA SQUAD"],
        ))
        .add_card(digimon(
            "DS5",
            "DS five",
            CardColor::Green,
            4,
            5,
            &["DATA SQUAD"],
        ))
        .add_card(digimon(
            "DS6",
            "DS six",
            CardColor::Green,
            5,
            6,
            &["DATA SQUAD"],
        ))
        .add_card(digimon("PLAIN3", "Plain", CardColor::Green, 3, 3, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

/// Answer every prompt with its first non-PASS option until the queue settles
/// or a prompt of `stop_prompt` appears.
fn drive_until(r: &mut DebugRunner, stop_prompt: &str) -> bool {
    for _ in 0..12 {
        let Some(sel) = r.game.pending_selection.as_ref() else {
            return false;
        };
        if sel.prompt.contains(stop_prompt) {
            return true;
        }
        let p = sel.selecting_player;
        let a = sel
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != digimon_engine::action::space::PASS)
            .unwrap();
        r.execute_action(p, a).unwrap();
    }
    false
}

#[test]
fn bt26_049_wd_suspends_exactly_two() {
    let mut r = setup();
    let rose = r.place_on_field(0, CARD_ID, Some(0));
    let a = r.place_on_field(1, "OPP", Some(0));
    let b = r.place_on_field(1, "OPP", Some(0));
    let c = r.place_on_field(1, "OPP-T", Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, rose);
    let v = r.pending_selection_view().expect("suspend pick");
    assert!(!v.is_optional, "'Suspend 2' is mandatory");
    drive_until(&mut r, "play or use");
    let n = [a, b, c].iter().filter(|&&h| suspended(&r, h)).count();
    assert_eq!(n, 2);
}

#[test]
fn bt26_049_wd_clamps_to_available() {
    let mut r = setup();
    let rose = r.place_on_field(0, CARD_ID, Some(0));
    let a = r.place_on_field(1, "OPP", Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, rose);
    drive_until(&mut r, "play or use");
    assert!(suspended(&r, a));
}

#[test]
fn bt26_049_wd_and_wa_share_one_use() {
    let mut r = setup();
    let rose = r.place_on_field(0, CARD_ID, Some(0));
    let a = r.place_on_field(1, "OPP", Some(0));
    let b = r.place_on_field(1, "OPP", Some(0));
    let c = r.place_on_field(1, "OPP", Some(0));
    let d = r.place_on_field(1, "OPP", Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, rose);
    drive_until(&mut r, "play or use");
    // Decline the reactive play so it doesn't matter here.
    let _ = r.execute_action(0, digimon_engine::action::space::PASS);
    let _ = r.auto_resolve();
    fire(&mut r, EffectTiming::WhenAttacking, rose);
    let _ = r.auto_resolve();
    let n = [a, b, c, d].iter().filter(|&&h| suspended(&r, h)).count();
    assert_eq!(n, 2, "[Once Per Turn] shared by WD and WA");
}

/// Rosemon's own suspension of 2 opponent permanents triggers its [All Turns]
/// reactive: cap = 3 + 2 suspended = 5, so a cost-5 [DATA SQUAD] card plays
/// free and a cost-6 one is not offered.
#[test]
fn bt26_049_reactive_cap_counts_suspended() {
    let mut r = setup();
    let rose = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP", Some(0));
    r.place_on_field(1, "OPP", Some(0));
    push_hand(&mut r, 0, "DS5");
    push_hand(&mut r, 0, "DS6");
    push_hand(&mut r, 0, "PLAIN3");
    let mem = r.memory();
    fire(&mut r, EffectTiming::WhenDigivolving, rose);
    assert!(drive_until(&mut r, "play or use"), "reactive offered");
    let v = r.pending_selection_view().unwrap();
    assert!(v.is_optional);
    let offered: Vec<u16> = v
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != digimon_engine::action::space::PASS)
        .collect();
    assert_eq!(offered.len(), 1, "only DS5 is within 3 + 2");
    pick_hand(&mut r, 0, "DS5");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"DS5".to_string()));
    assert_eq!(r.memory(), mem, "free");
}

#[test]
fn bt26_049_reactive_decline_refunds() {
    let mut r = setup();
    let rose = r.place_on_field(0, CARD_ID, Some(0));
    // ONE opponent permanent: the engine fires on_suspend per permanent (DCGO
    // batches one OnTappedAnyone per suspend action —
    // G-ENGINE-ON-SUSPEND-BATCH), so keep this test to a single suspend.
    r.place_on_field(1, "OPP", Some(0));
    push_hand(&mut r, 0, "DS3");
    fire(&mut r, EffectTiming::WhenDigivolving, rose);
    assert!(drive_until(&mut r, "play or use"));
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), 1);
    // A later effect trash from under a Tamer re-offers it (RemoveUse).
    let t = tamer_with_face_down(&mut r, 0, "OPP-T", 1);
    let _ = t;
    // Trash via the engine surface a card effect would use.
    {
        let rc = rose_card(&r, rose);
        let mut ctx =
            digimon_engine::effect_context::EffectContext::new(&mut r.game, rc, Some(rose), 0);
        assert!(ctx.trash_bottom_face_down_source(t));
    }
    r.game.drain_effect_queue();
    assert!(
        r.pending_selection_view().is_some(),
        "re-offered after a decline"
    );
}

fn rose_card(r: &DebugRunner, h: PermanentHandle) -> digimon_engine::card_source::CardHandle {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .handle()
}

/// G-ENGINE-ON-SUSPEND-BATCH: suspending 2 permanents in one action is ONE
/// "when … suspend" event (rule 15-5-2; DCGO fires one OnTappedAnyone with the
/// whole list). Declining the reactive refunds its [Once Per Turn], but the
/// same batch must not re-offer it.
#[test]
fn bt26_049_two_suspends_in_one_action_trigger_once() {
    let mut r = setup();
    let rose = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP", Some(0));
    r.place_on_field(1, "OPP", Some(0));
    push_hand(&mut r, 0, "DS3");
    fire(&mut r, EffectTiming::WhenDigivolving, rose);
    assert!(drive_until(&mut r, "play or use"), "reactive offered once");
    pass(&mut r, 0);
    assert!(
        r.game.pending_selection.is_none(),
        "no second offer from the same 2-permanent suspend"
    );
    assert_eq!(r.hand_size(0), 1);
}

/// A [DATA SQUAD] Digi-Egg has no play cost (its cost 0 is a missing value), so
/// it is never a play-or-use candidate: alone in hand it does not trigger the
/// [All Turns] clause, and beside a legal card it is not offered.
#[test]
fn bt26_049_digi_egg_not_offered_rose_rave_audit() {
    // Rose Rave audit 2026-11-11: CR 7-1-1 / DCGO HasPlayCost -- a Digi-Egg can't be played.
    fn runner() -> DebugRunner {
        let mut r = DebugRunner::builder()
            .dsl_card(CARD_ID)
            .expect("BT26-049")
            .add_card(filler("FILLER"))
            .add_card(digimon("OPP", "Opp", CardColor::Red, 4, 5, &[]))
            .add_card({
                let mut c = digimon("DSEGG", "DS Egg", CardColor::Green, 2, 0, &["DATA SQUAD"]);
                c.card_kind = digimon_engine::enums::CardKind::DigiEgg;
                c.dp = None;
                c
            })
            .add_card(digimon(
                "DS3",
                "DS three",
                CardColor::Green,
                3,
                3,
                &["DATA SQUAD"],
            ))
            .deck(0, &["FILLER"; 6])
            .deck(1, &["FILLER"; 6])
            .memory(3)
            .start();
        r.set_first_player(0);
        r
    }

    // Egg alone: the suspend does not trigger the reactive.
    let mut r = runner();
    let rose = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP", Some(0));
    push_hand(&mut r, 0, "DSEGG");
    fire(&mut r, EffectTiming::WhenDigivolving, rose);
    assert!(
        !drive_until(&mut r, "play or use"),
        "a Digi-Egg alone must not trigger the play-or-use clause"
    );
    assert!(r.game.pending_selection.is_none());
    assert_eq!(hand_ids(&r, 0), vec!["DSEGG".to_string()]);

    // Egg beside a legal [DATA SQUAD] card: only the legal card is offered.
    let mut r = runner();
    let rose = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP", Some(0));
    push_hand(&mut r, 0, "DSEGG");
    push_hand(&mut r, 0, "DS3");
    fire(&mut r, EffectTiming::WhenDigivolving, rose);
    assert!(drive_until(&mut r, "play or use"), "reactive offered for DS3");
    assert_eq!(
        non_pass(&r).len(),
        1,
        "the Digi-Egg is not offered (valid {:?})",
        r.pending_selection_view().unwrap().valid_action_ids
    );
    pick_hand(&mut r, 0, "DS3");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"DS3".to_string()));
    assert_eq!(hand_ids(&r, 0), vec!["DSEGG".to_string()]);
}

// ─── Play-or-use candidate legality (G-DSL-PLAY-OR-USE-CANDIDATE-LEGALITY) ───
// DCGO BT26_049.cs:125-131 CanSelectCardCondition: `EqualsTraits("DATA SQUAD")
// && GetCostItself <= maxCost && ((IsOption && !CanNotPlayThisOption) ||
// (HasPlayCost && CanPlayAsNewPermanent(card, payCost:false, activateClass)))`,
// and CanActivateCondition (:117-123) needs one such card in hand. CR 4.3 1-3-2
// (an impossible action is not carried out), 7-1-1 (playing places a new card
// with a play cost onto the field), 4-22 (an Option's colour requirement must be
// met to use it; Q1500 / Q2785: an effect use does not waive it).

/// A [DATA SQUAD] Option of `color`, use cost 2.
fn ds_option(id: &str, color: CardColor) -> digimon_engine::card_data::CardData {
    let mut c = digimon_engine::debug_runner::make_test_card(id, id);
    c.card_kind = digimon_engine::enums::CardKind::Option;
    c.level = None;
    c.dp = None;
    c.play_cost = 2;
    c.colors = vec![color];
    c.traits = vec!["DATA SQUAD".to_string()];
    c
}

/// Rosemon (green) + 1 opponent Digimon; `hand` in player 0's hand.
fn legality_runner(hand: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-049")
        .add_card(filler("FILLER"))
        .add_card(digimon("OPP", "Opp", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("DS3", "DS three", CardColor::Green, 3, 3, &["DATA SQUAD"]))
        .add_card(ds_option("DS-OPT-G", CardColor::Green))
        .add_card(ds_option("DS-OPT-R", CardColor::Red))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(3)
        .start();
    r.set_first_player(0);
    r.game.players[0].hand.clear();
    let rose = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP", Some(0));
    for id in hand {
        push_hand(&mut r, 0, id);
    }
    (r, rose)
}

/// Sorted card ids of the non-PASS candidates of the pending hand pick.
fn offered(r: &DebugRunner) -> Vec<String> {
    use digimon_engine::action::space::{PASS, PLAY_HAND_START};
    let Some(pending) = r.game.pending_selection.as_ref() else {
        return Vec::new();
    };
    let mut ids: Vec<String> = if pending.card_choice.is_some() {
        physical_menu::commits(&r.game)
            .into_iter()
            .filter_map(|row| row.card_id)
            .collect()
    } else {
        let p = pending.selecting_player as usize;
        r.pending_selection_view()
            .map(|v| v.valid_action_ids)
            .unwrap_or_default()
            .into_iter()
            .filter(|&a| a != PASS)
            .filter_map(|a| a.checked_sub(PLAY_HAND_START))
            .filter_map(|i| r.game.players[p].hand.get(i as usize))
            .map(|c| c.card_id(&r.game.card_data).to_string())
            .collect()
    };
    ids.sort();
    ids
}

fn add_player_lock(r: &mut DebugRunner, m: digimon_engine::enums::ModifierType) {
    use digimon_engine::enums::Expiry;
    use digimon_engine::modifiers::PlayerModifierEntry;
    r.game
        .modifiers
        .add_player_modifier(0, PlayerModifierEntry::simple(m, 0, Expiry::Permanent, None, 1));
}

/// A red [DATA SQUAD] Option with only green Rosemon on its owner's field: its
/// colour requirement is unmet (CR 4.3 4-22-2), so it can't be used and is not
/// offered; the green one is.
#[test]
fn bt26_049_option_with_unmet_colour_requirement_not_offered() {
    let (mut r, rose) = legality_runner(&["DS-OPT-G", "DS-OPT-R"]);
    fire(&mut r, EffectTiming::WhenDigivolving, rose);
    assert!(drive_until(&mut r, "play or use"), "reactive offered");
    assert_eq!(offered(&r), vec!["DS-OPT-G".to_string()]);
}

/// Only an unusable (red) Option in hand: no legal candidate, so the [All
/// Turns] clause does not activate (DCGO CanActivateCondition).
#[test]
fn bt26_049_only_unusable_option_does_not_activate() {
    let (mut r, rose) = legality_runner(&["DS-OPT-R"]);
    fire(&mut r, EffectTiming::WhenDigivolving, rose);
    assert!(!drive_until(&mut r, "play or use"), "nothing can be used");
    assert_eq!(hand_ids(&r, 0), vec!["DS-OPT-R".to_string()]);
}

/// "You can't use Option cards" (DCGO CanNotPlayThisOption): the Option is not
/// offered, the Digimon still is.
#[test]
fn bt26_049_option_lock_excludes_options() {
    let (mut r, rose) = legality_runner(&["DS-OPT-G", "DS3"]);
    add_player_lock(&mut r, digimon_engine::enums::ModifierType::CannotUseOptionCards);
    fire(&mut r, EffectTiming::WhenDigivolving, rose);
    assert!(drive_until(&mut r, "play or use"), "reactive offered");
    assert_eq!(offered(&r), vec!["DS3".to_string()]);
}

/// "You can't play Digimon by effects" (DCGO CanPlayAsNewPermanent false): the
/// Digimon is not offered, the Option still is.
#[test]
fn bt26_049_effect_play_prohibition_excludes_digimon() {
    let (mut r, rose) = legality_runner(&["DS-OPT-G", "DS3"]);
    add_player_lock(&mut r, digimon_engine::enums::ModifierType::CannotPlayDigimonByEffect);
    fire(&mut r, EffectTiming::WhenDigivolving, rose);
    assert!(drive_until(&mut r, "play or use"), "reactive offered");
    assert_eq!(offered(&r), vec!["DS-OPT-G".to_string()]);
}

/// A full battle area (DCGO CanPlayAsNewPermanent needs a free frame): the
/// Digimon is not offered; with only that Digimon in hand the clause does not
/// activate at all.
#[test]
fn bt26_049_full_battle_area_excludes_digimon() {
    let (mut r, rose) = legality_runner(&["DS-OPT-G", "DS3"]);
    r.game.rules.field_slots = r.game.players[0].battle_area.len() as u8;
    fire(&mut r, EffectTiming::WhenDigivolving, rose);
    assert!(drive_until(&mut r, "play or use"), "reactive offered");
    assert_eq!(offered(&r), vec!["DS-OPT-G".to_string()]);

    let (mut r, rose) = legality_runner(&["DS3"]);
    r.game.rules.field_slots = r.game.players[0].battle_area.len() as u8;
    fire(&mut r, EffectTiming::WhenDigivolving, rose);
    assert!(!drive_until(&mut r, "play or use"), "nothing can be played");
    assert_eq!(hand_ids(&r, 0), vec!["DS3".to_string()]);
}

#[path = "../../support/owned_card_choice.rs"]
mod physical_menu;
