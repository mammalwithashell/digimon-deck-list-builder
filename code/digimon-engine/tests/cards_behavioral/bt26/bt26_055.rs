//! BT26-055 Giromon — Lv.5 Black, Mine/DM/Ver.3.
//!
//! <Fragment <2>>
//! [On Play] [When Digivolving] [Counter] [Once Per Turn] You may place 1 card
//! in your hand face down as this Digimon's bottom digivolution card. Then,
//! you may delete 1 of your Digimon with the [Ver.3] trait and all of your
//! opponent's Digimon with the lowest play cost.
//! Inherited: [All Turns] [Once Per Turn] When this Digimon would leave the
//! battle area, trash your opponent's top security card.
//!
//! DCGO: BT26/Black/BT26_055.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};

const CARD_ID: &str = "BT26-055";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-055")
        .add_card(filler("FILLER"))
        .add_card(digimon("VER3", "Gear", CardColor::Black, 4, 5, &["Ver.3"]))
        .add_card(digimon("OPPA", "OppA", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("OPPB", "OppB", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("OPPC", "OppC", CardColor::Red, 5, 7, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(1, &["FILLER"; 3])
        .memory(5)
        .start();
    r.set_first_player(0);
    r
}

fn own_field_action(h: digimon_engine::permanent::PermanentHandle) -> u16 {
    digimon_engine::action::space::encode_attack(0, h.index as u16)
}

#[test]
fn bt26_055_places_face_down_then_deletes_ver3_and_lowest_cost_batch() {
    let mut r = setup();
    let giro = r.place_on_field(0, CARD_ID, Some(0));
    let ver3 = r.place_on_field(0, "VER3", Some(0));
    r.place_on_field(1, "OPPA", Some(0));
    r.place_on_field(1, "OPPB", Some(0));
    r.place_on_field(1, "OPPC", Some(0));
    push_hand(&mut r, 0, "FILLER");
    fire(&mut r, EffectTiming::OnPlay, giro);
    let v = r.pending_selection_view().expect("optional hand pick");
    assert!(v.is_optional);
    pick_hand(&mut r, 0, "FILLER");
    assert_eq!(sources(&r, giro), 1);
    assert_eq!(face_down_flags(&r, giro), vec![true]);
    r.execute_branch(0).expect("Yes — delete");
    // Giromon itself is [Ver.3] too: 2 candidates.
    assert_eq!(non_pass(&r).len(), 2);
    r.execute_action(0, own_field_action(ver3)).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(
        field_ids(&r, 1),
        vec!["OPPC".to_string()],
        "both cost-3 ties deleted"
    );
    assert_eq!(field_ids(&r, 0), vec![CARD_ID.to_string()]);
}

#[test]
fn bt26_055_no_deletes_on_no_and_once_per_turn_holds() {
    let mut r = setup();
    let giro = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPPA", Some(0));
    push_hand(&mut r, 0, "FILLER");
    push_hand(&mut r, 0, "FILLER");
    fire(&mut r, EffectTiming::WhenDigivolving, giro);
    pick_hand(&mut r, 0, "FILLER");
    r.execute_branch(1).expect("No");
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPPA".to_string()]);
    // A card was placed ⇒ the [Once Per Turn] is spent.
    fire(&mut r, EffectTiming::WhenDigivolving, giro);
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn bt26_055_doing_nothing_refunds_once_per_turn() {
    let mut r = setup();
    let giro = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPPA", Some(0));
    push_hand(&mut r, 0, "FILLER");
    fire(&mut r, EffectTiming::OnPlay, giro);
    pass(&mut r, 0); // place nothing
    r.execute_branch(1).expect("No");
    let _ = r.auto_resolve();
    fire(&mut r, EffectTiming::OnPlay, giro);
    pass(&mut r, 0);
    r.execute_branch(0).expect("Yes this time");
    // Only Giromon is [Ver.3]: it is picked (mandatory) and deleted with OPPA.
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).is_empty());
    assert!(field_ids(&r, 0).is_empty());
}

/// The [Counter] timing of the same printed "You may place ... Then, you may
/// delete ..." effect: with both halves declined nothing executed, so the use
/// is kept at [Counter] as at [On Play] / [When Digivolving] (owner-decided
/// 2026-10-21, W13; DCGO BT26_055.cs runs one shared effect for all three
/// timings and RemoveUse()s when neither the placement nor the deletion
/// happened). The kept use is the one [Once Per Turn] of every timing
/// (official Q1550).
#[test]
fn bt26_055_counter_doing_nothing_keeps_once_per_turn() {
    let mut r = setup();
    let giro = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPPA", Some(0));
    push_hand(&mut r, 0, "FILLER");
    fire(&mut r, EffectTiming::CounterEffect, giro);
    pass(&mut r, 0); // place nothing
    r.execute_branch(1).expect("No");
    assert!(r.pending_selection().is_none(), "no decision is left");
    assert_eq!(field_ids(&r, 1), vec!["OPPA".to_string()]);
    assert_eq!(r.hand_size(0), 1, "nothing was placed");
    fire(&mut r, EffectTiming::CounterEffect, giro);
    assert!(
        r.pending_selection_view().is_some_and(|v| v.is_optional),
        "a [Counter] that declined both halves keeps the [Once Per Turn]"
    );
    pass(&mut r, 0);
    r.execute_branch(1).expect("No");
    assert!(r.pending_selection().is_none(), "no decision is left");
    fire(&mut r, EffectTiming::OnPlay, giro);
    assert!(
        r.pending_selection_view().is_some_and(|v| v.is_optional),
        "and the [On Play] timing of the same effect is still offered"
    );
}

/// One half executed at [Counter] spends the one [Once Per Turn] of every
/// timing (official Q1550; W13: spent as soon as either half executes).
#[test]
fn bt26_055_counter_placing_spends_once_per_turn_for_every_timing() {
    let mut r = setup();
    let giro = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPPA", Some(0));
    push_hand(&mut r, 0, "FILLER");
    push_hand(&mut r, 0, "FILLER");
    fire(&mut r, EffectTiming::CounterEffect, giro);
    pick_hand(&mut r, 0, "FILLER");
    r.execute_branch(1).expect("No");
    assert!(r.pending_selection().is_none(), "no decision is left");
    assert_eq!(sources(&r, giro), 1, "a card was placed");
    fire(&mut r, EffectTiming::CounterEffect, giro);
    assert!(r.pending_selection_view().is_none(), "[Counter] spent");
    fire(&mut r, EffectTiming::OnPlay, giro);
    assert!(
        r.pending_selection_view().is_none(),
        "the [On Play] timing of the same effect shares that use"
    );
}

/// One half executed at [On Play] spends the [Counter] timing too.
#[test]
fn bt26_055_on_play_placing_spends_the_counter_timing() {
    let mut r = setup();
    let giro = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPPA", Some(0));
    push_hand(&mut r, 0, "FILLER");
    push_hand(&mut r, 0, "FILLER");
    fire(&mut r, EffectTiming::OnPlay, giro);
    pick_hand(&mut r, 0, "FILLER");
    r.execute_branch(1).expect("No");
    assert!(r.pending_selection().is_none(), "no decision is left");
    fire(&mut r, EffectTiming::CounterEffect, giro);
    assert!(
        r.pending_selection_view().is_none(),
        "the [On Play] use spent the [Counter] timing"
    );
}

/// There is no "nothing was possible" state for this effect: with an empty
/// hand (nothing to place) and no opponent Digimon, the delete half is still
/// offered, because Giromon is itself a [Ver.3] Digimon. Declining it is then
/// declining something possible, which keeps the use (W13); choosing it
/// spends the use.
#[test]
fn bt26_055_empty_hand_still_offers_the_delete_half() {
    let mut r = setup();
    let giro = r.place_on_field(0, CARD_ID, Some(0));
    assert_eq!(r.hand_size(0), 0);
    fire(&mut r, EffectTiming::CounterEffect, giro);
    let v = r.pending_selection_view().expect("the delete half is offered");
    assert!(v.prompt.contains("Delete"), "{:?}", v.prompt);
    r.execute_branch(1).expect("No");
    assert!(r.pending_selection().is_none(), "no decision is left");
    fire(&mut r, EffectTiming::CounterEffect, giro);
    assert!(
        r.pending_selection_view().is_some(),
        "declining the possible half kept the [Once Per Turn]"
    );
    r.execute_branch(0).expect("Yes");
    let v = r.pending_selection_view().expect("the [Ver.3] pick: Giromon is the only one");
    assert_eq!(v.valid_action_ids.len(), 1, "{v:?}");
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
    assert!(r.pending_selection().is_none(), "no decision is left");
    assert!(field_ids(&r, 0).is_empty(), "Giromon deleted itself");
}

/// One printed effect, one clause: [On Play] [When Digivolving] [Counter]
/// share a single [Once Per Turn] (official Q1550) and its W13 refund, as
/// DCGO's one shared effect does.
#[test]
fn bt26_055_one_clause_carries_all_three_timings() {
    use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
    let r = setup();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    let opt: Vec<_> = card
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::Counter) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(opt.len(), 1, "one clause for the printed effect");
    for timing in [CompiledTiming::OnPlay, CompiledTiming::WhenDigivolving] {
        assert!(opt[0].when.contains(&timing), "{timing:?}");
    }
    assert!(opt[0].once_per_turn);
}

#[test]
fn bt26_055_counter_timing_fires() {
    let mut r = setup();
    let giro = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "FILLER");
    fire(&mut r, EffectTiming::CounterEffect, giro);
    let v = r.pending_selection_view().expect("hand pick at [Counter]");
    assert!(v.is_optional);
}

/// Answer every remaining decision of the current attack (PASS where
/// optional, else the first legal action) and report whether Giromon's
/// [Counter] (`declare`) was offered again in the counter timing.
fn bt26_055_finish_attack(r: &mut DebugRunner, declare: u16) -> bool {
    use digimon_engine::action::space::PASS;
    use digimon_engine::enums::GamePhase;
    let mut reoffered = false;
    for _ in 0..16 {
        let Some(v) = r.pending_selection_view() else {
            break;
        };
        if r.current_phase() == GamePhase::CounterTiming && v.valid_action_ids.contains(&declare) {
            reoffered = true;
        }
        let action = if v.valid_action_ids.contains(&PASS) { PASS } else { v.valid_action_ids[0] };
        r.execute_action(v.selecting_player, action).unwrap();
    }
    assert!(r.pending_selection().is_none(), "no decision is left");
    reoffered
}

/// Real combat. [Counter] is declared in the counter timing (CR 4.3 11-3-1,
/// 15-16-14-1), and only 1 [Counter] effect can be activated per attack
/// (CR 4.3 11-3-2; official Q7058). Declaring Giromon's [Counter] and
/// declining both halves keeps the [Once Per Turn] (W13), but that attack's
/// counter timing is then over: Giromon is not offered again in the same
/// attack, so there is no declare-and-pass cycle. The kept use is offered at
/// the next attack's counter timing that turn; placing a card there spends
/// it, and a third attack opens no counter timing for it.
#[test]
fn bt26_055_real_counter_declare_and_pass_closes_the_window_and_keeps_the_use() {
    use digimon_engine::enums::GamePhase;
    let mut r = setup();
    // 5000 DP attackers survive the 3000 DP security Digimon, so the
    // handles stay valid for the later attacks.
    let first = r.place_on_field(0, "OPPC", Some(0));
    let second = r.place_on_field(0, "OPPC", Some(0));
    let giro = r.place_on_field(1, CARD_ID, Some(0));
    push_hand(&mut r, 1, "FILLER");
    push_hand(&mut r, 1, "FILLER");
    let declare = own_field_action(giro);

    assert_ne!(r.attack_player(first, 1, false), digimon_engine::combat::AttackResult::Invalid);
    assert_eq!(r.current_phase(), GamePhase::CounterTiming);
    assert!(r.pending_selection_view().unwrap().valid_action_ids.contains(&declare));
    r.execute_action(1, declare).unwrap();
    pass(&mut r, 1); // place nothing
    r.execute_branch(1).expect("Don't delete");
    assert!(
        !bt26_055_finish_attack(&mut r, declare),
        "CR 4.3 11-3-2: one [Counter] per attack, so the declaration is not offered again"
    );
    assert!(r.game.pending_attack.is_none(), "the first attack finished");
    assert_eq!(r.hand_size(1), 2, "nothing was placed");
    assert_eq!(sources(&r, giro), 0);

    assert_ne!(r.attack_player(second, 1, false), digimon_engine::combat::AttackResult::Invalid);
    assert_eq!(r.current_phase(), GamePhase::CounterTiming, "the kept use is offered at the next attack");
    assert!(r.pending_selection_view().unwrap().valid_action_ids.contains(&declare));
    r.execute_action(1, declare).unwrap();
    pick_hand(&mut r, 1, "FILLER");
    r.execute_branch(1).expect("Don't delete");
    bt26_055_finish_attack(&mut r, declare);
    assert_eq!(sources(&r, giro), 1, "a card was placed");

    r.game.players[0].battle_area[first.index as usize].is_suspended = false;
    let security_before = r.security_count(1);
    assert_ne!(r.attack_player(first, 1, false), digimon_engine::combat::AttackResult::Invalid);
    bt26_055_finish_attack(&mut r, declare);
    assert_eq!(r.security_count(1), security_before - 1, "the third attack happened");
    assert!(
        !(r.current_phase() == GamePhase::CounterTiming
            && r.pending_selection_view().is_some_and(|v| v.valid_action_ids.contains(&declare))),
        "the [Once Per Turn] was spent by placing a card"
    );
}

/// Uses are counted per copy (CR 4.3 15-14-1-3): spending one Giromon's
/// [Counter] leaves the other copy's use, and the turn change resets the
/// spent copy (CR 4.3 15-14-1-5-1).
#[test]
fn bt26_055_two_copies_have_separate_uses_and_the_use_resets_at_turn_change() {
    let mut r = setup();
    let a = r.place_on_field(0, CARD_ID, Some(0));
    let b = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "FILLER");
    push_hand(&mut r, 0, "FILLER");
    fire(&mut r, EffectTiming::CounterEffect, a);
    pick_hand(&mut r, 0, "FILLER");
    r.execute_branch(1).expect("Don't delete");
    assert!(r.pending_selection().is_none(), "no decision is left");
    assert_eq!(sources(&r, a), 1, "copy A placed a card");
    fire(&mut r, EffectTiming::CounterEffect, a);
    assert!(r.pending_selection_view().is_none(), "copy A's use is spent");
    fire(&mut r, EffectTiming::CounterEffect, b);
    assert!(
        r.pending_selection_view().is_some_and(|v| v.is_optional),
        "copy B has its own [Once Per Turn]"
    );
    pass(&mut r, 0);
    r.execute_branch(1).expect("Don't delete");
    assert!(r.pending_selection().is_none(), "no decision is left");
    r.end_turn();
    assert!(r.pending_selection().is_none(), "no decision is left");
    fire(&mut r, EffectTiming::CounterEffect, a);
    assert!(
        r.pending_selection_view().is_some(),
        "the turn change reset copy A's [Once Per Turn]"
    );
}

#[test]
fn bt26_055_has_fragment_2() {
    let r = setup();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    let dbg = format!("{card:?}");
    assert!(dbg.contains("Fragment"), "Fragment keyword granted");
}

#[test]
fn bt26_055_inherited_would_leave_trashes_opponent_top_security() {
    let mut r = setup();
    let carrier = r.place_stack(0, &[CARD_ID, "FILLER"]);
    r.game.set_effect_source_player_for_test(Some(1));
    r.game.delete_permanent_with_effects(carrier);
    r.game.set_effect_source_player_for_test(None);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(1), 2);
    assert!(field_ids(&r, 0).is_empty(), "it still leaves");
}
