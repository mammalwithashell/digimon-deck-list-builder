//! ST20-07 Tentomon — Digimon, Lv.3, Green, DP 3000, Cost 3.
//! Traits: Insectoid / ADVENTURE. Form: Rookie. Attribute: Vaccine.
//!
//! # Card text (official Bandai DB, data/card_bundles/ST20-07.md)
//!
//! ```text
//! [Opponent's Turn] Your opponent can't reduce digivolution costs.
//!
//! [Digivolve] Lv.2 w/[ADVENTURE] trait: Cost 0      (xros_req box)
//! [evo_costs] Lv.2 Green: Cost 0
//!
//! Official Q&A: the effect prevents digivolution costs from being reduced
//! when the costs are to be paid. This effect affects the opponent player.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST20/Green/ST20_07.cs
//! - EffectTiming.None AddSelfDigivolutionRequirementStaticEffect: Lv.2 +
//!   HasAdventureTraits, cost 0, requirements respected.
//! - EffectTiming.None CannotReduceCostClass: owner's enemy, IsOpponentTurn,
//!   on the battle area.
//!
//! # Patterns this test file covers
//!
//! | Clause                                        | Pattern                                            |
//! |-----------------------------------------------|----------------------------------------------------|
//! | Standard digivolve Lv.2 Green / cost 0        | alt_paths (evo-cost row) + route legality          |
//! | xros_req Lv.2 ADVENTURE / cost 0              | alt_paths (trait gate) + route legality, negatives |
//! | [Opponent's Turn] no digivolution cost reduce | D6 flood gate — structural + integrated + turn gate |

use std::sync::Arc;

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledPlayerRef,
};
use digimon_engine::action::{build_action_mask, encode_digivolve};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, ModifierType, PlaySource};

const ID: &str = "ST20-07";

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Field-resident -3 digivolution cost reducer for the *opponent* of Tentomon.
struct FieldCostReducer;

impl CardEffect for FieldCostReducer {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card).cost_reduction(3).build()]
    }
}

fn make_level_digimon(id: &str, level: u8) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(3000);
    card.play_cost = 3;
    card.colors = vec![CardColor::Yellow];
    card
}

fn make_evo_digimon(id: &str, level: u8, from_level: u8, cost: u16) -> CardData {
    let mut card = make_level_digimon(id, level);
    card.evo_costs = vec![EvoCost {
        level: from_level,
        memory_cost: cost,
        card_color: CardColor::Yellow as u8,
    }];
    card
}

/// Runner for the cost-reduction interaction: P0 owns Tentomon, P1 owns a
/// Lv.3 base, a -3 field reducer and a Lv.4 "EVO" in hand (cost 4 from Lv.3).
fn tentomon_reduction_runner() -> DebugRunner {
    let mut registry = digimon_engine::cards::build_registry();
    registry.insert("REDUCER", Arc::new(FieldCostReducer));

    DebugRunner::builder()
        .with_registry(registry)
        .dsl_card(ID)
        .expect("ST20-07 found in embedded DSL pack")
        .add_card(make_level_digimon("BASE", 3))
        .add_card(make_evo_digimon("EVO", 4, 3, 4))
        .add_card(make_level_digimon("REDUCER", 4))
        .add_card(make_test_card("FILLER", "FILLER"))
        .hand(1, &["EVO"])
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .memory(10)
        .start()
}

/// Runner for digivolve-route legality: Tentomon in P0's hand plus Lv.2/Lv.3
/// candidate bases of various colours and traits.
fn route_runner() -> DebugRunner {
    let mut b = DebugRunner::builder()
        .dsl_card(ID)
        .expect("ST20-07 found in embedded DSL pack");
    for (id, level, color, adventure) in [
        ("GREEN-LV2", 2, CardColor::Green, false),
        ("ADV-RED-LV2", 2, CardColor::Red, true),
        ("RED-LV2", 2, CardColor::Red, false),
        ("ADV-GREEN-LV3", 3, CardColor::Green, true),
    ] {
        let mut c = make_test_card(id, id);
        c.card_kind = CardKind::Digimon;
        c.level = Some(level);
        c.colors = vec![color];
        c.dp = Some(2000);
        if adventure {
            c.traits = vec!["ADVENTURE".to_string()];
        }
        b = b.add_card(c);
    }
    let mut r = b
        .add_card(make_test_card("FILLER", "FILLER"))
        .hand(0, &[ID])
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.current_phase = GamePhase::Main;
    r
}

fn top_card_id(r: &DebugRunner, base: digimon_engine::permanent::PermanentHandle) -> String {
    r.game.players[0].battle_area[base.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st20_07_identity_matches_printed_card() {
    let runner = DebugRunner::builder()
        .dsl_card(ID)
        .expect("ST20-07 found in embedded DSL pack")
        .start();
    let compiled = runner.compiled_card(ID).expect("ST20-07 compiled");

    assert_eq!(compiled.name, "Tentomon");
    assert_eq!(compiled.level, Some(3));
    assert_eq!(compiled.dp, Some(3000));
    assert_eq!(compiled.cost, Some(3));
    assert_eq!(compiled.color, vec![CompiledColor::Green]);
    for t in ["Insectoid", "ADVENTURE"] {
        assert!(
            compiled.traits.iter().any(|x| x == t),
            "missing trait {t}, got {:?}",
            compiled.traits
        );
    }
    assert_eq!(compiled.attribute, ["Vaccine"]);
}

#[test]
fn st20_07_declares_standard_and_xros_alt_paths() {
    let runner = DebugRunner::builder()
        .dsl_card(ID)
        .expect("ST20-07 found in embedded DSL pack")
        .start();
    let compiled = runner.compiled_card(ID).expect("ST20-07 compiled");
    assert_eq!(compiled.alt_paths.len(), 2, "standard + xros_req");

    // Standard circle: Lv.2 Green, cost 0.
    let standard = compiled
        .alt_paths
        .iter()
        .find(|p| {
            p.from
                .as_ref()
                .is_some_and(|f| f.color_is == Some(CompiledColor::Green))
        })
        .expect("standard Lv.2 Green path");
    assert_eq!(standard.kind, CompiledAltPathKind::Digivolve);
    assert_eq!(standard.cost, Some(CompiledCost::Literal(0)));
    assert_eq!(standard.from.as_ref().unwrap().level_eq, Some(2));
    assert!(!standard.ignore_requirements);

    // xros_req box: Lv.2 w/[ADVENTURE] trait, cost 0, requirements respected.
    let xros = compiled
        .alt_paths
        .iter()
        .find(|p| {
            p.from
                .as_ref()
                .is_some_and(|f| f.trait_has.as_deref() == Some("ADVENTURE"))
        })
        .expect("xros_req Lv.2 ADVENTURE path");
    assert_eq!(xros.kind, CompiledAltPathKind::Digivolve);
    assert_eq!(xros.cost, Some(CompiledCost::Literal(0)));
    assert_eq!(xros.from.as_ref().unwrap().level_eq, Some(2));
    assert!(
        !xros.ignore_requirements,
        "DCGO ignoreDigivolutionRequirement: false"
    );
}

#[test]
fn st20_07_declares_opponent_targeted_cannot_reduce_digivolve_cost_flood_gate() {
    let runner = DebugRunner::builder()
        .dsl_card(ID)
        .expect("ST20-07 found in embedded DSL pack")
        .start();
    let compiled = runner.compiled_card(ID).expect("ST20-07 compiled");

    let flood_gate = compiled.effects.iter().find_map(|clause| match clause {
        CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate {
            modifier,
            target_player,
            ..
        }) => Some((modifier.as_str(), *target_player)),
        _ => None,
    });
    assert_eq!(
        flood_gate,
        Some((
            "CannotReduceDigivolveCost",
            Some(CompiledPlayerRef::Opponent),
        )),
        "Tentomon must install an opponent-targeted CannotReduceDigivolveCost flood gate"
    );
    assert_eq!(
        compiled.effects.len(),
        1,
        "the only effect clause is the [Opponent's Turn] flood gate"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Digivolve routes (standard + xros_req), positive and negative
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: the standard Lv.2 Green circle digivolves into Tentomon for 0.
#[test]
fn st20_07_digivolves_from_green_lv2_for_0() {
    let mut r = route_runner();
    let base = r.place_on_field(0, "GREEN-LV2", Some(0));
    let action = encode_digivolve(0, base.index as u16);
    assert!(
        build_action_mask(&r.game, 0)[action as usize] > 0.5,
        "Lv.2 Green -> Tentomon must be mask-legal"
    );
    let before = r.memory();
    r.game.decode_action(action, 0);
    assert_eq!(top_card_id(&r, base), ID, "Tentomon is now the top card");
    assert_eq!(before - r.memory(), 0, "digivolution cost is 0");
}

/// Positive: the xros_req box accepts a Lv.2 [ADVENTURE] Digimon of ANY color.
#[test]
fn st20_07_digivolves_from_adventure_lv2_of_another_color_for_0() {
    let mut r = route_runner();
    let base = r.place_on_field(0, "ADV-RED-LV2", Some(0));
    let action = encode_digivolve(0, base.index as u16);
    assert!(
        build_action_mask(&r.game, 0)[action as usize] > 0.5,
        "Lv.2 ADVENTURE (red) -> Tentomon must be mask-legal via the xros_req box"
    );
    let before = r.memory();
    r.game.decode_action(action, 0);
    assert_eq!(top_card_id(&r, base), ID);
    assert_eq!(before - r.memory(), 0, "digivolution cost is 0");
}

/// Negative: a Lv.2 Digimon that is neither Green nor [ADVENTURE] is illegal.
#[test]
fn st20_07_cannot_digivolve_from_non_green_non_adventure_lv2() {
    let mut r = route_runner();
    let base = r.place_on_field(0, "RED-LV2", Some(0));
    let action = encode_digivolve(0, base.index as u16);
    assert!(
        build_action_mask(&r.game, 0)[action as usize] < 0.5,
        "Lv.2 red without ADVENTURE must not be a legal base"
    );
}

/// Negative: the xros_req box is level-gated — a Lv.3 [ADVENTURE] base is illegal.
#[test]
fn st20_07_cannot_digivolve_from_adventure_lv3() {
    let mut r = route_runner();
    let base = r.place_on_field(0, "ADV-GREEN-LV3", Some(0));
    let action = encode_digivolve(0, base.index as u16);
    assert!(
        build_action_mask(&r.game, 0)[action as usize] < 0.5,
        "Lv.3 ADVENTURE must not be a legal base (needs Lv.2)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — [Opponent's Turn] Your opponent can't reduce digivolution costs
// ═══════════════════════════════════════════════════════════════════════════

/// Turn gate: while Tentomon is on P0's field, P1 carries the lock only on
/// P1's turn; P0 never carries it.
#[test]
fn st20_07_flood_gate_active_only_on_opponents_turn() {
    let mut runner = tentomon_reduction_runner();
    runner.place_on_field(0, ID, None);

    // P0's own turn: the [Opponent's Turn] gate must be inactive.
    runner.game.tick_declarative_effects();
    assert!(
        !runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotReduceDigivolveCost),
        "flood gate must be inactive on Tentomon's controller's own turn"
    );
    assert!(
        !runner
            .game
            .modifiers
            .player_has(0, ModifierType::CannotReduceDigivolveCost),
        "the controller must never receive their own flood gate"
    );

    // P1's turn: the gate becomes active on the opponent only.
    runner.end_turn();
    runner.game.tick_declarative_effects();
    assert!(
        runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotReduceDigivolveCost),
        "opponent must carry CannotReduceDigivolveCost on their turn"
    );
    assert!(
        !runner
            .game
            .modifiers
            .player_has(0, ModifierType::CannotReduceDigivolveCost),
        "the controller must never receive their own flood gate"
    );
}

/// Integrated positive: with Tentomon on P0's field, P1's -3 digivolution
/// reducer is suppressed on P1's turn — the digivolve stays mask-legal but
/// pays the full printed cost 4.
#[test]
fn st20_07_blocks_opponents_digivolution_cost_reduction_but_digivolve_stays_legal() {
    let mut runner = tentomon_reduction_runner();

    runner.place_on_field(0, ID, Some(0));
    runner.place_on_field(1, "BASE", Some(0));
    runner.place_on_field(1, "REDUCER", Some(0));
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.game.set_memory(10);
    runner.game.tick_declarative_effects();

    let action = encode_digivolve(0, 0);
    let mask = build_action_mask(&runner.game, 1);
    assert_eq!(
        mask[action as usize], 1.0,
        "Tentomon must not hide the opponent's ordinary digivolve action from the mask"
    );

    let memory_before = runner.memory();
    let digivolved = runner
        .game
        .digivolve_from_hand(1, 0, 0, PlaySource::ByDigivolve);
    assert!(
        digivolved,
        "opponent's ordinary digivolve must remain legal"
    );
    assert_eq!(
        memory_before - runner.memory(),
        4,
        "Tentomon must suppress the opponent's -3 digivolution cost reducer"
    );
}

/// Integrated negative (baseline): without Tentomon the same reducer applies.
#[test]
fn st20_07_baseline_without_tentomon_allows_digivolution_cost_reduction() {
    let mut runner = tentomon_reduction_runner();

    runner.place_on_field(1, "BASE", Some(0));
    runner.place_on_field(1, "REDUCER", Some(0));
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.game.set_memory(10);
    runner.game.tick_declarative_effects();

    let memory_before = runner.memory();
    let digivolved = runner
        .game
        .digivolve_from_hand(1, 0, 0, PlaySource::ByDigivolve);
    assert!(digivolved, "ordinary digivolve should be legal");
    assert_eq!(
        memory_before - runner.memory(),
        1,
        "digivolution cost reducer should apply normally without Tentomon"
    );
}

/// Scope negative: the lock affects only the OPPONENT. Tentomon's own
/// controller keeps their digivolution cost reducers on their own turn.
#[test]
fn st20_07_does_not_block_its_own_controllers_cost_reduction() {
    let mut registry = digimon_engine::cards::build_registry();
    registry.insert("REDUCER", Arc::new(FieldCostReducer));
    let mut runner = DebugRunner::builder()
        .with_registry(registry)
        .dsl_card(ID)
        .expect("ST20-07 found in embedded DSL pack")
        .add_card(make_level_digimon("BASE", 3))
        .add_card(make_evo_digimon("EVO", 4, 3, 4))
        .add_card(make_level_digimon("REDUCER", 4))
        .add_card(make_test_card("FILLER", "FILLER"))
        .hand(0, &["EVO"])
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .memory(10)
        .start();

    runner.place_on_field(0, ID, Some(0));
    runner.place_on_field(0, "BASE", Some(0));
    runner.place_on_field(0, "REDUCER", Some(0));
    runner.game.enter_main_phase();
    runner.game.set_memory(10);
    runner.game.tick_declarative_effects();

    // BASE sits at battle-area index 1 (index 0 is Tentomon).
    let memory_before = runner.memory();
    let digivolved = runner
        .game
        .digivolve_from_hand(0, 0, 1, PlaySource::ByDigivolve);
    assert!(digivolved, "controller's own digivolve must be legal");
    assert_eq!(
        memory_before - runner.memory(),
        1,
        "Tentomon's lock never applies to its own controller's reducers"
    );
}
