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
//! → effect_initiated_digivolve { target: target, from_hand: evo }` terminates
//! after the permanent pick. The trash-pick prompt never installs and the
//! digivolve verb never executes. This is the same gap that blocked BT17-015
//! branch 1 and BT17-027 branch 1 (see those files' gap analysis).
//!
//! Note: `effect_initiated_digivolve` with `from_hand: <trash_binding>` is
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

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
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

// ─── Section 2 — Clause 0: SOMP+OnPlay digivolve from trash (PARTIAL/BLOCKED) ─

/// On play, the SOMP+OnPlay clause may install a selection when there is an
/// eligible Digimon target to digivolve into.
/// BLOCKED by G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET:
/// chain terminates after the permanent pick; the trash-pick prompt never
/// installs and the digivolve verb never executes.
#[test]
#[ignore = "BLOCKED: G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET — \
            select_own_permanent + select_trash + effect_initiated_digivolve(target: <binding>) \
            chain terminates after the permanent pick; the trash-pick prompt never installs \
            and the digivolve verb never executes. \
            Compare BT17-015 branch 1 and BT17-027 branch 1 (same gap)."]
fn bt16_040_on_play_with_eligible_trash_installs_own_field_then_trash_selection() {
    let mut runner = wormmon_base()
        .add_card(make_own_digimon("ALLY"))
        .add_card(make_trash_lv4_insectoid("TRASH-INSECTOID"))
        .hand(0, &["BT16-040"])
        .start();

    let ally = runner.place_on_field(0, "ALLY", Some(0));

    // Manually place eligible Lv4 Insectoid in trash.
    push_to_trash(&mut runner, 0, "TRASH-INSECTOID");

    runner.play(0, 0).expect("play Wormmon");

    // First selection: pick your Digimon.
    let kind = runner
        .pending_kind()
        .expect("OwnField selection must install after OnPlay");
    assert_eq!(
        kind,
        SelectionKind::OwnField,
        "first selection must be OwnField"
    );
    runner.auto_resolve().expect("pick Digimon");

    // Second selection (gap-blocked): trash pick should install but won't under the gap.
    let kind2 = runner.pending_kind();
    assert_eq!(
        kind2,
        Some(SelectionKind::Trash),
        "second selection must be Trash (pick Lv4 Insectoid/Free card)"
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

/// OPT resets after a turn cycle.
/// NOTE: Affected by G-OPT-RESET-VIA-ATTACK-CYCLE — inherited [When Attacking]
/// OPT may not re-fire after a full P0→P1→P0 turn cycle (observed on BT17-015).
#[test]
#[ignore = "BLOCKED: G-OPT-RESET-VIA-ATTACK-CYCLE — inherited [When Attacking][OPT] may \
            not re-fire on a fresh attack after a full P0→P1→P0 turn cycle (same structural \
            gap observed on BT17-015). Verify and remove ignore once the gap closes."]
fn bt16_040_inherited_when_attacking_opt_resets_after_turn_cycle() {
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

    // First attack this turn.
    runner.attack_digimon(carrier, opp1, false);
    runner.auto_resolve().expect("first attack");

    // Full turn cycle.
    runner.end_turn();
    runner.end_turn();

    // Unsuspend carrier.
    runner.game.players[0].battle_area[carrier.index as usize].is_suspended = false;

    // Second attack (fresh turn): OPT should have reset.
    runner.attack_digimon(carrier, opp2, false);
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
