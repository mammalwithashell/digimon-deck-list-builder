//! BT19-075 MoonMillenniummon — Track E forced hand-reduction + Tamer-deletion
//! rider (result-count binding), plus the inline-fixture proof of the
//! `trash_opponent_hand_to_count { bind_count_as }` → `floor_div` mechanism.
//!
//! Printed [On Play] / [When Digivolving] (shared):
//!   Your opponent trashes cards in their hand until they have 5 left. For
//!   every 2 cards trashed by this effect, delete 1 of your opponent's Tamers.
//!
//! DCGO oracle (BT19_075.cs, OnEnterFieldAnyone blocks):
//!   - Opponent trashes (hand - 5) cards (mandatory, canNoSelect:false).
//!   - `maxDeletes = min(floor(trashed/2), opponentTamerCount)`.
//!   - The CARD'S CONTROLLER picks `maxDeletes` opponent Tamers to delete
//!     (mandatory, canNoSelect:false), only when `trashed >= 2`.
//!
//! [All Turns]: "When this Digimon would leave the battle area, by deleting 1
//! of your [Composite] trait Digimon, it doesn't leave."
//!
//! DCGO oracle (BT19_075.cs, WhenRemoveField block):
//!   - `CanActivateCondition`: carrier still an own-battle-area Digimon AND at
//!     least 1 own Composite-trait Digimon exists (payable cost target).
//!   - No `replacement_cause` exclusion, no `[Once Per Turn]` tag (trigger
//!     limit `-1` = unlimited uses/turn).
//!   - The delete-selection is `canNoSelect: true` → declining is legal
//!     (`optional: true` on the DSL clause).
//!
//! [All Turns] [Once Per Turn] (this revision): "When other Digimon or Tamers
//! are deleted, trash your opponent's top security card."
//!
//! DCGO oracle (BT19_075.cs, OnDestroyedAnyone block, hash
//! "TrashSecurity_BT19-075"):
//!   - Trigger limit 1, isOptional false → once_per_turn, mandatory.
//!   - `OtherPermanentDeleted`: the deleted permanent is a Digimon or Tamer
//!     and NOT the carrier itself — no OWNER filter (either player's
//!     deletions fire it; the printed text has no "your"/"your opponent's"
//!     qualifier).
//!   - `CanActivateCondition`: carrier on battle area AND
//!     `card.Owner.Enemy.SecurityCards.Count > 0` → 0 security = clean no-op.
//!   - Body: `IDestroySecurity(player: card.Owner.Enemy, count: 1,
//!     fromTop: true)` → `trash_top_security: { of: opponent }`.

use digimon_dsl::compiled::{
    CompiledClause, CompiledPredicate, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{
    make_test_card, make_test_card_with_level, DebugRunner, DebugRunnerBuilder,
};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT19-075";

fn digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card_with_level(id, name, level);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(dp);
    card
}

fn tamer(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card
}

fn digimon_with_traits(id: &str, name: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut card = digimon(id, name, level, dp);
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

/// Shared fixture: BT19-075 + hand filler + Tamers + Composite cost targets.
/// `runner()` starts it bare; `observer_runner()` adds decks + opp security
/// for the [All Turns][OPT] deletion-observer tests.
fn base_builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-075 YAML loads")
        // MoonMillenniummon carrier goes on P0's field.
        // Opponent hand filler + opponent Tamers.
        .add_card(digimon("HAND-A", "Hand A", 3, 1000))
        .add_card(digimon("HAND-B", "Hand B", 3, 1000))
        .add_card(digimon("HAND-C", "Hand C", 3, 1000))
        .add_card(digimon("HAND-D", "Hand D", 3, 1000))
        .add_card(digimon("HAND-E", "Hand E", 3, 1000))
        .add_card(digimon("HAND-F", "Hand F", 3, 1000))
        .add_card(digimon("HAND-G", "Hand G", 3, 1000))
        .add_card(digimon("HAND-H", "Hand H", 3, 1000))
        .add_card(digimon("HAND-I", "Hand I", 3, 1000))
        .add_card(tamer("OPP-TAMER-1", "Opp Tamer 1"))
        .add_card(tamer("OPP-TAMER-2", "Opp Tamer 2"))
        .add_card(tamer("OPP-TAMER-3", "Opp Tamer 3"))
        .add_card(digimon("OWN-TAMER-DECOY", "own decoy", 3, 1000))
        .add_card(digimon_with_traits(
            "OWN-COMPOSITE-1",
            "Own Composite 1",
            4,
            2000,
            &["Composite"],
        ))
        .add_card(digimon_with_traits(
            "OWN-COMPOSITE-2",
            "Own Composite 2",
            4,
            2000,
            &["Composite"],
        ))
        .add_card(digimon_with_traits(
            "OWN-NON-COMPOSITE",
            "Own Non-Composite",
            4,
            2000,
            &["Beast"],
        ))
        .add_card(digimon_with_traits(
            "OPP-COMPOSITE",
            "Opp Composite",
            4,
            2000,
            &["Composite"],
        ))
        .memory(10)
}

fn runner() -> DebugRunner {
    base_builder().start()
}

/// Observer fixture: opponent (P1) security seeded with 3 cards, plus small
/// filler decks on BOTH sides so `end_turn()` turn-start draws don't deck
/// anyone out (needed by the OPT re-arm test).
fn observer_runner() -> DebugRunner {
    base_builder()
        .deck(0, &["HAND-D", "HAND-E", "HAND-F"])
        .deck(1, &["HAND-D", "HAND-E", "HAND-F"])
        .security(1, &["HAND-G", "HAND-H", "HAND-I"])
        .start()
}

/// Standalone accept-prompt helper for the [All Turns] leave-replacement
/// clause (mirrors BT24-018's `accept_replacement`).
fn accept_leave_replacement(runner: &mut DebugRunner) {
    let view = runner
        .pending_selection_view()
        .expect("leave-replacement accept prompt");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert!(view.is_optional, "declining the replacement must be legal");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("accept replacement");
}

fn permanent_exists(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .battle_area
        .iter()
        .any(|permanent| permanent.top_card().card_id(&runner.game.card_data) == card_id)
}

/// Re-resolve a permanent's handle by card id (indices shift after earlier
/// deletions — bt16_101 OPT-test idiom).
fn handle_of(runner: &DebugRunner, player: u8, card_id: &str) -> PermanentHandle {
    PermanentHandle {
        player,
        index: runner
            .game
            .player(player)
            .battle_area
            .iter()
            .position(|p| p.top_card().card_id(&runner.game.card_data) == card_id)
            .unwrap_or_else(|| panic!("handle_of: {card_id} not on P{player}'s field"))
            as u8,
    }
}

/// Push `n` filler cards into the opponent's (P1) hand.
fn fill_opp_hand(runner: &mut DebugRunner, ids: &[&str]) {
    for id in ids {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| &c.card_id == id)
            .unwrap_or_else(|| panic!("fill_opp_hand: unknown card {id}"));
        let next = runner.game.next_card_index();
        let card = digimon_engine::card_source::CardSource::new(data_idx, 1, next);
        runner.game.players[1].hand.push(card);
    }
}

// ───────────────────── Structural ─────────────────────────────────────────

/// All THREE printed clauses are authored: (1) the [On Play][When Digivolving]
/// trash-to-5 + Tamer-deletion rider, (2) the [All Turns] Composite leave
/// replacement, (3) the [All Turns][OPT] deletion observer.
#[test]
fn bt19_075_authors_all_three_printed_clauses() {
    let r = runner();
    let compiled = r.compiled_card(CARD_ID).expect("compiled BT19-075");
    assert_eq!(
        compiled.effects.len(),
        3,
        "BT19-075 prints three effect clauses; all three must be authored"
    );
}

fn predicate_mentions_is_source_false(p: &CompiledPredicate) -> bool {
    p.event_permanent_is_source == Some(false)
        || p.all_of.iter().any(predicate_mentions_is_source_false)
        || p.any_of.iter().any(predicate_mentions_is_source_false)
}

fn predicate_mentions_event_target_owner(p: &CompiledPredicate) -> bool {
    p.event_target_owner.is_some()
        || p.all_of.iter().any(predicate_mentions_event_target_owner)
        || p.any_of.iter().any(predicate_mentions_event_target_owner)
}

/// Exactly one OnAnyDeletion observer, [Once Per Turn], gated on the deleted
/// object being an OTHER (non-source) permanent — and with NO owner filter
/// (either player's Digimon/Tamer deletions fire it, per the printed text and
/// DCGO's `OtherPermanentDeleted`).
#[test]
fn bt19_075_has_one_once_per_turn_deletion_observer_without_owner_filter() {
    let r = runner();
    let compiled = r.compiled_card(CARD_ID).expect("compiled BT19-075");
    let del: Vec<&CompiledTriggeredClause> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnAnyDeletion) => {
                Some(t)
            }
            _ => None,
        })
        .collect();
    assert_eq!(del.len(), 1, "exactly one OnAnyDeletion observer");
    assert!(
        del[0].once_per_turn,
        "the security-trash observer is [Once Per Turn]"
    );
    let cond = del[0]
        .condition
        .as_ref()
        .expect("observer must gate on the deleted object");
    assert!(
        predicate_mentions_is_source_false(cond),
        "'other' — the carrier's own deletion must be excluded \
         (event_permanent_is_source: false)"
    );
    assert!(
        !predicate_mentions_event_target_owner(cond),
        "printed text has NO owner qualifier — either player's deletions fire \
         it; the condition must not mention event_target_owner"
    );
}

// ───────────────────── Real card behavior ─────────────────────────────────

/// Opponent has 9 hand cards → trashes 4 to reach 5 → floor(4/2)=2 Tamers
/// deleted. Opponent has 3 Tamers, so 2 of them go.
#[test]
fn bt19_075_trash_four_deletes_two_tamers() {
    let mut runner = runner();
    let moon = runner.place_on_field(0, CARD_ID, Some(0));
    // Opponent field: 3 Tamers (deletion candidates).
    runner.place_on_field(1, "OPP-TAMER-1", Some(0));
    runner.place_on_field(1, "OPP-TAMER-2", Some(0));
    runner.place_on_field(1, "OPP-TAMER-3", Some(0));
    // Opponent hand: 9 cards → must trash 4 to reach 5.
    fill_opp_hand(
        &mut runner,
        &[
            "HAND-A", "HAND-B", "HAND-C", "HAND-D", "HAND-E", "HAND-F", "HAND-G", "HAND-H",
            "HAND-I",
        ],
    );
    assert_eq!(runner.game.players[1].hand.len(), 9);
    assert_eq!(runner.battle_area_size(1), 3, "3 opponent Tamers to start");

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(moon));
    runner.game.drain_effect_queue();

    // First: the opponent trashes 4 hand cards (mandatory count-capped).
    runner
        .auto_resolve()
        .expect("resolve trash + Tamer deletion");

    assert_eq!(
        runner.game.players[1].hand.len(),
        5,
        "opponent trashes down to 5 cards"
    );
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "floor(4/2)=2 opponent Tamers deleted (3 → 1)"
    );
}

/// Opponent has 8 hand cards → trashes 3 → floor(3/2)=1 Tamer deleted.
#[test]
fn bt19_075_trash_three_deletes_one_tamer() {
    let mut runner = runner();
    let moon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-TAMER-1", Some(0));
    runner.place_on_field(1, "OPP-TAMER-2", Some(0));
    fill_opp_hand(
        &mut runner,
        &[
            "HAND-A", "HAND-B", "HAND-C", "HAND-D", "HAND-E", "HAND-F", "HAND-G", "HAND-H",
        ],
    );
    assert_eq!(runner.game.players[1].hand.len(), 8);

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(moon));
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("resolve");

    assert_eq!(runner.game.players[1].hand.len(), 5, "trashed 3 to reach 5");
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "floor(3/2)=1 Tamer deleted (2 → 1)"
    );
}

/// Opponent has 6 hand cards → trashes 1 → floor(1/2)=0 → no Tamer deleted.
#[test]
fn bt19_075_trash_one_deletes_no_tamer() {
    let mut runner = runner();
    let moon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-TAMER-1", Some(0));
    runner.place_on_field(1, "OPP-TAMER-2", Some(0));
    fill_opp_hand(
        &mut runner,
        &["HAND-A", "HAND-B", "HAND-C", "HAND-D", "HAND-E", "HAND-F"],
    );
    assert_eq!(runner.game.players[1].hand.len(), 6);

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(moon));
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("resolve");

    assert_eq!(runner.game.players[1].hand.len(), 5, "trashed 1 to reach 5");
    assert_eq!(
        runner.battle_area_size(1),
        2,
        "floor(1/2)=0 → no Tamers deleted"
    );
}

/// Deletion clamps to the number of opponent Tamers actually present:
/// trash 4 → floor(4/2)=2 wanted, but only 1 Tamer exists → delete 1.
#[test]
fn bt19_075_tamer_deletion_clamps_to_available() {
    let mut runner = runner();
    let moon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-TAMER-1", Some(0)); // only 1 Tamer
    fill_opp_hand(
        &mut runner,
        &[
            "HAND-A", "HAND-B", "HAND-C", "HAND-D", "HAND-E", "HAND-F", "HAND-G", "HAND-H",
            "HAND-I",
        ],
    );

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(moon));
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("resolve");

    assert_eq!(runner.game.players[1].hand.len(), 5, "trashed 4 to reach 5");
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "wanted 2 deletes but only 1 Tamer present → delete the 1"
    );
}

/// Opponent Digimon are NOT deletion candidates (only Tamers). trash 4 → 2
/// wanted, but the only opponent permanent is a Digimon → nothing deleted.
#[test]
fn bt19_075_only_deletes_tamers_not_digimon() {
    let mut runner = runner();
    let moon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "HAND-A", Some(0)); // an opponent Digimon on field
    fill_opp_hand(
        &mut runner,
        &[
            "HAND-A", "HAND-B", "HAND-C", "HAND-D", "HAND-E", "HAND-F", "HAND-G", "HAND-H",
            "HAND-I",
        ],
    );

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(moon));
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("resolve");

    assert_eq!(runner.game.players[1].hand.len(), 5);
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "the opponent Digimon is not a Tamer → not deleted"
    );
}

/// No-op path: opponent hand already ≤ 5 → no trashing, no Tamer deletion,
/// no lingering selection.
#[test]
fn bt19_075_noop_when_hand_at_or_below_five() {
    let mut runner = runner();
    let moon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-TAMER-1", Some(0));
    runner.place_on_field(1, "OPP-TAMER-2", Some(0));
    fill_opp_hand(&mut runner, &["HAND-A", "HAND-B", "HAND-C"]); // only 3 in hand

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(moon));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "hand already ≤ 5 → no trash selection installs"
    );
    assert_eq!(runner.game.players[1].hand.len(), 3, "hand untouched");
    assert_eq!(runner.battle_area_size(1), 2, "no Tamers deleted");
}

/// [When Digivolving] fires the same clause.
#[test]
fn bt19_075_when_digivolving_also_trashes_and_deletes() {
    let mut runner = runner();
    let moon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-TAMER-1", Some(0));
    runner.place_on_field(1, "OPP-TAMER-2", Some(0));
    fill_opp_hand(
        &mut runner,
        &[
            "HAND-A", "HAND-B", "HAND-C", "HAND-D", "HAND-E", "HAND-F", "HAND-G",
        ],
    ); // 7 → trash 2 → floor(2/2)=1

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(moon),
    );
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("resolve");

    assert_eq!(runner.game.players[1].hand.len(), 5, "trashed 2 to reach 5");
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "floor(2/2)=1 Tamer deleted (2 → 1)"
    );
}

// ───────────────────── [All Turns] Composite leave replacement ─────────────

/// MoonMillenniummon would leave the battle area; controller owns 1 Composite
/// Digimon → replacement prompt installs, accepting deletes the Composite
/// Digimon and MoonMillenniummon stays.
#[test]
fn bt19_075_composite_cost_prevents_leaving_battle_area() {
    let mut runner = runner();
    let moon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "OWN-COMPOSITE-1", Some(0));

    runner
        .game
        .delete_permanent_with_cause(moon, ReplacementCause::OpponentEffect);

    accept_leave_replacement(&mut runner);
    let cost = runner
        .pending_selection_view()
        .expect("Composite cost prompt");
    assert_eq!(cost.kind, SelectionKind::OwnField);
    assert_eq!(
        cost.valid_action_ids.len(),
        1,
        "only 1 Composite Digimon owned"
    );
    runner
        .execute_action(0, cost.valid_action_ids[0])
        .expect("delete own Composite Digimon");
    runner.auto_resolve().expect("finish replacement");

    assert!(
        permanent_exists(&runner, 0, CARD_ID),
        "MoonMillenniummon stays after the Composite cost is paid"
    );
    assert!(
        !permanent_exists(&runner, 0, "OWN-COMPOSITE-1"),
        "the Composite Digimon paid as the cost is deleted"
    );
}

/// Declining the replacement prompt lets MoonMillenniummon leave normally.
#[test]
fn bt19_075_decline_leave_replacement_moon_leaves_normally() {
    let mut runner = runner();
    let moon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "OWN-COMPOSITE-1", Some(0));

    runner
        .game
        .delete_permanent_with_cause(moon, ReplacementCause::OpponentEffect);

    let view = runner
        .pending_selection_view()
        .expect("leave-replacement prompt");
    assert_eq!(view.kind, SelectionKind::Replacement);
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline replacement");
    runner.auto_resolve().expect("finish decline");

    assert!(
        !permanent_exists(&runner, 0, CARD_ID),
        "declining lets MoonMillenniummon leave as normal"
    );
    assert!(
        permanent_exists(&runner, 0, "OWN-COMPOSITE-1"),
        "the Composite Digimon is untouched when the cost isn't paid"
    );
}

/// With NO own Composite Digimon on the field, the cost is unpayable and the
/// replacement prompt must not install — MoonMillenniummon leaves outright.
#[test]
fn bt19_075_no_composite_no_replacement_prompt() {
    let mut runner = runner();
    let moon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "OWN-NON-COMPOSITE", Some(0));

    runner
        .game
        .delete_permanent_with_cause(moon, ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_selection_view().is_none(),
        "unpayable Composite cost must not install a replacement prompt"
    );
    assert!(
        !permanent_exists(&runner, 0, CARD_ID),
        "MoonMillenniummon leaves outright with no Composite Digimon to pay the cost"
    );
    assert!(permanent_exists(&runner, 0, "OWN-NON-COMPOSITE"));
}

/// The cost pool only offers own Composite Digimon, not other own Digimon
/// (e.g. a Beast-trait Digimon must not be offered as a cost target).
#[test]
fn bt19_075_cost_offers_only_composite_not_other_own_digimon() {
    let mut runner = runner();
    let moon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "OWN-COMPOSITE-1", Some(0));
    runner.place_on_field(0, "OWN-NON-COMPOSITE", Some(0));

    runner
        .game
        .delete_permanent_with_cause(moon, ReplacementCause::OpponentEffect);

    accept_leave_replacement(&mut runner);
    let cost = runner
        .pending_selection_view()
        .expect("Composite cost prompt");
    assert_eq!(
        cost.valid_action_ids.len(),
        1,
        "only the Composite Digimon may be offered as the cost, not the Beast"
    );
    runner
        .execute_action(0, cost.valid_action_ids[0])
        .expect("delete the Composite Digimon");
    runner.auto_resolve().expect("finish replacement");

    assert!(permanent_exists(&runner, 0, CARD_ID));
    assert!(!permanent_exists(&runner, 0, "OWN-COMPOSITE-1"));
    assert!(
        permanent_exists(&runner, 0, "OWN-NON-COMPOSITE"),
        "the non-Composite own Digimon must be untouched"
    );
}

/// With 2 own Composite Digimon, the controller picks which one to delete —
/// no auto-pick.
#[test]
fn bt19_075_cost_offers_all_own_composite_no_auto_pick() {
    let mut runner = runner();
    let moon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "OWN-COMPOSITE-1", Some(0));
    runner.place_on_field(0, "OWN-COMPOSITE-2", Some(0));

    runner
        .game
        .delete_permanent_with_cause(moon, ReplacementCause::OpponentEffect);

    accept_leave_replacement(&mut runner);
    let cost = runner
        .pending_selection_view()
        .expect("Composite cost prompt");
    assert_eq!(
        cost.valid_action_ids.len(),
        2,
        "both own Composite Digimon must be offered; controller picks, no auto-pick"
    );
    runner
        .execute_action(0, cost.valid_action_ids[0])
        .expect("delete one Composite Digimon");
    runner.auto_resolve().expect("finish replacement");

    assert!(permanent_exists(&runner, 0, CARD_ID));
    assert_eq!(
        runner.battle_area_size(0),
        2,
        "MoonMillenniummon + exactly 1 remaining Composite Digimon"
    );
}

/// This replacement clause is card-local ("this Digimon", not "any of your
/// [X] Digimon") — it must NOT fire when a different own Digimon would leave,
/// even though the controller owns Composite Digimon to pay with.
#[test]
fn bt19_075_does_not_fire_for_a_different_own_digimon_leaving() {
    let mut runner = runner();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "OWN-COMPOSITE-1", Some(0));
    let decoy = runner.place_on_field(0, "OWN-TAMER-DECOY", Some(0));

    runner
        .game
        .delete_permanent_with_cause(decoy, ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_selection_view().is_none(),
        "the clause only protects MoonMillenniummon itself, not other own Digimon"
    );
    assert!(!permanent_exists(&runner, 0, "OWN-TAMER-DECOY"));
    assert!(
        permanent_exists(&runner, 0, "OWN-COMPOSITE-1"),
        "untouched — the decoy's own leave never offered this cost"
    );
}

/// The opponent's own copy of MoonMillenniummon carries the SAME printed
/// ability (every copy of the card does) — so when IT would leave, its own
/// controller (P1) is offered the replacement, payable only with P1's OWN
/// Composite Digimon. This is the correct "your Digimon" scoping — each
/// side's clause is self-contained and never reads the other controller's
/// board. Regression target: a `replacement_subject_is_mine` / `is_source`
/// mis-lowering that let P0's clause read a P1-side leave, or offered P0's
/// Composite Digimon as P1's cost.
#[test]
fn bt19_075_opponent_copy_leaving_is_offered_to_its_own_controller_only() {
    let mut runner = runner();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "OWN-COMPOSITE-1", Some(0));
    let opp_moon = runner.place_on_field(1, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-COMPOSITE", Some(0));

    runner
        .game
        .delete_permanent_with_cause(opp_moon, ReplacementCause::OpponentEffect);

    let view = runner
        .pending_selection_view()
        .expect("the opponent's own copy protects itself with the opponent's own clause");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert_eq!(
        view.selecting_player, 1,
        "the accept/decline choice belongs to P1 (opp_moon's own controller), not P0"
    );
    runner
        .execute_action(1, view.valid_action_ids[0])
        .expect("P1 accepts its own replacement");

    let cost = runner
        .pending_selection_view()
        .expect("P1's own Composite cost prompt");
    assert_eq!(
        cost.selecting_player, 1,
        "the cost-payment choice also belongs to P1"
    );
    assert_eq!(
        cost.valid_action_ids.len(),
        1,
        "only P1's own Composite Digimon (OPP-COMPOSITE) may be offered, never P0's"
    );
    runner
        .execute_action(1, cost.valid_action_ids[0])
        .expect("P1 pays with its own Composite Digimon");
    runner.auto_resolve().expect("finish replacement");

    assert!(
        permanent_exists(&runner, 1, CARD_ID),
        "the opponent's MoonMillenniummon stays after paying with its own Composite Digimon"
    );
    assert!(!permanent_exists(&runner, 1, "OPP-COMPOSITE"));
    assert!(
        permanent_exists(&runner, 0, "OWN-COMPOSITE-1"),
        "P0's own Composite Digimon must be untouched by P1's replacement"
    );
}

/// No [Once Per Turn] tag on this clause — it must be able to fire twice in
/// the same turn if MoonMillenniummon is forced to leave, saved, then forced
/// to leave again (paying with a second Composite Digimon).
#[test]
fn bt19_075_no_once_per_turn_lockout_fires_twice_same_turn() {
    let mut runner = runner();
    let moon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "OWN-COMPOSITE-1", Some(0));
    runner.place_on_field(0, "OWN-COMPOSITE-2", Some(0));

    // First forced leave: pay with OWN-COMPOSITE-1 (or whichever offered
    // first), MoonMillenniummon stays.
    runner
        .game
        .delete_permanent_with_cause(moon, ReplacementCause::OpponentEffect);
    accept_leave_replacement(&mut runner);
    let cost1 = runner
        .pending_selection_view()
        .expect("first Composite cost prompt");
    runner
        .execute_action(0, cost1.valid_action_ids[0])
        .expect("pay first Composite cost");
    runner.auto_resolve().expect("finish first replacement");
    assert!(permanent_exists(&runner, 0, CARD_ID));
    assert_eq!(
        runner.battle_area_size(0),
        2,
        "MoonMillenniummon + 1 remaining Composite Digimon"
    );

    // Second forced leave in the SAME turn: still no [Once Per Turn] lockout,
    // so the prompt installs again and can be paid with the remaining
    // Composite Digimon.
    runner
        .game
        .delete_permanent_with_cause(moon, ReplacementCause::OpponentEffect);
    accept_leave_replacement(&mut runner);
    let cost2 = runner
        .pending_selection_view()
        .expect("second Composite cost prompt — no OPT lockout on this clause");
    runner
        .execute_action(0, cost2.valid_action_ids[0])
        .expect("pay second Composite cost");
    runner.auto_resolve().expect("finish second replacement");

    assert!(
        permanent_exists(&runner, 0, CARD_ID),
        "MoonMillenniummon stays after the second same-turn replacement"
    );
    assert_eq!(
        runner.battle_area_size(0),
        1,
        "MoonMillenniummon only — both Composite Digimon spent"
    );
}

// ───────────────── [All Turns][OPT] deletion observer — security trash ──────

/// POSITIVE: an OTHER Digimon (the opponent's) is deleted → the opponent's
/// top security card is trashed (3 → 2).
#[test]
fn bt19_075_other_digimon_deleted_trashes_opp_top_security() {
    let mut runner = observer_runner();
    let _moon = runner.place_on_field(0, CARD_ID, Some(0));
    let opp_digimon = runner.place_on_field(1, "HAND-A", Some(0));
    assert_eq!(runner.security_count(1), 3, "precondition: 3 opp security");

    runner
        .game
        .delete_permanent_with_cause(opp_digimon, ReplacementCause::Battle);
    let _ = runner.auto_resolve();

    assert!(!permanent_exists(&runner, 1, "HAND-A"));
    assert_eq!(
        runner.security_count(1),
        2,
        "another Digimon's deletion must trash the opponent's top security card"
    );
}

/// POSITIVE (no owner filter): P0's OWN other Digimon being deleted also
/// fires the observer — the printed text has no "your"/"your opponent's"
/// qualifier (DCGO `OtherPermanentDeleted` checks kind + not-self only).
#[test]
fn bt19_075_own_other_digimon_deleted_also_trashes_opp_security() {
    let mut runner = observer_runner();
    let _moon = runner.place_on_field(0, CARD_ID, Some(0));
    let own_other = runner.place_on_field(0, "OWN-NON-COMPOSITE", Some(0));
    assert_eq!(runner.security_count(1), 3);

    runner
        .game
        .delete_permanent_with_cause(own_other, ReplacementCause::Battle);
    let _ = runner.auto_resolve();

    assert!(!permanent_exists(&runner, 0, "OWN-NON-COMPOSITE"));
    assert_eq!(
        runner.security_count(1),
        2,
        "either player's Digimon deletion fires the observer (no owner filter)"
    );
}

/// POSITIVE: an OTHER Tamer deleted → same security trash.
#[test]
fn bt19_075_other_tamer_deleted_trashes_opp_top_security() {
    let mut runner = observer_runner();
    let _moon = runner.place_on_field(0, CARD_ID, Some(0));
    let opp_tamer = runner.place_on_field(1, "OPP-TAMER-1", Some(0));
    assert_eq!(runner.security_count(1), 3);

    runner
        .game
        .delete_permanent_with_cause(opp_tamer, ReplacementCause::OpponentEffect);
    let _ = runner.auto_resolve();

    assert!(!permanent_exists(&runner, 1, "OPP-TAMER-1"));
    assert_eq!(
        runner.security_count(1),
        2,
        "a Tamer's deletion must trash the opponent's top security card"
    );
}

/// NEGATIVE ("other"): the carrier ITSELF being deleted must NOT fire its own
/// observer. (No Composite Digimon on P0's field, so the leave replacement is
/// unpayable and MoonMillenniummon leaves outright.)
#[test]
fn bt19_075_carrier_own_deletion_does_not_fire_observer() {
    let mut runner = observer_runner();
    let moon = runner.place_on_field(0, CARD_ID, Some(0));
    assert_eq!(runner.security_count(1), 3);

    runner
        .game
        .delete_permanent_with_cause(moon, ReplacementCause::OpponentEffect);
    let _ = runner.auto_resolve();

    assert!(
        !permanent_exists(&runner, 0, CARD_ID),
        "precondition: MoonMillenniummon left (no Composite to pay the leave cost)"
    );
    assert_eq!(
        runner.security_count(1),
        3,
        "'other Digimon or Tamers' — the carrier's own deletion must NOT trash security"
    );
}

/// OPT LOCKOUT: a second qualifying deletion the SAME turn must not trash a
/// second security card (DCGO trigger limit 1 on hash "TrashSecurity_BT19-075").
#[test]
fn bt19_075_opt_locks_second_deletion_same_turn() {
    let mut runner = observer_runner();
    let _moon = runner.place_on_field(0, CARD_ID, Some(0));
    let opp_a = runner.place_on_field(1, "HAND-A", Some(0));
    runner.place_on_field(1, "HAND-B", Some(0));
    assert_eq!(runner.security_count(1), 3);

    // First deletion this turn → trash 1 (3 → 2).
    runner
        .game
        .delete_permanent_with_cause(opp_a, ReplacementCause::Battle);
    let _ = runner.auto_resolve();
    assert_eq!(runner.security_count(1), 2, "first deletion trashes 1");

    // Second deletion the SAME turn → OPT lockout, still 2.
    let opp_b = handle_of(&runner, 1, "HAND-B"); // index shifted after HAND-A left
    runner
        .game
        .delete_permanent_with_cause(opp_b, ReplacementCause::Battle);
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.security_count(1),
        2,
        "OPT lockout: a second same-turn deletion must NOT trash another security card"
    );
}

/// OPT RE-ARM (controller's next turn): after the turn cycles back to the
/// carrier controller (two `end_turn()`s), the [Once Per Turn] re-arms and a
/// new deletion trashes again. (The engine clears `effect_activations` on
/// `Permanent::new_turn`, called at the CARRIER CONTROLLER's begin-turn.)
#[test]
fn bt19_075_opt_rearms_by_controllers_next_turn() {
    let mut runner = observer_runner();
    let _moon = runner.place_on_field(0, CARD_ID, Some(0));
    let opp_a = runner.place_on_field(1, "HAND-A", Some(0));
    runner.place_on_field(1, "HAND-B", Some(0));
    assert_eq!(runner.security_count(1), 3);

    // Consume the OPT on P0's turn 1.
    runner
        .game
        .delete_permanent_with_cause(opp_a, ReplacementCause::Battle);
    let _ = runner.auto_resolve();
    assert_eq!(runner.security_count(1), 2, "first deletion trashes 1");

    // Cycle the turn back to P0 (turn 2 = P1's, turn 3 = P0's).
    runner.end_turn();
    runner.end_turn();

    let opp_b = handle_of(&runner, 1, "HAND-B");
    runner
        .game
        .delete_permanent_with_cause(opp_b, ReplacementCause::Battle);
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.security_count(1),
        1,
        "on the controller's next turn the [Once Per Turn] has re-armed (2 → 1)"
    );
}

/// OPT RE-ARM (opponent's next turn — rules-faithful): [Once Per Turn] is per
/// TURN, not per "your turn" (rule 15-14-1-2: used X times during 1 turn → it
/// won't trigger again during THAT turn), and this observer is [All Turns].
/// DCGO resets every card's per-turn use count at EVERY end phase
/// (`TurnStateMachine` end phase → `InitUseCountThisTurn()` over
/// `gameContext.ActiveCardList` — both players' cards). So an OPT consumed on
/// your turn must be usable again on the opponent's very next turn.
///
/// The Rust engine only clears `Permanent::effect_activations` for the NEW
/// TURN PLAYER's permanents (`game_phases.rs` `begin_turn` →
/// `self.player_mut(tp).new_turn()`), so the non-turn-player's [All Turns][OPT]
/// carriers stay locked through the opponent's turn — one full turn later than
/// the rules/DCGO allow. See G-OPT-RESET-ONLY-TURN-PLAYER in
/// docs/RUST_ENGINE_GAPS.md.
#[test]
#[ignore = "pending: G-OPT-RESET-ONLY-TURN-PLAYER — begin_turn resets effect_activations only for the turn player's permanents, so an [All Turns][OPT] observer consumed on your turn stays locked through the opponent's next turn (rule 15-14-1-2 + DCGO InitUseCountThisTurn reset ALL cards each turn)"]
fn bt19_075_opt_rearms_on_opponents_next_turn() {
    let mut runner = observer_runner();
    let _moon = runner.place_on_field(0, CARD_ID, Some(0));
    let opp_a = runner.place_on_field(1, "HAND-A", Some(0));
    runner.place_on_field(1, "HAND-B", Some(0));
    assert_eq!(runner.security_count(1), 3);

    // Consume the OPT on P0's turn 1.
    runner
        .game
        .delete_permanent_with_cause(opp_a, ReplacementCause::Battle);
    let _ = runner.auto_resolve();
    assert_eq!(runner.security_count(1), 2, "first deletion trashes 1");

    // One end_turn → P1's turn. Per rules/DCGO the OPT must already have
    // re-armed ([All Turns] observer + fresh turn).
    runner.end_turn();
    let opp_b = handle_of(&runner, 1, "HAND-B");
    runner
        .game
        .delete_permanent_with_cause(opp_b, ReplacementCause::Battle);
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.security_count(1),
        1,
        "a new turn began (opponent's) → the [Once Per Turn] must re-arm (2 → 1)"
    );
}

/// NEGATIVE (0 security): with the opponent's security stack empty, a
/// qualifying deletion is a clean no-op — no crash, no lingering selection
/// (DCGO CanActivateCondition requires SecurityCards.Count > 0).
#[test]
fn bt19_075_zero_opp_security_clean_noop() {
    let mut runner = runner(); // base fixture: NO security seeded
    let _moon = runner.place_on_field(0, CARD_ID, Some(0));
    let opp_digimon = runner.place_on_field(1, "HAND-A", Some(0));
    assert_eq!(runner.security_count(1), 0, "precondition: 0 opp security");

    runner
        .game
        .delete_permanent_with_cause(opp_digimon, ReplacementCause::Battle);
    let _ = runner.auto_resolve();

    assert!(!permanent_exists(&runner, 1, "HAND-A"));
    assert_eq!(
        runner.security_count(1),
        0,
        "nothing to trash — clean no-op"
    );
    assert!(
        runner.pending_selection().is_none(),
        "no lingering selection after the 0-security no-op"
    );
}
