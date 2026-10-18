//! Chapter 16 (Comprehensive Rules Ver. 4.3), the keywords added in Ver. 4.x
//! and never verified against their rule text before: `<Use Req.>` (§16-42),
//! `<Ascension>` (§16-43), `<Engage>` (§16-44), `<Guard>` (§16-45),
//! `<Detach>` (§16-46) and `<Succession>` (§16-47); with the Ver. 4.x card
//! mechanics they travel with: DUAL cards (§4-6), `<Overflow>` (§4-19) and
//! Arts Digivolve (§4-20).
//!
//! Every test asserts the RULE, never the engine's current output. A test the
//! engine fails is `#[ignore]`d with the gap id that explains it (discover then
//! pin); see `docs/RUST_ENGINE_GAPS.md` and `qa/rules/cr_obligations_status.yaml`.
//!
//! Per keyword: `<kw>_kind_matches_rule` checks the observable consequence of
//! the §16-N-2 effect type (a trigger-type effect pends with the other effects
//! triggered at the same time and activates in the order the player picks; an
//! immediate-type effect interrupts before its cause completes; a persistent
//! effect applies with no activation), `<kw>_optionality_matches_rule` the
//! §16-N-3 optional / mandatory split, `<kw>_timing_matches_rule` the
//! "triggers when ..." condition, plus one test per behavioural sub-rule.
//!
//! Engine reality: `Keyword::{Engage, Guard, Ascension, ArtsDigivolve}` exist;
//! `<Use Req.>`, `<Detach>`, `<Succession>` and `<Overflow>` do not, and are
//! proven through the production DSL idioms instead (card-level
//! `use_requirement`, a `replacement` clause with a `trash_own_link_card`
//! cost, `kind: succession`, and the `ace_overflow` card field). Each fixture
//! card's header names the idiom it uses.
//!
//! Fixture cards: `fixtures/common/*.yaml` + `fixtures/ch16_new/*.yaml` (plus
//! `LV4_PLAIN` / `EOT_MEM` / `MAIN_DEL1` / `MAIN_DEL_ALL` from `ch15_08` and
//! `LINK_A` / `MAIN_PLACE_SELF` from `ch17`), one synthetic card per file,
//! authored to printed-style text (each clause's `summary:` is that text).
//!
//! Security stacks are listed bottom first: the LAST card of `.security(..)`
//! is the top card.

use crate::fixtures;
use crate::support::*;
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    encode_attack, encode_digivolve, ATTACK_START, PASS, PLAY_HAND_START, SECURITY_TARGET,
};
use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{GamePhase, PlayerId};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{OptionPlayResult, SelectionKind, SourceSelectionRef};

fn ch16_new() -> DebugRunnerBuilder {
    fixtures::builder(&[
        fixtures::ch15_04_cards(),
        fixtures::ch15_08_cards(),
        fixtures::ch17_cards(),
        fixtures::ch16_new_cards(),
    ])
}

/// P0 (memory 5, hand `hand0`) to move in its main phase; both players have
/// 6 deck cards and 3 security cards.
fn board(hand0: &[&str]) -> DebugRunner {
    ch16_new()
        .hand(0, hand0)
        .deck(0, &[FILL; 6])
        .deck(1, &[FILL; 6])
        .security(0, &[FILL; 3])
        .security(1, &[FILL; 3])
        .memory(5)
        .start()
}

/// Put the red `TAMER_PLAIN` (no text) on P0's field: it meets the color
/// requirement of P0's red Options without being a Digimon any effect here
/// can choose.
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

/// The `<Ascension>` "Yes / No" decision, owned by `player`.
fn yes_no_for(player: PlayerId) -> impl Fn(&Prompt) -> bool {
    move |p: &Prompt| p.player == player && p.labels == ["Yes", "No"]
}

/// Use the Option `option` from P0's hand and pick P1's battle-area `slot`
/// as its target.
fn use_on_opponent_slot(r: &mut DebugRunner, option: &str, slot: u16) {
    use_option(r, 0, option);
    pick_field(r, slot);
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

/// Whether `card_id` was trashed by `player` since `cp`.
fn trashed_since(r: &DebugRunner, cp: usize, player: PlayerId, card_id: &str) -> bool {
    r.events_since(cp).iter().any(|e| {
        matches!(e, GameEvent::Trash { player: p, card_id: c, .. } if *p == player && c == card_id)
    })
}

/// `(player, delta)` of every memory change with NO effect source (a rule
/// process such as `<Overflow>`) since `cp`, in event order.
fn rule_memory_moves(r: &DebugRunner, cp: usize) -> Vec<(PlayerId, i16)> {
    r.events_since(cp)
        .iter()
        .filter_map(|e| match e {
            GameEvent::MemoryChange {
                player,
                delta,
                source_card_id: None,
                ..
            } => Some((*player, *delta)),
            _ => None,
        })
        .collect()
}

/// Card ids in `player`'s security stack, bottom first (the last is the top).
fn security_ids(r: &DebugRunner, player: PlayerId) -> Vec<String> {
    card_ids(r, &r.game.player(player).security)
}

fn linked_ids(r: &DebugRunner, host: PermanentHandle) -> Vec<String> {
    card_ids(
        r,
        &r.game.player(host.player).battle_area[host.index as usize].linked_cards,
    )
}

fn stack_ids(r: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    card_ids(
        r,
        &r.game.player(h.player).battle_area[h.index as usize].card_sources,
    )
}

/// The action id that picks battle-area `slot` in an own-/opponent-field
/// selection (`OwnField` / `OppField` encode a slot as `ATTACK_START + slot`;
/// see `support::pick_field`).
fn field_pick_action(slot: u16) -> u16 {
    ATTACK_START + slot
}

/// Whether P0 may use the hand card at `index` now (its play / use bit).
fn can_use_from_hand(r: &DebugRunner, index: usize) -> bool {
    build_action_mask(&r.game, 0)[(PLAY_HAND_START as usize) + index] == 1.0
}

// ─── §16-42 <Use Req.> ──────────────────────────────────────────────────────

/// P0 holds `USE_REQ_OPT` (purple Option, "<Use Req. ([TS] trait)> [Main]
/// Gain 1 memory", use cost 1) and has `field` on its battle area.
fn use_req_board(field: &[&str]) -> DebugRunner {
    let mut r = board(&["USE_REQ_OPT"]);
    for id in field {
        r.place_on_field(0, id, Some(0));
    }
    r
}

/// §16-42-1 "<Use Req.> is a keyword effect that allows a player to ignore
/// the color requirements with the specified cards."
/// §16-42-3 "<Use Req.> allows a player to ignore the color requirements with
/// specific Digimon and/or Tamers on the field."
///
/// P0 holds the purple `USE_REQ_OPT` and has no purple card.
/// - Control: with only a red `FILL` on the field it can't be used (§1-3-11-1
///   color requirement); with a purple `PURPLE_LV3` it can.
/// - A red [TS] Tamer (`TS_TAMER`) or a red [TS] Digimon (`TS_DIGI`) on the
///   field lets P0 ignore the color requirement: the Option is usable, and
///   using it pays its cost and resolves its [Main].
/// - A [TS] Digimon in the HAND (not on the field) does not.
#[test]
fn use_req_ignores_color_requirements_with_specified_cards() {
    cite!(
        "16-42-1",
        "fb4905c7f7a4a936d95248fbae3959208ce364d675e69f0fcee37ab835780963"
    );
    cite!(
        "16-42-3",
        "db54467706f7f5b78a660bece28876796c3c9d76dd1a147c8c8328615cd81152"
    );
    let r = use_req_board(&[FILL]);
    assert!(
        !can_use_from_hand(&r, 0),
        "control: no purple card, no [TS] card"
    );
    let r = use_req_board(&["PURPLE_LV3"]);
    assert!(
        can_use_from_hand(&r, 0),
        "control: the color requirement is met"
    );

    for specified in ["TS_TAMER", "TS_DIGI"] {
        let mut r = use_req_board(&[FILL, specified]);
        assert!(
            can_use_from_hand(&r, 0),
            "{specified} on the field: the color requirement is ignored"
        );
        let cp = r.event_checkpoint();
        use_option(&mut r, 0, "USE_REQ_OPT");
        resolve_prompts(&mut r);
        assert_eq!(
            memory_sources(&r, cp),
            vec![(0, "USE_REQ_OPT".to_string())],
            "{specified}: the [Main] resolved"
        );
        assert_eq!(r.memory(), 5, "{specified}: paid 1, gained 1");
    }

    let mut r = use_req_board(&[FILL]);
    r.add_to_hand(0, "TS_DIGI");
    assert!(
        !can_use_from_hand(&r, 0),
        "a [TS] card in the hand is not on the field"
    );
}

/// §16-42-2 "<Use Req.> is a persistent effect."
///
/// A persistent effect applies whenever its condition holds, with no
/// activation and no pending state (§15-8-2). The moment P0's `TS_DIGI` is on
/// the field `USE_REQ_OPT` is usable, and nothing was activated, offered or
/// logged to make it so; P0 can take an unrelated action and the permission
/// is still there.
#[test]
fn use_req_kind_matches_rule() {
    cite!(
        "16-42-2",
        "850b00529efc64f0c14387d0e692f17d27405241f4af9be930c32d8d24f884fb"
    );
    let mut r = use_req_board(&[FILL]);
    let cp = r.event_checkpoint();
    r.place_on_field(0, "TS_DIGI", Some(0));
    assert!(
        r.pending_selection().is_none(),
        "nothing to activate or decide"
    );
    assert!(r.events_since(cp).is_empty(), "nothing activated");
    assert!(can_use_from_hand(&r, 0), "the permission applies at once");
}

/// §16-42-1 "<Use Req.> is a keyword effect that ALLOWS a player to ignore
/// the color requirements ..."
///
/// <Use Req.> is a permission, never an obligation or a restriction: it adds
/// no decision, and when the color requirement is met anyway (a purple card
/// on the field, no [TS] card) the Option is still usable as normal.
#[test]
fn use_req_optionality_matches_rule() {
    cite!(
        "16-42-1",
        "fb4905c7f7a4a936d95248fbae3959208ce364d675e69f0fcee37ab835780963"
    );
    for field in [&["PURPLE_LV3"][..], &["PURPLE_LV3", "TS_TAMER"][..]] {
        let mut r = use_req_board(field);
        assert!(can_use_from_hand(&r, 0), "{field:?}");
        let cp = r.event_checkpoint();
        use_option(&mut r, 0, "USE_REQ_OPT");
        let prompts = settle_declining(&mut r);
        assert!(
            prompts.iter().all(|p| !p.optional),
            "{field:?}: no 'use <Use Req.>?' decision: {prompts:?}"
        );
        assert_eq!(
            memory_sources(&r, cp),
            vec![(0, "USE_REQ_OPT".to_string())],
            "{field:?}"
        );
    }
}

/// §16-42-3 "<Use Req.> allows a player to ignore the color requirements with
/// specific Digimon and/or Tamers ON THE FIELD."
///
/// The requirement is read from the field at the time of use. P0's `TS_DIGI`
/// makes `USE_REQ_OPT` usable; once P0's own `MAIN_DEL_OWN_MEM` ("[Main]
/// Delete 1 of your Digimon. Then, gain 1 memory.") deletes it, the Option is
/// no longer usable; a `TS_TAMER` placed afterwards makes it usable again.
#[test]
fn use_req_timing_matches_rule() {
    cite!(
        "16-42-3",
        "db54467706f7f5b78a660bece28876796c3c9d76dd1a147c8c8328615cd81152"
    );
    let mut r = board(&["USE_REQ_OPT", "MAIN_DEL_OWN_MEM"]);
    r.place_on_field(0, "TS_DIGI", Some(0));
    assert!(can_use_from_hand(&r, 0), "TS_DIGI is on the field");

    use_option(&mut r, 0, "MAIN_DEL_OWN_MEM");
    resolve_prompts(&mut r);
    assert!(field_top_ids(&r, 0).is_empty(), "TS_DIGI was deleted");
    assert_eq!(hand_ids(&r, 0), owned(&["USE_REQ_OPT"]));
    assert!(
        !can_use_from_hand(&r, 0),
        "no [TS] card on the field any more"
    );

    r.place_on_field(0, "TS_TAMER", Some(0));
    assert!(
        can_use_from_hand(&r, 0),
        "a [TS] Tamer is on the field again"
    );
}

// ─── §16-43 <Ascension> ─────────────────────────────────────────────────────

/// P1 has `ASCEND` (<Ascension>) on its field and 2 security cards; P0 holds
/// `hand0`.
fn ascension_board(hand0: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = ch16_new()
        .hand(0, hand0)
        .deck(0, &[FILL; 6])
        .deck(1, &[FILL; 6])
        .security(1, &[FILL; 2])
        .memory(5)
        .start();
    red_anchor(&mut r);
    let h = r.place_on_field(1, "ASCEND", Some(0));
    (r, h)
}

/// §16-43-1 "<Ascension> is a keyword effect. When the card with this effect
/// is deleted, the player may place this card at the top of the security
/// stack."
///
/// P0's `MAIN_DEL1` deletes P1's `ASCEND`. P1 is asked whether to place it;
/// taking "Yes" moves `ASCEND` from the trash to the TOP of P1's security
/// stack (3 cards, `ASCEND` last).
#[test]
fn ascension_optional_places_on_top_of_security() {
    cite!(
        "16-43-1",
        "895f599dbce18901f27793e233be68b8b25c32c5ddcd98bbecee0c75c2eb1c3e"
    );
    let (mut r, _) = ascension_board(&["MAIN_DEL1"]);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    assert!(
        advance_to(&mut r, yes_no_for(1)),
        "P1 is asked whether to use <Ascension>"
    );
    assert_eq!(trash_ids(&r, 1), owned(&["ASCEND"]), "deleted first");
    r.execute_branch(0).expect("Yes");
    settle_declining(&mut r);

    assert!(field_top_ids(&r, 1).is_empty());
    assert!(trash_ids(&r, 1).is_empty(), "ASCEND left the trash");
    assert_eq!(
        security_ids(&r, 1),
        owned(&[FILL, FILL, "ASCEND"]),
        "ASCEND is the top security card"
    );
}

/// §16-43-3 "The processing from <Ascension> is optional."
///
/// The decision offers "Yes" and "No"; "No" leaves `ASCEND` in the trash and
/// the security stack unchanged. (Matrix row `Ascension`: `optional`, agrees.)
#[test]
fn ascension_optionality_matches_rule() {
    cite!(
        "16-43-3",
        "d351eb580ea166c2c1c397dce2a9b2b967d5d81ef5ee7825aa7741fa74f3b407"
    );
    let (mut r, _) = ascension_board(&["MAIN_DEL1"]);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    assert!(advance_to(&mut r, yes_no_for(1)));
    r.execute_branch(1).expect("No");
    settle_declining(&mut r);

    assert_eq!(
        trash_ids(&r, 1),
        owned(&["ASCEND"]),
        "declined: stays in the trash"
    );
    assert_eq!(security_ids(&r, 1), owned(&[FILL, FILL]));
}

/// §16-43-2 "<Ascension> is a trigger-type effect that triggers when the
/// Digimon with this effect is deleted."
///
/// A trigger-type effect activates after its cause and pends with the other
/// effects triggered at the same time (§15-8-3). P0's `MAIN_DEL_ALL` ("Delete
/// all of your opponent's Digimon") deletes P1's `ASCEND` and `OD_MEM`
/// ("[On Deletion] Gain 1 memory") together: P1 orders <Ascension> and the
/// [On Deletion] in one `TriggerOrder`, with both cards already in the trash.
/// P1 activates the [On Deletion] first, then <Ascension> places `ASCEND` on
/// top of the security stack.
#[test]
fn ascension_kind_matches_rule() {
    cite!(
        "16-43-2",
        "9f35a91d5a7e8b538c942c43ffb1f0981fbc37d2c49d82c9dab27f2b131cf337"
    );
    let (mut r, _) = ascension_board(&["MAIN_DEL_ALL"]);
    r.place_on_field(1, "OD_MEM", Some(0));
    let cp = r.event_checkpoint();
    use_option(&mut r, 0, "MAIN_DEL_ALL");
    assert!(
        advance_to(&mut r, |p| p.player == 1
            && p.kind == SelectionKind::TriggerOrder),
        "P1 orders its simultaneous triggers"
    );
    let mut cards: Vec<String> = order_choices(&r, 1)
        .iter()
        .map(|l| branch_card(l).to_string())
        .collect();
    cards.sort();
    assert_eq!(cards, owned(&["ASCEND", "OD_MEM"]), "both pending together");
    assert_eq!(
        trash_ids(&r, 1),
        owned(&["ASCEND", "OD_MEM"]),
        "trigger-type: the deletion completed before either activates"
    );

    choose(&mut r, "OD_MEM");
    assert!(
        advance_to(&mut r, yes_no_for(1)),
        "then <Ascension> activates"
    );
    r.execute_branch(0).expect("Yes");
    settle_declining(&mut r);
    assert_eq!(memory_sources(&r, cp), vec![(1, "OD_MEM".to_string())]);
    assert_eq!(security_ids(&r, 1), owned(&[FILL, FILL, "ASCEND"]));
}

/// §16-43-2 "<Ascension> is a trigger-type effect that triggers WHEN THE
/// DIGIMON WITH THIS EFFECT IS DELETED."
///
/// - Deleted in battle (P0's `LV5_PLAIN` attacks the suspended `ASCEND`):
///   <Ascension> triggers.
/// - Returned to the hand by P0's `MAIN_BOUNCE1`: it leaves the battle area
///   without being deleted, so <Ascension> does not trigger.
#[test]
fn ascension_timing_matches_rule() {
    cite!(
        "16-43-2",
        "9f35a91d5a7e8b538c942c43ffb1f0981fbc37d2c49d82c9dab27f2b131cf337"
    );
    let (mut r, ascend) = ascension_board(&[]);
    set_suspended(&mut r, ascend, true);
    let attacker = r.place_on_field(0, "LV5_PLAIN", Some(0));
    r.attack_digimon(attacker, ascend, false);
    assert!(
        advance_to(&mut r, yes_no_for(1)),
        "deleted in battle: <Ascension> triggers"
    );

    let (mut r, _) = ascension_board(&["MAIN_BOUNCE1"]);
    use_on_opponent_slot(&mut r, "MAIN_BOUNCE1", 0);
    let prompts = settle_declining(&mut r);
    assert_eq!(hand_ids(&r, 1), owned(&["ASCEND"]), "returned to the hand");
    assert!(
        !prompts.iter().any(|p| yes_no_for(1)(p)),
        "not deleted: no <Ascension>: {prompts:?}"
    );
    assert!(security_ids(&r, 1).len() == 2);
}

// ─── §16-44 <Engage> ────────────────────────────────────────────────────────

/// P0 (memory 5) has `ENGAGE` (<Engage>, 5000 DP, unsuspended) plus `extra`;
/// P1 has 3 security cards.
fn engage_board(extra: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = board(&[]);
    let engage = r.place_on_field(0, "ENGAGE", Some(0));
    for id in extra {
        r.place_on_field(0, id, Some(0));
    }
    (r, engage)
}

fn engage_attack_offered(r: &DebugRunner, engage: PermanentHandle) -> bool {
    r.current_phase() == GamePhase::EndOfTurnAction
        && build_action_mask(&r.game, 0)
            [encode_attack(engage.index as u16, SECURITY_TARGET) as usize]
            == 1.0
}

/// §16-44-1 "<Engage> is a keyword effect where a Digimon with this effect
/// may attack at the end of your turn."
///
/// P0 ends its turn: P0 is offered an attack with `ENGAGE` (and PASS). It
/// attacks P1: the attack checks P1's top security card, then the turn passes
/// to P1. Once the attack is over the end of turn continues with no further
/// decision (§16-44-1: the Digimon "may attack at the end of your turn";
/// nothing else is offered): no PASS-only step follows the attack.
#[test]
fn engage_optional_eot_attack() {
    cite!(
        "16-44-1",
        "83f9fd35763868aef8b7abb5a551e9a077c1cec4987d41294f0a22bc8c51abec"
    );
    let (mut r, engage) = engage_board(&[]);
    r.end_turn();
    settle_declining(&mut r);
    assert!(
        engage_attack_offered(&r, engage),
        "the end-of-turn attack is offered"
    );
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 1.0);

    let cp = r.event_checkpoint();
    r.game
        .decode_action(encode_attack(engage.index as u16, SECURITY_TARGET), 0);
    settle_declining(&mut r);
    assert_eq!(revealed(&r, cp, 1), owned(&[FILL]), "ENGAGE attacked");
    assert_eq!(r.security_count(1), 2);
    // §16-44-1: the attack was the end-of-turn action; the turn has already
    // passed, with no PASS-only `EndOfTurnAction` step after it.
    assert_ne!(
        r.current_phase(),
        GamePhase::EndOfTurnAction,
        "no PASS-only step after the attack"
    );
    assert_eq!(r.turn_player(), 1, "then the turn ended");
}

/// §16-44-3 "The processing from <Engage> is optional."
///
/// P0 ends its turn and declines the <Engage> attack (PASS): no attack, and
/// the turn passes to P1. (Matrix row `Engage`: `optional`, agrees.)
#[test]
fn engage_optionality_matches_rule() {
    cite!(
        "16-44-3",
        "bee7c4e4a1849d921b29b2f3fb56c5399293e7a77dc35885a4e51ab043d04291"
    );
    let (mut r, engage) = engage_board(&[]);
    let cp = r.event_checkpoint();
    r.end_turn();
    settle_declining(&mut r);
    assert!(engage_attack_offered(&r, engage));
    r.game.decode_action(PASS, 0);
    settle_declining(&mut r);
    assert!(revealed(&r, cp, 1).is_empty(), "declined: no attack");
    assert_eq!(r.security_count(1), 3);
    assert_eq!(r.turn_player(), 1);
}

/// §16-44-2 "<Engage> is a trigger-type effect that triggers at the End of
/// YOUR Turn timing."
///
/// P0 passes its turn without attacking; when P1 then ends ITS turn, P0's
/// `ENGAGE` (unsuspended, able to attack) does not trigger: no end-of-turn
/// attack is offered to either player and the turn passes back to P0.
#[test]
fn engage_timing_matches_rule() {
    cite!(
        "16-44-2",
        "79b60bcf0f08f7a04f7b1871680339ca5d113e9d2fe8b5302292d199840ba737"
    );
    let (mut r, engage) = engage_board(&[]);
    r.end_turn();
    settle_declining(&mut r);
    assert!(
        engage_attack_offered(&r, engage),
        "P0's end of turn: offered"
    );
    r.game.decode_action(PASS, 0);
    settle_declining(&mut r);
    assert_eq!(r.turn_player(), 1);

    let cp = r.event_checkpoint();
    r.end_turn();
    settle_declining(&mut r);
    assert_ne!(
        r.current_phase(),
        GamePhase::EndOfTurnAction,
        "P1's end of turn: P0's <Engage> does not trigger"
    );
    assert!(revealed(&r, cp, 1).is_empty());
    assert_eq!(r.turn_player(), 0, "the turn passed back to P0");
}

/// §16-44-2 "<Engage> is a TRIGGER-TYPE effect that triggers at the End of
/// Your Turn timing."
///
/// A trigger-type effect pends with the other effects triggered at the same
/// timing (§15-4-2-3, §15-8-3). P0 has `ENGAGE` and `EOT_MEM` ("[End of Your
/// Turn] Gain 2 memory"). At the end of P0's turn both pend in one
/// `TriggerOrder`. P0 activates <Engage> first: its processing ("this
/// Digimon may attack") is offered while `EOT_MEM` is still pending, and
/// `EOT_MEM` resolves only after the attack declaration.
///
/// Official Q&A Q3868 / Q3869 (EX7-064, <Vortex>, applied to <Engage> by
/// analogy: both are trigger-type end-of-turn "may attack" keywords):
/// activating the keyword first, "you can activate <Vortex> and make an
/// attack declaration, [the other] effect that triggered simultaneously will
/// activate, then the counter timing will occur." So the still-pending
/// effect resolves AFTER the attack declaration and BEFORE the counter timing
/// / security check; the test pins declaration < `EOT_MEM` < security check,
/// exactly as `ch16_attack_keywords::{vortex,execute,overclock}_kind_matches_rule`.
///
/// DCGO: `CardEffectFactory/KeyWordEffects/Engage.cs` builds <Engage> as an
/// `ActivateClass` on `OnEndTurn`; `EngageProcess` runs the whole attack
/// (`SelectAttackEffect.Activate()`) inside its `ActivateCoroutine`, so in
/// DCGO the other effect waits for the whole attack. The official ruling
/// governs over DCGO here.
#[test]
#[ignore = "G-ENGINE-ENGAGE-ATTACK-AFTER-END-OF-TURN-TRIGGERS (§16-44-2)"]
fn engage_kind_matches_rule() {
    cite!(
        "16-44-2",
        "79b60bcf0f08f7a04f7b1871680339ca5d113e9d2fe8b5302292d199840ba737"
    );
    let (mut r, engage) = engage_board(&["EOT_MEM"]);
    let cp = r.event_checkpoint();
    r.end_turn();
    let mut cards: Vec<String> = order_choices(&r, 0)
        .iter()
        .map(|l| branch_card(l).to_string())
        .collect();
    cards.sort();
    assert_eq!(cards, owned(&["ENGAGE", "EOT_MEM"]), "both pend together");

    choose(&mut r, "ENGAGE");
    assert!(
        memory_sources(&r, cp).is_empty(),
        "EOT_MEM is still pending while <Engage> is being processed: {:?}",
        memory_sources(&r, cp)
    );
    assert!(
        engage_attack_offered(&r, engage),
        "<Engage>'s attack is offered as its own processing"
    );
    r.game
        .decode_action(encode_attack(engage.index as u16, SECURITY_TARGET), 0);
    settle_declining(&mut r);
    let events = r.events_since(cp);
    let declared = events
        .iter()
        .position(|e| matches!(e, GameEvent::Attack { .. }))
        .expect("the <Engage> attack was declared");
    let check = events
        .iter()
        .position(|e| matches!(e, GameEvent::SecurityReveal { .. }))
        .expect("the <Engage> attack checked security");
    let gain = events
        .iter()
        .position(|e| {
            matches!(e, GameEvent::MemoryChange { source_card_id: Some(id), .. } if id == "EOT_MEM")
        })
        .expect("EOT_MEM resolved");
    assert!(
        declared < gain && gain < check,
        "declaration (#{declared}) < EOT_MEM (#{gain}) < security check (#{check})"
    );
}

// ─── §16-45 <Guard> ─────────────────────────────────────────────────────────

/// `guard_owner` has `GUARD` (slot 0) and a plain `FILL` (slot 1); P0 holds
/// `hand0`.
fn guard_board(guard_owner: PlayerId, hand0: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = board(hand0);
    r.place_on_field(guard_owner, "GUARD", Some(0));
    let ally = r.place_on_field(guard_owner, FILL, Some(0));
    red_anchor(&mut r);
    (r, ally)
}

/// §16-45-1 "<Guard> is a keyword effect. When any of a player's other
/// Digimon would leave the battle area by an opponent's effect, by deleting
/// the Digimon with this effect, it doesn't leave."
/// §16-45-2 "<Guard> is an immediate-type effect that triggers when any of a
/// player's other Digimon would leave the battle area by an opponent's
/// effect."
///
/// - By an opponent's effect: P0's `MAIN_DEL1` targets P1's `FILL`. P1 is
///   offered <Guard>; accepting deletes `GUARD` and `FILL` stays.
/// - By its owner's own effect: P0's own `MAIN_DEL_OWN_MEM` deletes P0's own
///   `FILL` next to P0's `GUARD`: no <Guard>, `FILL` is deleted.
/// - In battle: P0's `LV5_PLAIN` attacks P1's suspended `FILL`: no <Guard>,
///   `FILL` is deleted.
/// - Not another Digimon: P0's `MAIN_DEL1` targets P1's `GUARD` itself: no
///   <Guard>, `GUARD` is deleted.
#[test]
fn guard_immediate_optional_other_digimon_leave_by_opp_effect() {
    cite!(
        "16-45-1",
        "3f69efe465a58bf7f6ecdb4eb4f8a80d9466e4992b8745399c692b3a1d7c9355"
    );
    cite!(
        "16-45-2",
        "9798e2ec5294f24ca58a3367ba0624cd8070d53e095b93daad0e595832d37857"
    );
    // By an opponent's effect.
    let (mut r, _) = guard_board(1, &["MAIN_DEL1"]);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 1);
    assert!(advance_to(&mut r, optional_for(1)), "P1 is offered <Guard>");
    accept(&mut r, 1);
    settle_declining(&mut r);
    assert_eq!(field_top_ids(&r, 1), owned(&[FILL]), "FILL doesn't leave");
    assert_eq!(trash_ids(&r, 1), owned(&["GUARD"]), "GUARD was deleted");

    // By its owner's own effect.
    let (mut r, _) = guard_board(0, &["MAIN_DEL_OWN_MEM"]);
    use_option(&mut r, 0, "MAIN_DEL_OWN_MEM");
    pick_field(&mut r, 1);
    let prompts = settle_declining(&mut r);
    assert!(
        prompts.iter().all(|p| !p.optional),
        "own effect: no <Guard>: {prompts:?}"
    );
    assert_eq!(field_top_ids(&r, 0), owned(&["GUARD", "TAMER_PLAIN"]));

    // In battle.
    let (mut r, ally) = guard_board(1, &[]);
    set_suspended(&mut r, ally, true);
    let attacker = r.place_on_field(0, "LV5_PLAIN", Some(0));
    r.attack_digimon(attacker, ally, false);
    let prompts = settle_declining(&mut r);
    assert!(
        !prompts.iter().any(|p| optional_for(1)(p)),
        "battle: no <Guard>: {prompts:?}"
    );
    assert_eq!(field_top_ids(&r, 1), owned(&["GUARD"]));

    // The Digimon with <Guard> itself.
    let (mut r, _) = guard_board(1, &["MAIN_DEL1"]);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    let prompts = settle_declining(&mut r);
    assert!(
        !prompts.iter().any(|p| optional_for(1)(p)),
        "not another Digimon: no <Guard>: {prompts:?}"
    );
    assert_eq!(field_top_ids(&r, 1), owned(&[FILL]));
}

/// §16-45-2 "<Guard> is an IMMEDIATE-TYPE effect that triggers when any of a
/// player's other Digimon WOULD leave the battle area ..."
///
/// An immediate-type effect interrupts before its cause completes (§15-8-5):
/// when P1 is offered <Guard>, `FILL` is still in the battle area and nothing
/// is in P1's trash; after accepting, `FILL` was never trashed.
#[test]
fn guard_kind_matches_rule() {
    cite!(
        "16-45-2",
        "9798e2ec5294f24ca58a3367ba0624cd8070d53e095b93daad0e595832d37857"
    );
    let (mut r, _) = guard_board(1, &["MAIN_DEL1"]);
    let cp = r.event_checkpoint();
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 1);
    assert!(advance_to(&mut r, optional_for(1)));
    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["GUARD", FILL]),
        "offered before the deletion"
    );
    assert!(trash_ids(&r, 1).is_empty());
    accept(&mut r, 1);
    settle_declining(&mut r);
    assert!(!trashed_since(&r, cp, 1, FILL), "FILL never left");
    assert_eq!(field_top_ids(&r, 1), owned(&[FILL]));
}

/// §16-45-3 "The processing from <Guard> is optional."
///
/// P1 declines <Guard>: `FILL` is deleted, `GUARD` stays. (Matrix row `Guard`:
/// `optional_cost_then_mandatory`, consistent: the decision is optional, and
/// once `GUARD` is deleted the prevention needs no further decision -- see
/// `guard_immediate_optional_other_digimon_leave_by_opp_effect`.)
#[test]
fn guard_optionality_matches_rule() {
    cite!(
        "16-45-3",
        "3a1d47170e876b1acee5a28aa29fbe978ea2769c915f5f3f7c8dc8fed32aaac2"
    );
    let (mut r, _) = guard_board(1, &["MAIN_DEL1"]);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 1);
    assert!(advance_to(&mut r, optional_for(1)));
    r.decline_optional_trigger().expect("decline <Guard>");
    settle_declining(&mut r);
    assert_eq!(field_top_ids(&r, 1), owned(&["GUARD"]));
    assert_eq!(trash_ids(&r, 1), owned(&[FILL]));
}

/// §16-45-2 "<Guard> ... triggers when any of a player's other Digimon would
/// LEAVE THE BATTLE AREA by an opponent's effect."
///
/// Not only deletion: P0's `MAIN_BOUNCE1` ("Return 1 of your opponent's
/// Digimon to the hand") targets P1's `FILL`; P1 is offered <Guard>, and
/// accepting keeps `FILL` on the field (not in the hand).
#[test]
fn guard_timing_matches_rule() {
    cite!(
        "16-45-2",
        "9798e2ec5294f24ca58a3367ba0624cd8070d53e095b93daad0e595832d37857"
    );
    let (mut r, _) = guard_board(1, &["MAIN_BOUNCE1"]);
    use_on_opponent_slot(&mut r, "MAIN_BOUNCE1", 1);
    assert!(
        advance_to(&mut r, optional_for(1)),
        "a return to the hand is leaving the battle area"
    );
    accept(&mut r, 1);
    settle_declining(&mut r);
    assert_eq!(field_top_ids(&r, 1), owned(&[FILL]));
    assert!(hand_ids(&r, 1).is_empty());
    assert_eq!(trash_ids(&r, 1), owned(&["GUARD"]));
}

// ─── §16-46 <Detach> ────────────────────────────────────────────────────────

/// `owner` has `DETACH` (<Detach ([Seven Code] trait)>, 5000 DP, slot 0) with
/// the link cards `links`; P0 holds `hand0`.
fn detach_board(owner: PlayerId, links: &[&str], hand0: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = board(hand0);
    let host = r.place_on_field(owner, "DETACH", Some(0));
    for id in links {
        r.push_linked_owned(host, id, owner);
    }
    red_anchor(&mut r);
    (r, host)
}

/// §16-46-1 "<Detach> is a keyword effect that reads "When this Digimon would
/// leave the battle area other than by your effects, by trashing 1 of its
/// specified link cards, it doesn't leave." ..."
/// §16-46-3 "... If the content of that condition is executed, the processing
/// for "it doesn't leave" will be mandatory."
///
/// - P0's `MAIN_BOUNCE1` targets P1's `DETACH` (linked `SEVEN_LINK`, a
///   [Seven Code] card): P1 is offered <Detach>; accepting trashes
///   `SEVEN_LINK`, and with no further decision `DETACH` stays (not in the
///   hand).
/// - By its owner's own effect: P0's `MAIN_DEL_OWN_MEM` deletes P0's own
///   `DETACH`: no <Detach>; it is deleted with its link card.
#[test]
fn detach_optional_cost_then_mandatory_stay() {
    cite!(
        "16-46-1",
        "3745c18b54379994f437c8c266070ad06d8a3719050f28966c351423c82f707a"
    );
    cite!(
        "16-46-3",
        "a694a66de1e9d4c60d781da2782d75204e30be072709eb93ea0853182c9dec4a"
    );
    let (mut r, host) = detach_board(1, &["SEVEN_LINK"], &["MAIN_BOUNCE1"]);
    use_on_opponent_slot(&mut r, "MAIN_BOUNCE1", 0);
    assert!(
        advance_to(&mut r, optional_for(1)),
        "P1 is offered <Detach>"
    );
    accept(&mut r, 1);
    let after = settle_declining(&mut r);
    assert!(
        after.iter().all(|p| !p.optional),
        "'it doesn't leave' is mandatory once the link card is trashed: {after:?}"
    );
    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["DETACH"]),
        "DETACH doesn't leave"
    );
    assert!(hand_ids(&r, 1).is_empty());
    assert!(linked_ids(&r, host).is_empty());
    assert_eq!(trash_ids(&r, 1), owned(&["SEVEN_LINK"]));

    let (mut r, _) = detach_board(0, &["SEVEN_LINK"], &["MAIN_DEL_OWN_MEM"]);
    use_option(&mut r, 0, "MAIN_DEL_OWN_MEM");
    let prompts = settle_declining(&mut r);
    assert!(
        prompts.iter().all(|p| !p.optional),
        "own effect: no <Detach>: {prompts:?}"
    );
    assert_eq!(
        field_top_ids(&r, 0),
        owned(&["TAMER_PLAIN"]),
        "DETACH was deleted"
    );
    assert_eq!(
        trash_ids(&r, 0),
        owned(&["DETACH", "MAIN_DEL_OWN_MEM", "SEVEN_LINK"])
    );
}

/// §16-46-1 "... by trashing 1 of its SPECIFIED link cards, it doesn't
/// leave." ... "<Detach ([Seven Code] trait)>"
///
/// `DETACH`'s only link card is `LINK_A` (no [Seven Code] trait). P0's
/// `MAIN_BOUNCE1` returns it to the hand: <Detach> can't be paid (§15-7-4:
/// a player can't choose an optional processing condition that can't be
/// executed), so it is not offered and `DETACH` leaves.
#[test]
fn detach_needs_a_specified_link_card() {
    cite!(
        "16-46-1",
        "3745c18b54379994f437c8c266070ad06d8a3719050f28966c351423c82f707a"
    );
    let (mut r, _) = detach_board(1, &["LINK_A"], &["MAIN_BOUNCE1"]);
    use_on_opponent_slot(&mut r, "MAIN_BOUNCE1", 0);
    let prompts = settle_declining(&mut r);
    assert!(
        !prompts.iter().any(|p| optional_for(1)(p)),
        "no [Seven Code] link card: <Detach> is not offered: {prompts:?}"
    );
    assert_eq!(hand_ids(&r, 1), owned(&["DETACH"]));
}

/// §16-46-2 "<Detach> is a trigger-type effect that triggers when the Digimon
/// with this effect WOULD leave the battle area other than by its owner's
/// effects. (For details, refer to 15-8-5 "Immediate-Type Effects")"
///
/// The rule calls <Detach> trigger-type but defines it by "would" and points
/// at §15-8-5; its "it doesn't leave" outcome needs an interrupt before the
/// leave. Observable: when P1 is offered <Detach> (P0's `MAIN_DEL1` targets
/// P1's `DETACH`), `DETACH` is still in the battle area with its link card;
/// after accepting, `DETACH` was never trashed.
#[test]
fn detach_kind_matches_rule() {
    cite!(
        "16-46-2",
        "bd7ffa9f837404fd1b3e0668b4c31d8bd7df6128fe66e751fa41ab1629b76e67"
    );
    let (mut r, host) = detach_board(1, &["SEVEN_LINK"], &["MAIN_DEL1"]);
    let cp = r.event_checkpoint();
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    assert!(advance_to(&mut r, optional_for(1)));
    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["DETACH"]),
        "offered before it leaves"
    );
    assert_eq!(linked_ids(&r, host), owned(&["SEVEN_LINK"]));
    accept(&mut r, 1);
    settle_declining(&mut r);
    assert!(!trashed_since(&r, cp, 1, "DETACH"), "DETACH never left");
    assert_eq!(field_top_ids(&r, 1), owned(&["DETACH"]));
}

/// §16-46-3 ""By trashing 1 of its specified link cards" within <Detach> is
/// an optional processing condition. ..."
///
/// P1 declines <Detach> against P0's `MAIN_DEL1`: `SEVEN_LINK` is not
/// trashed by <Detach>, and `DETACH` is deleted (both cards in the trash).
#[test]
fn detach_optionality_matches_rule() {
    cite!(
        "16-46-3",
        "a694a66de1e9d4c60d781da2782d75204e30be072709eb93ea0853182c9dec4a"
    );
    let (mut r, _) = detach_board(1, &["SEVEN_LINK"], &["MAIN_DEL1"]);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    assert!(advance_to(&mut r, optional_for(1)));
    r.decline_optional_trigger().expect("decline <Detach>");
    settle_declining(&mut r);
    assert!(
        field_top_ids(&r, 1).is_empty(),
        "declined: DETACH is deleted"
    );
    assert_eq!(trash_ids(&r, 1), owned(&["DETACH", "SEVEN_LINK"]));
}

/// §16-46-2 "<Detach> ... triggers when the Digimon with this effect would
/// leave the battle area OTHER THAN BY ITS OWNER'S EFFECTS."
///
/// "Other than by its owner's effects" includes battle: P0's `LV5_PLAIN`
/// (7000 DP) attacks P1's suspended `DETACH` (5000 DP) and wins; P1 is
/// offered <Detach>, and accepting keeps `DETACH` on the field. (The
/// own-effect exclusion is in `detach_optional_cost_then_mandatory_stay`.)
#[test]
fn detach_timing_matches_rule() {
    cite!(
        "16-46-2",
        "bd7ffa9f837404fd1b3e0668b4c31d8bd7df6128fe66e751fa41ab1629b76e67"
    );
    let (mut r, host) = detach_board(1, &["SEVEN_LINK"], &[]);
    set_suspended(&mut r, host, true);
    let attacker = r.place_on_field(0, "LV5_PLAIN", Some(0));
    r.attack_digimon(attacker, host, false);
    assert!(
        advance_to(&mut r, optional_for(1)),
        "losing a battle: <Detach> is offered"
    );
    accept(&mut r, 1);
    settle_declining(&mut r);
    assert_eq!(field_top_ids(&r, 1), owned(&["DETACH"]));
    assert_eq!(trash_ids(&r, 1), owned(&["SEVEN_LINK"]));
}

// ─── §16-47 <Succession> ────────────────────────────────────────────────────

/// P0's stack (bottom first) `[LOWMON, BASEMON_B, BASEMON_A, top]` attacks
/// P1 (4 security cards). Returns the runner, the event checkpoint taken
/// before the attack, and the stack.
fn succession_attack(
    top: &str,
    before_attack: impl Fn(&mut DebugRunner, PermanentHandle),
) -> (DebugRunner, usize) {
    let mut r = ch16_new()
        .deck(0, &[FILL; 6])
        .deck(1, &[FILL; 6])
        .security(1, &[FILL; 4])
        .memory(0)
        .start();
    let h = r.place_stack(0, &["LOWMON", "BASEMON_B", "BASEMON_A", top]);
    before_attack(&mut r, h);
    let cp = r.event_checkpoint();
    r.attack_player(h, 1, false);
    settle_declining(&mut r);
    (r, cp)
}

/// §16-47-1 "<Succession> is a keyword effect that reads "This Digimon gains
/// all effects other than <Succession> on its topmost specified digivolution
/// card." It is shown on cards using text such as "<Succession
/// ([Ceresmon])>.""
///
/// P0's `SUCC_TOP` (<Succession ([Basemon])>) sits on `[LOWMON, BASEMON_B,
/// BASEMON_A]`. Its topmost [Basemon] is `BASEMON_A` (<Security A. +1>,
/// <Succession ([Lowmon])>, "[When Attacking] Gain 1 memory"). Attacking:
/// - it gains `BASEMON_A`'s <Security A. +1> (2 cards checked) and its
///   [When Attacking] (gain 1);
/// - not `BASEMON_B`'s "[When Attacking] Gain 2 memory" (not the topmost);
/// - not `BASEMON_A`'s <Succession ([Lowmon])>, so not `LOWMON`'s
///   "[When Attacking] Gain 3 memory".
/// Control: with a plain `LV5_PLAIN` on top, none of these apply.
#[test]
fn succession_gains_all_effects_of_topmost_specified_source() {
    cite!(
        "16-47-1",
        "0817e46eb33800704404f7049bda9fbd50dac3ad2d8ee33a3037738b8493104e"
    );
    let (r, cp) = succession_attack("LV5_PLAIN", |_, _| {});
    assert!(
        memory_sources(&r, cp).is_empty(),
        "control: no adopted effects"
    );
    assert_eq!(revealed(&r, cp, 1), owned(&[FILL]), "control: 1 check");

    let (r, cp) = succession_attack("SUCC_TOP", |_, _| {});
    assert_eq!(
        memory_sources(&r, cp),
        vec![(0, "BASEMON_A".to_string())],
        "only the topmost [Basemon]'s [When Attacking]; never <Succession ([Lowmon])>"
    );
    assert_eq!(
        revealed(&r, cp, 1),
        owned(&[FILL, FILL]),
        "the adopted <Security A. +1>"
    );
}

/// §16-47-2 "<Succession> is a persistent effect."
///
/// A persistent effect is applied continuously from the current board, with
/// no activation (§15-8-2). With `BASEMON_A` trashed from under `SUCC_TOP`
/// before the attack, the topmost [Basemon] is now `BASEMON_B`: the attack
/// gains 2 memory (its [When Attacking]) and checks 1 card (no more
/// <Security A. +1>). Neither adoption needs a decision.
#[test]
fn succession_kind_matches_rule() {
    cite!(
        "16-47-2",
        "b285c11f7ea87b85851adf5cea6132a14ed07ae8da2f7de1770d1dbccf7e08e6"
    );
    let (r, cp) = succession_attack("SUCC_TOP", |r, h| {
        let perm = &r.game.player(0).battle_area[h.index as usize];
        let card = perm.card_sources[2].handle();
        assert!(r.game.trash_source_ref(SourceSelectionRef {
            permanent: h,
            field_index: h.index,
            source_index: 2,
            card,
        }));
        assert_eq!(stack_ids(r, h), owned(&["LOWMON", "BASEMON_B", "SUCC_TOP"]));
    });
    assert_eq!(
        memory_sources(&r, cp),
        vec![(0, "BASEMON_B".to_string())],
        "the next topmost [Basemon] is adopted at once"
    );
    assert_eq!(revealed(&r, cp, 1), owned(&[FILL]));
}

/// §16-47-1 "... This Digimon GAINS all effects ..."
///
/// Gaining is not optional: no decision is offered during the attack, and
/// the adopted mandatory [When Attacking] resolves like the card's own.
#[test]
fn succession_optionality_matches_rule() {
    cite!(
        "16-47-1",
        "0817e46eb33800704404f7049bda9fbd50dac3ad2d8ee33a3037738b8493104e"
    );
    let mut r = ch16_new()
        .deck(0, &[FILL; 6])
        .deck(1, &[FILL; 6])
        .security(1, &[FILL; 4])
        .memory(0)
        .start();
    let h = r.place_stack(0, &["LOWMON", "BASEMON_B", "BASEMON_A", "SUCC_TOP"]);
    let cp = r.event_checkpoint();
    r.attack_player(h, 1, false);
    let prompts = settle_declining(&mut r);
    assert!(prompts.is_empty(), "no decision: {prompts:?}");
    assert_eq!(memory_sources(&r, cp), vec![(0, "BASEMON_A".to_string())]);
}

/// §16-47-1 "... THIS DIGIMON gains all effects ... on ITS topmost specified
/// digivolution card."
///
/// <Succession> is an effect of the Digimon whose top card prints it. With
/// `SUCC_TOP` itself a digivolution card (stack `[LOWMON, BASEMON_B,
/// BASEMON_A, SUCC_TOP, LV5_PLAIN]`), its <Succession> is not that Digimon's
/// effect and nothing is adopted.
#[test]
fn succession_timing_matches_rule() {
    cite!(
        "16-47-1",
        "0817e46eb33800704404f7049bda9fbd50dac3ad2d8ee33a3037738b8493104e"
    );
    let mut r = ch16_new()
        .deck(0, &[FILL; 6])
        .deck(1, &[FILL; 6])
        .security(1, &[FILL; 4])
        .memory(0)
        .start();
    let h = r.place_stack(
        0,
        &["LOWMON", "BASEMON_B", "BASEMON_A", "SUCC_TOP", "LV5_PLAIN"],
    );
    let cp = r.event_checkpoint();
    r.attack_player(h, 1, false);
    settle_declining(&mut r);
    assert!(memory_sources(&r, cp).is_empty(), "nothing adopted");
    assert_eq!(revealed(&r, cp, 1), owned(&[FILL]));
}

// ─── §4-19 <Overflow> ───────────────────────────────────────────────────────

/// §4-19-1 "<Overflow> is a rule on Digimon ACE cards. When a card with
/// <Overflow> moves from the field or from under a card to another area, the
/// memory marker is moved according to the value specified for <Overflow>."
///
/// P1's `ACE` (<Overflow <-3>>) leaves; each time P1 loses 3 memory (P0's
/// side goes from 5 to 8):
/// - deleted from the field by P0's `MAIN_DEL1`;
/// - returned to the hand from the field by P0's `MAIN_BOUNCE1`;
/// - trashed from under `LV5_PLAIN` when P0's `MAIN_DEL1` deletes the stack.
/// Control: deleting a plain `LV5_PLAIN` moves no memory.
#[test]
fn overflow_moves_memory_when_card_leaves_field_or_from_under() {
    cite!(
        "4-19-1",
        "0255eb92537f1502aa8d2248bd6b5245cdd49bdfba33d07362e8971018cf49eb"
    );
    let run = |option: &str, stack: &[&str]| {
        let mut r = board(&[option]);
        red_anchor(&mut r);
        r.place_stack(1, stack);
        let cp = r.event_checkpoint();
        use_on_opponent_slot(&mut r, option, 0);
        settle_declining(&mut r);
        (rule_memory_moves(&r, cp), r.memory())
    };
    assert_eq!(run("MAIN_DEL1", &["LV5_PLAIN"]), (vec![], 5), "control");
    assert_eq!(
        run("MAIN_DEL1", &["ACE"]),
        (vec![(1, 3)], 8),
        "deleted from the field"
    );
    assert_eq!(
        run("MAIN_BOUNCE1", &["ACE"]),
        (vec![(1, 3)], 8),
        "returned to the hand from the field"
    );
    assert_eq!(
        run("MAIN_DEL1", &["ACE", "LV5_PLAIN"]),
        (vec![(1, 3)], 8),
        "trashed from under a card"
    );
}

/// §4-19-2 "When <Overflow> causes the memory marker to move, the processing
/// for moving it is performed immediately, even if the processing for
/// another action is already being performed."
///
/// P0's `MAIN_DEL_OWN_MEM` ("Delete 1 of your Digimon. Then, gain 1 memory.")
/// deletes P0's own `ACE`: P0 loses 3 memory right away, before the effect's
/// "Then, gain 1 memory" (5 -> 2 -> 3).
#[test]
fn overflow_memory_moves_immediately_during_processing() {
    cite!(
        "4-19-2",
        "d6db8dc3136f327c25d595c20134e8de6fa54669bd39dab96172cecbb7a283ad"
    );
    let mut r = board(&["MAIN_DEL_OWN_MEM"]);
    r.place_on_field(0, "ACE", Some(0));
    let cp = r.event_checkpoint();
    use_option(&mut r, 0, "MAIN_DEL_OWN_MEM");
    settle_declining(&mut r);
    let moves: Vec<(Option<String>, i16)> = r
        .events_since(cp)
        .iter()
        .filter_map(|e| match e {
            GameEvent::MemoryChange {
                delta,
                source_card_id,
                ..
            } => Some((source_card_id.clone(), *delta)),
            _ => None,
        })
        .collect();
    assert_eq!(
        moves,
        vec![(None, -3), (Some("MAIN_DEL_OWN_MEM".to_string()), 1)],
        "<Overflow> moves the marker inside the effect's processing"
    );
    assert_eq!(r.memory(), 3);
}

/// §4-19-3 "<Overflow> isn't processed when a card is moved from another area
/// to the field."
///
/// P0 plays `ACE` from the hand (cost 5: memory 5 -> 0) and, separately,
/// digivolves `ACE` from the hand onto `LV4_PLAIN` (cost 3: 5 -> 2). The only
/// memory moves are those payments.
#[test]
fn overflow_not_processed_entering_field() {
    cite!(
        "4-19-3",
        "3385cabc0dabefada859e829a43f7e9d70d391fd1c2424560c2deb9f5bdb976d"
    );
    let mut r = board(&["ACE"]);
    let cp = r.event_checkpoint();
    r.play(0, 0);
    settle_declining(&mut r);
    assert_eq!(field_top_ids(&r, 0), owned(&["ACE"]));
    assert_eq!(r.memory(), 0, "paid the play cost only");
    assert!(
        r.events_since(cp).iter().all(|e| !matches!(
            e,
            GameEvent::MemoryChange { delta, .. } if *delta != -5
        )),
        "no <Overflow> move"
    );

    let mut r = board(&["ACE"]);
    let base = r.place_on_field(0, "LV4_PLAIN", Some(0));
    r.game
        .decode_action(encode_digivolve(0, base.index as u16), 0);
    settle_declining(&mut r);
    assert_eq!(field_top_ids(&r, 0), owned(&["ACE"]), "digivolved");
    assert_eq!(r.memory(), 2, "paid the digivolution cost only");
}

/// §4-19-4 "<Overflow> isn't processed when a card is moved from another area
/// to under a card."
///
/// P0's `MAIN_PLACE_UNDER` places `ACE` from the hand as the bottom
/// digivolution card of P0's `LV5_PLAIN`: no memory moves.
#[test]
fn overflow_not_processed_entering_under_a_card() {
    cite!(
        "4-19-4",
        "7c2d6aa83e6f77893134a746dc209918ee52fd5af4e7c15f960d71513b0491a0"
    );
    let mut r = board(&["MAIN_PLACE_UNDER", "ACE"]);
    let host = r.place_on_field(0, "LV5_PLAIN", Some(0));
    let cp = r.event_checkpoint();
    use_option(&mut r, 0, "MAIN_PLACE_UNDER");
    pick_hand_card(&mut r, 0, "ACE");
    settle_declining(&mut r);
    assert_eq!(
        stack_ids(&r, host),
        owned(&["ACE", "LV5_PLAIN"]),
        "placed under"
    );
    assert!(rule_memory_moves(&r, cp).is_empty(), "no <Overflow> move");
    assert_eq!(r.memory(), 5);
}

/// §4-19-5 "When multiple instances of <Overflow> are processed
/// simultaneously, they are processed using the following procedure."
/// §4-19-5-1 "The turn player chooses 1 instance of <Overflow> on their cards
/// and processes it. The turn player repeats this procedure until there are
/// no more unresolved instances of <Overflow> on their cards."
/// §4-19-5-2 "Once all instances of <Overflow> on the turn player's cards
/// have been processed, then the non-turn player chooses 1 instance of
/// <Overflow> on their cards and processes it. ..."
///
/// P0 (turn player, memory 9) uses `MAIN_DEL_ALL_ANY` ("Delete all Digimon"):
/// two P0 `ACE`s and one P1 `ACE` leave at once. P0's two are processed
/// first (9 -> 6 -> 3), then P1's (3 -> 6). The order is observable through
/// the memory cap: P1's first would give 9 -> 10 (capped) -> 7 -> 4.
///
/// The likelier real path is a battle where both ACEs are deleted: P0's `ACE`
/// (7000 DP) attacks P1's suspended `ACE` (7000 DP) at memory 9; both are
/// deleted together, P0's instance first (9 -> 6), then P1's (6 -> 9); P1's
/// first would give 9 -> 10 (capped) -> 7.
///
/// The turn player's choice among their own instances is not observable:
/// they all move the marker the same way, so any order gives the same result
/// (the engine offers no prompt for it; nothing is lost).
#[test]
fn overflow_turn_player_orders_then_non_turn_player() {
    cite!(
        "4-19-5",
        "db673babf873b635f4000e8a40250beca96b214ad6f7a98c2ae89cc342169865"
    );
    cite!(
        "4-19-5-1",
        "2aa92cc4fda8bccbd05ada27cdaf868a00a675960ba19d78d3fc8cbaaf9dee81"
    );
    cite!(
        "4-19-5-2",
        "a60b917e4326f91400b6ec0304d6d0fca2db87be4de9c94faf18251a4c08d765"
    );
    // Mutual battle deletion.
    let mut r = board(&[]);
    r.game.set_memory(9);
    let attacker = r.place_on_field(0, "ACE", Some(0));
    let defender = r.place_on_field(1, "ACE", Some(0));
    set_suspended(&mut r, defender, true);
    let cp = r.event_checkpoint();
    r.attack_digimon(attacker, defender, false);
    settle_declining(&mut r);
    assert!(field_top_ids(&r, 0).is_empty() && field_top_ids(&r, 1).is_empty());
    assert_eq!(
        (rule_memory_moves(&r, cp), r.memory()),
        (vec![(0, -3), (1, 3)], 9),
        "battle: the turn player's instance first"
    );

    // One effect deleting all Digimon.
    let mut r = board(&["MAIN_DEL_ALL_ANY"]);
    r.game.set_memory(9);
    r.place_on_field(0, "ACE", Some(0));
    r.place_on_field(0, "ACE", Some(0));
    r.place_on_field(1, "ACE", Some(0));
    let cp = r.event_checkpoint();
    use_option(&mut r, 0, "MAIN_DEL_ALL_ANY");
    settle_declining(&mut r);
    assert!(field_top_ids(&r, 0).is_empty() && field_top_ids(&r, 1).is_empty());
    assert_eq!(
        rule_memory_moves(&r, cp),
        vec![(0, -3), (0, -3), (1, 3)],
        "the turn player's instances first, then the non-turn player's"
    );
    assert_eq!(r.memory(), 6);
}

// ─── §4-20 Arts Digivolve ───────────────────────────────────────────────────

/// P0 (memory 5) has `PURPLE_LV3` (slot 0, meets the Option face's purple)
/// and `LV5_PLAIN` (slot 1, a legal red Lv.5 base) and holds `DUAL_ARTS`.
fn dual_board(hand0: &[&str]) -> DebugRunner {
    let mut r = board(hand0);
    r.place_on_field(0, "PURPLE_LV3", Some(0));
    r.place_on_field(0, "LV5_PLAIN", Some(0));
    r
}

/// §4-20-1 "Arts Digivolve is a rule on DUAL cards. Instead of the trashing
/// from the pending processing after using an Option card, one of your cards
/// on the field may digivolve into that DUAL card without paying the cost."
/// §4-20-2 "Arts Digivolve is overwrite processing that replaces the trashing
/// of an Option card from the pending processing."
///
/// P0 uses `DUAL_ARTS` as an Option (use cost 2: memory 5 -> 3); its [Main]
/// finds no target. Instead of the trash, P0 may digivolve a card on the
/// field into it:
/// - only the cost is waived, not the digivolution requirements: the red
///   Lv.5 `LV5_PLAIN` (slot 1) is a legal target, the purple Lv.3
///   `PURPLE_LV3` (slot 0) is not;
/// - accepting on `LV5_PLAIN`: `DUAL_ARTS` becomes the top of that stack,
///   memory stays 3 (no digivolution cost), and it never reaches the trash;
/// - declining: the used Option is trashed as usual.
#[test]
fn arts_digivolve_replaces_option_trash_with_free_digivolve() {
    cite!(
        "4-20-1",
        "a37486cebf34be622bfc1425418567531559905774dec2fa4b31ab3b56dd3a25"
    );
    cite!(
        "4-20-2",
        "ce3d9c823f344247e192955517445b21385f5cf16c2cd0f0aa92c1585a185a02"
    );
    let mut r = dual_board(&["DUAL_ARTS"]);
    use_option(&mut r, 0, "DUAL_ARTS");
    assert_eq!(r.memory(), 3, "paid the Option use cost");
    assert!(
        advance_to(&mut r, |p| p.player == 0
            && p.kind == SelectionKind::OwnField
            && p.optional),
        "P0 may Arts Digivolve instead of trashing"
    );
    assert!(trash_ids(&r, 0).is_empty(), "not trashed yet");
    let cp = r.event_checkpoint();
    let offered = r.pending_selection().unwrap().valid_action_ids.clone();
    assert!(
        offered.contains(&field_pick_action(1)),
        "LV5_PLAIN meets the digivolution requirements: {offered:?}"
    );
    assert!(
        !offered.contains(&field_pick_action(0)),
        "PURPLE_LV3 does not (only the cost is waived): {offered:?}"
    );
    pick_field(&mut r, 1);
    settle_declining(&mut r);
    assert_eq!(
        stack_ids(&r, r.perm_handle(0, 1)),
        owned(&["LV5_PLAIN", "DUAL_ARTS"]),
        "digivolved into the DUAL card"
    );
    assert_eq!(r.memory(), 3, "without paying the digivolution cost");
    assert!(trash_ids(&r, 0).is_empty(), "the trashing was replaced");
    // Complements the trash read above: the event stream shows no trash of the
    // card either (the plain Option-trash path emits none at all,
    // G-ENGINE-OPTION-TRASH-NO-EVENT, so this alone would not be a witness).
    assert!(
        !trashed_since(&r, cp, 0, "DUAL_ARTS"),
        "DUAL_ARTS never reached the trash"
    );

    let mut r = dual_board(&["DUAL_ARTS"]);
    use_option(&mut r, 0, "DUAL_ARTS");
    assert!(advance_to(&mut r, |p| p.player == 0 && p.optional));
    r.decline_optional_trigger()
        .expect("decline Arts Digivolve");
    settle_declining(&mut r);
    assert_eq!(trash_ids(&r, 0), owned(&["DUAL_ARTS"]), "declined: trashed");
    assert_eq!(field_top_ids(&r, 0), owned(&["PURPLE_LV3", "LV5_PLAIN"]));
}

// ─── §4-6 DUAL cards ────────────────────────────────────────────────────────

/// §4-6-1 "A DUAL card is a card that can be treated as being in either the
/// Digimon card category or the Option card category."
/// §4-6-2 "When using a DUAL card, the player must declare whether they are
/// digivolving with the Digimon information or using the Option information
/// on that card."
/// §4-6-4 "A single DUAL card has both Digimon information and Option
/// information."
/// §4-6-5-2 "A DUAL card's Digimon information can be referenced when
/// referencing the card as a Digimon card or Digimon."
/// §4-6-6-2 "A DUAL card's Option information can be referenced when
/// referencing the card as an Option card."
///
/// P0 holds `DUAL_ARTS` (Digimon information: red Lv.6, 12000 DP, digivolve
/// from red Lv.5 for 3; Option information: purple, use cost 2).
/// - Both declarations are offered as separate actions: use (the Option) and
///   digivolve onto `LV5_PLAIN` (the Digimon).
/// - Digivolving reads the Digimon information: cost 3 (5 -> 2), and the
///   result is a Lv.6, 12000 DP Digimon.
/// - Using reads the Option information: cost 2 (5 -> 3). Its purple color
///   requirement is met only by `PURPLE_LV3`; the red `LV5_PLAIN` (which
///   matches the Digimon information's color) does not meet it.
#[test]
fn dual_use_declares_digimon_or_option_information() {
    cite!(
        "4-6-1",
        "71e521e2421527fc20a64b564c7792b256d164651256c309effe1e311448eee7"
    );
    cite!(
        "4-6-2",
        "cfdad708809ed7420d5a8ae92a2cedb63403b511fe14e8ca4df5750630cc9fff"
    );
    cite!(
        "4-6-4",
        "55ebeedd0a4754fc0a2de83cb3ddc60a3e97784d9c441372ff9935efb8013e16"
    );
    cite!(
        "4-6-5-2",
        "79757dcc3de9dc978264cf27d196ee4386376ad1478c699408b7b64f73194b31"
    );
    cite!(
        "4-6-6-2",
        "048c65a256d7f9e5a4d1bfb47bdb99c098feb6678a7d999b6b2062c4e2e40b52"
    );
    let r = dual_board(&["DUAL_ARTS"]);
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 1.0,
        "use with the Option information"
    );
    assert_eq!(
        mask[encode_digivolve(0, 1) as usize],
        1.0,
        "digivolve with the Digimon information"
    );

    let mut r = dual_board(&["DUAL_ARTS"]);
    r.game.decode_action(encode_digivolve(0, 1), 0);
    settle_declining(&mut r);
    let h = r.perm_handle(0, 1);
    assert_eq!(stack_ids(&r, h), owned(&["LV5_PLAIN", "DUAL_ARTS"]));
    assert_eq!(r.memory(), 2, "the Digimon information's digivolution cost");
    assert_eq!(r.effective_dp(h), Some(12000));

    let mut r = dual_board(&["DUAL_ARTS"]);
    assert_ne!(
        r.game.play_option_from_hand(0, 0),
        OptionPlayResult::Invalid
    );
    assert_eq!(r.memory(), 3, "the Option information's use cost");

    let mut r = board(&["DUAL_ARTS"]);
    r.place_on_field(0, "LV5_PLAIN", Some(0));
    assert!(
        !can_use_from_hand(&r, 0),
        "a red card does not meet the Option information's purple requirement"
    );
}

/// §4-6-5-1 "This refers to a DUAL card's traits and the information above
/// them." (Digimon information)
/// §4-6-6-1 "This refers a DUAL card's traits and the information below
/// them." (Option information)
///
/// The traits belong to both. In the hand `DUAL_ARTS` ([Artsy]) is:
/// - a legal pick for `HAND_PICK_DIGI` ("Trash 1 level 6 Digimon card with
///   the [Artsy] trait from your hand"), read through its Digimon
///   information (level 6, §4-6-5-2);
/// - a legal pick for `HAND_PICK_OPT` ("Trash 1 Option card with the
///   [Artsy] trait from your hand").
#[test]
fn dual_traits_belong_to_both_informations() {
    cite!(
        "4-6-5-1",
        "0245d75bdf7a39db36cb72e1051ba741497b46b09ce89da135677d55dfa5a78e"
    );
    cite!(
        "4-6-6-1",
        "1b4db132ca34153df020b5209caa1b7e325c273f4ce478655d5d9d05211e36d2"
    );
    for picker in ["HAND_PICK_DIGI", "HAND_PICK_OPT"] {
        let mut r = board(&[picker, "DUAL_ARTS"]);
        red_anchor(&mut r);
        use_option(&mut r, 0, picker);
        assert!(
            r.pending_selection().is_some(),
            "{picker}: DUAL_ARTS is a legal pick (no pick offered)"
        );
        pick_hand_card(&mut r, 0, "DUAL_ARTS");
        resolve_prompts(&mut r);
        assert!(hand_ids(&r, 0).is_empty(), "{picker}");
        assert!(
            trash_ids(&r, 0).contains(&"DUAL_ARTS".to_string()),
            "{picker}"
        );
    }
}

/// Use the Option `option` from P0's hand (on the `dual_board`) against
/// P1's lone `target`, declining every optional decision. Whether `target`
/// is still in the battle area afterwards.
fn survives_option(option: &str, target: &str) -> bool {
    let mut r = dual_board(&[option]);
    r.place_on_field(1, target, Some(0));
    use_option(&mut r, 0, option);
    settle_declining(&mut r);
    field_top_ids(&r, 1) == owned(&[target])
}

/// Digivolve `card` from P0's hand onto `LV5_PLAIN` (on the `dual_board`),
/// its [When Digivolving] aimed at P1's lone `target`. Whether `target` is
/// still in the battle area afterwards.
fn survives_digivolve(card: &str, target: &str) -> bool {
    let mut r = dual_board(&[card]);
    r.place_on_field(1, target, Some(0));
    r.game.decode_action(encode_digivolve(0, 1), 0);
    settle_declining(&mut r);
    assert_eq!(
        r.game.player(0).battle_area[1]
            .top_card()
            .card_id(&r.game.card_data),
        card,
        "{card} digivolved"
    );
    field_top_ids(&r, 1) == owned(&[target])
}

/// §4-6-5-3 "Effects in the Digimon information are considered Digimon
/// effects."
/// §4-6-6-3 "Effects in the Option information are treated as Option card
/// effects."
///
/// P1 has one Digimon: `IMM_OPT` (not affected by P0's Option card effects)
/// or `IMM_DIGI` (not affected by P0's Digimon effects).
/// - Controls: the plain Option `MAIN_DEL1` ("[Main] Delete 1 of your
///   opponent's Digimon") deletes `IMM_DIGI` but not `IMM_OPT`; the plain
///   Digimon `WD_DEL_RET` ("[When Digivolving] Delete 1 of your opponent's
///   Digimon with 8000 DP or less ...") deletes `IMM_OPT` but not `IMM_DIGI`.
/// - `DUAL_ARTS` used as an Option: its [Main] "Delete 1 of your opponent's
///   Digimon" is an Option card effect -- like `MAIN_DEL1`.
/// - `DUAL_ARTS` digivolved into: its [When Digivolving] "Delete 1 of your
///   opponent's Digimon" is a Digimon effect -- like `WD_DEL_RET`.
#[test]
fn dual_effects_are_digimon_or_option_effects_by_information() {
    cite!(
        "4-6-5-3",
        "803a0bb1b5e8ba4d51f86dd20be1724fc693396d740998685d78a5cf00dee01d"
    );
    cite!(
        "4-6-6-3",
        "def98ba9ea5441aa0946752853918c9cd5d21f5ff38464751db1097a745f0956"
    );
    assert!(survives_option("MAIN_DEL1", "IMM_OPT"), "control");
    assert!(!survives_option("MAIN_DEL1", "IMM_DIGI"), "control");
    assert!(survives_digivolve("WD_DEL_RET", "IMM_DIGI"), "control");
    assert!(!survives_digivolve("WD_DEL_RET", "IMM_OPT"), "control");

    assert!(
        !survives_option("DUAL_ARTS", "IMM_DIGI"),
        "the Option information's [Main] resolves"
    );
    assert!(
        survives_option("DUAL_ARTS", "IMM_OPT"),
        "the Option information's [Main] is an Option card effect"
    );
    assert!(!survives_digivolve("DUAL_ARTS", "IMM_OPT"));
    assert!(
        survives_digivolve("DUAL_ARTS", "IMM_DIGI"),
        "the Digimon information's [When Digivolving] is a Digimon effect"
    );
}

/// §4-6-6-2 "A DUAL card's Option information can be referenced when
/// referencing the card as an Option card."
///
/// In the hand, `DUAL_ARTS` referenced as an Option card has its Option
/// information's color, purple -- and only that: its Digimon information is
/// red. Referenced as a Digimon card it has only the Digimon information's
/// red. (Option USE already reads the Option information's color and cost:
/// `dual_use_declares_digimon_or_option_information`.)
/// - `HAND_PICK_PURPLE_OPT` ("Trash 1 purple Option card from your hand")
///   can choose it;
/// - `HAND_PICK_RED_OPT` ("Trash 1 red Option card from your hand") can't;
/// - `HAND_PICK_PURPLE_DIGI` ("Trash 1 purple Digimon card from your hand")
///   can't.
/// The two negatives rule out a "Digimon colors and Option colors together"
/// reading, which would offer illegal picks.
#[test]
fn dual_option_information_referenced_as_option_card() {
    cite!(
        "4-6-6-2",
        "048c65a256d7f9e5a4d1bfb47bdb99c098feb6678a7d999b6b2062c4e2e40b52"
    );
    let mut r = board(&["HAND_PICK_PURPLE_OPT", "DUAL_ARTS"]);
    red_anchor(&mut r);
    use_option(&mut r, 0, "HAND_PICK_PURPLE_OPT");
    assert!(
        r.pending_selection().is_some(),
        "DUAL_ARTS is a purple Option card: a legal pick (no pick offered)"
    );
    pick_hand_card(&mut r, 0, "DUAL_ARTS");
    resolve_prompts(&mut r);
    assert!(hand_ids(&r, 0).is_empty());
    assert!(trash_ids(&r, 0).contains(&"DUAL_ARTS".to_string()));

    for (picker, why) in [
        (
            "HAND_PICK_RED_OPT",
            "referenced as an Option card, DUAL_ARTS is purple, not red",
        ),
        (
            "HAND_PICK_PURPLE_DIGI",
            "referenced as a Digimon card, DUAL_ARTS is red, not purple",
        ),
    ] {
        let mut r = board(&[picker, "DUAL_ARTS"]);
        red_anchor(&mut r);
        use_option(&mut r, 0, picker);
        assert!(
            r.pending_selection().is_none(),
            "{picker}: {why}: {:?}",
            r.pending_selection()
        );
        resolve_prompts(&mut r);
        assert_eq!(hand_ids(&r, 0), owned(&["DUAL_ARTS"]), "{picker}: {why}");
    }
}

/// §4-6-1 "A DUAL card is a card that can be treated as being in either the
/// Digimon card category or the Option card category."
///
/// An UNQUALIFIED reference ("Trash 1 purple card from your hand") names
/// neither category; by owner ruling W5 (b) (2026-10-17, "This would be the
/// Digimon colour/s"; `qa/rules/mechanics-gaps-2026-10.md`) it reads the DUAL
/// card's Digimon information only, as a `kind: digimon` reference does
/// (§4-6-5-2). The Option information is read only when the reference names
/// "Option card" (§4-6-6-2). `DUAL_ARTS`: Digimon information red, Option
/// information purple.
/// - "red card" (the Digimon information's colour) can choose it;
/// - "purple card" (only the Option information's colour) can't;
/// - "blue card" (neither) can't.
#[test]
fn dual_unqualified_colour_reference_reads_digimon_information() {
    cite!(
        "4-6-1",
        "71e521e2421527fc20a64b564c7792b256d164651256c309effe1e311448eee7"
    );
    let mut r = board(&["HAND_PICK_RED_ANY", "DUAL_ARTS"]);
    red_anchor(&mut r);
    use_option(&mut r, 0, "HAND_PICK_RED_ANY");
    assert!(
        r.pending_selection().is_some(),
        "the Digimon information is red: a legal pick (no pick offered)"
    );
    pick_hand_card(&mut r, 0, "DUAL_ARTS");
    resolve_prompts(&mut r);
    assert!(hand_ids(&r, 0).is_empty());
    assert!(trash_ids(&r, 0).contains(&"DUAL_ARTS".to_string()));

    for (picker, why) in [
        (
            "HAND_PICK_PURPLE_ANY",
            "purple is only the Option information; an unqualified reference reads the Digimon information (red)",
        ),
        ("HAND_PICK_BLUE_ANY", "neither information is blue"),
    ] {
        let mut r = board(&[picker, "DUAL_ARTS"]);
        red_anchor(&mut r);
        use_option(&mut r, 0, picker);
        assert!(
            r.pending_selection().is_none(),
            "{picker}: {why}: {:?}",
            r.pending_selection()
        );
        resolve_prompts(&mut r);
        assert_eq!(hand_ids(&r, 0), owned(&["DUAL_ARTS"]), "{picker}: {why}");
    }
}

/// §4-6-1 "A DUAL card is a card that can be treated as being in either the
/// Digimon card category or the Option card category."
///
/// An unqualified reference reads the WHOLE reference through the DUAL card's
/// Digimon information (owner ruling W5 (b), 2026-10-17; §4-6-5-2), never the
/// Option information and never leaf by leaf across both. `DUAL_ARTS`:
/// Digimon information red Lv.6, Option information purple (no level).
/// - `HAND_PICK_PURPLE_LV6_ANY` ("Trash 1 purple Lv.6 card from your hand")
///   can't choose it: the Digimon information is Lv.6 but red (purple is only
///   the Option information);
/// - `HAND_PICK_NOT_RED_ANY` ("Trash 1 card that is not red from your hand")
///   can't either: the Digimon information is red (the Option information's
///   purple is not read).
#[test]
fn dual_unqualified_reference_holds_through_one_information() {
    cite!(
        "4-6-1",
        "71e521e2421527fc20a64b564c7792b256d164651256c309effe1e311448eee7"
    );
    for (picker, why) in [
        (
            "HAND_PICK_PURPLE_LV6_ANY",
            "the Digimon information is Lv.6 but red, not purple",
        ),
        (
            "HAND_PICK_NOT_RED_ANY",
            "the Digimon information is red; the Option information is not read",
        ),
    ] {
        let mut r = board(&[picker, "DUAL_ARTS"]);
        red_anchor(&mut r);
        use_option(&mut r, 0, picker);
        assert!(
            r.pending_selection().is_none(),
            "{picker}: {why}: {:?}",
            r.pending_selection()
        );
        resolve_prompts(&mut r);
        assert_eq!(hand_ids(&r, 0), owned(&["DUAL_ARTS"]), "{picker}: {why}");
    }
}

/// §4-6-3 "A DUAL card placed on the field is treated as a Digimon. It can't
/// be referenced as an Option card placed on the field."
///
/// A DUAL card under a Digimon (a digivolution card) is on the field, so a
/// color reference over digivolution cards reads only its Digimon information.
/// P1's stack is `[LV5_PLAIN, DUAL_ARTS]` with `DUAL_ARTS` underneath
/// (Digimon information red, Option information purple):
/// - `SRC_PICK_RED_ANY` ("Trash 1 red card from the digivolution cards of 1 of
///   your opponent's Digimon") can choose it and trashes it;
/// - `SRC_PICK_PURPLE_ANY` can't: purple is only its Option information.
#[test]
fn dual_digivolution_card_on_field_reads_digimon_information() {
    cite!(
        "4-6-3",
        "a73913aee829c3e62eba607984ce4c0cf52b6708908f0c90353cff4b678be173"
    );
    let staged = |picker: &str| {
        let mut r = board(&[picker]);
        red_anchor(&mut r);
        r.place_stack(1, &["DUAL_ARTS", "LV5_PLAIN"]);
        use_option(&mut r, 0, picker);
        r
    };

    let mut r = staged("SRC_PICK_RED_ANY");
    step_prompt(&mut r); // the opponent's only Digimon
    assert!(
        r.pending_selection().is_some(),
        "the DUAL digivolution card is red: a legal pick (no pick offered)"
    );
    step_prompt(&mut r);
    resolve_prompts(&mut r);
    assert!(trash_ids(&r, 1).contains(&"DUAL_ARTS".to_string()));

    let mut r = staged("SRC_PICK_PURPLE_ANY");
    step_prompt(&mut r); // the opponent's only Digimon
    assert!(
        r.pending_selection().is_none(),
        "purple is only the Option information, not readable on the field: {:?}",
        r.pending_selection()
    );
    resolve_prompts(&mut r);
    assert!(trash_ids(&r, 1).is_empty());
}

/// §4-6-3 "A DUAL card placed on the field is treated as a Digimon. It can't
/// be referenced as an Option card placed on the field."
///
/// Whatever reads a card through the general predicate -- here a Digimon
/// counting its OWN digivolution cards (`self_source_count`) -- sees a DUAL
/// digivolution card as a Digimon card with only its Digimon information.
/// `DUAL_ARTS` (Digimon information red, Option information purple) under the
/// attacker: "[When Attacking] If this Digimon has ... in its digivolution
/// cards, gain 3 memory."
/// - `DUALSRC_RED` ("a red card") and `DUALSRC_DIGI` ("a Digimon card") count it;
/// - `DUALSRC_PURPLE` ("a purple card") counts 0: purple is only the Option
///   information;
/// - `DUALSRC_OPT` ("an Option card") counts 0: it can't be referenced as one.
#[test]
fn dual_digivolution_card_counts_as_digimon_only() {
    cite!(
        "4-6-3",
        "a73913aee829c3e62eba607984ce4c0cf52b6708908f0c90353cff4b678be173"
    );
    let gains_memory = |top: &str| {
        let mut r = ch16_new()
            .deck(0, &[FILL; 6])
            .deck(1, &[FILL; 6])
            .security(1, &[FILL; 4])
            .memory(0)
            .start();
        let h = r.place_stack(0, &["DUAL_ARTS", top]);
        let cp = r.event_checkpoint();
        r.attack_player(h, 1, false);
        settle_declining(&mut r);
        !memory_sources(&r, cp).is_empty()
    };
    assert!(gains_memory("DUALSRC_RED"), "red: its Digimon information");
    assert!(gains_memory("DUALSRC_DIGI"), "a Digimon card");
    assert!(
        !gains_memory("DUALSRC_PURPLE"),
        "purple is only the Option information, which isn't readable on the field"
    );
    assert!(
        !gains_memory("DUALSRC_OPT"),
        "a DUAL on the field can't be referenced as an Option card"
    );
}

/// §4-6-3 "A DUAL card placed on the field is treated as a Digimon. It can't
/// be referenced as an Option card placed on the field. (Example: A DUAL
/// card that is digivolved by placing it on top of one of your cards on the
/// field will be treated as a Digimon, and it can't be trashed by a "trash an
/// Option card in the battle area" effect.)"
///
/// The rule's own example. P1 has the stack `[LV5_PLAIN, DUAL_ARTS]`.
/// - P0's `MAIN_TRASH_OPP_OPT` ("Trash 1 of your opponent's Option cards in
///   the battle area"): with P1's Option card `MAIN_PLACE_SELF` also in the
///   battle area, that card is trashed and the DUAL stack stays; with the
///   DUAL stack alone, nothing is trashed.
/// - P0's `MAIN_DEL1` ("Delete 1 of your opponent's Digimon") deletes the
///   DUAL stack.
#[test]
fn dual_on_field_is_a_digimon_not_an_option() {
    cite!(
        "4-6-3",
        "a73913aee829c3e62eba607984ce4c0cf52b6708908f0c90353cff4b678be173"
    );
    let staged = |option: &str, with_option_card: bool| {
        let mut r = board(&[option]);
        red_anchor(&mut r);
        r.place_stack(1, &["LV5_PLAIN", "DUAL_ARTS"]);
        if with_option_card {
            r.place_on_field(1, "MAIN_PLACE_SELF", Some(0));
        }
        use_option(&mut r, 0, option);
        settle_declining(&mut r);
        r
    };

    let r = staged("MAIN_TRASH_OPP_OPT", true);
    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["DUAL_ARTS"]),
        "the DUAL stack stays"
    );
    assert_eq!(
        trash_ids(&r, 1),
        owned(&["MAIN_PLACE_SELF"]),
        "the Option card is trashed"
    );

    let r = staged("MAIN_TRASH_OPP_OPT", false);
    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["DUAL_ARTS"]),
        "the DUAL card is not an Option card in the battle area"
    );
    assert!(trash_ids(&r, 1).is_empty());

    let r = staged("MAIN_DEL1", false);
    assert!(
        field_top_ids(&r, 1).is_empty(),
        "the DUAL card is a Digimon"
    );
    assert_eq!(trash_ids(&r, 1), owned(&["DUAL_ARTS", "LV5_PLAIN"]));
}
