//! BT25-009 Bearmon — Digimon, Lv.3, Red, DP 1000, Cost 3.
//! Traits: Beast, Iliad, TS. Attribute: Vaccine.
//! Evo: Lv.2 Red / cost 0 (+ DCGO self-digivolution-requirement on a
//! [TS]-trait Lv.2 base, cost 0).
//!
//! # Card text (cards.json — verbatim)
//!
//! [Start of Your Main Phase] If you have 4 or less memory, this Digimon may
//! digivolve into a Digimon card with [Beast], [Animal] or [Sovereign], other
//! than [Sea Animal], in any of its traits or the [TS] trait in the hand
//! without paying the cost.
//!
//! Inherited Effect:
//! [All Turns] This Digimon gets +1000 DP.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Red/BT25_009.cs
//!
//! OnStartMainPhase ActivateClass:
//!   - CanActivateCondition: IsExistOnBattleAreaDigimon && IsOwnerTurn &&
//!       Owner.MemoryForPlayer <= 4.
//!   - SetUpActivateClass(..., -1, true, ...) → isOptional: true ("may").
//!   - Body: DigivolveIntoHandOrTrashCard(this, condition: IsDigimon &&
//!       (HasBeastTraits || HasTSTraits), payCost: false, isHand: true).
//! EffectTiming.None static effects:
//!   - AddSelfDigivolutionRequirementStaticEffect(TopCard.HasTSTraits,
//!       digivolutionCost: 0, level: 2).
//!   - ChangeSelfDPStaticEffect(1000, isInheritedEffect: true).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - B (start-of-main-phase triggered clause, memory-gated, optional)
//! - F (effect-initiated digivolve self from hand, free)
//! - G (inherited continuous +DP aura)
//! - H (alt digivolution path)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::CardKind;
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-009";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

/// A Lv.4 Digimon with the given traits, evolvable from a Lv.3 colorless base
/// so the effect-initiated self-digivolve onto Bearmon (Lv.3) matches the
/// rules digivolve route under `ignore_requirements: true`.
fn make_evo_target(id: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(5000);
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 3,
        memory_cost: 2,
    }];
    card
}

fn bearmon_base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-009 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_evo_target("BEAST-LV4", &["Beast"]))
        .add_card(make_evo_target("ANIMAL-LV4", &["Animal"]))
        .add_card(make_evo_target("SOVEREIGN-LV4", &["Sovereign"]))
        .add_card(make_evo_target("TS-LV4", &["TS"]))
        .add_card(make_evo_target("SEAANIMAL-LV4", &["Sea Animal"]))
        .add_card(make_evo_target("INSECT-LV4", &["Insectoid"]))
        .deck(0, &["DECK-PAD"; 6])
        .deck(1, &["DECK-PAD"; 6])
}

fn place_bearmon(runner: &mut DebugRunner) -> PermanentHandle {
    runner.place_on_field(0, CARD_ID, Some(0))
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_009_yaml_has_printed_metadata() {
    let runner = bearmon_base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT25-009 must be present in embedded DSL pack");

    assert_eq!(card.name, "Bearmon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(1000));
    for trait_name in ["Beast", "Iliad", "TS"] {
        assert!(
            card.traits.contains(&trait_name.to_string()),
            "BT25-009 metadata must include trait {trait_name}"
        );
    }
}

#[test]
fn bt25_009_has_ts_level2_cost0_alt_path() {
    let runner = bearmon_base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let path = card
        .alt_paths
        .iter()
        .find(|p| {
            p.kind == CompiledAltPathKind::Digivolve
                && p.from
                    .as_ref()
                    .and_then(|pred| pred.level_eq.as_ref())
                    .is_some()
        })
        .expect("BT25-009 must include a digivolve alt-path");

    let from = path.from.as_ref().unwrap();
    assert_eq!(
        from.trait_has.as_deref(),
        Some("TS"),
        "alt-path base must require the [TS] trait (DCGO TopCard.HasTSTraits)"
    );
    assert_eq!(
        path.cost,
        Some(CompiledCost::Literal(0)),
        "TS alt digivolve path must cost 0"
    );
}

#[test]
fn bt25_009_somp_clause_is_optional_and_memory_gated() {
    let runner = bearmon_base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::StartOfYourMainPhase) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("BT25-009 must have a StartOfYourMainPhase triggered clause");

    assert_eq!(clause.scope, CompiledScope::FaceUp);
    assert!(clause.optional, "printed text says 'may' → clause optional");
    assert!(!clause.once_per_turn, "printed text has no [Once Per Turn]");
    let active = clause
        .active_when
        .as_ref()
        .expect("clause must be gated by your_turn + memory_lte: 4");
    let mentions_memory =
        active.memory_lte.is_some() || active.all_of.iter().any(|p| p.memory_lte.is_some());
    assert!(
        mentions_memory,
        "'If you have 4 or less memory' must compile to a memory_lte gate"
    );
}

#[test]
fn bt25_009_somp_clause_digivolves_self_from_hand_for_free() {
    let runner = bearmon_base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::StartOfYourMainPhase) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("SOMP clause present");

    let has_eid = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::EffectInitiatedDigivolve { .. }));
    assert!(
        has_eid,
        "SOMP clause body must contain an effect_initiated_digivolve step"
    );
}

#[test]
fn bt25_009_has_inherited_dp_aura() {
    let runner = bearmon_base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let aura = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope: CompiledScope::Inherited,
                dp_modifier: Some(1000),
                ..
            })
        )
    });
    assert!(aura, "BT25-009 must have an inherited +1000 DP aura");
}

// ─── Section 2 — Behavior: SOMP free-digivolve ───────────────────────────────

#[test]
fn bt25_009_somp_free_digivolves_into_beast_from_hand() {
    let mut runner = bearmon_base().hand(0, &["BEAST-LV4"]).memory(3).start();
    let bear = place_bearmon(&mut runner);

    let hand_before = runner.hand_size(0);
    runner.game.enter_main_phase();

    assert!(
        runner.pending_selection().is_some(),
        "SOMP clause must install a player-visible prompt (the printed 'may')"
    );

    runner
        .accept_optional_trigger()
        .expect("accept the optional SOMP trigger");
    runner
        .auto_resolve()
        .expect("pick hand card + free digivolve resolves");

    let perm = &runner.game.players[0].battle_area[bear.index as usize];
    assert_eq!(
        perm.top_card().card_id(&runner.game.card_data),
        "BEAST-LV4",
        "Bearmon must have digivolved into the [Beast] hand card"
    );
    assert!(
        perm.card_sources.len() >= 2,
        "digivolution stack must now hold Bearmon underneath the Lv.4 result"
    );
    // §8-1-3-3: digivolving draws 1 card, so the hand SIZE is unchanged --
    // assert the [Beast] card's departure by identity, not by arithmetic.
    assert!(
        !runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "BEAST-LV4"),
        "the [Beast] card left hand to become the new top card"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "the digivolve draw replaced it (§8-1-3-3)"
    );
}

#[test]
fn bt25_009_somp_accepts_animal_trait_hand_card() {
    let mut runner = bearmon_base().hand(0, &["ANIMAL-LV4"]).memory(3).start();
    let bear = place_bearmon(&mut runner);
    runner.game.enter_main_phase();
    assert!(
        runner.pending_selection().is_some(),
        "an [Animal] Digimon must be an eligible evolution target"
    );
    runner.accept_optional_trigger().expect("accept");
    runner.auto_resolve().expect("resolve");
    let perm = &runner.game.players[0].battle_area[bear.index as usize];
    assert_eq!(
        perm.top_card().card_id(&runner.game.card_data),
        "ANIMAL-LV4"
    );
}

#[test]
fn bt25_009_somp_accepts_sovereign_trait_hand_card() {
    let mut runner = bearmon_base().hand(0, &["SOVEREIGN-LV4"]).memory(3).start();
    let _bear = place_bearmon(&mut runner);
    runner.game.enter_main_phase();
    assert!(
        runner.pending_selection().is_some(),
        "a [Sovereign] Digimon must be an eligible evolution target"
    );
}

#[test]
fn bt25_009_somp_accepts_ts_trait_hand_card() {
    let mut runner = bearmon_base().hand(0, &["TS-LV4"]).memory(3).start();
    let _bear = place_bearmon(&mut runner);
    runner.game.enter_main_phase();
    assert!(
        runner.pending_selection().is_some(),
        "a [TS] Digimon must be an eligible evolution target"
    );
}

// ─── Section 2b — Negative gating ────────────────────────────────────────────

#[test]
fn bt25_009_somp_no_fire_with_5_or_more_memory() {
    let mut runner = bearmon_base().hand(0, &["BEAST-LV4"]).memory(5).start();
    let _bear = place_bearmon(&mut runner);
    runner.game.enter_main_phase();
    assert!(
        runner.pending_selection().is_none(),
        "memory > 4 → SOMP clause must not fire"
    );
}

#[test]
fn bt25_009_somp_excludes_sea_animal() {
    // [Sea Animal] is explicitly excluded ("other than [Sea Animal]"). A hand
    // holding ONLY a [Sea Animal] Lv.4 leaves the picker with no legal target.
    let mut runner = bearmon_base().hand(0, &["SEAANIMAL-LV4"]).memory(3).start();
    let _bear = place_bearmon(&mut runner);
    runner.game.enter_main_phase();
    assert!(
        runner.pending_selection().is_none(),
        "[Sea Animal] is excluded → SOMP clause is a no-op with only a Sea Animal in hand"
    );
}

#[test]
fn bt25_009_somp_no_fire_without_eligible_hand_card() {
    let mut runner = bearmon_base().hand(0, &["INSECT-LV4"]).memory(3).start();
    let _bear = place_bearmon(&mut runner);
    runner.game.enter_main_phase();
    assert!(
        runner.pending_selection().is_none(),
        "no eligible Digimon in hand → SOMP clause is a no-op"
    );
}

#[test]
fn bt25_009_somp_no_fire_on_opponents_turn() {
    let mut runner = bearmon_base().hand(0, &["BEAST-LV4"]).memory(3).start();
    let _bear = place_bearmon(&mut runner);
    runner.game.turn_player_idx = 1;
    runner.game.enter_main_phase();
    assert!(
        runner.pending_selection().is_none(),
        "[Start of Your Main Phase] must not fire on the opponent's turn"
    );
}

#[test]
fn bt25_009_somp_decline_keeps_bearmon() {
    let mut runner = bearmon_base().hand(0, &["BEAST-LV4"]).memory(3).start();
    let bear = place_bearmon(&mut runner);
    runner.game.enter_main_phase();
    assert!(runner.pending_selection().is_some(), "prompt installed");
    runner
        .decline_optional_trigger()
        .expect("declining the optional SOMP trigger is legal");
    let perm = &runner.game.players[0].battle_area[bear.index as usize];
    assert_eq!(
        perm.top_card().card_id(&runner.game.card_data),
        CARD_ID,
        "declining leaves Bearmon as the top card (no digivolve)"
    );
}

// ─── Section 3 — Behavior: inherited +1000 DP aura ───────────────────────────

#[test]
fn bt25_009_inherited_grants_plus_1000_dp_to_carrier() {
    let mut carrier = make_evo_target("CARRIER", &["Beast"]);
    carrier.dp = Some(5000);
    let mut runner = bearmon_base().add_card(carrier).memory(10).start();
    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    assert_eq!(
        runner.effective_dp(stack),
        Some(6000),
        "carrier base 5000 + inherited 1000 from Bearmon source = 6000"
    );
}
