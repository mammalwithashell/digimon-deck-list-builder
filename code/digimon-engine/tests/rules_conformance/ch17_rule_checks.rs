//! Chapter 17 "Rule Checks" (Comprehensive Rules Ver. 4.3), with §15-4-3-3
//! (an effect triggered by a rule check triggers simultaneously with the other
//! effects of that timing).
//!
//! Every test asserts the RULE, never the engine's current output. A test the
//! engine fails is `#[ignore]`d with the gap id that explains it (discover then
//! pin); see `docs/RUST_ENGINE_GAPS.md` and `qa/rules/cr_obligations_status.yaml`.
//!
//! Fixture cards: `fixtures/common/*.yaml` + `fixtures/ch17/*.yaml`, one
//! synthetic card per file, authored to printed-style text (each clause's
//! `summary:` is that text).
//!
//! Staging: the trash-target tests put the illegal state on the board directly
//! (a face-down card, a Tamer in the breeding area, an Option no effect
//! placed, an excess link card) and then reach a timing when rule checks are
//! possible (§17-1-1) with `rule_check_timing`, the engine's outermost
//! effect-queue drain, which every game action ends in.

use crate::fixtures;
use crate::support::*;
use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{ModifierType, PlayerId};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;

fn ch17() -> DebugRunnerBuilder {
    fixtures::builder(&[fixtures::ch17_cards()])
}

/// Reach a timing when rule checks are possible (§17-1-1): the end of the
/// engine's outermost effect-queue drain, where it runs its rule check.
fn rule_check_timing(r: &mut DebugRunner) {
    r.game.drain_effect_queue();
}

/// The pending prompt is an optional decision owned by `player`.
fn assert_optional_decision(r: &DebugRunner, player: PlayerId, what: &str) {
    let sel = r
        .pending_selection()
        .unwrap_or_else(|| panic!("{what}: expected an optional decision for P{player}"));
    assert_eq!(sel.selecting_player, player, "{what}: {sel:?}");
    assert!(sel.is_optional, "{what}: the decision is optional: {sel:?}");
}

/// Card ids of the link cards plugged into `host`, in link order.
fn linked_ids(r: &DebugRunner, host: PermanentHandle) -> Vec<String> {
    card_ids(
        r,
        &r.game.player(host.player).battle_area[host.index as usize].linked_cards,
    )
}

/// Index in the event log (since `cp`) of the first `Trash` event for
/// `card_id`.
fn trash_event_index(r: &DebugRunner, cp: usize, card_id: &str) -> Option<usize> {
    r.events_since(cp)
        .iter()
        .position(|e| matches!(e, GameEvent::Trash { card_id: id, .. } if id == card_id))
}

/// Index in the event log (since `cp`) of the first effect-sourced memory
/// change whose source is `card_id`.
fn memory_event_index(r: &DebugRunner, cp: usize, card_id: &str) -> Option<usize> {
    r.events_since(cp).iter().position(
        |e| matches!(e, GameEvent::MemoryChange { source_card_id: Some(id), .. } if id == card_id),
    )
}

// ─── §17-1-2-2 ──────────────────────────────────────────────────────────────

/// §17-1-2-2 "Rule checks aren't performed during effect processing.
/// (Example: If an effect reads "[On Play] [When Digivolving] 1 of your
/// opponent's Digimon gets -3000 DP and <Security Attack -1> until the end of
/// their turn," a Digimon whose DP becomes 0 due to getting -3000 DP will be
/// deleted upon a rule check after it gains <Security A. -1>.)"
///
/// The rule's own example (`OP_MINUS3000_SA_MINUS1`), played by P0 against
/// P1's 3000 DP `WOULD_DEL_MAY_MEM`. Its optional "When this Digimon would be
/// deleted, you may gain 1 memory" parks a decision right before the
/// deletion, while the Digimon is still in the battle area: at that moment it
/// is at 0 DP AND already has <Security A. -1> -- the effect finished before
/// the rule check deleted it. (An engine that ran the check between "-3000
/// DP" and "<Security A. -1>" would park the decision with no -1 applied.)
#[test]
fn rule_check_not_during_effect_processing() {
    cite!(
        "17-1-2-2",
        "2e178c5d778b0b24047f2cf0f540dcaa2350111062d4b8799d07617d16333a9c"
    );
    let mut r = ch17()
        .hand(0, &["OP_MINUS3000_SA_MINUS1"])
        .deck(0, &[FILL; 4])
        .deck(1, &[FILL; 4])
        .memory(5)
        .start();
    let target = r.place_on_field(1, "WOULD_DEL_MAY_MEM", Some(0));
    let cp = r.event_checkpoint();
    r.play(0, 0);
    step_prompt(&mut r); // the target: P1's only Digimon

    assert_optional_decision(&r, 1, "WOULD_DEL_MAY_MEM would be deleted");
    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["WOULD_DEL_MAY_MEM"]),
        "right before the deletion it is still in the battle area"
    );
    assert_eq!(r.dp_of(target), Some(0), "-3000 DP: 0 DP");
    assert_eq!(
        r.modifiers()
            .sum(target, ModifierType::SecurityAttackChange),
        -1,
        "it had gained <Security A. -1> before the rule check deleted it"
    );
    accept(&mut r, 1);

    assert!(
        r.pending_selection().is_none(),
        "{:?}",
        r.pending_selection()
    );
    assert!(
        field_top_ids(&r, 1).is_empty(),
        "deleted upon the rule check"
    );
    assert_eq!(trash_ids(&r, 1), owned(&["WOULD_DEL_MAY_MEM"]));
    assert_eq!(
        memory_sources(&r, cp),
        vec![(1, "WOULD_DEL_MAY_MEM".to_string())]
    );
}

// ─── §17-1-2-1 ──────────────────────────────────────────────────────────────

/// §17-1-2-1 "Rule checks aren't performed during rule processing.
/// (Examples: If an immediate-type effect activates during rule check
/// processing and a Digimon is played, a rule check won't yet be performed
/// for that Digimon even if it has 0 DP. A rule check will only be performed
/// after the previous rule check is resolved. If the Digimon has 0 DP upon
/// the new rule check, it will then be deleted.)"
///
/// The rule's own example. P0's 0 DP `WD_PLAY_ZEROMON` is a rule-check
/// deletion target; its immediate-type "When this Digimon would be deleted,
/// you may play 1 [Zeromon] from your hand without paying the cost" activates
/// during that rule check and plays the 0 DP `ZEROMON`. `ZEROMON`'s own
/// optional "would be deleted" decision shows WHEN it is deleted: only once
/// the first rule check has resolved (`WD_PLAY_ZEROMON` already in the trash),
/// by the next one.
#[test]
fn rule_check_not_during_rule_processing() {
    cite!(
        "17-1-2-1",
        "ce73774b8aa59b05a5680a8c8898ba95a219f018f3890e91731cb520f5b04203"
    );
    let mut r = ch17()
        .hand(0, &["ZEROMON"])
        .deck(0, &[FILL; 4])
        .memory(3)
        .start();
    r.place_on_field(0, "WD_PLAY_ZEROMON", Some(0));
    rule_check_timing(&mut r);

    assert_optional_decision(&r, 0, "WD_PLAY_ZEROMON would be deleted");
    accept(&mut r, 0);
    pick_hand_card(&mut r, 0, "ZEROMON");
    assert_eq!(r.hand_size(0), 0, "Zeromon was played");

    assert_optional_decision(&r, 0, "ZEROMON would be deleted");
    assert_eq!(
        field_top_ids(&r, 0),
        owned(&["ZEROMON"]),
        "the 0 DP Zeromon is deleted only after the previous rule check resolved"
    );
    assert_eq!(
        trash_ids(&r, 0),
        owned(&["WD_PLAY_ZEROMON"]),
        "the previous rule check's deletion is complete"
    );
    accept(&mut r, 0);

    assert!(
        r.pending_selection().is_none(),
        "{:?}",
        r.pending_selection()
    );
    assert!(
        field_top_ids(&r, 0).is_empty(),
        "deleted upon the new rule check"
    );
    assert_eq!(trash_ids(&r, 0), owned(&["WD_PLAY_ZEROMON", "ZEROMON"]));
}

// ─── §17-1-3-1-1 ────────────────────────────────────────────────────────────

/// A board for the §17-1-3-1-1 test: P0 plays `OP_MINUS3000_SA_MINUS1`
/// against P1's `OD_MEM` (3000 DP, slot 0) or `LV5_PLAIN` (7000 DP, slot 1).
fn minus3000_board(target_slot: u16) -> (DebugRunner, usize) {
    let mut r = ch17()
        .hand(0, &["OP_MINUS3000_SA_MINUS1"])
        .deck(0, &[FILL; 4])
        .deck(1, &[FILL; 4])
        .memory(5)
        .start();
    r.place_on_field(1, "OD_MEM", Some(0));
    r.place_on_field(1, "LV5_PLAIN", Some(0));
    let cp = r.event_checkpoint();
    r.play(0, 0);
    pick_field(&mut r, target_slot);
    (r, cp)
}

/// §17-1-3-1-1 (a deletion target) "A Digimon placed in the battle area whose
/// DP is 0"
///
/// P1's `OD_MEM` (3000 DP) gets -3000 DP: at 0 DP it is DELETED (its "[On
/// Deletion] Gain 1 memory" activates). Control: `LV5_PLAIN` (7000 DP) gets
/// -3000 DP and stays at 4000 DP.
#[test]
fn zero_dp_digimon_in_battle_area_is_deleted() {
    cite!(
        "17-1-3-1-1",
        "02e57104aeaae9b771c6b28fd8c9a804176f67148a3e47868caa3cb5000597cc"
    );
    let (mut r, cp) = minus3000_board(0);
    resolve_prompts(&mut r);
    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["LV5_PLAIN"]),
        "the 0 DP Digimon left the battle area"
    );
    assert_eq!(trash_ids(&r, 1), owned(&["OD_MEM"]));
    assert_eq!(
        memory_sources(&r, cp),
        vec![(1, "OD_MEM".to_string())],
        "it was deleted: its [On Deletion] activated"
    );

    // Control: DP above 0.
    let (mut r, _) = minus3000_board(1);
    resolve_prompts(&mut r);
    assert_eq!(field_top_ids(&r, 1), owned(&["OD_MEM", "LV5_PLAIN"]));
    let big = r.perm_handle(1, 1);
    assert_eq!(r.dp_of(big), Some(4000));
}

// ─── §17-1-3-2-1 ────────────────────────────────────────────────────────────

/// §17-1-3-2-1 (a trash target) "Digimon without DP placed in the battle
/// area"
///
/// P0's `NO_DP_DIGIMON` (printed without DP; "[On Deletion] Gain 1 memory") is
/// in the battle area at a rule check: it is TRASHED, not deleted, so its [On
/// Deletion] doesn't activate. Control: `FILL` (1000 DP) next to it stays.
#[test]
fn digimon_without_dp_in_battle_area_is_trashed_not_deleted() {
    cite!(
        "17-1-3-2-1",
        "3438d2e26bcb3d2fcc869347ce2d006a7bb5b27407e985ec5cce57505c1dd951"
    );
    let mut r = ch17().deck(0, &[FILL; 4]).memory(3).start();
    r.place_on_field(0, "NO_DP_DIGIMON", Some(0));
    r.place_on_field(0, FILL, Some(0));
    let cp = r.event_checkpoint();
    rule_check_timing(&mut r);

    assert!(
        r.pending_selection().is_none(),
        "{:?}",
        r.pending_selection()
    );
    assert_eq!(field_top_ids(&r, 0), owned(&[FILL]), "only the DP-less one");
    assert_eq!(trash_ids(&r, 0), owned(&["NO_DP_DIGIMON"]));
    assert!(
        memory_sources(&r, cp).is_empty(),
        "trashed, not deleted: no [On Deletion]: {:?}",
        memory_sources(&r, cp)
    );
}

// ─── §17-1-3-2-2 / §4-28-1 ──────────────────────────────────────────────────

/// §17-1-3-2-2 (a trash target) "Option cards placed in the battle area
/// (except Option cards placed in the battle area by an effect)"
/// §4-28-1 "An "Option card in the battle area" refers to an Option card that
/// was placed in the battle area by an effect. ..."
///
/// - `OPT_PLAIN` in P0's battle area, not placed there by an effect (staged
///   directly, as an Option left exposed when the cards above it leave): the
///   rule check trashes it.
/// - Control: P0 uses `MAIN_PLACE_SELF` ("[Main] Place this card in the
///   battle area."): placed by an effect, it stays through the rule checks.
#[test]
fn option_in_battle_area_not_placed_by_effect_is_trashed() {
    cite!(
        "17-1-3-2-2",
        "fe40a6ca4ea9741d5ba7d72125790687e46c481f6e3ec707f2826ad237fcce26"
    );
    // Not placed by an effect.
    let mut r = ch17().deck(0, &[FILL; 4]).memory(3).start();
    r.place_on_field(0, FILL, Some(0));
    r.place_on_field(0, "OPT_PLAIN", Some(0));
    rule_check_timing(&mut r);
    assert!(
        r.pending_selection().is_none(),
        "{:?}",
        r.pending_selection()
    );
    assert_eq!(field_top_ids(&r, 0), owned(&[FILL]));
    assert_eq!(trash_ids(&r, 0), owned(&["OPT_PLAIN"]));

    // Placed by an effect.
    let mut r = ch17()
        .hand(0, &["MAIN_PLACE_SELF"])
        .deck(0, &[FILL; 4])
        .memory(3)
        .start();
    r.place_on_field(0, FILL, Some(0)); // a red Digimon to use the red Option
    use_option(&mut r, 0, "MAIN_PLACE_SELF");
    resolve_prompts(&mut r);
    rule_check_timing(&mut r);
    assert_eq!(
        field_top_ids(&r, 0),
        owned(&[FILL, "MAIN_PLACE_SELF"]),
        "placed in the battle area by an effect: not a rule-check target"
    );
    assert!(trash_ids(&r, 0).is_empty(), "{:?}", trash_ids(&r, 0));
}

// ─── §17-1-3-2-3 ────────────────────────────────────────────────────────────

/// §17-1-3-2-3 (a trash target) "Cards other than Digimon placed in the
/// breeding area (except cards placed in the breeding area by an effect)"
///
/// A Tamer (`TAMER_PLAIN`) in P0's breeding area, not placed there by an
/// effect, is trashed at a rule check. Control: a Digimon (`FILL`) there
/// stays.
///
/// Red: `Game::run_state_based_rules_check` scans only the battle areas; the
/// Tamer stays in the breeding area (`G-ENGINE-RULE-CHECK-TARGETS-INCOMPLETE`).
#[test]
fn non_digimon_in_breeding_area_is_trashed() {
    cite!(
        "17-1-3-2-3",
        "977d5c310b19401d2946ed0ff9ac79ad8aec8ed1ab8c15d5d21d5e1c7ad73975"
    );
    let mut r = ch17().deck(0, &[FILL; 4]).memory(3).start();
    r.place_in_breeding(0, "TAMER_PLAIN");
    rule_check_timing(&mut r);
    assert!(
        r.game.player(0).breeding_area.is_none(),
        "the Tamer left the breeding area"
    );
    assert_eq!(trash_ids(&r, 0), owned(&["TAMER_PLAIN"]));

    // Control: a Digimon.
    let mut r = ch17().deck(0, &[FILL; 4]).memory(3).start();
    r.place_in_breeding(0, FILL);
    rule_check_timing(&mut r);
    assert!(r.game.player(0).breeding_area.is_some());
    assert!(trash_ids(&r, 0).is_empty());
}

// ─── §17-1-3-2-4 ────────────────────────────────────────────────────────────

/// §17-1-3-2-4 (a trash target) "Face-down cards placed on the field"
///
/// A face-down card (`FILL`) placed on its own in P0's battle area is trashed
/// at a rule check. Control: the face-down card stacked under P0's
/// `TAMER_PLAIN` is a stacked card of that Tamer, not a card placed on the
/// field, and stays (DCGO `AutoProcessing.IsPermanentFaceDown` reads only a
/// permanent's top card).
///
/// Red: the rule check never reads `face_down`; the face-down card stays in
/// the battle area as a 1000 DP Digimon (`G-ENGINE-RULE-CHECK-TARGETS-INCOMPLETE`).
#[test]
fn face_down_card_on_field_is_trashed() {
    cite!(
        "17-1-3-2-4",
        "3f8b095b8a7c5d7b54ecb43f897639cfbd959c313c361a00299d86a7b1bc0131"
    );
    let mut r = ch17().deck(0, &[FILL; 4]).memory(3).start();
    let tamer = r.place_on_field(0, "TAMER_PLAIN", Some(0));
    attach_face_down(&mut r, tamer, FILL);
    let face_down = r.place_on_field(0, FILL, Some(0));
    r.game.players[0].battle_area[face_down.index as usize].card_sources[0].face_down = true;
    rule_check_timing(&mut r);

    assert_eq!(
        field_top_ids(&r, 0),
        owned(&["TAMER_PLAIN"]),
        "the face-down card on the field was trashed"
    );
    assert_eq!(trash_ids(&r, 0), owned(&[FILL]));
    assert_eq!(
        r.game.players[0].battle_area[tamer.index as usize]
            .card_sources
            .len(),
        2,
        "control: the face-down card under the Tamer stays"
    );
}

// ─── §17-1-3-2-5 ────────────────────────────────────────────────────────────

/// A board for the §17-1-3-2-5 test: P0's `LINK_HOST_LV4` (no <Link +N>, so
/// its link limit is 1, §4-9-5) holds the link cards `LINK_A` and `LINK_B`.
fn excess_link_board() -> (DebugRunner, PermanentHandle) {
    let mut r = ch17().deck(0, &[FILL; 4]).memory(3).start();
    let host = r.place_on_field(0, "LINK_HOST_LV4", Some(0));
    r.push_linked_owned(host, "LINK_A", 0);
    r.push_linked_owned(host, "LINK_B", 0);
    (r, host)
}

/// Finish the excess-link choice: answer any follow-up prompt (e.g. a
/// confirm step) with its first legal option until nothing is pending.
fn settle_link_choice(r: &mut DebugRunner) {
    for _ in 0..8 {
        let Some(sel) = r.pending_selection() else {
            return;
        };
        let (player, action) = (sel.selecting_player, sel.valid_action_ids[0]);
        r.execute_action(player, action)
            .unwrap_or_else(|e| panic!("follow-up pick rejected: {e:?}"));
    }
    panic!("the excess-link choice did not settle after 8 picks");
}

/// §17-1-3-2-5 (a trash target) "Link cards for a Digimon that has exceeded
/// the link limit (only the cards that exceed link limit are trashed)"
/// Official Q&A Q6370 (BT25-075): on losing <Link +N> "its maximum number of
/// link cards will become 1, and any link cards that exceed the maximum will
/// be trashed upon the rule check timing. The link cards to trash are chosen
/// by the player."
///
/// P0's host has 2 link cards and a link limit of 1. At the rule check, P0 is
/// asked which link card to trash (no automatic pick, e.g. the most recent):
/// whichever option P0 takes, exactly 1 link card is trashed, and across the
/// options P0 can keep either one.
///
/// Red for two independent reasons: nothing is pending after the rule check
/// -- `run_state_based_rules_check` has no link-card step at all -- and the
/// engine's base link limit is 5, not 1 (`G-ENGINE-LINK-MAX-BASE`), so it
/// would not see this host as over its limit either.
/// `excess_link_cards_over_any_limit_prompt_before_trash` pins the missing
/// step alone. DCGO agrees with Q6370:
/// `AutoProcessing.DigimonLackLinkMaxCountProcess` calls
/// `Permanent.RemoveLinkedCard(null, excess)`, a `SelectCardEffect` over the
/// link cards with `selectPlayer: TopCard.Owner`.
#[test]
fn excess_link_cards_trashed_player_chooses_which() {
    cite!(
        "17-1-3-2-5",
        "42c41c1ce213cb5e7b7f877a9d012fbe936ffa3580b5c7c7af917c437a7fc518"
    );
    let (mut r, host) = excess_link_board();
    rule_check_timing(&mut r);
    let options = {
        let sel = r
            .pending_selection()
            .expect("P0 chooses which link card exceeds the limit (Q6370)");
        assert_eq!(sel.selecting_player, 0, "{sel:?}");
        sel.valid_action_ids.clone()
    };
    assert!(
        options.len() >= 2,
        "both link cards are choosable: {options:?}"
    );
    assert_eq!(
        linked_ids(&r, host),
        owned(&["LINK_A", "LINK_B"]),
        "nothing is trashed before the choice"
    );

    let mut kept = Vec::new();
    for action in options {
        let (mut r, host) = excess_link_board();
        rule_check_timing(&mut r);
        r.execute_action(0, action)
            .unwrap_or_else(|e| panic!("link pick rejected: {e:?}"));
        settle_link_choice(&mut r);
        let linked = linked_ids(&r, host);
        assert_eq!(
            linked.len(),
            1,
            "only the excess card is trashed: {linked:?}"
        );
        assert_eq!(trash_ids(&r, 0).len(), 1, "{:?}", trash_ids(&r, 0));
        kept.push(linked[0].clone());
    }
    kept.sort();
    kept.dedup();
    assert_eq!(
        kept,
        owned(&["LINK_A", "LINK_B"]),
        "P0 can keep either link card"
    );
}

/// §17-1-3-2-5, independent of the base link limit's value: P0's
/// `LINK_HOST_LV4` holds 6 link cards (`LINK_A` x3, `LINK_B` x3), over both the
/// printed base limit of 1 (§4-9-5) and the engine's current base of 5
/// (`G-ENGINE-LINK-MAX-BASE`). At the rule check P0 must be asked which link
/// cards to trash (Q6370), and nothing may be trashed before that choice.
///
/// Red: nothing is pending after the rule check --
/// `run_state_based_rules_check` has no link-card step
/// (`G-ENGINE-RULE-CHECK-TARGETS-INCOMPLETE`).
#[test]
fn excess_link_cards_over_any_limit_prompt_before_trash() {
    cite!(
        "17-1-3-2-5",
        "42c41c1ce213cb5e7b7f877a9d012fbe936ffa3580b5c7c7af917c437a7fc518"
    );
    let mut r = ch17().deck(0, &[FILL; 4]).memory(3).start();
    let host = r.place_on_field(0, "LINK_HOST_LV4", Some(0));
    for card in ["LINK_A", "LINK_B", "LINK_A", "LINK_B", "LINK_A", "LINK_B"] {
        r.push_linked_owned(host, card, 0);
    }
    rule_check_timing(&mut r);
    let sel = r
        .pending_selection()
        .expect("P0 chooses which link cards exceed the limit (Q6370)");
    assert_eq!(sel.selecting_player, 0, "{sel:?}");
    assert_eq!(
        linked_ids(&r, host).len(),
        6,
        "nothing trashed before the choice"
    );
    assert!(trash_ids(&r, 0).is_empty(), "{:?}", trash_ids(&r, 0));
}

// ─── §17-1-3-2-6 ────────────────────────────────────────────────────────────

/// §17-1-3-2-6 (a trash target) "Link cards that don't meet the link
/// requirements"
///
/// `LINK_A` and `LINK_B` have the link requirement "1 level 4 Digimon".
/// `LINK_A` is linked to P0's level 5 `LV5_PLAIN` (requirement not met): the
/// rule check trashes it. Control: `LINK_B` linked to the level 4
/// `LINK_HOST_LV4` stays.
///
/// Red: the rule check never re-checks link requirements; `LINK_A` stays
/// linked to the level 5 host (`G-ENGINE-RULE-CHECK-TARGETS-INCOMPLETE`).
#[test]
fn link_card_failing_requirement_is_trashed() {
    cite!(
        "17-1-3-2-6",
        "4d0487af2812eb49a5d5472851d3a6f002ea5106d06bae3f863caf8714c2eefb"
    );
    let mut r = ch17().deck(0, &[FILL; 4]).memory(3).start();
    let ok = r.place_on_field(0, "LINK_HOST_LV4", Some(0));
    let bad = r.place_on_field(0, "LV5_PLAIN", Some(0));
    r.push_linked_owned(ok, "LINK_B", 0);
    r.push_linked_owned(bad, "LINK_A", 0);
    rule_check_timing(&mut r);

    assert!(
        r.pending_selection().is_none(),
        "{:?}",
        r.pending_selection()
    );
    assert!(
        linked_ids(&r, bad).is_empty(),
        "the level 5 host doesn't meet LINK_A's requirement"
    );
    assert_eq!(trash_ids(&r, 0), owned(&["LINK_A"]));
    assert_eq!(linked_ids(&r, ok), owned(&["LINK_B"]), "control");
    assert_eq!(field_top_ids(&r, 0), owned(&["LINK_HOST_LV4", "LV5_PLAIN"]));
}

// ─── §17-1-3-2-7 ────────────────────────────────────────────────────────────

/// §17-1-3-2-7 (a trash target) "Linked cards in a card category other than
/// those specified in the notes for <Link>"
///
/// The closest stageable case: a Tamer card (`TAMER_PLAIN`, which has no
/// [Link] at all, so no <Link> note names its category) linked to P0's
/// `LINK_HOST_LV4` is trashed at the rule check. Control: the Digimon link
/// card `LINK_B` on a second `LINK_HOST_LV4` stays (its category, Digimon, is
/// the one its link requirement names). A Tamer has no link requirements, so
/// this board also falls under §17-1-3-2-6; a purer witness needs a <Link>
/// note restricting categories, which no fixture models yet.
///
/// Red: `run_state_based_rules_check` has no category step; the Tamer stays
/// linked (`G-ENGINE-RULE-CHECK-TARGETS-INCOMPLETE`).
#[test]
fn linked_card_of_unlisted_category_is_trashed() {
    cite!(
        "17-1-3-2-7",
        "8ae92f0207a8efec74343ffeab23eb9ba401473c30810a609572336931e98e04"
    );
    let mut r = ch17().deck(0, &[FILL; 4]).memory(3).start();
    let bad = r.place_on_field(0, "LINK_HOST_LV4", Some(0));
    let ok = r.place_on_field(0, "LINK_HOST_LV4", Some(0));
    r.push_linked_owned(bad, "TAMER_PLAIN", 0);
    r.push_linked_owned(ok, "LINK_B", 0);
    rule_check_timing(&mut r);

    assert!(
        r.pending_selection().is_none(),
        "{:?}",
        r.pending_selection()
    );
    assert!(
        linked_ids(&r, bad).is_empty(),
        "the linked Tamer card was trashed"
    );
    assert_eq!(trash_ids(&r, 0), owned(&["TAMER_PLAIN"]));
    assert_eq!(linked_ids(&r, ok), owned(&["LINK_B"]), "control");
}

// ─── §15-4-3-3 ──────────────────────────────────────────────────────────────

/// §15-4-3-3 "When an effect is triggered by a rule check, it triggers
/// simultaneously with other effects that triggered at that timing.
/// (Example: An [On Deletion] effect on a Digimon that was deleted by its DP
/// becoming 0 at the start of your turn will trigger simultaneously with
/// [Start of Your Turn] effects.)"
///
/// The rule's own example. P0 has `OD_MEM` (3000 DP, "[On Deletion] Gain 1
/// memory") and the Tamer `SOYT_MEM` ("[Start of Your Turn] Gain 1 memory");
/// P1 has `OPP_TURN_MINUS3000` ("[Opponent's Turn] All of your opponent's
/// Digimon get -3000 DP"), placed during P1's turn. When P0's next turn
/// starts, `OD_MEM` is at 0 DP and is deleted by the rule check: its [On
/// Deletion] and the [Start of Your Turn] effect trigger simultaneously, so P0
/// gets ONE ordering choice holding both.
#[test]
fn rule_check_deletion_trigger_is_simultaneous_with_timing_triggers() {
    cite!(
        "15-4-3-3",
        "efae2fbeb8e4b5a4143c869cd417e1b33da48f07c2603d922ab1acc947401e3f"
    );
    let mut r = ch17()
        .deck(0, &[FILL; 6])
        .deck(1, &[FILL; 6])
        .memory(3)
        .start();
    assert_eq!(r.turn_player(), 0);
    r.place_on_field(0, "OD_MEM", Some(0));
    r.place_on_field(0, "SOYT_MEM", Some(0));
    r.pass_turn();
    resolve_prompts(&mut r);
    assert_eq!(r.turn_player(), 1, "P1's turn");
    r.place_on_field(1, "OPP_TURN_MINUS3000", Some(0));
    assert_eq!(
        field_top_ids(&r, 0),
        owned(&["OD_MEM", "SOYT_MEM"]),
        "the aura is off during P1's own turn"
    );
    let cp = r.event_checkpoint();
    r.pass_turn();

    assert_eq!(r.turn_player(), 0, "P0's turn started");
    assert!(
        !field_top_ids(&r, 0).contains(&"OD_MEM".to_string()),
        "0 DP at the start of the turn: deleted by the rule check"
    );
    let choices = order_choices(&r, 0);
    let mut cards: Vec<&str> = choices.iter().map(|l| branch_card(l)).collect();
    cards.sort();
    assert_eq!(
        cards,
        vec!["OD_MEM", "SOYT_MEM"],
        "the [On Deletion] and the [Start of Your Turn] effect trigger simultaneously: \
         {choices:?}"
    );
    assert!(
        memory_sources(&r, cp).is_empty(),
        "nothing activated before the choice: {:?}",
        memory_sources(&r, cp)
    );
    choose(&mut r, "SOYT_MEM");
    resolve_prompts(&mut r);
    assert_eq!(
        memory_sources(&r, cp),
        vec![(0, "SOYT_MEM".to_string()), (0, "OD_MEM".to_string())],
        "both activated, in the order P0 chose"
    );
}

// ─── §17-1-3 ────────────────────────────────────────────────────────────────

/// §17-1-3 "The respective processing is performed simultaneously for the
/// following circumstances during timings when rule checks are possible."
///
/// - Two 0 DP Digimon with "[On Deletion] Gain 1 memory" (`ZERO_OD_MEM` x2):
///   both are deleted by the one rule check, and their [On Deletion] effects
///   are pending together -- one ordering choice, made with both already in
///   the trash.
/// - P0's battle area holds an Option no effect placed there (`OPT_PLAIN`, a
/// trash target, §17-1-3-2-2) and a 0 DP Digimon (`ZERO_OD_MEM`, "[On
/// Deletion] Gain 1 memory", a deletion target, §17-1-3-1-1). One rule check
/// removes both; the [On Deletion] activates once, after BOTH removals (it
/// can't interleave between them). Both slot orders, so neither the engine's
/// scan order nor its removal order can hide an interleave.
///
/// Red: `Game::run_state_based_rules_check` removes its targets one at a time
/// (descending slot), each 0 DP Digimon through its own one-element
/// `delete_permanent_with_effects` batch, which drains that Digimon's [On
/// Deletion] before the next target is touched. Two 0 DP Digimon: `Trash`,
/// its memory, `Trash`, its memory -- no ordering choice. Option + Digimon:
/// the Digimon's [On Deletion] resolves before the Option is trashed
/// (`G-ENGINE-RULE-CHECK-TARGETS-NOT-SIMULTANEOUS`).
#[test]
fn all_rule_check_targets_processed_simultaneously() {
    cite!(
        "17-1-3",
        "258e62c6c447db35c1ba047e10bbd202a7280b501eaa465b1425850bff9b7caa"
    );
    // Two deletion targets: deleted simultaneously, so their [On Deletion]
    // effects trigger simultaneously (§15-4-3-2) -- one ordering choice, made
    // with both Digimon already in the trash.
    let mut r = ch17().deck(0, &[FILL; 4]).memory(3).start();
    r.place_on_field(0, "ZERO_OD_MEM", Some(0));
    r.place_on_field(0, "ZERO_OD_MEM", Some(0));
    let cp = r.event_checkpoint();
    rule_check_timing(&mut r);
    assert!(
        field_top_ids(&r, 0).is_empty(),
        "both 0 DP Digimon deleted by the one rule check"
    );
    assert!(
        memory_sources(&r, cp).is_empty(),
        "no [On Deletion] activated before the other deletion / the ordering choice: {:?}",
        r.events_since(cp)
    );
    let choices = order_choices(&r, 0);
    assert_eq!(
        choices.len(),
        2,
        "both [On Deletion] effects pending together: {choices:?}"
    );
    choose(&mut r, &choices[0].clone());
    resolve_prompts(&mut r);
    assert_eq!(
        memory_sources(&r, cp),
        vec![
            (0, "ZERO_OD_MEM".to_string()),
            (0, "ZERO_OD_MEM".to_string())
        ]
    );

    // An Option trash target and a Digimon deletion target.
    for layout in [["OPT_PLAIN", "ZERO_OD_MEM"], ["ZERO_OD_MEM", "OPT_PLAIN"]] {
        let mut r = ch17().deck(0, &[FILL; 4]).memory(3).start();
        for card in layout {
            r.place_on_field(0, card, Some(0));
        }
        let cp = r.event_checkpoint();
        rule_check_timing(&mut r);
        resolve_prompts(&mut r);

        assert!(
            field_top_ids(&r, 0).is_empty(),
            "{layout:?}: both removed by the one rule check"
        );
        assert_eq!(trash_ids(&r, 0), owned(&["OPT_PLAIN", "ZERO_OD_MEM"]));
        assert_eq!(
            memory_sources(&r, cp),
            vec![(0, "ZERO_OD_MEM".to_string())],
            "{layout:?}: the [On Deletion] activated once"
        );
        let od = memory_event_index(&r, cp, "ZERO_OD_MEM").expect("[On Deletion]");
        let option_trashed = trash_event_index(&r, cp, "OPT_PLAIN").expect("Option trashed");
        let digimon_trashed = trash_event_index(&r, cp, "ZERO_OD_MEM").expect("Digimon deleted");
        assert!(
            option_trashed < od && digimon_trashed < od,
            "{layout:?}: the [On Deletion] activated after both removals: {:?}",
            r.events_since(cp)
        );
    }
}
