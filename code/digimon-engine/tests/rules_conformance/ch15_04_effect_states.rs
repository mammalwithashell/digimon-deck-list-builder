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
#[ignore = "G-ENGINE-ALLIANCE-OUTSIDE-TRIGGER-POOL (§15-4-3-2, §16-24-2)"]
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
#[ignore = "G-ENGINE-WHEN-ATTACKING-PARK-DROPS-ALLY-ATTACK-OBSERVERS (§15-4-3-5-2)"]
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
