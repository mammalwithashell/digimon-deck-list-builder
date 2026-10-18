//! Chapters 11-14 (Comprehensive Rules Ver. 4.3): the attack procedure
//! (§11), blocking (§12), security checks (§13) and battles (§14), with the
//! keyword rules they lean on: `<Security A.>` (§16-4), `<Piercing>` (§16-7),
//! `<Jamming>` (§16-9) and `[Security]` (§15-16-10).
//!
//! Every test asserts the RULE, never the engine's current output. A test the
//! engine fails is `#[ignore]`d with the gap id that explains it (discover then
//! pin); see `docs/RUST_ENGINE_GAPS.md` and `qa/rules/cr_obligations_status.yaml`.
//!
//! Fixture cards: `fixtures/common/*.yaml` + `fixtures/ch11_14/*.yaml` (plus
//! `WA_DEL4K` from `ch15_04`, `LV4_PLAIN` from `ch15_08`, `OPT_PLAIN` /
//! `TAMER_PLAIN` from `ch17` and `INH_PIERCE` from `ch18`), one synthetic card
//! per file, authored to printed-style text (each clause's `summary:` is that
//! text).
//!
//! Security stacks are listed bottom first: the LAST card of `.security(..)`
//! is the top card, the one a check reveals first.

use crate::fixtures;
use crate::support::*;
use digimon_engine::action::space::ATTACK_START;
use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{GamePhase, PlayerId};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

fn ch11_14() -> DebugRunnerBuilder {
    fixtures::builder(&[
        fixtures::ch15_04_cards(),
        fixtures::ch15_08_cards(),
        fixtures::ch17_cards(),
        fixtures::ch18_cards(),
        fixtures::ch11_14_cards(),
    ])
}

/// P0 (memory 5) to move, P1 with the security stack `security` (bottom
/// first).
fn board(security: &[&str]) -> DebugRunner {
    ch11_14()
        .deck(0, &[FILL; 4])
        .deck(1, &[FILL; 4])
        .security(1, security)
        .memory(5)
        .start()
}

/// Place a suspended `card_id` on `player`'s field (a legal attack target).
fn place_suspended(r: &mut DebugRunner, player: PlayerId, card_id: &str) -> PermanentHandle {
    let h = r.place_on_field(player, card_id, Some(0));
    set_suspended(r, h, true);
    h
}

/// Card ids revealed from `defender`'s security stack since `cp`, in order.
fn revealed(r: &DebugRunner, cp: usize, defender: PlayerId) -> Vec<String> {
    r.events_since(cp)
        .iter()
        .filter_map(|e| match e {
            GameEvent::SecurityReveal {
                defender: d,
                card_id,
                ..
            } if *d == defender => Some(card_id.clone()),
            _ => None,
        })
        .collect()
}

/// Index in the event log (since `cp`) of the first effect-sourced memory
/// change whose source is `card_id`.
fn memory_event_index(r: &DebugRunner, cp: usize, card_id: &str) -> Option<usize> {
    r.events_since(cp).iter().position(
        |e| matches!(e, GameEvent::MemoryChange { source_card_id: Some(id), .. } if id == card_id),
    )
}

/// Whether `card_id`'s memory effect resolved since `cp`.
fn resolved(r: &DebugRunner, cp: usize, card_id: &str) -> bool {
    memory_event_index(r, cp, card_id).is_some()
}

/// An attack window the defending player was offered.
#[derive(Debug, PartialEq)]
enum Window {
    /// The counter timing (§11-3), offered to this player.
    Counter(PlayerId),
    /// The block timing (§11-4), offered to this player with these blocker
    /// slots.
    Block(PlayerId, Vec<u16>),
}

/// Resolve the attack, declining every counter and block window and taking
/// the only option of any other prompt. Returns the windows offered, in
/// order.
fn decline_windows(r: &mut DebugRunner) -> Vec<Window> {
    let mut windows = Vec::new();
    for _ in 0..16 {
        let Some(sel) = r.pending_selection() else {
            return windows;
        };
        match r.current_phase() {
            GamePhase::CounterTiming => {
                windows.push(Window::Counter(sel.selecting_player));
                r.decline_optional_trigger()
                    .expect("decline the counter timing");
            }
            GamePhase::BlockTiming => {
                let mut slots: Vec<u16> = sel
                    .valid_action_ids
                    .iter()
                    .map(|&a| a - ATTACK_START)
                    .collect();
                slots.sort();
                windows.push(Window::Block(sel.selecting_player, slots));
                r.decline_optional_trigger().expect("decline the block");
            }
            _ => step_prompt(r),
        }
    }
    panic!("the attack did not settle after 16 decisions");
}

// ─── §11-1-5 / §12-1-6 ──────────────────────────────────────────────────────

/// §11-1-5 "Once an attack declaration has been made, all of the timings that
/// follow will occur. (Example: Even if an attacking Digimon leaves the
/// battle area, the non-turn player's counter timing and block timing will
/// still occur.)"
///
/// P1 has `COUNTER_MEM` ("[Counter] Gain 1 memory") and an unsuspended
/// `BLOCKER`; P0 has `EOA_OBS` ("[All Turns] At the end of an attack, gain 1
/// memory").
/// - Control: P0's `LV5_PLAIN` attacks P1: P1 is offered the counter timing,
///   then the block timing; the attack checks security and ends.
/// - P0's `WA_DEL_SELF` ("[When Attacking] Delete this Digimon") attacks P1
///   and deletes itself. The timings still follow: P1 is still offered the
///   counter timing (§11-1-5), the block timing offers no block (no attacking
///   Digimon, §12-1-6; pinned on its own by
///   `block_not_offered_when_attacker_left_battle_area`), nothing checks
///   security, and the end of attack timing arrives (`EOA_OBS` resolves).
#[test]
fn attack_timings_continue_when_attacker_leaves_mid_resolution() {
    cite!(
        "11-1-5",
        "cd109077bced1cb39983cddfc3851c42da90acec2ee7c54922c7d0df6542d1c9"
    );
    let leave_board = |attacker: &str| {
        let mut r = board(&[FILL; 3]);
        r.place_on_field(0, "EOA_OBS", Some(0));
        let a = r.place_on_field(0, attacker, Some(0));
        r.place_on_field(1, "COUNTER_MEM", Some(0));
        r.place_on_field(1, "BLOCKER", Some(0));
        let cp = r.event_checkpoint();
        r.attack_player(a, 1, false);
        let windows = decline_windows(&mut r);
        (r, cp, windows)
    };

    // Control: the attacker stays.
    let (r, cp, windows) = leave_board("LV5_PLAIN");
    assert_eq!(
        windows,
        vec![Window::Counter(1), Window::Block(1, vec![1])],
        "the fixture's counter and block windows"
    );
    assert_eq!(revealed(&r, cp, 1), owned(&[FILL]));
    assert!(resolved(&r, cp, "EOA_OBS"));

    // The attacker deletes itself on its [When Attacking].
    let (r, cp, windows) = leave_board("WA_DEL_SELF");
    assert_eq!(
        field_top_ids(&r, 0),
        owned(&["EOA_OBS"]),
        "the attacker left the battle area"
    );
    assert_eq!(
        windows,
        vec![Window::Counter(1)],
        "the counter timing still occurs (§11-1-5); the block timing offers no block without \
         an attacking Digimon (§12-1-6)"
    );
    assert!(
        revealed(&r, cp, 1).is_empty(),
        "no Digimon to check security"
    );
    assert_eq!(r.security_count(1), 3);
    assert!(
        resolved(&r, cp, "EOA_OBS"),
        "the end of attack timing still arrives"
    );
}

/// §12-1-6 "A block can only be performed if an attacking Digimon is in the
/// battle area."
///
/// P1 has an unsuspended `BLOCKER` (and no [Counter] effect); P0 has
/// `EOA_OBS`.
/// - Control: P0's `LV5_PLAIN` attacks P1: the block timing offers `BLOCKER`.
/// - P0's `WA_DEL_SELF` ("[When Attacking] Delete this Digimon") attacks P1
///   and deletes itself: with no attacking Digimon in the battle area no block
///   is offered; nothing checks security, `BLOCKER` stays unsuspended, and the
///   end of attack timing still arrives (`EOA_OBS` resolves, §11-1-5).
#[test]
fn block_not_offered_when_attacker_left_battle_area() {
    cite!(
        "12-1-6",
        "7c4fe4546109936efb5011ee2e4e08278f57a2c7f8d563fbee572015561172c9"
    );
    let attack = |attacker: &str| {
        let mut r = board(&[FILL; 3]);
        r.place_on_field(0, "EOA_OBS", Some(0));
        let a = r.place_on_field(0, attacker, Some(0));
        r.place_on_field(1, "BLOCKER", Some(0));
        let cp = r.event_checkpoint();
        r.attack_player(a, 1, false);
        let windows = decline_windows(&mut r);
        (r, cp, windows)
    };

    let (_, _, windows) = attack("LV5_PLAIN");
    assert_eq!(windows, vec![Window::Block(1, vec![0])], "control");

    let (r, cp, windows) = attack("WA_DEL_SELF");
    assert_eq!(
        field_top_ids(&r, 0),
        owned(&["EOA_OBS"]),
        "the attacker left the battle area"
    );
    assert!(
        windows.is_empty(),
        "no block without an attacking Digimon in the battle area: {windows:?}"
    );
    assert!(revealed(&r, cp, 1).is_empty(), "no security check");
    assert!(
        !is_suspended(&r, r.perm_handle(1, 0)),
        "the blocker did not block"
    );
    assert!(
        resolved(&r, cp, "EOA_OBS"),
        "the end of attack timing arrives"
    );
}

// ─── §11-2-6 ────────────────────────────────────────────────────────────────

/// §11-2-6 "Even if the attack target Digimon is removed during an attack,
/// that Digimon remains the attack target, but the attack fails. (Example:
/// Even if an attack target Digimon is deleted by a [When Attacking] effect,
/// that Digimon remains the attack target. The attack fails because the
/// attack target Digimon is removed.)"
///
/// The rule's own example. P0's `WA_DEL4K` ("[When Attacking] Delete 1 of your
/// opponent's Digimon with 4000 DP or less") attacks P1's suspended `FILL`
/// (1000 DP) and deletes it. P1's suspended `LV5_PLAIN` (7000 DP) stays on
/// the field. The deleted Digimon is still the target, so the attack fails:
/// no battle (P0's `EOB_OBS`, "[All Turns] At the end of a battle, gain 1
/// memory", does not resolve; neither the attacker nor `LV5_PLAIN` is
/// deleted) and no security check.
#[test]
fn removed_target_stays_the_target_and_attack_fails() {
    cite!(
        "11-2-6",
        "e60a2eaee1b38ae5a8c29be9bb45b4cc36ed04dfdad365b10fa5eb3b7953c938"
    );
    let mut r = board(&[FILL; 3]);
    r.place_on_field(0, "EOB_OBS", Some(0));
    let attacker = r.place_on_field(0, "WA_DEL4K", Some(0));
    let target = place_suspended(&mut r, 1, FILL);
    place_suspended(&mut r, 1, "LV5_PLAIN");
    let cp = r.event_checkpoint();

    r.attack_digimon(attacker, target, false);
    resolve_prompts(&mut r);

    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["LV5_PLAIN"]),
        "the target was deleted by [When Attacking]; the other Digimon was not attacked"
    );
    assert_eq!(
        field_top_ids(&r, 0),
        owned(&["EOB_OBS", "WA_DEL4K"]),
        "the attacker battled nothing"
    );
    assert!(!resolved(&r, cp, "EOB_OBS"), "no battle happened");
    assert!(revealed(&r, cp, 1).is_empty(), "no security check");
    assert_eq!(r.security_count(1), 3);
}

// ─── §11-5-1-4-1 ────────────────────────────────────────────────────────────

/// §11-5-1-4 "The Digimon's attack is not successful"
/// §11-5-1-4-1 "In this situation, no other actions are taken and the end of
/// attack timing occurs."
///
/// P0's `WA_DEL4K` attacks P1's only Digimon, a suspended `FILL`, and its
/// [When Attacking] deletes it: the attack is not successful. No other action
/// is taken (no battle: `EOB_OBS` does not resolve; no security check), and
/// the end of attack timing occurs (`EOA_OBS` resolves).
#[test]
fn unsuccessful_attack_reaches_end_of_attack_timing() {
    cite!(
        "11-5-1-4-1",
        "943bbb3d367f25255ebe7c1912bfda028ffcced96566ae352b3dacfa6a2d9d43"
    );
    let mut r = board(&[FILL; 3]);
    r.place_on_field(0, "EOA_OBS", Some(0));
    r.place_on_field(0, "EOB_OBS", Some(0));
    let attacker = r.place_on_field(0, "WA_DEL4K", Some(0));
    let target = place_suspended(&mut r, 1, FILL);
    let cp = r.event_checkpoint();

    r.attack_digimon(attacker, target, false);
    resolve_prompts(&mut r);

    assert!(
        field_top_ids(&r, 1).is_empty(),
        "the target was deleted before the battle"
    );
    assert!(!resolved(&r, cp, "EOB_OBS"), "no battle");
    assert!(revealed(&r, cp, 1).is_empty(), "no security check");
    assert_eq!(r.security_count(1), 3);
    assert!(
        resolved(&r, cp, "EOA_OBS"),
        "the end of attack timing occurs"
    );
    assert!(r.game.pending_attack.is_none(), "the attack ended");
}

// ─── §11-6-2 ────────────────────────────────────────────────────────────────

/// §11-6-2 "For the end of the attack timing, the attack won't end until all
/// processing has been resolved."
///
/// P0's `WA_DP_UP_EOA_MAY` ("[When Attacking] This Digimon gets +2000 DP for
/// the attack. [End of Attack] You may gain 1 memory.", 6000 DP) attacks P1
/// and checks a `FILL`. At the end of attack timing its optional effect waits
/// on P0's decision: the attack has not ended yet, so the attack is still in
/// progress and the "+2000 DP for the attack" still applies (8000 DP). Once
/// the effect resolves, the attack ends and the DP is back to 6000.
#[test]
#[ignore = "G-ENGINE-ATTACK-ENDS-BEFORE-END-OF-ATTACK-EFFECTS-RESOLVE (§11-6-2)"]
fn attack_does_not_end_until_all_processing_resolved() {
    cite!(
        "11-6-2",
        "1ff1d8bbe91ca39303301f28674c016bedbefa0af1de276c657c3d443fed84f7"
    );
    let mut r = board(&[FILL; 3]);
    let attacker = r.place_on_field(0, "WA_DP_UP_EOA_MAY", Some(0));
    let cp = r.event_checkpoint();

    r.attack_player(attacker, 1, false);

    assert_eq!(revealed(&r, cp, 1), owned(&[FILL]), "the security check ran");
    let sel = r
        .pending_selection()
        .expect("the [End of Attack] decision");
    assert!(
        sel.is_optional && sel.selecting_player == 0,
        "P0's optional [End of Attack]: {sel:?}"
    );
    assert!(
        !resolved(&r, cp, "WA_DP_UP_EOA_MAY"),
        "the [End of Attack] effect has not resolved"
    );
    assert_eq!(
        r.effective_dp(attacker),
        Some(8000),
        "'+2000 DP for the attack' still applies: the attack is still going"
    );
    assert!(
        r.game.pending_attack.is_some(),
        "the attack has not ended while its end-of-attack processing is unresolved"
    );

    accept(&mut r, 0);
    resolve_prompts(&mut r);
    assert!(resolved(&r, cp, "WA_DP_UP_EOA_MAY"));
    assert!(r.game.pending_attack.is_none(), "now the attack has ended");
    assert_eq!(r.effective_dp(attacker), Some(6000));
}

// ─── §12-1-2 ────────────────────────────────────────────────────────────────

/// §12-1-2 "A block can only be performed once per attack. Multiple Digimon
/// can't block at the same time."
///
/// P0's `LV5_PLAIN` (7000 DP) attacks P1, who has two unsuspended `BLOCKER`s
/// (3000 DP). The block timing offers both; P1 blocks with the first. No
/// second block is offered: the first blocker battles and is deleted, the
/// second stays unsuspended, and nothing checks security (the attack target
/// switched to the blocker).
#[test]
fn one_block_per_attack() {
    cite!(
        "12-1-2",
        "9c77099b07ff154b6700de54b00bd08aa903d5a8387c113a361c76e2c2fc28d5"
    );
    let mut r = board(&[FILL; 3]);
    let attacker = r.place_on_field(0, "LV5_PLAIN", Some(0));
    r.place_on_field(1, "BLOCKER", Some(0));
    r.place_on_field(1, "BLOCKER", Some(0));
    let cp = r.event_checkpoint();

    r.attack_player(attacker, 1, false);

    assert_eq!(r.current_phase(), GamePhase::BlockTiming);
    let sel = r.pending_selection().expect("the block timing");
    assert_eq!(
        (sel.kind, sel.selecting_player),
        (SelectionKind::OwnField, 1),
        "{sel:?}"
    );
    let mut offered: Vec<u16> = sel
        .valid_action_ids
        .iter()
        .map(|&a| a - ATTACK_START)
        .collect();
    offered.sort();
    assert_eq!(offered, vec![0, 1], "both blockers are offered");

    pick_field(&mut r, 0);
    assert!(
        r.pending_selection().is_none(),
        "no second block: {:?}",
        r.pending_selection()
    );
    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["BLOCKER"]),
        "the blocker battled and was deleted"
    );
    assert!(
        !is_suspended(&r, r.perm_handle(1, 0)),
        "the other blocker did not block"
    );
    assert!(revealed(&r, cp, 1).is_empty(), "the attack was blocked");
    assert_eq!(r.security_count(1), 3);
}

// ─── §12-1-4 ────────────────────────────────────────────────────────────────

/// §12-1-4 "A block can't be performed using a Digimon that can't suspend."
///
/// P0's `LV5_PLAIN` attacks P1.
/// - Control: P1's unsuspended `BLOCKER` is offered a block.
/// - P1's unsuspended `BLOCKER_CANNOT_SUSPEND` (<Blocker>, "[All Turns] This
///   Digimon can't suspend") is not: no block timing decision, and the attack
///   checks security.
#[test]
fn digimon_that_cannot_suspend_cannot_block() {
    cite!(
        "12-1-4",
        "9b174637433f57cb42d83751f0814a3226f8fc55c34764b2f29bdfb8e5e8f44c"
    );
    let attack_with_blocker = |blocker: &str| {
        let mut r = board(&[FILL; 3]);
        let attacker = r.place_on_field(0, "LV5_PLAIN", Some(0));
        r.place_on_field(1, blocker, Some(0));
        let cp = r.event_checkpoint();
        r.attack_player(attacker, 1, false);
        let windows = decline_windows(&mut r);
        (r, cp, windows)
    };

    let (_, _, windows) = attack_with_blocker("BLOCKER");
    assert_eq!(windows, vec![Window::Block(1, vec![0])], "control");

    let (r, cp, windows) = attack_with_blocker("BLOCKER_CANNOT_SUSPEND");
    assert!(
        windows.is_empty(),
        "a Digimon that can't suspend can't block: {windows:?}"
    );
    assert_eq!(revealed(&r, cp, 1), owned(&[FILL]), "the attack checked security");
    assert!(!is_suspended(&r, r.perm_handle(1, 0)));
}

// ─── §12-1-5 ────────────────────────────────────────────────────────────────

/// §12-1-5 "The attack target Digimon can't perform a block."
///
/// P0's `LV5_PLAIN` attacks P1's suspended `BLOCKER_WAKES` (<Blocker>, "[All
/// Turns] When an opponent's Digimon attacks, unsuspend this Digimon"); P1
/// also has an unsuspended `BLOCKER`. The attack unsuspends the target, so at
/// the block timing both are unsuspended <Blocker>s, but only `BLOCKER` is
/// offered. P1 declines: the target battles and is deleted.
#[test]
fn attack_target_cannot_block() {
    cite!(
        "12-1-5",
        "4e08bbe91f1566c3a4511871f42f9cf6e0f183f5a086ca160d54cd8b60d180bd"
    );
    let mut r = board(&[FILL; 3]);
    let attacker = r.place_on_field(0, "LV5_PLAIN", Some(0));
    let target = place_suspended(&mut r, 1, "BLOCKER_WAKES");
    r.place_on_field(1, "BLOCKER", Some(0));

    r.attack_digimon(attacker, target, false);

    assert_eq!(r.current_phase(), GamePhase::BlockTiming);
    assert!(
        !is_suspended(&r, target),
        "the target unsuspended itself when attacked"
    );
    let windows = decline_windows(&mut r);
    assert_eq!(
        windows,
        vec![Window::Block(1, vec![1])],
        "only the Digimon that is not the attack target is offered"
    );
    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["BLOCKER"]),
        "the target battled and was deleted"
    );
}

// ─── §13-1-2 / §16-7-5 ──────────────────────────────────────────────────────

/// §13-1-2 "Only 1 security check can be performed during a single attack.
/// However, if the number of cards that can be checked is modified by an
/// effect on the attacking Digimon, the modified number of cards will be
/// checked in a single security check."
/// §16-7-5 "Only 1 security check can be performed during a single attack,
/// even if <Piercing> activates multiple times. However, if the number of
/// cards that can be checked by the attacking Digimon is modified, that
/// number of cards will be checked in a single security check."
///
/// P0's attacker deletes P1's suspended `FILL` in battle, then its <Piercing>
/// checks security; P1 has 5 security cards.
/// - `SA_PLUS1` (<Security A. +1>) over `INH_PIERCE` (inherited <Piercing>):
///   the one <Piercing> check checks 2 cards.
/// - `PIERCING_X2` (<Piercing>, "[Your Turn] This Digimon gains <Piercing>")
///   over `SA_PLUS1` (inherited <Security A. +1>): two <Piercing> instances,
///   still ONE check of 2 cards (not 2 checks, not 4 cards).
#[test]
fn one_security_check_per_attack_sa_plus_counts_inside_it() {
    cite!(
        "13-1-2",
        "c9b9765e890e2e8ac06b181a8b876d7e791bb251f170d985bcd465ddc2b778dc"
    );
    cite!(
        "16-7-5",
        "17fb451c882263bc48e6451c841935dbb5263e4d5aa17ee704b94c8f4c47ca82"
    );
    for stack in [["INH_PIERCE", "SA_PLUS1"], ["SA_PLUS1", "PIERCING_X2"]] {
        let mut r = board(&[FILL; 5]);
        let attacker = r.place_stack(0, &stack);
        let defender = place_suspended(&mut r, 1, FILL);
        let cp = r.event_checkpoint();

        r.attack_digimon(attacker, defender, false);
        resolve_prompts(&mut r);

        assert!(
            field_top_ids(&r, 1).is_empty(),
            "{stack:?}: the defender was deleted in battle"
        );
        assert_eq!(
            revealed(&r, cp, 1),
            owned(&[FILL, FILL]),
            "{stack:?}: one security check, of the modified 2 cards"
        );
        assert_eq!(r.security_count(1), 3, "{stack:?}");
    }
}

// ─── §13-1-5 ────────────────────────────────────────────────────────────────

/// §13-1-5 "If the Digimon performing the security check is no longer present
/// in the battle area, that card can't perform any more security checks."
///
/// P0's `SA_PLUS1` (5000 DP, <Security A. +1>) attacks P1.
/// - Control: P1's top 2 cards are `FILL`s: both are checked.
/// - P1's top card is `LV5_PLAIN` (7000 DP): `SA_PLUS1` loses the battle
///   against it and is deleted, so the second card is never checked.
#[test]
fn checking_digimon_leaving_stops_further_checks() {
    cite!(
        "13-1-5",
        "691eac6b95b973174af71e4f9642ae2b4a06681f8886896e47f51bea1c485873"
    );
    let attack = |security: &[&str]| {
        let mut r = board(security);
        let attacker = r.place_on_field(0, "SA_PLUS1", Some(0));
        let cp = r.event_checkpoint();
        r.attack_player(attacker, 1, false);
        resolve_prompts(&mut r);
        (r, cp)
    };

    let (r, cp) = attack(&[FILL, FILL, FILL]);
    assert_eq!(revealed(&r, cp, 1), owned(&[FILL, FILL]), "control: 2 checks");
    assert_eq!(field_top_ids(&r, 0), owned(&["SA_PLUS1"]));

    let (r, cp) = attack(&[FILL, FILL, "LV5_PLAIN"]);
    assert_eq!(
        revealed(&r, cp, 1),
        owned(&["LV5_PLAIN"]),
        "the checking Digimon left: no more checks"
    );
    assert!(
        field_top_ids(&r, 0).is_empty(),
        "SA_PLUS1 lost the security battle"
    );
    assert_eq!(r.security_count(1), 2);
}

// ─── §13-1-8-2 / §13-1-8-3 ──────────────────────────────────────────────────

/// §13-1-8 "A security check is performed using the following procedure."
/// §13-1-8-2-1 "If an effect is triggered by a security check, that effect is
/// to be resolved. The next action won't begin until all of the processing
/// is resolved for the current action."
/// §13-1-8-3-1 "A battle will occur between the Security Digimon and the
/// Digimon that performed the security check. …"
///
/// P0's `LV5_PLAIN` (7000 DP) attacks P1, whose top security card is
/// `LV4_PLAIN` (5000 DP).
/// - Control: `LV5_PLAIN` wins the security battle.
/// - P1 also has `SEC_CHECK_OBS_MINUS_DP` ("[All Turns] When your opponent's
///   Digimon checks your security, that Digimon gets -3000 DP for the turn").
///   It triggers on the check and resolves BEFORE the battle (§13-1-8-2 comes
///   before §13-1-8-3): `LV5_PLAIN` battles at 4000 DP, loses and is deleted.
#[test]
fn check_triggered_effects_resolve_before_security_battle() {
    cite!(
        "13-1-8",
        "d796225ea662544003beeb852eb91ae73c1a94a74d48c0ff98caf97ba27bda77"
    );
    cite!(
        "13-1-8-2-1",
        "0e1852a3368ea62c1d17008d028f5130221066709cb66d79f3170065d76d34ca"
    );
    cite!(
        "13-1-8-3-1",
        "bfd93b2798f4fdd0540acabda0f8ec1ef16c15c5701a0d71dc1cce3a42e05c85"
    );
    let attack = |observer: bool| {
        let mut r = board(&[FILL, FILL, "LV4_PLAIN"]);
        let attacker = r.place_on_field(0, "LV5_PLAIN", Some(0));
        if observer {
            r.place_on_field(1, "SEC_CHECK_OBS_MINUS_DP", Some(0));
        }
        let cp = r.event_checkpoint();
        r.attack_player(attacker, 1, false);
        resolve_prompts(&mut r);
        (r, cp)
    };

    let (r, cp) = attack(false);
    assert_eq!(revealed(&r, cp, 1), owned(&["LV4_PLAIN"]));
    assert_eq!(
        field_top_ids(&r, 0),
        owned(&["LV5_PLAIN"]),
        "control: 7000 DP beats 5000 DP"
    );

    let (r, cp) = attack(true);
    assert_eq!(revealed(&r, cp, 1), owned(&["LV4_PLAIN"]));
    assert!(
        field_top_ids(&r, 0).is_empty(),
        "the check-triggered -3000 DP resolved before the battle: 4000 DP loses to 5000 DP \
         (the attacker survived at {:?} DP)",
        r.effective_dp(r.perm_handle(0, 0))
    );
}

// ─── §15-16-10-2 ────────────────────────────────────────────────────────────

/// §15-16-10-2 "A triggered [Security] effect will immediately activate
/// without pending activation. Therefore, [Security] effects take precedence
/// for activation even when they trigger simultaneously with other effects."
///
/// P0's `LV5_PLAIN` attacks P1 and checks `SEC_MEM_OPT` ("[Security] Gain 1
/// memory"). P1's `SEC_CHECK_OBS_MEM` ("[All Turns] When your opponent's
/// Digimon checks your security, gain 1 memory") triggers on the same check.
/// The [Security] effect activates first, with no ordering choice offered.
#[test]
fn security_effect_activates_before_other_check_triggers() {
    cite!(
        "15-16-10-2",
        "d7a8caaf1d5483ae03127829c74e535ea327713ed7961dccfa1c12e9a895287b"
    );
    let mut r = board(&[FILL, FILL, "SEC_MEM_OPT"]);
    let attacker = r.place_on_field(0, "LV5_PLAIN", Some(0));
    r.place_on_field(1, "SEC_CHECK_OBS_MEM", Some(0));
    let cp = r.event_checkpoint();

    r.attack_player(attacker, 1, false);

    assert!(
        r.pending_selection().is_none(),
        "no ordering choice between the [Security] effect and the observer: {:?}",
        r.pending_selection()
    );
    assert_eq!(revealed(&r, cp, 1), owned(&["SEC_MEM_OPT"]));
    assert_eq!(
        memory_sources(&r, cp),
        vec![
            (1, "SEC_MEM_OPT".to_string()),
            (1, "SEC_CHECK_OBS_MEM".to_string()),
        ],
        "the [Security] effect first"
    );
}

// ─── §13-1-8-3-2 / §13-1-8-4 ────────────────────────────────────────────────

/// §13-1-8-3-2 "If a Security Digimon isn't present, proceed to the next
/// step."
/// §13-1-8-4 "A card revealed from a security check is placed in the trash
/// unless it belongs to an area."
///
/// P0's `FILL` attacks P1, whose top security card is an Option
/// (`OPT_PLAIN`, "[Main] Gain 1 memory", no [Security] effect), then a Tamer
/// (`TAMER_PLAIN`, no text). Neither is a Security Digimon: no battle (P0's
/// `EOB_OBS` does not resolve; the attacker stays), the card's [Main] does
/// not activate, and the card is placed in P1's trash.
#[test]
fn non_digimon_security_card_skips_battle_and_is_trashed() {
    cite!(
        "13-1-8-3-2",
        "daf5f2fb2e75b242e2f16a9c8f11fe9d5312a1581217d1598b49dae255e8a5e9"
    );
    cite!(
        "13-1-8-4",
        "c297ad62a051178c0a1ef333f6e19429eb7bb8ca898e20616839146fe65fb3dc"
    );
    for top in ["OPT_PLAIN", "TAMER_PLAIN"] {
        let mut r = board(&[FILL, FILL, top]);
        r.place_on_field(0, "EOB_OBS", Some(0));
        let attacker = r.place_on_field(0, FILL, Some(0));
        let cp = r.event_checkpoint();

        r.attack_player(attacker, 1, false);
        resolve_prompts(&mut r);

        assert_eq!(revealed(&r, cp, 1), owned(&[top]), "{top}");
        assert!(!resolved(&r, cp, "EOB_OBS"), "{top}: no battle");
        assert!(!resolved(&r, cp, top), "{top}: nothing activated");
        assert_eq!(
            field_top_ids(&r, 0),
            owned(&["EOB_OBS", FILL]),
            "{top}: the attacker stays"
        );
        assert!(field_top_ids(&r, 1).is_empty(), "{top}: not played");
        assert_eq!(trash_ids(&r, 1), owned(&[top]), "{top}: trashed");
        assert_eq!(r.security_count(1), 2, "{top}");
    }
}

// ─── §14-2 ──────────────────────────────────────────────────────────────────

/// §14-2-1 "The DP values of the two battling cards are compared."
/// §14-2-1-1 "The card with the higher DP is the winner."
/// §14-2-1-2 "The card with the lower DP loses."
/// §14-2-1-3 "If both cards have the same DP, both lose the battle."
/// §14-2-2 "The Digimon that loses is immediately deleted. If both lose, both
/// are deleted at the same time."
/// §14-2-4 "If an effect is triggered by a battle, that effect is to be
/// resolved. The next action won't begin until all of the processing is
/// resolved for the current action."
/// §14-2-5 "If an effect is triggered by the end of the battle timing when a
/// battle ends, that effect is to be resolved."
///
/// P0 (with `EOB_OBS`, "[All Turns] At the end of a battle, gain 1 memory")
/// attacks a suspended P1 Digimon:
/// - `LV5_PLAIN` (7000) vs `OD_MEM` (3000, "[On Deletion] Gain 1 memory"):
///   `OD_MEM` is deleted; its [On Deletion], triggered by the battle,
///   resolves before the end of battle effect.
/// - `FILL` (1000) vs `OD_MEM`: the attacker is deleted; `OD_MEM` stays.
/// - `LV4_PLAIN` (5000) vs `LV4_PLAIN`: both are deleted.
/// - `OD_MEM` vs `OD_MEM`: both are deleted at the same time -- both are in
///   the trash before either [On Deletion] resolves.
/// Each battle reaches the end of battle (`EOB_OBS` resolves once), and none
/// checks security.
#[test]
fn battle_compares_dp_then_deletes_loser_then_end_of_battle() {
    cite!(
        "14-2-1",
        "53b93d965e67972f89def137ae32cf78277935213edf430a5a3cfb131acb80c2"
    );
    cite!(
        "14-2-1-1",
        "b211c7d576005ea488210976b53a7107b1d4f947dd21d75f9b40b28db2ba9d54"
    );
    cite!(
        "14-2-1-2",
        "133e36e0c5c9490df1affd661378caee55e121fa70da8b49e62574ffeaed4e51"
    );
    cite!(
        "14-2-1-3",
        "1ca4e24571eb74f8cb693fac6491641fd10f533ebf5ac11be8288ccdb7d53031"
    );
    cite!(
        "14-2-2",
        "8eafdf6422cad42526b7df715508e30d34174cc29a32b30606253dbc6a04f0bf"
    );
    cite!(
        "14-2-4",
        "2e2abfeea7dd5c9b26f20c6fb0bc32ff1f90ed476a8e1ee77441f2c64a5677f2"
    );
    cite!(
        "14-2-5",
        "2a85b51d06f6d8c5131ab14caa6bbce08a733d80eb911587ac8ee1471c330c91"
    );
    let battle = |attacker: &str, defender: &str| {
        let mut r = board(&[FILL; 3]);
        r.place_on_field(0, "EOB_OBS", Some(0));
        let a = r.place_on_field(0, attacker, Some(0));
        let d = place_suspended(&mut r, 1, defender);
        let cp = r.event_checkpoint();
        r.attack_digimon(a, d, false);
        resolve_prompts(&mut r);
        assert!(revealed(&r, cp, 1).is_empty(), "no security check");
        assert_eq!(
            memory_sources(&r, cp)
                .iter()
                .filter(|(_, id)| id == "EOB_OBS")
                .count(),
            1,
            "{attacker} vs {defender}: one end of battle"
        );
        (r, cp)
    };

    // Higher DP wins; the loser is deleted; battle triggers, then end of battle.
    let (r, cp) = battle("LV5_PLAIN", "OD_MEM");
    assert_eq!(field_top_ids(&r, 0), owned(&["EOB_OBS", "LV5_PLAIN"]));
    assert!(field_top_ids(&r, 1).is_empty(), "the loser was deleted");
    assert_eq!(trash_ids(&r, 1), owned(&["OD_MEM"]));
    let on_deletion = memory_event_index(&r, cp, "OD_MEM").expect("[On Deletion] resolved");
    let end_of_battle = memory_event_index(&r, cp, "EOB_OBS").expect("end of battle");
    assert!(
        on_deletion < end_of_battle,
        "the battle-triggered [On Deletion] (#{on_deletion}) resolves before the end of battle \
         (#{end_of_battle})"
    );

    // Lower DP loses.
    let (r, _) = battle(FILL, "OD_MEM");
    assert_eq!(field_top_ids(&r, 0), owned(&["EOB_OBS"]), "the attacker lost");
    assert_eq!(trash_ids(&r, 0), owned(&[FILL]));
    assert_eq!(field_top_ids(&r, 1), owned(&["OD_MEM"]));

    // Same DP: both lose and both are deleted.
    let (r, _) = battle("LV4_PLAIN", "LV4_PLAIN");
    assert_eq!(field_top_ids(&r, 0), owned(&["EOB_OBS"]));
    assert!(field_top_ids(&r, 1).is_empty());
    assert_eq!(trash_ids(&r, 0), owned(&["LV4_PLAIN"]));
    assert_eq!(trash_ids(&r, 1), owned(&["LV4_PLAIN"]));

    // Same DP, both with [On Deletion]: deleted at the same time, so both are
    // in the trash before either [On Deletion] resolves.
    let (r, cp) = battle("OD_MEM", "OD_MEM");
    assert!(field_top_ids(&r, 1).is_empty());
    assert_eq!(field_top_ids(&r, 0), owned(&["EOB_OBS"]));
    let events = r.events_since(cp);
    let trashed = |player: PlayerId| {
        events
            .iter()
            .position(|e| {
                matches!(e, GameEvent::Trash { player: p, card_id, .. }
                    if *p == player && card_id == "OD_MEM")
            })
            .unwrap_or_else(|| panic!("P{player}'s OD_MEM was trashed"))
    };
    let first_on_deletion = memory_event_index(&r, cp, "OD_MEM").expect("[On Deletion] resolved");
    assert!(
        trashed(0) < first_on_deletion && trashed(1) < first_on_deletion,
        "both deleted at the same time: trashed #{} and #{}, first [On Deletion] #{first_on_deletion}",
        trashed(0),
        trashed(1)
    );
    assert_eq!(
        memory_sources(&r, cp)
            .iter()
            .filter(|(_, id)| id == "OD_MEM")
            .count(),
        2,
        "both [On Deletion] effects resolved"
    );
}

// ─── §16-9 ──────────────────────────────────────────────────────────────────

/// §16-9-1 "<Jamming> is a keyword effect where a Digimon with this effect
/// isn't deleted as a result of a battle with an opponent's Security
/// Digimon."
///
/// P0's `JAMMING` (5000 DP, <Jamming>):
/// - attacks P1 and checks `LV5_PLAIN` (7000 DP): it loses the security
///   battle but is not deleted; the checked card is trashed.
/// - attacks P1's suspended `LV5_PLAIN` on the field: it loses that battle and
///   IS deleted (<Jamming> covers Security Digimon only).
#[test]
fn jamming_survives_security_battle_only() {
    cite!(
        "16-9-1",
        "cd92d38dc3687e7ddd246d345627e6c6d0ceae58450b8162f7e0264135f9601d"
    );
    let mut r = board(&[FILL, FILL, "LV5_PLAIN"]);
    let attacker = r.place_on_field(0, "JAMMING", Some(0));
    let cp = r.event_checkpoint();
    r.attack_player(attacker, 1, false);
    resolve_prompts(&mut r);
    assert_eq!(revealed(&r, cp, 1), owned(&["LV5_PLAIN"]));
    assert_eq!(
        field_top_ids(&r, 0),
        owned(&["JAMMING"]),
        "not deleted by losing to a Security Digimon"
    );
    assert_eq!(trash_ids(&r, 1), owned(&["LV5_PLAIN"]));

    let mut r = board(&[FILL; 3]);
    let attacker = r.place_on_field(0, "JAMMING", Some(0));
    let defender = place_suspended(&mut r, 1, "LV5_PLAIN");
    r.attack_digimon(attacker, defender, false);
    resolve_prompts(&mut r);
    assert!(
        field_top_ids(&r, 0).is_empty(),
        "deleted by losing a battle against a Digimon in the battle area"
    );
    assert_eq!(field_top_ids(&r, 1), owned(&["LV5_PLAIN"]));
}
