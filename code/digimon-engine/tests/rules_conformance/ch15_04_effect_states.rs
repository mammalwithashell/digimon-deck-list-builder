//! Chapter 15-4 "Effect States" (Comprehensive Rules Ver. 4.3): activation,
//! triggering, simultaneous triggering, pending activation and derived
//! triggering.
//!
//! Every test asserts the RULE, never the engine's current output. A test the
//! engine fails is `#[ignore]`d with the gap id that explains it (discover then
//! pin); see `docs/RUST_ENGINE_GAPS.md` and `qa/rules/cr_obligations_status.yaml`.
//!
//! Fixture cards: `fixtures/common/*.yaml` + `fixtures/ch15_04/*.yaml`, one
//! synthetic card per file, authored to printed-style text (each clause's
//! `summary:` is that text).

use crate::fixtures;
use crate::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{EffectTiming, Keyword, PlaySource};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

// ─── §15-4-2-3 / §15-4-4-2 ──────────────────────────────────────────────────

/// §15-4-2-3 "Effects that are pending activation must be activated 1 at a
/// time. Multiple triggered effects can't be activated at the same time."
/// (§15-4-4-2 repeats the sentence for pending activation.)
///
/// An attacker carrying three `[When Attacking]` effects (its own
/// `WA_MEM` "Gain 1 memory" plus two inherited `WA_DRAW_INH` "<Draw 1>")
/// attacks.
/// Nothing activates before the turn player picks; picking `WA_MEM` activates
/// exactly that effect (memory +1, hand unchanged) and the two draws remain
/// pending behind a fresh choice.
#[test]
fn pending_effects_activate_one_at_a_time() {
    cite!(
        "15-4-2-3",
        "069d26193438f6af16e878c2242693af81dd37012aa70720f9d5082099a88bdb"
    );
    cite!(
        "15-4-4-2",
        "069d26193438f6af16e878c2242693af81dd37012aa70720f9d5082099a88bdb"
    );
    let mut r = fixtures::builder(&[fixtures::ch15_04_cards()])
        .deck(0, &[FILL; 6])
        .security(1, &[FILL; 3])
        .memory(3)
        .start();
    let attacker = r.place_stack(0, &["WA_DRAW_INH", "WA_DRAW_INH", "WA_MEM"]);
    let mem0 = r.memory();
    let hand0 = r.hand_size(0);

    r.attack_player(attacker, 1, false);

    let choices = order_choices(&r, 0);
    assert_eq!(
        choices.len(),
        3,
        "all three [When Attacking] effects pending: {choices:?}"
    );
    assert_eq!(
        (r.memory(), r.hand_size(0)),
        (mem0, hand0),
        "nothing activates before the choice"
    );

    choose(&mut r, "WA_MEM");
    assert_eq!(r.memory(), mem0 + 1, "the chosen effect activated");
    assert_eq!(
        r.hand_size(0),
        hand0,
        "the other pending effects did not activate with it"
    );
    let rest = order_choices(&r, 0);
    assert_eq!(rest.len(), 2, "the two draws are still pending: {rest:?}");
    assert!(
        rest.iter().all(|l| branch_card(l) == "WA_DRAW_INH"),
        "{rest:?}"
    );

    choose(&mut r, "WA_DRAW_INH");
    assert_eq!(
        r.hand_size(0),
        hand0 + 2,
        "both draws activate, one after the other"
    );
    assert_eq!(r.memory(), mem0 + 1);
}

// ─── §15-4-3-2 / §16-24-2 ───────────────────────────────────────────────────

/// §15-4-3-2 "If multiple effects are triggered before a single rule or effect
/// is resolved, they all trigger simultaneously. If effects trigger
/// simultaneously, the effects will be pending activation and activate 1 at a
/// time."
/// §16-24-2 "<Alliance> is a trigger-type effect that triggers when a Digimon
/// with this effect attacks."
///
/// `WA_ALLIANCE` (printed `<Alliance>` and `[When Attacking] Gain 1 memory`)
/// attacks with an unsuspended ally available. Both effects trigger on the
/// same attack, so the turn player gets ONE ordering choice offering both;
/// neither activates first in a window of its own. (DCGO: `AttackProcess.cs`
/// stacks both in one `StackSkillInfos(..., EffectTiming.OnAllyAttack)` batch;
/// `Alliance.cs` builds `<Alliance>` as an `ActivateClass` in that batch.)
#[test]
fn when_attacking_and_alliance_share_one_pool() {
    cite!(
        "15-4-3-2",
        "9806b36822f7fbe00db55ccc91fbb1cd9e2988f7064073bf57491efb6f425b57"
    );
    cite!(
        "16-24-2",
        "bd670d479e92a9d73ac4a88f81b37040639fd11848da35490e846b59ffb112dc"
    );
    let mut r = fixtures::builder(&[fixtures::ch15_04_cards()])
        .deck(0, &[FILL; 4])
        .security(1, &[FILL; 3])
        .memory(3)
        .start();
    let attacker = r.place_on_field(0, "WA_ALLIANCE", Some(0));
    r.place_on_field(0, FILL, Some(0));
    let mem0 = r.memory();

    r.attack_player(attacker, 1, false);

    let sel = r
        .pending_selection()
        .expect("an ordering decision after the attack");
    assert_eq!(
        sel.kind,
        SelectionKind::TriggerOrder,
        "<Alliance> and [When Attacking] trigger together: one ordering choice, not a separate \
         Alliance window: {sel:?}"
    );
    let choices = sel.effect_choices.as_ref().expect("branches");
    assert_eq!(choices.len(), 2, "{choices:?}");
    assert!(
        choices.iter().any(|c| c.keyword == Some(Keyword::Alliance)),
        "<Alliance> is offered in the pool: {choices:?}"
    );
    assert!(
        choices
            .iter()
            .any(|c| c.keyword.is_none() && c.timing == Some(EffectTiming::WhenAttacking)),
        "the [When Attacking] effect is offered in the pool: {choices:?}"
    );
    assert_eq!(
        r.memory(),
        mem0,
        "nothing activated before the turn player ordered the pool"
    );
}

// ─── §15-4-3-5-1 / §15-4-3-5-2 ──────────────────────────────────────────────

/// §15-4-3-5-1 "The turn player chooses 1 of their effects to activate from
/// among the effects that triggered simultaneously. This step is repeated
/// until there are no more pending activation effects for the turn player."
/// §15-4-3-5-2 "Once there are no more pending activation effects for the turn
/// player, the non-turn player chooses 1 of their effects to activate from
/// among the effects that triggered simultaneously. This step is repeated
/// until there are no more pending activation effects for the non-turn
/// player."
///
/// P0's attacker carries two `[When Attacking]` effects (`WA_MEM` + inherited
/// `WA_DRAW_INH`); P1 has two `OPP_WA_OBS` ("When an opponent's Digimon attacks,
/// gain 1 memory"). All four trigger on the attack. P0 orders and activates
/// both of theirs first; only then does P1 get to order theirs.
#[test]
fn turn_player_orders_first_then_non_turn_player() {
    cite!(
        "15-4-3-5-1",
        "d89bb63cefda9cf4c0485d6d3b9c0c7f799c40b360e7255e3e91e64c3cfd0616"
    );
    cite!(
        "15-4-3-5-2",
        "e71f29e0e84022ff4c506913e8c99313a93bc72e23d18d5a0a140dcf63c3235d"
    );
    let mut r = fixtures::builder(&[fixtures::ch15_04_cards()])
        .deck(0, &[FILL; 4])
        .security(1, &[FILL; 3])
        .memory(3)
        .start();
    let attacker = r.place_stack(0, &["WA_DRAW_INH", "WA_MEM"]);
    r.place_on_field(1, "OPP_WA_OBS", Some(0));
    r.place_on_field(1, "OPP_WA_OBS", Some(0));
    let hand0 = r.hand_size(0);
    let cp = r.event_checkpoint();

    r.attack_player(attacker, 1, false);

    let p0 = order_choices(&r, 0);
    assert_eq!(
        p0.len(),
        2,
        "the turn player chooses among THEIR effects only: {p0:?}"
    );
    assert!(
        p0.iter()
            .all(|l| ["WA_MEM", "WA_DRAW_INH"].contains(&branch_card(l))),
        "{p0:?}"
    );
    choose(&mut r, "WA_MEM");
    assert_eq!(
        r.hand_size(0),
        hand0 + 1,
        "the turn player's last pending effect activates before the non-turn player chooses"
    );

    let p1 = order_choices(&r, 1);
    assert_eq!(
        p1.len(),
        2,
        "then the non-turn player chooses among theirs: {p1:?}"
    );
    assert!(p1.iter().all(|l| branch_card(l) == "OPP_WA_OBS"), "{p1:?}");
    choose(&mut r, "OPP_WA_OBS");

    assert_eq!(
        memory_sources(&r, cp),
        vec![
            (0, "WA_MEM".to_string()),
            (1, "OPP_WA_OBS".to_string()),
            (1, "OPP_WA_OBS".to_string()),
        ],
        "every turn-player effect activates before any non-turn-player effect"
    );
}

// ─── §14-2-4 / §15-4-3-5-1 / §16-7-4 — the battle's simultaneous pool ──────

/// The board of the two battle-pool tests: P0 (the turn player) has `p0` and
/// attacks P1's suspended `defender` with `attacker`; P1's hand is `p1_hand`.
fn battle_pool_board(
    p0: &[&str],
    attacker: &str,
    defender: &str,
    p1_hand: &[&str],
) -> (DebugRunner, PermanentHandle, usize) {
    let mut r = fixtures::builder(&[fixtures::ch15_04_cards(), fixtures::ch11_14_cards()])
        .hand(1, p1_hand)
        .deck(0, &[FILL; 4])
        .deck(1, &[FILL; 4])
        .security(1, &[FILL; 3])
        .memory(3)
        .start();
    for id in p0 {
        r.place_on_field(0, id, Some(0));
    }
    let attacker = r.place_on_field(0, attacker, Some(0));
    let defender = r.place_on_field(1, defender, Some(0));
    set_suspended(&mut r, defender, true);
    let cp = r.event_checkpoint();
    r.attack_digimon(attacker, defender, false);
    (r, attacker, cp)
}

/// §14-2-4 "If an effect is triggered by a battle, that effect is to be
/// resolved. The next action won't begin until all of the processing is
/// resolved for the current action."
/// §15-4-3-2: effects triggered before a single rule or effect is resolved
/// trigger simultaneously and are pending activation together.
/// §15-4-3-5-1: the turn player chooses among their simultaneously triggered
/// effects first.
/// §14-2-5: an effect triggered by the end of the battle timing resolves when
/// the battle ends.
/// Official Q&A Q3931 (EX8-045 Callismon): the attacker's battle trigger and
/// the deleted Digimon's [On Deletion] "will trigger simultaneously, then the
/// turn player's will activate first."
///
/// P0's `WIN_MEM` ("[All Turns] When this Digimon wins a battle, gain 1
/// memory") attacks and deletes P1's `OD_MEM` ("[On Deletion] Gain 1
/// memory"); P0 also has the Tamer `OBS_OPP_DEL_MEM` ("[All Turns] When one of
/// your opponent's Digimon is deleted, gain 1 memory") and `EOB_OBS` ("[All
/// Turns] At the end of a battle, gain 1 memory"). The battle win, the
/// deletion observer and the [On Deletion] are one pool: P0 orders its two
/// effects in one choice and both activate before P1's [On Deletion]; the end
/// of battle effect resolves after the pool. (DCGO: `Destroy()` stacks
/// `OnDestroyedAnyone`, the battle stacks its "wins a battle" effects on the
/// same list, and one `AutoProcessCheck` resolves both, turn player first.)
#[test]
fn battle_result_trigger_pools_with_the_losers_on_deletion_turn_player_first() {
    cite!(
        "14-2-4",
        "2e2abfeea7dd5c9b26f20c6fb0bc32ff1f90ed476a8e1ee77441f2c64a5677f2"
    );
    cite!(
        "14-2-5",
        "2a85b51d06f6d8c5131ab14caa6bbce08a733d80eb911587ac8ee1471c330c91"
    );
    cite!(
        "15-4-3-2",
        "9806b36822f7fbe00db55ccc91fbb1cd9e2988f7064073bf57491efb6f425b57"
    );
    cite!(
        "15-4-3-5-1",
        "d89bb63cefda9cf4c0485d6d3b9c0c7f799c40b360e7255e3e91e64c3cfd0616"
    );
    let (mut r, _, cp) =
        battle_pool_board(&["OBS_OPP_DEL_MEM", "EOB_OBS"], "WIN_MEM", "OD_MEM", &[]);

    assert!(field_top_ids(&r, 1).is_empty(), "OD_MEM was deleted in battle");
    let p0 = order_choices(&r, 0);
    assert_eq!(
        p0.len(),
        2,
        "the battle win and the deletion observer pend together for the turn player: {p0:?}"
    );
    assert!(
        p0.iter().any(|l| branch_card(l) == "WIN_MEM")
            && p0.iter().any(|l| branch_card(l) == "OBS_OPP_DEL_MEM"),
        "{p0:?}"
    );
    assert!(
        memory_sources(&r, cp).is_empty(),
        "nothing activated before the turn player ordered the pool: {:?}",
        memory_sources(&r, cp)
    );

    choose(&mut r, "WIN_MEM");
    resolve_prompts(&mut r);
    assert_eq!(
        memory_sources(&r, cp),
        vec![
            (0, "WIN_MEM".to_string()),
            (0, "OBS_OPP_DEL_MEM".to_string()),
            (1, "OD_MEM".to_string()),
            (0, "EOB_OBS".to_string()),
        ],
        "the turn player's battle-triggered effects, then the [On Deletion], then the end of battle"
    );
}

/// §16-7-4 "<Piercing> is pending processing that is processed immediately
/// before the transition to the end of attack timing. Therefore, it will
/// trigger simultaneously with effects triggered by the battle, but it is
/// processed before the end of the attack." Its example: the [On Deletion] of
/// the Digimon deleted in the battle is resolved first, then <Piercing> is
/// processed.
/// §15-4-3-5-1: the turn player's simultaneously triggered effects activate
/// first.
/// Official Q&A Q3931 (EX8-045 Callismon, whose <Piercing> holds only while
/// the opponent has no Digimon with as much or more DP): the <Piercing> and the
/// deleted Digimon's [On Deletion] "will trigger simultaneously, then the turn
/// player's will activate first"; the check is then performed before the End
/// of Attack timing "even if <Piercing> is lost after activation."
///
/// P0's `PIERCE_IF_TOP` (8000 DP; "[Your Turn] While your opponent has no
/// Digimon with as much or more DP as this Digimon, this Digimon gains
/// <Piercing>") deletes P1's `OD_PLAY_LV6` ("[On Deletion] Play 1 level 6
/// Digimon card from your hand without paying the cost") in battle. The
/// <Piercing> (P0's) activates before the [On Deletion] (P1's), which plays
/// `BIG_LV6` (12000 DP) and so ends the <Piercing>; the check was already
/// pending and is performed after the [On Deletion], once.
#[test]
fn piercing_triggers_with_the_battle_deletion_pool() {
    cite!(
        "16-7-4",
        "b5591590a21a8204243c527c38e7b651c10bb88a277084b7153a0d3babc80d91"
    );
    cite!(
        "15-4-3-5-1",
        "d89bb63cefda9cf4c0485d6d3b9c0c7f799c40b360e7255e3e91e64c3cfd0616"
    );
    let (mut r, _, cp) = battle_pool_board(&[], "PIERCE_IF_TOP", "OD_PLAY_LV6", &["BIG_LV6"]);
    resolve_prompts(&mut r); // the [On Deletion]'s hand pick: BIG_LV6

    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["BIG_LV6"]),
        "the [On Deletion] played the level 6 Digimon"
    );
    let events = r.events_since(cp);
    let play = events
        .iter()
        .position(|e| {
            matches!(e, GameEvent::Play { player: 1, card_id, .. } if card_id == "BIG_LV6")
        })
        .expect("the [On Deletion] resolved");
    let checks: Vec<usize> = events
        .iter()
        .enumerate()
        .filter(|(_, e)| matches!(e, GameEvent::SecurityReveal { defender: 1, .. }))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(
        checks.len(),
        1,
        "<Piercing> activated before the [On Deletion]: its check is performed although \
         <Piercing> was lost"
    );
    assert!(
        play < checks[0],
        "the [On Deletion] (#{play}) resolves before the <Piercing> check (#{})",
        checks[0]
    );
    assert_eq!(r.security_count(1), 2);
}

// ─── §15-4-4-3 ──────────────────────────────────────────────────────────────

/// §15-4-4-3 "When a card with an effect that's pending activation becomes a
/// new card before the effect activates, the effect can no longer be
/// activated. (Example: When an [On Play] effect is triggered by a card being
/// played, if the card is removed from the battle area before the effect is
/// activated, the effect can no longer be activated.)"
///
/// P0 plays `OP_PLAY_LV3` ("[On Play] Play 1 level 3 Digimon card from your
/// hand without paying the cost") while P0's Tamer `RET_PLAYED` ("[Your Turn]
/// When one of your Digimon is played, return that Digimon to the hand")
/// triggers on the same play. P0 activates the Tamer first: `OP_PLAY_LV3`
/// leaves the battle area, so its pending `[On Play]` can no longer activate
/// and the level 3 card stays in hand.
#[test]
fn pending_on_play_lost_when_card_leaves_before_activation() {
    cite!(
        "15-4-4-3",
        "1941c7ad46f8d54144ad9c35af181481892e72795e3a087c88af4adb28a64aea"
    );
    let mut r = fixtures::builder(&[fixtures::ch15_04_cards()])
        .hand(0, &["OP_PLAY_LV3", FILL])
        .deck(0, &[FILL; 4])
        .memory(5)
        .start();
    r.place_on_field(0, "RET_PLAYED", Some(0));

    r.play(0, 0);

    let choices = order_choices(&r, 0);
    assert_eq!(
        choices.len(),
        2,
        "[On Play] and the Tamer's observer both pending: {choices:?}"
    );
    choose(&mut r, "RET_PLAYED");

    assert!(
        r.pending_selection().is_none(),
        "the returned card's [On Play] can no longer activate (no hand pick offered): {:?}",
        r.pending_selection()
    );
    let mut hand = hand_ids(&r, 0);
    hand.sort();
    assert_eq!(
        hand,
        owned(&["OP_PLAY_LV3", FILL]),
        "OP_PLAY_LV3 returned; the level 3 card was never played"
    );
    assert_eq!(field_top_ids(&r, 0), owned(&["RET_PLAYED"]));
}

// ─── §15-4-4-4 ──────────────────────────────────────────────────────────────

/// §15-4-4-4 "When a card with an effect that's pending activation loses the
/// effect before it activates, the effect can no longer be activated.
/// (Example: When a Digimon with the text "[When Attacking] <Draw 1>" and
/// "[When Attacking] This Digimon may digivolve into a Digimon card with the
/// [Hybrid] trait in the hand with the digivolution cost reduced by 1"
/// attacks, then it digivolves from the 2nd [When Attacking] effect, the 1st
/// [When Attacking] effect can no longer activate.)"
///
/// The rule's own example card (`WA_HYBRID_DIGI`). Choosing the digivolve
/// first buries the card under `HYBRID_LV5`, so it loses its printed
/// `<Draw 1>`. (Official Q&A Q1366, BT5-089, rules the same shape.)
///
/// The engine emits no draw event, so the two possible draws are told apart by
/// WHEN the deck shrinks: `HYBRID_LV5`'s optional `[When Digivolving]` is a
/// derived trigger (§15-4-5-2), so it parks before the still-pending
/// `<Draw 1>` could be reconsidered. At that park the deck is down exactly 1
/// (the digivolution draw); after declining it, still exactly 1.
#[test]
fn digivolving_mid_pending_cancels_sibling_when_attacking() {
    cite!(
        "15-4-4-4",
        "469d8b351a87db94b824d5e64effbbeed85e1994c8c84b87d84e52ce4ae4ad7c"
    );
    let mut r = fixtures::builder(&[fixtures::ch15_04_cards()])
        .hand(0, &["HYBRID_LV5"])
        .deck(0, &[FILL; 6])
        .security(1, &[FILL; 3])
        .memory(5)
        .start();
    let attacker = r.place_on_field(0, "WA_HYBRID_DIGI", Some(0));
    let deck0 = r.deck_size(0);

    r.attack_player(attacker, 1, false);

    let choices = order_choices(&r, 0);
    assert_eq!(
        choices.len(),
        2,
        "both [When Attacking] effects pending: {choices:?}"
    );
    // Clause 0 is the <Draw 1>, clause 1 the digivolve (fixture order).
    choose(&mut r, "WA_HYBRID_DIGI slot 1");
    assert_eq!(r.deck_size(0), deck0, "nothing drawn before the digivolve");
    pick_hand_card(&mut r, 0, "HYBRID_LV5");

    assert_eq!(
        field_top_ids(&r, 0),
        owned(&["HYBRID_LV5"]),
        "the 2nd [When Attacking] digivolved the attacker"
    );
    assert!(
        r.pending_is_optional(),
        "HYBRID_LV5's optional [When Digivolving] is pending: {:?}",
        r.pending_selection()
    );
    assert_eq!(
        r.deck_size(0),
        deck0 - 1,
        "the digivolution draw has happened and nothing else has"
    );
    r.decline_optional_trigger()
        .expect("decline the [When Digivolving]");

    assert_eq!(
        r.deck_size(0),
        deck0 - 1,
        "the lost [When Attacking] <Draw 1> never activates"
    );
    assert!(
        r.pending_selection().is_none(),
        "{:?}",
        r.pending_selection()
    );
}

// ─── §15-4-4-5 ──────────────────────────────────────────────────────────────

/// A board for the §15-4-4-5 test: P0's Tamer `OBS_PLAY_LE4K` ("[Your Turn]
/// When one of your Digimon with 4000 DP or less is played, gain 1 memory")
/// on the field and `PLAY_DP_UP` (3000 DP; "[On Play] This Digimon gets +3000
/// DP for the turn") in hand.
fn trigger_condition_board() -> DebugRunner {
    let mut r = fixtures::builder(&[fixtures::ch15_04_cards()])
        .hand(0, &["PLAY_DP_UP"])
        .deck(0, &[FILL; 4])
        .memory(5)
        .start();
    r.place_on_field(0, "OBS_PLAY_LE4K", Some(0));
    r
}

/// §15-4-4-5 "When a card with an effect that's pending activation no longer
/// meets the trigger conditions of that effect before it activates, the
/// effect can no longer be activated."
///
/// Playing the 3000 DP `PLAY_DP_UP` meets the Tamer's trigger condition ("one
/// of your Digimon with 4000 DP or less is played"; "with 4000 DP or less" is
/// part of the trigger, not an "if" processing condition) and also triggers
/// the played card's `[On Play]`. Both are pending; the turn player orders.
/// - Control: the Tamer first -- the condition still holds, it activates.
/// - Lapse: the `[On Play]` first -- the played Digimon is now 6000 DP, so the
///   Tamer's effect no longer meets its trigger conditions and cannot
///   activate.
#[test]
fn pending_effect_whose_trigger_condition_lapsed_cannot_activate() {
    cite!(
        "15-4-4-5",
        "71bf5b7f1273d0267e98f23107451b614fc0644a66410a19605e28c4c4274329"
    );
    // Control: observer first.
    let mut r = trigger_condition_board();
    r.play(0, 0);
    let choices = order_choices(&r, 0);
    assert_eq!(
        choices.len(),
        2,
        "both effects triggered on the play: {choices:?}"
    );
    let mem0 = r.memory();
    choose(&mut r, "OBS_PLAY_LE4K");
    assert_eq!(
        r.memory(),
        mem0 + 1,
        "while its trigger condition still holds the Tamer's effect activates"
    );

    // Lapse: the DP boost first.
    let mut r = trigger_condition_board();
    r.play(0, 0);
    let choices = order_choices(&r, 0);
    assert_eq!(choices.len(), 2, "{choices:?}");
    let mem0 = r.memory();
    choose(&mut r, "PLAY_DP_UP");
    let played = r.perm_handle(0, 1);
    assert_eq!(
        field_top_ids(&r, 0),
        owned(&["OBS_PLAY_LE4K", "PLAY_DP_UP"])
    );
    assert_eq!(r.effective_dp(played), Some(6000), "the [On Play] resolved");
    assert_eq!(
        r.memory(),
        mem0,
        "the Tamer's pending effect no longer meets its trigger condition: it cannot activate"
    );
    assert!(
        r.pending_selection().is_none(),
        "{:?}",
        r.pending_selection()
    );
}

// ─── §15-8-3-9-2 / §15-6-3 (Q1009) ──────────────────────────────────────────

/// §15-8-3-9-2 "Trigger-type effects will trigger if their trigger conditions
/// are met, regardless of whether or not their processing conditions are met.
/// However, the processing conditions must be met at the time of activation
/// in order for the effect to activate."
/// §15-6-3 "An effect can't be activated when none of its processing
/// conditions are met."
///
/// Official Q&A Q1009 (BT2-034 Salamon, the text `OD_RECOVER_LE3` copies):
/// two copies of "[On Deletion] If you have 3 or fewer security cards,
/// <Recovery +1 (Deck)>" deleted together with 3 security cards -- the
/// effects activate one at a time; the first brings security to 4, so "the '3
/// cards or less' condition for the second Salamon's effect no longer
/// applies" and it doesn't activate. `SWEEPER`'s `[On Play]` deletes both at
/// once.
///
/// "If you have 3 or fewer security cards" is a PROCESSING condition (§15-6-1:
/// "if"), so this is §15-8-3-9-2, not §15-4-4-5 (trigger conditions). Reading
/// of §15-8-3-9-4 ("If a reference is to be made in the processing conditions
/// for a trigger-type effect that will trigger when a card is removed from an
/// area, the reference is made in the state when the effect triggered"): it
/// covers the REMOVED card's own last-known state (its example: "this
/// Digimon has digivolution cards"). A reference to general game state, such
/// as your security count, is made when the processing is performed
/// (§15-8-3-9-3), which is what Q1009 rules.
#[test]
fn pending_effect_whose_condition_lapsed_is_dropped() {
    cite!(
        "15-8-3-9-2",
        "20cccd04bf8c3740c1a02584acf244e96aa238524471e7c84760d928a2a487ee"
    );
    cite!(
        "15-6-3",
        "60afdff4c707b8d75fddfad99108b2407141ed4391014f83d44118f82ae9a3fd"
    );
    let mut r = fixtures::builder(&[fixtures::ch15_04_cards()])
        .hand(0, &["SWEEPER"])
        .deck(0, &[FILL; 6])
        .security(0, &[FILL; 3])
        .memory(5)
        .start();
    r.place_on_field(0, "OD_RECOVER_LE3", Some(0));
    r.place_on_field(0, "OD_RECOVER_LE3", Some(0));

    r.play(0, 0);

    let choices = order_choices(&r, 0);
    assert_eq!(
        choices.len(),
        2,
        "both [On Deletion] effects triggered: {choices:?}"
    );
    assert!(
        choices.iter().all(|l| branch_card(l) == "OD_RECOVER_LE3"),
        "{choices:?}"
    );
    choose(&mut r, "OD_RECOVER_LE3");

    assert_eq!(
        r.security_count(0),
        4,
        "the first activation brings security to 4; the second's processing condition is no \
         longer met when it would activate"
    );
    assert!(
        r.pending_selection().is_none(),
        "{:?}",
        r.pending_selection()
    );
}

// ─── §15-4-5-2 ──────────────────────────────────────────────────────────────

/// §15-4-5-2 "A derived triggering effect will activate before previously
/// triggered effects that are pending activation." (§15-4-5-1: derived
/// triggering is an effect that newly triggers while simultaneously triggered
/// effects are still resolving.)
///
/// P0's attacker has two `[When Attacking]` effects: `WA_DEL4K` ("Delete 1 of
/// your opponent's Digimon with 4000 DP or less") and inherited `WA_MEM_INH`.
/// P0 activates `WA_DEL4K` first; the deletion triggers P0's Tamer
/// `OBS_OPP_DEL_MEM` ("When one of your opponent's Digimon is deleted, gain 1
/// memory"). That derived trigger activates before the still-pending
/// `WA_MEM_INH`.
#[test]
fn derived_trigger_resolves_before_pending_siblings() {
    cite!(
        "15-4-5-2",
        "4673e5ae035f311cbd55487d1113223036236e4b3a0313d40fd3071df6a86fd5"
    );
    let mut r = fixtures::builder(&[fixtures::ch15_04_cards()])
        .deck(0, &[FILL; 4])
        .security(1, &[FILL; 3])
        .memory(3)
        .start();
    let attacker = r.place_stack(0, &["WA_MEM_INH", "WA_DEL4K"]);
    r.place_on_field(0, "OBS_OPP_DEL_MEM", Some(0));
    r.place_on_field(1, FILL, Some(0));
    let cp = r.event_checkpoint();

    r.attack_player(attacker, 1, false);

    let choices = order_choices(&r, 0);
    assert_eq!(choices.len(), 2, "{choices:?}");
    choose(&mut r, "WA_DEL4K");
    resolve_prompts(&mut r); // the deletion target

    assert!(
        field_top_ids(&r, 1).is_empty(),
        "the opponent's Digimon was deleted"
    );
    assert_eq!(
        memory_sources(&r, cp),
        vec![
            (0, "OBS_OPP_DEL_MEM".to_string()),
            (0, "WA_MEM_INH".to_string()),
        ],
        "the derived trigger activates before the previously pending [When Attacking]"
    );
}

// ─── §15-4-5-3 ──────────────────────────────────────────────────────────────

/// §15-4-5-3 "If a derived triggering effect occurs for the non-turn player
/// when there are pending activation effects for the turn player, the derived
/// triggering effect will activate first."
///
/// As above, but the deleted Digimon is P1's `OD_MEM` ("[On Deletion] Gain 1
/// memory"): the NON-turn player's derived trigger activates before the turn
/// player's still-pending inherited `WA_MEM_INH`.
#[test]
fn non_turn_player_derived_trigger_precedes_turn_player_pending() {
    cite!(
        "15-4-5-3",
        "ebf0e5a69eb565a471a02bb908f97532882d08996359cb4e113b2031c92ca83a"
    );
    let mut r = fixtures::builder(&[fixtures::ch15_04_cards()])
        .deck(0, &[FILL; 4])
        .security(1, &[FILL; 3])
        .memory(3)
        .start();
    let attacker = r.place_stack(0, &["WA_MEM_INH", "WA_DEL4K"]);
    r.place_on_field(1, "OD_MEM", Some(0));
    let cp = r.event_checkpoint();

    r.attack_player(attacker, 1, false);

    let choices = order_choices(&r, 0);
    assert_eq!(choices.len(), 2, "{choices:?}");
    choose(&mut r, "WA_DEL4K");
    resolve_prompts(&mut r); // the deletion target

    assert!(field_top_ids(&r, 1).is_empty(), "OD_MEM was deleted");
    assert_eq!(
        memory_sources(&r, cp),
        vec![(1, "OD_MEM".to_string()), (0, "WA_MEM_INH".to_string())],
        "the non-turn player's derived trigger activates before the turn player's pending effect"
    );
}

// ─── §15-4-1-2 ──────────────────────────────────────────────────────────────

/// §15-4-1-2 "When a card activates an effect, all of the effect is resolved,
/// even if that card becomes a new card during the processing for the
/// activated effect. (Example: If your Digimon has "[When Digivolving] Delete
/// 1 of your opponent's Digimon with 8000 DP or less and return 1 of their
/// level 6 or higher Digimon to the bottom of the deck," when you delete 1 of
/// your opponent's Digimon with 8000 DP or less, you will process "return 1
/// of their level 6 or higher Digimon to the bottom of the deck," even if
/// another immediate-type effect deletes your Digimon.)"
///
/// The rule's own example: P0 digivolves into `WD_DEL_RET`; deleting P1's
/// `WOULD_DEL_STRIKE` fires its immediate-type "When this Digimon would be
/// deleted, delete 1 of your opponent's Digimon", which deletes `WD_DEL_RET`
/// MID-effect: when the effect's second step (the return-target prompt) is
/// reached, `WD_DEL_RET` is already in P0's trash. That step still resolves:
/// P1's `BIG_LV6` goes to the bottom of P1's deck.
#[test]
fn activated_effect_completes_after_source_becomes_new_card() {
    cite!(
        "15-4-1-2",
        "972b67e36e718115c54e7c0e6a05b2aa6a5b89381cd9e43317ba8c322579e8a9"
    );
    let mut r = fixtures::builder(&[fixtures::ch15_04_cards()])
        .hand(0, &["WD_DEL_RET"])
        .deck(0, &[FILL; 4])
        .deck(1, &[FILL; 2])
        .memory(5)
        .start();
    let base = r.place_on_field(0, "HYBRID_LV5", Some(0));
    r.place_on_field(1, "WOULD_DEL_STRIKE", Some(0));
    r.place_on_field(1, "BIG_LV6", Some(0));

    assert!(
        r.game
            .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByHand),
        "digivolve into WD_DEL_RET"
    );
    // Picks: the delete target, the immediate effect's target, the return
    // target. Check the source's state when the return step is reached.
    let mut saw_return_step = false;
    for _ in 0..16 {
        let Some(sel) = r.pending_selection() else {
            break;
        };
        if sel
            .prompt
            .starts_with("Return 1 of your opponent's level 6")
        {
            saw_return_step = true;
            assert!(
                field_top_ids(&r, 0).is_empty(),
                "the source Digimon was deleted before the rest of its effect is processed"
            );
            assert!(
                card_ids(&r, &r.game.player(0).trash).contains(&"WD_DEL_RET".to_string()),
                "WD_DEL_RET is in the trash (a new card) while its effect is still resolving"
            );
        }
        step_prompt(&mut r);
    }
    assert!(
        saw_return_step,
        "the effect's return step was reached after the source left"
    );

    assert!(
        field_top_ids(&r, 1).is_empty(),
        "WOULD_DEL_STRIKE deleted and BIG_LV6 returned: {:?}",
        field_top_ids(&r, 1)
    );
    let deck1 = card_ids(&r, &r.game.player(1).deck);
    assert_eq!(
        deck1.first().map(String::as_str),
        Some("BIG_LV6"),
        "the rest of the effect still resolved: BIG_LV6 is at the bottom of the deck ({deck1:?})"
    );
}

// ─── §15-5-2 ────────────────────────────────────────────────────────────────

/// Use `MAIN_TRASH2_UNDER_MEM` ("[Main] By trashing 2 bottom face-down cards
/// from under any of your Tamers, gain 1 memory.") on a board of P0 Tamers
/// `TAMER_TRASH_OBS_MEM` ("[All Turns] When effects trash cards from under
/// this Tamer, gain 1 memory."), one per entry of `stacks` carrying those
/// cards under it (bottom first; `true` = face-down), plus P0's
/// `OBS_TAMER_TRASH` ("[All Turns] When effects trash cards from under your
/// Tamers, gain 1 memory."). `picks` are the Tamer slots chosen for the 2
/// cards.
///
/// Returns the card ids of the first pending pool after the Option resolves
/// (sorted), of every effect-sourced memory gain (sorted), and the cards left
/// under each Tamer (bottom first, as face-down flags).
fn multi_card_cost_trash(
    stacks: &[&[bool]],
    picks: &[u16],
) -> (Vec<String>, Vec<String>, Vec<Vec<bool>>) {
    let mut r = fixtures::builder(&[fixtures::ch15_04_cards(), fixtures::ch15_08_cards()])
        .hand(0, &["MAIN_TRASH2_UNDER_MEM"])
        .deck(0, &[FILL; 4])
        .memory(3)
        .start();
    let mut tamers = Vec::new();
    for cards in stacks {
        let tamer = r.place_on_field(0, "TAMER_TRASH_OBS_MEM", Some(0));
        // `attach_face_down` inserts at the bottom: attach top-down.
        for &face_down in cards.iter().rev() {
            attach_face_down(&mut r, tamer, FILL);
            r.game.players[0].battle_area[tamer.index as usize].card_sources[0].face_down =
                face_down;
        }
        tamers.push(tamer);
    }
    r.place_on_field(0, "OBS_TAMER_TRASH", Some(0));
    let cp = r.event_checkpoint();

    use_option(&mut r, 0, "MAIN_TRASH2_UNDER_MEM");
    for &slot in picks {
        pick_field(&mut r, slot);
    }
    let left: Vec<Vec<bool>> = tamers
        .iter()
        .map(|t| {
            let perm = &r.game.players[0].battle_area[t.index as usize];
            let under = perm.card_sources.len() - 1;
            perm.card_sources[..under].iter().map(|c| c.face_down).collect()
        })
        .collect();
    let face_down = |cards: &[bool]| cards.iter().filter(|&&d| d).count();
    assert_eq!(
        left.iter().map(|c| face_down(c)).sum::<usize>(),
        stacks.iter().map(|c| face_down(c)).sum::<usize>() - 2,
        "the cost trashed exactly 2 face-down cards"
    );

    let mut pool: Option<Vec<String>> = None;
    for _ in 0..8 {
        if r.pending_selection().is_none() {
            break;
        }
        let labels = order_choices(&r, 0);
        if pool.is_none() {
            let mut cards: Vec<String> =
                labels.iter().map(|l| branch_card(l).to_string()).collect();
            cards.sort();
            pool = Some(cards);
        }
        let first = labels[0].clone();
        choose(&mut r, &first);
    }
    assert!(r.pending_selection().is_none(), "the pool settled");
    let mut gains: Vec<String> = memory_sources(&r, cp)
        .into_iter()
        .map(|(_, id)| id)
        .collect();
    gains.sort();
    (pool.expect("the observers pended after the Option"), gains, left)
}

/// §15-5-2 "A triggering from 1 trigger condition is considered to only
/// trigger once, even if it occurred multiple times at the same time.
/// (Example: An effect that reads "[Opponent's Turn] When a card is removed
/// from your security stack, <De-Digivolve 1> 1 of your opponent's Digimon"
/// will only trigger once, even if 2 or more cards are trashed from your
/// security stack at the same time.)"
///
/// "By trashing 2 bottom face-down cards from under any of your Tamers" is one
/// processing condition (§15-7-1): its 2 cards are trashed at the same time,
/// as one event, whichever Tamers they come from (the shape of Cougarmon
/// BT25-035; DCGO asks one Tamer selection and stacks one
/// `OnDigivolutionCardDiscarded`). So:
/// - 2 cards from ONE Tamer: that Tamer's "under this Tamer" observer triggers
///   once, and the "under your Tamers" observer once.
/// - 1 card from EACH of two Tamers: each Tamer's own "under this Tamer"
///   observer triggers once (each saw its trigger condition met once), and the
///   "under your Tamers" observer still once.
/// - 2 cards from ONE Tamer whose stack is [face-down, face-up, face-down]
///   from the bottom: the two lowest face-down cards are trashed and the
///   face-up card stays, still as one event (each observer once).
#[test]
fn one_multi_card_cost_trash_triggers_each_observer_once() {
    cite!(
        "15-5-2",
        "316122b26772b466e8ad851c4b88b40688944039549ecbf4591f5b20c7b33b51"
    );
    // 2 cards from one Tamer (slot 0).
    let (pool, gains, _) = multi_card_cost_trash(&[&[true, true]], &[0, 0]);
    assert_eq!(
        pool,
        owned(&["OBS_TAMER_TRASH", "TAMER_TRASH_OBS_MEM"]),
        "2 cards from one Tamer: each observer pends once"
    );
    assert_eq!(
        gains,
        owned(&[
            "MAIN_TRASH2_UNDER_MEM",
            "OBS_TAMER_TRASH",
            "TAMER_TRASH_OBS_MEM"
        ]),
        "2 cards from one Tamer: each observer activates once"
    );

    // 1 card from each of two Tamers (slots 0 and 1).
    let (pool, gains, _) = multi_card_cost_trash(&[&[true], &[true]], &[0, 1]);
    assert_eq!(
        pool,
        owned(&[
            "OBS_TAMER_TRASH",
            "TAMER_TRASH_OBS_MEM",
            "TAMER_TRASH_OBS_MEM"
        ]),
        "1 card from each of two Tamers: each Tamer's own observer pends once, the global one once"
    );
    assert_eq!(
        gains,
        owned(&[
            "MAIN_TRASH2_UNDER_MEM",
            "OBS_TAMER_TRASH",
            "TAMER_TRASH_OBS_MEM",
            "TAMER_TRASH_OBS_MEM"
        ]),
        "1 card from each of two Tamers: each observer activates once"
    );

    // 2 cards from ONE Tamer whose stack is [face-down, face-up, face-down]
    // from the bottom: "the bottom face-down card" is the lowest face-down
    // card (card text BT26-005 / BT25-035, DCGO `BT26_005.cs`
    // `LastOrDefault(IsFlipped)`; the rules manual is silent), so the two
    // face-down cards are trashed and the face-up card between them stays --
    // in ONE event, so each observer triggers once.
    let (pool, gains, left) = multi_card_cost_trash(&[&[true, false, true]], &[0, 0]);
    assert_eq!(
        left,
        vec![vec![false]],
        "both face-down cards were trashed; the face-up card stays"
    );
    assert_eq!(
        pool,
        owned(&["OBS_TAMER_TRASH", "TAMER_TRASH_OBS_MEM"]),
        "around a face-up card: one event, each observer pends once"
    );
    assert_eq!(
        gains,
        owned(&[
            "MAIN_TRASH2_UNDER_MEM",
            "OBS_TAMER_TRASH",
            "TAMER_TRASH_OBS_MEM"
        ]),
        "around a face-up card: each observer activates once"
    );
}

// ─── §15-4-3-2 across the events of one effect ──────────────────────────────

/// Resolve the using effect's own prompts (the Tamer pick, P1's hand pick)
/// until the first `TriggerOrder` is offered or nothing is pending.
fn resolve_until_trigger_order(r: &mut DebugRunner) {
    for _ in 0..8 {
        let Some(sel) = r.pending_selection() else {
            return;
        };
        if sel.kind == SelectionKind::TriggerOrder {
            return;
        }
        if sel.selecting_player == 1 {
            pick_hand_card(r, 1, FILL);
        } else {
            step_prompt(r);
        }
    }
    panic!("the effect's own prompts did not settle");
}

/// §15-4-3-2 "If multiple effects are triggered before a single rule or effect
/// is resolved, they all trigger simultaneously. If effects trigger
/// simultaneously, the effects will be pending activation and activate 1 at a
/// time."
/// §15-4-3-5-1 "The turn player chooses 1 of their effects to activate from
/// among the effects that triggered simultaneously. This step is repeated
/// until there are no more pending activation effects for the turn player."
///
/// P0 uses `MAIN_TRASH_UNDER_OPP_DISCARD` ("[Main] Trash the bottom face-down
/// card from under 1 of your Tamers. Then, your opponent trashes 1 card in
/// their hand."): ONE effect, TWO events. P0's `TAMER_TWO_EVENT_OBS_MEM`
/// ("When your opponent's hand is trashed from or effects trash cards from
/// under this Tamer, gain 1 memory", Keenan Crier BT26-094's shape) and
/// `OPT_TWO_EVENT_OBS_DRAW` ("[Once Per Turn] When your opponent's hand is
/// trashed from or effects trash cards from under your Tamers, <Draw 1>",
/// Crowmon BT26-076's shape) each trigger on each event (§15-4-2-4: met again
/// before it activates, triggered again) -- four triggers, all before the one
/// effect has resolved, so they trigger simultaneously: ONE pool of four the
/// turn player orders, not one pool per event. (DCGO: one 4-entry
/// `MultipleSkills` for Crowmon + Keenan, `qa/qa-reports/exam-verdicts/BT26-076.json`.)
///
/// Both [Once Per Turn] entries are offered: the limit is checked when an
/// effect activates (DCGO `ActivateClass` `timesPerTurn`), so the pool holds
/// both, and once one has activated the other can no longer activate and is
/// not offered again.
#[test]
fn triggers_from_two_events_of_one_effect_pend_in_one_pool() {
    cite!(
        "15-4-3-2",
        "9806b36822f7fbe00db55ccc91fbb1cd9e2988f7064073bf57491efb6f425b57"
    );
    cite!(
        "15-4-2-4",
        "dea76fe97f7debaf2f9af69a29d70795982cfed0f27ca910ae763c21b4515663"
    );
    cite!(
        "15-4-3-5-1",
        "d89bb63cefda9cf4c0485d6d3b9c0c7f799c40b360e7255e3e91e64c3cfd0616"
    );
    let mut r = fixtures::builder(&[fixtures::ch15_04_cards()])
        .hand(0, &["MAIN_TRASH_UNDER_OPP_DISCARD"])
        .hand(1, &[FILL])
        .deck(0, &[FILL; 4])
        .memory(3)
        .start();
    let tamer = r.place_on_field(0, "TAMER_TWO_EVENT_OBS_MEM", Some(0));
    attach_face_down(&mut r, tamer, FILL);
    r.place_on_field(0, "OPT_TWO_EVENT_OBS_DRAW", Some(0));
    let cp = r.event_checkpoint();

    use_option(&mut r, 0, "MAIN_TRASH_UNDER_OPP_DISCARD");
    let hand0 = r.hand_size(0);
    resolve_until_trigger_order(&mut r);

    assert!(
        hand_ids(&r, 1).is_empty(),
        "both events happened: P1 trashed its hand card before anything activated"
    );
    assert!(
        memory_sources(&r, cp).is_empty() && r.hand_size(0) == hand0,
        "nothing activated before the pool was offered"
    );
    let mut pool: Vec<String> = order_choices(&r, 0)
        .iter()
        .map(|l| branch_card(l).to_string())
        .collect();
    pool.sort();
    assert_eq!(
        pool,
        owned(&[
            "OPT_TWO_EVENT_OBS_DRAW",
            "OPT_TWO_EVENT_OBS_DRAW",
            "TAMER_TWO_EVENT_OBS_MEM",
            "TAMER_TWO_EVENT_OBS_MEM"
        ]),
        "the triggers from both events pend in ONE pool"
    );

    choose(&mut r, "OPT_TWO_EVENT_OBS_DRAW");
    assert_eq!(
        r.hand_size(0),
        hand0 + 1,
        "the chosen [Once Per Turn] entry activated"
    );
    let rest = order_choices(&r, 0);
    assert_eq!(
        rest.iter().map(|l| branch_card(l)).collect::<Vec<_>>(),
        vec!["TAMER_TWO_EVENT_OBS_MEM", "TAMER_TWO_EVENT_OBS_MEM"],
        "the other [Once Per Turn] entry can no longer activate; both Tamer entries are still pending: {rest:?}"
    );
    choose(&mut r, "TAMER_TWO_EVENT_OBS_MEM");
    resolve_prompts(&mut r);

    assert_eq!(r.hand_size(0), hand0 + 1, "<Draw 1> activated once");
    assert_eq!(
        memory_sources(&r, cp),
        vec![
            (0, "TAMER_TWO_EVENT_OBS_MEM".to_string()),
            (0, "TAMER_TWO_EVENT_OBS_MEM".to_string()),
        ],
        "the Tamer's effect activated once per event"
    );
}

/// §15-4-3-2 applies per resolving effect: an effect activated FROM the pool
/// is itself "a single rule or effect", so the triggers raised while IT
/// resolves trigger simultaneously with each other -- a new pool -- not with
/// the entries still pending from the first one; and being derived they
/// activate before those (§15-4-5-2 "A derived triggering effect will activate
/// before previously triggered effects that are pending activation.").
///
/// As above with `OPT_OBS_OPP_DISCARD` ("[Once Per Turn] When effects trash
/// cards from under your Tamers, your opponent trashes 1 card in their hand")
/// in place of the drawing observer: the pool is [`OPT_OBS_OPP_DISCARD`, the
/// Tamer twice]. Activating `OPT_OBS_OPP_DISCARD` trashes P1's second hand
/// card, which triggers the Tamer a third time. That trigger is alone in its
/// own pool and activates at once; only then are the two earlier Tamer
/// entries offered again.
#[test]
fn triggers_raised_by_a_pooled_effect_pend_as_a_new_pool() {
    cite!(
        "15-4-3-2",
        "9806b36822f7fbe00db55ccc91fbb1cd9e2988f7064073bf57491efb6f425b57"
    );
    cite!(
        "15-4-2-4",
        "dea76fe97f7debaf2f9af69a29d70795982cfed0f27ca910ae763c21b4515663"
    );
    cite!(
        "15-4-5-2",
        "4673e5ae035f311cbd55487d1113223036236e4b3a0313d40fd3071df6a86fd5"
    );
    let mut r = fixtures::builder(&[fixtures::ch15_04_cards()])
        .hand(0, &["MAIN_TRASH_UNDER_OPP_DISCARD"])
        .hand(1, &[FILL, FILL])
        .deck(0, &[FILL; 4])
        .memory(3)
        .start();
    let tamer = r.place_on_field(0, "TAMER_TWO_EVENT_OBS_MEM", Some(0));
    attach_face_down(&mut r, tamer, FILL);
    r.place_on_field(0, "OPT_OBS_OPP_DISCARD", Some(0));
    let cp = r.event_checkpoint();

    use_option(&mut r, 0, "MAIN_TRASH_UNDER_OPP_DISCARD");
    resolve_until_trigger_order(&mut r);
    let mut pool: Vec<String> = order_choices(&r, 0)
        .iter()
        .map(|l| branch_card(l).to_string())
        .collect();
    pool.sort();
    assert_eq!(
        pool,
        owned(&[
            "OPT_OBS_OPP_DISCARD",
            "TAMER_TWO_EVENT_OBS_MEM",
            "TAMER_TWO_EVENT_OBS_MEM"
        ]),
        "the first effect's two events: one pool"
    );

    choose(&mut r, "OPT_OBS_OPP_DISCARD");
    pick_hand_card(&mut r, 1, FILL);
    assert!(hand_ids(&r, 1).is_empty(), "P1 trashed its second card");
    assert_eq!(
        memory_sources(&r, cp),
        vec![(0, "TAMER_TWO_EVENT_OBS_MEM".to_string())],
        "the derived trigger activated first, alone in its own pool"
    );
    let rest = order_choices(&r, 0);
    assert_eq!(
        rest.iter().map(|l| branch_card(l)).collect::<Vec<_>>(),
        vec!["TAMER_TWO_EVENT_OBS_MEM", "TAMER_TWO_EVENT_OBS_MEM"],
        "the first pool's two Tamer entries are still pending, not merged with the derived one: {rest:?}"
    );
    choose(&mut r, "TAMER_TWO_EVENT_OBS_MEM");
    resolve_prompts(&mut r);
    assert_eq!(
        memory_sources(&r, cp).len(),
        3,
        "the Tamer's effect activated once per event"
    );
}

/// Put `card_id` on top of `player`'s trash.
fn push_trash_card(r: &mut DebugRunner, player: u8, card_id: &str) {
    let data_index = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"));
    let index = r.game.next_card_index();
    let card = digimon_engine::card_source::CardSource::new(data_index, player, index);
    r.game.players[player as usize].trash.push(card);
}

/// Answer the pending prompt with its first legal non-PASS option.
fn pick_first_option(r: &mut DebugRunner) {
    let sel = r.pending_selection().expect("a pending prompt");
    let player = sel.selecting_player;
    let action = sel
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != digimon_engine::action::space::PASS)
        .unwrap_or_else(|| panic!("no non-PASS option: {sel:?}"));
    r.execute_action(player, action)
        .unwrap_or_else(|e| panic!("pick rejected: {e:?}"));
}

/// §15-4-3-2 (as above) with §15-8-3-2 "Trigger-type effects can't activate
/// during the processing for a rule or effect": the effect is still being
/// processed after it resumed from a decision, so nothing it triggers
/// activates in the middle of it, and every trigger of its events pends in
/// ONE pool when it has finished.
///
/// `WD_RETURN_TRASH_PLAY_TOKEN` ("[When Digivolving] Return 1 card from your
/// trash to the bottom of your deck. Then, play 1 [Familiar] Token.", the
/// shape of BT24-017 / EX11-012's "by returning 2 cards from their trash to
/// the bottom of the deck, they play 2 [Petrification] Tokens") resumes from
/// its trash pick, adds the card to the deck (triggering
/// `OBS_OWN_DECK_ADD_MEM`, "When your effects add cards to a deck", Butenmon
/// BT26-015's shape) and then plays the Token (triggering `OBS_PLAY_LE4K`,
/// "When one of your Digimon with 4000 DP or less is played"). Neither
/// activates during the effect; they are offered together afterwards.
#[test]
fn triggers_of_a_resumed_effect_pend_in_one_pool_when_it_finishes() {
    cite!(
        "15-4-3-2",
        "9806b36822f7fbe00db55ccc91fbb1cd9e2988f7064073bf57491efb6f425b57"
    );
    cite!(
        "15-8-3-2",
        "db9514d0faa35f84c637c60f04cbbe89569aafaa1a4d579674c36b56820b69e1"
    );
    let mut r = fixtures::builder(&[fixtures::ch15_04_cards()])
        .deck(0, &[FILL; 4])
        .memory(3)
        .start();
    r.place_on_field(0, "OBS_PLAY_LE4K", Some(0));
    r.place_on_field(0, "OBS_OWN_DECK_ADD_MEM", Some(0));
    push_trash_card(&mut r, 0, FILL);
    push_trash_card(&mut r, 0, FILL);
    let carrier = r.place_on_field(0, "WD_RETURN_TRASH_PLAY_TOKEN", Some(0));
    let cp = r.event_checkpoint();

    r.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        digimon_engine::selection::TriggerSource::Permanent(carrier),
    );
    r.game.drain_effect_queue();
    pick_first_option(&mut r); // the trash card to return (the effect resumes)

    assert_eq!(
        r.game.player(0).trash.len(),
        1,
        "the effect returned 1 trash card to the deck"
    );
    assert!(
        field_top_ids(&r, 0).iter().any(|id| id == "TOKEN_FAMILIAR"),
        "the effect played the Token: {:?}",
        field_top_ids(&r, 0)
    );
    assert!(
        memory_sources(&r, cp).is_empty(),
        "nothing activated during the effect: {:?}",
        memory_sources(&r, cp)
    );
    let mut pool: Vec<String> = order_choices(&r, 0)
        .iter()
        .map(|l| branch_card(l).to_string())
        .collect();
    pool.sort();
    assert_eq!(
        pool,
        owned(&["OBS_OWN_DECK_ADD_MEM", "OBS_PLAY_LE4K"]),
        "the deck-add and Token-play triggers pend in ONE pool"
    );
    choose(&mut r, "OBS_PLAY_LE4K");
    resolve_prompts(&mut r);
    let mut gains: Vec<String> = memory_sources(&r, cp)
        .into_iter()
        .map(|(_, id)| id)
        .collect();
    gains.sort();
    assert_eq!(gains, owned(&["OBS_OWN_DECK_ADD_MEM", "OBS_PLAY_LE4K"]));
}

/// §15-4-3-3 "When an effect is triggered by a rule check, it triggers
/// simultaneously with other effects that triggered at that timing."
///
/// `MAIN_OPP_DISCARD_MINUS3000` ("[Main] Your opponent trashes 1 card in their
/// hand. Then, 1 of your opponent's Digimon gets -3000 DP for the turn.")
/// leaves P1's 1000 DP Digimon at 0 DP; the rule check after the effect
/// deletes it (§17-1-2-2). The hand trash triggered `TAMER_TWO_EVENT_OBS_MEM`
/// ("When your opponent's hand is trashed from ..., gain 1 memory") and the
/// rule-check deletion triggers `OBS_OPP_DEL_MEM` ("When one of your
/// opponent's Digimon is deleted, gain 1 memory"): ONE pool, not the rule
/// check's trigger first and the effect's after.
#[test]
fn rule_check_triggers_pend_with_the_finished_effects_triggers() {
    cite!(
        "15-4-3-3",
        "efae2fbeb8e4b5a4143c869cd417e1b33da48f07c2603d922ab1acc947401e3f"
    );
    let mut r = fixtures::builder(&[fixtures::ch15_04_cards()])
        .hand(0, &["MAIN_OPP_DISCARD_MINUS3000"])
        .hand(1, &[FILL])
        .deck(0, &[FILL; 4])
        .memory(3)
        .start();
    r.place_on_field(0, "TAMER_TWO_EVENT_OBS_MEM", Some(0));
    r.place_on_field(0, "OBS_OPP_DEL_MEM", Some(0));
    r.place_on_field(1, FILL, Some(0));
    let cp = r.event_checkpoint();

    use_option(&mut r, 0, "MAIN_OPP_DISCARD_MINUS3000");
    resolve_until_trigger_order(&mut r);

    assert!(hand_ids(&r, 1).is_empty(), "P1 trashed its hand card");
    assert!(
        field_top_ids(&r, 1).is_empty(),
        "the 0 DP Digimon was deleted by the rule check"
    );
    assert!(
        memory_sources(&r, cp).is_empty(),
        "nothing activated before the pool: {:?}",
        memory_sources(&r, cp)
    );
    let mut pool: Vec<String> = order_choices(&r, 0)
        .iter()
        .map(|l| branch_card(l).to_string())
        .collect();
    pool.sort();
    assert_eq!(
        pool,
        owned(&["OBS_OPP_DEL_MEM", "TAMER_TWO_EVENT_OBS_MEM"]),
        "the rule check's trigger pends with the effect's"
    );
    choose(&mut r, "TAMER_TWO_EVENT_OBS_MEM");
    resolve_prompts(&mut r);
    assert_eq!(memory_sources(&r, cp).len(), 2);
}
