
//! BT19-099 The Wicked God Descends! — Option, Cost 4, Purple.
//! Traits: [Wicked God].
//!
//! # Card text (data/card_bundles/BT19-099.md — official Bandai DB)
//!
//! [Main] You may play 1 [Composite] trait Digimon card from your trash with
//! the play cost reduced by 4. Then, place this card in the battle area.
//! [All Turns] When any of your Digimon with [Millenniummon] in their names
//! would leave the battle area, ＜Delay＞.
//! ・You may play 1 [Wicked God] trait Digimon card with a play cost 1 higher
//! than that Digimon from your hand or trash without paying the cost.
//!
//! Inherited (Security): [Security] Place this card in the battle area.
//!
//! Official Q&A: "You can choose 1 of your Digimon with [Millenniummon] in
//! its name that was affected by the removal processing and play 1 [Wicked
//! God] trait Digimon card with a play cost 1 higher than that Digimon."
//! — confirms "that Digimon" = the replacement's own leaving subject.
//!
//! # DCGO C# reference (resolved — base repo)
//! DCGO/Assets/Scripts/CardEffect/BT19/Purple/BT19_099.cs:
//! - Offer gate (`CanUseCondition`, lines 205-219): own Millenniummon-name
//!   permanent leaving + this Option on the battle area + Delay declarable.
//!   NO reward-candidate check — the candidate scan (lines 275-277) runs
//!   inside `ActivateCoroutine` AFTER the self-trash. Accepting with zero
//!   candidates trashes the Option and plays nothing.
//! - Order (`ActivateCoroutine`, lines 234-251): the <Delay> self-trash runs
//!   FIRST; the reward only on a successful deletion.
//! - Comparator (line 261): `permanent.TopCard.GetCostItself + 1 ==
//!   candidate.GetCostItself` against the PRE-CAPTURED trigger-hashtable
//!   subject (a snapshot, not a live post-deletion lookup).
//! - No Cancel on the WhenRemoveField event: the leaving Digimon still ends
//!   in the trash.
//!
//! # Patterns this test file covers
//!
//! - Clause A (Main): `select_trash` (optional) + `play_from_trash` with
//!   `cost_delta: { reduce: 4 }` + mandatory trailing
//!   `place_self_as_delay_option` (DCGO places unconditionally).
//! - Clause B (All Turns leave-watcher): `kind: replacement` with
//!   `trigger: when_would_leave_battle_area`, lowered onto the recognised
//!   `DelaySelfTrashPlayFromUnionFlow` (lower_replacement.rs):
//!   `delete_permanent { target: source }` → `select_union_zone` (filter
//!   includes `play_cost_eq_binding { binding: replacement_subject, offset:
//!   1, op: eq }`, G-DSL-COST-RELATIVE-TO-EVENT-SUBJECT) →
//!   `play_union_bound_free`. The flow owns the leave (the subject still
//!   leaves — G-ENGINE-DELAY-SELF-TRASH-THEN-FREE-PLAY-IN-LEAVE-REPLACEMENT,
//!   RESOLVED 2026-07-04) and evaluates the comparator against the subject's
//!   pre-leave top-card snapshot.
//! - Clause C (Security inherited): bare mandatory `place_self_as_delay_option`.

#![allow(unused_imports, dead_code)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{PASS, PLAY_HAND_START, REPLACEMENT_ACCEPT, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, UnionZoneSet};

const BT19_099_YAML: &str = include_str!("../../../cards/bt19/BT19-099.yaml");

// ─── Helper cards ────────────────────────────────────────────────────────────

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A [Composite]-trait Digimon eligible for Clause A's trash-play. Cost 9 so
/// the -4 reduction lands on a non-trivial value (paid to 5).
fn make_composite_digimon(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(9000);
    c.play_cost = 9;
    c.colors = vec![CardColor::Purple];
    c.traits = vec!["Composite".to_string()];
    c
}

/// A non-[Composite] Digimon (negative test for Clause A's trait filter).
fn make_non_composite_digimon(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(9000);
    c.play_cost = 9;
    c.colors = vec![CardColor::Purple];
    c.traits = vec![];
    c
}

/// A Millenniummon-name Digimon at the given play cost — the Clause B leave
/// watcher's subject.
fn make_millenniummon_named(id: &str, play_cost: u16) -> CardData {
    let mut c = make_test_card(id, "Millenniummon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(12000);
    c.play_cost = play_cost;
    c.colors = vec![CardColor::Purple];
    c
}

/// A non-Millenniummon Digimon (negative test for Clause B's name filter).
fn make_non_millenniummon_named(id: &str, play_cost: u16) -> CardData {
    let mut c = make_test_card(id, "OtherWickedThing");
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(12000);
    c.play_cost = play_cost;
    c.colors = vec![CardColor::Purple];
    c
}

/// A [Wicked God]-trait Digimon at the given play cost — Clause B's reward
/// candidate.
fn make_wicked_god_digimon(id: &str, name: &str, play_cost: u16) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(11000);
    c.play_cost = play_cost;
    c.colors = vec![CardColor::Purple];
    c.traits = vec!["Wicked God".to_string()];
    c
}

/// A Digimon WITHOUT [Wicked God] at the reward's exact play cost (negative
/// test for Clause B's trait filter).
fn make_non_wicked_god_at_cost(id: &str, play_cost: u16) -> CardData {
    let mut c = make_test_card(id, "PlainDigimon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(11000);
    c.play_cost = play_cost;
    c.colors = vec![CardColor::Purple];
    c.traits = vec![];
    c
}

fn bt19_099_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(BT19_099_YAML)
        .expect("BT19-099 YAML must parse")
        .memory(10)
        .start()
}

/// Seat the BT19-099 option permanent at `handle` as a Delay-Option so its
/// Clause B `when_would_leave_battle_area` replacement can fire — mirrors
/// ST20-14 / BT17-095's `seat_as_delay_option` helper. `place_on_field`
/// yields a Standard option, so the delay state must be set directly here.
fn seat_as_delay_option(
    runner: &mut DebugRunner,
    handle: digimon_engine::permanent::PermanentHandle,
) {
    use digimon_engine::permanent::OptionState;
    let turn = runner.game.turn_count;
    let perm = &mut runner.game.players[handle.player as usize].battle_area[handle.index as usize];
    perm.option_state = OptionState::Delayed {
        owner: handle.player,
        trash_on_turn: turn + 2,
        trigger: digimon_engine::enums::DelayTrigger::EndOfYourNextTurn,
        placed_on_turn: turn,
    };
}

/// Seed `card_id`'s CardData into `player`'s trash. The card must already be
/// registered via `.add_card`.
fn seed_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|d| d.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} card data registered"));
    let card_index = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(data_index, player, card_index));
}

fn on_field(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == card_id)
}

fn in_trash(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner.game.players[player as usize]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == card_id)
}

fn in_hand(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner.game.players[player as usize]
        .hand
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == card_id)
}

/// Drive every installed selection to its FIRST non-PASS action (or PASS if
/// none exists), draining the effect queue between steps. Used for smoke
/// paths that don't need to assert a specific branch.
fn drive_to_completion(runner: &mut DebugRunner, max_steps: usize) {
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < max_steps {
        let (player, action) = {
            let sel = runner.game.pending_selection.as_ref().unwrap();
            let action = sel
                .valid_action_ids
                .iter()
                .copied()
                .find(|&a| a != PASS)
                .unwrap_or(sel.valid_action_ids[0]);
            (sel.selecting_player, action)
        };
        let _ = runner.game.resolve_selection(player, action);
        runner.game.drain_effect_queue();
        steps += 1;
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

/// YAML must parse and compile without error.
#[test]
fn bt19_099_yaml_parses_without_error() {
    let _runner = bt19_099_runner();
}

/// Three compiled clauses: Main (triggered), All-Turns (declarative
/// replacement), Security (triggered, inherited scope).
#[test]
fn bt19_099_has_three_clauses() {
    let runner = bt19_099_runner();
    let compiled = runner
        .compiled_card("BT19-099")
        .expect("BT19-099 must be in compiled_cards");

    assert_eq!(
        compiled.effects.len(),
        3,
        "BT19-099 must have exactly 3 compiled clauses (Main, Replacement, Security); got {}",
        compiled.effects.len()
    );
}

/// Clause A: triggered with `main_from_hand` timing, optional, FaceUp scope.
#[test]
fn bt19_099_main_clause_is_optional_face_up() {
    let runner = bt19_099_runner();
    let compiled = runner
        .compiled_card("BT19-099")
        .expect("BT19-099 must be in compiled_cards");

    let main_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::MainFromHand))
        .expect("Main clause with MainFromHand timing must exist");

    assert!(
        main_clause.optional,
        "BT19-099 Main clause must be optional (printed: 'You may play')"
    );
    assert_eq!(
        main_clause.scope,
        CompiledScope::FaceUp,
        "BT19-099 Main clause must have FaceUp scope (Option played from hand)"
    );
}

/// Clause B is a declarative replacement (NOT a triggered clause), targeting
/// `WhenWouldLeaveBattleArea` timing, and its process compiles to EXACTLY the
/// recognised `DelaySelfTrashPlayFromUnionFlow` shape (delete-source first,
/// per DCGO's ActivateCoroutine order) — guarding against a silent
/// fall-through to the generic step runner, which cannot thread this flow
/// (G-ENGINE-DELAY-SELF-TRASH-THEN-FREE-PLAY-IN-LEAVE-REPLACEMENT).
#[test]
fn bt19_099_replacement_clause_matches_recognised_union_flow_shape() {
    let runner = bt19_099_runner();
    let compiled = runner
        .compiled_card("BT19-099")
        .expect("BT19-099 must be in compiled_cards");

    let process = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
                trigger,
                process,
                ..
            }) if trigger == "when_would_leave_battle_area" => Some(process),
            _ => None,
        })
        .expect("BT19-099 must have a declarative `when_would_leave_battle_area` replacement clause");

    assert!(
        matches!(
            &process[..],
            [
                CompiledStep::DeletePermanent { .. },
                CompiledStep::SelectUnionZone { .. },
                CompiledStep::PlayUnionBoundFree { .. },
            ]
        ),
        "Clause B's process must be [delete_permanent, select_union_zone, \
         play_union_bound_free] (the recognised DelaySelfTrashPlayFromUnionFlow \
         shape, DCGO cost-first order); got {process:?}"
    );
}

/// Clause C: triggered with `on_security` timing, NOT optional, INHERITED
/// scope (printed Security effect has no "may").
#[test]
fn bt19_099_security_clause_is_mandatory_inherited() {
    let runner = bt19_099_runner();
    let compiled = runner
        .compiled_card("BT19-099")
        .expect("BT19-099 must be in compiled_cards");

    let sec_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("Security clause with OnSecurity timing must exist");

    assert!(
        !sec_clause.optional,
        "BT19-099 Security clause must be mandatory (printed text has no 'may')"
    );
    assert_eq!(
        sec_clause.scope,
        CompiledScope::Inherited,
        "BT19-099 Security clause must have Inherited scope (rides on the option's inherited surface)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Clause A: [Main] play [Composite] from trash (cost -4)
// ═══════════════════════════════════════════════════════════════════════════

/// Activating [Main] with an eligible [Composite] Digimon in trash installs
/// a pending selection (the optional trash pick).
#[test]
fn bt19_099_main_installs_trash_selection_when_composite_present() {
    let composite = make_composite_digimon("BT19099-COMP", "Diablomon");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT19_099_YAML)
        .expect("BT19-099 YAML parses")
        .add_card(composite.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["BT19-099"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    seed_trash(&mut runner, 0, "BT19099-COMP");

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(
        fired,
        "activate_hand_main must return true for BT19-099 at hand index 0"
    );

    assert!(
        runner.game.pending_selection.is_some(),
        "BT19-099 Main must install a pending trash-pick selection"
    );
}

/// Positive: with an eligible cost-9 [Composite] Digimon in trash, accepting
/// the pick plays it at cost 9 - 4 = 5, then BT19-099 places itself as a
/// Delay-Option permanent.
#[test]
fn bt19_099_main_plays_composite_from_trash_at_reduced_cost_and_places_self() {
    let composite = make_composite_digimon("BT19099-COMP2", "Diablomon");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT19_099_YAML)
        .expect("BT19-099 YAML parses")
        .add_card(composite.clone())
        .add_card(make_filler("FILL"))
        .memory(20)
        .hand(0, &["BT19-099"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    seed_trash(&mut runner, 0, "BT19099-COMP2");

    let memory_before = runner.game.memory;

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired, "BT19-099 Main must activate");

    // Accept the optional trash pick (first non-PASS action).
    drive_to_completion(&mut runner, 10);

    assert!(
        on_field(&runner, 0, "BT19099-COMP2"),
        "the [Composite] Digimon must be played onto P0's battle area"
    );
    assert!(
        on_field(&runner, 0, "BT19-099"),
        "BT19-099 must place itself in the battle area after the trash play"
    );

    // Playing at reduced cost 5 (9 - 4) shifts memory by 5 in the paying
    // player's direction (memory model: playing swings memory toward the
    // opponent by the paid cost).
    let memory_after = runner.game.memory;
    assert_ne!(
        memory_after, memory_before,
        "memory must move — the reduced play cost (5) was actually paid"
    );
}

/// Negative: a non-[Composite] Digimon in trash is NOT offered by Clause A's
/// trash-pick filter. With ZERO eligible candidates, the optional
/// `select_trash` step no-ops (no selection installs) and the step runner
/// continues to the mandatory `place_self_as_delay_option` tail — matching
/// DCGO, whose PlaceDelayOptionCards runs unconditionally at the end of the
/// Main coroutine.
#[test]
fn bt19_099_main_does_not_offer_non_composite_trash_digimon() {
    let non_comp = make_non_composite_digimon("BT19099-NONCOMP", "PlainMon");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT19_099_YAML)
        .expect("BT19-099 YAML parses")
        .add_card(non_comp.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["BT19-099"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    seed_trash(&mut runner, 0, "BT19099-NONCOMP");

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired);
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "with zero eligible [Composite] candidates, Clause A must not offer a trash pick"
    );
    assert!(
        in_trash(&runner, 0, "BT19099-NONCOMP"),
        "the non-[Composite] Digimon must not have been played"
    );
    assert!(
        on_field(&runner, 0, "BT19-099"),
        "BT19-099 must still place itself in the battle area with zero eligible trash candidates"
    );
}

/// Declining the trash pick (with an eligible card present) still runs the
/// mandatory tail — BT19-099 places itself as a Delay-Option regardless
/// (DCGO: PlaceDelayOptionCards runs unconditionally after the optional play).
#[test]
fn bt19_099_main_places_self_even_when_trash_pick_declined() {
    let composite = make_composite_digimon("BT19099-COMP3", "Diablomon");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT19_099_YAML)
        .expect("BT19-099 YAML parses")
        .add_card(composite.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["BT19-099"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    seed_trash(&mut runner, 0, "BT19099-COMP3");

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired);

    // Explicitly PASS the trash pick.
    {
        let sel = runner
            .game
            .pending_selection
            .as_ref()
            .expect("trash-pick selection installs");
        assert!(
            sel.is_optional,
            "the trash pick must be optional (printed: 'You may play')"
        );
        let player = sel.selecting_player;
        runner
            .game
            .resolve_selection(player, PASS)
            .expect("decline the trash pick");
    }
    runner.game.drain_effect_queue();

    assert!(
        in_trash(&runner, 0, "BT19099-COMP3"),
        "declining the pick must leave the [Composite] Digimon in trash"
    );
    assert!(
        on_field(&runner, 0, "BT19-099"),
        "BT19-099's 'Then, place this card in the battle area' still runs after declining the trash play"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — Clause B: [All Turns] Millenniummon leave watcher
// ═══════════════════════════════════════════════════════════════════════════

/// Negative: the replacement must NOT fire for an opponent's Millenniummon-
/// name Digimon leaving (subject_is_mine filter rejects).
#[test]
fn bt19_099_replacement_does_not_fire_for_opponent_millenniummon() {
    let opp_mm = make_millenniummon_named("BT19099-OPP-MM", 10);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT19_099_YAML)
        .expect("BT19-099 YAML parses")
        .add_card(opp_mm.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let bt19_099 = runner.place_on_field(0, "BT19-099", Some(0));
    seat_as_delay_option(&mut runner, bt19_099);
    let opp_handle = runner.place_on_field(1, "BT19099-OPP-MM", None);

    runner
        .game
        .delete_permanent_with_cause(opp_handle, ReplacementCause::OwnEffect);
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "no replacement prompt for the OPPONENT's leaving Millenniummon"
    );
    assert!(
        on_field(&runner, 0, "BT19-099"),
        "BT19-099 must remain on field; replacement should not fire for opponent's leaving Digimon"
    );
}

/// Negative: an OWN Digimon WITHOUT Millenniummon in its name leaving does
/// NOT fire the replacement (name filter rejects).
#[test]
fn bt19_099_replacement_does_not_fire_for_own_non_millenniummon() {
    let other = make_non_millenniummon_named("BT19099-OTHER", 10);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT19_099_YAML)
        .expect("BT19-099 YAML parses")
        .add_card(other.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let bt19_099 = runner.place_on_field(0, "BT19-099", Some(0));
    seat_as_delay_option(&mut runner, bt19_099);
    let own_handle = runner.place_on_field(0, "BT19099-OTHER", None);

    runner
        .game
        .delete_permanent_with_cause(own_handle, ReplacementCause::OwnEffect);
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "no replacement prompt for an own non-Millenniummon leaving"
    );
    assert!(
        on_field(&runner, 0, "BT19-099"),
        "BT19-099 must remain on field; name filter rejects non-Millenniummon subject"
    );
}

/// Positive: an OWN Millenniummon-name Digimon leaving fires the replacement
/// — the optional accept prompt installs (SelectionKind::Replacement with
/// REPLACEMENT_ACCEPT).
#[test]
fn bt19_099_replacement_fires_for_own_millenniummon_leaving() {
    let mm = make_millenniummon_named("BT19099-MM-FIRE", 10);
    let wicked_god = make_wicked_god_digimon("BT19099-WG-FIRE", "Armageddemon", 11);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT19_099_YAML)
        .expect("BT19-099 YAML parses")
        .add_card(mm.clone())
        .add_card(wicked_god.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["BT19099-WG-FIRE"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let bt19_099 = runner.place_on_field(0, "BT19-099", Some(0));
    seat_as_delay_option(&mut runner, bt19_099);
    let own_handle = runner.place_on_field(0, "BT19099-MM-FIRE", None);

    runner
        .game
        .delete_permanent_with_cause(own_handle, ReplacementCause::OwnEffect);

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("BT19-099's replacement must install an optional-accept prompt");
    assert_eq!(pending.kind, SelectionKind::Replacement);
    assert!(pending.valid_action_ids.contains(&REPLACEMENT_ACCEPT));
}

/// DCGO-faithful offer gating: the accept prompt installs EVEN WITH ZERO
/// eligible reward candidates — DCGO's `CanUseCondition` (BT19_099.cs:205-219)
/// has no candidate check; the candidate scan (lines 275-277) happens inside
/// `ActivateCoroutine` AFTER the self-trash. Accepting with zero candidates
/// trashes this Option, plays nothing, and the leave still completes.
#[test]
fn bt19_099_offer_installs_even_with_zero_reward_candidates() {
    let mm = make_millenniummon_named("BT19099-MM-Z", 10);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT19_099_YAML)
        .expect("BT19-099 YAML parses")
        .add_card(mm.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let bt19_099 = runner.place_on_field(0, "BT19-099", Some(0));
    seat_as_delay_option(&mut runner, bt19_099);
    let mm_handle = runner.place_on_field(0, "BT19099-MM-Z", None);
    let mm_card = runner.top_card(mm_handle);

    runner
        .game
        .delete_permanent_with_cause(mm_handle, ReplacementCause::OwnEffect);

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("the <Delay> accept prompt installs even with zero reward candidates (DCGO CanUseCondition has no candidate gate)");
    assert_eq!(pending.kind, SelectionKind::Replacement);

    runner
        .game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept the <Delay> response");
    runner.game.drain_effect_queue();

    assert!(
        in_trash(&runner, 0, "BT19-099"),
        "accepting pays the <Delay> cost (self-trash) even though nothing can be played"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.handle() == mm_card),
        "the leaving Millenniummon still completes its leave to the trash"
    );
    assert!(
        runner.game.pending_selection.is_none(),
        "with zero eligible candidates no reward prompt installs (DCGO post-trash whiff)"
    );
}

/// Positive end-to-end (the reviewer-named defect repro): accepting the
/// replacement pays the <Delay> cost (BT19-099 trashes itself) and the
/// leaving Millenniummon completes its leave; the reward's union-zone pick
/// then offers the [Wicked God] Digimon in hand whose play cost is EXACTLY
/// the leaving Millenniummon's cost + 1 — accepting plays it for free ONTO
/// THE BATTLE AREA (regression: it used to land in trash while the
/// Millenniummon survived — G-ENGINE-DELAY-SELF-TRASH-THEN-FREE-PLAY-IN-
/// LEAVE-REPLACEMENT).
#[test]
fn bt19_099_replacement_delay_then_plays_wicked_god_at_cost_plus_one_free() {
    let mm = make_millenniummon_named("BT19099-MM-A", 10);
    let wicked_god = make_wicked_god_digimon("BT19099-WG-A", "Armageddemon", 11);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT19_099_YAML)
        .expect("BT19-099 YAML parses")
        .add_card(mm.clone())
        .add_card(wicked_god.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["BT19099-WG-A"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let bt19_099 = runner.place_on_field(0, "BT19-099", Some(0));
    seat_as_delay_option(&mut runner, bt19_099);
    let mm_handle = runner.place_on_field(0, "BT19099-MM-A", None);
    let mm_card = runner.top_card(mm_handle);

    runner
        .game
        .delete_permanent_with_cause(mm_handle, ReplacementCause::OwnEffect);

    // Accept the optional replacement (<Delay> declaration).
    runner
        .game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept the <Delay> response");

    // The <Delay> cost is paid and the owned leave completes BEFORE the
    // reward pick (DCGO order: self-trash first; the leave is not prevented).
    assert!(
        in_trash(&runner, 0, "BT19-099"),
        "BT19-099 must trash itself to pay the <Delay> cost"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.handle() == mm_card),
        "the leaving Millenniummon completes its leave to the trash before the reward"
    );

    // The reward's union-zone pick installs (hand + trash surface).
    let sel = runner
        .game
        .pending_selection
        .as_ref()
        .expect("union-zone [Wicked God] pick installs");
    assert_eq!(
        sel.kind,
        SelectionKind::UnionZone {
            zones: UnionZoneSet::HAND | UnionZoneSet::TRASH
        },
        "the reward prompt must be the union-zone pick"
    );
    assert!(sel.is_optional, "the reward pick is the printed 'you may play'");

    // Accept: play the Wicked God Digimon (hand slot 0 → PLAY_HAND_START).
    let action = sel
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("at least one eligible [Wicked God] card action");
    assert_eq!(
        action, PLAY_HAND_START,
        "the sole candidate is the hand's [Wicked God] at hand index 0"
    );
    let player = sel.selecting_player;
    runner
        .game
        .resolve_selection(player, action)
        .expect("pick the Wicked God Digimon");
    runner.game.drain_effect_queue();
    drive_to_completion(&mut runner, 10);

    assert!(
        on_field(&runner, 0, "BT19099-WG-A"),
        "the [Wicked God] Digimon (cost 11 = 10 + 1) must be played onto the battle area — \
         NOT into the trash"
    );
    assert!(
        !in_trash(&runner, 0, "BT19099-WG-A"),
        "the played [Wicked God] Digimon must not be in the trash"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        0,
        "the [Wicked God] Digimon must leave the hand once played"
    );
}

/// The reward also plays from the TRASH — the printed "from your hand or
/// trash" (DCGO routes to the trash pick when only the trash has candidates).
#[test]
fn bt19_099_reward_plays_wicked_god_from_trash() {
    let mm = make_millenniummon_named("BT19099-MM-T", 10);
    let wicked_god = make_wicked_god_digimon("BT19099-WG-T", "Armageddemon", 11);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT19_099_YAML)
        .expect("BT19-099 YAML parses")
        .add_card(mm.clone())
        .add_card(wicked_god.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    // The [Wicked God] candidate sits in the TRASH (index 0 — seeded before
    // the self-trash and the leave add more cards behind it).
    seed_trash(&mut runner, 0, "BT19099-WG-T");

    let bt19_099 = runner.place_on_field(0, "BT19-099", Some(0));
    seat_as_delay_option(&mut runner, bt19_099);
    let mm_handle = runner.place_on_field(0, "BT19099-MM-T", None);

    runner
        .game
        .delete_permanent_with_cause(mm_handle, ReplacementCause::OwnEffect);
    runner
        .game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept the <Delay> response");

    let sel = runner
        .game
        .pending_selection
        .as_ref()
        .expect("union-zone [Wicked God] pick installs");
    // Exactly one candidate action, in the TRASH action range (the just-
    // trashed BT19-099 and Millenniummon are filtered out by trait/cost).
    let non_pass: Vec<u16> = sel
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    assert_eq!(
        non_pass,
        vec![TRASH_EFFECT_START],
        "the sole candidate is the trash's [Wicked God] at trash index 0"
    );

    let player = sel.selecting_player;
    runner
        .game
        .resolve_selection(player, TRASH_EFFECT_START)
        .expect("pick the Wicked God Digimon from trash");
    runner.game.drain_effect_queue();
    drive_to_completion(&mut runner, 10);

    assert!(
        on_field(&runner, 0, "BT19099-WG-T"),
        "the [Wicked God] Digimon must be played from the trash onto the battle area"
    );
    assert!(
        !in_trash(&runner, 0, "BT19099-WG-T"),
        "the played [Wicked God] Digimon must have left the trash"
    );
}

/// Snapshot-comparator precision: with BOTH a cost-11 (= subject 10 + 1) and a
/// cost-10 [Wicked God] in hand, only the cost-11 candidate is offered — the
/// comparator reads the leaving subject's PRE-LEAVE printed cost (DCGO:
/// `permanent.TopCard.GetCostItself + 1 == candidate.GetCostItself` over the
/// trigger-hashtable snapshot), even though the subject has already completed
/// its leave by the time the pick installs.
#[test]
fn bt19_099_reward_offers_only_exact_cost_plus_one() {
    let mm = make_millenniummon_named("BT19099-MM-P", 10);
    let wg_right = make_wicked_god_digimon("BT19099-WG-11", "Armageddemon", 11);
    let wg_wrong = make_wicked_god_digimon("BT19099-WG-10", "Machinedramon", 10);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT19_099_YAML)
        .expect("BT19-099 YAML parses")
        .add_card(mm.clone())
        .add_card(wg_right.clone())
        .add_card(wg_wrong.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["BT19099-WG-11", "BT19099-WG-10"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let bt19_099 = runner.place_on_field(0, "BT19-099", Some(0));
    seat_as_delay_option(&mut runner, bt19_099);
    let mm_handle = runner.place_on_field(0, "BT19099-MM-P", None);

    runner
        .game
        .delete_permanent_with_cause(mm_handle, ReplacementCause::OwnEffect);
    runner
        .game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept the <Delay> response");

    let sel = runner
        .game
        .pending_selection
        .as_ref()
        .expect("union-zone [Wicked God] pick installs");
    let non_pass: Vec<u16> = sel
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    assert_eq!(
        non_pass,
        vec![PLAY_HAND_START],
        "only the cost-11 [Wicked God] (hand index 0) is offered; the cost-10 \
         sibling (hand index 1) fails the +1 comparator"
    );
}

/// Negative filter: a [Wicked God] Digimon whose play cost is NOT exactly
/// (leaving Digimon's cost + 1) is NOT offered. With the sole candidate
/// rejected, accepting the <Delay> still self-trashes and the leave still
/// completes, but no reward prompt installs and nothing is played — DCGO's
/// post-trash candidate whiff (BT19_099.cs:275-277).
#[test]
fn bt19_099_reward_filter_rejects_wicked_god_at_wrong_cost() {
    let mm = make_millenniummon_named("BT19099-MM-B", 10);
    // Same cost as the leaving Digimon (10), NOT +1 (11) — must be rejected.
    let wrong_cost_wg = make_wicked_god_digimon("BT19099-WG-WRONG", "Machinedramon", 10);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT19_099_YAML)
        .expect("BT19-099 YAML parses")
        .add_card(mm.clone())
        .add_card(wrong_cost_wg.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["BT19099-WG-WRONG"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let bt19_099 = runner.place_on_field(0, "BT19-099", Some(0));
    seat_as_delay_option(&mut runner, bt19_099);
    let mm_handle = runner.place_on_field(0, "BT19099-MM-B", None);
    let mm_card = runner.top_card(mm_handle);

    runner
        .game
        .delete_permanent_with_cause(mm_handle, ReplacementCause::OwnEffect);
    runner
        .game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept the <Delay> response");
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "with the sole candidate rejected by the wrong-cost filter, no reward prompt installs"
    );
    assert!(
        in_hand(&runner, 0, "BT19099-WG-WRONG"),
        "the wrong-cost [Wicked God] Digimon must not have been played"
    );
    assert!(
        in_trash(&runner, 0, "BT19-099"),
        "the <Delay> cost was still paid (DCGO trashes before the candidate scan)"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.handle() == mm_card),
        "the leaving Millenniummon still completes its leave"
    );
}

/// Negative filter: a Digimon at the CORRECT cost (+1) but WITHOUT the
/// [Wicked God] trait is not offered (trait filter rejects). Same post-trash
/// whiff shape as the wrong-cost sibling.
#[test]
fn bt19_099_reward_filter_rejects_non_wicked_god_at_correct_cost() {
    let mm = make_millenniummon_named("BT19099-MM-C", 10);
    let non_wg = make_non_wicked_god_at_cost("BT19099-NONWG", 11);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT19_099_YAML)
        .expect("BT19-099 YAML parses")
        .add_card(mm.clone())
        .add_card(non_wg.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["BT19099-NONWG"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let bt19_099 = runner.place_on_field(0, "BT19-099", Some(0));
    seat_as_delay_option(&mut runner, bt19_099);
    let mm_handle = runner.place_on_field(0, "BT19099-MM-C", None);

    runner
        .game
        .delete_permanent_with_cause(mm_handle, ReplacementCause::OwnEffect);
    runner
        .game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept the <Delay> response");
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "with the sole candidate rejected by the trait filter, no reward prompt installs"
    );
    assert!(
        in_hand(&runner, 0, "BT19099-NONWG"),
        "the non-[Wicked God] Digimon must not have been played"
    );
    assert!(
        in_trash(&runner, 0, "BT19-099"),
        "the <Delay> cost was still paid"
    );
}

/// Declining the reward's union-zone pick (PASS) still leaves BT19-099 in the
/// trash (the Delay cost was already paid) and plays nothing; the leave has
/// already completed.
#[test]
fn bt19_099_reward_can_be_declined_after_delay_cost_paid() {
    let mm = make_millenniummon_named("BT19099-MM-D", 10);
    let wicked_god = make_wicked_god_digimon("BT19099-WG-D", "Armageddemon", 11);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT19_099_YAML)
        .expect("BT19-099 YAML parses")
        .add_card(mm.clone())
        .add_card(wicked_god.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["BT19099-WG-D"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let bt19_099 = runner.place_on_field(0, "BT19-099", Some(0));
    seat_as_delay_option(&mut runner, bt19_099);
    let mm_handle = runner.place_on_field(0, "BT19099-MM-D", None);
    let mm_card = runner.top_card(mm_handle);

    runner
        .game
        .delete_permanent_with_cause(mm_handle, ReplacementCause::OwnEffect);
    runner
        .game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept the <Delay> response");

    let sel = runner
        .game
        .pending_selection
        .as_ref()
        .expect("union-zone [Wicked God] pick installs");
    assert!(
        sel.is_optional,
        "the reward pick must be independently declinable"
    );
    let player = sel.selecting_player;
    runner
        .game
        .resolve_selection(player, PASS)
        .expect("decline the reward pick");
    runner.game.drain_effect_queue();

    assert!(
        in_hand(&runner, 0, "BT19099-WG-D"),
        "declining the reward must leave the [Wicked God] Digimon in hand"
    );
    assert!(
        in_trash(&runner, 0, "BT19-099"),
        "BT19-099 remains trashed (the Delay self-trash cost is not refunded on declining the reward)"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.handle() == mm_card),
        "the leaving Millenniummon still completed its leave"
    );
}

/// Declining the <Delay> declaration itself keeps BT19-099 on the field (the
/// cost is not paid), plays nothing, and the Millenniummon still leaves.
#[test]
fn bt19_099_decline_replacement_keeps_option_and_leave_proceeds() {
    let mm = make_millenniummon_named("BT19099-MM-DECL", 10);
    let wicked_god = make_wicked_god_digimon("BT19099-WG-DECL", "Armageddemon", 11);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT19_099_YAML)
        .expect("BT19-099 YAML parses")
        .add_card(mm.clone())
        .add_card(wicked_god.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["BT19099-WG-DECL"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let bt19_099 = runner.place_on_field(0, "BT19-099", Some(0));
    seat_as_delay_option(&mut runner, bt19_099);
    let mm_handle = runner.place_on_field(0, "BT19099-MM-DECL", None);
    let mm_card = runner.top_card(mm_handle);

    runner
        .game
        .delete_permanent_with_cause(mm_handle, ReplacementCause::OwnEffect);
    runner
        .game
        .resolve_selection(0, PASS)
        .expect("decline the <Delay> response");
    runner.game.drain_effect_queue();

    assert!(
        on_field(&runner, 0, "BT19-099"),
        "declining keeps BT19-099 on the field (the <Delay> cost is not paid)"
    );
    assert!(
        in_hand(&runner, 0, "BT19099-WG-DECL"),
        "declining plays nothing"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.handle() == mm_card),
        "the Millenniummon still leaves"
    );
}

/// The leaving Millenniummon-name Digimon proceeds to leave the battle area
/// even when the replacement actually FIRES and its reward resolves — the
/// effect is a reward, not a prevention (no cancel_replacement; DCGO places
/// no Cancel on the WhenRemoveField event).
#[test]
fn bt19_099_replacement_does_not_cancel_the_leave() {
    let mm = make_millenniummon_named("BT19099-MM-E", 10);
    let wicked_god = make_wicked_god_digimon("BT19099-WG-E", "Armageddemon", 11);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT19_099_YAML)
        .expect("BT19-099 YAML parses")
        .add_card(mm.clone())
        .add_card(wicked_god.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["BT19099-WG-E"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let bt19_099 = runner.place_on_field(0, "BT19-099", Some(0));
    seat_as_delay_option(&mut runner, bt19_099);
    let mm_handle = runner.place_on_field(0, "BT19099-MM-E", None);
    let mm_card = runner.top_card(mm_handle);

    runner
        .game
        .delete_permanent_with_cause(mm_handle, ReplacementCause::OwnEffect);

    // FIRE the replacement: accept the <Delay>, then resolve the reward pick.
    runner
        .game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept the <Delay> response");
    {
        let sel = runner
            .game
            .pending_selection
            .as_ref()
            .expect("union-zone [Wicked God] pick installs");
        let action = sel
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .expect("the eligible [Wicked God] action");
        let player = sel.selecting_player;
        runner
            .game
            .resolve_selection(player, action)
            .expect("play the Wicked God Digimon");
    }
    runner.game.drain_effect_queue();
    drive_to_completion(&mut runner, 10);

    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.handle() == mm_card),
        "the leaving Millenniummon-name Digimon must proceed to the trash \
         (BT19-099's reward is not a prevention effect)"
    );
    assert!(
        !on_field(&runner, 0, "BT19099-MM-E"),
        "the Millenniummon is no longer on the field"
    );
    assert!(
        on_field(&runner, 0, "BT19099-WG-E"),
        "the reward [Wicked God] Digimon is on the field"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Clause C: [Security] (inherited) bare self-placement
// ═══════════════════════════════════════════════════════════════════════════

/// Driven through the real combat/security-check path: BT19-099 in the
/// defender's security, attacked, flips, and its mandatory inherited
/// [Security] clause places it directly into the defender's battle area
/// (no draw/play sub-body — the printed text is only "Place this card in
/// the battle area").
#[test]
fn bt19_099_security_places_self_in_battle_area_on_attack() {
    let mut attacker = make_filler("BT19099-ATK");
    attacker.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT19_099_YAML)
        .expect("BT19-099 YAML parses")
        .add_card(attacker.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .security(1, &["BT19-099"])
        .start();

    let attacker_handle = runner.place_on_field(0, "BT19099-ATK", Some(0));
    assert_eq!(
        runner.security_count(1),
        1,
        "precondition: BT19-099 in security"
    );
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "precondition: defender's battle area empty"
    );

    let _ = runner.attack_player(attacker_handle, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    assert_eq!(
        runner.security_count(1),
        0,
        "BT19-099 must leave the security stack"
    );
    assert!(
        on_field(&runner, 1, "BT19-099"),
        "BT19-099's inherited [Security] clause must place it directly in the \
         defender's battle area"
    );
}
