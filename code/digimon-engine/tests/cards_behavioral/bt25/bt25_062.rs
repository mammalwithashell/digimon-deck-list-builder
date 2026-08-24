//! BT25-062 Kokuwamon — Digimon, Lv.3, Black, DP 1000, Cost 3.
//! Traits: Machine, Iliad, TS. Attribute: Data.
//! Evo: Lv.2 Black / cost 0 (+ DCGO self-digivolution-requirement on a
//! [TS]-trait Lv.2 base, cost 0).
//!
//! # Card text (cards.json — verbatim)
//!
//! [Start of Your Main Phase] If you have 4 or less memory, this Digimon may
//! digivolve into a [Machine], [Cyborg] or [TS] trait Digimon card in the hand
//! without paying the cost.
//!
//! Inherited Effect:
//! [All Turns] This Digimon gets +1000 DP.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Black/BT25_062.cs
//!
//! OnStartMainPhase ActivateClass:
//!   - CanActivateCondition: IsExistOnBattleAreaDigimon && IsOwnerTurn &&
//!       Owner.MemoryForPlayer <= 4.
//!   - SetUpActivateClass(..., -1, true, ...) → isOptional: true ("may").
//!   - Body: DigivolveIntoHandOrTrashCard(this, condition: Digimon &&
//!       (Machine || Cyborg || HasTSTraits), payCost: false, isHand: true).
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
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT25-062";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

/// A Lv.4 Digimon with the given traits, evolvable from a Lv.3 colorless base
/// so the effect-initiated self-digivolve onto Kokuwamon (Lv.3) matches the
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

fn kokuwamon_base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-062 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_evo_target("MACHINE-LV4", &["Machine"]))
        .add_card(make_evo_target("CYBORG-LV4", &["Cyborg"]))
        .add_card(make_evo_target("TS-LV4", &["TS"]))
        .add_card(make_evo_target("BEAST-LV4", &["Beast"]))
        .deck(0, &["DECK-PAD"; 6])
        .deck(1, &["DECK-PAD"; 6])
}

/// Place Kokuwamon on P0's field as a standalone face-up Digimon (its own
/// top card), so the SOMP triggered clause and the field DP aura both apply.
fn place_kokuwamon(runner: &mut DebugRunner) -> PermanentHandle {
    runner.place_on_field(0, CARD_ID, Some(0))
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_062_yaml_has_printed_metadata() {
    let runner = kokuwamon_base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT25-062 must be present in embedded DSL pack");

    assert_eq!(card.name, "Kokuwamon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(1000));
    for trait_name in ["Machine", "Iliad", "TS"] {
        assert!(
            card.traits.contains(&trait_name.to_string()),
            "BT25-062 metadata must include trait {trait_name}"
        );
    }
}

#[test]
fn bt25_062_has_ts_level2_cost0_alt_path() {
    let runner = kokuwamon_base().start();
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
        .expect("BT25-062 must include a digivolve alt-path");

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
fn bt25_062_somp_clause_is_optional_and_memory_gated() {
    let runner = kokuwamon_base().start();
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
        .expect("BT25-062 must have a StartOfYourMainPhase triggered clause");

    assert_eq!(clause.scope, CompiledScope::FaceUp);
    assert!(
        clause.optional,
        "printed text says 'may' → clause must be optional"
    );
    assert!(!clause.once_per_turn, "printed text has no [Once Per Turn]");
    let active = clause
        .active_when
        .as_ref()
        .expect("clause must be gated by your_turn + memory_lte: 4");
    // The memory gate lives in an all_of compound.
    let mentions_memory =
        active.memory_lte.is_some() || active.all_of.iter().any(|p| p.memory_lte.is_some());
    assert!(
        mentions_memory,
        "'If you have 4 or less memory' must compile to a memory_lte gate"
    );
}

#[test]
fn bt25_062_somp_clause_digivolves_self_from_hand_for_free() {
    let runner = kokuwamon_base().start();
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
fn bt25_062_has_inherited_dp_aura() {
    let runner = kokuwamon_base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    use digimon_dsl::compiled::CompiledDeclarativeClause;
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
    assert!(aura, "BT25-062 must have an inherited +1000 DP aura");
}

// ─── Section 2 — Behavior: SOMP free-digivolve ───────────────────────────────

/// Positive: <=4 memory on your turn, a [Machine] Digimon in hand → the SOMP
/// clause installs an accept/decline prompt and (on accept + pick) digivolves
/// Kokuwamon onto the hand card for free.
#[test]
fn bt25_062_somp_free_digivolves_into_machine_from_hand() {
    let mut runner = kokuwamon_base().hand(0, &["MACHINE-LV4"]).memory(3).start();
    let koku = place_kokuwamon(&mut runner);

    let hand_before = runner.hand_size(0);
    runner.game.enter_main_phase();

    // Optional accept/decline must surface ("may").
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

    let perm = &runner.game.players[0].battle_area[koku.index as usize];
    assert_eq!(
        perm.top_card().card_id(&runner.game.card_data),
        "MACHINE-LV4",
        "Kokuwamon must have digivolved into the [Machine] hand card"
    );
    assert!(
        perm.card_sources.len() >= 2,
        "digivolution stack must now hold Kokuwamon underneath the Lv.4 result"
    );
    // §8-1-3-3: digivolving draws 1 card, so the hand SIZE is unchanged --
    // assert the [Machine] card's departure by identity, not by arithmetic.
    assert!(
        !runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "MACHINE-LV4"),
        "the [Machine] card left hand to become the new top card"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "the digivolve draw replaced it (§8-1-3-3)"
    );
}

/// Negative: 5+ memory → the SOMP clause is gated out (no prompt).
#[test]
fn bt25_062_somp_no_fire_with_5_or_more_memory() {
    let mut runner = kokuwamon_base().hand(0, &["MACHINE-LV4"]).memory(5).start();
    let _koku = place_kokuwamon(&mut runner);

    runner.game.enter_main_phase();

    assert!(
        runner.pending_selection().is_none(),
        "memory > 4 → SOMP clause must not fire"
    );
}

/// Negative: <=4 memory but no [Machine]/[Cyborg]/[TS] Digimon in hand → the
/// select_hand picker has no legal target and the clause is a no-op.
#[test]
fn bt25_062_somp_no_fire_without_eligible_hand_card() {
    let mut runner = kokuwamon_base().hand(0, &["BEAST-LV4"]).memory(3).start();
    let _koku = place_kokuwamon(&mut runner);

    runner.game.enter_main_phase();

    assert!(
        runner.pending_selection().is_none(),
        "no [Machine]/[Cyborg]/[TS] Digimon in hand → SOMP clause is a no-op"
    );
}

/// The [TS] trait is one of the three accepted evolution categories.
#[test]
fn bt25_062_somp_accepts_ts_trait_hand_card() {
    let mut runner = kokuwamon_base().hand(0, &["TS-LV4"]).memory(3).start();
    let koku = place_kokuwamon(&mut runner);

    runner.game.enter_main_phase();
    assert!(
        runner.pending_selection().is_some(),
        "a [TS] Digimon in hand must be an eligible evolution target"
    );
    runner
        .accept_optional_trigger()
        .expect("accept the optional SOMP trigger");
    runner.auto_resolve().expect("resolve");

    let perm = &runner.game.players[0].battle_area[koku.index as usize];
    assert_eq!(
        perm.top_card().card_id(&runner.game.card_data),
        "TS-LV4",
        "Kokuwamon must digivolve into the [TS] hand card"
    );
}

/// Negative: opponent's turn → the [Start of *Your* Main Phase] clause is
/// `your_turn`-gated and must not fire when the opponent's main phase begins.
#[test]
fn bt25_062_somp_no_fire_on_opponents_turn() {
    let mut runner = kokuwamon_base().hand(0, &["MACHINE-LV4"]).memory(3).start();
    let _koku = place_kokuwamon(&mut runner);

    runner.game.turn_player_idx = 1;
    runner.game.enter_main_phase();

    assert!(
        runner.pending_selection().is_none(),
        "[Start of Your Main Phase] must not fire on the opponent's turn"
    );
}

/// Optional decline: accepting the trigger is not forced — the player can
/// decline and keep Kokuwamon as-is.
#[test]
fn bt25_062_somp_decline_keeps_kokuwamon() {
    let mut runner = kokuwamon_base().hand(0, &["MACHINE-LV4"]).memory(3).start();
    let koku = place_kokuwamon(&mut runner);

    runner.game.enter_main_phase();
    assert!(runner.pending_selection().is_some(), "prompt installed");

    runner
        .decline_optional_trigger()
        .expect("declining the optional SOMP trigger is legal");

    let perm = &runner.game.players[0].battle_area[koku.index as usize];
    assert_eq!(
        perm.top_card().card_id(&runner.game.card_data),
        CARD_ID,
        "declining leaves Kokuwamon as the top card (no digivolve)"
    );
}

// ─── Section 3 — Behavior: inherited +1000 DP aura ───────────────────────────

/// As a digivolution source under a carrier, Kokuwamon's inherited effect adds
/// +1000 DP to the carrier.
#[test]
fn bt25_062_inherited_grants_plus_1000_dp_to_carrier() {
    let mut carrier = make_evo_target("CARRIER", &["Machine"]);
    carrier.dp = Some(5000);

    let mut runner = kokuwamon_base().add_card(carrier).memory(10).start();
    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER"]);

    assert_eq!(
        runner.effective_dp(stack),
        Some(6000),
        "carrier base 5000 + inherited 1000 from Kokuwamon source = 6000"
    );
}
