//! BT22-005 Tsumemon — Digi-Egg, Lv.2, Black, Cost 0.
//! Traits: Unidentified, CS. Form: In-Training.
//!
//! # Card text (cards.json effect_description_eng)
//!
//! Inherited Effect [Your Turn] [Once Per Turn]
//! When any of your Digimon with the [Unidentified] or [CS] trait are played,
//! <Draw 1> (Draw 1 card from your deck.)
//!
//! (DigiEgg has no main effect — only the inherited triggered draw.)
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/BT22/Black/BT22_005.cs`
//!
//! # Patterns this test covers
//! - G4 inherited triggered effect on a DigiEgg (`scope: inherited`).
//! - Inherited [Your Turn] [Once Per Turn] gating on `on_enter_field_anyone`.
//! - First DSL card to use `event_target_trait_has` predicate (verified
//!   wired end-to-end through compile + eval_event_fields predicate.rs:429).
//! - Trait disjunction expressed as `any_of` over two single-trait
//!   `event_target_trait_has` leaves.
//! - Owner + kind gates: `event_target_owner: you`, `event_target_kind: digimon`.
//!
//! # Known engine and DSL gaps affecting these tests
//!
//! G-INHERITED-DISPATCH: `enqueue_from_permanent` only scans the top card
//! (+ linked_cards + Training inserts) for triggered effects. Inherited
//! clauses on cards in lower digivolution-stack slots (where Tsumemon lives
//! at runtime) are not enqueued today. Behavioral tests that require the
//! clause to actually fire are gated `#[ignore = "BLOCKED:
//! G-INHERITED-DISPATCH ..."]`. Same caveat as BT14-001 / BT21-001 /
//! BT24-001 sister DigiEgg DSL cards.
//!
//! G-OPT-TRIGGERED: `once_per_turn: true` compiles to
//! `Effect::max_per_turn=1` but `run_queued_effect_inner` does not yet
//! enforce per-turn lockout for triggered clauses. The OPT-lockout test
//! is `#[ignore]`'d alongside G-INHERITED-DISPATCH (OPT can't be tested
//! until the clause fires at all).

#![allow(dead_code)]

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::permanent::PermanentHandle;

const TSUMEMON_YAML: &str = include_str!("../../../cards/bt22/BT22-005.yaml");

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Vanilla filler with no traits. Used as carriers / draw fodder / tamers
/// that should NOT trigger Tsumemon's draw.
fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.traits = vec![];
    c
}

/// A Lv.4 Digimon with the [Unidentified] trait — should trigger Tsumemon's
/// inherited draw when played by Tsumemon's controller on their own turn.
fn make_unidentified_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(4);
    c.dp = Some(5000);
    c.traits = vec!["Unidentified".to_string()];
    c
}

/// A Lv.4 Digimon with the [CS] trait — should trigger Tsumemon's inherited
/// draw when played by Tsumemon's controller on their own turn.
fn make_cs_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(4);
    c.dp = Some(5000);
    c.traits = vec!["CS".to_string()];
    c
}

/// A Lv.4 Digimon with no [Unidentified] / [CS] trait — should NOT trigger
/// Tsumemon's inherited draw.
fn make_unrelated_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(4);
    c.dp = Some(5000);
    c.traits = vec!["Beast".to_string()];
    c
}

/// A Tamer with the [CS] trait — printed-text gate `event_target_kind: digimon`
/// must reject this; Tamer plays must NOT trigger the draw even if the
/// Tamer carries [CS] (matches DCGO `IsPermanentExistsOnOwnerBattleAreaDigimon`).
fn make_cs_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = digimon_engine::enums::CardKind::Tamer;
    c.traits = vec!["CS".to_string()];
    c
}

/// Standard runner: Tsumemon parsed from YAML, plus a generic carrier and
/// some filler so the deck is non-empty for draws.
fn tsumemon_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(TSUMEMON_YAML)
        .expect("BT22-005 YAML parses")
        .add_card(make_filler("CARRIER"))
        .add_card(make_filler("FILL"))
        .add_card(make_unidentified_digimon("UNID"))
        .add_card(make_cs_digimon("CSD"))
        .add_card(make_unrelated_digimon("OTHER"))
        .add_card(make_cs_tamer("CSTAMER"))
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start()
}

/// Place Tsumemon as a bottom digivolution source under a CARRIER permanent
/// on player 0's battle area. Mirrors the BT14-001 helper (sister DigiEgg
/// inherited-draw test).
fn place_tsumemon_as_source(runner: &mut DebugRunner) -> PermanentHandle {
    let handle = runner.place_on_field(0, "CARRIER", Some(0));

    {
        use digimon_engine::card_source::CardSource;
        let game = runner.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "BT22-005")
            .expect("BT22-005 registered in card_data");
        let next = game.next_card_index();
        let mut tsumemon_src = CardSource::new(data_idx, 0, next);
        tsumemon_src.card_index = next;
        let perm = &mut game.players[0].battle_area[handle.index as usize];
        // Insert at bottom (index 0); CARRIER remains at top (last position).
        perm.card_sources.insert(0, tsumemon_src);
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

/// Find a card's index in a player's hand by card_id.
fn find_hand_index(runner: &DebugRunner, player: u8, card_id: &str) -> Option<usize> {
    let data = &runner.game.card_data;
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(data) == card_id)
}

// ─── Section 1: Structural assertions ───────────────────────────────────────

#[test]
fn bt22_005_compiles_as_digi_egg_kind() {
    let runner = tsumemon_runner();
    let compiled = runner
        .compiled_card("BT22-005")
        .expect("BT22-005 compiled card present");

    assert_eq!(
        compiled.kind,
        CompiledCardKind::DigiEgg,
        "BT22-005 must compile as kind: digi_egg"
    );
}

#[test]
fn bt22_005_has_exactly_one_triggered_clause() {
    let runner = tsumemon_runner();
    let compiled = runner
        .compiled_card("BT22-005")
        .expect("BT22-005 compiled card present");

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
        "BT22-005 must have exactly one triggered clause (the inherited draw)"
    );
}

#[test]
fn bt22_005_inherited_clause_is_scope_inherited() {
    let runner = tsumemon_runner();
    let compiled = runner
        .compiled_card("BT22-005")
        .expect("BT22-005 compiled card present");

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
        "Tsumemon's only clause must be scope: inherited (DigiEgg inherited draw)"
    );
}

#[test]
fn bt22_005_inherited_clause_when_on_enter_field_anyone() {
    let runner = tsumemon_runner();
    let compiled = runner
        .compiled_card("BT22-005")
        .expect("BT22-005 compiled card present");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert!(
        triggered[0]
            .when
            .contains(&CompiledTiming::OnEnterFieldAnyone),
        "clause must fire on OnEnterFieldAnyone"
    );
}

#[test]
fn bt22_005_inherited_clause_is_once_per_turn() {
    let runner = tsumemon_runner();
    let compiled = runner
        .compiled_card("BT22-005")
        .expect("BT22-005 compiled card present");

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
fn bt22_005_inherited_clause_is_not_optional() {
    // Card text: "[Your Turn] [Once Per Turn] When any of your Digimon ... are
    // played, <Draw 1>." No "you may"; mandatory trigger. DCGO confirms
    // `isOptional: false`.
    let runner = tsumemon_runner();
    let compiled = runner
        .compiled_card("BT22-005")
        .expect("BT22-005 compiled card present");

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
fn bt22_005_traits_include_unidentified_and_cs() {
    // Self-trait sanity: Tsumemon itself carries [Unidentified] and [CS]
    // (printed traits per cards.json effect_description metadata). The
    // compiled card must surface both traits so it's recognized as a valid
    // alt-evolve / trait-search target by other cards.
    let runner = tsumemon_runner();
    let compiled = runner
        .compiled_card("BT22-005")
        .expect("BT22-005 compiled card present");

    let traits: Vec<String> = compiled.traits.iter().map(|t| t.to_lowercase()).collect();

    assert!(
        traits.iter().any(|t| t == "unidentified"),
        "Tsumemon must carry [Unidentified] trait; got {:?}",
        compiled.traits
    );
    assert!(
        traits.iter().any(|t| t == "cs"),
        "Tsumemon must carry [CS] trait; got {:?}",
        compiled.traits
    );
}

#[test]
fn bt22_005_is_cost_zero_lv2_black_in_training() {
    // Sanity: DigiEgg metadata matches printed values.
    let runner = tsumemon_runner();
    let compiled = runner
        .compiled_card("BT22-005")
        .expect("BT22-005 compiled card present");

    assert_eq!(compiled.level, Some(2), "Tsumemon must be Lv.2");
    // cost may be None for digi_egg (no printed play cost), but the
    // DSL surfaces it as 0 when given. Either is acceptable per spec.rs:21
    // ("Absent for digi_egg / token"). Normalise to 0 for the assertion.
    let cost = compiled.cost.unwrap_or(0);
    assert_eq!(cost, 0, "Tsumemon must be cost 0; got {cost}");
}

// ─── Section 2: Condition gating (your_turn) ────────────────────────────────

/// NEGATIVE: Tsumemon's inherited clause is gated by
/// `active_when: { your_turn: true }`. On the opponent's turn, even if the
/// opponent plays a Digimon with [Unidentified] / [CS] (or P0 somehow plays
/// one off-turn), the clause must NOT fire.
///
/// The most direct expression: P1 plays an [Unidentified] Digimon on P1's
/// turn while Tsumemon is under a P0 carrier; P0's hand must not grow.
///
/// This test is structurally correct and documents the intended gate.
/// Because G-INHERITED-DISPATCH is an active gap, the clause would not fire
/// anyway, so the assertion is true vacuously today.
#[test]
fn bt22_005_inherited_does_not_fire_on_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(TSUMEMON_YAML)
        .expect("BT22-005 YAML parses")
        .add_card(make_filler("CARRIER"))
        .add_card(make_filler("FILL"))
        .add_card(make_unidentified_digimon("UNID"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .hand(1, &["UNID"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let _carrier = place_tsumemon_as_source(&mut runner);

    // Advance to P1's turn.
    runner.end_turn();

    let hand0_before = runner.hand_size(0);

    // P1 plays the [Unidentified] Digimon. Tsumemon (under P0 carrier) must
    // NOT draw — both because the entering permanent's owner is P1 (fails
    // `event_target_owner: you`) AND because it's not P0's turn (fails
    // `active_when: your_turn`).
    let p1_hand_idx = find_hand_index(&runner, 1, "UNID").expect("UNID in P1 hand");
    let _ = runner.play(1, p1_hand_idx);
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    let hand0_after = runner.hand_size(0);

    assert_eq!(
        hand0_after, hand0_before,
        "BT22-005 inherited clause must NOT fire on opponent's turn / for opponent's plays; \
         P0 hand grew unexpectedly: {hand0_before} -> {hand0_after}"
    );
}

// ─── Section 3: Behavioral firing (POSITIVE — gated by G-INHERITED-DISPATCH) ─

/// POSITIVE (BLOCKED — G-INHERITED-DISPATCH): Tsumemon as inherited source
/// under a P0 carrier; P0 plays an [Unidentified] Digimon on P0's turn.
/// Clause fires → P0 draws 1.
///
/// Hand delta: -1 (played UNID) + 1 (Tsumemon's <Draw 1>) = 0. Deck delta: -1.
///
/// Blocked until engine gains digivolution-stack inherited triggered-effect
/// dispatch (`enqueue_from_permanent` iterates `card_sources[0..n-1]`).
/// Mirrors BT14-001 / BT21-001 / BT24-001 sister DigiEgg ignores.
#[test]
#[ignore = "BLOCKED: G-INHERITED-DISPATCH — engine lacks digivolution-stack inherited triggered-effect dispatch (enqueue_from_permanent only scans top card + linked_cards + Training)"]
fn bt22_005_inherited_fires_draws_one_on_unidentified_digimon_play() {
    let mut runner = tsumemon_runner();
    let _carrier = place_tsumemon_as_source(&mut runner);

    // Seed P0's hand with the [Unidentified] Digimon to be played.
    {
        use digimon_engine::card_source::CardSource;
        let game = runner.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "UNID")
            .expect("UNID registered in card_data");
        let next = game.next_card_index();
        let mut src = CardSource::new(data_idx, 0, next);
        src.card_index = next;
        game.players[0].hand.push(src);
    }

    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);

    let hand_idx = find_hand_index(&runner, 0, "UNID").expect("UNID in P0 hand");
    let _ = runner.play(0, hand_idx);
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    let hand_after = runner.hand_size(0);
    let deck_after = runner.deck_size(0);

    assert_eq!(
        hand_after, hand_before,
        "hand net unchanged: -1 played UNID + 1 from Tsumemon's <Draw 1>"
    );
    assert_eq!(
        deck_after,
        deck_before - 1,
        "deck must shrink by 1 from Tsumemon's <Draw 1>"
    );
}

/// POSITIVE (BLOCKED — G-INHERITED-DISPATCH): Same as above but for the [CS]
/// disjunction branch — confirms the `any_of` over `event_target_trait_has`
/// covers both traits.
#[test]
#[ignore = "BLOCKED: G-INHERITED-DISPATCH — engine lacks digivolution-stack inherited triggered-effect dispatch"]
fn bt22_005_inherited_fires_draws_one_on_cs_digimon_play() {
    let mut runner = tsumemon_runner();
    let _carrier = place_tsumemon_as_source(&mut runner);

    {
        use digimon_engine::card_source::CardSource;
        let game = runner.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "CSD")
            .expect("CSD registered in card_data");
        let next = game.next_card_index();
        let mut src = CardSource::new(data_idx, 0, next);
        src.card_index = next;
        game.players[0].hand.push(src);
    }

    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);

    let hand_idx = find_hand_index(&runner, 0, "CSD").expect("CSD in P0 hand");
    let _ = runner.play(0, hand_idx);
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "hand net unchanged: -1 played CSD + 1 from Tsumemon's <Draw 1>"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "deck must shrink by 1 from Tsumemon's <Draw 1>"
    );
}

/// NEGATIVE (live — does not depend on G-INHERITED-DISPATCH): when an
/// unrelated own Digimon (no [Unidentified] / [CS] trait) is played on P0's
/// turn with Tsumemon as a source, the trait-disjunction predicate must
/// reject it — no draw.
///
/// Even with G-INHERITED-DISPATCH closed, the condition gate must filter
/// out non-trait plays. Today the test passes vacuously (clause doesn't
/// fire at all), but it is structurally correct and will remain true
/// after dispatch is wired.
#[test]
fn bt22_005_inherited_does_not_fire_on_unrelated_digimon_play() {
    let mut runner = tsumemon_runner();
    let _carrier = place_tsumemon_as_source(&mut runner);

    {
        use digimon_engine::card_source::CardSource;
        let game = runner.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "OTHER")
            .expect("OTHER registered in card_data");
        let next = game.next_card_index();
        let mut src = CardSource::new(data_idx, 0, next);
        src.card_index = next;
        game.players[0].hand.push(src);
    }

    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);

    let hand_idx = find_hand_index(&runner, 0, "OTHER").expect("OTHER in P0 hand");
    let _ = runner.play(0, hand_idx);
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    // Played 1, drew 0 (no trait match). Hand: -1, deck: 0.
    assert_eq!(
        runner.hand_size(0),
        hand_before - 1,
        "hand must shrink by 1 (played OTHER, no draw fired) — {} -> {}",
        hand_before,
        runner.hand_size(0)
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "deck must NOT shrink (trait disjunction must reject [Beast]-only Digimon)"
    );
}

/// NEGATIVE (live — does not depend on G-INHERITED-DISPATCH): an own [CS]
/// Tamer being played must NOT trigger Tsumemon's draw. The printed text
/// scopes the trigger to "your **Digimon**", and DCGO enforces this via
/// `IsPermanentExistsOnOwnerBattleAreaDigimon`. Encoded in DSL as
/// `event_target_kind: digimon`.
///
/// As with the unrelated-Digimon case, this passes vacuously today
/// (G-INHERITED-DISPATCH); when dispatch closes, the `event_target_kind`
/// gate must keep it green.
#[test]
fn bt22_005_inherited_does_not_fire_on_cs_tamer_play() {
    let mut runner = tsumemon_runner();
    let _carrier = place_tsumemon_as_source(&mut runner);

    {
        use digimon_engine::card_source::CardSource;
        let game = runner.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "CSTAMER")
            .expect("CSTAMER registered in card_data");
        let next = game.next_card_index();
        let mut src = CardSource::new(data_idx, 0, next);
        src.card_index = next;
        game.players[0].hand.push(src);
    }

    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);

    let hand_idx = find_hand_index(&runner, 0, "CSTAMER").expect("CSTAMER in P0 hand");
    let _ = runner.play(0, hand_idx);
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    // Played 1 Tamer, drew 0 (kind gate rejects). Hand: -1, deck: 0.
    assert_eq!(
        runner.hand_size(0),
        hand_before - 1,
        "hand must shrink by 1 (played CSTAMER, no draw fired)"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "deck must NOT shrink (event_target_kind: digimon must reject Tamer plays \
         even if they carry [CS])"
    );
}

// ─── Section 4: OPT enforcement ─────────────────────────────────────────────

/// OPT: a SECOND qualifying Digimon play in the same turn must NOT trigger
/// the draw a second time.
///
/// BLOCKED: G-OPT-TRIGGERED — `max_per_turn` is not enforced in
/// `run_queued_effect_inner` for triggered effects. Also depends on
/// G-INHERITED-DISPATCH (the clause must fire at all before OPT can be
/// tested).
#[test]
#[ignore = "BLOCKED: G-OPT-TRIGGERED (and G-INHERITED-DISPATCH prerequisite) — OPT not enforced for triggered effects in queue drain path"]
fn bt22_005_inherited_opt_blocks_second_trigger_same_turn() {
    let mut runner = tsumemon_runner();
    let _carrier = place_tsumemon_as_source(&mut runner);

    // Seed two [Unidentified] Digimon into P0's hand.
    {
        use digimon_engine::card_source::CardSource;
        let game = runner.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "UNID")
            .expect("UNID registered in card_data");
        for _ in 0..2 {
            let next = game.next_card_index();
            let mut src = CardSource::new(data_idx, 0, next);
            src.card_index = next;
            game.players[0].hand.push(src);
        }
    }

    let h0 = runner.hand_size(0);
    let d0 = runner.deck_size(0);

    // First play: fires OPT → draw 1. Hand: -1 +1 = 0. Deck: -1.
    let idx1 = find_hand_index(&runner, 0, "UNID").expect("first UNID in P0 hand");
    let _ = runner.play(0, idx1);
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    let first_hand_delta = runner.hand_size(0) as i32 - h0 as i32;
    let first_deck_delta = d0 as i32 - runner.deck_size(0) as i32;

    assert_eq!(
        first_hand_delta, 0,
        "first play: hand net 0 (-1 played +1 drew); got {first_hand_delta}"
    );
    assert_eq!(
        first_deck_delta, 1,
        "first play: deck shrinks by 1 from Tsumemon's <Draw 1>; got {first_deck_delta}"
    );

    if runner.game_over() {
        return;
    }

    // Second play same turn: OPT used — clause must be locked.
    // Hand: -1 +0 = -1. Deck: 0.
    let h1 = runner.hand_size(0);
    let d1 = runner.deck_size(0);

    let idx2 = find_hand_index(&runner, 0, "UNID").expect("second UNID in P0 hand");
    let _ = runner.play(0, idx2);
    runner.game.drain_effect_queue();
    drain_pending(&mut runner);

    let second_hand_delta = runner.hand_size(0) as i32 - h1 as i32;
    let second_deck_delta = d1 as i32 - runner.deck_size(0) as i32;

    assert_eq!(
        second_hand_delta, -1,
        "second play (OPT locked): hand shrinks by 1 only (no extra draw); got {second_hand_delta}"
    );
    assert_eq!(
        second_deck_delta, 0,
        "second play (OPT locked): deck must not shrink; got {second_deck_delta}"
    );
}
