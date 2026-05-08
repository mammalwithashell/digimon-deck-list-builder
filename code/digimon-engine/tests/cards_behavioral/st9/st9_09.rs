//! ST9-09 Stingmon — Digimon, Lv.4, Green, DP 4000, Cost 4.
//! Traits: Insectoid
//! Evo: Lv.3 Green / cost 2
//!
//! # Card text (cards.json — verbatim)
//!
//! **Effect:**
//! When you would play this card from your hand, if you have a blue Digimon in
//! play, reduce its play cost by 1.
//!
//! **Inherited Effect:**
//! [When Attacking] If you have a blue Digimon in play, ＜Draw 1＞ (Draw 1 card
//! from your deck.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST9/Green/ST9_09.cs
//!
//! # Patterns this test covers
//!
//! - Clause 0 (D2): BeforePayCost cost_reduction with `when_playing_this: true`,
//!   `amount: 1`, conditioned on `any_permanent { of: you, kind: digimon,
//!   color_is: blue, zone: [battle_area] }`.
//!   - Positive: blue Digimon in play → cost paid is 3 (4 - 1).
//!   - Negative: no blue Digimon in play → cost paid is full 4.
//!   - Negative: non-blue Digimon in play → cost paid is full 4.
//!   - Negative: cost reduction does not leak to other cards.
//!
//! - Clause 1 (G4): Inherited [When Attacking] conditional Draw 1, gated on
//!   `any_permanent { of: you, kind: digimon, color_is: blue, zone: [battle_area] }`.
//!   - Positive: Stingmon as source under carrier with blue Digimon on field →
//!     attack draws 1 card.
//!   - Negative: no blue Digimon on field → no draw from inherited trigger.
//!
//! # Faithfulness notes (DCGO comparison)
//!
//! - DCGO `CanUseCondition` (clause 0): `card.Owner.HandCards.Contains(card)` AND
//!   `HasMatchConditionOwnersPermanent(p => p.IsDigimon && p.TopCard.CardColors.Contains(Blue))`.
//!   DSL: `when_playing_this: true` encodes the "this card from hand" context;
//!   `any_permanent { of: you, kind: digimon, color_is: blue, zone: [battle_area] }` encodes the
//!   board condition.
//! - DCGO `CardSourceCondition`: `cardSource == card` → `when_playing_this: true` in DSL.
//! - DCGO `RootCondition`: `root == SelectCardEffect.Root.Hand` → `when_playing_this: true`
//!   (BeforePayCost only fires when playing this specific card from hand in the DSL pipeline).
//! - DCGO `isUpDown: true` → cost reduction (not increase) → `amount: 1` reduces.
//! - Clause 1 `CanActivateCondition` (DCGO): `IsExistOnBattleArea(card) &&
//!   HasMatchConditionOwnersPermanent(p => p.IsDigimon && p.TopCard.CardColors.Contains(Blue)) &&
//!   Owner.LibraryCards.Count >= 1`. The deck-empty guard is implicit in the `draw` step.
//! - Both clauses are MANDATORY (no "may" in printed text). `optional: false`.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
    CompiledTriggeredClause,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, PlayerId};
use digimon_engine::permanent::PermanentHandle;

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// A blue Digimon that qualifies for both clause gates.
fn make_blue_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c.level = Some(4);
    c.dp = Some(5000);
    c
}

/// A green Digimon (non-blue) that does NOT qualify for either gate.
fn make_green_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(4);
    c.dp = Some(5000);
    c
}

/// A filler card for padding decks.
fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A Lv.5 carrier (green, no special effects) for inherited clause tests.
fn make_lv5_carrier(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(5);
    c.dp = Some(6000);
    c
}

/// Base runner with ST9-09 loaded from the embedded DSL pack.
fn stingmon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("ST9-09")
        .expect("ST9-09 in embedded DSL pack")
        .add_card(make_blue_digimon("BLUE-DIGI"))
        .add_card(make_green_digimon("GREEN-DIGI"))
        .add_card(make_lv5_carrier("LV5-CARRIER"))
        .add_card(make_filler("DECK-PAD"))
        .deck(
            0,
            &["DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD"],
        )
        .deck(
            1,
            &["DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD"],
        )
        .memory(10)
        .start()
}

/// Place ST9-09 as a bottom source under the LV5-CARRIER on P0's field.
/// Returns the carrier permanent handle.
fn place_stingmon_as_source(runner: &mut DebugRunner) -> PermanentHandle {
    let handle = runner.place_on_field(0, "LV5-CARRIER", Some(0));
    {
        let game = runner.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "ST9-09")
            .expect("ST9-09 registered in card_data");
        let next = game.next_card_index();
        let mut stingmon_src = CardSource::new(data_idx, 0, next);
        stingmon_src.card_index = next;
        let perm = &mut game.players[0].battle_area[handle.index as usize];
        // Insert at bottom; carrier remains on top.
        perm.card_sources.insert(0, stingmon_src);
    }
    handle
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// ST9-09 YAML must be present in the embedded DSL pack and compile cleanly.
#[test]
fn st9_09_yaml_compiles_and_is_in_pack() {
    let runner = stingmon_runner();
    assert!(
        runner.compiled_card("ST9-09").is_some(),
        "ST9-09 must be present in the embedded DSL pack"
    );
}

/// ST9-09 must compile to exactly 1 CostReduction declarative clause and
/// 1 inherited Triggered clause (when_attacking).
#[test]
fn st9_09_has_one_cost_reduction_and_one_inherited_triggered() {
    let runner = stingmon_runner();
    let card = runner.compiled_card("ST9-09").expect("ST9-09 in pack");

    let cost_reduction_count = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
            )
        })
        .count();

    let triggered: Vec<&CompiledTriggeredClause> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(
        cost_reduction_count, 1,
        "ST9-09 must have exactly 1 cost_reduction declarative clause (clause 0)"
    );
    assert_eq!(
        triggered.len(),
        1,
        "ST9-09 must have exactly 1 triggered clause (inherited when_attacking)"
    );
    assert_eq!(
        card.effects.len(),
        2,
        "total compiled clauses must be 2 (1 cost_reduction + 1 triggered)"
    );
}

/// The inherited triggered clause must be WhenAttacking, Inherited scope, not optional,
/// and NOT once_per_turn (printed text has no [OPT]).
#[test]
fn st9_09_inherited_when_attacking_clause_shape() {
    let runner = stingmon_runner();
    let card = runner.compiled_card("ST9-09").expect("ST9-09 in pack");

    let clause: &CompiledTriggeredClause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if matches!(t.scope, CompiledScope::Inherited)
                    && t.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("must have an inherited WhenAttacking triggered clause");

    assert_eq!(
        clause.scope,
        CompiledScope::Inherited,
        "when_attacking clause must be Inherited scope"
    );
    assert!(
        !clause.optional,
        "Draw 1 is mandatory per printed text (no 'you may')"
    );
    assert!(
        !clause.once_per_turn,
        "printed text has no [Once Per Turn] restriction on the inherited clause"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Behavioral: Clause 0 cost reduction gating
// ═══════════════════════════════════════════════════════════════════════════════

/// POSITIVE: With a blue Digimon in play, cost is reduced by 1 (4 → 3).
#[test]
fn st9_09_cost_reduction_applies_with_blue_digimon_in_play() {
    let mut runner = stingmon_runner();
    runner.place_on_field(0, "BLUE-DIGI", None);
    runner.game_mut().players[0].hand.clear();

    // Refill hand with ST9-09 and put memory at 10.
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "ST9-09")
        .expect("ST9-09 in card_data");
    let next = runner.game.next_card_index();
    let src = CardSource::new(data_idx, 0, next);
    runner.game_mut().players[0].hand.push(src);

    let mem_before = runner.memory();
    runner.play(0, 0).expect("ST9-09 plays from hand");
    let cost = mem_before - runner.memory();

    assert_eq!(
        cost, 3,
        "with blue Digimon in play, ST9-09 cost must be 4-1=3; got {cost}"
    );
}

/// NEGATIVE: With no blue Digimon in play, the full cost of 4 is paid.
#[test]
fn st9_09_cost_reduction_blocked_without_blue_digimon() {
    let mut runner = stingmon_runner();
    // No blue Digimon placed on field — only clear the hand and put ST9-09 in it.
    runner.game_mut().players[0].hand.clear();

    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "ST9-09")
        .expect("ST9-09 in card_data");
    let next = runner.game.next_card_index();
    let src = CardSource::new(data_idx, 0, next);
    runner.game_mut().players[0].hand.push(src);

    let mem_before = runner.memory();
    runner.play(0, 0).expect("ST9-09 plays from hand");
    let cost = mem_before - runner.memory();

    assert_eq!(
        cost, 4,
        "with no blue Digimon in play, full cost of 4 must be paid; got {cost}"
    );
}

/// NEGATIVE: A non-blue Digimon (e.g. Green) does NOT satisfy the blue gate.
#[test]
fn st9_09_cost_reduction_blocked_by_non_blue_digimon() {
    let mut runner = stingmon_runner();
    runner.place_on_field(0, "GREEN-DIGI", None);
    runner.game_mut().players[0].hand.clear();

    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "ST9-09")
        .expect("ST9-09 in card_data");
    let next = runner.game.next_card_index();
    let src = CardSource::new(data_idx, 0, next);
    runner.game_mut().players[0].hand.push(src);

    let mem_before = runner.memory();
    runner.play(0, 0).expect("ST9-09 plays from hand");
    let cost = mem_before - runner.memory();

    assert_eq!(
        cost, 4,
        "non-blue Digimon must NOT satisfy the blue-gate condition; got cost {cost}"
    );
}

/// NEGATIVE: Cost reduction must not leak to other cards played from hand.
#[test]
fn st9_09_cost_reduction_does_not_leak_to_other_cards() {
    let mut other = make_green_digimon("OTHER-CARD");
    other.play_cost = 5;

    let mut runner = DebugRunner::builder()
        .dsl_card("ST9-09")
        .expect("ST9-09 in embedded DSL pack")
        .add_card(make_blue_digimon("BLUE-DIGI"))
        .add_card(other)
        .hand(0, &["OTHER-CARD"])
        .memory(10)
        .start();

    runner.place_on_field(0, "BLUE-DIGI", None);

    let mem_before = runner.memory();
    runner.play(0, 0).expect("OTHER-CARD plays");
    let cost = mem_before - runner.memory();

    assert_eq!(
        cost, 5,
        "ST9-09's cost reduction must not leak to OTHER-CARD; expected 5, got {cost}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral: Clause 1 inherited [When Attacking] Draw 1
// ═══════════════════════════════════════════════════════════════════════════════

/// POSITIVE: Stingmon as source under a carrier, blue Digimon on field → attack
/// triggers the inherited When Attacking, which draws 1 card.
#[test]
fn st9_09_inherited_when_attacking_draws_with_blue_digimon_in_play() {
    let mut runner = stingmon_runner();
    // Clear P1 security so we can attack player directly.
    runner.game_mut().players[1].security.clear();

    let carrier = place_stingmon_as_source(&mut runner);
    runner.place_on_field(0, "BLUE-DIGI", None);

    let hand_before = runner.hand_size(0);

    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();

    let hand_after = runner.hand_size(0);
    assert!(
        hand_after >= hand_before + 1,
        "carrier with Stingmon source + blue Digimon in play must draw at least 1; \
         hand before={hand_before} after={hand_after}"
    );
}

/// NEGATIVE: Stingmon as source under a carrier, NO blue Digimon on field → no draw.
#[test]
fn st9_09_inherited_when_attacking_no_draw_without_blue_digimon() {
    let mut runner = stingmon_runner();
    // Clear P1 security so we can attack player directly.
    runner.game_mut().players[1].security.clear();

    // Only place a non-blue Digimon (the carrier itself is the only Digimon, but it
    // is Green, so no blue-gate match).
    let carrier = place_stingmon_as_source(&mut runner);
    // Do NOT place a blue Digimon.

    let hand_before = runner.hand_size(0);

    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();

    let hand_after = runner.hand_size(0);
    assert_eq!(
        hand_after, hand_before,
        "without blue Digimon in play the inherited draw must NOT fire; \
         hand before={hand_before} after={hand_after}"
    );
}
