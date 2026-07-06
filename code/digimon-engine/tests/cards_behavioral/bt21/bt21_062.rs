//! BT21-062 Galacticmon — Digimon, Lv.6, Black, DP 14000, Cost 14.
//! Traits: Unknown/LIBERATOR. Form: Mega. Attribute: Unknown.
//!
//! # Card text (card image / data/card_bundles/BT21-062.md — authoritative)
//!
//! ```text
//! [Digivolve] [Snatchmon]: Cost 9
//! [When Digivolving] By placing 4 cards with [Vemmon] in their texts from
//!   your trash as this Digimon's bottom digivolution cards, you may use 1
//!   [Ragnarok Cannon] from your hand or trash without paying the cost.
//! [Start of Your Main Phase] Delete 1 of your opponent's Digimon.
//! [All Turns] When this Digimon would leave the battle area, by returning 4
//!   [Vemmon] from its digivolution cards to the bottom of the deck, it
//!   doesn't leave.
//! ```
//!
//! NOTE: the intake brief hinted "[Start of Opp Main?]" for clause 2 — the
//! card IMAGE and the official bundle both confirm "Start of YOUR Main
//! Phase" (matching DCGO `EffectTiming.OnStartMainPhase` +
//! `CanUseCondition: IsOwnerTurn(card)`). This test file verifies the clause
//! fires on the CONTROLLER's own start-of-main, per the card face.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Black/BT21_062.cs
//!
//! # Implementation
//! - Clause 1 ([When Digivolving]): a `condition: count_gte { n: 4 }` gate
//!   (DCGO `MatchConditionOwnersCardCountInTrash >= 4`) precedes 4 sequential
//!   `select_trash` + `place_as_bottom_source` picks (pick 1
//!   `optional: true, cost: true`, picks 2-4 mandatory — modeling DCGO's
//!   "decline entirely, or commit to exactly 4" `canEndNotMax:false` multi-
//!   select). Then a declinable `select_union_zone { zones: [hand, trash] }`
//!   + `use_option_bound` plays 1 [Ragnarok Cannon] free from its true
//!   origin zone.
//! - Clause 2 ([Start of Your Main Phase]): `select_opponent_permanent`
//!   (free choice, no selector) + `delete_permanent`.
//! - Clause 3 ([All Turns] would-leave): a `kind: replacement` using the
//!   purpose-built `cost: { return_own_sources_to_deck: { filter, count: 4,
//!   position: bottom } }` declarative shortcut (Gap 2 / BT21-062 driver in
//!   `digimon-dsl/src/clause.rs`) — pay-in-full exactly 4 [Vemmon] sources,
//!   returned to the BOTTOM of the owner's deck, then `outcome: prevent`
//!   auto-appends `cancel_replacement`. No "by an effect" / battle-cause
//!   exclusion (DCGO does not gate this clause on `IsByEffect`), so it fires
//!   for ANY leave cause including battle.

use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Fixtures ────────────────────────────────────────────────────────────────

/// A Digimon whose printed text/name contains "Vemmon" — satisfies clause 1's
/// broad `in_text_contains: "Vemmon"` cost filter. Named exactly "Vemmon" so
/// it ALSO satisfies clause 3's exact `name_is: "Vemmon"` filter (both
/// filters are exercised against this same fixture family across tests).
fn make_vemmon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Vemmon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.colors = vec![CardColor::Black];
    c
}

/// A card whose text does NOT mention "Vemmon" — negative control.
fn make_plain(id: &str) -> CardData {
    let mut c = make_test_card(id, "Plain Filler");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Black];
    c
}

/// A generic opponent Digimon for the [Start of Your Main Phase] delete clause.
fn make_opp_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.dp = Some(dp);
    c.colors = vec![CardColor::Red];
    c
}

/// A synthetic "Ragnarok Cannon" Option fixture (BT21-098 has no YAML in this
/// worktree — Galacticmon only needs to move/use it, not resolve its own
/// effects). `use_cost` models the printed Option use cost.
fn make_ragnarok_cannon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Ragnarok Cannon");
    c.card_kind = CardKind::Option;
    c.play_cost = 8;
    c.colors = vec![CardColor::Black];
    c
}

/// A distractor Option that is NOT named "Ragnarok Cannon" — negative control
/// for the union-zone filter.
fn make_other_option(id: &str) -> CardData {
    let mut c = make_test_card(id, "Some Other Option");
    c.card_kind = CardKind::Option;
    c.play_cost = 3;
    c.colors = vec![CardColor::Black];
    c
}

/// Push a card straight into a player's trash zone (bypasses hand/discard flow).
fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_trash: unknown card_id {card_id}"));
    let src = CardSource::new(data_idx, player, runner.game.next_card_index());
    runner.game.players[player as usize].trash.push(src);
}

/// Push a card straight into a player's hand.
fn push_to_hand(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_hand: unknown card_id {card_id}"));
    let src = CardSource::new(data_idx, player, runner.game.next_card_index());
    runner.game.players[player as usize].hand.push(src);
}

fn fire_when_digivolving(runner: &mut DebugRunner, source: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

fn fire_start_of_your_main_phase(runner: &mut DebugRunner, source: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(source),
    );
    runner.game.drain_effect_queue();
}

fn permanent_exists(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == card_id)
}

fn galacticmon_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT21-062")
        .expect("BT21-062 YAML loads")
}

/// Take the single valid action id for the current pending selection, or PASS
/// if the selection is optional and offers no legal action (shouldn't happen
/// here since every prompt in these tests has >=1 legal candidate).
fn take_first_or_pass(runner: &mut DebugRunner) {
    let view = runner
        .pending_selection_view()
        .expect("a pending selection is installed");
    let action = view
        .valid_action_ids
        .first()
        .copied()
        .unwrap_or(PASS);
    runner
        .execute_action(0, action)
        .expect("resolve pending selection");
}

// ─── Section 1: metadata / structure ─────────────────────────────────────────

#[test]
fn bt21_062_loads_with_printed_metadata() {
    use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause};

    let runner = galacticmon_runner().start();
    let compiled = runner.compiled_card("BT21-062").expect("compiled card");
    assert_eq!(compiled.name, "Galacticmon");
    assert_eq!(compiled.level, Some(6));
    assert_eq!(compiled.dp, Some(14000));
    assert_eq!(compiled.cost, Some(14));
    assert!(compiled.traits.iter().any(|t| t == "LIBERATOR"));

    let has_replacement = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement { .. })
        )
    });
    assert!(
        has_replacement,
        "the would-leave self-protection compiles as a replacement clause"
    );
    let triggered_count = compiled
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();
    assert_eq!(
        triggered_count, 2,
        "[When Digivolving] and [Start of Your Main Phase] both compile as triggered clauses"
    );
}

#[test]
fn bt21_062_has_snatchmon_alt_digivolve() {
    let runner = galacticmon_runner().start();
    let compiled = runner.compiled_card("BT21-062").expect("compiled card");
    assert!(
        !compiled.alt_paths.is_empty(),
        "Galacticmon has an alt-digivolve path from Snatchmon"
    );
}

// ─── Section 2: Clause 1 — [When Digivolving] place 4 Vemmon sources ────────

/// Positive: with exactly 4 "Vemmon"-text cards in trash, WhenDigivolving
/// prompts a Trash selection; after placing all 4, each becomes a
/// digivolution source of Galacticmon.
#[test]
fn bt21_062_when_digivolving_places_four_vemmon_sources() {
    let mut runner = galacticmon_runner()
        .add_card(make_vemmon("VEM-1"))
        .add_card(make_vemmon("VEM-2"))
        .add_card(make_vemmon("VEM-3"))
        .add_card(make_vemmon("VEM-4"))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .start();
    push_to_trash(&mut runner, 0, "VEM-1");
    push_to_trash(&mut runner, 0, "VEM-2");
    push_to_trash(&mut runner, 0, "VEM-3");
    push_to_trash(&mut runner, 0, "VEM-4");

    let galacticmon = runner.place_on_field(0, "BT21-062", Some(0));

    fire_when_digivolving(&mut runner, galacticmon);

    // Pick 1/4 — declinable cost-pay pick; take it.
    let view = runner
        .pending_selection_view()
        .expect("first Vemmon-placement prompt installs");
    assert_eq!(view.kind, SelectionKind::Trash);
    assert!(
        runner.pending_is_optional(),
        "the first pick is declinable (the whole cost is a may-pay)"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        4,
        "all 4 Vemmon-text trash cards are legal for pick 1"
    );
    take_first_or_pass(&mut runner);

    // Picks 2-4/4 — mandatory once engaged.
    for i in 2..=4 {
        let view = runner
            .pending_selection_view()
            .unwrap_or_else(|| panic!("Vemmon-placement prompt {i}/4 installs"));
        assert_eq!(view.kind, SelectionKind::Trash);
        assert!(
            !runner.pending_is_optional(),
            "pick {i}/4 is mandatory once the cost is engaged"
        );
        take_first_or_pass(&mut runner);
    }

    // After all 4 placements, the Ragnarok Cannon offer is declined (none in
    // hand/trash here) — no prompt should install, or if one installs it must
    // be optional and PASS-able.
    if let Some(view) = runner.pending_selection_view() {
        assert!(view.is_optional, "the free Ragnarok Cannon use is optional");
        runner.execute_action(0, PASS).expect("decline cannon use");
    }
    runner.auto_resolve().expect("finish resolution");

    let stack = &runner.game.players[0].battle_area[galacticmon.index as usize];
    for id in ["VEM-1", "VEM-2", "VEM-3", "VEM-4"] {
        assert!(
            stack
                .card_sources
                .iter()
                .any(|s| s.card_id(&runner.game.card_data) == id),
            "{id} became a digivolution source of Galacticmon"
        );
    }
    assert_eq!(
        runner.trash_size(0),
        0,
        "all 4 Vemmon-text cards left the trash"
    );
}

/// Positive: after placing 4 Vemmon sources, a [Ragnarok Cannon] sitting in
/// trash may be used for free — it leaves the trash without any memory/cost
/// being spent.
#[test]
fn bt21_062_when_digivolving_may_use_ragnarok_cannon_from_trash_free() {
    let mut runner = galacticmon_runner()
        .add_card(make_vemmon("VEM-1"))
        .add_card(make_vemmon("VEM-2"))
        .add_card(make_vemmon("VEM-3"))
        .add_card(make_vemmon("VEM-4"))
        .add_card(make_ragnarok_cannon("CANNON"))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .memory(0)
        .start();
    push_to_trash(&mut runner, 0, "VEM-1");
    push_to_trash(&mut runner, 0, "VEM-2");
    push_to_trash(&mut runner, 0, "VEM-3");
    push_to_trash(&mut runner, 0, "VEM-4");
    push_to_trash(&mut runner, 0, "CANNON");

    let galacticmon = runner.place_on_field(0, "BT21-062", Some(0));

    fire_when_digivolving(&mut runner, galacticmon);

    // Resolve the 4 Vemmon placement picks (first is optional+cost, rest
    // mandatory) by always taking the first legal action.
    for _ in 0..4 {
        take_first_or_pass(&mut runner);
    }

    // The free Ragnarok Cannon offer should now be a UnionZone prompt over
    // {hand, trash} filtered to "Ragnarok Cannon".
    let view = runner
        .pending_selection_view()
        .expect("Ragnarok Cannon free-use prompt installs after 4 placements");
    assert!(
        matches!(view.kind, SelectionKind::UnionZone { .. }),
        "the free-use pick is a union-zone selection, got {:?}",
        view.kind
    );
    assert!(view.is_optional, "using the Cannon is declinable (\"you may\")");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the Ragnarok Cannon in trash is a legal pick"
    );
    let memory_before = runner.game.memory;
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("use the Ragnarok Cannon for free");
    runner.auto_resolve().expect("finish resolution");

    assert_eq!(
        runner.trash_size(0),
        0,
        "the Ragnarok Cannon left the trash when used"
    );
    assert_eq!(
        runner.game.memory, memory_before,
        "using the Cannon for free must not spend memory"
    );
}

/// Negative: with only 3 "Vemmon"-text cards in trash, the whole clause never
/// fires — the `condition: count_gte { n: 4 }` gate blocks it entirely.
#[test]
fn bt21_062_when_digivolving_does_nothing_with_only_three_vemmon_in_trash() {
    let mut runner = galacticmon_runner()
        .add_card(make_vemmon("VEM-1"))
        .add_card(make_vemmon("VEM-2"))
        .add_card(make_vemmon("VEM-3"))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .start();
    push_to_trash(&mut runner, 0, "VEM-1");
    push_to_trash(&mut runner, 0, "VEM-2");
    push_to_trash(&mut runner, 0, "VEM-3");

    let galacticmon = runner.place_on_field(0, "BT21-062", Some(0));
    fire_when_digivolving(&mut runner, galacticmon);

    assert!(
        runner.pending_selection().is_none(),
        "with fewer than 4 Vemmon-text cards in trash, the clause does not fire"
    );
    assert_eq!(
        runner.trash_size(0),
        3,
        "no cards were moved out of trash"
    );
}

/// Negative: plain (non-"Vemmon"-text) cards in trash do not count toward the
/// 4-card cost, even if there are 4+ of them.
#[test]
fn bt21_062_when_digivolving_ignores_non_vemmon_trash_cards() {
    let mut runner = galacticmon_runner()
        .add_card(make_plain("PLAIN-1"))
        .add_card(make_plain("PLAIN-2"))
        .add_card(make_plain("PLAIN-3"))
        .add_card(make_plain("PLAIN-4"))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .start();
    push_to_trash(&mut runner, 0, "PLAIN-1");
    push_to_trash(&mut runner, 0, "PLAIN-2");
    push_to_trash(&mut runner, 0, "PLAIN-3");
    push_to_trash(&mut runner, 0, "PLAIN-4");

    let galacticmon = runner.place_on_field(0, "BT21-062", Some(0));
    fire_when_digivolving(&mut runner, galacticmon);

    assert!(
        runner.pending_selection().is_none(),
        "plain cards without 'Vemmon' in their text do not satisfy the cost"
    );
}

/// Declining pick 1/4 aborts the entire clause — nothing is placed and no
/// Ragnarok Cannon offer follows.
#[test]
fn bt21_062_when_digivolving_decline_pick_one_aborts_whole_clause() {
    let mut runner = galacticmon_runner()
        .add_card(make_vemmon("VEM-1"))
        .add_card(make_vemmon("VEM-2"))
        .add_card(make_vemmon("VEM-3"))
        .add_card(make_vemmon("VEM-4"))
        .add_card(make_ragnarok_cannon("CANNON"))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .start();
    push_to_trash(&mut runner, 0, "VEM-1");
    push_to_trash(&mut runner, 0, "VEM-2");
    push_to_trash(&mut runner, 0, "VEM-3");
    push_to_trash(&mut runner, 0, "VEM-4");
    push_to_trash(&mut runner, 0, "CANNON");

    let galacticmon = runner.place_on_field(0, "BT21-062", Some(0));
    fire_when_digivolving(&mut runner, galacticmon);

    let view = runner
        .pending_selection_view()
        .expect("first Vemmon-placement prompt installs");
    assert!(view.is_optional, "pick 1/4 is declinable");
    runner
        .execute_action(0, PASS)
        .expect("decline the whole placement cost");
    runner.auto_resolve().expect("finish resolution");

    assert_eq!(
        runner.trash_size(0),
        5,
        "declining leaves all 5 trash cards untouched (4 Vemmon + 1 Cannon)"
    );
    let stack = &runner.game.players[0].battle_area[galacticmon.index as usize];
    assert!(
        stack.card_sources.is_empty(),
        "no digivolution sources were placed after declining"
    );
}

// ─── Section 3: Clause 2 — [Start of Your Main Phase] delete 1 opp Digimon ──

/// Positive: at the controller's own start-of-main, an opponent Digimon may
/// be freely chosen and is deleted.
#[test]
fn bt21_062_start_of_your_main_deletes_one_opponent_digimon() {
    let mut runner = galacticmon_runner()
        .add_card(make_opp_digimon("OPP-A", 3000))
        .add_card(make_opp_digimon("OPP-B", 5000))
        .start();

    let galacticmon = runner.place_on_field(0, "BT21-062", Some(0));
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));
    let _opp_b = runner.place_on_field(1, "OPP-B", Some(0));

    fire_start_of_your_main_phase(&mut runner, galacticmon);

    let view = runner
        .pending_selection_view()
        .expect("Start of Your Main Phase delete prompt installs");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(
        !view.is_optional,
        "the delete is mandatory when an opponent Digimon exists"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "both opponent Digimon are free-choice legal targets"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose an opponent Digimon to delete");
    runner.auto_resolve().expect("finish resolution");

    let opp_count = runner.game.players[1].battle_area.len();
    assert_eq!(
        opp_count, 1,
        "exactly 1 opponent Digimon was deleted (either target is legal)"
    );
    let _ = opp_a;
}

/// Negative: with no opponent Digimon on the field, no prompt installs.
#[test]
fn bt21_062_start_of_your_main_does_nothing_with_no_opponent_digimon() {
    let mut runner = galacticmon_runner().start();
    let galacticmon = runner.place_on_field(0, "BT21-062", Some(0));

    fire_start_of_your_main_phase(&mut runner, galacticmon);

    assert!(
        runner.pending_selection().is_none(),
        "with no opponent Digimon, the delete clause does not fire"
    );
}

// ─── Section 4: Clause 3 — [All Turns] would-leave: return 4 Vemmon sources ─

/// Positive: with exactly 4 "Vemmon"-named sources beneath Galacticmon, an
/// effect-driven removal offers the leave-prevention replacement; paying the
/// cost returns all 4 to the BOTTOM of the deck and Galacticmon stays.
#[test]
fn bt21_062_would_leave_returns_four_vemmon_sources_and_prevents() {
    let mut runner = galacticmon_runner()
        .add_card(make_vemmon("VEM-1"))
        .add_card(make_vemmon("VEM-2"))
        .add_card(make_vemmon("VEM-3"))
        .add_card(make_vemmon("VEM-4"))
        .start();
    // Bottom -> top: 4 Vemmon sources beneath the Galacticmon top card.
    let galacticmon = runner.place_stack(
        0,
        &["VEM-1", "VEM-2", "VEM-3", "VEM-4", "BT21-062"],
    );

    let deck_before = runner.game.players[0].deck.len();

    runner
        .game
        .delete_permanent_with_cause(galacticmon, ReplacementCause::OpponentEffect);

    // Accept the optional replacement.
    let view = runner
        .pending_selection_view()
        .expect("would-leave replacement offers an accept prompt");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert!(runner.pending_is_optional(), "replacement may be declined");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("accept the leave-prevention replacement");

    // Pay the cost — pick all 4 Vemmon sources (SourceMulti requires exactly
    // 4, pay-in-full).
    let cost_view = runner
        .pending_selection_view()
        .expect("source-return cost selection installs");
    assert!(
        matches!(cost_view.kind, SelectionKind::SourceMulti { min: 4, max: 4, .. }),
        "the cost is an exact 4-source pick, got {:?}",
        cost_view.kind
    );
    assert_eq!(
        cost_view.valid_action_ids.len(),
        4,
        "all 4 Vemmon sources are legal cost picks"
    );
    // Pick all 4 in sequence.
    for _ in 0..4 {
        let view = runner
            .pending_selection_view()
            .expect("source pick prompt installs for each of the 4 picks");
        let action = view.valid_action_ids[0];
        runner.execute_action(0, action).expect("pick a source");
    }
    runner.auto_resolve().expect("finish replacement");

    // Galacticmon stays on the field.
    assert!(
        permanent_exists(&runner, 0, "BT21-062"),
        "paying the cost prevents the removal — Galacticmon stays"
    );
    // No Vemmon sources remain beneath it.
    let perm = &runner.game.players[0].battle_area[galacticmon.index as usize];
    assert!(
        perm.card_sources
            .iter()
            .all(|s| s.card_id(&runner.game.card_data) != "VEM-1"
                && s.card_id(&runner.game.card_data) != "VEM-2"
                && s.card_id(&runner.game.card_data) != "VEM-3"
                && s.card_id(&runner.game.card_data) != "VEM-4"),
        "all 4 returned sources are gone from Galacticmon's digivolution cards"
    );
    // All 4 sources are at the BOTTOM of the deck, not in trash.
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before + 4,
        "deck grows by the 4 returned sources"
    );
    assert_eq!(
        runner.trash_size(0),
        0,
        "the return-to-deck cost must NOT route sources to trash"
    );
}

/// Declining the optional replacement lets Galacticmon leave the battle area,
/// with the 4 Vemmon sources untouched (still trashed along with it, per
/// normal deletion — none returned to deck).
#[test]
fn bt21_062_would_leave_decline_lets_it_leave() {
    let mut runner = galacticmon_runner()
        .add_card(make_vemmon("VEM-1"))
        .add_card(make_vemmon("VEM-2"))
        .add_card(make_vemmon("VEM-3"))
        .add_card(make_vemmon("VEM-4"))
        .start();
    let galacticmon = runner.place_stack(
        0,
        &["VEM-1", "VEM-2", "VEM-3", "VEM-4", "BT21-062"],
    );

    runner
        .game
        .delete_permanent_with_cause(galacticmon, ReplacementCause::OpponentEffect);

    let view = runner
        .pending_selection_view()
        .expect("would-leave replacement offers an accept prompt");
    assert_eq!(view.kind, SelectionKind::Replacement);
    runner
        .execute_action(0, PASS)
        .expect("decline the leave-prevention");
    runner.auto_resolve().expect("finish declined removal");

    assert!(
        !permanent_exists(&runner, 0, "BT21-062"),
        "declining the replacement lets Galacticmon leave the battle area"
    );
}

/// Negative gate: with fewer than 4 "Vemmon"-named sources beneath
/// Galacticmon, the prevention cost is unpayable — no prompt installs and the
/// removal goes through.
#[test]
fn bt21_062_would_leave_no_prevention_with_fewer_than_four_vemmon_sources() {
    let mut runner = galacticmon_runner()
        .add_card(make_vemmon("VEM-1"))
        .add_card(make_vemmon("VEM-2"))
        .add_card(make_vemmon("VEM-3"))
        .start();
    let galacticmon = runner.place_stack(0, &["VEM-1", "VEM-2", "VEM-3", "BT21-062"]);

    runner
        .game
        .delete_permanent_with_cause(galacticmon, ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_selection().is_none(),
        "with only 3 Vemmon sources, the prevention cost is unpayable -> never offered"
    );
    assert!(
        !permanent_exists(&runner, 0, "BT21-062"),
        "with no payable cost Galacticmon leaves the battle area"
    );
}

/// Clause 3 has no "by an effect" exclusion — it also offers the prevention
/// on a BATTLE removal (unlike BT13-075's clause, which DCGO gates on
/// IsByEffect; Galacticmon's `CanTriggerWhenRemoveField` carries no such
/// gate).
#[test]
fn bt21_062_would_leave_also_triggers_on_battle_removal() {
    let mut runner = galacticmon_runner()
        .add_card(make_vemmon("VEM-1"))
        .add_card(make_vemmon("VEM-2"))
        .add_card(make_vemmon("VEM-3"))
        .add_card(make_vemmon("VEM-4"))
        .start();
    let galacticmon = runner.place_stack(
        0,
        &["VEM-1", "VEM-2", "VEM-3", "VEM-4", "BT21-062"],
    );

    runner
        .game
        .delete_permanent_with_cause(galacticmon, ReplacementCause::Battle);

    let view = runner.pending_selection_view();
    assert!(
        view.is_some(),
        "the would-leave prevention also offers on a BATTLE removal cause (no IsByEffect gate)"
    );
    assert_eq!(view.unwrap().kind, SelectionKind::Replacement);
}

/// Negative: a distractor Option that is NOT named "Ragnarok Cannon" is never
/// offered by clause 1's union-zone free-use pick.
#[test]
fn bt21_062_when_digivolving_ignores_non_cannon_options() {
    let mut runner = galacticmon_runner()
        .add_card(make_vemmon("VEM-1"))
        .add_card(make_vemmon("VEM-2"))
        .add_card(make_vemmon("VEM-3"))
        .add_card(make_vemmon("VEM-4"))
        .add_card(make_other_option("OTHER-OPT"))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .start();
    push_to_trash(&mut runner, 0, "VEM-1");
    push_to_trash(&mut runner, 0, "VEM-2");
    push_to_trash(&mut runner, 0, "VEM-3");
    push_to_trash(&mut runner, 0, "VEM-4");
    push_to_hand(&mut runner, 0, "OTHER-OPT");

    let galacticmon = runner.place_on_field(0, "BT21-062", Some(0));
    fire_when_digivolving(&mut runner, galacticmon);

    for _ in 0..4 {
        take_first_or_pass(&mut runner);
    }

    // No Ragnarok Cannon anywhere -> no union-zone prompt installs (the
    // distractor Option does not satisfy the `name_is: "Ragnarok Cannon"`
    // filter).
    assert!(
        runner.pending_selection().is_none(),
        "with no [Ragnarok Cannon] in hand/trash, no free-use prompt installs"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        1,
        "the distractor Option remains untouched in hand"
    );
}
