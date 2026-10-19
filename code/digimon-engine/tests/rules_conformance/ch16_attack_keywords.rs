//! Chapter 16 (Comprehensive Rules Ver. 4.3), the attack keywords:
//! `<Security A.>` (§16-4), `<Blocker>` (§16-5), `<Piercing>` (§16-7, the ids
//! ch18 / ch11-14 did not cover), `<Jamming>` (§16-9-2), `<Retaliation>`
//! (§16-13), `<Rush>` (§16-15), `<Blitz>` (§16-16), `<Raid>` (§16-23),
//! `<Alliance>` (§16-24), `<Collision>` (§16-30), `<Vortex>` (§16-33),
//! `<Overclock>` (§16-34), `<Iceclad>` (§16-35), `<Execute>` (§16-38) and
//! `<Progress>` (§16-39).
//!
//! Every test asserts the RULE, never the engine's current output. A test the
//! engine fails is `#[ignore]`d with the gap id that explains it (discover then
//! pin); see `docs/RUST_ENGINE_GAPS.md` and `qa/rules/cr_obligations_status.yaml`.
//!
//! Per keyword: `<kw>_kind_matches_rule` checks the observable consequence of
//! the §16-N-2 effect type (a trigger-type effect pends with the other effects
//! triggered at the same time and activates -- all of its processing -- in the
//! order the player picks; a persistent effect applies with no activation and
//! no decision, as soon as it is gained), `<kw>_optionality_matches_rule` the
//! optional / mandatory split, `<kw>_timing_matches_rule` the "triggers when
//! ..." / "while ..." condition, plus one test per behavioural sub-rule. A
//! persistent keyword has no decision to make optional or mandatory: its
//! kind test also asserts that nothing is offered.
//!
//! Engine reality: every keyword here is a `Keyword` variant granted with
//! `kind: grant_keyword` (or a `grant_keyword` step for "gains <X> for the
//! turn"); `<Blitz>` is authored with the production idiom (EX3-063: a
//! `[When Digivolving]` step that grants `Keyword::Blitz` for the turn) and
//! `<Overclock ([Puppet] trait)>` with `overclock_cost_filter` (BT22-036).
//! No Token fixture exists in the DSL, so the "1 of your Tokens" half of
//! `<Overclock>` is not staged.
//!
//! Fixture cards: `fixtures/common/*.yaml` + `fixtures/ch16_atk/*.yaml` (plus
//! `BLOCKER` / `BLOCKER_CANNOT_SUSPEND` / `SA_PLUS1` / `EOA_OBS` from
//! `ch11_14`, `WA_ALLIANCE` from `ch15_04`, `MAIN_TRASH_SEC_DRAW` from
//! `ch15_07`, `LV4_PLAIN` / `EOT_MEM` / `MAIN_DEL1` from `ch15_08` and
//! `BARRIER` from `ch16_repl`), one synthetic card per file, authored to
//! printed-style text (each clause's `summary:` is that text).
//!
//! Security stacks are listed bottom first: the LAST card of `.security(..)`
//! is the top card. Stacks (`place_stack`) are listed bottom first too: the
//! LAST card is the top card.

use crate::fixtures;
use crate::support::*;
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    encode_attack, encode_digivolve, ATTACK_START, EFFECTS_PER_PERMANENT,
    FIELD_EFFECT_SLOT_FOR_OVERCLOCK, FIELD_EFFECT_START, PASS, PLAY_HAND_START, SECURITY_TARGET,
};
use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{GamePhase, Keyword, PlayerId};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

fn ch16_atk() -> DebugRunnerBuilder {
    fixtures::builder(&[
        fixtures::ch11_14_cards(),
        fixtures::ch15_04_cards(),
        fixtures::ch15_07_cards(),
        fixtures::ch15_08_cards(),
        fixtures::ch16_repl_cards(),
        fixtures::ch17_cards(),
        fixtures::ch16_atk_cards(),
    ])
}

/// P0 (memory `memory`, hand `hand0`) to move in its main phase; both
/// players have 6 deck cards; P0 has 3 security cards, P1 the stack `sec1`.
fn board_with(hand0: &[&str], sec1: &[&str], memory: i16) -> DebugRunner {
    ch16_atk()
        .hand(0, hand0)
        .deck(0, &[FILL; 6])
        .deck(1, &[FILL; 6])
        .security(0, &[FILL; 3])
        .security(1, sec1)
        .memory(memory)
        .start()
}

/// P0 (memory 5, empty hand) to move; P1 has the security stack `sec1`.
fn board(sec1: &[&str]) -> DebugRunner {
    board_with(&[], sec1, 5)
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

/// Index in the event log (since `cp`) of the first security reveal.
fn first_reveal(r: &DebugRunner, cp: usize) -> Option<usize> {
    r.events_since(cp)
        .iter()
        .position(|e| matches!(e, GameEvent::SecurityReveal { .. }))
}

/// Index in the event log (since `cp`) of the first effect-sourced memory
/// change whose source is `card_id`.
fn memory_event_index(r: &DebugRunner, cp: usize, card_id: &str) -> Option<usize> {
    r.events_since(cp).iter().position(
        |e| matches!(e, GameEvent::MemoryChange { source_card_id: Some(id), .. } if id == card_id),
    )
}

/// Index in the event log (since `cp`) of the first time `player`'s
/// `card_id` reached the trash.
fn trash_event_index(r: &DebugRunner, cp: usize, player: PlayerId, card_id: &str) -> Option<usize> {
    r.events_since(cp).iter().position(
        |e| matches!(e, GameEvent::Trash { player: p, card_id: c, .. } if *p == player && c == card_id),
    )
}

/// A decision the engine offered.
#[derive(Debug, Clone, PartialEq)]
struct Prompt {
    player: PlayerId,
    kind: SelectionKind,
    phase: GamePhase,
    optional: bool,
    labels: Vec<String>,
    actions: Vec<u16>,
}

fn current_prompt(r: &DebugRunner) -> Option<Prompt> {
    r.pending_selection().map(|sel| Prompt {
        player: sel.selecting_player,
        kind: sel.kind,
        phase: r.current_phase(),
        optional: sel.is_optional,
        labels: sel
            .effect_choices
            .iter()
            .flatten()
            .map(|c| c.label.clone())
            .collect(),
        actions: sel.valid_action_ids.clone(),
    })
}

/// Battle-area slots a field pick (`OwnField` / `OppField`, or a block
/// window) offers, sorted.
fn offered_slots(p: &Prompt) -> Vec<u16> {
    let mut slots: Vec<u16> = p.actions.iter().map(|&a| a - ATTACK_START).collect();
    slots.sort();
    slots
}

/// Settle every pending decision: DECLINE every optional prompt (counter and
/// block windows included) and take the only option of every other prompt.
/// Returns the decisions offered, in order.
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

/// Whether `player` may take `action` now (its mask bit).
fn legal(r: &DebugRunner, player: PlayerId, action: u16) -> bool {
    build_action_mask(&r.game, player)[action as usize] == 1.0
}

/// Whether `attacker`'s player may attack `target` with it at the end of its
/// turn: a pending selection of that player whose actions include the attack
/// (an end-of-turn keyword offering its attack as its own decision inside the
/// `[End of Your Turn]` pool, §16-33-2 / §16-38-2 / §16-44-2).
fn eot_attack_offered(r: &DebugRunner, attacker: PermanentHandle, target: u16) -> bool {
    let attack = encode_attack(attacker.index as u16, target);
    r.turn_player() == attacker.player
        && r.pending_selection().is_some_and(|sel| {
            sel.selecting_player == attacker.player && sel.valid_action_ids.contains(&attack)
        })
}

/// Declare the attack `eot_attack_offered` found (the keyword's decision).
fn declare_eot_attack(r: &mut DebugRunner, attacker: PermanentHandle, target: u16) {
    let attack = encode_attack(attacker.index as u16, target);
    r.execute_action(attacker.player, attack)
        .unwrap_or_else(|e| panic!("the attack was rejected: {e:?}"));
}

/// Index in the event log (since `cp`) of the first attack declaration.
fn attack_event_index(r: &DebugRunner, cp: usize) -> Option<usize> {
    r.events_since(cp)
        .iter()
        .position(|e| matches!(e, GameEvent::Attack { .. }))
}

/// After an end-of-turn attack, the engine may park in `EndOfTurnAction`
/// with PASS as the only legal action (G-ENGINE-ENGAGE-EOT-PASS-ONLY-STEP);
/// take it.
fn finish_end_of_turn(r: &mut DebugRunner) {
    for _ in 0..4 {
        if r.current_phase() != GamePhase::EndOfTurnAction {
            return;
        }
        let tp = r.turn_player();
        let mask = build_action_mask(&r.game, tp);
        let legal: Vec<usize> = (0..mask.len()).filter(|&i| mask[i] == 1.0).collect();
        assert_eq!(legal, vec![PASS as usize], "only PASS is left");
        r.game.decode_action(PASS, tp);
        settle_declining(r);
    }
}

/// End P0's turn and settle the end-of-turn triggers (declining any optional
/// one).
fn end_p0_turn(r: &mut DebugRunner) {
    assert_eq!(r.turn_player(), 0);
    r.end_turn();
    settle_declining(r);
}

/// Pass P0's turn (PASS at any end-of-turn window), then end P1's turn and
/// settle it, so the scenario observes P1's end of turn.
fn through_p1_end_of_turn(r: &mut DebugRunner) {
    end_p0_turn(r);
    if r.current_phase() == GamePhase::EndOfTurnAction {
        r.game.decode_action(PASS, 0);
        settle_declining(r);
    }
    assert_eq!(r.turn_player(), 1, "P1's turn");
    r.end_turn();
    settle_declining(r);
}

/// The memory at `player`'s side: positive when `player` has memory.
fn memory_for(r: &DebugRunner, player: PlayerId) -> i16 {
    if r.turn_player() == player {
        r.memory()
    } else {
        -r.memory()
    }
}

// ─── §16-4 <Security A.> ────────────────────────────────────────────────────

/// §16-4-2 "<Security A.> effects are persistent effects. The number of
/// security checks is modified as soon as the effect is applied. (Example: If
/// a Digimon gained <Security A. +1> at the point when it performs a security
/// check, it will perform 1 additional security check.)"
///
/// The rule's own example. P0's `WA_GAIN_SA` ("[When Attacking] This Digimon
/// gains <Security A. +1> for the turn") attacks P1: the keyword it gained
/// before checking applies at once, so the attack checks 2 cards. Nothing is
/// activated for <Security A.> itself: the only decision is none at all.
/// Control: `LV4_PLAIN` (same DP, no text) checks 1 card.
#[test]
fn security_attack_kind_matches_rule() {
    cite!(
        "16-4-2",
        "ede600958f0890b95c789008d3afe5b53e50db4161d4281e67c63863397b17b6"
    );
    let attack = |attacker: &str| {
        let mut r = board(&[FILL; 3]);
        let a = r.place_on_field(0, attacker, Some(0));
        let cp = r.event_checkpoint();
        r.attack_player(a, 1, false);
        let prompts = settle_declining(&mut r);
        (r, cp, prompts)
    };

    let (r, cp, _) = attack("LV4_PLAIN");
    assert_eq!(revealed(&r, cp, 1), owned(&[FILL]), "control: 1 check");

    let (r, cp, prompts) = attack("WA_GAIN_SA");
    assert!(prompts.is_empty(), "no decision: {prompts:?}");
    assert_eq!(
        revealed(&r, cp, 1),
        owned(&[FILL, FILL]),
        "the <Security A. +1> gained before the check modifies it"
    );
    assert_eq!(r.security_count(1), 1);
}

/// §16-4-1 "<Security A.> is a keyword effect where the Digimon with this
/// effect will have a modified number of security checks according to the
/// number specified for this effect."
/// §16-4-3 "If 1 Digimon has multiple instances of <Security A.>, the number
/// of security checks that can be performed is modified according to their
/// numerical values. However, even if a card has multiple instances of
/// <Security A.>, the effect isn't considered to be an effect with the
/// combined values from <Security A.>."
/// §16-4-4 "Even if a negative value is the result of the number of security
/// checks being modified, the actual number of security checks is 0."
/// §11-5-1-2-1 "In this situation, the player that made the attack
/// declaration wins the game. However, they don't win if the attacking
/// Digimon can't perform any security checks."
///
/// - Two instances: `SA_PLUS1` over `SA_PLUS1` (its own <Security A. +1>
///   and the inherited one of its digivolution card) checks 1 + 1 + 1 = 3
///   cards (each instance counts by its own value).
/// - `SA_MINUS2` (<Security A. -2>) attacks P1 (3 security cards): 1 - 2
///   floors at 0 checks; nothing is checked.
/// - `SA_MINUS2` attacks P1 with 0 security cards: the attack succeeds but
///   checks nothing, so P0 does not win (11-5-1-2-1). Control: `LV4_PLAIN`
///   attacking P1 with 0 security cards wins.
#[test]
fn security_attack_instances_do_not_combine_and_floor_at_zero() {
    cite!(
        "16-4-1",
        "1f02fe61f85b802f89d5e973c2fec646175ce8f951dd5e99bb64977a11e7a6f3"
    );
    cite!(
        "16-4-3",
        "34c504c97d55fc3021d205944741c2ba552d607bc3a55760b55007201ed204f2"
    );
    cite!(
        "16-4-4",
        "5e0cbbbee8f1e891cb758919c6c03d3676008586d95c277efc59647da32be870"
    );
    cite!(
        "11-5-1-2-1",
        "0b0b19e86be3ec53ff9ab2c06de6dd83ead8717b34a4515f033a08e2e73d367e"
    );
    let mut r = board(&[FILL; 5]);
    let a = r.place_stack(0, &["SA_PLUS1", "SA_PLUS1"]);
    let cp = r.event_checkpoint();
    r.attack_player(a, 1, false);
    settle_declining(&mut r);
    assert_eq!(
        revealed(&r, cp, 1),
        owned(&[FILL, FILL, FILL]),
        "two <Security A. +1> instances: 3 checks"
    );

    let mut r = board(&[FILL; 3]);
    let a = r.place_on_field(0, "SA_MINUS2", Some(0));
    let cp = r.event_checkpoint();
    r.attack_player(a, 1, false);
    settle_declining(&mut r);
    assert!(revealed(&r, cp, 1).is_empty(), "1 - 2 floors at 0 checks");
    assert_eq!(r.security_count(1), 3);

    let mut r = board(&[]);
    let a = r.place_on_field(0, "LV4_PLAIN", Some(0));
    r.attack_player(a, 1, false);
    settle_declining(&mut r);
    assert_eq!(
        r.winner(),
        Some(0),
        "control: a direct attack on 0 security wins"
    );

    let mut r = board(&[]);
    let a = r.place_on_field(0, "SA_MINUS2", Some(0));
    r.attack_player(a, 1, false);
    settle_declining(&mut r);
    assert!(
        !r.game_over(),
        "an attacker that can perform no security check does not win (winner {:?})",
        r.winner()
    );
}

// ─── §16-5 <Blocker> ────────────────────────────────────────────────────────

/// §16-5-1 "<Blocker> is a keyword effect that applies a rule that allows a
/// Digimon with that effect to block."
/// §16-5-2 "<Blocker> is a persistent effect."
///
/// P0's `LV5_PLAIN` attacks P1. P1 has `BLOCKER` (<Blocker>, slot 0), a
/// `FILL` over `BLOCKER_INH` (inherited <Blocker>, slot 1) and a plain `FILL`
/// (slot 2). The first decision of the attack is the block window, offering
/// the two Digimon with <Blocker> (the inherited one included) and not the
/// plain one; nothing is activated for <Blocker> before it.
#[test]
fn blocker_kind_matches_rule() {
    cite!(
        "16-5-1",
        "b0b00e672002d738955ef43656dbb33d78209a1493df8a9b98df1f8ad2287ff2"
    );
    cite!(
        "16-5-2",
        "3b93854b487ec576497d20e038299d4d8d7bda8d6432337fcf31694269d49faf"
    );
    let mut r = board(&[FILL; 3]);
    let attacker = r.place_on_field(0, "LV5_PLAIN", Some(0));
    r.place_on_field(1, "BLOCKER", Some(0));
    r.place_stack(1, &["BLOCKER_INH", FILL]);
    r.place_on_field(1, FILL, Some(0));
    r.attack_player(attacker, 1, false);
    let prompts = settle_declining(&mut r);
    assert_eq!(
        prompts.len(),
        1,
        "one decision: the block window: {prompts:?}"
    );
    assert_eq!(prompts[0].phase, GamePhase::BlockTiming);
    assert_eq!(prompts[0].player, 1);
    assert_eq!(offered_slots(&prompts[0]), vec![0, 1]);
}

/// §16-5-3 "Even if 1 Digimon has multiple instances of <Blocker>, the number
/// of blocks that can be performed during the block timing is only 1."
///
/// P1's only Digimon is `BLOCKER` over `BLOCKER_INH`: two <Blocker>
/// instances. P0's `LV5_PLAIN` attacks P1: the block window offers that
/// Digimon once; it blocks, battles and is deleted, and no second block is
/// offered (nothing checks security).
#[test]
fn blocker_once_per_block_timing_even_with_two_instances() {
    cite!(
        "16-5-3",
        "346dc562f7c33a90433532a482b9e41e01bbc0ffa54ddd3f099ec0d100b40620"
    );
    let mut r = board(&[FILL; 3]);
    let attacker = r.place_on_field(0, "LV5_PLAIN", Some(0));
    r.place_stack(1, &["BLOCKER_INH", "BLOCKER"]);
    let cp = r.event_checkpoint();
    r.attack_player(attacker, 1, false);
    let p = current_prompt(&r).expect("the block window");
    assert_eq!(p.phase, GamePhase::BlockTiming);
    assert_eq!(p.actions, vec![ATTACK_START], "offered once: {p:?}");
    pick_field(&mut r, 0);
    assert!(
        current_prompt(&r).is_none(),
        "no second block: {:?}",
        current_prompt(&r)
    );
    assert!(
        field_top_ids(&r, 1).is_empty(),
        "the blocker battled and was deleted"
    );
    assert!(revealed(&r, cp, 1).is_empty());
}

// ─── §16-7 <Piercing> ───────────────────────────────────────────────────────

/// §16-7-2 "<Piercing> is a trigger-type effect that triggers and activates
/// when a Digimon with this effect battles an opponent's Digimon and the
/// opponent's Digimon is deleted in battle."
///
/// P0's `PIERCING` (7000 DP) attacks P1.
/// - P1 blocks with `BLOCKER` (3000 DP): the Digimon it battles is deleted
///   in battle, so <Piercing> checks security (1 card).
/// - P0's `PIERCING` attacks P1's suspended `BARRIER` (4000 DP, <Barrier>);
///   P1 trashes its top security card instead of the deletion: the battled
///   Digimon is NOT deleted in battle, so <Piercing> doesn't trigger
///   (nothing is checked; the `BARRIER` trash is the only security change).
#[test]
fn piercing_kind_matches_rule() {
    cite!(
        "16-7-2",
        "22c318db58017ed28fc297f8b3350ac8c26c87005c3e65930422e8b2f1cc8328"
    );
    let mut r = board(&[FILL; 3]);
    let attacker = r.place_on_field(0, "PIERCING", Some(0));
    r.place_on_field(1, "BLOCKER", Some(0));
    let cp = r.event_checkpoint();
    r.attack_player(attacker, 1, false);
    assert_eq!(r.current_phase(), GamePhase::BlockTiming);
    pick_field(&mut r, 0);
    settle_declining(&mut r);
    assert!(
        field_top_ids(&r, 1).is_empty(),
        "the blocker was deleted in battle"
    );
    assert_eq!(revealed(&r, cp, 1), owned(&[FILL]), "<Piercing> checked");

    let mut r = board(&[FILL; 3]);
    let attacker = r.place_on_field(0, "PIERCING", Some(0));
    let target = place_suspended(&mut r, 1, "BARRIER");
    let cp = r.event_checkpoint();
    r.attack_digimon(attacker, target, false);
    let p = current_prompt(&r).expect("the <Barrier> offer");
    assert!(p.optional && p.player == 1, "{p:?}");
    accept(&mut r, 1);
    settle_declining(&mut r);
    assert_eq!(field_top_ids(&r, 1), owned(&["BARRIER"]), "not deleted");
    assert!(revealed(&r, cp, 1).is_empty(), "no <Piercing> check");
    assert_eq!(r.security_count(1), 2, "only <Barrier>'s trash");
}

/// §16-7-3 "The security check from using <Piercing> is mandatory."
///
/// P0's `PIERCING` attacks P1's suspended `FILL` and deletes it: the security
/// check follows with no decision offered to either player.
#[test]
fn piercing_optionality_matches_rule() {
    cite!(
        "16-7-3",
        "e87129c8e51cd2c0615512a5986d73c0c8a274076b23a10f1af8e0d7d9f50e04"
    );
    let mut r = board(&[FILL; 3]);
    let attacker = r.place_on_field(0, "PIERCING", Some(0));
    let target = place_suspended(&mut r, 1, FILL);
    let cp = r.event_checkpoint();
    r.attack_digimon(attacker, target, false);
    assert!(
        current_prompt(&r).is_none(),
        "no decision: {:?}",
        current_prompt(&r)
    );
    assert_eq!(revealed(&r, cp, 1), owned(&[FILL]));
}

/// §16-7-1 "<Piercing> is a keyword effect that reads "When this Digimon
/// deletes your opponent's Digimon in battle while attacking, it checks
/// security before the attack ends.""
///
/// P0's `PIERCING`, NOT attacking, battles P1's `FILL` through an effect
/// battle in P0's main phase and deletes it: it is not attacking, so nothing
/// checks security. Control: the same deletion while attacking checks 1 card
/// (`piercing_optionality_matches_rule`).
#[test]
fn piercing_timing_matches_rule() {
    cite!(
        "16-7-1",
        "509c84fa24d0659b902cea3dd2adda97473201ddd484c088d2089eeddec4464e"
    );
    let mut r = board(&[FILL; 3]);
    let piercer = r.place_on_field(0, "PIERCING", Some(0));
    let target = r.place_on_field(1, FILL, Some(0));
    let cp = r.event_checkpoint();
    r.battle_digimon(piercer, target);
    settle_declining(&mut r);
    assert!(field_top_ids(&r, 1).is_empty(), "deleted in the battle");
    assert!(revealed(&r, cp, 1).is_empty(), "not attacking: no check");
    assert_eq!(r.security_count(1), 3);
}

// ─── §16-9 <Jamming> ────────────────────────────────────────────────────────

/// §16-9-2 "<Jamming> is a persistent effect."
///
/// P0's `WA_JAM` (5000 DP, "[When Attacking] This Digimon gains <Jamming>
/// for the turn") attacks P1 and checks `LV5_PLAIN` (7000 DP): the <Jamming>
/// it gained applies to that security battle at once, with no activation and
/// no decision: it loses but is not deleted. Control: `LV4_PLAIN` (5000 DP)
/// is deleted by the same check.
#[test]
fn jamming_kind_matches_rule() {
    cite!(
        "16-9-2",
        "6edfd036f97a93eca278c74400b9dbe3cddd227f44a88ec2fff6d17a61e88449"
    );
    let attack = |attacker: &str| {
        let mut r = board(&[FILL, FILL, "LV5_PLAIN"]);
        let a = r.place_on_field(0, attacker, Some(0));
        r.attack_player(a, 1, false);
        let prompts = settle_declining(&mut r);
        (r, prompts)
    };
    let (r, _) = attack("LV4_PLAIN");
    assert!(field_top_ids(&r, 0).is_empty(), "control: deleted");

    let (r, prompts) = attack("WA_JAM");
    assert!(prompts.is_empty(), "no decision: {prompts:?}");
    assert_eq!(field_top_ids(&r, 0), owned(&["WA_JAM"]), "not deleted");
}

// ─── §16-13 <Retaliation> ───────────────────────────────────────────────────

/// §16-13-1 "<Retaliation> is a keyword effect where a battled opponent's
/// Digimon is deleted when a Digimon with this effect is deleted in battle."
/// §16-13-3 "The processing from <Retaliation> is mandatory."
///
/// - P0's `LV5_PLAIN` (7000 DP) attacks P1's suspended `RETALIATION` (3000
///   DP) and deletes it in battle: <Retaliation> deletes `LV5_PLAIN`, with no
///   decision offered to either player.
/// - P0's own `RETALIATION` attacks P1's suspended `LV5_PLAIN` and loses: it
///   deletes `LV5_PLAIN` too.
#[test]
fn retaliation_mandatory_deletes_battled_digimon() {
    cite!(
        "16-13-1",
        "197dbe928f579248d02b9d5fb4f071bc92ac54fc072d563586d73ffe1a229d95"
    );
    cite!(
        "16-13-3",
        "962cf545a051589f87f8191288b34b9f4c9cf55f194b46dce7beace3b7e9aaee"
    );
    let mut r = board(&[FILL; 3]);
    let attacker = r.place_on_field(0, "LV5_PLAIN", Some(0));
    let target = place_suspended(&mut r, 1, "RETALIATION");
    r.attack_digimon(attacker, target, false);
    let prompts = settle_declining(&mut r);
    assert!(prompts.is_empty(), "no decision: {prompts:?}");
    assert!(
        field_top_ids(&r, 1).is_empty(),
        "RETALIATION lost the battle"
    );
    assert!(
        field_top_ids(&r, 0).is_empty(),
        "<Retaliation> deleted the attacker"
    );
    assert_eq!(trash_ids(&r, 0), owned(&["LV5_PLAIN"]));

    let mut r = board(&[FILL; 3]);
    let attacker = r.place_on_field(0, "RETALIATION", Some(0));
    let target = place_suspended(&mut r, 1, "LV5_PLAIN");
    r.attack_digimon(attacker, target, false);
    let prompts = settle_declining(&mut r);
    assert!(prompts.is_empty(), "no decision: {prompts:?}");
    assert!(
        field_top_ids(&r, 0).is_empty(),
        "RETALIATION lost the battle"
    );
    assert!(
        field_top_ids(&r, 1).is_empty(),
        "<Retaliation> deleted the winner"
    );
}

/// §16-13-2 "<Retaliation> is a TRIGGER-TYPE effect that triggers when just
/// the Digimon with this effect is deleted in battle."
///
/// P1 has `RETALIATION` (suspended) and `OBS_OWN_DEL_MEM` ("[All Turns] When
/// one of your Digimon is deleted, gain 1 memory"). P0's `LV5_PLAIN` deletes
/// `RETALIATION` in battle: both of P1's effects trigger on that deletion and
/// pend in one `TriggerOrder` for P1. P1 activates `OBS_OWN_DEL_MEM` first:
/// its memory gain happens before <Retaliation> deletes `LV5_PLAIN`.
#[test]
fn retaliation_kind_matches_rule() {
    cite!(
        "16-13-2",
        "e0808def8b55cd54b6651e11816946c6e64f1e9f0f3bf60ba4eb9618f7f87999"
    );
    let mut r = board(&[FILL; 3]);
    let attacker = r.place_on_field(0, "LV5_PLAIN", Some(0));
    let target = place_suspended(&mut r, 1, "RETALIATION");
    r.place_on_field(1, "OBS_OWN_DEL_MEM", Some(0));
    let cp = r.event_checkpoint();
    r.attack_digimon(attacker, target, false);
    let mut cards: Vec<String> = order_choices(&r, 1)
        .iter()
        .map(|l| branch_card(l).to_string())
        .collect();
    cards.sort();
    assert_eq!(
        cards,
        owned(&["OBS_OWN_DEL_MEM", "RETALIATION"]),
        "both pend together"
    );
    choose(&mut r, "OBS_OWN_DEL_MEM");
    settle_declining(&mut r);
    let gain = memory_event_index(&r, cp, "OBS_OWN_DEL_MEM").expect("the observer resolved");
    let retaliation =
        trash_event_index(&r, cp, 0, "LV5_PLAIN").expect("<Retaliation> deleted the attacker");
    assert!(
        gain < retaliation,
        "in the chosen order: {gain} < {retaliation}"
    );
}

/// §16-13-2 "<Retaliation> is a trigger-type effect that triggers when just
/// the Digimon with this EFFECT IS DELETED IN BATTLE."
///
/// - P0's `MAIN_DEL1` ("[Main] Delete 1 of your opponent's Digimon") deletes
///   P1's `RETALIATION`: not a battle, so nothing else is deleted.
/// - P0's `LV5_PLAIN` attacks P1's suspended `RETALIATION` and deletes it in
///   battle: the trigger fires (the battled `LV5_PLAIN` is deleted).
#[test]
fn retaliation_timing_matches_rule() {
    cite!(
        "16-13-2",
        "e0808def8b55cd54b6651e11816946c6e64f1e9f0f3bf60ba4eb9618f7f87999"
    );
    let mut r = board_with(&["MAIN_DEL1"], &[FILL; 3], 5);
    r.place_on_field(0, "LV5_PLAIN", Some(0));
    r.place_on_field(1, "RETALIATION", Some(0));
    use_option(&mut r, 0, "MAIN_DEL1");
    resolve_prompts(&mut r);
    assert!(
        field_top_ids(&r, 1).is_empty(),
        "RETALIATION deleted by the effect"
    );
    assert_eq!(
        field_top_ids(&r, 0),
        owned(&["LV5_PLAIN"]),
        "no <Retaliation>"
    );

    let mut r = board(&[FILL; 3]);
    let attacker = r.place_on_field(0, "LV5_PLAIN", Some(0));
    let target = place_suspended(&mut r, 1, "RETALIATION");
    r.attack_digimon(attacker, target, false);
    settle_declining(&mut r);
    assert!(
        field_top_ids(&r, 0).is_empty(),
        "battle deletion: <Retaliation>"
    );
}

/// §16-13-2 (with §15-4-3-2 / §15-4-3-5): the effects one deletion in battle
/// triggers pend in ONE pool, so a "when a Digimon is deleted" observer still
/// activates after an `[On Deletion]` of the same deletion has paused for a
/// player decision.
///
/// P0's `OD_TRASH_HAND_MEM` ("[On Deletion] By trashing 1 card in the hand,
/// gain 1 memory") attacks P1's suspended `LV5_PLAIN` and is deleted in
/// battle. P1 has `OBS_OPP_DEL_MEM` ("When one of your opponent's Digimon is
/// deleted, gain 1 memory"). The turn player's `[On Deletion]` activates
/// first and waits on P0's optional hand pick; P1's observer has not
/// resolved yet. Once P0 pays, P1's observer still activates (before Task 5
/// of the 2026-10 engine fixes it was dropped whenever the `[On Deletion]`
/// paused at a top-level deletion).
#[test]
fn deletion_observer_activates_after_on_deletion_pauses_for_a_decision() {
    cite!(
        "16-13-2",
        "e0808def8b55cd54b6651e11816946c6e64f1e9f0f3bf60ba4eb9618f7f87999"
    );
    let mut r = board_with(&[FILL], &[FILL; 3], 5);
    let attacker = r.place_on_field(0, "OD_TRASH_HAND_MEM", Some(0));
    let target = place_suspended(&mut r, 1, "LV5_PLAIN");
    r.place_on_field(1, "OBS_OPP_DEL_MEM", Some(0));
    let cp = r.event_checkpoint();
    r.attack_digimon(attacker, target, false);

    assert!(
        field_top_ids(&r, 0).is_empty(),
        "OD_TRASH_HAND_MEM lost the battle"
    );
    let sel = r.pending_selection().expect("the [On Deletion] hand pick");
    assert_eq!(sel.selecting_player, 0, "{sel:?}");
    assert!(sel.is_optional, "'By trashing' is optional: {sel:?}");
    assert!(
        memory_sources(&r, cp).is_empty(),
        "nothing has resolved yet: {:?}",
        memory_sources(&r, cp)
    );

    pick_hand_card(&mut r, 0, FILL);
    settle_declining(&mut r);
    assert_eq!(
        memory_sources(&r, cp),
        vec![
            (0, "OD_TRASH_HAND_MEM".to_string()),
            (1, "OBS_OPP_DEL_MEM".to_string()),
        ],
        "the turn player's [On Deletion], then the non-turn player's observer"
    );
}

// ─── §16-15 <Rush> ──────────────────────────────────────────────────────────

/// §16-15-1 "<Rush> is a keyword effect where the Digimon with this effect
/// can also attack in the same turn it was played."
///
/// P0 plays `RUSH` (<Rush>) from the hand: it may attack this turn, and its
/// attack checks P1's security. Control: P0 plays `FILL` (no <Rush>): it may
/// not attack this turn.
#[test]
fn rush_attacks_the_turn_it_was_played() {
    cite!(
        "16-15-1",
        "d3076a2fd1b9bd0395114d921a65a5a42b2b5c87f2ab8127af2f406c29fa6be5"
    );
    let mut r = board_with(&[FILL], &[FILL; 3], 5);
    r.game.decode_action(PLAY_HAND_START, 0);
    settle_declining(&mut r);
    assert_eq!(field_top_ids(&r, 0), owned(&[FILL]));
    assert!(
        !legal(&r, 0, encode_attack(0, SECURITY_TARGET)),
        "control: played this turn, no <Rush>"
    );

    let mut r = board_with(&["RUSH"], &[FILL; 3], 5);
    r.game.decode_action(PLAY_HAND_START, 0);
    settle_declining(&mut r);
    assert_eq!(field_top_ids(&r, 0), owned(&["RUSH"]));
    assert!(
        legal(&r, 0, encode_attack(0, SECURITY_TARGET)),
        "<Rush> attacks"
    );
    let cp = r.event_checkpoint();
    r.game.decode_action(encode_attack(0, SECURITY_TARGET), 0);
    settle_declining(&mut r);
    assert_eq!(revealed(&r, cp, 1), owned(&[FILL]));
}

/// §16-15-2 "<Rush> is a persistent effect."
///
/// P0 plays `FILL` (it can't attack this turn), then plays `RUSH_AURA`
/// ("[All Turns] All of your Digimon gain <Rush>"): `FILL` gains <Rush> with
/// no activation and no decision, and may attack at once.
#[test]
fn rush_kind_matches_rule() {
    cite!(
        "16-15-2",
        "ec1fa02add5e5d9049a9ea7618ce86302243c2d1c2b6a83e6c304fdcc4c9bd59"
    );
    let mut r = board_with(&[FILL, "RUSH_AURA"], &[FILL; 3], 5);
    r.game.decode_action(PLAY_HAND_START, 0);
    settle_declining(&mut r);
    assert!(!legal(&r, 0, encode_attack(0, SECURITY_TARGET)));
    r.game.decode_action(PLAY_HAND_START, 0);
    let prompts = settle_declining(&mut r);
    assert!(prompts.is_empty(), "no decision: {prompts:?}");
    assert_eq!(field_top_ids(&r, 0), owned(&[FILL, "RUSH_AURA"]));
    assert!(
        legal(&r, 0, encode_attack(0, SECURITY_TARGET)),
        "FILL gained <Rush>"
    );
}

// ─── §16-16 <Blitz> ─────────────────────────────────────────────────────────
//
// The <Blitz> attack is checked BEFORE any prompt is settled, in either
// representation a fix may choose: a pending selection for P0 whose actions
// include the attack, or an attack / PASS window read from the action mask.
// While that window is open, nothing but the <Blitz> Digimon's attacks and
// the decline may be legal: the memory is on the opponent's side, so P0's
// turn continues only for the <Blitz> processing. Each board carries a
// playable `FILL` in the hand and `MAIN_TRASH_SEC_DRAW` ([Main] effect) on
// the field as witnesses (both legal before the digivolution).

/// P0 (memory `memory`, hand `[blitz, FILL]`) has `FILL` (red Lv.3, slot 0,
/// the digivolution target), `MAIN_TRASH_SEC_DRAW` (slot 1) and `extra`; P1
/// has 3 security cards. Returns the board, the `FILL` handle and the
/// `MAIN_TRASH_SEC_DRAW` handle.
fn blitz_board(
    blitz: &str,
    memory: i16,
    extra: &[&str],
) -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut r = board_with(&[blitz, FILL], &[FILL; 3], memory);
    let base = r.place_on_field(0, FILL, Some(0));
    let main = r.place_on_field(0, "MAIN_TRASH_SEC_DRAW", Some(0));
    for id in extra {
        r.place_on_field(0, id, Some(0));
    }
    assert!(
        legal(&r, 0, PLAY_HAND_START + 1),
        "witness: FILL is playable"
    );
    assert!(
        legal(&r, 0, field_main_action(main)),
        "witness: the [Main] effect is usable"
    );
    (r, base, main)
}

/// Digivolve P0's `base` into the hand card at index 0.
fn digivolve_from_hand(r: &mut DebugRunner, base: PermanentHandle) {
    r.game
        .decode_action(encode_digivolve(0, base.index as u16), 0);
}

/// Whether `action` is an attack declared by P0's `h` (any target).
fn is_attack_by(h: PermanentHandle, action: u16) -> bool {
    action >= encode_attack(h.index as u16, 0)
        && action <= encode_attack(h.index as u16, SECURITY_TARGET)
}

/// Whether P0 is offered the <Blitz> attack on P1 with `h` NOW (nothing
/// settled): a pending selection for P0 whose actions include the attack, or
/// (no selection pending) a legal attack action in P0's turn.
fn blitz_offered(r: &DebugRunner, h: PermanentHandle) -> bool {
    let attack = encode_attack(h.index as u16, SECURITY_TARGET);
    if r.turn_player() != 0 {
        return false;
    }
    match r.pending_selection() {
        Some(sel) => sel.selecting_player == 0 && sel.valid_action_ids.contains(&attack),
        None => legal(r, 0, attack),
    }
}

/// While the <Blitz> window is open, only `h`'s attacks and the decline are
/// legal: in particular not the `FILL` in the hand (index `hand_index`) and
/// not `main`'s [Main] effect.
fn assert_only_blitz_legal(
    r: &DebugRunner,
    h: PermanentHandle,
    hand_index: u16,
    main: PermanentHandle,
) {
    let mask = build_action_mask(&r.game, 0);
    let stray: Vec<u16> = (0..mask.len())
        .filter(|&i| mask[i] == 1.0)
        .map(|i| i as u16)
        .filter(|&a| a != PASS && !is_attack_by(h, a))
        .collect();
    assert!(
        stray.is_empty(),
        "only the <Blitz> attack and its decline are legal, also: {stray:?}"
    );
    assert_eq!(
        mask[(PLAY_HAND_START + hand_index) as usize],
        0.0,
        "the hand card is not playable"
    );
    assert_eq!(
        mask[field_main_action(main) as usize],
        0.0,
        "the [Main] effect is not usable"
    );
}

/// Take the offered <Blitz> attack on P1 with `h`.
fn take_blitz_attack(r: &mut DebugRunner, h: PermanentHandle) {
    let attack = encode_attack(h.index as u16, SECURITY_TARGET);
    if r.pending_selection().is_some() {
        r.execute_action(0, attack)
            .unwrap_or_else(|e| panic!("the <Blitz> attack was rejected: {e:?}"));
    } else {
        r.game.decode_action(attack, 0);
    }
}

/// Decline the offered <Blitz> attack.
fn decline_blitz(r: &mut DebugRunner) {
    if r.pending_selection().is_some() {
        r.decline_optional_trigger()
            .unwrap_or_else(|e| panic!("declining <Blitz> was rejected: {e:?}"));
    } else {
        r.game.decode_action(PASS, 0);
    }
}

/// §16-16-1 "<Blitz> is a keyword effect where a Digimon with this effect
/// can attack if your opponent has 1 or more memory."
/// §16-16-3 "<Blitz> has a processing condition that requires the memory to
/// be at 1 or more on your opponent's side. "This Digimon may attack" is
/// optional processing."
///
/// P0 (memory 2) digivolves `FILL` into `BLITZ` (cost 3): the memory moves
/// to 1 on P1's side and `BLITZ`'s [When Digivolving] <Blitz> activates.
/// Before anything is settled the attack is offered, and nothing else (the
/// `FILL` in the hand, the [Main] effect) is legal.
/// - P0 attacks: it checks P1's security, then the turn passes to P1.
/// - Or P0 declines: no attack; the turn passes to P1.
#[test]
fn blitz_optionality_matches_rule() {
    cite!(
        "16-16-1",
        "d778aec128af5461b1af4cdea667523c605c1f9ec1464b2c5a19efa36599dfb7"
    );
    cite!(
        "16-16-3",
        "60219343db57508a25e6f5734ecf8fa3a4505d9edd50bc8820b15b4b42e87e95"
    );
    let (mut r, base, main) = blitz_board("BLITZ", 2, &[]);
    let cp = r.event_checkpoint();
    digivolve_from_hand(&mut r, base);
    assert_eq!(field_top_ids(&r, 0)[0], "BLITZ");
    assert!(blitz_offered(&r, base), "<Blitz>: the attack is offered");
    assert_only_blitz_legal(&r, base, 0, main);
    take_blitz_attack(&mut r, base);
    settle_declining(&mut r);
    assert_eq!(revealed(&r, cp, 1), owned(&[FILL]), "the <Blitz> attack");
    assert_eq!(r.turn_player(), 1, "then the turn ended");

    let (mut r, base, main) = blitz_board("BLITZ", 2, &[]);
    let cp = r.event_checkpoint();
    digivolve_from_hand(&mut r, base);
    assert!(blitz_offered(&r, base));
    assert_only_blitz_legal(&r, base, 0, main);
    decline_blitz(&mut r);
    settle_declining(&mut r);
    assert!(revealed(&r, cp, 1).is_empty(), "declined: no attack");
    assert_eq!(r.turn_player(), 1);
}

/// §16-16-2 "<Blitz> effects execute processing."
///
/// <Blitz> is processing of the effect that contains it: its attack happens
/// while that effect activates. P0 (memory 4) has `OBS_DIGI_LOSE2` ("[Your
/// Turn] When one of your Digimon digivolves, lose 2 memory") and digivolves
/// `FILL` into `BLITZ` (cost 3, memory 1 left): `BLITZ`'s [When Digivolving]
/// and the observer pend in one `TriggerOrder`. P0 activates the observer
/// first (memory 1 on P1's side), then `BLITZ`'s effect: <Blitz> offers the
/// attack (only it and its decline are legal) and P0 attacks.
#[test]
fn blitz_kind_matches_rule() {
    cite!(
        "16-16-2",
        "328603fde8002d1505f8afaf5078c58bef06791b58034c634d9da38918b24d68"
    );
    let (mut r, base, main) = blitz_board("BLITZ", 4, &["OBS_DIGI_LOSE2"]);
    let cp = r.event_checkpoint();
    digivolve_from_hand(&mut r, base);
    let mut cards: Vec<String> = order_choices(&r, 0)
        .iter()
        .map(|l| branch_card(l).to_string())
        .collect();
    cards.sort();
    assert_eq!(
        cards,
        owned(&["BLITZ", "OBS_DIGI_LOSE2"]),
        "both pend together"
    );
    choose(&mut r, "OBS_DIGI_LOSE2");
    assert_eq!(memory_for(&r, 1), 1, "1 memory on P1's side");
    assert!(blitz_offered(&r, base), "<Blitz> offers the attack");
    assert_only_blitz_legal(&r, base, 0, main);
    take_blitz_attack(&mut r, base);
    settle_declining(&mut r);
    assert_eq!(revealed(&r, cp, 1), owned(&[FILL]));
}

/// §16-16-4 "After using <Blitz> to make an attack declaration, the attack
/// will proceed even if the memory moves to 0 or more."
///
/// P0 (memory 2) digivolves `FILL` into `BLITZ_WA` (cost 3; memory 1 on P1's
/// side) and attacks with <Blitz>. Its "[When Attacking] Gain 2 memory"
/// moves the memory to 1 on P0's side: the attack proceeds and checks P1's
/// security.
#[test]
fn blitz_attack_proceeds_after_memory_returns_to_zero_or_more() {
    cite!(
        "16-16-4",
        "0d42f6969a3ca7e86703ef08206e4c5c78f52328f8cb0fc420891bead538b350"
    );
    let (mut r, base, main) = blitz_board("BLITZ_WA", 2, &[]);
    let cp = r.event_checkpoint();
    digivolve_from_hand(&mut r, base);
    assert!(blitz_offered(&r, base), "<Blitz>: the attack is offered");
    assert_only_blitz_legal(&r, base, 0, main);
    take_blitz_attack(&mut r, base);
    settle_declining(&mut r);
    assert!(
        memory_event_index(&r, cp, "BLITZ_WA").is_some(),
        "the [When Attacking] gain resolved"
    );
    assert_eq!(revealed(&r, cp, 1), owned(&[FILL]), "the attack proceeded");
}

/// §16-16-5 "An attack can't be made using <Blitz> if the memory is at 0 or
/// more upon activating <Blitz>."
///
/// P0 (memory 3) has `OBS_DIGI_LOSE2` ("[Your Turn] When one of your Digimon
/// digivolves, lose 2 memory") and digivolves `FILL` into `BLITZ` (cost 3):
/// the memory is 0; `BLITZ`'s [When Digivolving] and the observer pend
/// together.
/// - P0 activates <Blitz> first: the memory is 0 (not 1 or more on P1's
///   side), so no attack can be made with it. The observer then moves the
///   memory to 2 on P1's side and the turn ends: no attack is offered at
///   any point and nothing else is left for P0 to do.
/// - Control: P0 activates the observer first (memory 2 on P1's side), then
///   <Blitz>: the attack is offered.
#[test]
fn blitz_cannot_attack_when_memory_already_zero_or_more() {
    cite!(
        "16-16-5",
        "3761cbb0148f1f0d343686eefb060b1103f687ebd03f0d5125cd9c3821887794"
    );
    let (mut r, base, _) = blitz_board("BLITZ", 3, &["OBS_DIGI_LOSE2"]);
    let cp = r.event_checkpoint();
    digivolve_from_hand(&mut r, base);
    choose(&mut r, "BLITZ");
    assert!(
        !blitz_offered(&r, base),
        "memory 0 at activation: no attack"
    );
    assert!(revealed(&r, cp, 1).is_empty(), "no attack was made");
    assert_eq!(r.turn_player(), 1, "the turn ended");

    let (mut r, base, main) = blitz_board("BLITZ", 3, &["OBS_DIGI_LOSE2"]);
    digivolve_from_hand(&mut r, base);
    choose(&mut r, "OBS_DIGI_LOSE2");
    assert!(
        blitz_offered(&r, base),
        "control: memory 2 on P1's side at activation"
    );
    assert_only_blitz_legal(&r, base, 0, main);
}

// ─── §16-23 <Raid> ──────────────────────────────────────────────────────────

/// P1 has two unsuspended `LV4_PLAIN` (5000 DP, slots 0 and 2), an
/// unsuspended `FILL` (1000 DP, slot 1) and 3 security cards.
fn raid_board() -> DebugRunner {
    let mut r = board(&[FILL; 3]);
    r.place_on_field(1, "LV4_PLAIN", Some(0));
    r.place_on_field(1, FILL, Some(0));
    r.place_on_field(1, "LV4_PLAIN", Some(0));
    r
}

/// §16-23-1 "<Raid> is a keyword effect that can switch the target of attack
/// to the opponent's unsuspended Digimon with the highest DP when a Digimon
/// with this keyword effect attacks."
/// §16-23-4 "If the opponent has multiple unsuspended Digimon with the
/// highest DP when <Raid> activates, the player that activated the effect
/// chooses a Digimon from among them and the attack target is switched to
/// it."
///
/// P0's `RAID` (7000 DP) attacks P1. <Raid> asks P0 (the player that
/// activated it) to choose between the two highest-DP unsuspended Digimon
/// (slots 0 and 2; not the 1000 DP `FILL`). P0 picks slot 2: the attack
/// target switches to it, it battles and is deleted, and nothing checks
/// security.
#[test]
fn raid_switches_to_highest_dp_unsuspended_attacker_chooses_tie() {
    cite!(
        "16-23-1",
        "baa73e73e69753c0ecf0648f0d2bf6a59b8ef1da68985c91e493a96caa491d4a"
    );
    cite!(
        "16-23-4",
        "5f5d2517390c82cb574e56e53bccaaac7ee51e4dce400f4874b9a06fed362dc8"
    );
    let mut r = raid_board();
    let attacker = r.place_on_field(0, "RAID", Some(0));
    let cp = r.event_checkpoint();
    r.attack_player(attacker, 1, false);
    let p = current_prompt(&r).expect("the <Raid> choice");
    assert_eq!(p.player, 0, "the activating player chooses: {p:?}");
    assert_eq!(p.kind, SelectionKind::OppField);
    assert_eq!(offered_slots(&p), vec![0, 2], "the tied highest-DP Digimon");
    pick_field(&mut r, 2);
    settle_declining(&mut r);
    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["LV4_PLAIN", FILL]),
        "the chosen Digimon battled and was deleted"
    );
    assert!(revealed(&r, cp, 1).is_empty(), "the attack target switched");
}

/// §16-23-3 "The processing from <Raid> is optional."
///
/// P0's `RAID` attacks P1; P0 declines <Raid>: the attack stays on P1 and
/// checks security; P1's Digimon are untouched.
#[test]
fn raid_optionality_matches_rule() {
    cite!(
        "16-23-3",
        "cc64ea1efe2c3fd61e40c135d8caf2d476a75a0bdf12e124fb73476a1d561b02"
    );
    let mut r = raid_board();
    let attacker = r.place_on_field(0, "RAID", Some(0));
    let cp = r.event_checkpoint();
    r.attack_player(attacker, 1, false);
    let p = current_prompt(&r).expect("the <Raid> choice");
    assert!(p.optional, "{p:?}");
    r.decline_optional_trigger().expect("decline <Raid>");
    settle_declining(&mut r);
    assert_eq!(revealed(&r, cp, 1), owned(&[FILL]));
    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["LV4_PLAIN", FILL, "LV4_PLAIN"])
    );
}

/// §16-23-2 "<Raid> is a trigger-type effect that triggers when A DIGIMON
/// WITH THIS EFFECT ATTACKS."
///
/// P0 has `RAID` on its field; its `LV5_PLAIN` (no <Raid>) attacks P1: no
/// <Raid> choice is offered and the attack checks security.
#[test]
fn raid_timing_matches_rule() {
    cite!(
        "16-23-2",
        "4522f354b5a17616ecdab28334b9b6b8f785d5453bc43f20ea49449a060d6657"
    );
    let mut r = raid_board();
    r.place_on_field(0, "RAID", Some(0));
    let attacker = r.place_on_field(0, "LV5_PLAIN", Some(0));
    let cp = r.event_checkpoint();
    r.attack_player(attacker, 1, false);
    let prompts = settle_declining(&mut r);
    assert!(prompts.is_empty(), "no <Raid>: {prompts:?}");
    assert_eq!(revealed(&r, cp, 1), owned(&[FILL]));
}

/// §16-23-2 "<Raid> is a TRIGGER-TYPE effect that triggers when a Digimon
/// with this effect attacks."
///
/// A trigger-type effect pends with the effects triggered at the same time
/// and the turn player orders them (§15-4-3-5-1). P0's `RAID_WA` (<Raid>,
/// "[When Attacking] Gain 1 memory") attacks P1: <Raid> and the [When
/// Attacking] effect pend in one `TriggerOrder`; P0 activates <Raid> first,
/// so its choice is offered while the memory gain is still pending.
///
/// DCGO: `CardEffectFactory/KeyWordEffects/Raid.cs` builds <Raid> as an
/// optional `ActivateClass` on `OnAllyAttack`, stacked with the [When
/// Attacking] effects in one `StackSkillInfos` batch (`AttackProcess.cs`).
#[test]
fn raid_kind_matches_rule() {
    cite!(
        "16-23-2",
        "4522f354b5a17616ecdab28334b9b6b8f785d5453bc43f20ea49449a060d6657"
    );
    let mut r = raid_board();
    let attacker = r.place_on_field(0, "RAID_WA", Some(0));
    let cp = r.event_checkpoint();
    r.attack_player(attacker, 1, false);
    let labels = order_choices(&r, 0);
    assert_eq!(
        labels.len(),
        2,
        "<Raid> and [When Attacking] pend together: {labels:?}"
    );
    assert!(memory_sources(&r, cp).is_empty(), "nothing resolved yet");
}

/// §11-2-7-1 "When a player makes an attack declaration, they choose either
/// the opponent or 1 of the opponent's suspended Digimon as the attack
/// target."
/// §16-23-1 "<Raid> is a keyword effect that can SWITCH the target of attack
/// to the opponent's unsuspended Digimon with the highest DP when a Digimon
/// with this keyword effect attacks."
///
/// <Raid> switches the target after the declaration; it is not a permission
/// to declare on an unsuspended Digimon (contrast <Execute>, §16-38-1).
/// - P1 has only unsuspended Digimon (`raid_board`): P0's `RAID` may declare
///   its attack on P1 and on none of them, the highest-DP ones included.
///   Declared on P1, <Raid> then offers the optional switch to the two
///   highest-DP Digimon; P0 declines and the attack checks security.
/// - P1 also has a suspended `FILL` (slot 3): `RAID` may declare on it (and
///   still not on the unsuspended ones); <Raid> then offers the same switch;
///   P0 switches to slot 0, which battles and is deleted, and the suspended
///   `FILL` is untouched.
///
/// DCGO: `Permanent.CanAttackTargetDigimon` has no <Raid> arm (a Digimon
/// target is suspended, or the attacker may attack unsuspended Digimon);
/// `KeyWordEffects/Raid.cs` switches the defender on `OnAllyAttack`.
#[test]
fn raid_attacker_declares_on_a_legal_target_then_may_switch() {
    cite!(
        "16-23-1",
        "baa73e73e69753c0ecf0648f0d2bf6a59b8ef1da68985c91e493a96caa491d4a"
    );
    cite!(
        "11-2-7-1",
        "3a19c7490ca53f977f6a6cee04e7633ba2b90a9877d2cc013604793ac461d9d8"
    );
    // Only unsuspended Digimon: the opponent is the only legal target.
    let mut r = raid_board();
    let attacker = r.place_on_field(0, "RAID", Some(0));
    let a = attacker.index as u16;
    assert!(legal(&r, 0, encode_attack(a, SECURITY_TARGET)), "the opponent");
    for slot in 0..3 {
        assert!(
            !legal(&r, 0, encode_attack(a, slot)),
            "not the unsuspended Digimon in slot {slot}"
        );
    }
    let cp = r.event_checkpoint();
    r.game.decode_action(encode_attack(a, SECURITY_TARGET), 0);
    let p = current_prompt(&r).expect("the <Raid> switch after the declaration");
    assert_eq!(p.player, 0, "{p:?}");
    assert_eq!(p.kind, SelectionKind::OppField, "{p:?}");
    assert!(p.optional, "{p:?}");
    assert_eq!(offered_slots(&p), vec![0, 2], "the tied highest-DP Digimon");
    r.decline_optional_trigger().expect("decline <Raid>");
    settle_declining(&mut r);
    assert_eq!(revealed(&r, cp, 1), owned(&[FILL]), "the attack stayed on P1");

    // A suspended Digimon is a legal declaration target too.
    let mut r = raid_board();
    let attacker = r.place_on_field(0, "RAID", Some(0));
    let a = attacker.index as u16;
    let suspended = place_suspended(&mut r, 1, FILL);
    let s = suspended.index as u16;
    assert!(legal(&r, 0, encode_attack(a, s)), "the suspended Digimon");
    for slot in 0..3 {
        assert!(
            !legal(&r, 0, encode_attack(a, slot)),
            "not the unsuspended Digimon in slot {slot}"
        );
    }
    let cp = r.event_checkpoint();
    r.game.decode_action(encode_attack(a, s), 0);
    let p = current_prompt(&r).expect("the <Raid> switch after the declaration");
    assert_eq!(p.kind, SelectionKind::OppField, "{p:?}");
    assert!(p.optional, "{p:?}");
    assert_eq!(offered_slots(&p), vec![0, 2], "the tied highest-DP Digimon");
    assert!(
        r.game.player(1).battle_area[s as usize].is_suspended,
        "the declared target is still suspended (the switch offers UNSUSPENDED Digimon only)"
    );
    pick_field(&mut r, 0);
    settle_declining(&mut r);
    assert_eq!(
        field_top_ids(&r, 1),
        owned(&[FILL, "LV4_PLAIN", FILL]),
        "the switched-to Digimon battled and was deleted; the suspended FILL was not"
    );
    assert!(
        r.game.player(1).battle_area[2].is_suspended,
        "the suspended FILL (now slot 2) is untouched"
    );
    assert!(revealed(&r, cp, 1).is_empty(), "no security check");
}

// ─── §16-24 <Alliance> ──────────────────────────────────────────────────────

/// P0 has `ALLIANCE` (5000 DP, slot 0) and `LV4_PLAIN` (5000 DP, slot 1).
fn alliance_board(sec1: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = board(sec1);
    let a = r.place_on_field(0, "ALLIANCE", Some(0));
    r.place_on_field(0, "LV4_PLAIN", Some(0));
    (r, a)
}

/// The `<Alliance>` ally choice (P0's `OwnField` pick in `AllianceTiming`),
/// or `None`.
fn alliance_prompt(r: &DebugRunner) -> Option<Prompt> {
    current_prompt(r).filter(|p| p.player == 0 && p.kind == SelectionKind::OwnField)
}

/// §16-24-3 ""By suspending 1 of your other Digimon" in <Alliance> is an
/// optional processing condition. If the content of that condition is
/// executed, the processing for "... add the suspended Digimon's DP to the
/// attacking Digimon and it gains <Security A. +1> for the attack" will be
/// mandatory."
///
/// P0's `ALLIANCE` attacks P1 (top security card `LV5_PLAIN`, 7000 DP).
/// - P0 declines: no suspension; `ALLIANCE` (5000 DP) checks 1 card, loses
///   to `LV5_PLAIN` and is deleted.
/// - P0 suspends `LV4_PLAIN`: with no further decision `ALLIANCE` has
///   10000 DP and checks 2 cards (`LV5_PLAIN`, then `FILL`).
#[test]
fn alliance_optionality_matches_rule() {
    cite!(
        "16-24-3",
        "0b93558ff1c03e2665b12ab50fe46ad9db57d681f92f90764e44cf5cfdf78116"
    );
    let (mut r, a) = alliance_board(&[FILL, FILL, "LV5_PLAIN"]);
    let cp = r.event_checkpoint();
    r.attack_player(a, 1, false);
    let p = alliance_prompt(&r).expect("the <Alliance> condition");
    assert!(p.optional, "{p:?}");
    r.decline_optional_trigger().expect("decline");
    settle_declining(&mut r);
    assert_eq!(revealed(&r, cp, 1), owned(&["LV5_PLAIN"]));
    assert_eq!(field_top_ids(&r, 0), owned(&["LV4_PLAIN"]), "ALLIANCE lost");
    assert!(
        !is_suspended(&r, r.perm_handle(0, 0)),
        "declined: no suspension"
    );

    let (mut r, a) = alliance_board(&[FILL, FILL, "LV5_PLAIN"]);
    let cp = r.event_checkpoint();
    r.attack_player(a, 1, false);
    assert!(alliance_prompt(&r).is_some());
    pick_field(&mut r, 1);
    assert!(
        current_prompt(&r).is_none(),
        "the rest is mandatory: {:?}",
        current_prompt(&r)
    );
    assert_eq!(revealed(&r, cp, 1), owned(&["LV5_PLAIN", FILL]));
    assert_eq!(field_top_ids(&r, 0), owned(&["ALLIANCE", "LV4_PLAIN"]));
}

/// §16-24-1 "<Alliance> is a keyword effect. When a Digimon with this effect
/// attacks, by suspending 1 of your OTHER Digimon, add the suspended
/// Digimon's DP to the attacking Digimon and it gains <Security A. +1> FOR
/// THE ATTACK."
///
/// - P0's `LV5_PLAIN` attacks while `ALLIANCE` sits on P0's field: no
///   <Alliance> condition is offered.
/// - P0's `ALLIANCE` attacks with an unsuspended `LV4_PLAIN` and a suspended
///   `FILL` beside it: only `LV4_PLAIN` (an other, unsuspended Digimon) is
///   offered. After the attack `ALLIANCE` is back to 5000 DP.
#[test]
fn alliance_timing_matches_rule() {
    cite!(
        "16-24-1",
        "b667bf9355e9d6bcd1f3c1f0eabb7421f01f8e95577cc4910f062d2b06ac23be"
    );
    let (mut r, _) = alliance_board(&[FILL; 3]);
    let other = r.place_on_field(0, "LV5_PLAIN", Some(0));
    r.attack_player(other, 1, false);
    let prompts = settle_declining(&mut r);
    assert!(prompts.is_empty(), "another Digimon's attack: {prompts:?}");

    let (mut r, a) = alliance_board(&[FILL; 3]);
    place_suspended(&mut r, 0, FILL);
    r.attack_player(a, 1, false);
    let p = alliance_prompt(&r).expect("the <Alliance> condition");
    assert_eq!(
        offered_slots(&p),
        vec![1],
        "only the other unsuspended Digimon"
    );
    pick_field(&mut r, 1);
    settle_declining(&mut r);
    assert_eq!(
        r.effective_dp(a),
        Some(5000),
        "the DP was added for the attack"
    );
}

/// §16-24-2 "<Alliance> is a TRIGGER-TYPE effect that triggers when a
/// Digimon with this effect attacks."
///
/// P0's `WA_ALLIANCE` (<Alliance>, "[When Attacking] Gain 1 memory") attacks
/// P1 with an unsuspended `LV4_PLAIN` beside it: <Alliance> and the [When
/// Attacking] effect pend in one `TriggerOrder` before either activates.
#[test]
fn alliance_kind_matches_rule() {
    cite!(
        "16-24-2",
        "bd670d479e92a9d73ac4a88f81b37040639fd11848da35490e846b59ffb112dc"
    );
    let mut r = board(&[FILL; 3]);
    let a = r.place_on_field(0, "WA_ALLIANCE", Some(0));
    r.place_on_field(0, "LV4_PLAIN", Some(0));
    let cp = r.event_checkpoint();
    r.attack_player(a, 1, false);
    let labels = order_choices(&r, 0);
    assert_eq!(labels.len(), 2, "both pend together: {labels:?}");
    assert!(memory_sources(&r, cp).is_empty(), "nothing resolved yet");
}

/// §16-24-4 "The value added to DP by <Alliance> is the value of the
/// suspended Digimon upon the suspension. The added DP from <Alliance> isn't
/// modified, even if the DP of your suspended Digimon is later modified."
/// §16-24-5 "Even if your Digimon that was suspended by <Alliance> is removed
/// from the battle area during the attack, the added DP and <Security A. +1>
/// remain."
///
/// P0's `ALLIANCE` attacks P1 (top security card `LV5_PLAIN`, 7000 DP) and
/// suspends `LV4_PLAIN` (+5000 DP, <Security A. +1>). At P1's counter timing:
/// - `COUNTER_MINUS3000` gives `LV4_PLAIN` -3000 DP: `ALLIANCE` keeps its
///   10000 DP and checks 2 cards, beating `LV5_PLAIN`.
/// - `COUNTER_DEL5K` deletes `LV4_PLAIN`: `ALLIANCE` keeps its 10000 DP and
///   <Security A. +1>; it checks `LV5_PLAIN` and `FILL` and survives.
#[test]
fn alliance_dp_and_sa_persist_after_suspended_digimon_leaves() {
    cite!(
        "16-24-4",
        "6a5968c2d2928378f66eb7eae158cb189c9a75165a2a1100aa2ec4e4b772fd5b"
    );
    cite!(
        "16-24-5",
        "ac3fb6060b90e5050cf1eec195341da2cd920671e100b8a1b864e7d43108ffbc"
    );
    for counter in ["COUNTER_MINUS3000", "COUNTER_DEL5K"] {
        let (mut r, a) = alliance_board(&[FILL, FILL, "LV5_PLAIN"]);
        r.place_on_field(1, counter, Some(0));
        let cp = r.event_checkpoint();
        r.attack_player(a, 1, false);
        assert!(alliance_prompt(&r).is_some(), "{counter}");
        pick_field(&mut r, 1);
        let p = current_prompt(&r).expect("the counter timing");
        assert_eq!(p.phase, GamePhase::CounterTiming, "{counter}: {p:?}");
        step_prompt(&mut r); // activate the [Counter] effect
        let p = current_prompt(&r).expect("the [Counter] effect's target");
        assert_eq!(p.player, 1, "{counter}: {p:?}");
        pick_field(&mut r, 1); // the Digimon <Alliance> suspended
        settle_declining(&mut r);
        assert_eq!(
            revealed(&r, cp, 1),
            owned(&["LV5_PLAIN", FILL]),
            "{counter}: the added DP and <Security A. +1> remain"
        );
        assert_eq!(field_top_ids(&r, 0)[0], "ALLIANCE", "{counter}: survived");
    }
}

// ─── §16-30 <Collision> ─────────────────────────────────────────────────────

/// §16-30-1 "<Collision> is a keyword effect. While a Digimon with this
/// effect is attacking, all of your opponent's Digimon gain <Blocker> and the
/// opponent player is forced to block whenever possible during the block
/// timing."
///
/// P0's `COLLISION` (7000 DP) attacks P1.
/// - P1 has an unsuspended `FILL` and `LV4_PLAIN` (no <Blocker>): the block
///   window offers both and can't be declined; P1 blocks with `FILL`, which
///   battles and is deleted; nothing checks security.
/// - P1's only Digimon is `BLOCKER_CANNOT_SUSPEND` (can't suspend, so it
///   can't block, §12-1-4): no block is forced and the attack checks
///   security.
/// - Control: P0's `LV5_PLAIN` (no <Collision>) attacks P1 with an
///   unsuspended `FILL`: no block window.
#[test]
fn collision_forces_block_whenever_possible() {
    cite!(
        "16-30-1",
        "b98f2cdcdfd77f46069943046dfa3ee618912ff173dfbd42fdc5a6f56c577631"
    );
    let mut r = board(&[FILL; 3]);
    let attacker = r.place_on_field(0, "COLLISION", Some(0));
    r.place_on_field(1, FILL, Some(0));
    r.place_on_field(1, "LV4_PLAIN", Some(0));
    let cp = r.event_checkpoint();
    r.attack_player(attacker, 1, false);
    let p = current_prompt(&r).expect("the block window");
    assert_eq!(p.phase, GamePhase::BlockTiming);
    assert_eq!(p.player, 1);
    assert!(!p.optional, "the block is forced: {p:?}");
    assert_eq!(offered_slots(&p), vec![0, 1]);
    pick_field(&mut r, 0);
    settle_declining(&mut r);
    assert_eq!(field_top_ids(&r, 1), owned(&["LV4_PLAIN"]), "FILL blocked");
    assert!(revealed(&r, cp, 1).is_empty());

    let mut r = board(&[FILL; 3]);
    let attacker = r.place_on_field(0, "COLLISION", Some(0));
    r.place_on_field(1, "BLOCKER_CANNOT_SUSPEND", Some(0));
    let cp = r.event_checkpoint();
    r.attack_player(attacker, 1, false);
    let prompts = settle_declining(&mut r);
    assert!(prompts.is_empty(), "no block possible: {prompts:?}");
    assert_eq!(revealed(&r, cp, 1), owned(&[FILL]));

    let mut r = board(&[FILL; 3]);
    let attacker = r.place_on_field(0, "LV5_PLAIN", Some(0));
    r.place_on_field(1, FILL, Some(0));
    r.attack_player(attacker, 1, false);
    let prompts = settle_declining(&mut r);
    assert!(prompts.is_empty(), "control: {prompts:?}");
}

/// §16-30-2 "<Collision> is a persistent effect."
/// §16-30-1 "... all of your opponent's Digimon GAIN <Blocker> ..."
///
/// P0's `COLLISION` attacks P1, who has an unsuspended `FILL` (no text).
/// Before the attack `FILL` has no <Blocker>; at the block timing (while
/// `COLLISION` is attacking) it has <Blocker>, applied with no activation.
#[test]
fn collision_kind_matches_rule() {
    cite!(
        "16-30-2",
        "1511ba084735ea3cd9f9de5d009337204841f504177a579b5b3481287a254f7e"
    );
    cite!(
        "16-30-1",
        "b98f2cdcdfd77f46069943046dfa3ee618912ff173dfbd42fdc5a6f56c577631"
    );
    let mut r = board(&[FILL; 3]);
    let attacker = r.place_on_field(0, "COLLISION", Some(0));
    let fill = r.place_on_field(1, FILL, Some(0));
    assert!(
        !r.game.has_keyword(fill, Keyword::Blocker),
        "before the attack"
    );
    r.attack_player(attacker, 1, false);
    let p = current_prompt(&r).expect("the block window");
    assert_eq!(p.phase, GamePhase::BlockTiming, "the first decision: {p:?}");
    assert!(
        r.game.has_keyword(fill, Keyword::Blocker),
        "while COLLISION is attacking, P1's Digimon have <Blocker>"
    );
}

/// §16-30-3 "<Collision> will activate while the Digimon with this effect is
/// attacking."
///
/// P0 has `COLLISION` on its field; its `LV5_PLAIN` (no <Collision>) attacks
/// P1, who has an unsuspended `FILL`: no block is forced (no block window).
#[test]
fn collision_timing_matches_rule() {
    cite!(
        "16-30-3",
        "6dfefaa855810c1ae85b6a3cf229940b6668d01743f27c1e777503c2f852ab5e"
    );
    let mut r = board(&[FILL; 3]);
    r.place_on_field(0, "COLLISION", Some(0));
    let attacker = r.place_on_field(0, "LV5_PLAIN", Some(0));
    r.place_on_field(1, FILL, Some(0));
    let cp = r.event_checkpoint();
    r.attack_player(attacker, 1, false);
    let prompts = settle_declining(&mut r);
    assert!(prompts.is_empty(), "{prompts:?}");
    assert_eq!(revealed(&r, cp, 1), owned(&[FILL]));
}

/// §16-30-4 ""All of your opponent's Digimon gain <Blocker>" in <Collision>
/// is processing that affects Digimon. "The opponent player is forced to
/// block whenever possible" is processing that affects the opponent player."
///
/// P1's `IMMUNE` ("[All Turns] This Digimon isn't affected by your
/// opponent's effects") is not affected by `COLLISION`'s <Blocker> grant.
/// - P1 has `IMMUNE` (slot 0) and `FILL` (slot 1): the forced block window
///   offers `FILL` only.
/// - P1's only Digimon is `IMMUNE`: it has no <Blocker>, so no block is
///   possible and the attack checks security.
///
/// DCGO: `Permanent.HasBlocker` grants <Blocker> during a <Collision> attack
/// only `if (!TopCard.CanNotBeAffected(fakeCollisionClass))`.
#[test]
fn collision_blocker_grant_affects_digimon_forced_block_affects_player() {
    cite!(
        "16-30-4",
        "94657e983f0ec20140f6912a2ad374c65d16a7c9ad897b176a39ee14129157fe"
    );
    let mut r = board(&[FILL; 3]);
    let attacker = r.place_on_field(0, "COLLISION", Some(0));
    r.place_on_field(1, "IMMUNE", Some(0));
    r.place_on_field(1, FILL, Some(0));
    r.attack_player(attacker, 1, false);
    let p = current_prompt(&r).expect("the block window");
    assert_eq!(p.phase, GamePhase::BlockTiming);
    assert!(!p.optional);
    assert_eq!(offered_slots(&p), vec![1], "IMMUNE does not gain <Blocker>");

    let mut r = board(&[FILL; 3]);
    let attacker = r.place_on_field(0, "COLLISION", Some(0));
    r.place_on_field(1, "IMMUNE", Some(0));
    let cp = r.event_checkpoint();
    r.attack_player(attacker, 1, false);
    let prompts = settle_declining(&mut r);
    assert!(prompts.is_empty(), "no Digimon with <Blocker>: {prompts:?}");
    assert_eq!(revealed(&r, cp, 1), owned(&[FILL]));
}

// ─── §16-33 <Vortex> ────────────────────────────────────────────────────────

/// P0 has `VORTEX` (5000 DP), PLAYED THIS TURN, plus `extra`; P1 has a
/// suspended `FILL` (slot 0).
fn vortex_board(extra: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = board(&[FILL; 3]);
    let v = r.place_on_field(0, "VORTEX", None);
    for id in extra {
        r.place_on_field(0, id, Some(0));
    }
    place_suspended(&mut r, 1, FILL);
    (r, v)
}

/// §16-33-1 "<Vortex> allows the Digimon with this effect to attack an
/// opponent's Digimon at the end of your turn. <Vortex> is a keyword effect
/// that also allows a Digimon to attack in the same turn it was played."
/// §16-33-3 "The processing from <Vortex> is optional."
///
/// `VORTEX` was played this turn: in P0's main phase it can't attack. At the
/// end of P0's turn it may attack P1's suspended `FILL` (not P1 itself), or
/// P0 may decline. It attacks: `FILL` battles and is deleted, then the turn
/// passes to P1. (§16-33-2: <Vortex> is the only `[End of Your Turn]` effect,
/// so its attack decision is offered as soon as P0 ends the turn.)
#[test]
fn vortex_optional_eot_attack_on_digimon_and_same_turn_played() {
    cite!(
        "16-33-1",
        "0cc680ee2a3fb25134d4e8df2ef558b151c863305f9393912a8b68a1eef1b427"
    );
    cite!(
        "16-33-3",
        "5b2d7bf75281743817524e6703812bf0247df1956f34e6386b2aabec7d60cb7a"
    );
    let (mut r, v) = vortex_board(&[]);
    assert!(
        !legal(&r, 0, encode_attack(v.index as u16, 0)),
        "main phase: played this turn"
    );
    r.end_turn();
    assert!(eot_attack_offered(&r, v, 0), "end of turn: <Vortex> attack");
    assert!(
        !eot_attack_offered(&r, v, SECURITY_TARGET),
        "an opponent's Digimon only"
    );
    assert!(legal(&r, 0, PASS));
    r.game.decode_action(encode_attack(v.index as u16, 0), 0);
    settle_declining(&mut r);
    assert!(
        field_top_ids(&r, 1).is_empty(),
        "FILL battled and was deleted"
    );
    finish_end_of_turn(&mut r);
    assert_eq!(r.turn_player(), 1);
}

/// §16-33-3 "The processing from <Vortex> is optional."
///
/// At the end of P0's turn P0 declines the <Vortex> attack: no battle, and
/// the turn passes to P1. (§16-33-2: the decline of <Vortex>'s own decision
/// in the `[End of Your Turn]` pool, not a PASS in a later window.)
#[test]
fn vortex_optionality_matches_rule() {
    cite!(
        "16-33-3",
        "5b2d7bf75281743817524e6703812bf0247df1956f34e6386b2aabec7d60cb7a"
    );
    let (mut r, v) = vortex_board(&[]);
    r.end_turn();
    assert!(eot_attack_offered(&r, v, 0));
    let p = current_prompt(&r).expect("<Vortex>'s decision");
    assert!(p.optional, "{p:?}");
    r.decline_optional_trigger().expect("decline <Vortex>");
    settle_declining(&mut r);
    assert_eq!(field_top_ids(&r, 1), owned(&[FILL]), "declined");
    assert_eq!(r.turn_player(), 1);
}

/// §16-33-2 "<Vortex> is a trigger-type effect that triggers at the end of
/// YOUR turn."
///
/// P0 passes its turn; at the end of P1's turn, P0's `VORTEX` does not
/// trigger: no attack is offered and the turn passes back to P0.
#[test]
fn vortex_timing_matches_rule() {
    cite!(
        "16-33-2",
        "9ac3af1732d2fd1a4588d9b79e7717d9d4c870c92614e7be23d7a8f2b05d7ca1"
    );
    let (mut r, _) = vortex_board(&[]);
    through_p1_end_of_turn(&mut r);
    assert_ne!(r.current_phase(), GamePhase::EndOfTurnAction);
    assert_eq!(r.turn_player(), 0, "the turn passed back to P0");
    assert_eq!(field_top_ids(&r, 1), owned(&[FILL]));
}

/// §16-33-2 "<Vortex> is a TRIGGER-TYPE effect that triggers at the end of
/// your turn."
///
/// P0 has `VORTEX` and `EOT_MEM` ("[End of Your Turn] Gain 2 memory"). At
/// the end of P0's turn both pend in one `TriggerOrder`; P0 activates
/// <Vortex> first: its attack is offered while `EOT_MEM` is still pending,
/// and `EOT_MEM` resolves only after the attack declaration.
///
/// Official Q&A Q3868 / Q3869 (EX7-064, <Vortex>): <Vortex> "triggers
/// simultaneously with [End of Your Turn] effects, therefore you can choose
/// the activation order"; activating <Vortex> first, "you can activate
/// <Vortex> and make an attack declaration, [the other] effect that triggered
/// simultaneously will activate, then the counter timing will occur." So the
/// still-pending effect resolves AFTER the attack declaration and BEFORE the
/// counter timing / battle; the test pins declaration < `EOT_MEM` < battle.
#[test]
fn vortex_kind_matches_rule() {
    cite!(
        "16-33-2",
        "9ac3af1732d2fd1a4588d9b79e7717d9d4c870c92614e7be23d7a8f2b05d7ca1"
    );
    let (mut r, v) = vortex_board(&["EOT_MEM"]);
    let cp = r.event_checkpoint();
    r.end_turn();
    let mut cards: Vec<String> = order_choices(&r, 0)
        .iter()
        .map(|l| branch_card(l).to_string())
        .collect();
    cards.sort();
    assert_eq!(cards, owned(&["EOT_MEM", "VORTEX"]), "both pend together");
    choose(&mut r, "VORTEX");
    assert!(memory_sources(&r, cp).is_empty(), "EOT_MEM still pending");
    assert!(eot_attack_offered(&r, v, 0));
    declare_eot_attack(&mut r, v, 0);
    settle_declining(&mut r);
    let declared = attack_event_index(&r, cp).expect("the <Vortex> attack was declared");
    let gain = memory_event_index(&r, cp, "EOT_MEM").expect("EOT_MEM resolved");
    let battle = trash_event_index(&r, cp, 1, FILL).expect("FILL deleted in battle");
    assert!(
        declared < gain && gain < battle,
        "declaration (#{declared}) < EOT_MEM (#{gain}) < battle (#{battle})"
    );
}

/// §16-33-1 "<Vortex> allows the Digimon with this effect to attack an
/// opponent's Digimon at the end of your turn."
/// §11-2-7-1 "When a player makes an attack declaration, they choose either
/// the opponent or 1 of the opponent's SUSPENDED Digimon as the attack
/// target."
///
/// <Vortex> grants the attack, not a new target rule (contrast <Execute>,
/// §16-38-1, which states "also allows for attacking an opponent's
/// unsuspended Digimon"). At the end of P0's turn `VORTEX` may attack P1's
/// suspended `FILL` (slot 0) but not P1's unsuspended `LV4_PLAIN` (slot 1).
///
/// DCGO: `VortexProcess` runs `SelectAttackEffect` with `defenderCondition: _
/// => true`, but `Permanent.CanAttackTargetDigimon` still requires
/// `Defender.IsSuspended || isExecute`.
#[test]
fn vortex_attack_target_is_a_suspended_digimon() {
    cite!(
        "16-33-1",
        "0cc680ee2a3fb25134d4e8df2ef558b151c863305f9393912a8b68a1eef1b427"
    );
    cite!(
        "11-2-7-1",
        "3a19c7490ca53f977f6a6cee04e7633ba2b90a9877d2cc013604793ac461d9d8"
    );
    let (mut r, v) = vortex_board(&[]);
    r.place_on_field(1, "LV4_PLAIN", Some(0));
    r.end_turn();
    assert!(eot_attack_offered(&r, v, 0), "the suspended Digimon");
    assert!(!eot_attack_offered(&r, v, 1), "not the unsuspended Digimon");
}

/// §16-33-1 "<Vortex> allows the Digimon with this effect to ATTACK an
/// opponent's Digimon at the end of your turn."
/// §11-1-5 "Once an attack declaration has been made, all of the timings that
/// follow will occur."
///
/// A <Vortex> attack is an attack: P1's counter timing and block timing
/// follow it. P1 has an unsuspended `BLOCKER`; `VORTEX` attacks P1's
/// suspended `FILL` at the end of P0's turn: P1 is offered the block, blocks,
/// and `BLOCKER` battles instead of `FILL`.
///
/// §11-4-1 "This is the timing when a non-turn player who is being attacked
/// can use their Digimon with <Blocker> to perform a block."
///
/// Official Q&A Q3869 (EX7-064): after a <Vortex> attack declaration "the
/// counter timing will occur". DCGO: `SelectAttackEffect.SetIsVortex()` only
/// relaxes `CanAttack`'s played-this-turn check; the attack runs the normal
/// `AttackProcess` (`BlockTiming`).
#[test]
fn vortex_attack_has_block_timing() {
    cite!(
        "16-33-1",
        "0cc680ee2a3fb25134d4e8df2ef558b151c863305f9393912a8b68a1eef1b427"
    );
    cite!(
        "11-1-5",
        "cd109077bced1cb39983cddfc3851c42da90acec2ee7c54922c7d0df6542d1c9"
    );
    cite!(
        "11-4-1",
        "922aa181a790d6caa663a5a8fbc1bf38f95b19960de5d80ca4ef38437043007f"
    );
    let (mut r, v) = vortex_board(&[]);
    r.place_on_field(1, "BLOCKER", Some(0));
    r.end_turn();
    assert!(eot_attack_offered(&r, v, 0));
    r.game.decode_action(encode_attack(v.index as u16, 0), 0);
    let p = current_prompt(&r).expect("the block window");
    assert_eq!(p.phase, GamePhase::BlockTiming, "{p:?}");
    assert_eq!(offered_slots(&p), vec![1]);
    pick_field(&mut r, 1);
    settle_declining(&mut r);
    assert_eq!(
        field_top_ids(&r, 1),
        owned(&[FILL]),
        "BLOCKER battled instead"
    );
}

// ─── §16-34 <Overclock> ─────────────────────────────────────────────────────

/// P0 has `OVERCLOCK` (6000 DP, slot 0), `PUPPET` ([Puppet], slot 1) and
/// `FILL` (no trait, slot 2), plus `extra`; P1 has 3 security cards.
fn overclock_board(extra: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = board(&[FILL; 3]);
    let oc = r.place_on_field(0, "OVERCLOCK", Some(0));
    r.place_on_field(0, "PUPPET", Some(0));
    r.place_on_field(0, FILL, Some(0));
    for id in extra {
        r.place_on_field(0, id, Some(0));
    }
    (r, oc)
}

/// The former `EndOfTurnAction` field-effect bit for `<Overclock>`. Since
/// 2026-10-18 (§16-34-2) <Overclock> is offered only inside the `[End of Your
/// Turn]` pool, as its condition's pick; this bit is never legal.
fn overclock_action(oc: PermanentHandle) -> u16 {
    FIELD_EFFECT_START + oc.index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_OVERCLOCK
}

/// §16-34-1 "<Overclock> is a keyword effect. At the end of your turn, by
/// deleting 1 of your Tokens or 1 of your other specified Digimon, the
/// Digimon with this effect attacks a player without suspending."
/// §16-34-3 ""By deleting 1 of your Tokens or 1 of your other specified
/// Digimon" in <Overclock> is an optional processing condition. If the
/// content of that condition is executed, the processing for "the Digimon
/// with this effect attacks a player without suspending" will be mandatory."
///
/// At the end of P0's turn, <Overclock ([Puppet] trait)> offers its
/// condition: delete `PUPPET` (the [Puppet] Digimon; not `FILL`, not
/// `OVERCLOCK` itself). P0 deletes `PUPPET`: with no further decision
/// `OVERCLOCK` attacks P1 (a security check), and stays unsuspended.
/// (§16-34-2: <Overclock> is the only `[End of Your Turn]` effect, so its
/// condition is offered as soon as P0 ends the turn.)
#[test]
fn overclock_optional_cost_then_mandatory_attack_without_suspending() {
    cite!(
        "16-34-1",
        "23103e747d5791f23140bf743da41c19d38219f621b62ceb4fe5105146b0018e"
    );
    cite!(
        "16-34-3",
        "3b555b19ab0a61d1f67e101c4efabf15414ca10ee988576313a8c36c33aa9fd9"
    );
    let (mut r, oc) = overclock_board(&[]);
    let cp = r.event_checkpoint();
    r.end_turn();
    assert!(!legal(&r, 0, overclock_action(oc)), "no separate activation");
    let p = current_prompt(&r).expect("the condition's pick");
    assert_eq!(p.player, 0, "{p:?}");
    assert!(p.optional, "{p:?}");
    assert_eq!(offered_slots(&p), vec![1], "only the [Puppet] Digimon");
    pick_field(&mut r, 1);
    let prompts = settle_declining(&mut r);
    assert!(prompts.is_empty(), "the attack is mandatory: {prompts:?}");
    assert!(
        trash_ids(&r, 0).contains(&"PUPPET".to_string()),
        "PUPPET deleted"
    );
    assert_eq!(revealed(&r, cp, 1), owned(&[FILL]), "OVERCLOCK attacked P1");
    assert!(!is_suspended(&r, oc), "without suspending");
}

/// §16-34-3 ""By deleting 1 of your Tokens or 1 of your other specified
/// Digimon" in <Overclock> is an OPTIONAL processing condition."
///
/// At the end of P0's turn P0 declines the condition: nothing is deleted, no
/// attack, and the turn passes to P1. (§16-34-2: the condition is offered
/// inside the `[End of Your Turn]` pool.)
#[test]
fn overclock_optionality_matches_rule() {
    cite!(
        "16-34-3",
        "3b555b19ab0a61d1f67e101c4efabf15414ca10ee988576313a8c36c33aa9fd9"
    );
    let (mut r, _oc) = overclock_board(&[]);
    let cp = r.event_checkpoint();
    r.end_turn();
    let p = current_prompt(&r).expect("the condition's pick");
    assert!(p.optional, "{p:?}");
    r.decline_optional_trigger().expect("decline the condition");
    settle_declining(&mut r);
    if r.current_phase() == GamePhase::EndOfTurnAction {
        r.game.decode_action(PASS, 0);
        settle_declining(&mut r);
    }
    assert_eq!(field_top_ids(&r, 0), owned(&["OVERCLOCK", "PUPPET", FILL]));
    assert!(revealed(&r, cp, 1).is_empty(), "no attack");
    assert_eq!(r.turn_player(), 1);
}

/// §16-34-2 "<Overclock> is a trigger-type effect that triggers at the end of
/// YOUR turn."
///
/// In P0's main phase <Overclock> is not offered; P0 passes its turn
/// (declining <Overclock>); at the end of P1's turn P0's `OVERCLOCK` does
/// not trigger and the turn passes back to P0 with `PUPPET` still on P0's
/// field.
#[test]
fn overclock_timing_matches_rule() {
    cite!(
        "16-34-2",
        "d2abce098c733349c81dcc728df5453f32a89e52a3085a72af19a83d50e7d4aa"
    );
    let (mut r, oc) = overclock_board(&[]);
    assert!(!legal(&r, 0, overclock_action(oc)), "main phase");
    through_p1_end_of_turn(&mut r);
    assert_eq!(r.turn_player(), 0);
    assert_eq!(field_top_ids(&r, 0), owned(&["OVERCLOCK", "PUPPET", FILL]));
}

/// §16-34-2 "<Overclock> is a TRIGGER-TYPE effect that triggers at the end of
/// your turn."
///
/// P0 has `OVERCLOCK` and `EOT_MEM` ("[End of Your Turn] Gain 2 memory"). At
/// the end of P0's turn both pend in one `TriggerOrder`; P0 activates
/// <Overclock> first: its condition is offered while `EOT_MEM` is still
/// pending, and `EOT_MEM` resolves only after the attack declaration.
///
/// Official Q&A Q3868 / Q3869 (EX7-064, <Vortex>): <Vortex> "triggers
/// simultaneously with [End of Your Turn] effects, therefore you can choose
/// the activation order"; activating <Vortex> first, "you can activate
/// <Vortex> and make an attack declaration, [the other] effect that triggered
/// simultaneously will activate, then the counter timing will occur." So the
/// still-pending effect resolves AFTER the attack declaration and BEFORE the
/// counter timing / battle; the test pins declaration < `EOT_MEM` < security check (the same simultaneous-trigger order the rulings give for
/// <Vortex>).
#[test]
fn overclock_kind_matches_rule() {
    cite!(
        "16-34-2",
        "d2abce098c733349c81dcc728df5453f32a89e52a3085a72af19a83d50e7d4aa"
    );
    let (mut r, _) = overclock_board(&["EOT_MEM"]);
    let cp = r.event_checkpoint();
    r.end_turn();
    let mut cards: Vec<String> = order_choices(&r, 0)
        .iter()
        .map(|l| branch_card(l).to_string())
        .collect();
    cards.sort();
    assert_eq!(
        cards,
        owned(&["EOT_MEM", "OVERCLOCK"]),
        "both pend together"
    );
    choose(&mut r, "OVERCLOCK");
    assert!(memory_sources(&r, cp).is_empty(), "EOT_MEM still pending");
    pick_field(&mut r, 1);
    settle_declining(&mut r);
    let declared = attack_event_index(&r, cp).expect("the <Overclock> attack was declared");
    let gain = memory_event_index(&r, cp, "EOT_MEM").expect("EOT_MEM resolved");
    let check = first_reveal(&r, cp).expect("the <Overclock> attack checked security");
    assert!(
        declared < gain && gain < check,
        "declaration (#{declared}) < EOT_MEM (#{gain}) < security check (#{check})"
    );
}

// ─── §16-35 <Iceclad> ───────────────────────────────────────────────────────

/// Attack P1's suspended stack `defender` (bottom first) with P0's stack
/// `attacker`; returns the board after the attack resolved.
fn iceclad_battle(attacker: &[&str], defender: &[&str]) -> DebugRunner {
    let mut r = board(&[FILL; 3]);
    let a = r.place_stack(0, attacker);
    let d = r.place_stack(1, defender);
    set_suspended(&mut r, d, true);
    r.attack_digimon(a, d, false);
    settle_declining(&mut r);
    r
}

/// §16-35-1 "<Iceclad> is a keyword effect where you compare the number of
/// digivolution cards instead of DP when you battle using the Digimon with
/// this effect, other than battles against Security Digimon."
/// §16-35-3 "<Iceclad> is an effect that changes the rules of a battle."
/// §16-35-4 "When using <Iceclad> to compare the number of digivolution
/// cards for two cards in a battle, the winner and loser of the battle are
/// determined as follows."
/// §16-35-4-1 "The card with the higher number of digivolution cards is the
/// winner."
/// §16-35-4-2 "The card with the lower number of digivolution cards is the
/// loser."
/// §16-35-4-3 "If both have the same number of digivolution cards, both
/// cards lose the battle."
///
/// `ICECLAD` has 2000 DP; `LV5_PLAIN` 7000 DP.
/// - `ICECLAD` with 2 digivolution cards vs suspended `LV5_PLAIN` with 0:
///   `ICECLAD` wins (higher), `LV5_PLAIN` is deleted.
/// - `ICECLAD` with 0 vs `LV5_PLAIN` with 1: `ICECLAD` loses (lower).
/// - 1 vs 1: both lose and are deleted.
/// - <Iceclad> on the defender: P0's `LV5_PLAIN` (0) attacks P1's `ICECLAD`
///   with 1: `LV5_PLAIN` loses.
/// - Not against a Security Digimon: `ICECLAD` with 2 digivolution cards
///   attacks P1 and checks `LV5_PLAIN`: DP decides, `ICECLAD` loses.
#[test]
fn iceclad_compares_digivolution_cards_tie_both_lose_not_vs_security() {
    cite!(
        "16-35-1",
        "a4a0f532c95064b547b62a7dbe5dcee8070b4d216b6fe12e966f9b240cfa76b6"
    );
    cite!(
        "16-35-3",
        "5a464ccc42faf7f21c3df8391e8cfb2d1c8002ea95acbe5b49022ca060f6cc27"
    );
    cite!(
        "16-35-4",
        "38668197775b95f4bfaf8a0539465081605ed16d2018c8697c9ca87bcd086efe"
    );
    cite!(
        "16-35-4-1",
        "2e8096ccea71e2dbd4b34d89349d6446cf4b4ee2cb5bda780c16f51530b6caa0"
    );
    cite!(
        "16-35-4-2",
        "045bf4b2ecddedd3c1fa51b74527a212608665f51b691748c336a08388523f02"
    );
    cite!(
        "16-35-4-3",
        "dd608693ff0dd46bb75047ffbd698b1121d094f4bf9776b3d94097f752f0a4d2"
    );
    let r = iceclad_battle(&[FILL, FILL, "ICECLAD"], &["LV5_PLAIN"]);
    assert_eq!(
        field_top_ids(&r, 0),
        owned(&["ICECLAD"]),
        "higher count wins"
    );
    assert!(field_top_ids(&r, 1).is_empty(), "lower count loses");

    let r = iceclad_battle(&["ICECLAD"], &[FILL, "LV5_PLAIN"]);
    assert!(field_top_ids(&r, 0).is_empty(), "lower count loses");
    assert_eq!(field_top_ids(&r, 1), owned(&["LV5_PLAIN"]));

    let r = iceclad_battle(&[FILL, "ICECLAD"], &[FILL, "LV5_PLAIN"]);
    assert!(field_top_ids(&r, 0).is_empty(), "tie: both lose");
    assert!(field_top_ids(&r, 1).is_empty(), "tie: both lose");

    let r = iceclad_battle(&["LV5_PLAIN"], &[FILL, "ICECLAD"]);
    assert!(
        field_top_ids(&r, 0).is_empty(),
        "the defender's <Iceclad> applies"
    );
    assert_eq!(field_top_ids(&r, 1), owned(&["ICECLAD"]));

    let mut r = board(&[FILL, FILL, "LV5_PLAIN"]);
    let a = r.place_stack(0, &[FILL, FILL, "ICECLAD"]);
    let cp = r.event_checkpoint();
    r.attack_player(a, 1, false);
    settle_declining(&mut r);
    assert_eq!(revealed(&r, cp, 1), owned(&["LV5_PLAIN"]));
    assert!(
        field_top_ids(&r, 0).is_empty(),
        "vs a Security Digimon: DP decides"
    );
}

/// §16-35-2 "<Iceclad> is a persistent effect."
///
/// P0's `WA_ICECLAD` (2000 DP, "[When Attacking] This Digimon gains
/// <Iceclad> for the turn") with 2 digivolution cards attacks P1's suspended
/// `LV5_PLAIN` (7000 DP, 0 digivolution cards): the <Iceclad> it gained
/// applies to the battle at once, with no decision: it wins. Control: the
/// same stack under `LV4_PLAIN` (5000 DP, no text) loses on DP.
#[test]
fn iceclad_kind_matches_rule() {
    cite!(
        "16-35-2",
        "04c520092a6955df8505b23a727b25c4adaa8dc80b15f941f96f437d1f829243"
    );
    let r = iceclad_battle(&[FILL, FILL, "LV4_PLAIN"], &["LV5_PLAIN"]);
    assert!(field_top_ids(&r, 0).is_empty(), "control: DP decides");

    let mut r = board(&[FILL; 3]);
    let a = r.place_stack(0, &[FILL, FILL, "WA_ICECLAD"]);
    let d = place_suspended(&mut r, 1, "LV5_PLAIN");
    r.attack_digimon(a, d, false);
    let prompts = settle_declining(&mut r);
    assert!(prompts.is_empty(), "no decision: {prompts:?}");
    assert_eq!(field_top_ids(&r, 0), owned(&["WA_ICECLAD"]));
    assert!(field_top_ids(&r, 1).is_empty());
}

// ─── §16-38 <Execute> ───────────────────────────────────────────────────────

/// P0 has `EXECUTE` (5000 DP, slot 0) plus `extra`; P1 has an UNSUSPENDED
/// `FILL` (slot 0) and 3 security cards.
fn execute_board(extra: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = board(&[FILL; 3]);
    let e = r.place_on_field(0, "EXECUTE", Some(0));
    for id in extra {
        r.place_on_field(0, id, Some(0));
    }
    r.place_on_field(1, FILL, Some(0));
    (r, e)
}

/// §16-38-1 "<Execute> is a keyword effect. At the end of your turn, the
/// Digimon with this keyword effect may attack. At the end of the attack,
/// this Digimon is deleted. This effect also allows for attacking an
/// opponent's unsuspended Digimon."
/// §16-38-4 "The processing from <Execute> for deleting this Digimon at the
/// end of the attack is pending processing."
/// §18-1-2 "If other processing will be performed at the same time as
/// pending processing, it is performed similar to effects that trigger
/// simultaneously."
///
/// P0 also has `EOA_OBS` ("[All Turns] At the end of an attack, gain 1
/// memory"). At the end of P0's turn `EXECUTE` may attack P1's unsuspended
/// `FILL` (and P1). It attacks `FILL` and wins. At the end of the attack the
/// pending deletion and `EOA_OBS` pend together for P0 to order; P0 takes
/// `EOA_OBS` first: its memory gain happens while `EXECUTE` is still in the
/// battle area, then `EXECUTE` is deleted.
#[test]
fn execute_optional_eot_attack_then_pending_deletion_at_end_of_attack() {
    cite!(
        "16-38-1",
        "b090a85e9f9c4f35e89d7a59494000303d11788a70b96c098e7123119e38c356"
    );
    cite!(
        "16-38-4",
        "7128885b633f7006c37746edcd94e3a1a4a50abee76aa37d909ca823b1c4509a"
    );
    cite!(
        "18-1-2",
        "1b5f8339dafb6d36f24a37380b4ccbcf4131788de779f4d1800da5a65bbd3372"
    );
    let (mut r, e) = execute_board(&["EOA_OBS"]);
    // §16-38-2: <Execute> is the only `[End of Your Turn]` effect; its
    // attack decision is offered as soon as P0 ends the turn.
    r.end_turn();
    assert!(eot_attack_offered(&r, e, 0), "the unsuspended Digimon");
    assert!(eot_attack_offered(&r, e, SECURITY_TARGET), "or the player");
    assert!(legal(&r, 0, PASS));
    let cp = r.event_checkpoint();
    r.game.decode_action(encode_attack(e.index as u16, 0), 0);
    let mut cards: Vec<String> = order_choices(&r, 0)
        .iter()
        .map(|l| branch_card(l).to_string())
        .collect();
    cards.sort();
    assert_eq!(
        cards,
        owned(&["EOA_OBS", "EXECUTE"]),
        "the pending deletion and the end-of-attack effect pend together"
    );
    assert!(field_top_ids(&r, 1).is_empty(), "FILL lost the battle");
    choose(&mut r, "EOA_OBS");
    settle_declining(&mut r);
    let gain = memory_event_index(&r, cp, "EOA_OBS").expect("EOA_OBS resolved");
    let deletion = trash_event_index(&r, cp, 0, "EXECUTE").expect("EXECUTE was deleted");
    assert!(gain < deletion, "in the chosen order: {gain} < {deletion}");
    assert_eq!(field_top_ids(&r, 0), owned(&["EOA_OBS"]));
    finish_end_of_turn(&mut r);
    assert_eq!(r.turn_player(), 1);
}

/// §16-38-3 "The processing from <Execute> is optional."
///
/// At the end of P0's turn P0 declines the <Execute> attack: no attack,
/// `EXECUTE` is not deleted, and the turn passes to P1. (§16-38-2: the
/// decline of <Execute>'s own decision in the `[End of Your Turn]` pool, not
/// a PASS in a later window.)
#[test]
fn execute_optionality_matches_rule() {
    cite!(
        "16-38-3",
        "d5984a8699f79076ec9ab2d55b5b0b0e07891cb8491f35e1b0afcd0f32a899a2"
    );
    let (mut r, e) = execute_board(&[]);
    r.end_turn();
    assert!(eot_attack_offered(&r, e, 0));
    let p = current_prompt(&r).expect("<Execute>'s decision");
    assert!(p.optional, "{p:?}");
    r.decline_optional_trigger().expect("decline <Execute>");
    settle_declining(&mut r);
    assert_eq!(field_top_ids(&r, 0), owned(&["EXECUTE"]));
    assert_eq!(field_top_ids(&r, 1), owned(&[FILL]));
    assert_eq!(r.turn_player(), 1);
}

/// §16-38-2 "<Execute> is a trigger-type effect that triggers at the end of
/// YOUR turn."
///
/// - P0's `EXECUTE` attacks P1's suspended `LV4_PLAIN` in its MAIN phase and
///   wins: it is not deleted (no <Execute>).
/// - At the end of P1's turn P0's (unsuspended) `EXECUTE` does not trigger:
///   no attack is offered and the turn passes back to P0.
#[test]
fn execute_timing_matches_rule() {
    cite!(
        "16-38-2",
        "01e0440be41939f399496200cabe760c9f909e0e54aa9099b61c76b33435c75c"
    );
    let mut r = board(&[FILL; 3]);
    let e = r.place_on_field(0, "EXECUTE", Some(0));
    let d = place_suspended(&mut r, 1, FILL);
    r.attack_digimon(e, d, false);
    settle_declining(&mut r);
    assert_eq!(
        field_top_ids(&r, 0),
        owned(&["EXECUTE"]),
        "main-phase attack"
    );

    let (mut r, _) = execute_board(&[]);
    through_p1_end_of_turn(&mut r);
    assert_eq!(r.turn_player(), 0);
    assert_eq!(field_top_ids(&r, 0), owned(&["EXECUTE"]));
}

/// §16-38-2 "<Execute> is a TRIGGER-TYPE effect that triggers at the end of
/// your turn."
///
/// P0 has `EXECUTE` and `EOT_MEM` ("[End of Your Turn] Gain 2 memory"). At
/// the end of P0's turn both pend in one `TriggerOrder`; P0 activates
/// <Execute> first: its attack is offered while `EOT_MEM` is still pending,
/// and `EOT_MEM` resolves only after the attack declaration (here on P1).
///
/// Official Q&A Q3868 / Q3869 (EX7-064, <Vortex>): <Vortex> "triggers
/// simultaneously with [End of Your Turn] effects, therefore you can choose
/// the activation order"; activating <Vortex> first, "you can activate
/// <Vortex> and make an attack declaration, [the other] effect that triggered
/// simultaneously will activate, then the counter timing will occur." So the
/// still-pending effect resolves AFTER the attack declaration and BEFORE the
/// counter timing / battle; the test pins declaration < `EOT_MEM` < security check (the same simultaneous-trigger order the rulings give for
/// <Vortex>).
#[test]
fn execute_kind_matches_rule() {
    cite!(
        "16-38-2",
        "01e0440be41939f399496200cabe760c9f909e0e54aa9099b61c76b33435c75c"
    );
    let (mut r, e) = execute_board(&["EOT_MEM"]);
    let cp = r.event_checkpoint();
    r.end_turn();
    let mut cards: Vec<String> = order_choices(&r, 0)
        .iter()
        .map(|l| branch_card(l).to_string())
        .collect();
    cards.sort();
    assert_eq!(cards, owned(&["EOT_MEM", "EXECUTE"]), "both pend together");
    choose(&mut r, "EXECUTE");
    assert!(memory_sources(&r, cp).is_empty(), "EOT_MEM still pending");
    assert!(eot_attack_offered(&r, e, SECURITY_TARGET));
    declare_eot_attack(&mut r, e, SECURITY_TARGET);
    settle_declining(&mut r);
    let declared = attack_event_index(&r, cp).expect("the <Execute> attack was declared");
    let gain = memory_event_index(&r, cp, "EOT_MEM").expect("EOT_MEM resolved");
    let check = first_reveal(&r, cp).expect("the <Execute> attack checked security");
    assert!(
        declared < gain && gain < check,
        "declaration (#{declared}) < EOT_MEM (#{gain}) < security check (#{check})"
    );
}

// ─── §16-39 <Progress> ──────────────────────────────────────────────────────

/// The battle-area slots P1's pending "1 of your opponent's Digimon" pick
/// offers.
fn p1_pick_slots(r: &DebugRunner) -> Vec<u16> {
    let p = current_prompt(r).expect("P1's pick");
    assert_eq!(p.player, 1, "{p:?}");
    assert_eq!(p.kind, SelectionKind::OppField, "{p:?}");
    offered_slots(&p)
}

/// §16-39-1 "<Progress> is a keyword effect that reads "this Digimon isn't
/// affected by your opponent's effects while attacking.""
/// §16-39-3 "<Progress> will activate while the Digimon with this effect is
/// attacking."
/// §15-15-5-3 "'Isn't affected by effects' cards can still be chosen for
/// effects."
///
/// Re-pinned 2026-10-16 (Plan 4 Task 24 fix round 2, controller-authorised):
/// this test used to assert that P1's pick does NOT offer the attacking
/// `PROGRESS`. That was wrong against §15-15-5-3 (an unaffected card can still
/// be chosen; DCGO's `SelectPermanentEffect` has no `CanNotBeAffected`
/// candidacy filter either). It now asserts the attacker IS offered and, once
/// chosen, is NOT affected while attacking.
///
/// P1 has `OPP_WA_MINUS3000` ("[All Turns] When an opponent's Digimon
/// attacks, 1 of your opponent's Digimon gets -3000 DP for the turn"),
/// suspended. P0 has `PROGRESS` (slot 0, 6000 DP), `FILL` (slot 1) and
/// `LV5_PLAIN` (slot 2).
/// - `PROGRESS` attacks P1's suspended `OPP_WA_MINUS3000` (3000 DP): P1's pick
///   offers all three, P1 picks `PROGRESS`. It isn't affected while
///   attacking: it battles at 6000 DP and wins (at 3000 it would tie and both
///   would be deleted). When the attack ends the -3000 DP, still in its
///   duration, affects it (§15-15-5-2): 3000 DP.
/// - On a fresh board, `LV5_PLAIN` attacks: `PROGRESS` is not attacking, so
///   P1 picks it and it gets -3000 DP.
#[test]
fn progress_persistent_only_while_attacking() {
    cite!(
        "16-39-1",
        "e7e71a76297e6988256aba87d4bd179c8bdfee7253b8123a05dd238fa6870119"
    );
    cite!(
        "16-39-3",
        "29c92e50cafda2ea45440650f22852032229f11e2d7e895807b612d06ff892ef"
    );
    cite!(
        "15-15-5-3",
        "170d539a05566095ac27d28910127cd4991e960081e9eb56d89d9f94423b976d"
    );
    let mut r = board(&[FILL; 5]);
    let progress = r.place_on_field(0, "PROGRESS", Some(0));
    r.place_on_field(0, FILL, Some(0));
    r.place_on_field(0, "LV5_PLAIN", Some(0));
    let target = place_suspended(&mut r, 1, "OPP_WA_MINUS3000");

    r.attack_digimon(progress, target, false);
    assert_eq!(
        p1_pick_slots(&r),
        vec![0, 1, 2],
        "the attacking PROGRESS can still be chosen (§15-15-5-3)"
    );
    pick_field(&mut r, 0);
    settle_declining(&mut r);
    assert!(
        r.game.player(1).battle_area.is_empty(),
        "not affected while attacking: 6000 DP beats 3000 DP"
    );
    assert_eq!(
        r.effective_dp(progress),
        Some(3000),
        "the attack is over: the -3000 DP now affects it"
    );

    let mut r = board(&[FILL; 5]);
    let progress = r.place_on_field(0, "PROGRESS", Some(0));
    r.place_on_field(0, FILL, Some(0));
    let other = r.place_on_field(0, "LV5_PLAIN", Some(0));
    r.place_on_field(1, "OPP_WA_MINUS3000", Some(0));
    r.attack_player(other, 1, false);
    assert_eq!(
        p1_pick_slots(&r),
        vec![0, 1, 2],
        "PROGRESS is not attacking"
    );
    pick_field(&mut r, 0);
    settle_declining(&mut r);
    assert_eq!(
        r.effective_dp(progress),
        Some(3000),
        "affected when not attacking"
    );
}

/// §16-39-2 "<Progress> is a persistent effect."
/// §15-15-5-3 (as above).
///
/// Re-pinned 2026-10-16 (Plan 4 Task 24 fix round 2, controller-authorised):
/// this test used to assert that P1's pick offers only P0's `FILL`, not the
/// <Progress> attacker; §15-15-5-3 keeps the attacker choosable. It now
/// asserts the attacker is offered and, chosen, is not affected.
///
/// P0's `WA_PROGRESS` ("[When Attacking] This Digimon gains <Progress> for
/// the turn", 6000 DP) attacks P1's suspended `OPP_WA_MINUS3000` (3000 DP);
/// that Digimon's own "when an opponent's Digimon attacks" trigger resolves
/// after the [When Attacking] (the turn player's effects first, §15-4-3-5).
/// The <Progress> gained applies at once, with no activation: P1's pick
/// offers both of P0's Digimon, P1 picks `WA_PROGRESS`, and the -3000 DP
/// doesn't affect it during the attack (it battles at 6000 and wins). Once
/// the attack ends the -3000 DP applies: 3000 DP.
#[test]
fn progress_kind_matches_rule() {
    cite!(
        "16-39-2",
        "8ccc13d0144f4f4dcbaddcfbd9d2b8617d5d60d8f4a6bbe5b7203c7f725b953e"
    );
    cite!(
        "15-15-5-3",
        "170d539a05566095ac27d28910127cd4991e960081e9eb56d89d9f94423b976d"
    );
    let mut r = board(&[FILL; 3]);
    let a = r.place_on_field(0, "WA_PROGRESS", Some(0));
    r.place_on_field(0, FILL, Some(0));
    let target = place_suspended(&mut r, 1, "OPP_WA_MINUS3000");
    r.attack_digimon(a, target, false);
    assert_eq!(
        p1_pick_slots(&r),
        vec![0, 1],
        "the <Progress> attacker can still be chosen (§15-15-5-3)"
    );
    pick_field(&mut r, 0);
    settle_declining(&mut r);
    assert!(
        r.game.player(1).battle_area.is_empty(),
        "the gained <Progress> protects at once: 6000 DP beats 3000 DP"
    );
    assert_eq!(r.effective_dp(a), Some(3000), "affected once the attack ends");
}
