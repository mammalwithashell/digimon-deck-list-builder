//! EX4-003 Tsunomon — Digi-Egg, Lv.2, Black, Cost 0.
//! Traits: Lesser. Form: In-Training (DigiEgg slot).
//!
//! # Card text (cards.json inherited_effect_description_eng)
//!
//! Inherited Effect [Your Turn] [Once Per Turn]
//! When one of your other Digimon digivolves, <Draw 1>.
//! (Draw 1 card from your deck.)
//!
//! (DigiEgg has no main effect — only the inherited triggered draw.)
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/EX4/Black/EX4_003.cs`
//!
//! # Patterns this test covers
//! - G4 inherited triggered effect on a DigiEgg (`scope: inherited`).
//! - Inherited [Your Turn] [Once Per Turn] gating on `on_digivolve`.
//! - Owner + kind gates: `event_target_owner: you`, `event_target_kind: digimon`.
//! - Mandatory trigger (no `optional:` flag) — DCGO `isOptional: false`.
//!
//! # Known engine and DSL gaps affecting these tests
//!
//! G-INHERITED-DISPATCH: closed for battle-area below-top dispatch on
//! 2026-04-29 in general. The `EffectTiming::OnDigivolve` fan-out from
//! inherited (below-top) sources is still being audited card-by-card;
//! tests that REQUIRE the clause to actually fire from a Tsunomon source
//! sitting under another carrier are gated `#[ignore]`. Sister DigiEgg
//! tests (BT14-001, BT21-001, BT24-001, BT22-005) use the same gating
//! pattern.
//!
//! G-OPT-TRIGGERED: closed for permanent-backed queued triggered effects
//! on 2026-04-29; the OPT-lockout test is `#[ignore]`'d here only because
//! it depends on the fire-test prerequisite above.
//!
//! G-DSL-EVENT-TARGET-IS-OTHER: no DSL leaf to filter the digivolving
//! permanent against the source's own host carrier ("one of your OTHER
//! Digimon"). Sibling of G-DSL-EVENT-TARGET-IS-SELF (BT15-101).
//! The negative test for "Tsunomon's own carrier digivolves should NOT
//! draw" is `#[ignore]`'d behind this DSL gap (would over-fire today).

#![allow(dead_code)]

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::permanent::PermanentHandle;

const TSUNOMON_YAML: &str = include_str!("../../../cards/ex4/EX4-003.yaml");

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Vanilla filler with no traits. Used as carriers / draw fodder / cards
/// that should NOT trigger Tsunomon's draw.
fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.traits = vec![];
    c
}

/// A Lv.4 Digimon used as a digivolve target sitting on the battle area.
/// Used as the carrier that will be digivolved into.
fn make_lv3_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(3);
    c.dp = Some(3000);
    c
}

/// A Lv.4 Digimon used as the "digivolve into" hand card. Carries a printed
/// `Lv.3 Red, cost 0` evo_cost so `normal_digivolve_route_for_hand_card`
/// accepts the digivolve onto a `make_lv3_digimon` host (which is Lv.3 and
/// Red by `make_test_card` default). Without a non-empty `evo_costs`,
/// `rules_digivolve_memory_cost` returns None and the digivolve action is
/// rejected for a route mismatch — see the fire-test docstrings below.
fn make_lv4_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(4);
    c.dp = Some(5000);
    c.evo_costs = vec![EvoCost {
        card_color: 0, // Red — matches make_lv3_digimon's default Red color
        level: 3,
        memory_cost: 0,
    }];
    c
}

/// A Tamer — printed gate `event_target_kind: digimon` must reject any
/// Tamer event even though the engine will not normally fire OnDigivolve
/// for a Tamer. Kept for explicit kind-gate documentation.
fn make_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = digimon_engine::enums::CardKind::Tamer;
    c.traits = vec![];
    c
}

/// Standard runner: Tsunomon parsed from YAML, plus a generic carrier and
/// some filler so the deck is non-empty for draws.
fn tsunomon_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(TSUNOMON_YAML)
        .expect("EX4-003 YAML parses")
        .add_card(make_filler("CARRIER"))
        .add_card(make_filler("OTHER-CARRIER"))
        .add_card(make_filler("FILL"))
        .add_card(make_lv3_digimon("LV3"))
        .add_card(make_lv4_digimon("LV4"))
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start()
}

/// Place Tsunomon as a bottom digivolution source under a CARRIER permanent
/// on player 0's battle area. Mirrors the BT14-001 / BT22-005 helpers.
fn place_tsunomon_as_source(runner: &mut DebugRunner) -> PermanentHandle {
    let handle = runner.place_on_field(0, "CARRIER", Some(0));

    {
        use digimon_engine::card_source::CardSource;
        let game = runner.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "EX4-003")
            .expect("EX4-003 registered in card_data");
        let next = game.next_card_index();
        let mut tsunomon_src = CardSource::new(data_idx, 0, next);
        tsunomon_src.card_index = next;
        let perm = &mut game.players[0].battle_area[handle.index as usize];
        // Insert at bottom (index 0); CARRIER remains at top (last position).
        perm.card_sources.insert(0, tsunomon_src);
    }

    handle
}

/// Drain any pending selection by repeatedly picking the first legal action.
/// Capped to prevent infinite loops in test failures.
fn drain_pending(runner: &mut DebugRunner) {
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 30 {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .selecting_player;
        let a = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids[0];
        runner.game.resolve_selection(p, a).ok();
        runner.game.drain_effect_queue();
        steps += 1;
    }
}

// ─── Section 1: Structural assertions ───────────────────────────────────────

#[test]
fn ex4_003_compiles_as_digi_egg_kind() {
    let runner = tsunomon_runner();
    let compiled = runner
        .compiled_card("EX4-003")
        .expect("EX4-003 compiled card present");

    assert_eq!(
        compiled.kind,
        CompiledCardKind::DigiEgg,
        "EX4-003 must compile as kind: digi_egg"
    );
}

#[test]
fn ex4_003_has_exactly_one_triggered_clause() {
    let runner = tsunomon_runner();
    let compiled = runner
        .compiled_card("EX4-003")
        .expect("EX4-003 compiled card present");

    let triggered: Vec<&digimon_dsl::compiled::CompiledTriggeredClause> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(
        triggered.len(),
        1,
        "EX4-003 must have exactly one triggered clause (the inherited draw)"
    );
}

#[test]
fn ex4_003_inherited_clause_is_scope_inherited() {
    let runner = tsunomon_runner();
    let compiled = runner
        .compiled_card("EX4-003")
        .expect("EX4-003 compiled card present");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(
        triggered[0].scope,
        CompiledScope::Inherited,
        "Tsunomon's only clause must be scope: inherited (DigiEgg inherited draw)"
    );
}

#[test]
fn ex4_003_inherited_clause_when_on_digivolve() {
    let runner = tsunomon_runner();
    let compiled = runner
        .compiled_card("EX4-003")
        .expect("EX4-003 compiled card present");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert!(
        triggered[0].when.contains(&CompiledTiming::OnDigivolve),
        "clause must fire on OnDigivolve; got {:?}",
        triggered[0].when
    );
}

#[test]
fn ex4_003_inherited_clause_is_once_per_turn() {
    let runner = tsunomon_runner();
    let compiled = runner
        .compiled_card("EX4-003")
        .expect("EX4-003 compiled card present");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert!(
        triggered[0].once_per_turn,
        "clause must be once_per_turn (printed [Once Per Turn])"
    );
}

#[test]
fn ex4_003_inherited_clause_is_not_optional() {
    // Card text: "[Your Turn] [Once Per Turn] When one of your other Digimon
    // digivolves, <Draw 1>." No "you may"; mandatory. DCGO confirms
    // `isOptional: false`.
    let runner = tsunomon_runner();
    let compiled = runner
        .compiled_card("EX4-003")
        .expect("EX4-003 compiled card present");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert!(
        !triggered[0].optional,
        "clause is not optional per card text (no 'you may')"
    );
}

#[test]
fn ex4_003_traits_include_lesser() {
    // Self-trait sanity: Tsunomon carries [Lesser]. DigiEgg consumers that
    // search for [Lesser] DigiEggs (BT-series Lesser support) need this
    // surfaced on the compiled card.
    let runner = tsunomon_runner();
    let compiled = runner
        .compiled_card("EX4-003")
        .expect("EX4-003 compiled card present");

    let traits: Vec<String> = compiled.traits.iter().map(|t| t.to_lowercase()).collect();

    assert!(
        traits.iter().any(|t| t == "lesser"),
        "Tsunomon must carry [Lesser] trait; got {:?}",
        compiled.traits
    );
}

#[test]
fn ex4_003_is_cost_zero_lv2_black_in_training() {
    // Sanity: DigiEgg metadata matches printed values.
    let runner = tsunomon_runner();
    let compiled = runner
        .compiled_card("EX4-003")
        .expect("EX4-003 compiled card present");

    assert_eq!(compiled.level, Some(2), "Tsunomon must be Lv.2");
    // cost may be None for digi_egg (no printed play cost), but the DSL
    // surfaces it as 0 when given. Either is acceptable per spec.
    let cost = compiled.cost.unwrap_or(0);
    assert_eq!(cost, 0, "Tsunomon must be cost 0; got {cost}");
}

// ─── Section 2: Behavioral firing (POSITIVE — gated by G-INHERITED-DISPATCH) ─

/// POSITIVE: Tsunomon as inherited source under a P0 carrier; P0
/// digivolves a SEPARATE carrier on their own turn. Clause should fire → P0 draws 1.
///
/// Hand delta: -1 (digivolve consumes the LV4 from hand) + 1 (Tsunomon's
/// <Draw 1>) = 0. Deck delta: -1.
///
/// G-INHERITED-DISPATCH closed 2026-05-17 (Phase 2 Track D); the inherited
/// OnDigivolve fan-out from below-top sources now dispatches through
/// `enqueue_from_permanent` (Track D regression test in `timing_dispatch.rs`
/// exercises this surface). The test-fixture blocker is also closed:
/// `make_lv4_digimon` now carries a `Lv.3 Red, cost 0` `evo_cost` so
/// `normal_digivolve_route_for_hand_card` accepts the digivolve onto a
/// `make_lv3_digimon` (Lv.3 Red) host, the OnDigivolve event fires, and
/// Tsunomon's inherited clause reacts.
#[test]
fn ex4_003_inherited_fires_draws_one_on_other_digimon_digivolve() {
    let mut runner = tsunomon_runner();
    let _carrier = place_tsunomon_as_source(&mut runner);

    // Place a SEPARATE Lv.3 carrier on P0's field that we will digivolve
    // into. This is a different permanent from Tsunomon's host carrier, so
    // it satisfies the "other Digimon" requirement (DCGO `permanent !=
    // card.PermanentOfThisCard()`).
    let other_handle = runner.place_on_field(0, "LV3", Some(0));

    // Seed P0's hand with a Lv.4 digimon to digivolve into.
    {
        use digimon_engine::card_source::CardSource;
        let game = runner.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "LV4")
            .expect("LV4 registered in card_data");
        let next = game.next_card_index();
        let mut src = CardSource::new(data_idx, 0, next);
        src.card_index = next;
        game.players[0].hand.push(src);
    }

    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);

    // Find LV4 in hand and digivolve onto the OTHER carrier (NOT Tsunomon's
    // own carrier).
    let hand_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "LV4")
        .expect("LV4 in P0 hand");
    let _ = runner.game.digivolve_from_hand(
        0,
        hand_idx,
        other_handle.index as usize,
        digimon_engine::enums::PlaySource::ByDigivolve,
    );
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    let hand_after = runner.hand_size(0);
    let deck_after = runner.deck_size(0);

    // Note: digivolve_from_hand draws 1 by default for the digivolve itself.
    // The Tsunomon trigger is an ADDITIONAL Draw 1. So:
    //   hand: -1 (LV4 consumed) +1 (digivolve draw) +1 (Tsunomon draw) = +1
    //   deck: -1 (digivolve draw) -1 (Tsunomon draw) = -2
    assert_eq!(
        hand_after as i32 - hand_before as i32,
        1,
        "hand grows by 1 net (-1 LV4 +1 digivolve-draw +1 Tsunomon-draw); got {hand_before} -> {hand_after}"
    );
    assert_eq!(
        deck_before as i32 - deck_after as i32,
        2,
        "deck shrinks by 2 (1 digivolve-draw + 1 Tsunomon-draw); got {deck_before} -> {deck_after}"
    );
}

/// NEGATIVE (live — does NOT depend on G-INHERITED-DISPATCH): on the
/// opponent's turn, even if P1 digivolves, Tsunomon's clause must NOT fire
/// (the [Your Turn] gate on `active_when: { your_turn: true }` rejects),
/// AND `event_target_owner: you` rejects opponent-controlled digivolves.
/// P0's hand must not grow.
///
/// Even when G-INHERITED-DISPATCH closes for OnDigivolve, the gates here
/// must keep this green.
#[test]
fn ex4_003_inherited_does_not_fire_on_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(TSUNOMON_YAML)
        .expect("EX4-003 YAML parses")
        .add_card(make_filler("CARRIER"))
        .add_card(make_filler("FILL"))
        .add_card(make_lv3_digimon("LV3"))
        .add_card(make_lv4_digimon("LV4"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .hand(1, &["LV4"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let _carrier = place_tsunomon_as_source(&mut runner);

    // Give P1 a Lv.3 carrier on their field to digivolve.
    let p1_handle = runner.place_on_field(1, "LV3", Some(0));

    // Advance to P1's turn. `end_turn` flips the memory seesaw — the
    // builder's `.memory(10)` was set from P0's perspective, so after the
    // flip P1 holds -10. Re-set memory to a P1-favorable value so P1's
    // cost-0 digivolve does NOT immediately end P1's turn via
    // `check_turn_end` (which would rotate back to P0 and trigger P0's
    // turn-start draw, polluting the hand/deck deltas this test measures).
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 1, "precondition: now P1's turn");
    runner.game.set_memory(10);

    let hand0_before = runner.hand_size(0);
    let deck0_before = runner.deck_size(0);

    // P1 digivolves their LV3 with their hand LV4. Tsunomon (under P0
    // carrier) must NOT draw — fails BOTH `event_target_owner: you` (P1's
    // carrier) AND `active_when: your_turn` (P1's turn).
    let p1_hand_idx = runner.game.players[1]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "LV4")
        .expect("LV4 in P1 hand");
    let _ = runner.game.digivolve_from_hand(
        1,
        p1_hand_idx,
        p1_handle.index as usize,
        digimon_engine::enums::PlaySource::ByDigivolve,
    );
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    let hand0_after = runner.hand_size(0);
    let deck0_after = runner.deck_size(0);

    assert_eq!(
        hand0_after, hand0_before,
        "EX4-003 inherited clause must NOT fire on opponent's turn / for opponent's plays; \
         P0 hand grew unexpectedly: {hand0_before} -> {hand0_after}"
    );
    assert_eq!(
        deck0_after, deck0_before,
        "EX4-003 inherited clause must NOT touch P0's deck on opponent's turn; \
         deck0: {deck0_before} -> {deck0_after}"
    );
}

/// NEGATIVE: if Tsunomon's OWN host carrier digivolves, the trigger must be
/// suppressed ("one of your OTHER Digimon"). The clause's
/// `event_permanent_is_source: false` predicate compares the digivolving
/// permanent against Tsunomon's source carrier and rejects the fire when
/// they are the same — faithfully implementing DCGO's
/// `permanent != card.PermanentOfThisCard()`.
#[test]
fn ex4_003_inherited_does_not_fire_on_own_carrier_digivolve() {
    let mut runner = tsunomon_runner();
    let carrier = place_tsunomon_as_source(&mut runner);

    // Seed P0's hand with a Lv.4 digimon to digivolve INTO Tsunomon's own
    // host carrier (so the digivolving permanent IS Tsunomon's
    // `PermanentOfThisCard()`).
    {
        use digimon_engine::card_source::CardSource;
        let game = runner.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "LV4")
            .expect("LV4 registered in card_data");
        let next = game.next_card_index();
        let mut src = CardSource::new(data_idx, 0, next);
        src.card_index = next;
        game.players[0].hand.push(src);
    }

    let deck_before = runner.deck_size(0);

    let hand_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "LV4")
        .expect("LV4 in P0 hand");
    let _ = runner.game.digivolve_from_hand(
        0,
        hand_idx,
        carrier.index as usize,
        digimon_engine::enums::PlaySource::ByDigivolve,
    );
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    let deck_after = runner.deck_size(0);

    // Expected (per printed text + DCGO): only the digivolve's mandatory
    // draw fires (-1 deck), Tsunomon's clause does NOT fire because the
    // digivolving permanent IS Tsunomon's own carrier.
    assert_eq!(
        deck_before as i32 - deck_after as i32,
        1,
        "deck must shrink by 1 ONLY (the digivolve's mandatory draw); \
         Tsunomon's clause must NOT add a second draw because the digivolving \
         permanent IS Tsunomon's own host carrier (DCGO permanent != PermanentOfThisCard); \
         got {deck_before} -> {deck_after}"
    );
}

// ─── Section 3: OPT enforcement ─────────────────────────────────────────────

/// OPT: a SECOND qualifying digivolve in the same turn must NOT trigger
/// the draw a second time.
///
/// G-INHERITED-DISPATCH closed 2026-05-17 (Phase 2 Track D); G-OPT-TRIGGERED
/// closed in Phase 2 Track C. The LV3/LV4 test-fixture blocker is also
/// closed — `make_lv4_digimon` now carries a `Lv.3 Red, cost 0` `evo_cost`
/// so both digivolves complete.
#[test]
fn ex4_003_inherited_opt_blocks_second_trigger_same_turn() {
    let mut runner = tsunomon_runner();
    let _carrier = place_tsunomon_as_source(&mut runner);

    // Two SEPARATE other carriers on P0's field, each Lv.3 to digivolve.
    let other1 = runner.place_on_field(0, "LV3", Some(0));
    let other2 = runner.place_on_field(0, "LV3", Some(0));

    // Seed P0's hand with two Lv.4 digimon for the two digivolves.
    {
        use digimon_engine::card_source::CardSource;
        let game = runner.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "LV4")
            .expect("LV4 registered in card_data");
        for _ in 0..2 {
            let next = game.next_card_index();
            let mut src = CardSource::new(data_idx, 0, next);
            src.card_index = next;
            game.players[0].hand.push(src);
        }
    }

    let _h0 = runner.hand_size(0);
    let d0 = runner.deck_size(0);

    // First digivolve: fires OPT → +1 Tsunomon draw on top of the
    // digivolve-mandatory draw. Deck: -2 total.
    let idx1 = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "LV4")
        .expect("first LV4 in P0 hand");
    let _ = runner.game.digivolve_from_hand(
        0,
        idx1,
        other1.index as usize,
        digimon_engine::enums::PlaySource::ByDigivolve,
    );
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    let first_deck_delta = d0 as i32 - runner.deck_size(0) as i32;
    assert_eq!(
        first_deck_delta, 2,
        "first digivolve: deck shrinks by 2 (1 digivolve-mandatory + 1 Tsunomon-draw); got {first_deck_delta}"
    );

    if runner.game_over() {
        return;
    }

    // Second digivolve same turn: OPT used — Tsunomon's clause must NOT
    // fire a second time. Deck shrinks by 1 only (the digivolve-mandatory
    // draw).
    let d1 = runner.deck_size(0);

    let idx2 = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "LV4")
        .expect("second LV4 in P0 hand");
    let _ = runner.game.digivolve_from_hand(
        0,
        idx2,
        other2.index as usize,
        digimon_engine::enums::PlaySource::ByDigivolve,
    );
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    let second_deck_delta = d1 as i32 - runner.deck_size(0) as i32;
    assert_eq!(
        second_deck_delta, 1,
        "second digivolve (OPT locked): deck must shrink by 1 only (digivolve-mandatory draw, \
         no Tsunomon extra); got {second_deck_delta}"
    );
}
