//! BT20-056 Alphamon
//!
//! Printed text (card image / cards.json — authoritative):
//!   <Barrier>
//!   [On Play][When Digivolving] <Recovery +1 (Deck)>. Then, if during an
//!     attack, 1 of your Digimon in the breeding area may digivolve into a
//!     level 6 or lower [Chronicle] trait Digimon card in the hand or trash
//!     without paying the cost.
//!   [All Turns][Once Per Turn] When security stacks are removed from, 1 of
//!     your opponent's Digimon gets -8000 DP for the turn.
//!   [Inherited][All Turns][Once Per Turn] When this Digimon would leave the
//!     battle area other than by your effects, if this Digimon is
//!     [Alphamon: Ouryuken], by trashing your top security card, it doesn't
//!     leave.
//!
//! DCGO reference (READ-ONLY): DCGO/Assets/Scripts/CardEffect/BT20/Black/BT20_056.cs

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::CardKind;
use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

// ────────────────────────────── helpers ──────────────────────────────

fn make_digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card
}

fn permanent_exists(runner: &DebugRunner, player: usize, id: &str) -> bool {
    runner.game.players[player]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == id)
}

fn security_ids(runner: &DebugRunner, player: usize) -> Vec<&str> {
    runner.game.players[player]
        .security
        .iter()
        .map(|card| card.card_id(&runner.game.card_data))
        .collect()
}

/// Remove the top card of `player`'s security stack via an engine effect, which
/// is the trigger condition for the `on_own/opponent_security_removed`
/// observers. Uses a throwaway EffectContext sourced from the BT20-056
/// permanent (same pattern as BT24-101's security-loss test).
fn trash_top_security_via_effect(runner: &mut DebugRunner, source: digimon_engine::permanent::PermanentHandle, owner: u8) {
    let source_card = runner.game.players[source.player as usize].battle_area[source.index as usize]
        .top_card()
        .handle();
    let mut ctx = EffectContext::new(&mut runner.game, source_card, None, source.player);
    assert!(ctx.trash_top_security(owner));
}

// ───────────────────────────── slice loads ───────────────────────────

#[test]
fn bt20_056_loads_barrier_recovery_slice() {
    DebugRunner::builder()
        .dsl_card("BT20-056")
        .expect("BT20-056 must load from embedded DSL pack")
        .start();
}

// ─────────────── [All Turns][OPT] security-removed -8000 DP ───────────────

#[test]
fn bt20_056_security_removed_debuffs_one_opponent_digimon_by_8000() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-056")
        .expect("BT20-056 YAML loads")
        .add_card(make_digimon("OPP-A", "Opponent A", 6, 12000))
        .add_card(make_test_card("MY-SEC-A", "My Security A"))
        .add_card(make_test_card("MY-SEC-B", "My Security B"))
        .security(0, &["MY-SEC-A", "MY-SEC-B"])
        .start();
    let alphamon = runner.place_on_field(0, "BT20-056", Some(0));
    let target = runner.place_on_field(1, "OPP-A", Some(0));

    // Removing a card from *your own* security triggers the observer.
    trash_top_security_via_effect(&mut runner, alphamon, 0);

    let view = runner
        .pending_selection_view()
        .expect("security-removed observer should ask to pick an opponent Digimon");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pick the opponent Digimon");
    runner.auto_resolve().expect("finish -8000 DP debuff");

    assert_eq!(
        runner.dp_of(target),
        Some(4000),
        "the chosen opponent Digimon should be at 12000 - 8000 = 4000 DP"
    );
}

#[test]
fn bt20_056_security_removed_debuff_fires_once_across_both_stacks() {
    // The [All Turns][OPT] clause is keyed on BOTH own- and opponent-security
    // removal, sharing a single once-per-turn lockout. Removing from each stack
    // the same turn must produce exactly ONE -8000 debuff prompt.
    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-056")
        .expect("BT20-056 YAML loads")
        .add_card(make_digimon("OPP-A", "Opponent A", 6, 12000))
        .add_card(make_test_card("MY-SEC-A", "My Security A"))
        .add_card(make_test_card("MY-SEC-B", "My Security B"))
        .add_card(make_test_card("OPP-SEC-A", "Opp Security A"))
        .add_card(make_test_card("OPP-SEC-B", "Opp Security B"))
        .security(0, &["MY-SEC-A", "MY-SEC-B"])
        .security(1, &["OPP-SEC-A", "OPP-SEC-B"])
        .start();
    let alphamon = runner.place_on_field(0, "BT20-056", Some(0));
    let target = runner.place_on_field(1, "OPP-A", Some(0));

    // First removal (own stack): observer fires, resolve the debuff.
    trash_top_security_via_effect(&mut runner, alphamon, 0);
    let view = runner
        .pending_selection_view()
        .expect("first security removal should fire the observer");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pick the opponent Digimon");
    runner.auto_resolve().expect("finish first debuff");
    assert_eq!(runner.dp_of(target), Some(4000));

    // Second removal the same turn (opponent stack) must NOT re-arm the OPT.
    trash_top_security_via_effect(&mut runner, alphamon, 1);
    assert!(
        runner.pending_selection_view().is_none(),
        "once-per-turn lockout is shared across both security-removed timings"
    );
    assert_eq!(
        runner.dp_of(target),
        Some(4000),
        "no second -8000 debuff should be applied the same turn"
    );
}

#[test]
fn bt20_056_security_removed_debuff_is_optional_when_no_opponent_digimon() {
    // With no opponent Digimon to target, the observer must not soft-lock; the
    // clause simply finds no legal target. (DCGO HasMatchConditionPermanent gate.)
    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-056")
        .expect("BT20-056 YAML loads")
        .add_card(make_test_card("MY-SEC-A", "My Security A"))
        .add_card(make_test_card("MY-SEC-B", "My Security B"))
        .security(0, &["MY-SEC-A", "MY-SEC-B"])
        .start();
    let alphamon = runner.place_on_field(0, "BT20-056", Some(0));

    trash_top_security_via_effect(&mut runner, alphamon, 0);
    // No opponent Digimon exists; resolving must not panic or strand a prompt.
    runner.auto_resolve().expect("observer resolves with no target");
    assert_eq!(runner.security_count(0), 1);
}

// ───── [Inherited][All Turns][OPT] Ouryuken trash-top-security to stay ─────

#[test]
fn bt20_056_ouryuken_inherited_replacement_prevents_leave_for_top_security() {
    // BT20-056 is a digivolution source beneath a top card named
    // "Alphamon: Ouryuken"; an opponent-effect leave is prevented by trashing
    // the controller's top security card.
    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-056")
        .expect("BT20-056 YAML loads")
        .add_card(make_digimon("OURYUKEN", "Alphamon: Ouryuken", 7, 13000))
        .add_card(make_test_card("SEC-A", "Security A"))
        .add_card(make_test_card("SEC-B", "Security B"))
        .security(0, &["SEC-A", "SEC-B"])
        .start();
    let carrier = runner.place_stack(0, &["BT20-056", "OURYUKEN"]);

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    let view = runner
        .pending_selection_view()
        .expect("inherited leave-prevention should ask to accept");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert!(runner.pending_is_optional(), "replacement may be declined");
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept Ouryuken replacement");
    runner.auto_resolve().expect("finish replacement");

    assert!(
        permanent_exists(&runner, 0, "OURYUKEN"),
        "Alphamon: Ouryuken stays after trashing top security"
    );
    assert_eq!(runner.security_count(0), 1, "top security was trashed as the cost");
    assert_eq!(security_ids(&runner, 0), vec!["SEC-A"], "the top (SEC-B) was trashed");
}

#[test]
fn bt20_056_ouryuken_inherited_replacement_declined_lets_it_leave() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-056")
        .expect("BT20-056 YAML loads")
        .add_card(make_digimon("OURYUKEN", "Alphamon: Ouryuken", 7, 13000))
        .add_card(make_test_card("SEC-A", "Security A"))
        .add_card(make_test_card("SEC-B", "Security B"))
        .security(0, &["SEC-A", "SEC-B"])
        .start();
    let carrier = runner.place_stack(0, &["BT20-056", "OURYUKEN"]);

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    let view = runner
        .pending_selection_view()
        .expect("inherited leave-prevention prompt");
    assert!(runner.pending_is_optional());
    runner
        .execute_action(0, PASS)
        .expect("decline Ouryuken replacement");

    assert!(
        !permanent_exists(&runner, 0, "OURYUKEN"),
        "declining lets Alphamon: Ouryuken leave"
    );
    assert_eq!(
        security_ids(&runner, 0),
        vec!["SEC-A", "SEC-B"],
        "declining must not trash any security"
    );
}

#[test]
fn bt20_056_ouryuken_replacement_does_not_fire_for_non_ouryuken_top() {
    // If the top card is NOT named "Alphamon: Ouryuken", the inherited
    // replacement must not offer protection (the leave proceeds).
    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-056")
        .expect("BT20-056 YAML loads")
        .add_card(make_digimon("PLAIN-ALPHA", "Alphamon", 6, 11000))
        .add_card(make_test_card("SEC-A", "Security A"))
        .add_card(make_test_card("SEC-B", "Security B"))
        .security(0, &["SEC-A", "SEC-B"])
        .start();
    let carrier = runner.place_stack(0, &["BT20-056", "PLAIN-ALPHA"]);

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_selection_view().is_none(),
        "no replacement prompt when the top card is not [Alphamon: Ouryuken]"
    );
    assert!(
        !permanent_exists(&runner, 0, "PLAIN-ALPHA"),
        "a non-Ouryuken top card is not protected and leaves"
    );
}

#[test]
fn bt20_056_ouryuken_replacement_does_not_fire_for_own_effect_leave() {
    // "other than by your effects": an own-effect leave must NOT be preventable.
    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-056")
        .expect("BT20-056 YAML loads")
        .add_card(make_digimon("OURYUKEN", "Alphamon: Ouryuken", 7, 13000))
        .add_card(make_test_card("SEC-A", "Security A"))
        .add_card(make_test_card("SEC-B", "Security B"))
        .security(0, &["SEC-A", "SEC-B"])
        .start();
    let carrier = runner.place_stack(0, &["BT20-056", "OURYUKEN"]);

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OwnEffect);

    assert!(
        runner.pending_selection_view().is_none(),
        "own-effect leave must not offer the Ouryuken protection"
    );
    assert!(
        !permanent_exists(&runner, 0, "OURYUKEN"),
        "own-effect leave proceeds"
    );
}

// ─────────────────── Stubbed clause (genuine open gap) ───────────────────

// The [On Play][When Digivolving] "if during an attack, 1 breeding-area Digimon
// may digivolve into a level<=6 [Chronicle] Digimon from hand or trash without
// paying the cost" clause requires two genuinely-missing substrate pieces
// (gap G-BREEDING-DIGIVOLVE-UNION-ZONES, status OPEN, size L):
//   1. effect-initiated digivolve that can target the BREEDING-AREA permanent
//      (the engine hard-rejects target.index >= battle_area.len()), and
//   2. a `during_attack` predicate leaf (game.pending_attack.is_some()).
// Authoring it would require editing shared engine/DSL source, which is out of
// scope for this card-author task. Stubbed and gap-routed.
#[ignore = "pending: G-BREEDING-DIGIVOLVE-UNION-ZONES — during-attack breeding-area digivolve into a Lv<=6 [Chronicle] from hand/trash needs effect-initiated breeding digivolve + a during_attack predicate (open engine/DSL substrate)"]
#[test]
fn bt20_056_during_attack_breeding_digivolves_chronicle_from_hand_or_trash() {}
