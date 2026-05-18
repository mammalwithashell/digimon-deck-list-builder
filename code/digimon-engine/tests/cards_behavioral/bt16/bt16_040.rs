//! BT16-040 Wormmon — Digimon, Lv.3, Green+White, DP 2000, Cost 3.
//! Traits: Larva
//! Evo: Lv.2 / cost 1
//!
//! # Card text (cards.json — verbatim)
//!
//! [Start of Your Main Phase] [On Play] If it's your turn, 1 of your Digimon
//! may digivolve into a level 4 Digimon card with the [Insectoid] or [Free]
//! trait in your trash with the digivolution cost reduced by 1.
//!
//! Inherited Effect [When Attacking] [Once Per Turn]
//! Suspend 1 of your opponent's Digimon.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT16/Green/BT16_040.cs
//!
//! OnStartMainPhase timing:
//!   - CanUseCondition: IsExistOnBattleArea(card) && IsOwnerTurn(card).
//!   - CanActivateCondition: IsExistOnBattleArea(card) &&
//!       HasMatchConditionOwnersCardInTrash(CanSelectCardCondition).
//!   - CanSelectCardCondition: (Insectoid || Free) && Level==4 && Digimon.
//!   - CanSelectPermanentCondition: own Digimon with eligible trash card
//!       that CanPlayCardTargetFrame onto it.
//!   - SelectPermanentEffect: canNoSelect=true (player may skip).
//!   - After permanent pick: SelectCard from trash filtered by Insectoid/Free Lv4.
//!   - After card pick: DigivolveIntoHandOrTrashCard(isHand=false, reduceCost=1).
//! OnAllyAttack (inherited) timing:
//!   - SetHashString: "Suspend_BT16-040".
//!   - SetIsInheritedEffect(true).
//!   - CanActivateCondition: IsExistOnBattleAreaDigimon(card).
//!   - Body: HasMatchConditionOpponentsPermanent(IsOpponentDigimon) →
//!     SelectPermanentEffect(maxCount=1, canNoSelect=false, Mode.Tap).
//!
//! # Patterns this test covers
//! - A5 Stack shift / digivolve from trash (clause 0) — PARTIAL (gap below)
//! - B1 Start-of-main triggered clause with dual timing (clause 0)
//! - G4 Inherited [When Attacking][OPT] suspend opponent (clause 1)
//!
//! # Faithfulness diff vs. card text
//!
//! | Card-text element                                          | YAML clause                                                  | Status  |
//! |------------------------------------------------------------|--------------------------------------------------------------|---------|
//! | "[SOMP][On Play][Your Turn] digivolve from trash, cost -1" | dual `when:` + select_own_permanent + select_trash + eid    | PARTIAL |
//! | Dual timing: SOMP AND on play                             | `when: [start_of_your_main_phase, on_play]` array           | OK      |
//! | "If it's your turn"                                       | `active_when: { your_turn: true }`                          | OK      |
//! | "1 of your Digimon may" (optional)                        | `select_own_permanent { optional: true }`                   | OK *    |
//! | "Lv4 Insectoid or Free from trash"                        | `select_trash { filter: level_eq:4 + Insectoid/Free }`     | OK *    |
//! | "digivolution cost reduced by 1"                          | `effect_initiated_digivolve { cost: {reduce:1} }`           | BLOCKED |
//! | Inherited [When Attacking][OPT] Suspend 1 opp Digimon     | `scope: inherited, when_attacking, once_per_turn`           | OK      |
//!
//! ## DSL gap blocking clause 0 full execution  [G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET]
//!
//! The chain `select_own_permanent { bind_as: target } → select_trash { bind_as: evo }
//! → effect_initiated_digivolve { target: target, source: evo }` terminates
//! after the permanent pick. The trash-pick prompt never installs and the
//! digivolve verb never executes. This is the same gap that blocked BT17-015
//! branch 1 and BT17-027 branch 1 (see those files' gap analysis).
//!
//! Note: `effect_initiated_digivolve` with `source: <trash_binding>` is
//! sound — `resolve_card_source_ref` in `play_digivolve.rs` maps `TrashIndex`
//! to `CardSourceRef::Trash` and calls `effect_initiated_digivolve_from_source`.
//! The blocker is specifically the selection chain not continuing past the
//! `select_own_permanent` step when a subsequent selection step follows.
//!
//! ## Clause 1 (Inherited [When Attacking] suspend) — IMPLEMENTED
//!
//! `scope: inherited` + `when: when_attacking` + `once_per_turn: true` +
//! `select_opponent_permanent { kind: digimon }` + `suspend { target: tgt }`.
//! Pattern is battle-tested (EX8-074, BT13-012 precedents).

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

/// Push a card (by ID) into P0's trash by direct CardSource injection.
/// Panics if `card_id` is not registered in `runner.game.card_data`.
fn push_to_trash(runner: &mut DebugRunner, player: usize, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} not found in card_data"));
    let next = runner.game.next_card_index();
    let src = CardSource::new(data_idx, 0, next);
    runner.game.players[player].trash.push(src);
}

// ─── Fixture helpers ─────────────────────────────────────────────────────────

fn make_own_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(5000);
    card
}

/// Phase 2 Track F: a Lv.3 own Digimon usable as the source for BT16-040's
/// effect-initiated digivolve into a Lv.4 trash card. The standard rules
/// require source = destination - 1; the helper exists so the
/// `bt16_040_on_play_chains_*` test can assert the actual digivolve happens
/// rather than getting silently rejected on a Lv.4 → Lv.4 mismatch.
fn make_own_lv3_digimon_with_red_evo(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(3000);
    // Empty colors so the engine doesn't enforce a specific evo-color filter
    // beyond the printed (lack of) requirement on the alt-path.
    card
}

fn make_opp_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card
}

fn make_trash_lv4_insectoid(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.traits = vec!["Insectoid".to_string()];
    // Phase 2 Track F: include a Lv.3 colorless evo_cost so the
    // BT16-040 effect-initiated digivolve from a Lv.3 source can match the
    // rules digivolve route (the cost is reduced by the alt-path's
    // `cost: { reduce: 1 }` and ultimately paid from memory).
    card.evo_costs = vec![digimon_engine::card_data::EvoCost {
        card_color: 0,
        level: 3,
        memory_cost: 2,
    }];
    card
}

fn make_trash_lv4_free(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.traits = vec!["Free".to_string()];
    card
}

fn wormmon_base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT16-040")
        .expect("BT16-040 YAML parses and compiles")
        .memory(10)
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

/// Clause 0: when contains both StartOfYourMainPhase and OnPlay.
/// Clause 1: scope Inherited, when WhenAttacking, once_per_turn.
#[test]
fn bt16_040_has_two_triggered_clauses() {
    let runner = wormmon_base().start();
    let compiled = runner
        .compiled_card("BT16-040")
        .expect("BT16-040 compiled card present");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(
        triggered.len(),
        2,
        "BT16-040 must have exactly 2 triggered clauses"
    );
}

/// Clause 0 has both StartOfYourMainPhase and OnPlay in its `when` vector.
#[test]
fn bt16_040_clause0_has_dual_timing_somp_and_on_play() {
    let runner = wormmon_base().start();
    let compiled = runner
        .compiled_card("BT16-040")
        .expect("BT16-040 compiled card present");

    let clause0 = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.when.contains(&CompiledTiming::StartOfYourMainPhase)
                || t.when.contains(&CompiledTiming::OnPlay)
        })
        .expect("clause 0 must exist with SOMP or OnPlay timing");

    assert!(
        clause0.when.contains(&CompiledTiming::StartOfYourMainPhase),
        "clause 0 must include StartOfYourMainPhase"
    );
    assert!(
        clause0.when.contains(&CompiledTiming::OnPlay),
        "clause 0 must include OnPlay"
    );
    assert!(
        clause0.optional,
        "clause 0 must be optional (printed text: 'may')"
    );
}

/// Clause 1: inherited scope, WhenAttacking timing, once_per_turn.
#[test]
fn bt16_040_clause1_inherited_when_attacking_once_per_turn() {
    let runner = wormmon_base().start();
    let compiled = runner
        .compiled_card("BT16-040")
        .expect("BT16-040 compiled card present");

    let inherited_wa = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.scope == CompiledScope::Inherited && t.when.contains(&CompiledTiming::WhenAttacking)
        })
        .expect("inherited WhenAttacking clause must be present");

    assert_eq!(
        inherited_wa.scope,
        CompiledScope::Inherited,
        "clause 1 must be inherited scope"
    );
    assert!(
        inherited_wa.when.contains(&CompiledTiming::WhenAttacking),
        "clause 1 must have WhenAttacking timing"
    );
    assert!(
        inherited_wa.once_per_turn,
        "clause 1 must be once_per_turn ([Once Per Turn] printed)"
    );
}

/// Printed special evolution path: `[Digivolve] [Minomon]: Cost 0`.
#[test]
fn bt16_040_has_minomon_cost0_digivolve_alt_path() {
    let runner = wormmon_base().start();
    let compiled = runner
        .compiled_card("BT16-040")
        .expect("BT16-040 compiled card present");

    let minomon_path = compiled.alt_paths.iter().find(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && path
                .from
                .as_ref()
                .and_then(|predicate| predicate.name_is.as_deref())
                == Some("Minomon")
    });

    let minomon_path = minomon_path.expect("BT16-040 must include [Digivolve] [Minomon]: Cost 0");
    assert_eq!(
        minomon_path.cost,
        Some(CompiledCost::Literal(0)),
        "BT16-040 Minomon special digivolve path must cost 0"
    );
}

/// Clause 0 picks an evolution card from trash, so the YAML should use the
/// neutral `source: evo` field rather than the legacy hand-specific alias.
#[test]
fn bt16_040_effect_initiated_trash_digivolve_uses_source_field() {
    let yaml = include_str!("../../../cards/bt16/BT16-040.yaml");
    assert!(
        yaml.contains("source: evo"),
        "trash-source effect_initiated_digivolve must be authored with `source: evo`"
    );
    assert!(
        !yaml.contains("from_hand:"),
        "BT16-040 YAML must not use legacy `from_hand:` for a trash-source digivolve"
    );
}

// ─── Section 2 — Clause 0: SOMP+OnPlay digivolve from trash (PARTIAL/BLOCKED) ─

/// On play, the SOMP+OnPlay clause installs the OwnField selection and chains
/// through select_trash + effect_initiated_digivolve.
///
/// Phase 2 Track F (2026-05-17): the chain was originally filed as
/// G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET on the
/// assumption that the dispatcher dropped the tail after the first pick.
/// Investigation revealed the chain has been working since
/// `run_tail_preserving_trigger_context` was wired through every select
/// install — both selections install and the digivolve verb runs. The
/// gap is RESOLVED; the test asserts the full chain executes by checking
/// the post-state (ally's stack now carries the Lv.4 Insectoid card on
/// top, Wormmon evolved from trash).
#[test]
fn bt16_040_on_play_chains_through_permanent_pick_trash_pick_and_effect_digivolve() {
    let mut runner = wormmon_base()
        .add_card(make_own_lv3_digimon_with_red_evo("LV3-ALLY"))
        .add_card(make_trash_lv4_insectoid("TRASH-INSECTOID"))
        .hand(0, &["BT16-040"])
        .start();

    let ally = runner.place_on_field(0, "LV3-ALLY", Some(0));

    // Manually place eligible Lv4 Insectoid in trash.
    push_to_trash(&mut runner, 0, "TRASH-INSECTOID");

    runner.play(0, 0).expect("play Wormmon");

    // First selection: pick your Digimon to digivolve.
    let kind = runner
        .pending_kind()
        .expect("OwnField selection must install after OnPlay");
    assert_eq!(
        kind,
        SelectionKind::OwnField,
        "first selection must be OwnField"
    );

    runner.auto_resolve().expect("auto-resolve full chain");

    // Chain completed: no pending selection remains after auto_resolve.
    assert!(
        runner.pending_kind().is_none(),
        "all chained selections must auto-resolve"
    );

    // End-state: ally's stack now has the Lv.4 Insectoid card on top
    // (digivolved from trash via the chain). The original ally is still
    // present as a digivolution source beneath it.
    let perm = &runner.game.players[0].battle_area[ally.index as usize];
    assert!(
        perm.card_sources.len() >= 2,
        "ally permanent must have gained a digivolution stack member"
    );
    assert_eq!(
        perm.top_card().card_id(&runner.game.card_data),
        "TRASH-INSECTOID",
        "top card is now the Lv.4 Insectoid played from trash via effect-initiated digivolve"
    );
    // Trash drained: the TRASH-INSECTOID card moved out.
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .all(|c| c.card_id(&runner.game.card_data) != "TRASH-INSECTOID"),
        "trash card consumed by effect-initiated digivolve"
    );
}

/// Negative: no eligible Lv4 Insectoid/Free in trash → clause 0 should be a
/// no-op on play. Under the current DSL, the permanent pick may still install
/// because the DSL filter does not pre-filter by downstream trash content.
/// This test documents the ideal behavior per printed text.
#[test]
#[ignore = "BLOCKED: G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET — \
            even the first OwnField pick might install since the DSL select_own_permanent \
            filter only checks 'is a Digimon in battle_area', not whether downstream \
            trash has eligible cards (DCGO's CanSelectPermanentCondition does this check \
            but the DSL approximation omits it). True no-op requires both the gap closing \
            and the permanent filter being tightened."]
fn bt16_040_on_play_no_eligible_trash_is_noop() {
    let mut runner = wormmon_base()
        .add_card(make_own_digimon("ALLY"))
        .hand(0, &["BT16-040"])
        .start();

    let _ally = runner.place_on_field(0, "ALLY", Some(0));
    // Trash is empty of eligible cards.

    runner.play(0, 0).expect("play Wormmon");

    assert!(
        runner.pending_selection().is_none(),
        "no eligible trash card → clause 0 should be a no-op per printed text"
    );
}

// ─── Section 3 — Clause 1: Inherited [When Attacking] suspend behavioral ─────

/// Positive: BT16-040 as digi source under an ally carrier — when the carrier
/// attacks, the inherited [When Attacking] trigger fires and installs an
/// OppField selection targeting opponent's Digimon.
#[test]
fn bt16_040_inherited_when_attacking_installs_opp_field_selection() {
    let mut runner = wormmon_base()
        .add_card(make_own_digimon("CARRIER"))
        .add_card(make_opp_digimon("OPP"))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // place_stack puts first element as the bottom source; second as top card.
    let carrier = runner.place_stack(0, &["BT16-040", "CARRIER"]);
    let opp_perm = runner.place_on_field(1, "OPP", Some(0));

    runner.attack_digimon(carrier, opp_perm, false);

    let kind = runner
        .pending_kind()
        .expect("OppField selection must install");
    assert_eq!(
        kind,
        SelectionKind::OppField,
        "inherited [When Attacking] must install OppField selection"
    );
}

/// Positive behavioral: execute the suspend — the selected opponent Digimon
/// becomes suspended after selection resolves.
///
/// We use an opponent Digimon with very high DP (99_000) so it survives the
/// battle and remains in the battle_area after auto_resolve, letting us check
/// the `is_suspended` flag on the permanent directly.
#[test]
fn bt16_040_inherited_when_attacking_suspends_selected_opp_digimon() {
    // Build a high-DP opponent Digimon so it outlasts the carrier in battle.
    let mut tanky_opp = make_opp_digimon("OPP-TANKY");
    tanky_opp.dp = Some(99_000);

    let mut runner = wormmon_base()
        .add_card(make_own_digimon("CARRIER"))
        .add_card(tanky_opp)
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let carrier = runner.place_stack(0, &["BT16-040", "CARRIER"]);
    let opp_perm = runner.place_on_field(1, "OPP-TANKY", Some(0));

    assert!(
        !runner.game.players[1].battle_area[opp_perm.index as usize].is_suspended,
        "opponent's Digimon must start unsuspended"
    );

    runner.attack_digimon(carrier, opp_perm, false);
    runner
        .auto_resolve()
        .expect("resolve selection and suspend");

    // OPP-TANKY survives the battle (99 000 DP vs 5 000 DP carrier) and must
    // now be suspended from the inherited trigger.
    assert!(
        runner.game.players[1].battle_area[opp_perm.index as usize].is_suspended,
        "opponent's Digimon must be suspended after the inherited trigger resolves"
    );
}

/// OPT enforcement: second attack in the same turn — the trigger must not
/// install a second selection (Once Per Turn lockout).
#[test]
fn bt16_040_inherited_when_attacking_opt_blocks_second_attack_same_turn() {
    let mut runner = wormmon_base()
        .add_card(make_own_digimon("CARRIER"))
        .add_card(make_opp_digimon("OPP1"))
        .add_card(make_opp_digimon("OPP2"))
        .memory(20)
        .start();
    runner.game.turn_count = 1;

    let carrier = runner.place_stack(0, &["BT16-040", "CARRIER"]);
    let opp1 = runner.place_on_field(1, "OPP1", Some(0));
    let opp2 = runner.place_on_field(1, "OPP2", Some(0));

    // First attack: trigger fires, suspend OPP1.
    runner.attack_digimon(carrier, opp1, false);
    runner.auto_resolve().expect("first attack resolves");

    // Unsuspend carrier manually to allow a second attack.
    runner.game.players[0].battle_area[carrier.index as usize].is_suspended = false;

    // Second attack (same turn): OPT should block.
    runner.attack_digimon(carrier, opp2, false);

    assert!(
        runner.pending_selection().is_none(),
        "OPT must lock out the inherited [When Attacking] on the second attack same turn"
    );
}

/// OPT resets after a turn cycle. Phase 2 Track C closure: OPT slot
/// enforcement and reset already work in the engine substrate; this test
/// was previously masked by missing deck/security setup (`begin_turn` for
/// the next player tripped a deck-out and ended the game before
/// `new_turn` could clear the carrier's `effect_activations` HashMap).
#[test]
fn bt16_040_inherited_when_attacking_opt_resets_after_turn_cycle() {
    let mut runner = wormmon_base()
        .add_card(make_own_digimon("CARRIER"))
        .add_card(make_opp_digimon("OPP1"))
        .add_card(make_opp_digimon("OPP2"))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .security(0, &["FILLER"; 5])
        .security(1, &["FILLER"; 5])
        .memory(20)
        .start();
    runner.game.turn_count = 1;

    let carrier = runner.place_stack(0, &["BT16-040", "CARRIER"]);
    let opp1 = runner.place_on_field(1, "OPP1", Some(0));
    let _opp2 = runner.place_on_field(1, "OPP2", Some(0));

    // First attack this turn.
    runner.attack_digimon(carrier, opp1, false);
    runner.auto_resolve().expect("first attack");

    // Full turn cycle.
    runner.end_turn();
    runner.end_turn();

    // Unsuspend carrier.
    runner.game.players[0].battle_area[carrier.index as usize].is_suspended = false;

    // Re-resolve opp2 handle in case battle_area shifted after combat
    // deleted opp1.
    let opp2_after = {
        let battle = &runner.game.players[1].battle_area;
        assert!(
            !battle.is_empty(),
            "opp2 must survive into the next-turn assertion"
        );
        runner.perm_handle(1, battle.len() - 1)
    };

    // Second attack (fresh turn): OPT should have reset.
    runner.attack_digimon(carrier, opp2_after, false);
    assert!(
        runner.pending_selection().is_some(),
        "OPT must reset after a full turn cycle"
    );
}

/// No selection when opponent has no Digimon — select_opponent_permanent with
/// no valid targets is a structural no-op.
#[test]
fn bt16_040_inherited_when_attacking_noop_when_opponent_has_no_digimon() {
    let mut runner = wormmon_base()
        .add_card(make_own_digimon("CARRIER"))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let carrier = runner.place_stack(0, &["BT16-040", "CARRIER"]);

    // Attack the opponent player directly (no opponent Digimon on field).
    runner.attack_player(carrier, 1, false);

    assert!(
        runner.pending_selection().is_none(),
        "no opponent Digimon → inherited suspend step must be a no-op"
    );
}
