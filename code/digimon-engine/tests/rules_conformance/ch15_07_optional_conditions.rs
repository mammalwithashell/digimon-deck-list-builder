//! Chapter 15-7 "Optional Processing Conditions" (Comprehensive Rules Ver.
//! 4.3), with §15-1-2 (when the decision is made) and §15-15-7-4 (a mandatory
//! effect that activates another effect does not force that effect's optional
//! processing condition).
//!
//! Every test asserts the RULE, never the engine's current output. A test the
//! engine fails is `#[ignore]`d with the gap id that explains it (discover then
//! pin); see `docs/RUST_ENGINE_GAPS.md` and `qa/rules/cr_obligations_status.yaml`.
//!
//! Ver. 4.3 reversed §15-7-4: Ver. 3.6 let a player choose an optional
//! processing condition they could not execute; 4.3 says they can't (owner
//! ruling 2026-10-10 #6: never offer a trigger whose condition can't be paid).
//! 4.3 also updated §15-1-2 and rewrote §15-15-7-4.
//!
//! Fixture cards: `fixtures/common/*.yaml` + `fixtures/ch15_07/*.yaml`, one
//! synthetic card per file, authored to printed-style text (each clause's
//! `summary:` is that text). A "by X, Y" clause is `optional: true` with X as
//! its leading steps -- the DSL's only shape for an optional processing
//! condition. Payability of X is NOT hand-authored as a `condition:` /
//! `active_when:` gate (production cards such as BT22-041 / BT25-084 add one
//! per card): the rules under test are about the optional processing condition
//! itself, so the tests ask whether the engine derives them.

use crate::fixtures;
use crate::support::*;
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{GamePhase, PlaySource, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

fn ch15_07() -> DebugRunnerBuilder {
    fixtures::builder(&[fixtures::ch15_07_cards()])
}

/// Resolve the pending optional card pick (hand / trash) by taking its ONLY
/// non-PASS option, asserting it belongs to `player`.
fn pick_the_card(r: &mut DebugRunner, player: PlayerId) {
    let sel = r.pending_selection().expect("a card pick");
    assert_eq!(sel.selecting_player, player, "{sel:?}");
    let picks: Vec<u16> = sel
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    assert_eq!(
        picks.len(),
        1,
        "the scenario stages exactly one card: {sel:?}"
    );
    r.execute_action(player, picks[0])
        .unwrap_or_else(|e| panic!("card pick rejected: {e:?}"));
}

/// The pending prompt is an optional decision owned by `player`: an optional
/// prompt accepts PASS (decline) besides its `valid_action_ids`.
fn assert_optional_decision(r: &DebugRunner, player: PlayerId, what: &str) {
    let sel = r
        .pending_selection()
        .unwrap_or_else(|| panic!("{what}: expected an optional decision for P{player}"));
    assert_eq!(sel.selecting_player, player, "{what}: {sel:?}");
    assert!(sel.is_optional, "{what}: the decision is optional: {sel:?}");
}

// ─── §15-7-1 ────────────────────────────────────────────────────────────────

/// A board for the §15-7-1 test: P0's `LV5_PLAIN` (7000 DP) attacks and
/// deletes P1's suspended `OD_TRASH_HAND_MEM` (3000 DP); P1 holds one card.
fn on_deletion_trash_hand_board() -> DebugRunner {
    let mut r = ch15_07()
        .hand(1, &[FILL])
        .deck(0, &[FILL; 4])
        .deck(1, &[FILL; 4])
        .memory(3)
        .start();
    let attacker = r.place_on_field(0, "LV5_PLAIN", Some(0));
    let defender = r.place_on_field(1, "OD_TRASH_HAND_MEM", Some(0));
    set_suspended(&mut r, defender, true);
    r.attack_digimon(attacker, defender, false);
    r
}

/// §15-7-1 "If an effect has optional processing conditions, its text will
/// show the conditions for processing the effect. If the player chooses to
/// execute the processing for the conditions and is successful, the
/// processing after the conditions is performed. Optional processing
/// conditions include text such as "by X, Y." (Example: If an effect reads
/// "[On Deletion] By trashing 1 card in the hand, gain 1 memory," The "by
/// trashing 1 card in the hand" part is an optional processing condition.)"
///
/// The rule's own example (`OD_TRASH_HAND_MEM`), deleted in battle:
/// - P1 is asked whether to trash a card (an optional decision: PASS is
///   legal). Choosing to and succeeding performs the processing after the
///   condition -- "gain 1 memory" -- with no second prompt.
/// - Control: declining performs nothing after the condition.
#[test]
fn optional_condition_is_a_choice_then_mandatory_payload() {
    cite!(
        "15-7-1",
        "2052018b3b52f32d77bdeb2bff0ea218b7ddb0fd7a2690c9a70ad7884a1024f1"
    );
    // Yes.
    let mut r = on_deletion_trash_hand_board();
    assert!(field_top_ids(&r, 1).is_empty(), "deleted in battle");
    let cp = r.event_checkpoint();
    assert_optional_decision(&r, 1, "by trashing 1 card in the hand");
    pick_hand_card(&mut r, 1, FILL);
    assert!(
        r.pending_selection().is_none(),
        "the condition was executed: the memory gain follows with no second prompt: {:?}",
        r.pending_selection()
    );
    assert_eq!(r.hand_size(1), 0, "the card in the hand was trashed");
    assert_eq!(
        memory_sources(&r, cp),
        vec![(1, "OD_TRASH_HAND_MEM".to_string())],
        "the processing after the condition was performed"
    );

    // No.
    let mut r = on_deletion_trash_hand_board();
    let cp = r.event_checkpoint();
    assert_optional_decision(&r, 1, "by trashing 1 card in the hand");
    r.execute_action(1, PASS).expect("decline the condition");
    assert!(
        r.pending_selection().is_none(),
        "{:?}",
        r.pending_selection()
    );
    assert_eq!(hand_ids(&r, 1), owned(&[FILL]), "nothing was trashed");
    assert!(
        memory_sources(&r, cp).is_empty(),
        "declined: no memory: {:?}",
        memory_sources(&r, cp)
    );
}

// ─── §15-7-2 ────────────────────────────────────────────────────────────────

/// A board for the §15-7-2 test: P0's suspended `WEREGARU_LV5` (named
/// [WereGarurumon]) digivolves into `WD_TRASH2_UNSUSPEND_THEN`; P0 holds 2
/// other cards and has `GARU_LV4` ([Garurumon]) in the trash. Digivolving
/// draws 1, so P0 then holds 3 `FILL`.
fn trash2_unsuspend_board() -> (DebugRunner, PermanentHandle) {
    let mut r = ch15_07()
        .hand(0, &["WD_TRASH2_UNSUSPEND_THEN", FILL, FILL])
        .deck(0, &[FILL; 4])
        .deck(1, &[FILL; 4])
        .memory(5)
        .start();
    let base = r.place_on_field(0, "WEREGARU_LV5", Some(0));
    set_suspended(&mut r, base, true);
    r.inject_trash(0, "GARU_LV4");
    assert!(
        r.game
            .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByHand),
        "digivolve into WD_TRASH2_UNSUSPEND_THEN"
    );
    assert_eq!(
        field_top_ids(&r, 0),
        owned(&["WD_TRASH2_UNSUSPEND_THEN"]),
        "digivolved"
    );
    (r, base)
}

/// §15-7-2 "If the content of the optional processing conditions isn't
/// executed, the processing after the conditions can't be executed.
/// (Examples: If an effect reads "[When Digivolving] By trashing 2 cards in
/// your hand, this Digimon unsuspends. After, if [WereGarurumon] or [X
/// Antibody] is in this Digimon's digivolution cards, you may return 1 Digimon
/// card with [Garurumon] or [X Antibody] in its name from your trash to the
/// hand," unless the player trashes 2 cards in their hand, "this Digimon
/// unsuspends" and the content after "After" can't be executed.)"
///
/// The rule's own example (`WD_TRASH2_UNSUSPEND_THEN`, digivolved onto a
/// suspended [WereGarurumon] with a [Garurumon] in the trash):
/// - P0 declines the condition: the Digimon stays suspended and the "After"
///   return is never offered.
/// - Control: P0 trashes 2 cards: it unsuspends, and the "After" return of the
///   [Garurumon] is offered and performed.
#[test]
fn declining_condition_skips_everything_after_it() {
    cite!(
        "15-7-2",
        "b4c2e15738ef696e068c4b5861a502ad858a525699df4c730d60073609d7f467"
    );
    // Declined.
    let (mut r, base) = trash2_unsuspend_board();
    assert_eq!(hand_ids(&r, 0), owned(&[FILL, FILL, FILL]));
    assert_optional_decision(&r, 0, "by trashing 2 cards in your hand");
    r.execute_action(0, PASS).expect("decline the condition");
    assert!(
        r.pending_selection().is_none(),
        "declined: the \"After\" return is not offered: {:?}",
        r.pending_selection()
    );
    assert!(
        is_suspended(&r, base),
        "\"this Digimon unsuspends\" was not executed"
    );
    assert_eq!(
        hand_ids(&r, 0),
        owned(&[FILL, FILL, FILL]),
        "nothing trashed"
    );
    assert_eq!(
        trash_ids(&r, 0),
        owned(&["GARU_LV4"]),
        "the content after \"After\" was not executed"
    );

    // Control: the condition is executed.
    let (mut r, base) = trash2_unsuspend_board();
    assert_optional_decision(&r, 0, "by trashing 2 cards in your hand");
    pick_hand_card(&mut r, 0, FILL);
    pick_hand_card(&mut r, 0, FILL);
    assert!(!is_suspended(&r, base), "the Digimon unsuspended");
    assert_optional_decision(&r, 0, "the \"After\" return (\"you may\")");
    pick_the_card(&mut r, 0);
    assert!(
        r.pending_selection().is_none(),
        "{:?}",
        r.pending_selection()
    );
    assert_eq!(hand_ids(&r, 0), owned(&[FILL, "GARU_LV4"]));
    assert_eq!(trash_ids(&r, 0), owned(&[FILL, FILL]));
}

// ─── §15-7-3 ────────────────────────────────────────────────────────────────

/// A board for the §15-7-3 test: P0's `LV5_PLAIN` (7000 DP) attacks and
/// deletes P1's suspended `OD_RETURN_KIMERA_MACHINE_PLAY_MILLENNIUM` (5000 DP,
/// named [Millenniummon]); P1's trash holds `trash`.
fn compound_condition_board(trash: &[&str]) -> DebugRunner {
    let mut r = ch15_07()
        .deck(0, &[FILL; 4])
        .deck(1, &[FILL; 4])
        .memory(3)
        .start();
    for card in trash {
        r.inject_trash(1, card);
    }
    let attacker = r.place_on_field(0, "LV5_PLAIN", Some(0));
    let defender = r.place_on_field(1, "OD_RETURN_KIMERA_MACHINE_PLAY_MILLENNIUM", Some(0));
    set_suspended(&mut r, defender, true);
    r.attack_digimon(attacker, defender, false);
    assert!(field_top_ids(&r, 1).is_empty(), "deleted in battle");
    r
}

/// §15-7-3 "It isn't possible to only perform some of the optional processing
/// conditions. (Example: If an effect reads "[On Deletion] By returning 1
/// [Kimeramon] and 1 [Machinedramon] from your trash to the bottom of the
/// deck, you may play 1 [Millenniummon] from your trash without paying the
/// cost," a player can't just return 1 [Kimeramon] or 1 [Machinedramon] to
/// the bottom of their deck.)"
///
/// The rule's own example, authored with BT18-019's production idiom. With
/// only [Kimeramon] in the trash, the condition can only be performed in part,
/// which is impossible: however P1 answers, nothing is returned and
/// [Millenniummon] is not played. Control: with both in the trash, both are
/// returned and [Millenniummon] may be played.
///
/// The DSL has no construct for a compound optional processing condition: the
/// idiom authors "1 [Kimeramon] and 1 [Machinedramon]" as two independent
/// picks. The engine recognises the leading run of same-zone picks as the
/// condition and gates it all-or-nothing (one distinct trash card per pick;
/// `leading_cost_guard`, G-ENGINE-UNPAYABLE-OPTIONAL-CONDITION-OFFERED), so
/// with only [Kimeramon] nothing is offered. Compounds across different step
/// kinds still need the DSL `by:` block (`G-DSL-OPTIONAL-PROCESSING-CONDITION-BLOCK`).
///
/// DCGO (`BT18_019.cs`) activates the effect when EITHER name is in the trash
/// (`CanActivateCondition` ORs them) but its one multi-pick can't end without
/// both (`canEndSelectCondition`), so the player can only decline: no partial
/// return, though a decline-only prompt is shown. The PDF governs the prompt.
#[test]
fn partial_payment_of_compound_condition_is_impossible() {
    cite!(
        "15-7-3",
        "6fec437e69b9e804b0e88f485422acc9a753dbef3820933d7774817b35686d30"
    );
    // Control: both [Kimeramon] and [Machinedramon].
    let mut r = compound_condition_board(&["KIMERAMON", "MACHINEDRAMON"]);
    assert_optional_decision(&r, 1, "by returning [Kimeramon] and [Machinedramon]");
    pick_the_card(&mut r, 1); // Kimeramon
    pick_the_card(&mut r, 1); // Machinedramon
    pick_the_card(&mut r, 1); // you may play [Millenniummon]
    assert!(
        r.pending_selection().is_none(),
        "{:?}",
        r.pending_selection()
    );
    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["OD_RETURN_KIMERA_MACHINE_PLAY_MILLENNIUM"]),
        "[Millenniummon] was played from the trash"
    );
    let deck1 = card_ids(&r, &r.game.player(1).deck);
    let mut bottom2: Vec<String> = deck1.iter().take(2).cloned().collect();
    bottom2.sort();
    assert_eq!(
        bottom2,
        owned(&["KIMERAMON", "MACHINEDRAMON"]),
        "both returned to the bottom of the deck ({deck1:?})"
    );

    // Only [Kimeramon]: the condition can't be performed in part. Whatever
    // the engine offers, P1 takes every pick it can (the most a player could
    // pay); Kimeramon must still not leave the trash and [Millenniummon] must
    // not be played. (That no prompt appears at all is §15-7-4's consequence,
    // asserted in `unpayable_optional_condition_cannot_be_chosen`.)
    let mut r = compound_condition_board(&["KIMERAMON"]);
    let deck_before = card_ids(&r, &r.game.player(1).deck);
    for _ in 0..8 {
        let Some(sel) = r.pending_selection() else {
            break;
        };
        let player = sel.selecting_player;
        let action = sel
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .unwrap_or(PASS);
        r.execute_action(player, action)
            .unwrap_or_else(|e| panic!("pick rejected: {e:?}"));
    }
    assert_eq!(
        trash_ids(&r, 1),
        owned(&["KIMERAMON", "OD_RETURN_KIMERA_MACHINE_PLAY_MILLENNIUM"]),
        "[Kimeramon] alone can't be returned, and [Millenniummon] was not played"
    );
    assert!(
        field_top_ids(&r, 1).is_empty(),
        "[Millenniummon] was not played"
    );
    assert_eq!(card_ids(&r, &r.game.player(1).deck), deck_before);
}

// ─── §15-7-4 ────────────────────────────────────────────────────────────────

/// The action-mask bit of P0's `MAIN_TRASH_SEC_DRAW` [Main] in the main phase,
/// with `security` as P0's security stack.
fn main_trash_sec_draw_mask_bit(security: &[&str]) -> f32 {
    let mut r = ch15_07()
        .deck(0, &[FILL; 4])
        .security(0, security)
        .memory(3)
        .start();
    let main = r.place_on_field(0, "MAIN_TRASH_SEC_DRAW", Some(0));
    if r.current_phase() != GamePhase::Main {
        r.game.enter_main_phase();
    }
    build_action_mask(&r.game, 0)[field_main_action(main) as usize]
}

/// §15-7-4 (Ver. 4.3) "If executing an optional processing condition's
/// contents isn't possible, then the player can't choose to execute them.
/// (Example: If an effect reads "[When Attacking] By trashing your top
/// security card, 1 of your opponent's Digimon gets -5000 DP for the turn,"
/// and the player doesn't have even 1 security card, then they can't choose
/// to execute that condition's contents.)"
/// §15-8-4-4-1 "If an activation-type effect has optional processing
/// conditions, a player can only declare activation while the optional
/// processing conditions can be performed. ..."
///
/// Ver. 3.6 said the opposite (the player could choose the unpayable
/// condition); owner ruling 2026-10-10 #6: never offer a trigger whose
/// condition can't be paid -- an offered unpayable choice is an illegal action
/// in the RL action space.
///
/// - Trigger-type (the rule's own example, `WA_TRASH_SEC_MINUS5000`, 6000 DP):
///   with 0 security cards, P0 attacks P1's suspended `LV5_PLAIN` (7000 DP).
///   No decision is offered, the -5000 DP never applies, and the battle
///   proceeds: P0's Digimon loses.
/// - Activation-type (`MAIN_TRASH_SEC_DRAW`, "[Main] By trashing your top
///   security card, <Draw 1>"): with 0 security cards the [Main] is not in the
///   action mask (§15-8-4-4-1); with 1 it is (control). §15-8-4-4-1's own
///   <Training> example passes in `ch15_08_effect_categories`.
#[test]
fn unpayable_optional_condition_cannot_be_chosen() {
    cite!(
        "15-7-4",
        "210ce11e09b84d8a05b2b4861fcbff3ad5d398894a7da5879d8c869dc6446b14"
    );
    cite!(
        "15-8-4-4-1",
        "0e89941fdf078b6f25cc2ef1a3b55a9599c8d919038ff4109430d7112a2fabdd"
    );
    // Trigger-type.
    let mut r = ch15_07()
        .deck(0, &[FILL; 4])
        .deck(1, &[FILL; 4])
        .memory(3)
        .start();
    assert_eq!(r.security_count(0), 0);
    let attacker = r.place_on_field(0, "WA_TRASH_SEC_MINUS5000", Some(0));
    let defender = r.place_on_field(1, "LV5_PLAIN", Some(0));
    set_suspended(&mut r, defender, true);
    r.attack_digimon(attacker, defender, false);
    assert!(
        r.pending_selection().is_none(),
        "0 security cards: \"by trashing your top security card\" can't be chosen, so nothing is \
         offered: {:?}",
        r.pending_selection()
    );
    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["LV5_PLAIN"]),
        "no -5000 DP: the 7000 DP Digimon won the battle"
    );
    assert!(
        field_top_ids(&r, 0).is_empty(),
        "the attack proceeded: the 6000 DP attacker lost the battle"
    );

    // Activation-type.
    assert_eq!(
        main_trash_sec_draw_mask_bit(&[]),
        0.0,
        "0 security cards: the [Main] can't be declared"
    );
    assert_eq!(
        main_trash_sec_draw_mask_bit(&[FILL]),
        1.0,
        "control: 1 security card, the [Main] can be declared"
    );
}

// ─── §15-7-5 ────────────────────────────────────────────────────────────────

/// §15-7-5 "A player can choose to execute the content of optional processing
/// conditions, regardless of whether or not the content after the conditions
/// can be executed. (Example: If an effect reads "[When Attacking] By
/// trashing your top security card, 1 of your opponent's Digimon gets -5000
/// DP for the turn," a player can choose to trash their top security card
/// even when the opponent doesn't have any Digimon in the battle area.)"
///
/// The rule's own example: P0 has 1 security card; P1 has no Digimon. P0's
/// `WA_TRASH_SEC_MINUS5000` attacks P1. The decision is still offered;
/// accepting trashes P0's top security card, and nothing else happens (no
/// target to pick). The attack then checks P1's security.
#[test]
fn payable_condition_offered_even_when_payload_has_no_target() {
    cite!(
        "15-7-5",
        "365504508a7aa1371b0816150270634e4998acefc7d053f912e35160f5ae17e0"
    );
    let mut r = ch15_07()
        .deck(0, &[FILL; 4])
        .deck(1, &[FILL; 4])
        .security(0, &[FILL])
        .security(1, &[FILL; 2])
        .memory(3)
        .start();
    let attacker = r.place_on_field(0, "WA_TRASH_SEC_MINUS5000", Some(0));
    assert_eq!(r.battle_area_size(1), 0, "the opponent has no Digimon");
    r.attack_player(attacker, 1, false);

    assert_optional_decision(&r, 0, "by trashing your top security card");
    assert_eq!(r.security_count(1), 2, "offered before the security check");
    accept(&mut r, 0);
    assert!(
        r.pending_selection().is_none(),
        "no target for the -5000 DP: nothing else is asked: {:?}",
        r.pending_selection()
    );
    assert_eq!(r.security_count(0), 0, "the top security card was trashed");
    assert_eq!(trash_ids(&r, 0), owned(&[FILL]));
    assert_eq!(
        r.security_count(1),
        1,
        "the attack proceeded to the security check"
    );
}

// ─── §15-1-2 ────────────────────────────────────────────────────────────────

/// §15-1-2 (Ver. 4.3) "A single effect is processed in the order shown in the
/// text on the card. However, if that single effect has optional processing
/// conditions that affect it, decide whether or not to perform them before
/// processing the effect."
///
/// P0 (1 security card) attacks P1's suspended `FILL` with
/// `WA_TRASH_SEC_MINUS5000`; P1 also has `LV5_PLAIN`, so the -5000 DP has two
/// candidate targets. The first prompt is the yes/no decision on the
/// condition (not the target pick); the target pick comes only after the
/// decision, with the condition already performed.
#[test]
fn optional_condition_decided_before_processing_effect() {
    cite!(
        "15-1-2",
        "6e6be130b9b99fe1ea6789c1cdb39b7d840c8db04b5a9a22328d48efc25675e1"
    );
    let mut r = ch15_07()
        .deck(0, &[FILL; 4])
        .deck(1, &[FILL; 4])
        .security(0, &[FILL])
        .memory(3)
        .start();
    let attacker = r.place_on_field(0, "WA_TRASH_SEC_MINUS5000", Some(0));
    let target = r.place_on_field(1, FILL, Some(0));
    let big = r.place_on_field(1, "LV5_PLAIN", Some(1));
    set_suspended(&mut r, target, true);
    r.attack_digimon(attacker, target, false);

    let first = r.pending_selection_view().expect("a decision is pending");
    assert_ne!(
        r.pending_kind(),
        Some(SelectionKind::OppField),
        "the first prompt is not the payload's target pick: {first:?}"
    );
    assert!(
        first.is_optional,
        "the first prompt is the yes/no decision: {first:?}"
    );
    assert_eq!(first.selecting_player, 0);
    assert_eq!(
        r.security_count(0),
        1,
        "nothing processed before the decision"
    );
    accept(&mut r, 0);

    let second = r
        .pending_selection_view()
        .expect("the payload's target pick follows the decision");
    assert_eq!(
        second.kind,
        SelectionKind::OppField,
        "the -5000 DP target pick: {second:?}"
    );
    assert_eq!(
        r.security_count(0),
        0,
        "the condition was performed before the effect's processing"
    );
    pick_field(&mut r, big.index as u16);
    assert!(
        r.pending_selection().is_none(),
        "{:?}",
        r.pending_selection()
    );
    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["LV5_PLAIN"]),
        "the attacked Digimon was deleted in battle"
    );
    let big = r.perm_handle(1, 0);
    assert_eq!(r.dp_of(big), Some(2000), "-5000 DP for the turn");
}

// ─── §15-15-7-4 ─────────────────────────────────────────────────────────────

/// A board for the §15-15-7-4 test: P0's `WA_BORROW_OP` attacks P1's
/// suspended `FILL`; P0's trash holds the `[Lender]` card `lender`; P0 has 2
/// security cards and 1 card in hand. The mandatory [When Attacking] places
/// `lender` under it (the only candidate) and activates its [On Play].
fn borrow_board(lender: &str) -> DebugRunner {
    let mut r = ch15_07()
        .hand(0, &[FILL])
        .deck(0, &[FILL; 4])
        .deck(1, &[FILL; 4])
        .security(0, &[FILL; 2])
        .memory(3)
        .start();
    r.inject_trash(0, lender);
    let attacker = r.place_on_field(0, "WA_BORROW_OP", Some(0));
    let target = r.place_on_field(1, FILL, Some(0));
    set_suspended(&mut r, target, true);
    r.attack_digimon(attacker, target, false);
    // The mandatory placement pick (one candidate).
    step_prompt(&mut r);
    let perm = &r.game.players[0].battle_area[attacker.index as usize];
    assert_eq!(
        card_ids(&r, &perm.card_sources).first().map(String::as_str),
        Some(lender),
        "the [Lender] card was placed as the bottom digivolution card"
    );
    r
}

/// §15-15-7-4 (Ver. 4.3) "Even if the effect activating another effect is
/// mandatory, the player can still choose whether or not to processes that
/// other effect's optional processing conditions. (Example: When activating
/// another effect with the effect "[When Attacking] [Once Per Turn] Activate 1
/// [On Play] effect on a face-up [Zaxon] trait Digimon card in your security
/// stack as an effect of this Digimon" and that other effect has optional
/// processing conditions, the player can choose whether or not to process
/// them.)"
///
/// `WA_BORROW_OP`'s mandatory [When Attacking] activates the [On Play] of the
/// card it places under itself, as its own effect. Each borrowed [On Play]
/// has an optional processing condition, and P0 can decline it:
/// - `LEND_SEC_MEM` "By trashing your top security card, gain 1 memory"
///   (decision = the clause-level optional prompt);
/// - `LEND_HAND_MEM` "By trashing 1 card in your hand, gain 1 memory"
///   (decision = the optional hand pick).
/// Control: accepting `LEND_SEC_MEM`'s condition trashes the security card and
/// gains the memory.
#[test]
#[ignore = "G-ENGINE-REFIRED-EFFECT-SKIPS-OPTIONAL-DECISION (§15-15-7-4)"]
fn mandatory_invoker_does_not_force_invoked_optional_condition() {
    cite!(
        "15-15-7-4",
        "dfbb74f72cbbf179b325e51c074de8561efa8fba4c696d24501e848805b2752d"
    );
    // Security-trash condition, declined.
    let mut r = borrow_board("LEND_SEC_MEM");
    let cp = r.event_checkpoint();
    assert_optional_decision(&r, 0, "the borrowed \"by trashing your top security card\"");
    r.execute_action(0, PASS)
        .expect("decline the borrowed condition");
    assert!(
        r.pending_selection().is_none(),
        "{:?}",
        r.pending_selection()
    );
    assert_eq!(r.security_count(0), 2, "declined: no security card trashed");
    assert!(
        memory_sources(&r, cp).is_empty(),
        "declined: no memory: {:?}",
        memory_sources(&r, cp)
    );
    assert!(field_top_ids(&r, 1).is_empty(), "the attack proceeded");

    // Hand-trash condition, declined.
    let mut r = borrow_board("LEND_HAND_MEM");
    let cp = r.event_checkpoint();
    assert_optional_decision(&r, 0, "the borrowed \"by trashing 1 card in your hand\"");
    r.execute_action(0, PASS)
        .expect("decline the borrowed condition");
    assert!(
        r.pending_selection().is_none(),
        "{:?}",
        r.pending_selection()
    );
    assert_eq!(hand_ids(&r, 0), owned(&[FILL]), "declined: nothing trashed");
    assert!(
        memory_sources(&r, cp).is_empty(),
        "{:?}",
        memory_sources(&r, cp)
    );

    // Control: the security-trash condition, accepted.
    let mut r = borrow_board("LEND_SEC_MEM");
    let memory0 = r.memory();
    assert_optional_decision(&r, 0, "the borrowed \"by trashing your top security card\"");
    accept(&mut r, 0);
    assert!(
        r.pending_selection().is_none(),
        "{:?}",
        r.pending_selection()
    );
    assert_eq!(r.security_count(0), 1, "the top security card was trashed");
    assert_eq!(
        r.memory(),
        memory0 + 1,
        "the borrowed effect gained 1 memory"
    );
}
