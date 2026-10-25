//! Chapter 16 (Comprehensive Rules Ver. 4.3), the replacement keywords: the
//! immediate-type "when ... would ..." keyword effects `<Decoy>` (§16-18),
//! `<Armor Purge>` (§16-19), `<Evade>` (§16-22), `<Barrier>` (§16-25),
//! `<Partition>` (§16-29), `<Scapegoat>` (§16-32), `<Decode>` (§16-36) and
//! `<Fragment>` (§16-37), with the keyword generics §16-1..§16-3 (which in
//! Ver. 4.3 replace the deleted per-keyword "multiple instances" rules) and
//! the §15-8-5 immediate-type ordering they rely on. `<Guard>` (§16-45) and
//! `<Detach>` (§16-46) are in `ch16_new_keywords`.
//!
//! Every test asserts the RULE, never the engine's current output. A test the
//! engine fails is `#[ignore]`d with the gap id that explains it (discover then
//! pin); see `docs/RUST_ENGINE_GAPS.md` and `qa/rules/cr_obligations_status.yaml`.
//!
//! Per keyword: `<kw>_kind_matches_rule` checks the observable consequence of
//! the §16-N-2 immediate type (§15-8-5-2: it is offered BEFORE its cause
//! completes -- the threatened card is still in the battle area and nothing is
//! in the trash -- and a prevention means the card never reaches the trash);
//! `<kw>_optionality_matches_rule` the §16-N-3 split (an optional processing
//! condition whose execution makes the rest mandatory, or plain optional
//! processing); `<kw>_timing_matches_rule` the "triggers when ..." condition;
//! plus one test per behavioural sub-rule.
//!
//! Engine reality: `Keyword::{Decoy, ArmorPurge, Evade, Barrier, Partition,
//! Fragment}` are granted with `kind: grant_keyword` (or `kind: partition`);
//! `<Scapegoat>` is not DSL-grantable and rides on the printed-keyword parse
//! (`scapegoat_card`); `<Decode>` has no keyword-generic body and is authored
//! with the production `replacement` idiom (EX12-031). Each fixture card's
//! header names the idiom it uses.
//!
//! Fixture cards: `fixtures/common/*.yaml` + `fixtures/ch16_repl/*.yaml` (plus
//! `LV4_PLAIN` / `MAIN_DEL1` from `ch15_08`, `MAIN_BOUNCE1` /
//! `MAIN_DEL_OWN_MEM` from `ch16_new`, `TAMER_PLAIN` from `ch17` and
//! `MAIN_MINUS3000` from `ch18`), one synthetic card per file, authored to
//! printed-style text (each clause's `summary:` is that text).
//!
//! Security stacks are listed bottom first: the LAST card of `.security(..)`
//! is the top card. Stacks (`place_stack`) are listed bottom first too: the
//! LAST card is the top card.

use crate::fixtures;
use crate::support::*;
use digimon_engine::action::space::ATTACK_START;
use digimon_engine::card_data::{parse_printed_keywords, CardData};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{Keyword, PlayerId};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

fn ch16_repl() -> DebugRunnerBuilder {
    fixtures::builder(&[
        fixtures::ch15_08_cards(),
        fixtures::ch16_new_cards(),
        fixtures::ch17_cards(),
        fixtures::ch18_cards(),
        fixtures::ch16_repl_cards(),
    ])
}

/// P0 (memory 5, hand `hand0`) to move in its main phase; both players have
/// 6 deck cards; P0's and P1's security stacks are `sec0` / `sec1`.
fn board_with(
    builder: DebugRunnerBuilder,
    hand0: &[&str],
    sec0: &[&str],
    sec1: &[&str],
) -> DebugRunner {
    builder
        .hand(0, hand0)
        .deck(0, &[FILL; 6])
        .deck(1, &[FILL; 6])
        .security(0, sec0)
        .security(1, sec1)
        .memory(5)
        .start()
}

fn board(hand0: &[&str]) -> DebugRunner {
    board_with(ch16_repl(), hand0, &[FILL; 3], &[FILL; 3])
}

/// Put the red `TAMER_PLAIN` (no text) on P0's field: it meets the color
/// requirement of P0's red Options without being a Digimon any effect here
/// can choose. Placed LAST, so it never shifts a staged slot.
fn red_anchor(r: &mut DebugRunner) {
    r.place_on_field(0, "TAMER_PLAIN", Some(0));
}

/// A decision the engine offered.
#[derive(Debug, Clone, PartialEq)]
struct Prompt {
    player: PlayerId,
    kind: SelectionKind,
    optional: bool,
    labels: Vec<String>,
}

fn current_prompt(r: &DebugRunner) -> Option<Prompt> {
    r.pending_selection().map(|sel| Prompt {
        player: sel.selecting_player,
        kind: sel.kind,
        optional: sel.is_optional,
        labels: sel
            .effect_choices
            .iter()
            .flatten()
            .map(|c| c.label.clone())
            .collect(),
    })
}

/// Settle every pending decision: DECLINE every optional prompt, and take the
/// only option of every other prompt. Returns the decisions offered, in order.
fn settle_declining(r: &mut DebugRunner) -> Vec<Prompt> {
    let mut seen = Vec::new();
    for _ in 0..24 {
        let Some(p) = current_prompt(r) else {
            return seen;
        };
        seen.push(p.clone());
        if p.optional {
            r.decline_optional_trigger()
                .unwrap_or_else(|e| panic!("decline {p:?}: {e:?}"));
        } else {
            step_prompt(r);
        }
    }
    panic!("prompts did not settle after 24 decisions");
}

/// Settle decisions (as `settle_declining`) until one matching `want` is
/// pending. Returns whether it is.
fn advance_to(r: &mut DebugRunner, want: impl Fn(&Prompt) -> bool) -> bool {
    for _ in 0..24 {
        let Some(p) = current_prompt(r) else {
            return false;
        };
        if want(&p) {
            return true;
        }
        if p.optional {
            r.decline_optional_trigger()
                .unwrap_or_else(|e| panic!("decline {p:?}: {e:?}"));
        } else {
            step_prompt(r);
        }
    }
    panic!("no matching prompt after 24 decisions");
}

/// An optional prompt owned by `player` (an optional replacement / trigger).
fn optional_for(player: PlayerId) -> impl Fn(&Prompt) -> bool {
    move |p: &Prompt| p.optional && p.player == player
}

fn offered_to(prompts: &[Prompt], player: PlayerId) -> bool {
    prompts.iter().any(|p| optional_for(player)(p))
}

/// Use the Option `option` from P0's hand and pick P1's battle-area `slot`
/// as its target.
fn use_on_opponent_slot(r: &mut DebugRunner, option: &str, slot: u16) {
    use_option(r, 0, option);
    pick_field(r, slot);
}

/// Use P0's `MAIN_DEL_OWN_MEM` ("[Main] Delete 1 of your Digimon. Then, gain 1
/// memory.") on P0's own battle-area `slot`.
fn delete_own(r: &mut DebugRunner, slot: u16) {
    use_option(r, 0, "MAIN_DEL_OWN_MEM");
    pick_field(r, slot);
}

/// Whether `card_id` was trashed by `player` since `cp`.
fn trashed_since(r: &DebugRunner, cp: usize, player: PlayerId, card_id: &str) -> bool {
    r.events_since(cp).iter().any(|e| {
        matches!(e, GameEvent::Trash { player: p, card_id: c, .. } if *p == player && c == card_id)
    })
}

/// Card ids in `player`'s security stack, bottom first (the last is the top).
fn security_ids(r: &DebugRunner, player: PlayerId) -> Vec<String> {
    card_ids(r, &r.game.player(player).security)
}

/// The cards of the stack at `h`, bottom first (the last is the top card).
fn stack_ids(r: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    card_ids(
        r,
        &r.game.player(h.player).battle_area[h.index as usize].card_sources,
    )
}

fn sorted(mut v: Vec<String>) -> Vec<String> {
    v.sort();
    v
}

/// Have P0's `LV5_PLAIN` (7000 DP) attack `defender` (suspended first).
fn attack_with_lv5(r: &mut DebugRunner, defender: PermanentHandle) {
    set_suspended(r, defender, true);
    let attacker = r.place_on_field(0, "LV5_PLAIN", Some(0));
    r.attack_digimon(attacker, defender, false);
}

/// Look up the current legal row for a physical source, without old ID geometry.
fn source_action(r: &DebugRunner, host: PermanentHandle, index: usize) -> Option<u16> {
    use digimon_engine::{action::build_action_mask, owned_card_choice::{CardChoiceOperation, CardChoiceZone}};
    let pending = r.game.pending_selection.as_ref().expect("a source pick");
    let owner = pending.card_choice.as_ref().expect("source pick owns its physical page");
    let row = owner.view(&r.game, pending.selecting_player).rows.into_iter().find(|row| {
        matches!(row.operation, CardChoiceOperation::CommitCard)
            && matches!(row.zone, Some(CardChoiceZone::BattleSource | CardChoiceZone::BreedingSource))
            && row.owner == Some(host.player) && row.carrier_slot == Some(host.index)
            && row.position.as_deref().and_then(|p| p.parse::<usize>().ok()) == Some(index)
    })?;
    assert_eq!(build_action_mask(&r.game, pending.selecting_player)[row.action_id as usize], 1.0);
    assert_eq!(build_action_mask(&r.game, 1-pending.selecting_player)[row.action_id as usize], 0.0);
    Some(row.action_id)
}

/// Pick the exact currently resident digivolution card `card_id` under `h`.
fn pick_source(r: &mut DebugRunner, h: PermanentHandle, card_id: &str) {
    let index = stack_ids(r, h).iter().position(|c| c == card_id)
        .unwrap_or_else(|| panic!("{card_id} is not in the stack of {h:?}"));
    let selected = r.game.player(h.player).battle_area[h.index as usize].card_sources[index].handle();
    let action = source_action(r, h, index).unwrap_or_else(|| panic!("{card_id} (source {index}) is not in the legal physical menu"));
    let player = r.pending_selection().unwrap().selecting_player;
    // The lookup is tied to a real source, rather than accepting an alias that
    // can point at a different row after the candidate pool contracts.
    assert_eq!(r.game.card_data_for_handle(selected).unwrap().card_id, card_id);
    r.execute_action(player, action).unwrap_or_else(|e| panic!("source pick {card_id} rejected: {e:?}"));
}

// ─── §16-18 <Decoy> ─────────────────────────────────────────────────────────

/// `owner` has `DECOY` (<Decoy (Black)>, slot 0), the black `BLACK_LV3`
/// (slot 1, specified) and the red `FILL` (slot 2, not specified); P0 holds
/// `hand0`.
fn decoy_board(owner: PlayerId, hand0: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = board(hand0);
    r.place_on_field(owner, "DECOY", Some(0));
    let black = r.place_on_field(owner, "BLACK_LV3", Some(0));
    r.place_on_field(owner, FILL, Some(0));
    red_anchor(&mut r);
    (r, black)
}

/// §16-18-1 "<Decoy> is a keyword effect. When another of your specified
/// Digimon would be deleted by an opponent's effect, by deleting the Digimon
/// with this effect, <Decoy> prevents the Digimon specified by this effect
/// from being deleted. This is shown on cards using text such as <Decoy
/// (Black)>."
/// §16-18-2 "<Decoy> is an immediate-type effect that triggers when another of
/// your Digimon specified by this effect would be deleted by an opponent's
/// effect ..."
///
/// - By an opponent's effect, a specified Digimon: P0's `MAIN_DEL1` targets
///   P1's black `BLACK_LV3`. P1 is offered <Decoy>; accepting deletes `DECOY`
///   and `BLACK_LV3` stays.
/// - Not specified: P0's `MAIN_DEL1` targets P1's red `FILL`: no <Decoy>.
/// - By its owner's own effect: P0's `MAIN_DEL_OWN_MEM` deletes P0's own
///   `BLACK_LV3` next to P0's `DECOY`: no <Decoy>.
/// - In battle: P0's `LV5_PLAIN` attacks P1's suspended `BLACK_LV3`: no
///   <Decoy>.
/// - Not one of YOUR Digimon: P0's `MAIN_DEL_OWN_MEM` deletes P0's own black
///   `BLACK_LV3` while P1 has `DECOY`: P1 is not offered <Decoy>.
#[test]
fn decoy_only_for_specified_digimon_deleted_by_opponent_effect() {
    cite!(
        "16-18-1",
        "803a4227a8a50ce95ff752ac8f06d1cc6f59f6a31d5f6a377ec0dbba32f2ec09"
    );
    cite!(
        "16-18-2",
        "f33f5d7a2ab9065b3ee0f4865e82b3d5188c93c39700feb6a753f35d52928f20"
    );
    // By an opponent's effect, specified.
    let (mut r, _) = decoy_board(1, &["MAIN_DEL1"]);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 1);
    assert!(advance_to(&mut r, optional_for(1)), "P1 is offered <Decoy>");
    accept(&mut r, 1);
    settle_declining(&mut r);
    assert_eq!(field_top_ids(&r, 1), owned(&["BLACK_LV3", FILL]));
    assert_eq!(trash_ids(&r, 1), owned(&["DECOY"]), "DECOY was deleted");

    // Not a specified (black) Digimon.
    let (mut r, _) = decoy_board(1, &["MAIN_DEL1"]);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 2);
    let prompts = settle_declining(&mut r);
    assert!(
        !offered_to(&prompts, 1),
        "red FILL is not specified: no <Decoy>: {prompts:?}"
    );
    assert_eq!(field_top_ids(&r, 1), owned(&["DECOY", "BLACK_LV3"]));

    // By its owner's own effect.
    let (mut r, _) = decoy_board(0, &["MAIN_DEL_OWN_MEM"]);
    delete_own(&mut r, 1);
    let prompts = settle_declining(&mut r);
    assert!(
        !offered_to(&prompts, 0),
        "own effect: no <Decoy>: {prompts:?}"
    );
    assert_eq!(field_top_ids(&r, 0), owned(&["DECOY", FILL, "TAMER_PLAIN"]));

    // In battle.
    let (mut r, black) = decoy_board(1, &[]);
    attack_with_lv5(&mut r, black);
    let prompts = settle_declining(&mut r);
    assert!(!offered_to(&prompts, 1), "battle: no <Decoy>: {prompts:?}");
    assert_eq!(field_top_ids(&r, 1), owned(&["DECOY", FILL]));

    // Not one of YOUR Digimon: P0 deletes P0's own black Digimon while P1
    // has DECOY.
    let mut r = board(&["MAIN_DEL_OWN_MEM"]);
    r.place_on_field(1, "DECOY", Some(0));
    r.place_on_field(0, "BLACK_LV3", Some(0));
    red_anchor(&mut r);
    delete_own(&mut r, 0);
    let prompts = settle_declining(&mut r);
    assert!(
        !offered_to(&prompts, 1),
        "the opponent's black Digimon: no <Decoy>: {prompts:?}"
    );
    assert!(trash_ids(&r, 0).contains(&"BLACK_LV3".to_string()));
    assert_eq!(field_top_ids(&r, 1), owned(&["DECOY"]));
}

/// §16-18-3 ""By deleting this Digimon" in <Decoy> is an optional processing
/// condition. If the content of that condition is executed, the processing for
/// "<Decoy> prevents the Digimon specified by this effect from being deleted"
/// will be mandatory."
///
/// P1 accepts <Decoy> against P0's `MAIN_DEL1` on `BLACK_LV3`: `DECOY` is
/// deleted and, with no further decision, `BLACK_LV3` is not.
#[test]
fn decoy_optional_cost_then_mandatory_prevention() {
    cite!(
        "16-18-3",
        "748b4969a59f3391e66a67fbe587f4bbc535a3f006c4e415960962e97a3e105b"
    );
    let (mut r, _) = decoy_board(1, &["MAIN_DEL1"]);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 1);
    assert!(advance_to(&mut r, optional_for(1)));
    accept(&mut r, 1);
    let after = settle_declining(&mut r);
    assert!(
        !offered_to(&after, 1),
        "the prevention is mandatory once DECOY is deleted: {after:?}"
    );
    assert_eq!(trash_ids(&r, 1), owned(&["DECOY"]));
    assert_eq!(field_top_ids(&r, 1), owned(&["BLACK_LV3", FILL]));
}

/// §16-18-2 "<Decoy> is an IMMEDIATE-TYPE effect that triggers when another of
/// your Digimon specified by this effect WOULD be deleted ..."
///
/// When P1 is offered <Decoy>, `BLACK_LV3` is still in the battle area and
/// nothing is in P1's trash; after accepting, `BLACK_LV3` was never trashed.
#[test]
fn decoy_kind_matches_rule() {
    cite!(
        "16-18-2",
        "f33f5d7a2ab9065b3ee0f4865e82b3d5188c93c39700feb6a753f35d52928f20"
    );
    let (mut r, _) = decoy_board(1, &["MAIN_DEL1"]);
    let cp = r.event_checkpoint();
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 1);
    assert!(advance_to(&mut r, optional_for(1)));
    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["DECOY", "BLACK_LV3", FILL]),
        "offered before the deletion"
    );
    assert!(trash_ids(&r, 1).is_empty());
    accept(&mut r, 1);
    settle_declining(&mut r);
    assert!(
        !trashed_since(&r, cp, 1, "BLACK_LV3"),
        "BLACK_LV3 never left"
    );
}

/// §16-18-3 ""By deleting this Digimon" in <Decoy> is an OPTIONAL processing
/// condition. ..."
///
/// P1 declines <Decoy>: `BLACK_LV3` is deleted, `DECOY` stays. (Matrix row
/// `Decoy`: `optional_cost_then_mandatory`, agrees.)
#[test]
fn decoy_optionality_matches_rule() {
    cite!(
        "16-18-3",
        "748b4969a59f3391e66a67fbe587f4bbc535a3f006c4e415960962e97a3e105b"
    );
    let (mut r, _) = decoy_board(1, &["MAIN_DEL1"]);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 1);
    assert!(advance_to(&mut r, optional_for(1)));
    r.decline_optional_trigger().expect("decline <Decoy>");
    settle_declining(&mut r);
    assert_eq!(trash_ids(&r, 1), owned(&["BLACK_LV3"]));
    assert_eq!(field_top_ids(&r, 1), owned(&["DECOY", FILL]));
}

/// §16-18-2 "<Decoy> ... triggers when ANOTHER of your Digimon specified by
/// this effect WOULD BE DELETED by an opponent's effect"
///
/// - Leaving without deletion: P0's `MAIN_BOUNCE1` returns P1's `BLACK_LV3` to
///   the hand: no <Decoy>.
/// - Not another Digimon: P0's `MAIN_DEL1` targets `DECOY` itself: no
///   <Decoy>, `DECOY` is deleted.
#[test]
fn decoy_timing_matches_rule() {
    cite!(
        "16-18-2",
        "f33f5d7a2ab9065b3ee0f4865e82b3d5188c93c39700feb6a753f35d52928f20"
    );
    let (mut r, _) = decoy_board(1, &["MAIN_BOUNCE1"]);
    use_on_opponent_slot(&mut r, "MAIN_BOUNCE1", 1);
    let prompts = settle_declining(&mut r);
    assert!(
        !offered_to(&prompts, 1),
        "returned to the hand, not deleted: no <Decoy>: {prompts:?}"
    );
    assert_eq!(hand_ids(&r, 1), owned(&["BLACK_LV3"]));

    let (mut r, _) = decoy_board(1, &["MAIN_DEL1"]);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    let prompts = settle_declining(&mut r);
    assert!(
        !offered_to(&prompts, 1),
        "DECOY itself: no <Decoy>: {prompts:?}"
    );
    assert_eq!(trash_ids(&r, 1), owned(&["DECOY"]));
}

// ─── §16-19 <Armor Purge> ───────────────────────────────────────────────────

/// P1 has the stack `stack` (bottom first) in slot 0; P0 holds `hand0`.
fn p1_stack_board(stack: &[&str], hand0: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = board(hand0);
    let h = r.place_stack(1, stack);
    red_anchor(&mut r);
    (r, h)
}

/// §16-19-1 "<Armor Purge> is a keyword effect. When a Digimon with <Armor
/// Purge> would be deleted, by trashing the top card of the Digimon with this
/// effect, this effect prevents the deletion."
/// §16-19-3 "... If the content of that condition is executed, the processing
/// for "prevents the deletion" will be mandatory."
///
/// - P1's stack `[LV4_PLAIN, ARMOR]` gets -3000 DP for the turn from P0's
///   `MAIN_MINUS3000` (3000 DP), then P0's `MAIN_DEL1` targets it. P1 accepts
///   <Armor Purge>: only `ARMOR` (the top card) is trashed, with no further
///   decision the Digimon is not deleted, and it is the SAME Digimon: the
///   -3000 DP still applies to it (`LV4_PLAIN` 5000 -> 2000). (Matrix row
///   `ArmorPurge`: "surviving modifiers carry over".)
/// - A lone `ARMOR` (no digivolution card): trashing its top card can't leave a
///   Digimon to prevent the deletion of, so <Armor Purge> is not offered and
///   `ARMOR` is deleted. DCGO
///   `CardEffectCommons/KeyWordEffects/ArmorPurge.cs:13` gates on
///   `DigivolutionCards.Count >= 1`; matrix row: "needs >=1 digivolution card".
#[test]
fn armor_purge_requires_a_digivolution_card_and_keeps_modifiers() {
    cite!(
        "16-19-1",
        "135d1733ceef918a3a2d90783af66d72360f31353eab78ddeb864cf5efd5dd8c"
    );
    cite!(
        "16-19-3",
        "b5c2a62c9cae74e666d105e93fda468a2a6545b7a7cca9115635fff01bd3d38f"
    );
    let (mut r, armor) = p1_stack_board(&["LV4_PLAIN", "ARMOR"], &["MAIN_MINUS3000", "MAIN_DEL1"]);
    use_on_opponent_slot(&mut r, "MAIN_MINUS3000", 0);
    settle_declining(&mut r);
    assert_eq!(r.dp_of(armor), Some(3000), "control: 6000 - 3000");
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    assert!(
        advance_to(&mut r, optional_for(1)),
        "P1 is offered <Armor Purge>"
    );
    accept(&mut r, 1);
    let after = settle_declining(&mut r);
    assert!(
        !offered_to(&after, 1),
        "'prevents the deletion' is mandatory once the top card is trashed: {after:?}"
    );
    assert_eq!(field_top_ids(&r, 1), owned(&["LV4_PLAIN"]), "not deleted");
    assert_eq!(trash_ids(&r, 1), owned(&["ARMOR"]), "only the top card");
    assert_eq!(
        r.dp_of(armor),
        Some(2000),
        "the same Digimon: the -3000 DP for the turn still applies"
    );

    let (mut r, _) = p1_stack_board(&["ARMOR"], &["MAIN_DEL1"]);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    let prompts = settle_declining(&mut r);
    assert!(
        !offered_to(&prompts, 1),
        "no digivolution card: no <Armor Purge>: {prompts:?}"
    );
    assert_eq!(trash_ids(&r, 1), owned(&["ARMOR"]));
}

/// §16-19-2 "<Armor Purge> is an IMMEDIATE-TYPE effect that triggers when the
/// Digimon with this effect WOULD be deleted."
///
/// When P1 is offered <Armor Purge>, the stack `[LV4_PLAIN, ARMOR]` is intact
/// in the battle area and P1's trash is empty; after accepting, `LV4_PLAIN`
/// was never trashed.
#[test]
fn armor_purge_kind_matches_rule() {
    cite!(
        "16-19-2",
        "50d4eb51f97b5f60ed49a9dd27c5f828da0ace5946e7b1233594ddf206601455"
    );
    let (mut r, armor) = p1_stack_board(&["LV4_PLAIN", "ARMOR"], &["MAIN_DEL1"]);
    let cp = r.event_checkpoint();
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    assert!(advance_to(&mut r, optional_for(1)));
    assert_eq!(stack_ids(&r, armor), owned(&["LV4_PLAIN", "ARMOR"]));
    assert!(trash_ids(&r, 1).is_empty());
    accept(&mut r, 1);
    settle_declining(&mut r);
    assert!(!trashed_since(&r, cp, 1, "LV4_PLAIN"), "never deleted");
}

/// §16-19-3 ""By trashing the top card of the Digimon" in <Armor Purge> is an
/// OPTIONAL processing condition. ..."
///
/// P1 declines: the whole stack is deleted. (Matrix row `ArmorPurge`:
/// `optional_cost_then_mandatory`, agrees.)
#[test]
fn armor_purge_optionality_matches_rule() {
    cite!(
        "16-19-3",
        "b5c2a62c9cae74e666d105e93fda468a2a6545b7a7cca9115635fff01bd3d38f"
    );
    let (mut r, _) = p1_stack_board(&["LV4_PLAIN", "ARMOR"], &["MAIN_DEL1"]);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    assert!(advance_to(&mut r, optional_for(1)));
    r.decline_optional_trigger().expect("decline <Armor Purge>");
    settle_declining(&mut r);
    assert!(field_top_ids(&r, 1).is_empty());
    assert_eq!(trash_ids(&r, 1), owned(&["ARMOR", "LV4_PLAIN"]));
}

/// §16-19-2 "<Armor Purge> ... triggers when the Digimon with this effect
/// WOULD BE DELETED."
///
/// - Any deletion, including in battle: P0's `LV5_PLAIN` (7000 DP) attacks
///   P1's suspended `[LV4_PLAIN, ARMOR]` (6000 DP): offered; accepting keeps
///   `LV4_PLAIN` on the field.
/// - Leaving without deletion: P0's `MAIN_BOUNCE1` returns it to the hand: no
///   <Armor Purge>.
#[test]
fn armor_purge_timing_matches_rule() {
    cite!(
        "16-19-2",
        "50d4eb51f97b5f60ed49a9dd27c5f828da0ace5946e7b1233594ddf206601455"
    );
    let (mut r, armor) = p1_stack_board(&["LV4_PLAIN", "ARMOR"], &[]);
    attack_with_lv5(&mut r, armor);
    assert!(
        advance_to(&mut r, optional_for(1)),
        "deleted in battle: <Armor Purge> is offered"
    );
    accept(&mut r, 1);
    settle_declining(&mut r);
    assert_eq!(field_top_ids(&r, 1), owned(&["LV4_PLAIN"]));

    let (mut r, _) = p1_stack_board(&["LV4_PLAIN", "ARMOR"], &["MAIN_BOUNCE1"]);
    use_on_opponent_slot(&mut r, "MAIN_BOUNCE1", 0);
    let prompts = settle_declining(&mut r);
    assert!(
        !offered_to(&prompts, 1),
        "not deleted: no <Armor Purge>: {prompts:?}"
    );
    assert_eq!(hand_ids(&r, 1), owned(&["ARMOR"]));
}

// ─── §16-22 <Evade> ─────────────────────────────────────────────────────────

/// §16-22-1 "<Evade> is a keyword effect. By suspending a Digimon with this
/// effect when it would be deleted, this effect prevents the deletion."
/// §16-22-3 ""By suspending a Digimon with this effect" in <Evade> is an
/// optional processing condition. If the content of that condition is
/// executed, the processing for "prevents the deletion" will be mandatory."
/// §15-7-4 "If executing an optional processing condition's contents isn't
/// possible, then the player can't choose to execute them. ..."
/// §1-3-3 "If a card in a given state is requested to be changed to that same
/// state, the card remains in its current state. ..." (suspending a suspended
/// Digimon isn't performed, so the condition can't be executed)
///
/// - P0's `MAIN_DEL1` targets P1's unsuspended `EVADE`: offered; accepting
///   suspends it and, with no further decision, it is not deleted.
/// - Already suspended, `EVADE` can't be suspended (the condition can't be
///   executed): <Evade> is not offered and `EVADE` is deleted.
#[test]
fn evade_suspends_to_prevent_deletion() {
    cite!(
        "16-22-1",
        "00a788bedc3eabeb629feb4d9387c3e3c599982c4f499ff90348ed89fad5315d"
    );
    cite!(
        "16-22-3",
        "ac75eeecf730916ea8997134b03733879d57138067d38b558512274dc00ad005"
    );
    cite!(
        "15-7-4",
        "210ce11e09b84d8a05b2b4861fcbff3ad5d398894a7da5879d8c869dc6446b14"
    );
    cite!(
        "1-3-3",
        "ca899d2909e51caa64022d4c9553942bacb4a821e53cb52e8f2e1d02a4830324"
    );
    let (mut r, evade) = p1_stack_board(&["EVADE"], &["MAIN_DEL1"]);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    assert!(advance_to(&mut r, optional_for(1)), "P1 is offered <Evade>");
    accept(&mut r, 1);
    let after = settle_declining(&mut r);
    assert!(
        !offered_to(&after, 1),
        "'prevents the deletion' is mandatory once suspended: {after:?}"
    );
    assert_eq!(field_top_ids(&r, 1), owned(&["EVADE"]));
    assert!(is_suspended(&r, evade), "suspended to pay");

    let (mut r, evade) = p1_stack_board(&["EVADE"], &["MAIN_DEL1"]);
    set_suspended(&mut r, evade, true);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    let prompts = settle_declining(&mut r);
    assert!(
        !offered_to(&prompts, 1),
        "already suspended: <Evade> can't be paid, not offered: {prompts:?}"
    );
    assert_eq!(trash_ids(&r, 1), owned(&["EVADE"]));
}

/// §16-22-2 "<Evade> is an IMMEDIATE-TYPE effect that triggers when the
/// Digimon with this effect WOULD be deleted."
///
/// When P1 is offered <Evade>, `EVADE` is in the battle area, unsuspended, and
/// P1's trash is empty; after accepting, `EVADE` was never trashed.
#[test]
fn evade_kind_matches_rule() {
    cite!(
        "16-22-2",
        "c70b2dbb39add633ff6879aebeb2fbb7d3ccb66b32575808abacf16994a766cf"
    );
    let (mut r, evade) = p1_stack_board(&["EVADE"], &["MAIN_DEL1"]);
    let cp = r.event_checkpoint();
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    assert!(advance_to(&mut r, optional_for(1)));
    assert_eq!(field_top_ids(&r, 1), owned(&["EVADE"]));
    assert!(!is_suspended(&r, evade));
    assert!(trash_ids(&r, 1).is_empty());
    accept(&mut r, 1);
    settle_declining(&mut r);
    assert!(!trashed_since(&r, cp, 1, "EVADE"), "never deleted");
}

/// §16-22-3 ""By suspending a Digimon with this effect" in <Evade> is an
/// OPTIONAL processing condition. ..."
///
/// P1 declines: `EVADE` is deleted. (Matrix row `Evade`:
/// `optional_cost_then_mandatory`, agrees.)
#[test]
fn evade_optionality_matches_rule() {
    cite!(
        "16-22-3",
        "ac75eeecf730916ea8997134b03733879d57138067d38b558512274dc00ad005"
    );
    let (mut r, _) = p1_stack_board(&["EVADE"], &["MAIN_DEL1"]);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    assert!(advance_to(&mut r, optional_for(1)));
    r.decline_optional_trigger().expect("decline <Evade>");
    settle_declining(&mut r);
    assert_eq!(trash_ids(&r, 1), owned(&["EVADE"]));
}

/// §16-22-2 "<Evade> ... triggers when the Digimon with this effect WOULD BE
/// DELETED."
///
/// - Any cause, including its owner's own effect: P0's `MAIN_DEL_OWN_MEM`
///   deletes P0's own `EVADE`: offered; accepting keeps it on the field.
/// - In battle (P0's `LV5_PLAIN` battles P1's unsuspended `EVADE` through
///   `battle_digimon`): offered; accepting suspends it and it survives.
/// - Leaving without deletion: P0's `MAIN_BOUNCE1` returns P1's `EVADE` to
///   the hand: no <Evade>.
#[test]
fn evade_timing_matches_rule() {
    cite!(
        "16-22-2",
        "c70b2dbb39add633ff6879aebeb2fbb7d3ccb66b32575808abacf16994a766cf"
    );
    let mut r = board(&["MAIN_DEL_OWN_MEM"]);
    r.place_on_field(0, "EVADE", Some(0));
    red_anchor(&mut r);
    delete_own(&mut r, 0);
    assert!(
        advance_to(&mut r, optional_for(0)),
        "own effect: <Evade> is offered"
    );
    accept(&mut r, 0);
    settle_declining(&mut r);
    assert_eq!(field_top_ids(&r, 0), owned(&["EVADE", "TAMER_PLAIN"]));

    let (mut r, evade) = p1_stack_board(&["EVADE"], &[]);
    let attacker = r.place_on_field(0, "LV5_PLAIN", Some(0));
    r.battle_digimon(attacker, evade);
    assert!(
        advance_to(&mut r, optional_for(1)),
        "deleted in battle: <Evade> is offered"
    );
    accept(&mut r, 1);
    settle_declining(&mut r);
    assert_eq!(field_top_ids(&r, 1), owned(&["EVADE"]), "not deleted");
    assert!(is_suspended(&r, evade), "suspended to pay");

    let (mut r, _) = p1_stack_board(&["EVADE"], &["MAIN_BOUNCE1"]);
    use_on_opponent_slot(&mut r, "MAIN_BOUNCE1", 0);
    let prompts = settle_declining(&mut r);
    assert!(
        !offered_to(&prompts, 1),
        "not deleted: no <Evade>: {prompts:?}"
    );
    assert_eq!(hand_ids(&r, 1), owned(&["EVADE"]));
}

// ─── §16-25 <Barrier> ───────────────────────────────────────────────────────

/// P1's security stack `[FILL, FILL, BLUE_LV4]` (top: `BLUE_LV4`) and
/// `BARRIER` (slot 0); P0's security `[FILL, FILL, GREEN_LV4]`.
fn barrier_board(sec1: &[&str], hand0: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = board_with(ch16_repl(), hand0, &[FILL, FILL, "GREEN_LV4"], sec1);
    let h = r.place_on_field(1, "BARRIER", Some(0));
    red_anchor(&mut r);
    (r, h)
}

const SEC_TOP_BLUE: &[&str] = &[FILL, FILL, "BLUE_LV4"];

/// §16-25-1 "<Barrier> is a keyword effect. When a Digimon with this effect
/// would be deleted in battle, by trashing the top card of your security
/// stack, this effect prevents that deletion."
/// §16-25-2 "<Barrier> is an immediate-type effect that triggers when the
/// Digimon with this effect would be deleted in battle."
/// §15-7-4 "If executing an optional processing condition's contents isn't
/// possible, then the player can't choose to execute them. ..."
///
/// - In battle: P0's `LV5_PLAIN` attacks P1's suspended `BARRIER`. P1 accepts
///   <Barrier>: the TOP security card (`BLUE_LV4`) is trashed and `BARRIER`
///   survives.
/// - By an effect: P0's `MAIN_DEL1` targets `BARRIER`: no <Barrier>.
/// - With no security card the condition can't be executed: not offered,
///   `BARRIER` is deleted in battle.
///
/// §16-25-3 ""By trashing the top card of your security stack" in <Barrier>
/// is an optional processing condition. If the content of that condition is
/// executed, the processing for "prevents the deletion" will be mandatory."
/// (accepted: the top card is trashed and the deletion is prevented; not
/// executable: not offered).
#[test]
fn barrier_trashes_top_security_only_for_battle_deletion() {
    cite!(
        "16-25-1",
        "5fda7eb177e691c2fd8114e6d08765580610062fb1afb946fe0129ec0c8fd5bc"
    );
    cite!(
        "16-25-3",
        "d05740324cfe6e7edbc153b03e623dc3a61b350e0120701a63c0e0d9d111330d"
    );
    cite!(
        "16-25-2",
        "0a82a83eb0cb3dcbd9b128eda640da0d3f9d33b4bf45efe9a3d117f25baf3744"
    );
    cite!(
        "15-7-4",
        "210ce11e09b84d8a05b2b4861fcbff3ad5d398894a7da5879d8c869dc6446b14"
    );
    let (mut r, barrier) = barrier_board(SEC_TOP_BLUE, &[]);
    attack_with_lv5(&mut r, barrier);
    assert!(
        advance_to(&mut r, optional_for(1)),
        "P1 is offered <Barrier>"
    );
    accept(&mut r, 1);
    let after = settle_declining(&mut r);
    assert!(
        !offered_to(&after, 1),
        "'prevents that deletion' is mandatory once paid: {after:?}"
    );
    assert_eq!(field_top_ids(&r, 1), owned(&["BARRIER"]));
    assert_eq!(security_ids(&r, 1), owned(&[FILL, FILL]));
    assert_eq!(
        trash_ids(&r, 1),
        owned(&["BLUE_LV4"]),
        "the top security card"
    );

    let (mut r, _) = barrier_board(SEC_TOP_BLUE, &["MAIN_DEL1"]);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    let prompts = settle_declining(&mut r);
    assert!(
        !offered_to(&prompts, 1),
        "deleted by an effect: no <Barrier>: {prompts:?}"
    );
    assert_eq!(trash_ids(&r, 1), owned(&["BARRIER"]));
    assert_eq!(r.security_count(1), 3);

    let (mut r, barrier) = barrier_board(&[], &[]);
    attack_with_lv5(&mut r, barrier);
    let prompts = settle_declining(&mut r);
    assert!(
        !offered_to(&prompts, 1),
        "no security card: <Barrier> can't be paid, not offered: {prompts:?}"
    );
    assert_eq!(trash_ids(&r, 1), owned(&["BARRIER"]));
}

/// §16-25-2 "<Barrier> is an IMMEDIATE-TYPE effect that triggers when the
/// Digimon with this effect WOULD be deleted in battle."
///
/// When P1 is offered <Barrier>, `BARRIER` is in the battle area, P1 still has
/// 3 security cards and its trash is empty; after accepting, `BARRIER` was
/// never trashed.
#[test]
fn barrier_kind_matches_rule() {
    cite!(
        "16-25-2",
        "0a82a83eb0cb3dcbd9b128eda640da0d3f9d33b4bf45efe9a3d117f25baf3744"
    );
    let (mut r, barrier) = barrier_board(SEC_TOP_BLUE, &[]);
    let cp = r.event_checkpoint();
    attack_with_lv5(&mut r, barrier);
    assert!(advance_to(&mut r, optional_for(1)));
    assert_eq!(field_top_ids(&r, 1), owned(&["BARRIER"]));
    assert_eq!(r.security_count(1), 3);
    assert!(trash_ids(&r, 1).is_empty());
    accept(&mut r, 1);
    settle_declining(&mut r);
    assert!(!trashed_since(&r, cp, 1, "BARRIER"), "never deleted");
}

/// §16-25-3 ""By trashing the top card of your security stack" in <Barrier> is
/// an OPTIONAL processing condition. ..."
///
/// P1 declines: `BARRIER` is deleted and the security stack is untouched.
/// (Matrix row `Barrier`: `optional_cost_then_mandatory`, agrees.)
#[test]
fn barrier_optionality_matches_rule() {
    cite!(
        "16-25-3",
        "d05740324cfe6e7edbc153b03e623dc3a61b350e0120701a63c0e0d9d111330d"
    );
    let (mut r, barrier) = barrier_board(SEC_TOP_BLUE, &[]);
    attack_with_lv5(&mut r, barrier);
    assert!(advance_to(&mut r, optional_for(1)));
    r.decline_optional_trigger().expect("decline <Barrier>");
    settle_declining(&mut r);
    assert_eq!(trash_ids(&r, 1), owned(&["BARRIER"]));
    assert_eq!(security_ids(&r, 1), owned(SEC_TOP_BLUE));
}

/// §16-25-2 "<Barrier> ... triggers when the Digimon with this effect would be
/// DELETED IN BATTLE."
///
/// Attacking or defending: P0's own `BARRIER` (4000 DP) attacks P1's
/// suspended `LV5_PLAIN` (7000 DP) and loses. P0 is offered <Barrier>;
/// accepting trashes P0's top security card (`GREEN_LV4`) and `BARRIER` stays.
#[test]
fn barrier_timing_matches_rule() {
    cite!(
        "16-25-2",
        "0a82a83eb0cb3dcbd9b128eda640da0d3f9d33b4bf45efe9a3d117f25baf3744"
    );
    let mut r = board_with(ch16_repl(), &[], &[FILL, FILL, "GREEN_LV4"], &[FILL; 3]);
    let barrier = r.place_on_field(0, "BARRIER", Some(0));
    let defender = r.place_on_field(1, "LV5_PLAIN", Some(0));
    set_suspended(&mut r, defender, true);
    r.attack_digimon(barrier, defender, false);
    assert!(
        advance_to(&mut r, optional_for(0)),
        "the attacker losing its battle: <Barrier> is offered"
    );
    accept(&mut r, 0);
    settle_declining(&mut r);
    assert_eq!(field_top_ids(&r, 0), owned(&["BARRIER"]));
    assert_eq!(trash_ids(&r, 0), owned(&["GREEN_LV4"]));
}

// ─── §16-29 <Partition> ─────────────────────────────────────────────────────

const PARTITION_FULL: &[&str] = &["BLUE_LV4", "GREEN_LV4", "PARTITION"];

/// After <Partition> is accepted, answer any card pick the engine asks for by
/// taking a `BLUE_LV4`. Whatever the engine asks here (today: which specified
/// card to play first, an artifact of the sequential plays --
/// `G-ENGINE-PARTITION-PLAYS-NOT-SIMULTANEOUS`; or which of two blue Lv.4
/// cards fills the blue slot), it can't be declined: §16-29-4 forbids playing
/// none or only some of the specified cards. The test asserts the outcome.
fn pick_partition_blue_if_asked(r: &mut DebugRunner, partition: PermanentHandle) {
    let Some(p) = current_prompt(r) else {
        return;
    };
    if p.player != 1 || !p.labels.is_empty() {
        return;
    }
    assert!(!p.optional, "no way to play none or only some: {p:?}");
    pick_source(r, partition, "BLUE_LV4");
}

/// §16-29-1 "<Partition> is a keyword effect. When a Digimon with this effect
/// and 1 of each of the specified cards in its digivolution cards would be
/// removed from the battle area other than by one of your effects or a
/// battle, you may play 1 of each of the specified cards from the digivolution
/// cards without paying their costs. This is shown on cards using text such as
/// "<Partition (blue Lv.4 & green Lv.4)>.""
/// §16-29-4 "When <Partition> is activated, 1 of each of the specified cards
/// is played from digivolution cards without paying their costs. A player
/// can't choose to only play one or some of the specified cards."
///
/// - P0's `MAIN_DEL1` targets P1's `[BLUE_LV4, GREEN_LV4, PARTITION]`. P1
///   accepts <Partition>: any card pick that follows can't be declined, BOTH
///   `BLUE_LV4` and `GREEN_LV4` are played (no decision lets P1 play only
///   one) without paying (P1's memory is unchanged), and `PARTITION` is
///   deleted.
/// - 1 of EACH: on `[BLUE_LV4, BLUE_LV4, GREEN_LV4, PARTITION]` exactly one
///   blue Lv.4 and the green Lv.4 are played; the other blue Lv.4 is trashed
///   with `PARTITION`.
/// - Without 1 of each specified card (`[BLUE_LV4, LV4_PLAIN, PARTITION]`, no
///   green Lv.4): not offered.
/// - By its owner's own effect (P0's `MAIN_DEL_OWN_MEM` on P0's own
///   `PARTITION` stack): not offered.
/// - In battle (P0's `LV5_PLAIN` attacks P1's suspended `PARTITION`): not
///   offered.
#[test]
fn partition_plays_one_of_each_specified_all_or_nothing() {
    cite!(
        "16-29-1",
        "4afbb712a5a6e67bd82d50f976645c6ddfedde505b0b2bc9a0726472eb322620"
    );
    cite!(
        "16-29-4",
        "15ddd6ad329dff1933d98df38fb6401362e08121f13dbd684acf8fc014310dfc"
    );
    let (mut r, partition) = p1_stack_board(PARTITION_FULL, &["MAIN_DEL1"]);
    let memory = r.memory();
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    assert!(
        advance_to(&mut r, optional_for(1)),
        "P1 is offered <Partition>"
    );
    accept(&mut r, 1);
    pick_partition_blue_if_asked(&mut r, partition);
    let after = settle_declining(&mut r);
    assert!(
        !offered_to(&after, 1),
        "no decision to play only some of the specified cards: {after:?}"
    );
    assert_eq!(
        sorted(field_top_ids(&r, 1)),
        owned(&["BLUE_LV4", "GREEN_LV4"]),
        "1 of each specified card was played"
    );
    assert_eq!(trash_ids(&r, 1), owned(&["PARTITION"]));
    assert_eq!(r.memory(), memory, "played without paying their costs");

    // "1 of each": with two blue Lv.4 cards, only one of them is played.
    let (mut r, partition) = p1_stack_board(
        &["BLUE_LV4", "BLUE_LV4", "GREEN_LV4", "PARTITION"],
        &["MAIN_DEL1"],
    );
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    assert!(advance_to(&mut r, optional_for(1)));
    accept(&mut r, 1);
    pick_partition_blue_if_asked(&mut r, partition);
    settle_declining(&mut r);
    assert_eq!(
        sorted(field_top_ids(&r, 1)),
        owned(&["BLUE_LV4", "GREEN_LV4"]),
        "exactly 1 blue Lv.4 and 1 green Lv.4 were played"
    );
    assert_eq!(
        trash_ids(&r, 1),
        owned(&["BLUE_LV4", "PARTITION"]),
        "the other blue Lv.4 was not played"
    );

    let (mut r, _) = p1_stack_board(&["BLUE_LV4", "LV4_PLAIN", "PARTITION"], &["MAIN_DEL1"]);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    let prompts = settle_declining(&mut r);
    assert!(
        !offered_to(&prompts, 1),
        "no green Lv.4: <Partition> doesn't trigger: {prompts:?}"
    );
    assert_eq!(
        trash_ids(&r, 1),
        owned(&["BLUE_LV4", "LV4_PLAIN", "PARTITION"])
    );

    let mut r = board(&["MAIN_DEL_OWN_MEM"]);
    r.place_stack(0, PARTITION_FULL);
    red_anchor(&mut r);
    delete_own(&mut r, 0);
    let prompts = settle_declining(&mut r);
    assert!(
        !offered_to(&prompts, 0),
        "own effect: no <Partition>: {prompts:?}"
    );
    assert_eq!(field_top_ids(&r, 0), owned(&["TAMER_PLAIN"]));

    let (mut r, partition) = p1_stack_board(PARTITION_FULL, &[]);
    attack_with_lv5(&mut r, partition);
    let prompts = settle_declining(&mut r);
    assert!(
        !offered_to(&prompts, 1),
        "battle: no <Partition>: {prompts:?}"
    );
    assert!(field_top_ids(&r, 1).is_empty());
}

/// §16-29-2 "<Partition> is an IMMEDIATE-TYPE effect that triggers "when a
/// Digimon with this effect and 1 of each of the specified cards in its
/// digivolution cards WOULD be removed from the battle area ...""
///
/// When P1 is offered <Partition>, `PARTITION` is in the battle area with both
/// specified cards under it, and P1's trash is empty.
#[test]
fn partition_kind_matches_rule() {
    cite!(
        "16-29-2",
        "2cb953be978558ca6f2ed7709d006b03e659c93384ff173342bcc4a3a0759358"
    );
    let (mut r, partition) = p1_stack_board(PARTITION_FULL, &["MAIN_DEL1"]);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    assert!(advance_to(&mut r, optional_for(1)));
    assert_eq!(stack_ids(&r, partition), owned(PARTITION_FULL));
    assert!(trash_ids(&r, 1).is_empty());
}

/// §16-29-3 "The processing from <Partition> is optional."
///
/// P1 declines: nothing is played and the whole stack is deleted. (Matrix row
/// `Partition`: `optional`, agrees.)
#[test]
fn partition_optionality_matches_rule() {
    cite!(
        "16-29-3",
        "7476f5777612887f9684ed66b672761916b834ded6f98d0c5c41c50cb5a04eff"
    );
    let (mut r, _) = p1_stack_board(PARTITION_FULL, &["MAIN_DEL1"]);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    assert!(advance_to(&mut r, optional_for(1)));
    r.decline_optional_trigger().expect("decline <Partition>");
    settle_declining(&mut r);
    assert!(field_top_ids(&r, 1).is_empty());
    assert_eq!(trash_ids(&r, 1), sorted(owned(PARTITION_FULL)));
}

/// §16-29-2 "<Partition> ... triggers "when a Digimon with this effect ...
/// would be REMOVED FROM THE BATTLE AREA other than by one of your effects or
/// a battle.""
///
/// Not only deletion: P0's `MAIN_BOUNCE1` returns P1's `PARTITION` to the
/// hand. P1 is offered <Partition>; accepting plays `BLUE_LV4` and
/// `GREEN_LV4`, and `PARTITION` goes to the hand.
#[test]
fn partition_timing_matches_rule() {
    cite!(
        "16-29-2",
        "2cb953be978558ca6f2ed7709d006b03e659c93384ff173342bcc4a3a0759358"
    );
    let (mut r, partition) = p1_stack_board(PARTITION_FULL, &["MAIN_BOUNCE1"]);
    use_on_opponent_slot(&mut r, "MAIN_BOUNCE1", 0);
    assert!(
        advance_to(&mut r, optional_for(1)),
        "a return to the hand is removal from the battle area"
    );
    accept(&mut r, 1);
    pick_partition_blue_if_asked(&mut r, partition);
    settle_declining(&mut r);
    assert_eq!(
        sorted(field_top_ids(&r, 1)),
        owned(&["BLUE_LV4", "GREEN_LV4"])
    );
    assert_eq!(hand_ids(&r, 1), owned(&["PARTITION"]));
}

// ─── §16-32 <Scapegoat> ─────────────────────────────────────────────────────

/// `SCAPEGOAT`'s printed face, as production loads it: the DSL can't grant
/// <Scapegoat>, so the runtime behaviour rides on the printed-keyword parse of
/// the text (same staging as `ch15_08`'s <Training>).
fn scapegoat_card() -> CardData {
    let mut printed = make_test_card("SCAPEGOAT", "Goatmon");
    printed.level = Some(4);
    printed.dp = Some(4000);
    printed.play_cost = 4;
    printed.effect_text = concat!(
        "＜Scapegoat＞ (When this Digimon would be deleted other than by one of your ",
        "effects, by deleting 1 of your other Digimon, it isn't deleted.)"
    )
    .to_string();
    printed.keywords = parse_printed_keywords(&printed.effect_text, "", "");
    assert!(
        printed.keywords.contains(&Keyword::Scapegoat),
        "the printed text parses to <Scapegoat>: {:?}",
        printed.keywords
    );
    printed
}

/// `owner` has `SCAPEGOAT` (slot 0) and the Digimon `others`; P0 holds
/// `hand0`.
fn scapegoat_board(
    owner: PlayerId,
    others: &[&str],
    hand0: &[&str],
) -> (DebugRunner, PermanentHandle) {
    let mut r = board_with(
        ch16_repl().add_card(scapegoat_card()),
        hand0,
        &[FILL; 3],
        &[FILL; 3],
    );
    let h = r.place_on_field(owner, "SCAPEGOAT", Some(0));
    for id in others {
        r.place_on_field(owner, id, Some(0));
    }
    red_anchor(&mut r);
    (r, h)
}

/// §16-32-1 "<Scapegoat> is a keyword effect. By deleting 1 of your other
/// Digimon when a Digimon with this effect would be deleted other than by one
/// of your effects, this effect prevents the deletion."
/// §16-32-2 "<Scapegoat> is an immediate-type effect that triggers when the
/// Digimon with this effect would be deleted other than by one of your
/// effects."
///
/// - By an opponent's effect: P0's `MAIN_DEL1` targets P1's `SCAPEGOAT`. P1
///   accepts: `FILL` is deleted, `SCAPEGOAT` is not.
/// - In battle (not one of P1's effects): P0's `LV5_PLAIN` attacks P1's
///   suspended `SCAPEGOAT`: offered; accepting deletes `FILL` instead.
/// - By its owner's own effect: P0's `MAIN_DEL_OWN_MEM` deletes P0's own
///   `SCAPEGOAT`: not offered; `SCAPEGOAT` is deleted and `FILL` stays.
#[test]
fn scapegoat_other_than_your_own_effect() {
    cite!(
        "16-32-1",
        "31aeb81288d1ee77afb00c719a396f76f12a8b0d14d25e9362c4b1bb6d7b5bed"
    );
    cite!(
        "16-32-2",
        "85d9685d7cda979c34246eda204c87f4ca2db09cc0b7d5e6415af5765a2a802a"
    );
    let (mut r, _) = scapegoat_board(1, &[FILL], &["MAIN_DEL1"]);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    assert!(
        advance_to(&mut r, optional_for(1)),
        "P1 is offered <Scapegoat>"
    );
    accept(&mut r, 1);
    settle_declining(&mut r);
    assert_eq!(field_top_ids(&r, 1), owned(&["SCAPEGOAT"]));
    assert_eq!(trash_ids(&r, 1), owned(&[FILL]));

    let (mut r, goat) = scapegoat_board(1, &[FILL], &[]);
    attack_with_lv5(&mut r, goat);
    assert!(
        advance_to(&mut r, optional_for(1)),
        "battle is not one of P1's effects: offered"
    );
    accept(&mut r, 1);
    settle_declining(&mut r);
    assert_eq!(field_top_ids(&r, 1), owned(&["SCAPEGOAT"]));
    assert_eq!(trash_ids(&r, 1), owned(&[FILL]));

    let (mut r, _) = scapegoat_board(0, &[FILL], &["MAIN_DEL_OWN_MEM"]);
    delete_own(&mut r, 0);
    let prompts = settle_declining(&mut r);
    assert!(
        !offered_to(&prompts, 0),
        "own effect: no <Scapegoat>: {prompts:?}"
    );
    assert_eq!(field_top_ids(&r, 0), owned(&[FILL, "TAMER_PLAIN"]));
}

/// §16-32-2 "<Scapegoat> is an IMMEDIATE-TYPE effect that triggers when the
/// Digimon with this effect WOULD be deleted ..."
///
/// When P1 is offered <Scapegoat>, `SCAPEGOAT` is in the battle area and P1's
/// trash is empty; after accepting, `SCAPEGOAT` was never trashed.
#[test]
fn scapegoat_kind_matches_rule() {
    cite!(
        "16-32-2",
        "85d9685d7cda979c34246eda204c87f4ca2db09cc0b7d5e6415af5765a2a802a"
    );
    let (mut r, _) = scapegoat_board(1, &[FILL], &["MAIN_DEL1"]);
    let cp = r.event_checkpoint();
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    assert!(advance_to(&mut r, optional_for(1)));
    assert_eq!(field_top_ids(&r, 1), owned(&["SCAPEGOAT", FILL]));
    assert!(trash_ids(&r, 1).is_empty());
    accept(&mut r, 1);
    settle_declining(&mut r);
    assert!(!trashed_since(&r, cp, 1, "SCAPEGOAT"), "never deleted");
}

/// §16-32-3 ""By deleting 1 of your other Digimon" in <Scapegoat> is an
/// optional processing condition. If the content of that condition is
/// executed, the processing for "prevents the deletion" will be mandatory."
///
/// - P1 declines: `SCAPEGOAT` is deleted, `FILL` and `BLACK_LV3` stay.
/// - P1 accepts with two other Digimon: P1 chooses WHICH to delete (a field
///   pick that can't be declined: the condition was chosen; only P1's own
///   other Digimon, never P0's `LV4_PLAIN` -- "1 of YOUR other Digimon"), picks
///   `BLACK_LV3`, and with no further decision `SCAPEGOAT` is not deleted.
///   (Matrix row `Scapegoat`: `optional_cost_then_mandatory`, agrees.)
#[test]
fn scapegoat_optionality_matches_rule() {
    cite!(
        "16-32-3",
        "9d7c4328107b9624b2c573348d2fbd8bcac5533082210eba1f78efa8a7c5ad0a"
    );
    let (mut r, _) = scapegoat_board(1, &[FILL, "BLACK_LV3"], &["MAIN_DEL1"]);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    assert!(advance_to(&mut r, optional_for(1)));
    r.decline_optional_trigger().expect("decline <Scapegoat>");
    settle_declining(&mut r);
    assert_eq!(trash_ids(&r, 1), owned(&["SCAPEGOAT"]));
    assert_eq!(field_top_ids(&r, 1), owned(&[FILL, "BLACK_LV3"]));

    let (mut r, _) = scapegoat_board(1, &[FILL, "BLACK_LV3"], &["MAIN_DEL1"]);
    r.place_on_field(0, "LV4_PLAIN", Some(0));
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    assert!(advance_to(&mut r, optional_for(1)));
    accept(&mut r, 1);
    let pick = current_prompt(&r).expect("P1 chooses the Digimon to delete");
    assert_eq!(pick.player, 1);
    assert!(
        !pick.optional,
        "the chosen condition can't be skipped: {pick:?}"
    );
    assert_eq!(pick.kind, SelectionKind::OwnField, "{pick:?}");
    let mut valid = r.pending_selection().unwrap().valid_action_ids.clone();
    valid.sort();
    assert_eq!(
        valid,
        vec![ATTACK_START + 1, ATTACK_START + 2],
        "only P1's OTHER Digimon: not SCAPEGOAT, not P0's LV4_PLAIN"
    );
    pick_field(&mut r, 2);
    let after = settle_declining(&mut r);
    assert!(!offered_to(&after, 1), "{after:?}");
    assert_eq!(trash_ids(&r, 1), owned(&["BLACK_LV3"]));
    assert_eq!(field_top_ids(&r, 1), owned(&["SCAPEGOAT", FILL]));
}

/// §16-32-2 "<Scapegoat> ... triggers when the Digimon with this effect WOULD
/// BE DELETED other than by one of your effects."
/// §15-7-4 "If executing an optional processing condition's contents isn't
/// possible, then the player can't choose to execute them. ..."
///
/// - Leaving without deletion: P0's `MAIN_BOUNCE1` returns P1's `SCAPEGOAT` to
///   the hand: no <Scapegoat>.
/// - With no other Digimon to delete the condition can't be executed: P0's
///   `MAIN_DEL1` deletes P1's lone `SCAPEGOAT` with no <Scapegoat> offered.
#[test]
fn scapegoat_timing_matches_rule() {
    cite!(
        "16-32-2",
        "85d9685d7cda979c34246eda204c87f4ca2db09cc0b7d5e6415af5765a2a802a"
    );
    cite!(
        "15-7-4",
        "210ce11e09b84d8a05b2b4861fcbff3ad5d398894a7da5879d8c869dc6446b14"
    );
    let (mut r, _) = scapegoat_board(1, &[FILL], &["MAIN_BOUNCE1"]);
    use_on_opponent_slot(&mut r, "MAIN_BOUNCE1", 0);
    let prompts = settle_declining(&mut r);
    assert!(
        !offered_to(&prompts, 1),
        "not deleted: no <Scapegoat>: {prompts:?}"
    );
    assert_eq!(hand_ids(&r, 1), owned(&["SCAPEGOAT"]));

    let (mut r, _) = scapegoat_board(1, &[], &["MAIN_DEL1"]);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    let prompts = settle_declining(&mut r);
    assert!(
        !offered_to(&prompts, 1),
        "no other Digimon: <Scapegoat> can't be paid, not offered: {prompts:?}"
    );
    assert_eq!(trash_ids(&r, 1), owned(&["SCAPEGOAT"]));
}

// ─── §16-36 <Decode> ────────────────────────────────────────────────────────

/// §16-36-1 "<Decode> is a keyword effect. When the Digimon with this effect
/// would leave the battle area other than by a battle, you may play 1
/// specified Digimon card from that Digimon's digivolution cards without
/// paying the cost. This is shown on cards using text such as "<Decode (blue
/// Lv.4)>.""
/// §16-36-2 "<Decode> is an immediate-type effect that triggers when the
/// Digimon with this effect would leave the battle area other than by a
/// battle."
///
/// - P0's `MAIN_BOUNCE1` returns P1's `[BLUE_LV4, DECODE]` to the hand. P1
///   accepts <Decode>: `BLUE_LV4` is played without paying (P1's memory is
///   unchanged) and `DECODE` goes to the hand.
/// - "Play 1": on `[BLUE_LV4, BLUE_LV4, DECODE]` P1 chooses one blue Lv.4;
///   only it is played, the other is trashed.
/// - "Other than by a battle" includes its owner's own effects: P0's
///   `MAIN_DEL_OWN_MEM` deletes P0's own `[BLUE_LV4, DECODE]`: offered;
///   accepting plays `BLUE_LV4`.
/// - In battle: P0's `LV5_PLAIN` (7000 DP) attacks P1's suspended `DECODE`
///   (5000 DP): not offered; both cards are trashed.
#[test]
fn decode_plays_specified_card_not_in_battle() {
    cite!(
        "16-36-1",
        "8ce4a32cd674ba8e0dcf62ede2774eb6f60b895364f76918205608e15fc1f0b9"
    );
    cite!(
        "16-36-2",
        "c56e75b6c5dfdcab38792c2c60d7274cbe6df995348411282033cee98288078a"
    );
    let (mut r, _) = p1_stack_board(&["BLUE_LV4", "DECODE"], &["MAIN_BOUNCE1"]);
    use_on_opponent_slot(&mut r, "MAIN_BOUNCE1", 0);
    assert!(
        advance_to(&mut r, optional_for(1)),
        "P1 is offered <Decode>"
    );
    let memory = r.memory();
    accept(&mut r, 1);
    settle_declining(&mut r);
    assert_eq!(field_top_ids(&r, 1), owned(&["BLUE_LV4"]));
    assert_eq!(hand_ids(&r, 1), owned(&["DECODE"]));
    assert_eq!(r.memory(), memory, "played without paying the cost");

    // "Play 1": with two blue Lv.4 cards under it, P1 chooses one (a pick
    // that can't be declined) and only that one is played.
    let (mut r, decode) = p1_stack_board(&["BLUE_LV4", "BLUE_LV4", "DECODE"], &["MAIN_BOUNCE1"]);
    use_on_opponent_slot(&mut r, "MAIN_BOUNCE1", 0);
    assert!(advance_to(&mut r, optional_for(1)));
    accept(&mut r, 1);
    let pick = current_prompt(&r).expect("P1 chooses which blue Lv.4 to play");
    assert!(!pick.optional, "{pick:?}");
    assert_eq!(r.pending_action_count(), 2, "{:?}", r.pending_selection());
    pick_source(&mut r, decode, "BLUE_LV4");
    settle_declining(&mut r);
    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["BLUE_LV4"]),
        "exactly 1 played"
    );
    assert_eq!(
        trash_ids(&r, 1),
        owned(&["BLUE_LV4"]),
        "the other is trashed"
    );
    assert_eq!(hand_ids(&r, 1), owned(&["DECODE"]));

    let mut r = board(&["MAIN_DEL_OWN_MEM"]);
    r.place_stack(0, &["BLUE_LV4", "DECODE"]);
    red_anchor(&mut r);
    delete_own(&mut r, 0);
    assert!(
        advance_to(&mut r, optional_for(0)),
        "own effect: <Decode> is offered"
    );
    accept(&mut r, 0);
    settle_declining(&mut r);
    assert_eq!(
        sorted(field_top_ids(&r, 0)),
        owned(&["BLUE_LV4", "TAMER_PLAIN"])
    );
    assert!(trash_ids(&r, 0).contains(&"DECODE".to_string()));

    let (mut r, decode) = p1_stack_board(&["BLUE_LV4", "DECODE"], &[]);
    attack_with_lv5(&mut r, decode);
    let prompts = settle_declining(&mut r);
    assert!(!offered_to(&prompts, 1), "battle: no <Decode>: {prompts:?}");
    assert_eq!(trash_ids(&r, 1), owned(&["BLUE_LV4", "DECODE"]));
}

/// §16-36-2 "<Decode> is an IMMEDIATE-TYPE effect that triggers when the
/// Digimon with this effect WOULD leave the battle area ..."
///
/// When P1 is offered <Decode>, `DECODE` is in the battle area with
/// `BLUE_LV4` under it and P1's hand is empty.
#[test]
fn decode_kind_matches_rule() {
    cite!(
        "16-36-2",
        "c56e75b6c5dfdcab38792c2c60d7274cbe6df995348411282033cee98288078a"
    );
    let (mut r, decode) = p1_stack_board(&["BLUE_LV4", "DECODE"], &["MAIN_BOUNCE1"]);
    use_on_opponent_slot(&mut r, "MAIN_BOUNCE1", 0);
    assert!(advance_to(&mut r, optional_for(1)));
    assert_eq!(stack_ids(&r, decode), owned(&["BLUE_LV4", "DECODE"]));
    assert!(hand_ids(&r, 1).is_empty());
}

/// §16-36-3 "The processing from <Decode> is optional."
///
/// - Accepting: <Decode> asks ONE optional decision; the play that follows is
///   not a second "may".
/// - Declining: nothing is played; `DECODE` goes to the hand and `BLUE_LV4`
///   to the trash. (Matrix row `Decode`: `optional`, agrees.)
#[test]
fn decode_optionality_matches_rule() {
    cite!(
        "16-36-3",
        "5f1e7b945e64359ab07323febc71ea9dcb347eed61687dbad13bc11bfc514fa3"
    );
    let (mut r, _) = p1_stack_board(&["BLUE_LV4", "DECODE"], &["MAIN_BOUNCE1"]);
    use_on_opponent_slot(&mut r, "MAIN_BOUNCE1", 0);
    assert!(advance_to(&mut r, optional_for(1)));
    accept(&mut r, 1);
    let after = settle_declining(&mut r);
    assert!(
        !offered_to(&after, 1),
        "one optional decision only: {after:?}"
    );
    assert_eq!(field_top_ids(&r, 1), owned(&["BLUE_LV4"]));

    let (mut r, _) = p1_stack_board(&["BLUE_LV4", "DECODE"], &["MAIN_BOUNCE1"]);
    use_on_opponent_slot(&mut r, "MAIN_BOUNCE1", 0);
    assert!(advance_to(&mut r, optional_for(1)));
    r.decline_optional_trigger().expect("decline <Decode>");
    settle_declining(&mut r);
    assert!(field_top_ids(&r, 1).is_empty());
    assert_eq!(hand_ids(&r, 1), owned(&["DECODE"]));
    assert_eq!(trash_ids(&r, 1), owned(&["BLUE_LV4"]));
}

/// §16-36-2 "<Decode> ... triggers when the Digimon with this effect would
/// LEAVE THE BATTLE AREA other than by a battle."
///
/// Deletion is leaving the battle area: P0's `MAIN_DEL1` deletes P1's
/// `[BLUE_LV4, DECODE]`; P1 is offered <Decode>, and accepting plays
/// `BLUE_LV4`.
#[test]
fn decode_timing_matches_rule() {
    cite!(
        "16-36-2",
        "c56e75b6c5dfdcab38792c2c60d7274cbe6df995348411282033cee98288078a"
    );
    let (mut r, _) = p1_stack_board(&["BLUE_LV4", "DECODE"], &["MAIN_DEL1"]);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    assert!(
        advance_to(&mut r, optional_for(1)),
        "deletion by an effect: <Decode> is offered"
    );
    accept(&mut r, 1);
    settle_declining(&mut r);
    assert_eq!(field_top_ids(&r, 1), owned(&["BLUE_LV4"]));
    assert_eq!(trash_ids(&r, 1), owned(&["DECODE"]));
}

// ─── §16-37 <Fragment> ──────────────────────────────────────────────────────

const FRAG_FULL: &[&str] = &[FILL, "LV4_PLAIN", "BLUE_LV4", "FRAG2"];

/// §16-37-1 "<Fragment> is a keyword effect. When a Digimon with this effect
/// would be deleted, by choosing and trashing the specified number of this
/// Digimon's digivolution cards, it isn't deleted. The effect is indicated on
/// cards by icons such as <Fragment (3)>."
/// §16-37-3 "The processing from <Fragment> for choosing and trashing the
/// specified number of this Digimon's digivolution cards is optional. If the
/// content of that condition is executed, the processing for "prevents the
/// deletion" will be mandatory."
/// §15-7-4 "If executing an optional processing condition's contents isn't
/// possible, then the player can't choose to execute them. ..."
///
/// - P0's `MAIN_DEL1` targets P1's `[FILL, LV4_PLAIN, BLUE_LV4, FRAG2]`
///   (<Fragment (2)>, 3 digivolution cards). P1 accepts and CHOOSES 2: picks
///   `BLUE_LV4` (the pick can't be skipped, and one pick is not enough: a
///   second is asked), then `LV4_PLAIN`. Exactly those 2 are trashed, `FILL`
///   stays, and with no further decision `FRAG2` is not deleted.
/// - With only 1 digivolution card (`[FILL, FRAG2]`) 2 can't be trashed: not
///   offered, `FRAG2` is deleted.
#[test]
fn fragment_trashes_exact_count_of_chosen_sources() {
    cite!(
        "16-37-1",
        "4213638a32077a3279181f82230d3b419cc8a970fb9465c004ed42ac0dadf843"
    );
    cite!(
        "16-37-3",
        "e124e528a7eaab94a3f2377236f8728b4d74eec78c4458bc9d8187736ab7d666"
    );
    cite!(
        "15-7-4",
        "210ce11e09b84d8a05b2b4861fcbff3ad5d398894a7da5879d8c869dc6446b14"
    );
    let (mut r, frag) = p1_stack_board(FRAG_FULL, &["MAIN_DEL1"]);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    assert!(
        advance_to(&mut r, optional_for(1)),
        "P1 is offered <Fragment>"
    );
    accept(&mut r, 1);
    let first = current_prompt(&r).expect("P1 chooses the cards to trash");
    assert!(
        !first.optional,
        "the chosen condition can't be skipped: {first:?}"
    );
    assert_eq!(
        r.pending_action_count(),
        3,
        "any of the 3 digivolution cards: {:?}",
        r.pending_selection()
    );
    pick_source(&mut r, frag, "BLUE_LV4");
    assert!(
        r.pending_selection().is_some(),
        "1 of 2: the specified number is not reached yet"
    );
    assert!(!trashed_since(&r, 0, 1, "FRAG2"));
    pick_source(&mut r, frag, "LV4_PLAIN");
    let after = settle_declining(&mut r);
    assert!(!offered_to(&after, 1), "{after:?}");
    assert_eq!(stack_ids(&r, frag), owned(&[FILL, "FRAG2"]), "not deleted");
    assert_eq!(trash_ids(&r, 1), owned(&["BLUE_LV4", "LV4_PLAIN"]));

    let (mut r, _) = p1_stack_board(&[FILL, "FRAG2"], &["MAIN_DEL1"]);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    let prompts = settle_declining(&mut r);
    assert!(
        !offered_to(&prompts, 1),
        "1 digivolution card < 2: not offered: {prompts:?}"
    );
    assert_eq!(trash_ids(&r, 1), sorted(owned(&[FILL, "FRAG2"])));
}

/// §16-37-2 "<Fragment> is an IMMEDIATE-TYPE effect that triggers when the
/// Digimon with this effect WOULD be deleted."
///
/// When P1 is offered <Fragment>, the stack is intact in the battle area and
/// P1's trash is empty; after paying, `FRAG2` was never trashed.
#[test]
fn fragment_kind_matches_rule() {
    cite!(
        "16-37-2",
        "1373e29ae8dda3f28eceeb477aa0eb8bd27e9819a45ec21c9bc6330c21f595d3"
    );
    let (mut r, frag) = p1_stack_board(FRAG_FULL, &["MAIN_DEL1"]);
    let cp = r.event_checkpoint();
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    assert!(advance_to(&mut r, optional_for(1)));
    assert_eq!(stack_ids(&r, frag), owned(FRAG_FULL));
    assert!(trash_ids(&r, 1).is_empty());
    accept(&mut r, 1);
    pick_source(&mut r, frag, "BLUE_LV4");
    pick_source(&mut r, frag, "LV4_PLAIN");
    settle_declining(&mut r);
    assert!(!trashed_since(&r, cp, 1, "FRAG2"), "never deleted");
}

/// §16-37-3 "The processing from <Fragment> for choosing and trashing the
/// specified number of this Digimon's digivolution cards is OPTIONAL. ..."
///
/// P1 declines: the whole stack is deleted. (Matrix row `Fragment`:
/// `optional_cost_then_mandatory`, agrees.)
#[test]
fn fragment_optionality_matches_rule() {
    cite!(
        "16-37-3",
        "e124e528a7eaab94a3f2377236f8728b4d74eec78c4458bc9d8187736ab7d666"
    );
    let (mut r, _) = p1_stack_board(FRAG_FULL, &["MAIN_DEL1"]);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    assert!(advance_to(&mut r, optional_for(1)));
    r.decline_optional_trigger().expect("decline <Fragment>");
    settle_declining(&mut r);
    assert!(field_top_ids(&r, 1).is_empty());
    assert_eq!(trash_ids(&r, 1), sorted(owned(FRAG_FULL)));
}

/// §16-37-2 "<Fragment> ... triggers when the Digimon with this effect WOULD
/// BE DELETED."
///
/// - In battle: P0's `LV5_PLAIN` (7000 DP) attacks P1's suspended `FRAG2`
///   (5000 DP): offered; paying 2 sources keeps it on the field.
/// - Leaving without deletion: P0's `MAIN_BOUNCE1` returns it to the hand: no
///   <Fragment>.
#[test]
fn fragment_timing_matches_rule() {
    cite!(
        "16-37-2",
        "1373e29ae8dda3f28eceeb477aa0eb8bd27e9819a45ec21c9bc6330c21f595d3"
    );
    let (mut r, frag) = p1_stack_board(FRAG_FULL, &[]);
    attack_with_lv5(&mut r, frag);
    assert!(
        advance_to(&mut r, optional_for(1)),
        "deleted in battle: <Fragment> is offered"
    );
    accept(&mut r, 1);
    pick_source(&mut r, frag, "BLUE_LV4");
    pick_source(&mut r, frag, "LV4_PLAIN");
    settle_declining(&mut r);
    assert_eq!(stack_ids(&r, frag), owned(&[FILL, "FRAG2"]), "not deleted");

    let (mut r, _) = p1_stack_board(FRAG_FULL, &["MAIN_BOUNCE1"]);
    use_on_opponent_slot(&mut r, "MAIN_BOUNCE1", 0);
    let prompts = settle_declining(&mut r);
    assert!(
        !offered_to(&prompts, 1),
        "not deleted: no <Fragment>: {prompts:?}"
    );
    assert_eq!(hand_ids(&r, 1), owned(&["FRAG2"]));
}

// ─── §16-1 .. §16-3 keyword generics; §15-8-5-3 / -4 ────────────────────────

/// §16-3 ""Specified cards" for a keyword effect refers to the cards that meet
/// the conditions shown in parentheses for icons that use angle brackets."
///
/// The parenthetical, not the keyword, decides which cards qualify:
/// - <Decoy (Black)>: P1's black `BLACK_LV3` is protected (offered), P1's
///   red `FILL` is not.
/// - <Decode (blue Lv.4)> on `[LV4_PLAIN, BLUE_LV4, DECODE]` (a red Lv.4 and
///   a blue Lv.4 under it): after P1 accepts, the red `LV4_PLAIN` can't be
///   chosen; `BLUE_LV4` is played and `LV4_PLAIN` is trashed.
/// - <Partition (blue Lv.4 & green Lv.4)> on `[BLUE_LV4, LV4_PLAIN,
///   PARTITION]`: a red Lv.4 is not the green Lv.4, so it doesn't trigger.
#[test]
fn specified_cards_follow_the_icon_note() {
    cite!(
        "16-3",
        "8fa3e3e8da5ad9f99985974190d2aeca9b57ce5b539a988dd740440d05aba66d"
    );
    let (mut r, _) = decoy_board(1, &["MAIN_DEL1"]);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 1);
    assert!(
        advance_to(&mut r, optional_for(1)),
        "black: specified by <Decoy (Black)>"
    );
    let (mut r, _) = decoy_board(1, &["MAIN_DEL1"]);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 2);
    let prompts = settle_declining(&mut r);
    assert!(!offered_to(&prompts, 1), "red: not specified: {prompts:?}");

    let (mut r, decode) = p1_stack_board(&["LV4_PLAIN", "BLUE_LV4", "DECODE"], &["MAIN_BOUNCE1"]);
    use_on_opponent_slot(&mut r, "MAIN_BOUNCE1", 0);
    assert!(advance_to(&mut r, optional_for(1)));
    accept(&mut r, 1);
    if r.pending_selection().is_some() {
        assert!(source_action(&r, decode, 0).is_none(),
            "red Lv.4 is not a specified card and has no legal row");
        let blue = source_action(&r, decode, 1).expect("blue Lv.4 is the specified physical card");
        assert!(r.pending_selection().unwrap().valid_action_ids.contains(&blue));
    }
    settle_declining(&mut r);
    assert_eq!(field_top_ids(&r, 1), owned(&["BLUE_LV4"]));
    assert_eq!(trash_ids(&r, 1), owned(&["LV4_PLAIN"]));

    let (mut r, _) = p1_stack_board(&["BLUE_LV4", "LV4_PLAIN", "PARTITION"], &["MAIN_DEL1"]);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    let prompts = settle_declining(&mut r);
    assert!(
        !offered_to(&prompts, 1),
        "no green Lv.4 among the digivolution cards: {prompts:?}"
    );
}

/// §16-1 "Keyword effects are effects indicated by icons enclosed in
/// brackets."
/// §15-8-5-4 "Each immediate-type effect can be activated once until the cause
/// that first interrupted the immediate-type effect is resolved. ..."
///
/// Ver. 4.3 deleted the per-keyword "if a Digimon has multiple instances of
/// <X>" rules; what remains is that each instance is its own immediate-type
/// effect (§16-1), each usable once per cause (§15-8-5-4).
/// - Recognition: the bracketed icon `＜Barrier＞` parses to the keyword effect.
/// - Two instances: P1's `[BARRIER_INH, BARRIER]` has <Barrier> on its top
///   card and an inherited <Barrier>. Losing a battle to P0's `LV5_PLAIN`, P1
///   first chooses which instance to use (an `EffectChoice`), and is then
///   offered <Barrier> TWICE in that one deletion: declining the first leaves
///   the other instance; accepting it trashes ONE security card (the top,
///   `BLUE_LV4`) and the Digimon survives. Neither is offered a second time.
///
/// The second-EVENT half of §15-8-5-4 (a new deletion is a new cause) is in
/// `ch15_08_effect_categories::immediate_effect_once_per_interrupting_cause`.
#[test]
fn multiple_instances_of_one_keyword_each_offered_once_per_event() {
    cite!(
        "16-1",
        "ebab731ed17c952fefbe8e74966b8142d0e8fa4c59bb844e8d44aae6e1b9674d"
    );
    cite!(
        "15-8-5-4",
        "7586ddc0ccc9332f3ced38f131201eceaf1856a8343ac14248891fafc18fb30b"
    );
    assert_eq!(
        parse_printed_keywords("＜Barrier＞", "", ""),
        vec![Keyword::Barrier]
    );

    let mut r = board_with(ch16_repl(), &[], &[FILL; 3], SEC_TOP_BLUE);
    let h = r.place_stack(1, &["BARRIER_INH", "BARRIER"]);
    red_anchor(&mut r);
    attack_with_lv5(&mut r, h);
    let mut offers = 0;
    let mut ordered = false;
    let mut settled = false;
    for _ in 0..12 {
        let Some(p) = current_prompt(&r) else {
            settled = true;
            break;
        };
        if p.optional && p.player == 1 {
            offers += 1;
            match offers {
                1 => r.decline_optional_trigger().expect("decline the first"),
                2 => accept(&mut r, 1),
                _ => panic!("a <Barrier> instance was offered again: {p:?}"),
            }
        } else if p.player == 1 && p.kind == SelectionKind::EffectChoice {
            // Which instance first: P1's ordering choice.
            ordered = true;
            let action = r.pending_selection().unwrap().valid_action_ids[0];
            r.execute_action(1, action).expect("order pick");
        } else {
            step_prompt(&mut r);
        }
    }
    assert!(settled, "decisions did not settle after 12 steps");
    assert!(ordered, "P1 chose which <Barrier> instance to use first");
    assert_eq!(offers, 2, "each <Barrier> instance is offered once");
    assert_eq!(field_top_ids(&r, 1), owned(&["BARRIER"]), "prevented");
    assert_eq!(
        security_ids(&r, 1),
        owned(&[FILL, FILL]),
        "one instance paid once"
    );
}

/// §16-2 "A keyword effect is considered to be the same type even if it has a
/// different numerical value or specified card included in the icon enclosed
/// in brackets."
///
/// A reference to the keyword by name matches every value of it. P0's
/// `MAIN_DEL_FRAG` ("[Main] Delete 1 of your opponent's Digimon with
/// <Fragment>") can choose P1's `FRAG2` (<Fragment (2)>) and P1's
/// `EVADE_FRAG1` (<Fragment (1)>), but not P1's `FILL` (no <Fragment>).
///
/// Supporting evidence only (not counted toward the obligation): the printed
/// icons `＜Fragment (1)＞`/`＜Fragment (2)＞` and `＜Decoy＞`/`＜Decoy (Black)＞`
/// parse to one `Keyword` variant each.
#[test]
fn keyword_reference_matches_every_value_of_the_keyword() {
    cite!(
        "16-2",
        "25a61affcf23dd9a1afcd3484a1e3b3436b81c9211cd6491a7044ce3dcf8e8c9"
    );
    let kw = |text: &str| parse_printed_keywords(text, "", "");
    let (f1, f2) = (kw("＜Fragment (1)＞"), kw("＜Fragment (2)＞"));
    assert_eq!(
        std::mem::discriminant(&f1[0]),
        std::mem::discriminant(&f2[0])
    );
    let (d0, d1) = (kw("＜Decoy＞"), kw("＜Decoy (Black)＞"));
    assert_eq!(
        std::mem::discriminant(&d0[0]),
        std::mem::discriminant(&d1[0])
    );

    let mut r = board(&["MAIN_DEL_FRAG"]);
    r.place_on_field(1, "FRAG2", Some(0));
    r.place_on_field(1, "EVADE_FRAG1", Some(0));
    r.place_on_field(1, FILL, Some(0));
    red_anchor(&mut r);
    use_option(&mut r, 0, "MAIN_DEL_FRAG");
    let sel = r
        .pending_selection()
        .expect("P0 chooses the Digimon to delete");
    let mut valid = sel.valid_action_ids.clone();
    valid.sort();
    assert_eq!(
        valid,
        vec![ATTACK_START, ATTACK_START + 1],
        "<Fragment (2)> and <Fragment (1)> both qualify, FILL doesn't: {sel:?}"
    );
}

/// §15-8-5-3 "Immediate-type effects will only trigger simultaneously with
/// other immediate-type effects."
/// §15-4-3-5-2 "Once there are no more pending activation effects for the turn
/// player, the non-turn player chooses 1 of their effects to activate from
/// among the effects that triggered simultaneously. ..."
///
/// P1's `[FILL, EVADE_FRAG1]` has <Evade> and <Fragment (1)>, both triggered by
/// one "would be deleted" (P0's `MAIN_DEL1`). P1 (the non-turn player) chooses
/// which to use first:
/// - <Fragment (1)> first: `FILL` is trashed, the Digimon survives
///   UNSUSPENDED (any <Evade> offer afterwards is declined).
/// - <Evade> first: the Digimon is suspended and survives with `FILL` still
///   under it (any <Fragment> offer afterwards is declined).
#[test]
fn two_simultaneous_replacements_ordered_by_controller() {
    cite!(
        "15-8-5-3",
        "f848e083dad9a7afa537feb0cd86d95d2c53ecd40f071ef2aa2aac79948148e8"
    );
    cite!(
        "15-4-3-5-2",
        "e71f29e0e84022ff4c506913e8c99313a93bc72e23d18d5a0a140dcf63c3235d"
    );
    for (first, suspended, stack) in [
        ("<Fragment", false, owned(&["EVADE_FRAG1"])),
        ("<Evade", true, owned(&[FILL, "EVADE_FRAG1"])),
    ] {
        let (mut r, h) = p1_stack_board(&[FILL, "EVADE_FRAG1"], &["MAIN_DEL1"]);
        use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
        let order = current_prompt(&r).expect("P1 chooses which effect to use first");
        assert_eq!(order.player, 1, "{order:?}");
        assert!(
            order.kind == SelectionKind::EffectChoice && !order.optional,
            "an ordering choice, not a yes/no: {order:?}"
        );
        assert!(
            order.labels.iter().any(|l| l.starts_with("<Evade"))
                && order.labels.iter().any(|l| l.starts_with("<Fragment")),
            "both immediate-type effects are offered as one choice: {order:?}"
        );
        let action = r
            .pending_selection()
            .unwrap()
            .effect_choices
            .iter()
            .flatten()
            .find(|c| c.label.starts_with(first))
            .map(|c| c.action_id)
            .unwrap();
        r.execute_action(1, action)
            .expect("choose the first effect");
        accept(&mut r, 1);
        settle_declining(&mut r);
        assert_eq!(
            field_top_ids(&r, 1),
            owned(&["EVADE_FRAG1"]),
            "{first}: prevented"
        );
        assert_eq!(is_suspended(&r, h), suspended, "{first} first");
        assert_eq!(stack_ids(&r, h), stack, "{first} first");
    }
}
