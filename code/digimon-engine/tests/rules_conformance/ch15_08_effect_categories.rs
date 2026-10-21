//! Chapter 15-8 "Effect Categories" (Comprehensive Rules Ver. 4.3):
//! trigger-type (§15-8-3), activation-type (§15-8-4) and immediate-type
//! (§15-8-5) effects.
//!
//! Every test asserts the RULE, never the engine's current output. A test the
//! engine fails is `#[ignore]`d with the gap id that explains it (discover then
//! pin); see `docs/RUST_ENGINE_GAPS.md` and `qa/rules/cr_obligations_status.yaml`.
//!
//! Fixture cards: `fixtures/common/*.yaml` + `fixtures/ch15_08/*.yaml`, one
//! synthetic card per file, authored to printed-style text (each clause's
//! `summary:` is that text).
//!
//! §15-8-3-9-2 (processing conditions checked at activation) is proven in
//! `ch15_04_effect_states::pending_effect_whose_condition_lapsed_is_dropped`
//! and is not repeated here.
//!
//! The §15-8-3-8 test reads the played Digimon's level through the
//! snapshot-reading `event_target_level` formula (the event card's printed
//! level), while the §15-8-3-9-1/3 test reads it through the live-reading
//! `event_target_level_gte` / `_eq` conditions (the event permanent's current
//! top). Both tests show the DSL can express each semantics; they do not show
//! the engine chooses the right one unaided -- the card author picks it.

use crate::fixtures;
use crate::support::*;
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::parse_printed_keywords;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{GamePhase, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

// ─── §15-8-3-2 ──────────────────────────────────────────────────────────────

/// A board for the §15-8-3-2 test: P0's `PLAIN_TAMER` with one face-down card
/// under it, P0's observer `OBS_TAMER_TRASH`, and `trasher` in hand.
fn tamer_trash_board(trasher: &str) -> (DebugRunner, PermanentHandle) {
    let mut r = fixtures::builder(&[fixtures::ch15_08_cards()])
        .hand(0, &[trasher])
        .deck(0, &[FILL; 4])
        .memory(5)
        .start();
    let tamer = r.place_on_field(0, "PLAIN_TAMER", Some(0));
    attach_face_down(&mut r, tamer, FILL);
    r.place_on_field(0, "OBS_TAMER_TRASH", Some(0));
    (r, tamer)
}

/// §15-8-3-2 "Trigger-type effects can't activate during the processing for a
/// rule or effect."
///
/// Owner ruling 2026-10-10 (#1, recorded in the conformance design §6): a
/// trigger without interruptive "when … would" timing waits until the effect
/// that caused it finishes.
///
/// "Trash the bottom face-down card from under 1 of your Tamers. Then, gain 1
/// memory." trashes the card under P0's Tamer, which triggers P0's
/// `OBS_TAMER_TRASH` ("[All Turns] When effects trash cards from under your
/// Tamers, gain 1 memory"). That observer is a trigger-type effect: it
/// activates only after the causing effect's "Then, gain 1 memory". Two
/// causes carry the same text:
/// - the Option `MAIN_TRASH_UNDER_MEM` ([Main]);
/// - the Digimon `OP_TRASH_UNDER_MEM` ([On Play]), itself a triggered effect.
#[test]
fn trigger_type_waits_for_causing_effect_to_finish() {
    cite!(
        "15-8-3-2",
        "db9514d0faa35f84c637c60f04cbbe89569aafaa1a4d579674c36b56820b69e1"
    );
    // Cause: an Option's [Main] effect.
    let (mut r, tamer) = tamer_trash_board("MAIN_TRASH_UNDER_MEM");
    let cp = r.event_checkpoint();
    use_option(&mut r, 0, "MAIN_TRASH_UNDER_MEM");
    resolve_prompts(&mut r); // the Tamer pick (the Option's trash is no decision, §9-1-5)
    assert_eq!(
        r.game.players[0].battle_area[tamer.index as usize]
            .card_sources
            .len(),
        1,
        "the face-down card under the Tamer was trashed"
    );
    assert_eq!(
        memory_sources(&r, cp),
        vec![
            (0, "MAIN_TRASH_UNDER_MEM".to_string()),
            (0, "OBS_TAMER_TRASH".to_string()),
        ],
        "[Main] cause: the observer activates only after the trashing effect's remaining text"
    );

    // Cause: an [On Play] effect.
    let (mut r, tamer) = tamer_trash_board("OP_TRASH_UNDER_MEM");
    let cp = r.event_checkpoint();
    r.play(0, 0);
    resolve_prompts(&mut r); // the Tamer pick
    assert_eq!(
        r.game.players[0].battle_area[tamer.index as usize]
            .card_sources
            .len(),
        1,
        "the face-down card under the Tamer was trashed"
    );
    assert_eq!(
        memory_sources(&r, cp),
        vec![
            (0, "OP_TRASH_UNDER_MEM".to_string()),
            (0, "OBS_TAMER_TRASH".to_string()),
        ],
        "[On Play] cause: the observer activates only after the trashing effect's remaining text"
    );
}

/// §15-8-3-2 with official Q&A Q6958 (Pinamon BT26-005, "Does this card's
/// inherited effect allow me to play the card under the Tamer after trashing
/// it?" -- "Yes, you can play it."): a play from the trash of the very card
/// the same effect's cost just trashed from under a Tamer, while the
/// source-trash trigger that trash raised is still pending.
///
/// `MAIN_TRASH_UNDER_PLAY` ("[Main] By trashing the bottom face-down card from
/// under 1 of your Tamers, you may play 1 play cost 3 or lower Tamer card from
/// your trash without paying the cost") trashes the face-down `PLAIN_TAMER`
/// under P0's Tamer -- the only Tamer card in P0's trash -- which triggers P0's
/// `OBS_TAMER_TRASH`. That trigger waits for the effect to finish (§15-8-3-2),
/// so it is pending while the play runs; nothing lets a pending trigger change
/// what the resolving effect may play (§15-7-1: once the optional condition is
/// performed, the processing after it is performed). The just-trashed card is
/// played, and the observer activates after.
#[test]
fn play_from_trash_of_card_just_trashed_while_source_trash_trigger_pends() {
    cite!(
        "15-8-3-2",
        "db9514d0faa35f84c637c60f04cbbe89569aafaa1a4d579674c36b56820b69e1"
    );
    let mut r = fixtures::builder(&[fixtures::ch15_08_cards()])
        .hand(0, &["MAIN_TRASH_UNDER_PLAY"])
        .deck(0, &[FILL; 4])
        .memory(5)
        .start();
    let tamer = r.place_on_field(0, "PLAIN_TAMER", Some(0));
    attach_face_down(&mut r, tamer, "PLAIN_TAMER");
    r.place_on_field(0, "OBS_TAMER_TRASH", Some(0));
    let cp = r.event_checkpoint();
    use_option(&mut r, 0, "MAIN_TRASH_UNDER_PLAY");
    step_prompt(&mut r); // the Tamer pick (one Tamer carries a face-down card)
    assert_eq!(
        trash_ids(&r, 0),
        owned(&["PLAIN_TAMER"]),
        "the face-down card under the Tamer was trashed"
    );
    let pick = digimon_engine::action::space::TRASH_EFFECT_START;
    {
        let sel = r.pending_selection().expect("the trash pick");
        assert!(sel.valid_action_ids.contains(&pick), "{sel:?}");
    }
    r.execute_action(0, pick).expect("pick the just-trashed Tamer");
    assert_eq!(
        field_top_ids(&r, 0),
        owned(&["PLAIN_TAMER", "OBS_TAMER_TRASH", "PLAIN_TAMER"]),
        "Q6958: the card the cost just trashed is played"
    );
    assert!(
        !trash_ids(&r, 0).contains(&"PLAIN_TAMER".to_string()),
        "the played card left the trash: {:?}",
        trash_ids(&r, 0)
    );
    resolve_prompts(&mut r);
    assert_eq!(
        memory_sources(&r, cp),
        vec![(0, "OBS_TAMER_TRASH".to_string())],
        "the pending source-trash trigger activates after the effect"
    );
}

// ─── §15-8-3-5 ──────────────────────────────────────────────────────────────

/// A board for the §15-8-3-5 test: P0's `LV5_PLAIN` (7000 DP) attacks P1's
/// suspended stack `[INH_OD_MEM, OD_RET_SELF (top, 4000 DP)]` and deletes it
/// in battle.
fn inherited_on_deletion_board() -> DebugRunner {
    let mut r = fixtures::builder(&[fixtures::ch15_08_cards()])
        .deck(0, &[FILL; 4])
        .deck(1, &[FILL; 4])
        .memory(3)
        .start();
    let attacker = r.place_on_field(0, "LV5_PLAIN", Some(0));
    let defender = r.place_stack(1, &["INH_OD_MEM", "OD_RET_SELF"]);
    set_suspended(&mut r, defender, true);
    r.attack_digimon(attacker, defender, false);
    r
}

/// §15-8-3-5 "If a trigger-type effect triggers when a card is deleted, the
/// card where it will be pending activation is the card that was the top card
/// in the original area. (Example: Upon the deletion of a Digimon with an
/// inherited effect that reads "[On Deletion] Gain 1 memory" on its
/// digivolution card, that effect will trigger, and it will be pending
/// activation for the card that was the top card of that Digimon.)"
///
/// P1's stack `[INH_OD_MEM, OD_RET_SELF]` is deleted in battle. Two effects
/// trigger: the inherited "[On Deletion] Gain 1 memory" (offered once) and the
/// top card's own "[On Deletion] Return this card from your trash to the
/// hand". The inherited effect is pending on the former TOP card, so:
/// - Control: activating the inherited effect first, it gains 1 memory.
/// - Activating the return first moves the top card out of the trash; it
///   becomes a new card, so the inherited effect pending on it can no longer
///   activate (§15-4-4-3). (DCGO agrees: `CanActivateOnDeletion` requires
///   `IsTopCardStillInTrash`, `CardEffectCommons/CanUseEffects/OnDeletion.cs`.)
#[test]
fn inherited_on_deletion_is_pending_on_the_former_top_card() {
    cite!(
        "15-8-3-5",
        "01057988d79cb7e169d4d794dad0396abb58a958381b1786463ea48e466c6292"
    );
    // Control: inherited effect first.
    let mut r = inherited_on_deletion_board();
    assert!(
        field_top_ids(&r, 1).is_empty(),
        "the stack was deleted in battle"
    );
    let choices = order_choices(&r, 1);
    assert_eq!(choices.len(), 2, "{choices:?}");
    assert_eq!(
        choices
            .iter()
            .filter(|l| branch_card(l) == "INH_OD_MEM")
            .count(),
        1,
        "the inherited [On Deletion] is offered exactly once: {choices:?}"
    );
    let cp = r.event_checkpoint();
    choose(&mut r, "INH_OD_MEM");
    assert_eq!(
        memory_sources(&r, cp),
        vec![(1, "INH_OD_MEM".to_string())],
        "the inherited effect activates while the former top card is still in the trash"
    );
    assert_eq!(hand_ids(&r, 1), owned(&["OD_RET_SELF"]));

    // The former top card leaves the trash first.
    let mut r = inherited_on_deletion_board();
    let choices = order_choices(&r, 1);
    assert_eq!(choices.len(), 2, "{choices:?}");
    let cp = r.event_checkpoint();
    choose(&mut r, "OD_RET_SELF");
    assert_eq!(
        hand_ids(&r, 1),
        owned(&["OD_RET_SELF"]),
        "the former top card returned to the hand"
    );
    assert!(
        memory_sources(&r, cp).is_empty(),
        "the inherited effect was pending on the former top card, which became a new card: it \
         can no longer activate"
    );
    assert!(
        r.pending_selection().is_none(),
        "{:?}",
        r.pending_selection()
    );
}

// ─── §15-8-3-6 / §6-6-4 ─────────────────────────────────────────────────────

/// §15-8-3-6 "If a trigger-type effect triggers at the end of the turn
/// timing, it will trigger as soon as the end of the turn timing arrives.
/// Once an effect triggers, it will remain triggered, even if the memory
/// increases or decreases."
/// §6-6-4 "If the memory moves to 0 or more at the end of the turn, the end of
/// the turn will be postponed and the current phase will continue."
///
/// P0 passes (memory to 3 on the opponent's side). P0's Tamers `EOT_MEM3`
/// ("[End of Your Turn] Gain 3 memory") and `EOT_MEM` ("[End of Your Turn]
/// Gain 2 memory") both trigger. P0 activates `EOT_MEM3` first, bringing the
/// memory back to 0; `EOT_MEM` stays triggered and still resolves (memory 2).
/// The memory is on P0's side, so P0's turn continues in the main phase.
#[test]
fn end_of_turn_trigger_persists_through_memory_change() {
    cite!(
        "15-8-3-6",
        "850118383d943c126f1ce0eb0d18485a87d211364da31d0c8e12036a5cdef4fb"
    );
    cite!(
        "6-6-4",
        "b36e6c0d009621d1b40d9f2bc63107a4f6e02b1d7abdb1bcbdf7fa293aa6bd09"
    );
    let mut r = fixtures::builder(&[fixtures::ch15_08_cards()])
        .deck(0, &[FILL; 4])
        .deck(1, &[FILL; 4])
        .memory(0)
        .start();
    r.place_on_field(0, "EOT_MEM", Some(0));
    r.place_on_field(0, "EOT_MEM3", Some(0));
    let cp = r.event_checkpoint();

    r.pass_turn();

    let choices = order_choices(&r, 0);
    assert_eq!(
        choices.len(),
        2,
        "both [End of Your Turn] effects trigger: {choices:?}"
    );
    assert_eq!(r.memory(), -3, "nothing activated before the choice");
    choose(&mut r, "EOT_MEM3");

    assert_eq!(
        memory_sources(&r, cp),
        vec![(0, "EOT_MEM3".to_string()), (0, "EOT_MEM".to_string())],
        "EOT_MEM stays triggered after the memory returned to 0 and still activates"
    );
    assert_eq!(r.memory(), 2);
    assert_eq!(r.turn_player(), 0, "the end of the turn was postponed");
    assert_eq!(
        r.current_phase(),
        GamePhase::Main,
        "the turn continues in the main phase"
    );
}

// ─── §15-8-3-7 ──────────────────────────────────────────────────────────────

/// A board for the §15-8-3-7 test: P0 has 3 security cards, the Tamer
/// `PLAY_RET_LE_SEC` on the field and `REC_PLAY` in hand; P1 has a level 4
/// `LV4_PLAIN`.
fn security_reference_board() -> DebugRunner {
    let mut r = fixtures::builder(&[fixtures::ch15_08_cards()])
        .hand(0, &["REC_PLAY"])
        .deck(0, &[FILL; 4])
        .security(0, &[FILL; 3])
        .memory(5)
        .start();
    r.place_on_field(0, "PLAY_RET_LE_SEC", Some(0));
    r.place_on_field(1, "LV4_PLAIN", Some(0));
    r
}

/// §15-8-3-7 "If an unspecified reference is to be made during the processing
/// for a trigger-type effect, the reference is made in the state when the
/// processing is being performed. (Example: If an effect reads "[All Turns]
/// [Once Per Turn] When a card is added to your security stack, return 1 of
/// your opponent's Digimon with a level less than or equal to the number of
/// your security cards to the hand," if you have 4 security cards when
/// referencing them for the processing, you would return 1 of your opponent's
/// level 4 or lower Digimon to the hand.)"
///
/// Playing `REC_PLAY` ("[On Play] <Recovery +1 (Deck)>") triggers both its
/// `[On Play]` and the Tamer's "When one of your Digimon is played, return 1
/// of your opponent's Digimon with a level less than or equal to the number of
/// your security cards to the hand". The security count is not part of the
/// trigger condition, so it is read when the Tamer's effect is processed:
/// - Control: the Tamer first -- 3 security cards, the level 4 Digimon stays.
/// - The recovery first -- 4 security cards when processed, it is returned.
#[test]
fn trigger_reference_uses_state_at_processing() {
    cite!(
        "15-8-3-7",
        "a715ca6375466bbc5aa3adbfcda365c89ebb3639b4cb10d714d1a94efb9992e1"
    );
    // Control: the Tamer's effect first.
    let mut r = security_reference_board();
    r.play(0, 0);
    let choices = order_choices(&r, 0);
    assert_eq!(choices.len(), 2, "{choices:?}");
    choose(&mut r, "PLAY_RET_LE_SEC");
    resolve_prompts(&mut r);
    assert_eq!(r.security_count(0), 4, "the recovery resolved afterwards");
    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["LV4_PLAIN"]),
        "with 3 security cards at processing, the level 4 Digimon can't be returned"
    );

    // The recovery first.
    let mut r = security_reference_board();
    r.play(0, 0);
    let choices = order_choices(&r, 0);
    assert_eq!(choices.len(), 2, "{choices:?}");
    choose(&mut r, "REC_PLAY");
    resolve_prompts(&mut r);
    assert_eq!(r.security_count(0), 4);
    assert!(
        field_top_ids(&r, 1).is_empty(),
        "4 security cards when the Tamer's effect is processed: the level 4 Digimon is returned"
    );
    assert_eq!(hand_ids(&r, 1), owned(&["LV4_PLAIN"]));
}

// ─── §15-8-3-8 ──────────────────────────────────────────────────────────────

/// §15-8-3-8 "If a reference in the same state as the effect's trigger
/// conditions is to be made during the processing for a trigger-type effect,
/// the state where the effect has triggered will be referenced. The state
/// where the effect has triggered will be referenced, even if the state
/// changes after the effect triggers. This refers to effects where text such
/// as "when another of your Digimon is played" is a trigger condition and a
/// reference is made using text such as "Digimon with a level less than or
/// equal to the played Digimon." (Example: When an effect plays a level 4
/// Digimon and an effect reads "[All Turns] When an effect plays another of
/// your Digimon, delete 1 of your opponent's Digimon with a level less than
/// or equal to the played Digimon," this effect deletes 1 of your opponent's
/// level 4 or lower Digimon, even if the Digimon digivolves into a level 5
/// Digimon before this effect activates.)"
///
/// The rule's own example. P0's `MAIN_PLAY_LV4` plays the level 4
/// `OP_DIGI_LV4`, triggering its "[On Play] This Digimon may digivolve into a
/// level 5 Digimon card in your hand without paying the cost" and P0's Tamer
/// `OBS_EFFPLAY_DEL_LE` (the example text). P0 digivolves first (into
/// `LV5_PLAIN`); the Tamer's effect still reads level 4: of P1's level 4 and
/// level 5 Digimon, only the level 4 one can be deleted.
#[test]
fn trigger_reference_tied_to_condition_uses_trigger_time_state() {
    cite!(
        "15-8-3-8",
        "f9fa5d72cc53f4d558f8cb42b2bac4ae3cd435c46ad3cb75edb2e0efc5ab8a4e"
    );
    let mut r = fixtures::builder(&[fixtures::ch15_08_cards()])
        .hand(0, &["MAIN_PLAY_LV4", "OP_DIGI_LV4", "LV5_PLAIN"])
        .deck(0, &[FILL; 4])
        .memory(3)
        .start();
    r.place_on_field(0, "OBS_EFFPLAY_DEL_LE", Some(0));
    r.place_on_field(1, "LV4_PLAIN", Some(0));
    r.place_on_field(1, "LV5_PLAIN", Some(0));

    use_option(&mut r, 0, "MAIN_PLAY_LV4");
    pick_hand_card(&mut r, 0, "OP_DIGI_LV4");

    let choices = order_choices(&r, 0);
    assert_eq!(
        choices.len(),
        2,
        "the [On Play] and the Tamer's observer both triggered: {choices:?}"
    );
    choose(&mut r, "OP_DIGI_LV4");
    pick_hand_card(&mut r, 0, "LV5_PLAIN");

    assert_eq!(
        field_top_ids(&r, 0),
        owned(&["OBS_EFFPLAY_DEL_LE", "LV5_PLAIN"]),
        "the played Digimon digivolved to level 5 before the Tamer's effect activated"
    );
    // The Tamer's deletion target: only the level 4 Digimon (level <= 4, the
    // played Digimon's level when the effect triggered).
    step_prompt(&mut r);
    resolve_prompts(&mut r);

    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["LV5_PLAIN"]),
        "the level 4 Digimon was deleted; the level 5 one was never a legal target"
    );
    assert_eq!(trash_ids(&r, 1), owned(&["LV4_PLAIN"]));
}

// ─── §15-8-3-9-1 / §15-8-3-9-3 ──────────────────────────────────────────────

/// A board for the §15-8-3-9-1/3 test: P1's Tamer `observer` on the field;
/// P0 holds `OP_DIGI_LV3` ("[On Play] This Digimon may digivolve into a level
/// 4 Digimon card in your hand without paying the cost") and `LV4_PLAIN`. P0
/// plays `OP_DIGI_LV3`.
fn processing_reference_board(observer: &str) -> (DebugRunner, PermanentHandle) {
    let mut r = fixtures::builder(&[fixtures::ch15_08_cards()])
        .hand(0, &["OP_DIGI_LV3", "LV4_PLAIN"])
        .deck(0, &[FILL; 4])
        .deck(1, &[FILL; 4])
        .memory(5)
        .start();
    let tamer = r.place_on_field(1, observer, Some(0));
    r.play(0, 0);
    (r, tamer)
}

/// §15-8-3-9-1 "A trigger-type effect with processing conditions will always
/// trigger as soon as its trigger conditions are met, then the effect will
/// activate if the processing conditions are met."
/// §15-8-3-9-3 "If a reference is to be made in the processing conditions
/// for a trigger-type effect, the reference is made in the state when the
/// processing is being performed. (Example: If an effect reads "[All Turns]
/// When your opponent plays a Digimon, by suspending this Tamer, if that
/// Digimon is level 4 or higher, gain 1 memory. If it is level 3, <Draw 1>,"
/// you will gain 1 memory if your opponent's Digimon is level 4 upon being
/// referenced when this effect activates.)"
///
/// The rule's own example. P0 (turn player) plays the level 3 `OP_DIGI_LV3`;
/// P1's Tamer triggers. The turn player's `[On Play]` activates first
/// (§15-4-3-5):
/// - Control: P0 declines the digivolve -- the Digimon is level 3 when P1's
///   effect is processed, so P1 draws 1.
/// - P0 digivolves it into `LV4_PLAIN` -- it was level 3 when the effect
///   triggered ("level 4 or higher" unmet then), but level 4 when referenced
///   at processing, so P1 gains 1 memory and doesn't draw.
///
/// Single-branch §15-8-3-9-1 witness, `OBS_OPP_PLAY_GE4` ("[All Turns] When
/// your opponent plays a Digimon, if that Digimon is level 4 or higher, gain 1
/// memory"; mandatory, no other branch). Its only processing condition is
/// unmet when the level 3 Digimon is played, so "it triggered" is observable
/// only through what happens next: if P0 digivolves the Digimon to level 4
/// before it activates, it activates (it had triggered regardless of the
/// unmet condition); if P0 doesn't, it can't activate (§15-6-3). An engine
/// that gated the trigger on the processing condition would gain nothing in
/// the first board.
#[test]
fn processing_condition_checked_at_activation_not_trigger() {
    cite!(
        "15-8-3-9-1",
        "b202803ddf5ee44a7cbc37d5b044ad5342c963cfdb06d0f4c8cc4c87a96df213"
    );
    cite!(
        "15-8-3-9-3",
        "5b7a0031fb2110b5f52c02890fe2231471493a3cf527d6ee65f538904d4ca7ed"
    );
    // Control: no digivolve.
    let (mut r, tamer) = processing_reference_board("OBS_OPP_PLAY_LVL");
    let cp = r.event_checkpoint();
    r.execute_action(0, PASS)
        .expect("decline the optional digivolve");
    let hand1 = r.hand_size(1);
    accept(&mut r, 1);
    resolve_prompts(&mut r);
    assert!(
        is_suspended(&r, tamer),
        "the Tamer was suspended to activate"
    );
    assert_eq!(r.hand_size(1), hand1 + 1, "level 3 at processing: <Draw 1>");
    assert!(
        memory_sources(&r, cp).is_empty(),
        "{:?}",
        memory_sources(&r, cp)
    );

    // Digivolve to level 4 before P1's effect activates.
    let (mut r, tamer) = processing_reference_board("OBS_OPP_PLAY_LVL");
    let cp = r.event_checkpoint();
    pick_hand_card(&mut r, 0, "LV4_PLAIN");
    assert_eq!(
        field_top_ids(&r, 0),
        owned(&["LV4_PLAIN"]),
        "P0's Digimon is level 4 before P1's triggered effect activates"
    );
    let hand1 = r.hand_size(1);
    accept(&mut r, 1);
    resolve_prompts(&mut r);
    assert!(is_suspended(&r, tamer));
    assert_eq!(
        memory_sources(&r, cp),
        vec![(1, "OBS_OPP_PLAY_LVL".to_string())],
        "level 4 upon being referenced at processing: gain 1 memory"
    );
    assert_eq!(r.hand_size(1), hand1, "and no <Draw 1>");

    // Single branch: triggered at level 3, activates once it is level 4.
    let (mut r, _) = processing_reference_board("OBS_OPP_PLAY_GE4");
    let cp = r.event_checkpoint();
    pick_hand_card(&mut r, 0, "LV4_PLAIN");
    resolve_prompts(&mut r);
    assert_eq!(
        memory_sources(&r, cp),
        vec![(1, "OBS_OPP_PLAY_GE4".to_string())],
        "it triggered on the level 3 play although its processing condition was unmet, and \
         activated once the condition was met"
    );

    // Single branch, condition never met: it can't activate.
    let (mut r, _) = processing_reference_board("OBS_OPP_PLAY_GE4");
    let cp = r.event_checkpoint();
    r.execute_action(0, PASS)
        .expect("decline the optional digivolve");
    resolve_prompts(&mut r);
    assert!(
        memory_sources(&r, cp).is_empty(),
        "level 3 when it would activate: no memory: {:?}",
        memory_sources(&r, cp)
    );
}

// ─── §15-8-3-8 / §15-8-3-9-3 — attack observers after the attacker leaves ───

/// P0's Tamer `observer` and P0's `WA_RET_CHRON` (level 5, [Chronicle];
/// "[When Attacking] Return this Digimon to the hand"). P0's `WA_RET_CHRON`
/// attacks P1: its [When Attacking] and the Tamer's effect trigger together
/// and pend in one TriggerOrder for P0.
fn attacker_leaves_board(observer: &str) -> DebugRunner {
    let mut r = fixtures::builder(&[fixtures::ch15_08_cards()])
        .security(1, &[FILL; 3])
        .memory(3)
        .start();
    r.place_on_field(0, observer, Some(0));
    let attacker = r.place_on_field(0, "WA_RET_CHRON", Some(0));
    r.attack_player(attacker, 1, false);
    let choices = order_choices(&r, 0);
    assert_eq!(choices.len(), 2, "both triggered on the attack: {choices:?}");
    r
}

/// §15-8-3-8 (the state where the effect triggered is referenced for what
/// its trigger condition names). `OBS_ATK_CHRON`: "[Your Turn] When one of
/// your [Chronicle] trait Digimon attacks, gain 1 memory" -- "[Chronicle]
/// trait" qualifies the trigger condition. P0 activates `WA_RET_CHRON`'s
/// return first, so the attacking Digimon is gone when the Tamer's effect
/// activates; it was a [Chronicle] Digimon when the effect triggered, so the
/// effect still activates. (DCGO checks the qualifier only when stacking.)
#[test]
fn trigger_condition_qualifier_read_in_triggered_state_after_attacker_leaves() {
    cite!(
        "15-8-3-8",
        "f9fa5d72cc53f4d558f8cb42b2bac4ae3cd435c46ad3cb75edb2e0efc5ab8a4e"
    );
    let mut r = attacker_leaves_board("OBS_ATK_CHRON");
    let cp = r.event_checkpoint();
    choose(&mut r, "WA_RET_CHRON");
    resolve_prompts(&mut r);
    assert!(
        hand_ids(&r, 0).contains(&"WA_RET_CHRON".to_string()),
        "the attacker returned to the hand first"
    );
    assert_eq!(
        memory_sources(&r, cp),
        vec![(0, "OBS_ATK_CHRON".to_string())],
        "the effect activates: its trigger condition was met when it triggered"
    );
    assert_eq!(r.security_count(1), 3, "the attack ended without a check");
}

/// §15-8-3-9-3 (processing conditions are referenced when processing is
/// performed). `OBS_ATK_GE5`: "[Your Turn] When one of your Digimon attacks,
/// if that Digimon is level 5 or higher, gain 1 memory" -- "if that Digimon
/// is level 5 or higher" is a processing condition.
/// - Control: P0 activates the Tamer's effect first -- the level 5 attacker
///   is still there, so P0 gains 1 memory.
/// - P0 activates `WA_RET_CHRON`'s return first -- when the Tamer's effect is
///   processed there is no level 5 or higher attacking Digimon, so it does
///   not gain memory (the condition is a body `if:`, read live).
#[test]
fn trigger_processing_condition_read_at_processing_after_attacker_leaves() {
    cite!(
        "15-8-3-9-3",
        "5b7a0031fb2110b5f52c02890fe2231471493a3cf527d6ee65f538904d4ca7ed"
    );
    let mut r = attacker_leaves_board("OBS_ATK_GE5");
    let cp = r.event_checkpoint();
    choose(&mut r, "OBS_ATK_GE5");
    assert_eq!(
        memory_sources(&r, cp),
        vec![(0, "OBS_ATK_GE5".to_string())],
        "control: the level 5 attacker is still there"
    );

    let mut r = attacker_leaves_board("OBS_ATK_GE5");
    let cp = r.event_checkpoint();
    choose(&mut r, "WA_RET_CHRON");
    resolve_prompts(&mut r);
    assert!(hand_ids(&r, 0).contains(&"WA_RET_CHRON".to_string()));
    assert!(
        memory_sources(&r, cp).is_empty(),
        "the processing condition is unmet when processed: {:?}",
        memory_sources(&r, cp)
    );
}

/// §15-8-3-9-3 (processing conditions are referenced when processing is
/// performed). `WA_HAND7_DRAW`: "[When Attacking] <Draw 1>" and "[When
/// Attacking] If your hand has 7 or fewer cards, <Draw 1>" (the second
/// authored with its processing condition in `active_when`, the BT26-019
/// shape). P0 attacks with 7 cards in hand and activates the plain draw
/// first: 8 cards when the second effect is processed, so it does not draw.
#[test]
fn processing_condition_in_active_when_read_at_processing() {
    cite!(
        "15-8-3-9-3",
        "5b7a0031fb2110b5f52c02890fe2231471493a3cf527d6ee65f538904d4ca7ed"
    );
    let mut r = fixtures::builder(&[fixtures::ch15_08_cards()])
        .hand(0, &[FILL; 7])
        .deck(0, &[FILL; 4])
        .security(1, &[FILL; 3])
        .memory(3)
        .start();
    let attacker = r.place_on_field(0, "WA_HAND7_DRAW", Some(0));
    r.attack_player(attacker, 1, false);
    let choices = order_choices(&r, 0);
    assert_eq!(choices.len(), 2, "{choices:?}");
    choose(&mut r, "WA_HAND7_DRAW slot 0");
    resolve_prompts(&mut r);
    assert_eq!(
        r.hand_size(0),
        8,
        "7 -> 8 from the first draw; 8 cards when the second is processed: no draw"
    );
}

// ─── §15-8-3-1 / §15-8-3-4 vs §15-8-3-9-3 — trigger qualifier vs processing ─

/// P1's Tamer `observer` on the field; P0 (the turn player) holds `played`
/// and plays it. Returns the runner and an event checkpoint taken before the
/// play.
fn opponent_play_board(observer: &str, played: &str) -> (DebugRunner, usize) {
    let mut r = fixtures::builder(&[fixtures::ch15_08_cards()])
        .hand(0, &[played])
        .deck(0, &[FILL; 4])
        .deck(1, &[FILL; 4])
        .memory(5)
        .start();
    r.place_on_field(1, observer, Some(0));
    let cp = r.event_checkpoint();
    r.play(0, 0);
    (r, cp)
}

/// §15-8-3-1 "Trigger-type effects trigger as soon as their trigger
/// conditions are met." §15-8-3-4: an effect whose trigger conditions are
/// not met when the event happens does not trigger.
///
/// `OBS_OPP_PLAY_LE4K`: "[All Turns] When your opponent plays a Digimon with
/// 4000 DP or less, gain 1 memory" -- "with 4000 DP or less" qualifies the
/// trigger condition (`trigger_if:`). P0 plays `PLAY_DP_DOWN` at 6000 DP: the
/// play does not meet P1's trigger condition, so P1's effect never triggers.
/// The turn player's `[On Play]` ("This Digimon gets -2000 DP for the turn")
/// activates first (§15-4-3-5-1) and makes it a 4000 DP Digimon before P1's
/// effect would have activated: the effect still does not activate.
/// - Control: P0 plays the 1000 DP `VANILLA_LV3` -- the trigger condition is
///   met and P1 gains 1 memory.
///
/// (DCGO decides the trigger qualifier, `CanUseCondition`, when the event
/// stacks the effect, `AutoProcessing.StackSkillInfos`.)
/// G-ENGINE-OBSERVER-TRIGGER-QUALIFIER-NOT-CHECKED-AT-TRIGGER.
#[test]
fn observer_whose_qualifier_was_false_at_the_trigger_never_activates() {
    cite!(
        "15-8-3-1",
        "f6da78dabd1b18295353a717881824e6ae0d9133240bedc83b290853047e1483"
    );
    cite!(
        "15-8-3-4",
        "71d0451f75557827933628c0bf96a1437a5c750f4cd432b91ad8ce246f9be375"
    );
    // Control: a 1000 DP Digimon is played -- the trigger condition is met.
    let (mut r, cp) = opponent_play_board("OBS_OPP_PLAY_LE4K", "VANILLA_LV3");
    resolve_prompts(&mut r);
    assert_eq!(field_top_ids(&r, 0), owned(&["VANILLA_LV3"]));
    assert_eq!(
        memory_sources(&r, cp),
        vec![(1, "OBS_OPP_PLAY_LE4K".to_string())],
        "control: a Digimon with 4000 DP or less was played"
    );

    // A 6000 DP Digimon is played, then lowered to 4000 DP by its [On Play].
    let (mut r, cp) = opponent_play_board("OBS_OPP_PLAY_LE4K", "PLAY_DP_DOWN");
    resolve_prompts(&mut r);
    let played = r.perm_handle(0, 0);
    assert_eq!(field_top_ids(&r, 0), owned(&["PLAY_DP_DOWN"]));
    assert_eq!(
        r.effective_dp(played),
        Some(4000),
        "the [On Play] resolved: the played Digimon now has 4000 DP"
    );
    assert!(
        memory_sources(&r, cp).is_empty(),
        "the play did not meet the trigger condition (6000 DP when played): the effect never \
         triggered, so it does not activate once the Digimon has 4000 DP: {:?}",
        memory_sources(&r, cp)
    );
    assert!(
        r.pending_selection().is_none(),
        "{:?}",
        r.pending_selection()
    );
}

/// §15-8-3-9-3 "If a reference is to be made in the processing conditions
/// for a trigger-type effect, the reference is made in the state when the
/// processing is being performed."
///
/// The processing-condition twin of
/// `observer_whose_qualifier_was_false_at_the_trigger_never_activates`:
/// `OBS_OPP_PLAY_IF_LE4K` ("[All Turns] When your opponent plays a Digimon,
/// if that Digimon has 4000 DP or less, gain 1 memory") triggers on the 6000
/// DP play (its trigger condition is only "your opponent plays a Digimon"),
/// and its "if" is read when it activates, after the turn player's `[On
/// Play]` made the Digimon 4000 DP: it activates. The same board decides the
/// opposite way only because the DP reading is a processing condition here
/// and a trigger qualifier there.
///
/// The §15-8-3-9-1 single-branch shape (`OBS_OPP_PLAY_GE4`, its "if that
/// Digimon is level 4 or higher" authored in `condition:`) still triggers on
/// the level 3 play and activates once the Digimon is level 4.
#[test]
fn processing_condition_is_still_read_at_activation() {
    cite!(
        "15-8-3-9-3",
        "5b7a0031fb2110b5f52c02890fe2231471493a3cf527d6ee65f538904d4ca7ed"
    );
    let (mut r, cp) = opponent_play_board("OBS_OPP_PLAY_IF_LE4K", "PLAY_DP_DOWN");
    resolve_prompts(&mut r);
    let played = r.perm_handle(0, 0);
    assert_eq!(r.effective_dp(played), Some(4000), "the [On Play] resolved");
    assert_eq!(
        memory_sources(&r, cp),
        vec![(1, "OBS_OPP_PLAY_IF_LE4K".to_string())],
        "4000 DP when the processing condition is referenced at activation: gain 1 memory"
    );

    // Control: a 5000 DP Digimon with no [On Play] -- the condition is unmet
    // at activation.
    let (mut r, cp) = opponent_play_board("OBS_OPP_PLAY_IF_LE4K", "LV4_PLAIN");
    resolve_prompts(&mut r);
    assert_eq!(field_top_ids(&r, 0), owned(&["LV4_PLAIN"]));
    assert!(
        memory_sources(&r, cp).is_empty(),
        "more than 4000 DP at activation: no memory: {:?}",
        memory_sources(&r, cp)
    );

    // The GE4 shape: triggered at level 3, activates once it is level 4.
    let (mut r, _) = processing_reference_board("OBS_OPP_PLAY_GE4");
    let cp = r.event_checkpoint();
    pick_hand_card(&mut r, 0, "LV4_PLAIN");
    resolve_prompts(&mut r);
    assert_eq!(
        memory_sources(&r, cp),
        vec![(1, "OBS_OPP_PLAY_GE4".to_string())],
        "its processing condition, unmet at the trigger, is read when it activates"
    );
}

/// P0's Tamer `OBS_ATK_GE10K` ("[Your Turn] When one of your Digimon with
/// 10000 DP or more attacks, gain 1 memory") and P0's 7000 DP `LV5_PLAIN`;
/// P0 holds `PUMP_OPT` ("[Main] 1 of your Digimon gets +3000 DP for the
/// turn"). With `pump`, P0 uses it on `LV5_PLAIN` first. Then `LV5_PLAIN`
/// attacks P1. Returns the runner and an event checkpoint taken just before
/// the attack.
fn pumped_attacker_board(pump: bool) -> (DebugRunner, usize) {
    let mut r = fixtures::builder(&[fixtures::ch15_08_cards()])
        .hand(0, &["PUMP_OPT"])
        .deck(0, &[FILL; 4])
        .security(1, &[FILL; 3])
        .memory(5)
        .start();
    r.place_on_field(0, "OBS_ATK_GE10K", Some(0));
    let attacker = r.place_on_field(0, "LV5_PLAIN", Some(0));
    if pump {
        use_option(&mut r, 0, "PUMP_OPT");
        if r.pending_selection().is_some() {
            pick_field(&mut r, attacker.index as u16);
        }
        resolve_prompts(&mut r);
    }
    assert_eq!(
        r.effective_dp(attacker),
        Some(if pump { 10000 } else { 7000 }),
        "the attacker's DP before it attacks"
    );
    let cp = r.event_checkpoint();
    r.attack_player(attacker, 1, false);
    resolve_prompts(&mut r);
    (r, cp)
}

/// §15-8-3-1 "Trigger-type effects trigger as soon as their trigger
/// conditions are met." The trigger condition is checked against the game
/// state when the event happens; a referent still on the field is read live,
/// and a DP modifier is part of that state (only a referent that has left is
/// read through its last-known information, §15-8-3-8).
///
/// `LV5_PLAIN` prints 7000 DP. Pumped to 10000 DP by `PUMP_OPT` before it
/// attacks, it meets "one of your Digimon with 10000 DP or more attacks" (the
/// `trigger_if:` qualifier) and the Tamer's effect activates. Control: the
/// unpumped 7000 DP attack does not meet it.
#[test]
fn trigger_qualifier_reads_live_state_of_a_present_referent() {
    cite!(
        "15-8-3-1",
        "f6da78dabd1b18295353a717881824e6ae0d9133240bedc83b290853047e1483"
    );
    let (r, cp) = pumped_attacker_board(true);
    assert_eq!(
        memory_sources(&r, cp),
        vec![(0, "OBS_ATK_GE10K".to_string())],
        "a 10000 DP attacker (10000 only through a DP modifier) meets the trigger condition"
    );

    let (r, cp) = pumped_attacker_board(false);
    assert!(
        memory_sources(&r, cp).is_empty(),
        "control: a 7000 DP attacker does not meet it: {:?}",
        memory_sources(&r, cp)
    );
}

// ─── §15-8-3-1 / §15-8-3-8 — "wins a battle" after the winner leaves ───────

/// P0's `winner` and P0's Tamer `OBS_WIN_TS` ("[All Turns] When any of your
/// [TS] trait Digimon win a battle, gain 1 memory"); P1's `OD_DEL_ALL_OPP`
/// ("[On Deletion] Delete all of your opponent's Digimon", 2000 DP). On P1's
/// turn, P1's `OD_DEL_ALL_OPP` attacks P0's `winner` (7000 DP) and loses; its
/// [On Deletion] -- the turn player's effect, so it activates before P0's
/// observer whether or not the two pend together (§15-4-3-5-1) -- deletes
/// the winner before the Tamer's effect activates.
fn winner_deleted_by_loser_board(winner: &str) -> (DebugRunner, usize) {
    let mut r = fixtures::builder(&[fixtures::ch15_08_cards()])
        .security(0, &[FILL; 3])
        .memory(3)
        .start();
    let defender = r.place_on_field(0, winner, Some(0));
    r.place_on_field(0, "OBS_WIN_TS", Some(0));
    let attacker = r.place_on_field(1, "OD_DEL_ALL_OPP", Some(0));
    r.game.turn_player_idx = 1;
    let cp = r.event_checkpoint();
    r.attack_digimon(attacker, defender, false);
    resolve_prompts(&mut r);
    assert!(
        trash_ids(&r, 1).contains(&"OD_DEL_ALL_OPP".to_string()),
        "the attacker lost the battle and was deleted"
    );
    assert!(
        trash_ids(&r, 0).contains(&winner.to_string()),
        "the loser's [On Deletion] deleted the winner"
    );
    (r, cp)
}

/// §15-8-3-1 (a trigger-type effect triggers as soon as its trigger
/// conditions are met, then activates) and §15-8-3-8 (what the trigger
/// condition names is referenced in the state where the effect triggered).
/// "[TS] trait" qualifies the trigger condition "any of your [TS] trait
/// Digimon win a battle". The [TS] `TS_LV5` won the battle, then left the
/// field to the loser's [On Deletion] before `OBS_WIN_TS` activated: it was a
/// [TS] Digimon that won, so the effect still activates (its last-known
/// information, §15-4-4-5's departed-referent half). Control: the same board
/// with a winner without [TS] (`LV5_PLAIN`) -- the last-known trait is read,
/// not assumed. (DCGO: `CanTriggerWhenWinBattle` reads the copy of each
/// winner kept in the battle hashtable's `WinnerPermanents`.)
#[test]
fn battle_winner_qualifier_reads_last_known_information_after_the_winner_leaves() {
    cite!(
        "15-8-3-1",
        "f6da78dabd1b18295353a717881824e6ae0d9133240bedc83b290853047e1483"
    );
    cite!(
        "15-8-3-8",
        "f9fa5d72cc53f4d558f8cb42b2bac4ae3cd435c46ad3cb75edb2e0efc5ab8a4e"
    );
    let (r, cp) = winner_deleted_by_loser_board("TS_LV5");
    assert_eq!(
        memory_sources(&r, cp),
        vec![(0, "OBS_WIN_TS".to_string())],
        "a [TS] Digimon won the battle: the effect activates after the winner left"
    );

    let (r, cp) = winner_deleted_by_loser_board("LV5_PLAIN");
    assert!(
        memory_sources(&r, cp).is_empty(),
        "control: the winner had no [TS] trait: {:?}",
        memory_sources(&r, cp)
    );
}

/// §15-8-3-8. P0's battle area holds, in order, `VANILLA_LV3`, the [TS]
/// `TS_LV5`, the Tamer `OBS_WIN_TS` ("[All Turns] When any of your [TS] trait
/// Digimon win a battle, gain 1 memory") and the Tamer `OBS_WIN_RET_LV3`
/// ("[All Turns] When any of your Digimon win a battle, return all of your
/// level 3 Digimon to the hand"). `TS_LV5` attacks P1's `VANILLA_LV3` and
/// wins: both Tamers' effects trigger. P0 activates the return first, so the
/// level 3 Digimon in front of the winner leaves and the winner moves up.
/// The winner is the same [TS] Digimon and is still on the field, so
/// `OBS_WIN_TS` activates; the Tamer that moved into the winner's former
/// position is not the winner.
#[test]
fn battle_winner_qualifier_does_not_read_the_digimon_that_slid_into_the_slot() {
    cite!(
        "15-8-3-8",
        "f9fa5d72cc53f4d558f8cb42b2bac4ae3cd435c46ad3cb75edb2e0efc5ab8a4e"
    );
    let mut r = fixtures::builder(&[fixtures::ch15_08_cards()])
        .security(1, &[FILL; 3])
        .memory(3)
        .start();
    r.place_on_field(0, "VANILLA_LV3", Some(0));
    let winner = r.place_on_field(0, "TS_LV5", Some(0));
    r.place_on_field(0, "OBS_WIN_TS", Some(0));
    r.place_on_field(0, "OBS_WIN_RET_LV3", Some(0));
    let loser = r.place_on_field(1, "VANILLA_LV3", Some(0));
    let cp = r.event_checkpoint();
    r.attack_digimon(winner, loser, false);
    let choices = order_choices(&r, 0);
    assert_eq!(choices.len(), 2, "both triggered on the battle win: {choices:?}");
    choose(&mut r, "OBS_WIN_RET_LV3");
    resolve_prompts(&mut r);
    assert!(
        hand_ids(&r, 0).contains(&"VANILLA_LV3".to_string()),
        "the level 3 Digimon in front of the winner returned to the hand first"
    );
    assert_eq!(
        field_top_ids(&r, 0),
        owned(&["TS_LV5", "OBS_WIN_TS", "OBS_WIN_RET_LV3"]),
        "the winner moved up one position"
    );
    assert_eq!(
        memory_sources(&r, cp),
        vec![(0, "OBS_WIN_TS".to_string())],
        "the [TS] winner is still on the field: the effect activates"
    );
}

/// §15-8-3-8. P0's battle area holds, in order, `VANILLA_LV3`, the [TS]
/// `TS_LV5`, the Tamer `OBS_WIN_TS` ("[All Turns] When any of your [TS] trait
/// Digimon win a battle, gain 1 memory") and the Tamer `OBS_WIN_DNA` ("[All
/// Turns] When any of your Digimon win a battle, 1 of your [TS] trait Digimon
/// and 1 of your level 3 Digimon DNA digivolve into a level 6 Digimon card in
/// your hand without paying the cost, ignoring DNA digivolution
/// requirements"); P0's hand holds `DNA_LV6` (no traits). `TS_LV5` attacks
/// P1's `VANILLA_LV3` and wins: both Tamers' effects trigger. P0 activates
/// the DNA digivolution first: the [TS] winner becomes part of a new Digimon
/// with no [TS] trait (§8-2-2-1), so the Digimon that won is no longer on the
/// field. It was a [TS] Digimon when it won, so `OBS_WIN_TS` activates (its
/// last-known information), and the new Digimon is not read in its place.
#[test]
fn battle_winner_qualifier_reads_the_material_after_the_winner_dna_digivolves() {
    cite!(
        "15-8-3-8",
        "f9fa5d72cc53f4d558f8cb42b2bac4ae3cd435c46ad3cb75edb2e0efc5ab8a4e"
    );
    let mut r = fixtures::builder(&[fixtures::ch15_08_cards()])
        .hand(0, &["DNA_LV6"])
        .security(1, &[FILL; 3])
        .memory(3)
        .start();
    r.place_on_field(0, "VANILLA_LV3", Some(0));
    let winner = r.place_on_field(0, "TS_LV5", Some(0));
    r.place_on_field(0, "OBS_WIN_TS", Some(0));
    r.place_on_field(0, "OBS_WIN_DNA", Some(0));
    let loser = r.place_on_field(1, "VANILLA_LV3", Some(0));
    let cp = r.event_checkpoint();
    r.attack_digimon(winner, loser, false);
    let choices = order_choices(&r, 0);
    assert_eq!(choices.len(), 2, "both triggered on the battle win: {choices:?}");
    choose(&mut r, "OBS_WIN_DNA");
    resolve_prompts(&mut r);
    let field = field_top_ids(&r, 0);
    assert!(
        field.contains(&"DNA_LV6".to_string()) && !field.contains(&"TS_LV5".to_string()),
        "the winner DNA digivolved into a new Digimon first: {field:?}"
    );
    assert_eq!(
        memory_sources(&r, cp),
        vec![(0, "OBS_WIN_TS".to_string())],
        "a [TS] Digimon won the battle: the effect activates after it became a new Digimon"
    );
}

/// §15-8-3-1 "A trigger-type effect will always trigger as soon as its
/// trigger conditions are met, then the effect will activate." (and
/// §15-8-3-4: it won't trigger when they aren't met) and §15-4-4-5 (a pending
/// effect whose trigger conditions are no longer met can't activate -- the
/// activation-time re-check is not where a trigger is decided).
///
/// P0's `LV5_PLAIN` (no [TS] trait) attacks P1's `VANILLA_LV3` and wins. P0's
/// Tamer `OBS_WIN_DIGI_TS` ("[All Turns] When any of your Digimon win a
/// battle, 1 of your Digimon may digivolve into a [TS] trait Digimon card in
/// your hand without paying the cost") triggers; P0's Tamer `OBS_WIN_TS`
/// ("[All Turns] When any of your [TS] trait Digimon win a battle, gain 1
/// memory") does not: the Digimon that won had no [TS] trait. Its digivolution
/// into the [TS] `TS_LV6` after the battle makes the winner a [TS] Digimon,
/// but the battle was not won by a [TS] Digimon, so `OBS_WIN_TS` never
/// triggered and never activates. (DCGO: `CanTriggerWhenWinBattle` is
/// evaluated when the battle stacks its `OnEndBattle` effects; the converse,
/// a [TS] winner that leaves before activation, is
/// `battle_winner_qualifier_reads_last_known_information_after_the_winner_leaves`.)
/// G-ENGINE-BATTLE-RESULT-TRIGGER-CONDITION-NOT-CHECKED-AT-TRIGGER.
#[test]
fn battle_result_trigger_condition_false_at_the_battle_never_activates() {
    cite!(
        "15-8-3-1",
        "f6da78dabd1b18295353a717881824e6ae0d9133240bedc83b290853047e1483"
    );
    cite!(
        "15-8-3-4",
        "71d0451f75557827933628c0bf96a1437a5c750f4cd432b91ad8ce246f9be375"
    );
    cite!(
        "15-4-4-5",
        "71bf5b7f1273d0267e98f23107451b614fc0644a66410a19605e28c4c4274329"
    );
    let mut r = fixtures::builder(&[fixtures::ch15_08_cards()])
        .hand(0, &["TS_LV6"])
        .security(1, &[FILL; 3])
        .memory(3)
        .start();
    let winner = r.place_on_field(0, "LV5_PLAIN", Some(0));
    r.place_on_field(0, "OBS_WIN_TS", Some(0));
    r.place_on_field(0, "OBS_WIN_DIGI_TS", Some(0));
    let loser = r.place_on_field(1, "VANILLA_LV3", Some(0));
    let cp = r.event_checkpoint();
    r.attack_digimon(winner, loser, false);
    // `OBS_WIN_DIGI_TS`: its only Digimon is the winner; it digivolves.
    step_prompt(&mut r);
    pick_hand_card(&mut r, 0, "TS_LV6");
    resolve_prompts(&mut r);
    assert!(
        trash_ids(&r, 1).contains(&"VANILLA_LV3".to_string()),
        "P0's Digimon won the battle"
    );
    assert_eq!(
        field_top_ids(&r, 0),
        owned(&["TS_LV6", "OBS_WIN_TS", "OBS_WIN_DIGI_TS"]),
        "the winner digivolved into a [TS] Digimon after the battle"
    );
    assert!(
        memory_sources(&r, cp).is_empty(),
        "no [TS] Digimon won the battle: the effect never triggered: {:?}",
        memory_sources(&r, cp)
    );
}

// ─── §4-7-7 — a Digimon's traits are its top card's ─────────────────────────

/// P0's stack `stack` (bottom first, top card last), P0's Tamer `OBS_WIN_TS`
/// ("[All Turns] When any of your [TS] trait Digimon win a battle, gain 1
/// memory"), P0's other cards `extra` and P1's `VANILLA_LV3` (1000 DP). On
/// P0's turn the stack attacks P1's Digimon and wins. Returns the memory
/// gains since the attack.
fn stack_wins_battle(stack: &[&str], extra: &[&str]) -> Vec<(PlayerId, String)> {
    let mut r = fixtures::builder(&[fixtures::ch15_08_cards()])
        .security(1, &[FILL; 3])
        .memory(3)
        .start();
    let winner = r.place_stack(0, stack);
    r.place_on_field(0, "OBS_WIN_TS", Some(0));
    for id in extra {
        r.place_on_field(0, id, Some(0));
    }
    let loser = r.place_on_field(1, "VANILLA_LV3", Some(0));
    let cp = r.event_checkpoint();
    r.attack_digimon(winner, loser, false);
    resolve_prompts(&mut r);
    assert!(
        trash_ids(&r, 1).contains(&"VANILLA_LV3".to_string()),
        "P0's Digimon won the battle"
    );
    memory_sources(&r, cp)
}

/// §4-7-7 "The bottom cards of a stack are treated as part of the information
/// on the top card. They aren't considered to be placed on the field." The
/// bottom cards are part of the top card's information -- they contribute
/// their inherited effects (§4-7-4) -- but the Digimon IS its top card: its
/// traits are the top card's (with its `(Rule) Trait` grants, §2-3-4-2).
/// The manual has no sentence that says "a Digimon's traits are its top
/// card's"; official Q&A Q3225 (EX1-038 Stingmon) is the ruling: trashing the
/// top card turns the Digimon into one "without [Free] in its traits" while
/// the [Free] EX1-038 is still one of its digivolution cards. DCGO reads
/// `permanent.TopCard.HasTSTraits` (`BT25_020.cs`, `CanTriggerWhenWinBattle`).
///
/// The [TS] `TS_LV5` is the digivolution card of a `LV5_PLAIN` (no traits)
/// that wins a battle: no [TS] Digimon won, so `OBS_WIN_TS` does not trigger.
/// Control: the same two cards with `TS_LV5` on top -- a [TS] Digimon won.
/// Second control: a `LV5_PLAIN` alone under P0's `AURA_TS_TAMER` ("[Your
/// Turn] All of your Digimon gain the [TS] trait") -- the top card's traits
/// with the trait-changing effect applied (DCGO `CardSource.CardTraits`
/// applies `IChangeTraitsEffect`).
#[test]
fn battle_winner_trait_is_the_top_cards_trait() {
    cite!(
        "4-7-7",
        "d4517eedd82a9caf085a83c46d1898d22958f368614580d07fac9600f57fbeb4"
    );
    let gains = stack_wins_battle(&["TS_LV5", "LV5_PLAIN"], &[]);
    assert!(
        gains.is_empty(),
        "the winner's top card has no [TS] trait; its [TS] digivolution card does not make it a [TS] Digimon: {gains:?}"
    );

    let gains = stack_wins_battle(&["LV5_PLAIN", "TS_LV5"], &[]);
    assert_eq!(
        gains,
        vec![(0, "OBS_WIN_TS".to_string())],
        "control: the winner's top card is [TS]"
    );

    let gains = stack_wins_battle(&["LV5_PLAIN"], &["AURA_TS_TAMER"]);
    assert_eq!(
        gains,
        vec![(0, "OBS_WIN_TS".to_string())],
        "control: an effect gives the winner the [TS] trait"
    );
}

/// P0's stack `INH_TRASHED_FROM_TS` (inherited: "When effects trash this card
/// from a [TS] trait Digimon's digivolution cards, gain 1 memory") under
/// `middle` under `top`; P0 uses `MAIN_TRASH_BOTTOM_SRC` ("[Main] Trash the
/// bottom digivolution card of 1 of your Digimon") on it. Returns the memory
/// gains from the inherited effect.
fn trash_from_stack(middle: &str, top: &str) -> Vec<(PlayerId, String)> {
    let mut r = fixtures::builder(&[fixtures::ch15_08_cards()])
        .hand(0, &["MAIN_TRASH_BOTTOM_SRC"])
        .deck(0, &[FILL; 4])
        .memory(3)
        .start();
    r.place_stack(0, &["INH_TRASHED_FROM_TS", middle, top]);
    let cp = r.event_checkpoint();
    use_option(&mut r, 0, "MAIN_TRASH_BOTTOM_SRC");
    resolve_prompts(&mut r);
    assert!(
        trash_ids(&r, 0).contains(&"INH_TRASHED_FROM_TS".to_string()),
        "the bottom digivolution card was trashed"
    );
    memory_sources(&r, cp)
        .into_iter()
        .filter(|(_, id)| id == "INH_TRASHED_FROM_TS")
        .collect()
}

/// §4-7-7 (see `battle_winner_trait_is_the_top_cards_trait`): the shared
/// permanent trait reader, not only the battle winner's. "a [TS] trait
/// Digimon's digivolution cards" names the Digimon the card is trashed from;
/// a [TS] card that is another of its digivolution cards does not make it a
/// [TS] Digimon (official Q&A Q3225; DCGO `EX8_047.cs` reads
/// `trashedPermanent.TopCard.ContainsTraits`). Control: the [TS] card on top.
#[test]
fn permanent_trait_is_the_top_cards_trait() {
    cite!(
        "4-7-7",
        "d4517eedd82a9caf085a83c46d1898d22958f368614580d07fac9600f57fbeb4"
    );
    let gains = trash_from_stack("TS_LV5", "LV5_PLAIN");
    assert!(
        gains.is_empty(),
        "the Digimon's top card has no [TS] trait: {gains:?}"
    );

    let gains = trash_from_stack("LV5_PLAIN", "TS_LV5");
    assert_eq!(
        gains,
        vec![(0, "INH_TRASHED_FROM_TS".to_string())],
        "control: the Digimon's top card is [TS]"
    );
}

// ─── §15-8-3-9-4 ────────────────────────────────────────────────────────────

/// A board for the §15-8-3-9-4 test: P0 holds `MAIN_DEL1` ("[Main] Delete 1
/// of your opponent's Digimon") and has a red Digimon to use it; P1 has the
/// stack `stack` (top card last).
fn removal_board(stack: &[&str]) -> DebugRunner {
    let mut r = fixtures::builder(&[fixtures::ch15_08_cards()])
        .hand(0, &["MAIN_DEL1"])
        .deck(0, &[FILL; 4])
        .deck(1, &[FILL; 4])
        .memory(3)
        .start();
    r.place_on_field(0, FILL, Some(0));
    r.place_stack(1, stack);
    r
}

/// §15-8-3-9-4 "If a reference is to be made in the processing conditions for
/// a trigger-type effect that will trigger when a card is removed from an
/// area, the reference is made in the state when the effect triggered.
/// (Example: If an effect reads "[On Deletion] When this Digimon has
/// digivolution cards, you may play this card from your trash without paying
/// the cost," it references whether or not that Digimon has digivolution
/// cards upon deletion.)"
///
/// The rule's own example (`OD_REPLAY_SRC`). Deleted with a digivolution card
/// under it, it may play itself from the trash even though, in the trash, it
/// no longer has any; deleted alone, it can't.
#[test]
fn removal_trigger_processing_condition_uses_trigger_state() {
    cite!(
        "15-8-3-9-4",
        "abf363e841e63a5614583b8332981592eb0f1fbf312f9d4aac51377791f870df"
    );
    // With a digivolution card upon deletion.
    let mut r = removal_board(&[FILL, "OD_REPLAY_SRC"]);
    use_option(&mut r, 0, "MAIN_DEL1");
    step_prompt(&mut r); // the deletion target
    assert!(field_top_ids(&r, 1).is_empty(), "the stack was deleted");
    accept(&mut r, 1);
    resolve_prompts(&mut r);
    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["OD_REPLAY_SRC"]),
        "it had digivolution cards upon deletion: it plays itself from the trash"
    );
    assert_eq!(trash_ids(&r, 1), owned(&[FILL]));

    // Without digivolution cards upon deletion.
    let mut r = removal_board(&["OD_REPLAY_SRC"]);
    use_option(&mut r, 0, "MAIN_DEL1");
    step_prompt(&mut r);
    assert!(
        r.pending_selection().is_none(),
        "no digivolution cards upon deletion: the effect can't activate: {:?}",
        r.pending_selection()
    );
    assert!(field_top_ids(&r, 1).is_empty());
    assert_eq!(trash_ids(&r, 1), owned(&["OD_REPLAY_SRC"]));
}

// ─── §15-8-4-4-1 ────────────────────────────────────────────────────────────

/// §15-8-4-4-1 "If an activation-type effect has optional processing
/// conditions, a player can only declare activation while the optional
/// processing conditions can be performed. In addition, once declared, the
/// optional processing conditions must be performed. (Example: Activation of
/// <Training> can be declared if suspending is possible. Once declared,
/// suspending must be performed.)"
///
/// The rule's own example (`TRAIN`, printed `<Training>`). While it is
/// suspended, the action mask does not offer its `[Main]` activation; while it
/// is unsuspended, it does, and declaring it suspends the Digimon (no prompt
/// lets the player skip the suspension) and places the top deck card face down
/// as its bottom digivolution card.
#[test]
fn activation_type_declarable_only_when_optional_condition_payable() {
    cite!(
        "15-8-4-4-1",
        "0e89941fdf078b6f25cc2ef1a3b55a9599c8d919038ff4109430d7112a2fabdd"
    );
    // `TRAIN`'s printed face, as production loads it: the DSL's `grant_keyword:
    // Training` clause is the visible declaration only (by design --
    // `digimon-dsl/src/validator.rs` KNOWN keyword list, "Training" note);
    // the runtime behaviour rides on the printed-keyword parse of the text.
    let mut printed = make_test_card("TRAIN", "Drillmon");
    printed.level = Some(3);
    printed.dp = Some(3000);
    printed.play_cost = 3;
    printed.effect_text = concat!(
        "＜Training＞ (In the main phase, by suspending this Digimon, place your top deck ",
        "card face down as this Digimon's bottom digivolution card.)"
    )
    .to_string();
    printed.keywords = parse_printed_keywords(&printed.effect_text, "", "");
    assert!(
        printed.keywords.contains(&Keyword::Training),
        "the printed text parses to <Training>: {:?}",
        printed.keywords
    );
    let mut r = fixtures::builder(&[fixtures::ch15_08_cards()])
        .add_card(printed)
        .deck(0, &[FILL; 4])
        .memory(3)
        .start();
    let train = r.place_on_field(0, "TRAIN", Some(0));
    if r.current_phase() != GamePhase::Main {
        r.game.enter_main_phase();
    }
    let action = field_main_action(train);

    set_suspended(&mut r, train, true);
    assert_eq!(
        build_action_mask(&r.game, 0)[action as usize],
        0.0,
        "suspending is impossible: <Training> can't be declared"
    );

    set_suspended(&mut r, train, false);
    assert_eq!(
        build_action_mask(&r.game, 0)[action as usize],
        1.0,
        "suspending is possible: <Training> can be declared"
    );
    let deck0 = r.deck_size(0);
    r.game.decode_action(action, 0);

    assert!(
        r.pending_selection().is_none(),
        "once declared, nothing lets the player skip the suspension: {:?}",
        r.pending_selection()
    );
    assert!(
        is_suspended(&r, train),
        "declared: suspending was performed"
    );
    let perm = &r.game.players[0].battle_area[train.index as usize];
    assert_eq!(perm.card_sources.len(), 2, "a card was placed under it");
    assert!(perm.card_sources[0].face_down, "face down, at the bottom");
    assert_eq!(r.deck_size(0), deck0 - 1, "from the top of the deck");
}

// ─── §15-8-5-2 ──────────────────────────────────────────────────────────────

/// §15-8-5-2 "Activation of immediate-type effects will interrupt right
/// before the cause of such effects. (Example: An immediate-type effect that
/// triggers "when X would be deleted" will trigger immediately before the
/// deletion, and if that "when X would be deleted" effect prevents the card
/// from being removed from the battle area, it isn't deleted.)"
///
/// P0's `MAIN_DEL1` targets P1's stack `[INH_OD_MEM, SCAPEGOAT_LIKE]`
/// (`SCAPEGOAT_LIKE`: "[All Turns] When this Digimon would be deleted, by
/// deleting 1 other Digimon with [Sukamon] in its name, it isn't deleted").
/// The immediate-type effect is offered while the Digimon is still in the
/// battle area; paying with P1's `SUKAMON` prevents the deletion, so the
/// Digimon never reaches the trash and the inherited "[On Deletion] Gain 1
/// memory" never triggers.
#[test]
fn immediate_effect_interrupts_before_cause() {
    cite!(
        "15-8-5-2",
        "92c1ac4f55c3d717e5c055ef497a79ba30a0a7499eb571b5223bef9cd2b9914f"
    );
    let mut r = fixtures::builder(&[fixtures::ch15_08_cards()])
        .hand(0, &["MAIN_DEL1"])
        .deck(0, &[FILL; 4])
        .deck(1, &[FILL; 4])
        .memory(3)
        .start();
    r.place_on_field(0, FILL, Some(0));
    r.place_stack(1, &["INH_OD_MEM", "SCAPEGOAT_LIKE"]);
    r.place_on_field(1, "SUKAMON", Some(0));
    let cp = r.event_checkpoint();

    use_option(&mut r, 0, "MAIN_DEL1");
    pick_field(&mut r, 0); // SCAPEGOAT_LIKE

    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["SCAPEGOAT_LIKE", "SUKAMON"]),
        "the immediate-type effect is offered right before the deletion"
    );
    assert!(trash_ids(&r, 1).is_empty());
    accept(&mut r, 1);
    step_prompt(&mut r); // the only other [Sukamon]

    assert!(
        r.pending_selection().is_none(),
        "{:?}",
        r.pending_selection()
    );
    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["SCAPEGOAT_LIKE"]),
        "the deletion was prevented"
    );
    assert_eq!(
        trash_ids(&r, 1),
        owned(&["SUKAMON"]),
        "only the payment reached the trash"
    );
    assert!(
        memory_sources(&r, cp).is_empty(),
        "SCAPEGOAT_LIKE was never deleted, so its inherited [On Deletion] never triggered: {:?}",
        memory_sources(&r, cp)
    );
}

// ─── §15-8-5-3 ──────────────────────────────────────────────────────────────

/// §15-8-5-3 "Immediate-type effects will only trigger simultaneously with
/// other immediate-type effects."
///
/// P0's `MAIN_DEL_ALL` ("[Main] Delete all of your opponent's Digimon") hits
/// P1's `WOULD_DEL_MEM` ("[All Turns] When this Digimon would be deleted, you
/// may gain 1 memory") and `OD_MEM` ("[On Deletion] Gain 1 memory") in one
/// deletion. The immediate-type effect is offered on its own, before the
/// deletion (no ordering choice shares it with the `[On Deletion]`); the
/// trigger-type `[On Deletion]` triggers only after the deletion.
#[test]
fn immediate_effects_only_simultaneous_with_immediate_effects() {
    cite!(
        "15-8-5-3",
        "f848e083dad9a7afa537feb0cd86d95d2c53ecd40f071ef2aa2aac79948148e8"
    );
    let mut r = fixtures::builder(&[fixtures::ch15_08_cards()])
        .hand(0, &["MAIN_DEL_ALL"])
        .deck(0, &[FILL; 4])
        .deck(1, &[FILL; 4])
        .memory(3)
        .start();
    r.place_on_field(0, FILL, Some(0));
    r.place_on_field(1, "WOULD_DEL_MEM", Some(0));
    r.place_on_field(1, "OD_MEM", Some(0));
    let cp = r.event_checkpoint();

    use_option(&mut r, 0, "MAIN_DEL_ALL");

    let sel = r
        .pending_selection()
        .expect("the immediate-type effect is offered");
    assert_ne!(
        sel.kind,
        SelectionKind::TriggerOrder,
        "the immediate-type effect does not share an ordering choice with the [On Deletion]: \
         {sel:?}"
    );
    assert!(
        sel.effect_choices
            .iter()
            .flatten()
            .all(|c| !c.label.starts_with("OD_MEM")),
        "{sel:?}"
    );
    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["WOULD_DEL_MEM", "OD_MEM"]),
        "offered before the deletion"
    );
    accept(&mut r, 1);
    resolve_prompts(&mut r);

    assert!(field_top_ids(&r, 1).is_empty(), "both were deleted");
    assert_eq!(
        memory_sources(&r, cp),
        vec![(1, "WOULD_DEL_MEM".to_string()), (1, "OD_MEM".to_string())],
        "the immediate-type effect before the deletion, the [On Deletion] after it"
    );
}

// ─── §15-8-5-4 ──────────────────────────────────────────────────────────────

/// P1's board for the §15-8-5-4 test: `SCAPEGOAT_LIKE` (slot 0) and two
/// `SUKAMON` (slots 1, 2); P0 holds `hand` and has a red Digimon.
fn once_per_cause_board(hand: &[&str]) -> DebugRunner {
    let mut r = fixtures::builder(&[fixtures::ch15_08_cards()])
        .hand(0, hand)
        .deck(0, &[FILL; 4])
        .deck(1, &[FILL; 4])
        .memory(3)
        .start();
    r.place_on_field(0, FILL, Some(0));
    r.place_on_field(1, "SCAPEGOAT_LIKE", Some(0));
    r.place_on_field(1, "SUKAMON", Some(0));
    r.place_on_field(1, "SUKAMON", Some(0));
    r
}

/// Prevent one deletion of `SCAPEGOAT_LIKE` (slot 0): pick it as the target,
/// accept its immediate-type effect, pay with the [Sukamon] in `pay_slot`.
fn delete_scapegoat_and_prevent(r: &mut DebugRunner, pay_slot: u16) {
    pick_field(r, 0); // SCAPEGOAT_LIKE
    accept(r, 1);
    if r.pending_action_count() == 1 {
        step_prompt(r);
    } else {
        pick_field(r, pay_slot);
    }
}

/// §15-8-5-4 "Each immediate-type effect can be activated once until the
/// cause that first interrupted the immediate-type effect is resolved.
/// (Example: If "[Main] Delete 1 of your opponent's Digimon" causes "[All
/// Turns] When this Digimon would be deleted, by deleting 1 other Digimon with
/// [Sukamon] in its name, it isn't deleted" to activate, until the deletion is
/// resolved, the already activated "[All Turns] When this Digimon would be
/// deleted, by deleting 1 other Digimon with [Sukamon] in its name, it isn't
/// deleted" can't activate again.)"
///
/// Controller ruling (task 8 review, PDF governs): the cause is the
/// interrupting EVENT -- immediate-type effects interrupt "right before the
/// cause" (§15-8-5-1/-2), here "X would be deleted", and the example closes
/// the window "until the deletion is resolved". DCGO enforces neither reading
/// for BT11-041 (`ChainActivations` defaults to -1; `IsCutInEffectHasUsed` is a
/// stub).
/// - Two deletions from ONE `[Main]` (`MAIN_DEL2`: "Delete 1 of your
///   opponent's Digimon. Then, delete 1 of your opponent's Digimon."): the
///   first deletion of `SCAPEGOAT_LIKE` is resolved (prevented) before "Then,
///   delete 1" begins, so the second deletion is a new cause and the effect
///   activates again: both [Sukamon] are spent and it survives.
/// - Two separate `[Main]` uses (`MAIN_DEL1` twice): the same.
/// - ONE deletion event: `IMM_TWO_MAY` has two immediate-type effects on
///   "would be deleted" ("you may gain 1 memory", "you may <Draw 1>"), neither
///   of which prevents the deletion. Each activates once; after the first one
///   resolves it is not offered again within that event, and the deletion
///   then proceeds.
#[test]
fn immediate_effect_once_per_interrupting_cause() {
    cite!(
        "15-8-5-4",
        "7586ddc0ccc9332f3ced38f131201eceaf1856a8343ac14248891fafc18fb30b"
    );
    // One [Main], two deletions: each deletion is its own cause.
    let mut r = once_per_cause_board(&["MAIN_DEL2"]);
    use_option(&mut r, 0, "MAIN_DEL2");
    delete_scapegoat_and_prevent(&mut r, 1);
    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["SCAPEGOAT_LIKE", "SUKAMON"]),
        "the first deletion was prevented"
    );
    delete_scapegoat_and_prevent(&mut r, 1); // "Then, delete 1": a new deletion
    assert!(
        r.pending_selection().is_none(),
        "{:?}",
        r.pending_selection()
    );
    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["SCAPEGOAT_LIKE"]),
        "the second deletion is a new cause: the effect activated again and prevented it"
    );
    assert_eq!(trash_ids(&r, 1), owned(&["SUKAMON", "SUKAMON"]));

    // Two [Main] uses: the same.
    let mut r = once_per_cause_board(&["MAIN_DEL1", "MAIN_DEL1"]);
    use_option(&mut r, 0, "MAIN_DEL1");
    delete_scapegoat_and_prevent(&mut r, 1);
    use_option(&mut r, 0, "MAIN_DEL1");
    delete_scapegoat_and_prevent(&mut r, 1);
    assert!(
        r.pending_selection().is_none(),
        "{:?}",
        r.pending_selection()
    );
    assert_eq!(field_top_ids(&r, 1), owned(&["SCAPEGOAT_LIKE"]));
    assert_eq!(trash_ids(&r, 1), owned(&["SUKAMON", "SUKAMON"]));

    // One deletion event, two non-preventing immediate-type effects.
    let mut r = fixtures::builder(&[fixtures::ch15_08_cards()])
        .hand(0, &["MAIN_DEL1"])
        .deck(0, &[FILL; 4])
        .deck(1, &[FILL; 4])
        .memory(3)
        .start();
    r.place_on_field(0, FILL, Some(0));
    r.place_on_field(1, "IMM_TWO_MAY", Some(0));
    let hand1 = r.hand_size(1);
    let cp = r.event_checkpoint();
    use_option(&mut r, 0, "MAIN_DEL1");
    step_prompt(&mut r); // the deletion target
    let mut accepted = 0;
    for _ in 0..8 {
        let Some(sel) = r.pending_selection() else {
            break;
        };
        assert_eq!(sel.selecting_player, 1, "{sel:?}");
        if sel.is_optional {
            accepted += 1;
            accept(&mut r, 1);
        } else {
            // An ordering choice between the two: take the first.
            let action = sel.valid_action_ids[0];
            r.execute_action(1, action)
                .unwrap_or_else(|e| panic!("order pick rejected: {e:?}"));
        }
    }
    assert_eq!(
        accepted, 2,
        "each immediate-type effect is offered exactly once for the one deletion"
    );
    assert_eq!(
        memory_sources(&r, cp),
        vec![(1, "IMM_TWO_MAY".to_string())],
        "the memory effect activated once"
    );
    assert_eq!(r.hand_size(1), hand1 + 1, "the <Draw 1> activated once");
    assert_eq!(
        trash_ids(&r, 1),
        owned(&["IMM_TWO_MAY"]),
        "then the deletion proceeded"
    );
}

// ─── §15-8-5-5-2 ────────────────────────────────────────────────────────────

/// §15-8-5-5-2 "Even if an immediate-type effect with processing conditions
/// triggers when its processing conditions aren't met, it can be activated if
/// its processing conditions are later met."
///
/// P0's `MAIN_DEL1` targets P1's `IMM_COND`, which has two immediate-type
/// effects on "When this Digimon would be deleted": "if you have 4 or more
/// security cards, it isn't deleted" and "<Recovery +1 (Deck)>". P1 has 3
/// security cards, so the first one's processing condition isn't met when it
/// triggers; the recovery meets it, and the first one then activates: the
/// Digimon isn't deleted.
#[test]
fn immediate_effect_with_condition_met_later_can_activate() {
    cite!(
        "15-8-5-5-2",
        "c66e115e90704b92a7c4dd8a6ddeebe90ae321eec0ca2d90ebc51d0ee4d3eb96"
    );
    let mut r = fixtures::builder(&[fixtures::ch15_08_cards()])
        .hand(0, &["MAIN_DEL1"])
        .deck(0, &[FILL; 4])
        .deck(1, &[FILL; 4])
        .security(1, &[FILL; 3])
        .memory(3)
        .start();
    r.place_on_field(0, FILL, Some(0));
    r.place_on_field(1, "IMM_COND", Some(0));

    use_option(&mut r, 0, "MAIN_DEL1");
    step_prompt(&mut r); // the deletion target
    for _ in 0..8 {
        match r.pending_selection().map(|s| s.kind) {
            None => break,
            // If the engine offers an order, take the recovery (clause 1).
            Some(SelectionKind::TriggerOrder) => choose(&mut r, "IMM_COND slot 1"),
            Some(_) => step_prompt(&mut r),
        }
    }

    assert_eq!(r.security_count(1), 4, "the recovery resolved");
    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["IMM_COND"]),
        "its processing condition was met later in the window: it activated and the Digimon \
         wasn't deleted"
    );

    // Control: with an empty deck the recovery adds nothing, the processing
    // condition is never met, and the Digimon is deleted.
    let mut r = fixtures::builder(&[fixtures::ch15_08_cards()])
        .hand(0, &["MAIN_DEL1"])
        .deck(0, &[FILL; 4])
        .security(1, &[FILL; 3])
        .memory(3)
        .start();
    r.place_on_field(0, FILL, Some(0));
    r.place_on_field(1, "IMM_COND", Some(0));
    use_option(&mut r, 0, "MAIN_DEL1");
    step_prompt(&mut r);
    for _ in 0..8 {
        match r.pending_selection().map(|s| s.kind) {
            None => break,
            Some(SelectionKind::TriggerOrder) => choose(&mut r, "IMM_COND slot 1"),
            Some(_) => step_prompt(&mut r),
        }
    }
    assert_eq!(r.security_count(1), 3, "nothing to recover with");
    assert!(
        field_top_ids(&r, 1).is_empty(),
        "the processing condition was never met: the Digimon was deleted"
    );
}

// ─── §4-8-2 / §4-7-7 (a Digimon's name) ─────────────────────────────────────

/// §4-8-2 "When referencing digivolution card information, the information is
/// referenced on cards that are treated as digivolution cards."
///
/// The converse, read with §4-7-7 and the provisional controller ruling of
/// Task 31 / Task 35 (official Q&A Q3225, EX1-038: a Digimon whose top card
/// was trashed is "without ... [Imperialdramon] in its name" although such a
/// card may remain in the stack; DCGO reads `PermanentOfThisCard().TopCard`):
/// a Digimon's NAME is its top card's, and digivolution cards are read only
/// when the text references digivolution cards
/// (G-DSL-CARRIER-NAME-READS-DIGIVOLUTION-CARDS).
///
/// 1. `INH_GREY_NAME_DP` ("While this Digimon has [Greymon] in its name, it
///    gets +1000 DP") under a [Greymon] digivolution card and a non-[Greymon]
///    top card: no +1000. With the [Greymon] card on top: +1000.
/// 2. `GREY_DIGICARD_DP` (named [Greymon]; "While [Greymon] is in this
///    Digimon's digivolution cards, this Digimon gets +1000 DP"): its own
///    top-card name does not count; a [Greymon] digivolution card does.
#[test]
fn digimon_name_is_the_top_cards_name() {
    cite!(
        "4-8-2",
        "0f82c2964b229056dd15ce7131f53fd3b0582382f8e266c3504a06596dc0b8df"
    );
    cite!(
        "4-7-7",
        "d4517eedd82a9caf085a83c46d1898d22958f368614580d07fac9600f57fbeb4"
    );
    let dp = |stack: &[&str]| {
        let mut r = fixtures::builder(&[fixtures::ch15_08_cards()])
            .deck(0, &[FILL; 4])
            .deck(1, &[FILL; 4])
            .memory(3)
            .start();
        let h = r.place_stack(0, stack);
        r.game.tick_declarative_effects();
        r.effective_dp(h).expect("the Digimon has DP")
    };

    // 1. "this Digimon's name" — the top card's.
    assert_eq!(
        dp(&["INH_GREY_NAME_DP", "GREY_LV4", "LV5_PLAIN"]),
        7000,
        "a [Greymon] digivolution card under a [Fivemon] top card does not put [Greymon] in \
         this Digimon's name: no +1000"
    );
    assert_eq!(
        dp(&["INH_GREY_NAME_DP", "LV4_PLAIN", "GREY_LV4"]),
        6000,
        "control: a [Greymon] top card gives this Digimon [Greymon] in its name: +1000"
    );

    // 2. "in this Digimon's digivolution cards" — only the cards under it.
    assert_eq!(
        dp(&["LV4_PLAIN", "GREY_DIGICARD_DP"]),
        7000,
        "the [Greymon] top card is not a digivolution card: no +1000"
    );
    assert_eq!(
        dp(&["GREY_LV4", "GREY_DIGICARD_DP"]),
        8000,
        "a [Greymon] digivolution card satisfies the condition: +1000"
    );
}
