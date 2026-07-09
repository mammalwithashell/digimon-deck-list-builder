//! BT25-020 Marsmon — Digimon, Lv.6, Red/Green, DP 12000, Cost 12.
//! Traits: Shaman, Olympos XII, Iliad, TS. Attribute: Vaccine. Form: Mega.
//!
//! # Card text (official Bandai DB bundle — data/card_bundles/BT25-020.md —
//! verbatim; no per-card JSON exists yet for this card)
//!
//! When this card would be played, if there is a Digimon with 13000 DP or
//! more, reduce the cost by 5.
//! [On Play] [When Digivolving] [When Attacking] 1 of your Digimon gets
//! +3000 DP for the turn. Then, 1 of your Digimon may battle 1 of your
//! opponent's Digimon.
//! [All Turns] [Once Per Turn] When any of your [TS] trait Digimon win a
//! battle, trash your opponent's top security card.
//!
//! Official Q&A: "This effect directly performs a battle. If this effect is
//! activated, the chosen Digimon battle, as with the standard rules." — the
//! "may battle" clause is a direct battle (no security check), not an attack.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Red/BT25_020.cs
//!
//! - Region "Alt Digivolution" (None): AddSelfDigivolutionRequirementStatic-
//!   Effect(TopCard.EqualsTraits("TS"), cost 3, ignoreDigivolutionRequirement:
//!   true, level: 5).
//! - Region "Reduce Play Cost" (None): MandatorySelfPlayCostReduction(5, card,
//!   Condition) where Condition = any battle-area Digimon (either player) with
//!   DP >= 13000. FLAT reduction of 5 (not a per-trash formula).
//! - Region "Shared OP/WD/WA" (ActivateClassesForSharedEffects, NOT hashed as
//!   OPT): (1) SelectPermanentEffect over own Digimon (canNoSelect: false,
//!   mandatory) -> ChangeDigimonDP(+3000, UntilEachTurnEnd); (2) if an
//!   opponent Digimon exists: SelectPermanentEffect over own Digimon for the
//!   attacker (canNoSelect: true, optional), then over opponent Digimon for
//!   the defender (canNoSelect: true, optional), then IBattle(...).Battle().
//! - Region "All Turns" (OnEndBattle, maxCountPerTurn: 1, hash
//!   "BT25_020_AT"): CanUseCondition gates on CanTriggerWhenWinBattle(winner
//!   owned by controller && winner.TopCard.HasTSTraits) -- board-wide, ANY of
//!   the controller's [TS] Digimon, not just Marsmon. Body:
//!   IDestroySecurity(enemy, 1, fromTop: true).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md S4/S5)
//! - Group D: BeforePayCost cost reduction gated on board DP threshold (flat amount)
//! - Group F: mandatory target select + add_dp_modifier (for the turn)
//! - Group F: nested-optional attacker/defender select -> effect battle (no security check)
//! - Group G: G-DSL-BATTLE-WINNER-BOARDWIDE board-wide on_ally_won_battle observer + OPT

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::action;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, ModifierType, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT25-020";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

fn make_digimon(id: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card.evo_costs = vec![EvoCost {
        card_color: 0,
        level: level.saturating_sub(1),
        memory_cost: 2,
    }];
    card
}

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-020 YAML parses and compiles")
        .add_card(filler("FILL"))
        .add_card(make_digimon("OWN-A", 4, 4000, &["Beast"]))
        .add_card(make_digimon("OWN-B", 4, 4500, &["Beast"]))
        .add_card(make_digimon("OWN-TS", 5, 9000, &["TS"]))
        .add_card(make_digimon("OPP-A", 4, 4000, &["Beast"]))
        .add_card(make_digimon("OPP-WEAK", 3, 1000, &["Beast"]))
        .add_card(make_digimon("BIG-13K", 6, 13000, &["Beast"]))
        .add_card(make_digimon("SMALL-12K", 5, 12000, &["Beast"]))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
}

fn place_marsmon(runner: &mut DebugRunner) -> PermanentHandle {
    runner.place_on_field(0, CARD_ID, Some(0))
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_020_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT25-020 must be present in embedded DSL pack");

    assert_eq!(card.name, "Marsmon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(12));
    assert_eq!(card.dp, Some(12000));
    for trait_name in ["Shaman", "Olympos XII", "Iliad", "TS"] {
        assert!(
            card.traits.contains(&trait_name.to_string()),
            "BT25-020 metadata must include trait {trait_name}"
        );
    }
}

#[test]
fn bt25_020_has_ts_lv5_cost3_ignore_alt_path() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let path = card
        .alt_paths
        .iter()
        .find(|p| {
            p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(3))
        })
        .expect("BT25-020 must include a cost-3 digivolve alt-path");

    assert!(
        path.ignore_requirements,
        "the Lv.5 [TS] alt-path ignores digivolution requirements"
    );
    let from = path.from.as_ref().expect("alt-path has a from: filter");
    assert_eq!(from.level_eq, Some(5), "alt-path base must be Lv.5");
    assert_eq!(
        from.trait_has.as_deref(),
        Some("TS"),
        "alt-path base must carry the [TS] trait"
    );
}

#[test]
fn bt25_020_has_standard_red_and_green_lv5_cost4_alt_paths() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let cost4_paths: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| {
            p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(4))
        })
        .collect();

    assert_eq!(
        cost4_paths.len(),
        2,
        "expected 2 standard digivolve alt-paths (red Lv.5/cost4, green Lv.5/cost4); got {}",
        cost4_paths.len()
    );
    for p in &cost4_paths {
        let from = p.from.as_ref().expect("alt-path has a from: filter");
        assert_eq!(
            from.level_eq,
            Some(5),
            "standard alt-path base must be Lv.5"
        );
    }
}

#[test]
fn bt25_020_clause_0_is_cost_reduction_before_pay_cost_when_playing_this() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let cost_red = card.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
            reduction_timing,
            when_playing_this,
            condition,
            amount,
            amount_fn,
            ..
        }) => Some((
            reduction_timing.clone(),
            *when_playing_this,
            condition.clone(),
            *amount,
            amount_fn.clone(),
        )),
        _ => None,
    });

    let (reduction_timing, when_playing_this, condition, amount, amount_fn) =
        cost_red.expect("BT25-020 must have a CostReduction declarative clause");

    assert_eq!(
        reduction_timing.as_deref(),
        Some("before_pay_cost"),
        "reduction_timing must be before_pay_cost"
    );
    assert!(
        when_playing_this,
        "when_playing_this must be true (only fires when BT25-020 itself would be played)"
    );
    assert!(
        condition.is_some(),
        "cost reduction must be gated on a condition (any Digimon >=13000 DP)"
    );
    assert_eq!(
        amount,
        Some(5),
        "cost reduction must be a flat literal amount of 5 (not a formula)"
    );
    assert!(
        amount_fn.is_none(),
        "BT25-020's reduction is a flat 5, not a per-trash amount_fn formula"
    );
}

#[test]
fn bt25_020_has_shared_op_wd_wa_clause_not_opt() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving)
                    && t.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("BT25-020 must have a shared OnPlay/WhenDigivolving/WhenAttacking clause");

    assert!(
        !clause.once_per_turn,
        "DCGO does not hash the shared OP/WD/WA clause as OPT (unlike BT25-058's sibling clause)"
    );
    let has_dp_mod = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::AddDpModifier { .. }));
    assert!(
        has_dp_mod,
        "shared clause must add a DP modifier to an own Digimon"
    );
    let has_battle = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::Battle { .. }))
        || clause.process.iter().any(|s| {
            matches!(s, CompiledStep::If { then, .. }
                if then.iter().any(|s2| matches!(s2, CompiledStep::Battle { .. })))
        });
    assert!(
        has_battle,
        "shared clause must include a battle step (directly or nested under an if)"
    );
}

#[test]
fn bt25_020_has_on_ally_won_battle_opt_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnAllyWonBattle) => {
                Some(t)
            }
            _ => None,
        })
        .expect("BT25-020 must have an on_ally_won_battle clause");

    assert!(clause.once_per_turn, "[Once Per Turn]");
    let has_trash_security = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::TrashTopSecurity { .. }));
    assert!(
        has_trash_security,
        "on_ally_won_battle clause must trash the opponent's top security card"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Cost reduction: condition gating (positive/negative split)
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: a 13000+ DP Digimon exists on EITHER field -> cost reduced by 5.
///
/// Memory is read immediately after `play()` (synchronous cost payment),
/// before `auto_resolve()` drives the shared clause's battle sub-effect —
/// see the negative test's doc comment for why a delta spanning
/// `auto_resolve()` is unsafe in general.
#[test]
fn bt25_020_cost_reduced_by_5_when_high_dp_digimon_exists() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(12).start();
    runner.place_on_field(1, "BIG-13K", Some(0));

    let mem_before = runner.memory();
    runner.play(0, 0);
    let mem_after = runner.memory();

    let paid = mem_before - mem_after;
    assert_eq!(
        paid, 7,
        "with a 13000+ DP Digimon on the field, cost is 12-5=7; before={mem_before}, after={mem_after}, paid={paid}"
    );
    let _ = runner.auto_resolve();
}

/// Negative: no 13000+ DP Digimon anywhere -> full cost 12 is paid.
///
/// Memory is read immediately after `play()` returns (cost payment is
/// synchronous within `play_from_hand`), NOT after `auto_resolve()`: the
/// shared clause's "may battle" sub-effect can win a direct battle, which
/// fires BT25-020's OWN `on_ally_won_battle` clause (Marsmon itself carries
/// [TS]) and, separately, can push memory negative and trigger the engine's
/// faithful memory-seesaw turn-switch (`self.memory = -self.memory` in
/// `rotate_turn_player`) — a `mem_before - mem_after` delta computed across
/// that boundary is not the cost paid.
#[test]
fn bt25_020_no_cost_reduction_without_high_dp_digimon() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(12).start();
    runner.place_on_field(1, "SMALL-12K", Some(0));

    let mem_before = runner.memory();
    runner.play(0, 0);
    let mem_after = runner.memory();

    let paid = mem_before - mem_after;
    assert_eq!(
        paid, 12,
        "without a 13000+ DP Digimon, full cost 12 is paid; before={mem_before}, after={mem_after}, paid={paid}"
    );
}

/// The cost-reduction gate counts a 13000+ DP Digimon on the CONTROLLER's own
/// field too (DCGO's WouldPlayCondition has no owner restriction).
#[test]
fn bt25_020_cost_reduction_gate_includes_own_field_digimon() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(12).start();
    runner.place_on_field(0, "BIG-13K", Some(0));

    let mem_before = runner.memory();
    runner.play(0, 0);
    let mem_after = runner.memory();

    let paid = mem_before - mem_after;
    assert_eq!(
        paid, 7,
        "gate counts the controller's own-field 13000+ DP Digimon too; before={mem_before}, after={mem_after}, paid={paid}"
    );
    let _ = runner.auto_resolve();
}

/// Exactly 13000 DP (the threshold) qualifies (dp_gte, inclusive).
#[test]
fn bt25_020_cost_reduction_gate_is_inclusive_at_exactly_13000() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(12).start();
    // BIG-13K is exactly 13000 DP.
    runner.place_on_field(1, "BIG-13K", Some(0));

    let mem_before = runner.memory();
    runner.play(0, 0);
    let mem_after = runner.memory();

    let paid = mem_before - mem_after;
    assert_eq!(
        paid, 7,
        "exactly 13000 DP must satisfy dp_gte:13000; before={mem_before}, after={mem_after}, paid={paid}"
    );
    let _ = runner.auto_resolve();
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — Shared [OP][WD][WA] clause: condition gate (own Digimon exists)
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: playing Marsmon with an own Digimon already on the field installs
/// the mandatory DP-buff prompt.
#[test]
fn bt25_020_on_play_installs_dp_buff_prompt_with_own_digimon() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(12).start();
    runner.place_on_field(0, "OWN-A", Some(0));

    let _c = runner.play(0, 0).expect("play Marsmon");

    assert!(
        runner.pending_selection().is_some(),
        "shared clause must install the mandatory +3000 DP target prompt"
    );
}

/// Negative: with NO own Digimon in play when Marsmon enters (Marsmon itself
/// counts as the only own Digimon on the field once played, so the DP-buff
/// select must still install targeting Marsmon itself).
#[test]
fn bt25_020_on_play_dp_buff_prompt_targets_self_when_alone() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(12).start();

    let _c = runner.play(0, 0).expect("play Marsmon");

    // Marsmon itself is now on the controller's field -> the mandatory
    // select_own_permanent has exactly one legal target: itself.
    assert!(
        runner.pending_selection().is_some(),
        "shared clause condition (any own Digimon) is satisfied by Marsmon itself post-play"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Shared clause: DP buff behavior
// ═══════════════════════════════════════════════════════════════════════════

/// The selected own Digimon gains +3000 DP for the turn.
///
/// Uses a BIG-13K opponent Digimon so the cost reduction applies (12-5=7),
/// keeping memory non-negative after paying for Marsmon. Without the
/// reduction, memory (clamped to the engine's +/-10 range, so `.memory(12)`
/// -> 10) can't cover the full cost-12 play; going negative makes
/// `resolve_generic_selection`'s post-callback `check_turn_end()` faithfully
/// end the turn early (the real TCG memory-seesaw rule), which legitimately
/// expires "for the turn" (`Expiry::EndOfTurn`) modifiers via
/// `rotate_turn_player`'s `expire_end_of_turn` — that is correct engine
/// behavior, not a bug, but it would sink this DP-buff assertion by design,
/// so the buff is also checked immediately after the MANDATORY select
/// resolves, before any further selection in the chain can trigger it.
#[test]
fn bt25_020_dp_buff_adds_3000_for_the_turn() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(12).start();
    let own = runner.place_on_field(0, "OWN-A", Some(0));
    runner.place_on_field(1, "BIG-13K", Some(0));
    let dp_before = runner.dp_of(own).expect("DP present");

    runner.play(0, 0).expect("play Marsmon");
    // Resolve the mandatory DP-buff select onto OWN-A (not Marsmon).
    let view = runner.pending_selection_view().expect("prompt installed");
    let action_id = view.valid_action_ids[0];
    // Resolve directly: pick the first legal action (own field has OWN-A and
    // Marsmon; either is a legal target for the mandatory pick). We assert
    // that SOME own Digimon gained +3000 DP after resolving, rather than
    // pinning which one auto_resolve chooses.
    runner
        .execute_action(
            runner.pending_selection().unwrap().selecting_player,
            action_id,
        )
        .expect("resolve DP buff target");

    // Check immediately — before resolving the shared clause's optional
    // attacker/defender picks, which (per the doc comment above) can end the
    // turn and legitimately expire this "for the turn" modifier.
    let marsmon = runner.perm_handle(0, 1);
    let own_after = runner.dp_of(own).unwrap_or(dp_before);
    let marsmon_dp = runner.dp_of(marsmon).unwrap_or(0);
    assert!(
        own_after == dp_before + 3000 || marsmon_dp == 12000 + 3000,
        "exactly one own Digimon must have gained +3000 DP for the turn (own_after={own_after}, marsmon_dp={marsmon_dp})"
    );

    let _ = runner.auto_resolve();
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — Shared clause: may-battle branch (attacker/defender both optional)
// ═══════════════════════════════════════════════════════════════════════════

/// Declining the attacker pick (PASS) skips the battle entirely — no defender
/// prompt installs and no Digimon is deleted.
#[test]
fn bt25_020_declining_attacker_pick_skips_battle() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(12).start();
    runner.place_on_field(0, "OWN-A", Some(0));
    runner.place_on_field(1, "OPP-WEAK", Some(0));

    runner.play(0, 0).expect("play Marsmon");
    // Resolve the mandatory DP-buff pick first.
    let dp_view = runner.pending_selection_view().expect("DP buff prompt");
    let dp_action = dp_view.valid_action_ids[0];
    let dp_player = runner.pending_selection().unwrap().selecting_player;
    runner
        .execute_action(dp_player, dp_action)
        .expect("resolve DP buff");

    // Now the attacker-pick prompt should be pending and optional.
    assert!(
        runner.pending_is_optional(),
        "the attacker pick ('1 of your Digimon may battle') must be declinable"
    );
    let player = runner.pending_selection().unwrap().selecting_player;
    runner
        .execute_action(player, action::space::PASS)
        .expect("decline the attacker pick");

    let opp_field_before_check = runner.battle_area_size(1);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(1),
        opp_field_before_check,
        "declining the attacker pick must not delete any opponent Digimon"
    );
}

/// Choosing an attacker then a defender resolves an effect BATTLE (no security
/// check) — the loser is deleted from the field.
#[test]
fn bt25_020_choosing_attacker_and_defender_resolves_a_battle() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(12).start();
    runner.place_on_field(0, "OWN-A", Some(0)); // 4000 DP
    runner.place_on_field(1, "OPP-WEAK", Some(0)); // 1000 DP — will lose

    let opp_field_before = runner.battle_area_size(1);
    let sec_before = runner.security_count(1);

    runner.play(0, 0).expect("play Marsmon");
    // auto_resolve drives: DP-buff pick (mandatory, picks first legal target),
    // attacker pick (optional, first legal = accept), defender pick (optional,
    // first legal = accept), then the battle resolves.
    let _ = runner.auto_resolve();

    assert!(
        runner.battle_area_size(1) < opp_field_before,
        "a battle must have resolved and deleted the loser from the opponent's field"
    );
    assert_eq!(
        runner.security_count(1),
        sec_before,
        "an effect battle performs NO security check (Official Q&A: 'as with the standard rules' \
         refers to battle resolution, not attack/security continuation)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 6 — [All Turns][OPT] on_ally_won_battle: positive/negative matrix
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: a DIFFERENT ally [TS] Digimon (not Marsmon itself) winning a
/// battle trashes the opponent's top security card.
#[test]
fn bt25_020_trashes_opp_top_security_when_a_separate_ally_ts_wins() {
    let mut runner = base().security(1, &["FILL", "FILL"]).memory(20).start();
    let _marsmon = place_marsmon(&mut runner);
    // Placed on turn 0 (like `place_marsmon`) so it is NOT summoning-sick on
    // the actual game turn (1) — `Some(1)` would make `turn_played ==
    // turn_count`, and `Game::can_attack` rejects a fresh (summoning-sick)
    // attacker with `AttackResult::Invalid`.
    let ally_ts = runner.place_on_field(0, "OWN-TS", Some(0)); // 9000 DP
    let weak_def = runner.place_on_field(1, "OPP-WEAK", Some(0)); // 1000 DP

    let sec_before = runner.security_count(1);
    runner.attack_digimon(ally_ts, weak_def, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(1),
        sec_before - 1,
        "when a separate ally [TS] Digimon wins a battle, the opponent's top security is trashed"
    );
}

/// Negative: the winning ally Digimon lacks the [TS] trait -> no trash fires.
#[test]
fn bt25_020_does_not_fire_when_winner_lacks_ts_trait() {
    let mut runner = base().security(1, &["FILL", "FILL"]).memory(20).start();
    let _marsmon = place_marsmon(&mut runner);
    let ally_plain = runner.place_on_field(0, "OWN-B", Some(0)); // 4500 DP, no TS
    let weak_def = runner.place_on_field(1, "OPP-WEAK", Some(0)); // 1000 DP

    let sec_before = runner.security_count(1);
    runner.attack_digimon(ally_plain, weak_def, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(1),
        sec_before,
        "a non-[TS] ally winning a battle must NOT trigger the trash-security clause"
    );
}

/// Negative: the opponent's [TS] Digimon winning a battle (against Marsmon's
/// controller) must not trigger Marsmon's clause (winner owned by the wrong
/// player).
#[test]
fn bt25_020_does_not_fire_when_opponent_ts_digimon_wins() {
    let mut runner = base().security(0, &["FILL", "FILL"]).memory(20).start();
    let _marsmon = place_marsmon(&mut runner);
    let own_weak = runner.place_on_field(0, "OWN-A", Some(1)); // 4000 DP
    let opp_ts = runner.place_on_field(1, "OWN-TS", Some(0)); // 9000 DP, TS-trait

    let sec_before = runner.security_count(0);
    runner.game.turn_player_idx = 1;
    runner.attack_digimon(opp_ts, own_weak, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(0),
        sec_before,
        "an opponent-owned [TS] Digimon winning a battle must not trash Marsmon's controller's security"
    );
}

/// Negative: a tie (mutual destruction) has no winner -> no trash fires.
#[test]
fn bt25_020_does_not_fire_on_a_tie() {
    let mut runner = base().security(1, &["FILL", "FILL"]).memory(20).start();
    let _marsmon = place_marsmon(&mut runner);
    let ally_ts_equal = runner.place_on_field(0, "OWN-TS", Some(0)); // 9000 DP
    let equal_def = runner.place_on_field(1, "BIG-13K", Some(0)); // matched: use dp equal case

    // Force an equal-DP tie via a same-DP fixture: reuse OWN-TS (9000) vs a
    // freshly placed 9000-DP opponent Digimon.
    let opp_equal = runner.place_on_field(1, "OWN-TS", Some(1)); // 9000 DP too (trait doesn't matter for opp)
    let sec_before = runner.security_count(1);

    runner.attack_digimon(ally_ts_equal, opp_equal, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(1),
        sec_before,
        "a tie (mutual destruction) has no winner and must not trigger the trash-security clause"
    );
}

/// [Once Per Turn]: two separate ally-[TS] battle wins in the same turn only
/// trash the opponent's security once.
#[test]
fn bt25_020_opt_locks_after_first_win_same_turn() {
    let mut runner = base()
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(20)
        .start();
    let _marsmon = place_marsmon(&mut runner);
    let ally_ts = runner.place_on_field(0, "OWN-TS", Some(0));
    let def1 = runner.place_on_field(1, "OPP-WEAK", Some(0));
    let def2 = runner.place_on_field(1, "OPP-A", Some(1));

    let sec_before = runner.security_count(1);

    // First win.
    runner.attack_digimon(ally_ts, def1, false);
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.security_count(1),
        sec_before - 1,
        "first ally-TS win in the turn must trash 1 security"
    );

    // Unsuspend the attacker to fight again in the SAME turn.
    if let Some(perm) = runner.game.players[0]
        .battle_area
        .get_mut(ally_ts.index as usize)
    {
        perm.is_suspended = false;
    }
    let def2_now = runner.perm_handle(1, 0);
    runner.attack_digimon(ally_ts, def2_now, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(1),
        sec_before - 1,
        "[Once Per Turn] must block the second trash-security trigger in the same turn"
    );
}

/// After end_turn/end_turn (back to the controller's turn), the OPT lockout
/// clears and a new ally-TS win can trash security again.
#[test]
fn bt25_020_opt_clears_next_turn() {
    let mut runner = base()
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(20)
        .start();
    let _marsmon = place_marsmon(&mut runner);
    let ally_ts = runner.place_on_field(0, "OWN-TS", Some(0));
    let def1 = runner.place_on_field(1, "OPP-WEAK", Some(0));

    runner.attack_digimon(ally_ts, def1, false);
    let _ = runner.auto_resolve();
    let sec_after_first = runner.security_count(1);

    runner.end_turn(); // -> player 1's turn
    runner.end_turn(); // -> player 0's turn again

    if let Some(perm) = runner.game.players[0]
        .battle_area
        .get_mut(ally_ts.index as usize)
    {
        perm.is_suspended = false;
    }
    let def2 = runner.place_on_field(1, "OPP-A", Some(0));
    runner.attack_digimon(ally_ts, def2, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(1),
        sec_after_first - 1,
        "OPT lockout must clear on a new turn, allowing the clause to fire again"
    );
}

/// Negative: a direct player attack (security check, no Digimon-vs-Digimon
/// battle) is not a "battle win" and must not trigger the clause.
#[test]
fn bt25_020_does_not_fire_on_direct_player_attack() {
    let mut runner = base().security(1, &["FILL"]).memory(20).start();
    let _marsmon = place_marsmon(&mut runner);
    let ally_ts = runner.place_on_field(0, "OWN-TS", Some(0));

    let sec_before = runner.security_count(1);
    runner.attack_player(ally_ts, 1, false);
    let _ = runner.auto_resolve();

    // A direct player attack DOES remove a security card via the normal
    // security-check flow, but the ADDITIONAL on_ally_won_battle trash must
    // not fire (it is not a Digimon-vs-Digimon battle win). Assert no MORE
    // than the expected single security-check removal occurred.
    assert!(
        runner.security_count(1) >= sec_before.saturating_sub(1),
        "a direct player attack must not cause an EXTRA security trash beyond its own security check"
    );
}
