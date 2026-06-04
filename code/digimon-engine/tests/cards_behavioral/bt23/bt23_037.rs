//! BT23-037 Tentomon — Digimon, Lv.3, Green/Yellow, DP 1000, Cost 3.
//! Traits: Insectoid, Hudie, CS.
//! Evo costs: Lv.2 Green / cost 1.
//!
//! # Card text (cards.json / BT23-037.json)
//!
//! **Own effect:**
//! [Your Turn] When this Digimon would digivolve into a Digimon card with the
//! [CS] trait, reduce the digivolution cost by 1.
//!
//! **Inherited effect:**
//! [When Attacking] [Once Per Turn] You may play 1 play cost 5 or lower Digimon
//! card with the [Hudie] trait from your hand without paying the cost. The
//! Digimon this effect played can't digivolve and is deleted at the end of your
//! opponent's turn.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT23/Green/BT23_037.cs
//!   - EffectTiming.None (Your Turn): ChangeDigivolutionCostStaticEffect(-1)
//!     gated on PermanentCondition (the digivolving permanent IS Tentomon's
//!     permanent) + CardSourceCondition (target HasCSTraits). IsOwnerTurn.
//!     NOT [Once Per Turn] (SetUpActivateClass max-count -1 = unlimited).
//!   - EffectTiming.OnAllyAttack (inherited When Attacking, maxCount 1 = OPT):
//!     optional SelectHandEffect (canNoSelect: true) over IsDigimon &&
//!     BasePlayCost <= 5 && EqualsTraits("Hudie"); PlayPermanentCards(
//!     payCost: false, activateETB: true) — the played Digimon's On Play DOES
//!     fire; then on the played permanent: CanNotDigivolveClass +
//!     AddSelfDeleteEffect(DeleteTiming.AtOpponentTurnEnd).
//!
//! # Alternate digivolution paths (BT23-037.json)
//!   - Path 1: printed evo cost Lv.2 Green / cost 1.
//!   - Path 2: printed `xros_req` "[Digivolve] Lv.2 w/[CS] trait: Cost 0" —
//!     DCGO `AddSelfDigivolutionRequirementStaticEffect(IsLevel2 && HasCSTraits,
//!     digivolutionCost: 0)`. Idiom mirrors BT10-029.
//!
//! # Patterns this test file covers
//! - D2 — [Your Turn] BeforePayCost cost reduction gated on digivolve-target
//!   [CS] trait (Clause 1; G-BEFORE-PAY-COST-DIGIVOLVE-TARGET, exemplar
//!   BT23-005 Elizamon).
//! - G4 — inherited [When Attacking][OPT] (Clause 2), driven via place_stack +
//!   attack_player (BT15-003 Nyaromon pattern).
//! - play_from_hand_free + bind_as → CannotDigivolve modifier (expiry
//!   end_of_opponents_turn) + schedule_delete_played_at_turn_end (at:
//!   opponents_turn). On Play fires (suppress_on_play = default false).
//!   Exemplars: BT13-020, EX11-061, P-165.
//! - E2 — OPT lockout on the inherited clause (second attack same turn → no
//!   replay; clears next turn).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause,
    CompiledPredicate, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Expiry, ModifierType, PlaySource};
use digimon_engine::permanent::PermanentHandle;

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// A Lv.4 [CS]-trait Green Digimon that can digivolve from a Lv.3 (evo cost 1).
fn make_cs_lv4() -> CardData {
    let mut c = make_test_card("CS-LV4", "CsBeetle");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.traits = vec!["CS".to_string()];
    c.colors = vec![CardColor::Green];
    c.evo_costs = vec![EvoCost {
        level: 3,
        card_color: 3, // Green
        memory_cost: 1,
    }];
    c
}

/// A Lv.4 Digimon WITHOUT the [CS] trait (evo cost 1).
fn make_non_cs_lv4() -> CardData {
    let mut c = make_test_card("PLAIN-LV4", "PlainBeast");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.traits = vec!["Insectoid".to_string()];
    c.colors = vec![CardColor::Green];
    c.evo_costs = vec![EvoCost {
        level: 3,
        card_color: 3, // Green
        memory_cost: 1,
    }];
    c
}

/// A Lv.3 vanilla Digimon used as a digivolution base / attacker top card.
fn make_vanilla_lv3(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.colors = vec![CardColor::Green];
    c
}

/// A Lv.3 [Hudie] Digimon with play cost `cost`.
/// `cost` matters for the "play cost 5 or lower" filter on Clause 2.
fn make_hudie_digimon(id: &str, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = cost;
    c.traits = vec!["Hudie".to_string()];
    c.colors = vec![CardColor::Green];
    c
}

/// A Lv.2 Green Digimon with the [CS] trait — eligible for the cost-0
/// xros_req alt-path AND the cost-1 color path.
fn make_cs_lv2() -> CardData {
    let mut c = make_test_card("CS-LV2", "CsRookieBase");
    c.card_kind = CardKind::Digimon;
    c.level = Some(2);
    c.dp = Some(2000);
    c.play_cost = 2;
    c.traits = vec!["CS".to_string()];
    c.colors = vec![CardColor::Green];
    c
}

/// A Lv.2 Green Digimon WITHOUT the [CS] trait — only eligible for the
/// printed cost-1 color path.
fn make_non_cs_lv2() -> CardData {
    let mut c = make_test_card("PLAIN-LV2", "PlainRookieBase");
    c.card_kind = CardKind::Digimon;
    c.level = Some(2);
    c.dp = Some(2000);
    c.play_cost = 2;
    c.traits = vec!["Insectoid".to_string()];
    c.colors = vec![CardColor::Green];
    c
}

/// A non-Hudie Lv.3 Digimon (play cost 3) — must NOT be offered by Clause 2.
fn make_non_hudie_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.traits = vec!["Insectoid".to_string()];
    c.colors = vec![CardColor::Green];
    c
}

/// Push a card (by id) into player `p`'s hand and return its hand index.
fn push_to_hand(runner: &mut DebugRunner, p: usize, card_id: &str) -> usize {
    use digimon_engine::card_source::CardSource;
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} in card_data"));
    let card_index = runner.game.next_card_index();
    runner.game.players[p]
        .hand
        .push(CardSource::new(data_idx, p as u8, card_index));
    runner.game.players[p].hand.len() - 1
}

fn field_card_ids(runner: &DebugRunner, p: usize) -> Vec<String> {
    runner.game.players[p]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn bt23_037_metadata_matches_printed() {
    use digimon_dsl::compiled::{CompiledCardKind, CompiledColor};
    let runner = DebugRunner::builder()
        .dsl_card("BT23-037")
        .expect("BT23-037 in embedded DSL pack")
        .memory(0)
        .start();
    let compiled = runner.compiled_card("BT23-037").expect("compiled");

    assert_eq!(compiled.name, "Tentomon");
    assert_eq!(compiled.kind, CompiledCardKind::Digimon);
    assert_eq!(compiled.level, Some(3));
    assert_eq!(compiled.dp, Some(1000));
    assert_eq!(compiled.cost, Some(3));
    assert!(compiled.color.contains(&CompiledColor::Green), "Green");
    assert!(compiled.color.contains(&CompiledColor::Yellow), "Yellow");
    for t in ["Insectoid", "Hudie", "CS"] {
        assert!(
            compiled.traits.iter().any(|x| x == t),
            "must carry trait {t}; got {:?}",
            compiled.traits
        );
    }
}

#[test]
fn bt23_037_has_cost_reduction_and_inherited_when_attacking_clauses() {
    let runner = DebugRunner::builder()
        .dsl_card("BT23-037")
        .expect("BT23-037 in embedded DSL pack")
        .memory(0)
        .start();
    let compiled = runner.compiled_card("BT23-037").expect("compiled");

    let has_cost_reduction = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
        )
    });
    assert!(has_cost_reduction, "Clause 1 must be a CostReduction clause");

    let inherited_wa = compiled.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => {
            t.scope == CompiledScope::Inherited && t.when.contains(&CompiledTiming::WhenAttacking)
        }
        _ => false,
    });
    assert!(
        inherited_wa,
        "Clause 2 must be an inherited [When Attacking] triggered clause"
    );
}

#[test]
fn bt23_037_cost_reduction_is_not_once_per_turn() {
    let runner = DebugRunner::builder()
        .dsl_card("BT23-037")
        .expect("BT23-037 in embedded DSL pack")
        .memory(0)
        .start();
    let compiled = runner.compiled_card("BT23-037").expect("compiled");

    let opt = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
            once_per_turn,
            ..
        }) => Some(*once_per_turn),
        _ => None,
    });
    assert_eq!(
        opt,
        Some(false),
        "DCGO SetUpActivateClass max-count -1 (unlimited); printed text has no OPT"
    );
}

#[test]
fn bt23_037_cost_reduction_clause_carries_your_turn_gate() {
    let runner = DebugRunner::builder()
        .dsl_card("BT23-037")
        .expect("parses")
        .memory(0)
        .start();
    let compiled = runner.compiled_card("BT23-037").expect("compiled");

    let active_when = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
                active_when,
                ..
            }) => Some(active_when.clone()),
            _ => None,
        })
        .expect("CostReduction clause")
        .expect("CostReduction clause must carry an active_when predicate");

    fn has_your_turn(p: &CompiledPredicate) -> bool {
        p.your_turn == Some(true)
            || p.all_of.iter().any(has_your_turn)
            || p.any_of.iter().any(has_your_turn)
    }
    assert!(
        has_your_turn(&active_when),
        "the cost-reduction clause must carry a [Your Turn] gate"
    );
}

#[test]
fn bt23_037_inherited_when_attacking_is_optional_and_opt() {
    let runner = DebugRunner::builder()
        .dsl_card("BT23-037")
        .expect("parses")
        .memory(0)
        .start();
    let compiled = runner.compiled_card("BT23-037").expect("compiled");

    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("inherited When Attacking clause");

    assert!(clause.optional, "'You may play' must be optional");
    assert!(clause.once_per_turn, "[Once Per Turn]");
}

// ─── Alt-paths: standard color path + xros_req [CS] cost-0 path ───────────────

#[test]
fn bt23_037_declares_cs_level_two_cost_zero_alt_path() {
    let runner = DebugRunner::builder()
        .dsl_card("BT23-037")
        .expect("parses")
        .memory(0)
        .start();
    let card = runner.compiled_card("BT23-037").expect("compiled");

    // xros_req "[Digivolve] Lv.2 w/[CS] trait: Cost 0".
    assert!(
        card.alt_paths.iter().any(|path| {
            let from = format!("{:?}", path.from);
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(0))
                && from.contains("level_eq: Some(2)")
                && from.contains("\"CS\"")
        }),
        "must declare a Lv.2 [CS]-trait cost-0 digivolve alt-path; alt_paths={:?}",
        card.alt_paths
    );
    // Standard printed evo cost: Lv.2 Green / cost 1.
    assert!(
        card.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(1))
        }),
        "must declare the printed Lv.2 cost-1 color path; alt_paths={:?}",
        card.alt_paths
    );
}

/// POSITIVE: digivolving Tentomon FROM a Lv.2 [CS] base uses the cost-0
/// xros_req path (memory unchanged).
#[test]
fn bt23_037_digivolves_from_cs_lv2_for_cost_zero() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT23-037")
        .expect("parses")
        .add_card(make_cs_lv2())
        .memory(5)
        .start();
    runner.game.turn_count = 1;

    let base = runner.place_on_field(0, "CS-LV2", Some(0));
    let hand_idx = push_to_hand(&mut runner, 0, "BT23-037");

    let memory_before = runner.game.memory;
    let ok = runner
        .game
        .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByHand);
    assert!(ok, "Tentomon must digivolve from a Lv.2 [CS] base (cost 0)");
    assert_eq!(
        runner.game.memory, memory_before,
        "the Lv.2 [CS] xros_req path costs 0 — memory unchanged"
    );
}

/// NEGATIVE: digivolving Tentomon FROM a Lv.2 NON-[CS] Green base uses the
/// printed cost-1 color path (memory drops by 1) — the cost-0 path requires
/// the [CS] trait.
#[test]
fn bt23_037_digivolves_from_non_cs_lv2_for_cost_one() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT23-037")
        .expect("parses")
        .add_card(make_non_cs_lv2())
        .memory(5)
        .start();
    runner.game.turn_count = 1;

    let base = runner.place_on_field(0, "PLAIN-LV2", Some(0));
    let hand_idx = push_to_hand(&mut runner, 0, "BT23-037");

    let memory_before = runner.game.memory;
    let ok = runner
        .game
        .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByHand);
    assert!(ok, "Tentomon must digivolve from a Lv.2 Green base (cost 1)");
    assert_eq!(
        runner.game.memory,
        memory_before - 1,
        "a non-[CS] Lv.2 base uses the printed cost-1 color path"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — Clause 1 [Your Turn] cost reduction on digivolve into [CS]
// ─────────────────────────────────────────────────────────────────────────────

/// POSITIVE: digivolving FROM Tentomon INTO a [CS] Lv.4 reduces the
/// digivolution cost by 1 (printed evo cost 1 → effective 0; memory unchanged).
#[test]
fn bt23_037_cost_reduction_fires_digivolving_into_cs() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT23-037")
        .expect("parses")
        .add_card(make_cs_lv4())
        .memory(5)
        .start();
    runner.game.turn_count = 1;

    let tentomon = runner.place_on_field(0, "BT23-037", Some(0));
    let hand_idx = push_to_hand(&mut runner, 0, "CS-LV4");

    let memory_before = runner.game.memory;
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_idx, tentomon.index as usize, PlaySource::ByHand);
    assert!(
        digivolved,
        "Tentomon must digivolve into CS-LV4 (effective cost 1 - 1 = 0)"
    );
    assert_eq!(
        runner.game.memory, memory_before,
        "memory unchanged: 1 evo cost - 1 Tentomon reduction = 0"
    );
}

/// NEGATIVE (trait gate): digivolving FROM Tentomon INTO a non-[CS] Lv.4 must
/// NOT trigger the reduction; the full evo cost 1 is paid.
#[test]
fn bt23_037_cost_reduction_does_not_fire_for_non_cs_target() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT23-037")
        .expect("parses")
        .add_card(make_non_cs_lv4())
        .memory(5)
        .start();
    runner.game.turn_count = 1;

    let tentomon = runner.place_on_field(0, "BT23-037", Some(0));
    let hand_idx = push_to_hand(&mut runner, 0, "PLAIN-LV4");

    let memory_before = runner.game.memory;
    let _ =
        runner
            .game
            .digivolve_from_hand(0, hand_idx, tentomon.index as usize, PlaySource::ByHand);
    assert_eq!(
        runner.game.memory,
        memory_before - 1,
        "non-[CS] target — reduction must not apply, full evo cost 1 is paid"
    );
}

/// NEGATIVE ("THIS Digimon" gate): with Tentomon present on the field as a
/// BYSTANDER, digivolving a DIFFERENT Lv.3 permanent into a [CS] target gets NO
/// cost reduction. `source_is_cost_target_permanent` requires Tentomon itself
/// to be the permanent being digivolved.
#[test]
fn bt23_037_cost_reduction_does_not_fire_for_different_source() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT23-037")
        .expect("parses")
        .add_card(make_vanilla_lv3("PLAIN-LV3"))
        .add_card(make_cs_lv4())
        .memory(5)
        .start();
    runner.game.turn_count = 1;

    let _tentomon = runner.place_on_field(0, "BT23-037", Some(0));
    let plain = runner.place_on_field(0, "PLAIN-LV3", Some(0));
    let hand_idx = push_to_hand(&mut runner, 0, "CS-LV4");

    let memory_before = runner.game.memory;
    let _ = runner
        .game
        .digivolve_from_hand(0, hand_idx, plain.index as usize, PlaySource::ByHand);
    assert_eq!(
        runner.game.memory,
        memory_before - 1,
        "a non-Tentomon digivolution source must not receive the reduction"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — Clause 2 inherited [When Attacking][OPT]: free-play a Hudie ≤5
// ─────────────────────────────────────────────────────────────────────────────

/// Build a runner where BT23-037 is a digivolution SOURCE (inherited effects
/// active) under a Lv.4 attacker, with a Hudie ≤5 in hand. P1 gets enough
/// security so attacking the player does not auto-win, and both players get
/// filler decks for turn rotation (OPT-clears test).
fn attacking_runner() -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT23-037")
        .expect("BT23-037 in pack")
        .add_card(make_cs_lv4()) // CS-LV4: Lv.4 attacker top card
        .add_card(make_hudie_digimon("HUDIE-3", 3)) // play cost 3 ≤ 5 — eligible
        .add_card(make_hudie_digimon("HUDIE-5", 5)) // play cost 5 ≤ 5 — eligible
        .add_card(make_hudie_digimon("HUDIE-6", 6)) // play cost 6 > 5 — NOT eligible
        .add_card(make_non_hudie_digimon("NON-HUDIE-3")) // not Hudie — NOT eligible
        .add_card(make_vanilla_lv3("FILLER"))
        .security(1, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
        .deck(0, &["FILLER"; 30])
        .deck(1, &["FILLER"; 30])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner
}

/// Stack BT23-037 (base) under CS-LV4 (top attacker). Returns the attacker
/// handle.
fn stack_tentomon_attacker(runner: &mut DebugRunner) -> PermanentHandle {
    runner.place_stack(0, &["BT23-037", "CS-LV4"])
}

/// POSITIVE: the inherited When Attacking offers an optional Hudie ≤5 play, and
/// only the eligible Hudie ≤5 cards are candidates.
#[test]
fn bt23_037_when_attacking_offers_optional_hudie_play() {
    let mut runner = attacking_runner();
    push_to_hand(&mut runner, 0, "HUDIE-3");
    push_to_hand(&mut runner, 0, "HUDIE-5");
    push_to_hand(&mut runner, 0, "HUDIE-6");
    push_to_hand(&mut runner, 0, "NON-HUDIE-3");
    let attacker = stack_tentomon_attacker(&mut runner);

    let _ = runner.attack_player(attacker, 1, false);

    let view = runner
        .pending_selection_view()
        .expect("inherited When Attacking must install a hand-play prompt");
    assert!(
        view.is_optional,
        "'You may play' — PASS must be a legal decline"
    );

    // Exactly two hand candidates (HUDIE-3 and HUDIE-5); HUDIE-6 (cost 6) and
    // NON-HUDIE-3 (not Hudie) must be filtered out.
    let hand_candidates: Vec<u16> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|id| *id != PASS && *id >= PLAY_HAND_START && *id < PLAY_HAND_START + 50)
        .collect();
    assert_eq!(
        hand_candidates.len(),
        2,
        "only the two Hudie ≤5 cards may be offered; got {:?}",
        view.valid_action_ids
    );
}

/// NEGATIVE (no eligible card): only ineligible cards in hand → no play prompt
/// installs (DCGO does not prompt for an optional ability with no legal target).
#[test]
fn bt23_037_when_attacking_no_prompt_when_no_eligible_hudie() {
    let mut runner = attacking_runner();
    push_to_hand(&mut runner, 0, "HUDIE-6"); // cost 6 > 5
    push_to_hand(&mut runner, 0, "NON-HUDIE-3"); // not Hudie
    let attacker = stack_tentomon_attacker(&mut runner);

    let _ = runner.attack_player(attacker, 1, false);

    assert!(
        runner.pending_selection().is_none(),
        "no eligible Hudie ≤5 in hand → no play prompt installs; got {:?}",
        runner.pending_selection_view().map(|v| v.kind)
    );
}

/// POSITIVE: declining the play does nothing (battle area unchanged save the
/// attacker; hand unchanged).
#[test]
fn bt23_037_when_attacking_decline_plays_nothing() {
    let mut runner = attacking_runner();
    push_to_hand(&mut runner, 0, "HUDIE-3");
    let attacker = stack_tentomon_attacker(&mut runner);

    let battle_before = runner.battle_area_size(0);
    let hand_before = runner.game.players[0].hand.len();

    let _ = runner.attack_player(attacker, 1, false);
    let player = runner
        .pending_selection_view()
        .expect("play prompt installed")
        .selecting_player;
    runner.execute_action(player, PASS).expect("decline resolves");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(0),
        battle_before,
        "declining must not add a permanent"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "the Hudie stays in hand on decline"
    );
}

/// POSITIVE: accepting plays the Hudie for free (memory unchanged) and it lands
/// on the field.
#[test]
fn bt23_037_when_attacking_plays_hudie_for_free() {
    let mut runner = attacking_runner();
    push_to_hand(&mut runner, 0, "HUDIE-3");
    let attacker = stack_tentomon_attacker(&mut runner);

    let battle_before = runner.battle_area_size(0);
    let memory_before = runner.game.memory;

    let _ = runner.attack_player(attacker, 1, false);
    let (player, action_id) = {
        let view = runner
            .pending_selection_view()
            .expect("play prompt installed");
        let first = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|id| *id != PASS)
            .expect("a Hudie candidate is offered");
        (view.selecting_player, first)
    };
    runner.execute_action(player, action_id).expect("play resolves");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(0),
        battle_before + 1,
        "the free-played Hudie must land on the field"
    );
    assert!(
        field_card_ids(&runner, 0).contains(&"HUDIE-3".to_string()),
        "HUDIE-3 must be on the field"
    );
    assert_eq!(
        runner.game.memory, memory_before,
        "the Hudie is played WITHOUT paying its cost — memory unchanged"
    );
}

/// POSITIVE: the played Digimon's On Play DOES fire (DCGO activateETB: true).
/// We use a Hudie whose On Play gains 1 memory and assert the gain.
#[test]
fn bt23_037_played_hudie_on_play_fires() {
    // A Hudie ≤5 with an On Play that gains 1 memory, authored inline.
    let on_play_hudie = r#"
card: DSL-HUDIE-ONPLAY
name: Hudie OnPlay
kind: digimon
level: 3
color: [green]
cost: 3
dp: 3000
traits: [Hudie]
effects:
  - when: on_play
    summary: "[On Play] Gain 1 memory"
    process:
      - gain_memory: 1
"#;
    // Start memory at 3 (well below the +10 cap) so the On Play's +1 gain is
    // observable and not clamped at the memory ceiling.
    let mut runner = DebugRunner::builder()
        .dsl_card("BT23-037")
        .expect("BT23-037 in pack")
        .add_card(make_cs_lv4())
        .from_dsl_yaml(on_play_hudie)
        .expect("inline Hudie compiles")
        .add_card(make_vanilla_lv3("FILLER"))
        .security(1, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
        .memory(3)
        .start();
    runner.game.turn_count = 1;
    push_to_hand(&mut runner, 0, "DSL-HUDIE-ONPLAY");
    let attacker = runner.place_stack(0, &["BT23-037", "CS-LV4"]);

    let memory_before = runner.game.memory;
    let _ = runner.attack_player(attacker, 1, false);
    let (player, action_id) = {
        let view = runner
            .pending_selection_view()
            .expect("play prompt installed");
        let first = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|id| *id != PASS)
            .expect("a Hudie candidate is offered");
        (view.selecting_player, first)
    };
    runner.execute_action(player, action_id).expect("play resolves");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.game.memory,
        memory_before + 1,
        "the played Hudie's On Play (gain 1 memory) must fire (activateETB: true)"
    );
}

/// POSITIVE: the played Digimon is marked CannotDigivolve.
#[test]
fn bt23_037_played_hudie_cannot_digivolve() {
    let mut runner = attacking_runner();
    push_to_hand(&mut runner, 0, "HUDIE-3");
    let attacker = stack_tentomon_attacker(&mut runner);

    let _ = runner.attack_player(attacker, 1, false);
    let (player, action_id) = {
        let view = runner
            .pending_selection_view()
            .expect("play prompt installed");
        let first = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|id| *id != PASS)
            .expect("a Hudie candidate");
        (view.selecting_player, first)
    };
    runner.execute_action(player, action_id).expect("play resolves");
    let _ = runner.auto_resolve();

    let played = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "HUDIE-3")
        .map(|i| PermanentHandle {
            player: 0,
            index: i as u8,
        })
        .expect("HUDIE-3 on field");

    assert!(
        runner.game.modifiers.has(played, ModifierType::CannotDigivolve),
        "the played Hudie must be marked CannotDigivolve"
    );
}

/// POSITIVE: the played Digimon is deleted at the end of the OPPONENT's turn —
/// it survives the controller's own turn end, then is deleted when the
/// opponent's turn ends.
#[test]
fn bt23_037_played_hudie_deleted_at_end_of_opponents_turn() {
    let mut runner = attacking_runner();
    push_to_hand(&mut runner, 0, "HUDIE-3");
    let attacker = stack_tentomon_attacker(&mut runner);

    let _ = runner.attack_player(attacker, 1, false);
    let (player, action_id) = {
        let view = runner
            .pending_selection_view()
            .expect("play prompt installed");
        let first = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|id| *id != PASS)
            .expect("a Hudie candidate");
        (view.selecting_player, first)
    };
    runner.execute_action(player, action_id).expect("play resolves");
    let _ = runner.auto_resolve();

    let played_card_index = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "HUDIE-3")
        .map(|p| p.top_card().card_index)
        .expect("HUDIE-3 played");

    // End P0's OWN turn — the Hudie must SURVIVE (deletion is at opp turn end).
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_index == played_card_index),
        "the played Hudie must survive the controller's own turn end"
    );

    // End P1's (opponent's) turn — now the Hudie is deleted.
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_index == played_card_index),
        "the played Hudie must be deleted at the end of the opponent's turn"
    );
}

/// OPT: a second attack in the SAME turn must NOT re-offer the play; the
/// lockout clears on the controller's next turn.
#[test]
fn bt23_037_when_attacking_opt_locks_second_attack_and_clears_next_turn() {
    let mut runner = attacking_runner();
    push_to_hand(&mut runner, 0, "HUDIE-3");
    push_to_hand(&mut runner, 0, "HUDIE-5");
    let attacker = stack_tentomon_attacker(&mut runner);

    // First attack — fires the inherited clause; decline the play (we only
    // care about OPT consumption, which fires on activation).
    let _ = runner.attack_player(attacker, 1, false);
    let player = runner
        .pending_selection_view()
        .expect("first attack installs the play prompt")
        .selecting_player;
    runner.execute_action(player, PASS).expect("decline");
    let _ = runner.auto_resolve();

    // Unsuspend the attacker so it can attack again this turn.
    runner.game.players[0].battle_area[attacker.index as usize].is_suspended = false;

    // Second attack SAME turn — OPT lockout, no prompt installs.
    let _ = runner.attack_player(attacker, 1, false);
    assert!(
        runner.pending_selection().is_none(),
        "[Once Per Turn] must block the second attack in the same turn; got {:?}",
        runner.pending_selection_view().map(|v| v.kind)
    );

    // Cycle to P0's next turn — the lockout clears.
    for _ in 0..6 {
        runner.end_turn();
        let _ = runner.auto_resolve();
        if runner.game.turn_player() == 0 {
            break;
        }
    }
    assert_eq!(runner.game.turn_player(), 0, "back to P0's turn");

    // Re-attack on the new turn — the prompt installs again (OPT cleared).
    let attacker = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "CS-LV4")
        .map(|i| PermanentHandle {
            player: 0,
            index: i as u8,
        })
        .expect("attacker still on field");
    runner.game.players[0].battle_area[attacker.index as usize].is_suspended = false;
    let _ = runner.attack_player(attacker, 1, false);
    assert!(
        runner.pending_selection().is_some(),
        "OPT lockout must clear on the new turn — the play prompt installs again"
    );
}

/// NEGATIVE ([When Attacking] window): the inherited clause is a When Attacking
/// trigger, so on the OPPONENT's attack it must not fire for the controller.
#[test]
fn bt23_037_when_attacking_does_not_fire_on_opponent_attack() {
    let mut runner = attacking_runner();
    push_to_hand(&mut runner, 0, "HUDIE-3");
    // BT23-037 is a source under a P0 Digimon (inherited active), but the
    // ATTACK is made by the OPPONENT.
    stack_tentomon_attacker(&mut runner);
    let opp_attacker = runner.place_on_field(1, "CS-LV4", Some(0));

    // Make it P1's turn so the opponent can attack.
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 1, "P1's turn");

    let _ = runner.attack_player(opp_attacker, 0, false);
    // No P0 play prompt may install (the When Attacking is the attacker's own).
    let installed_for_p0 = runner
        .pending_selection_view()
        .map(|v| v.selecting_player == 0)
        .unwrap_or(false);
    assert!(
        !installed_for_p0,
        "the opponent attacking must not fire P0's inherited When Attacking"
    );
}
