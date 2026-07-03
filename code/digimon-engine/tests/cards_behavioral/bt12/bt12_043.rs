//! BT12-043 ShineGreymon — Digimon, Lv.6, Yellow/Red, DP 12000, Cost 12.
//! Traits: Light Dragon. Form: Mega. Attribute: Vaccine.
//!
//! # Card text (code/digimon-engine/cards/bt12/BT12-043.json — verbatim)
//!
//! ```text
//! [Digivolve] 3 from Lv.5 w/[RizeGreymon] in name
//!
//! [When Digivolving] For each yellow and/or red Tamer you have in play, 1 of
//! your opponent's Digimon and all of your opponent's Security Digimon get
//! -3000 DP for the turn.
//! [Your Turn] All of your [Marcus Damon] in play get +3000 DP and gain
//! ＜Security A. +1＞ (This Digimon checks 1 additional security card.)
//! ```
//!
//! Standard digivolve circles (data/card_bundles/BT12-043.md — official
//! Bandai DB): Yellow Lv.5 / cost 4, Red Lv.5 / cost 4.
//!
//! Official Q&A (data/card_bundles/BT12-043.md):
//! "No, you can't [choose 2]. You choose 1 of your opponent's Digimon, then
//! that Digimon and all security Digimon get -3000 DP for each of your
//! yellow or red Tamers." — confirms the debuff targets exactly ONE opponent
//! Digimon (mandatory single select) plus the whole opponent security stack,
//! both scaled by the SAME per-Tamer count.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT12/Yellow/BT12_043.cs
//!
//! DCGO crosscheck (per clause):
//! - Region `EffectTiming.None` (`AddSelfDigivolutionRequirementStaticEffect`):
//!   `permanentCondition` = top card contains "RizeGreymon" && level == 5;
//!   `digivolutionCost: 3`. → alt_paths digivolve { level_eq: 5,
//!   name_contains: "RizeGreymon" }, cost 3.
//! - Region `EffectTiming.OnEnterFieldAnyone` (`ActivateClass`):
//!   `CanUseCondition` = `CanTriggerWhenDigivolving`. `CanActivateCondition`
//!   gates on `IsExistOnBattleArea(card)` AND count of own battle-area
//!   permanents that are Tamers AND (Yellow-colored OR Red-colored) >= 1.
//!   `ActivateCoroutine`: recomputes `count`; `minusDP = 3000 * count`; if any
//!   opponent battle-area Digimon exists, mandatorily
//!   (`canNoSelect: false`, `maxCount: Min(1, candidateCount)`) selects ONE
//!   and applies `ChangeDigimonDP(-minusDP, UntilEachTurnEnd)`; THEN
//!   (unconditionally, not gated on the select happening)
//!   `ChangeSecurityDigimonCardDPPlayerEffect(cardCondition: owner == enemy,
//!   -minusDP, UntilEachTurnEnd)` — hits the whole opponent security stack
//!   regardless of whether an opponent battle-area Digimon existed to select.
//!   → when: when_digivolving; condition: any_permanent{of:you, kind:tamer,
//!   any_of[color_is:yellow, color_is:red]}; process: [select_opponent_permanent
//!   (mandatory, kind: digimon) -> add_dp_modifier(formula, expiry:
//!   end_of_turn), add_player_modifier(target_player: opponent, modifier:
//!   ChangeOwnSecurityDigimonDp, value: formula, expiry: end_of_turn)].
//!   UntilEachTurnEnd → end_of_turn (ST1-14 / BT25-018 idiom).
//! - Region `EffectTiming.None` x2 (`ChangeDPStaticEffect` +
//!   `ChangeSAttackStaticEffect`): both gated by `IsExistOnBattleArea(card) &&
//!   IsOwnerTurn(card)`, `permanentCondition` = own battle-area permanent
//!   whose top card's `CardNames` contains "Marcus Damon" (or the
//!   no-space "MarcusDamon" spelling — a defensive DCGO alias, not a second
//!   printed name). → two declarative Aura clauses: `active_when: {
//!   your_turn: true }`, `target: { owner: you, name_contains: "Marcus
//!   Damon" }`; one with `dp_modifier: 3000`, one with `grant_keyword: {
//!   keyword: SecurityAttackPlus, value: 1 }` (two separate DCGO static
//!   effects, faithfully authored as two clauses per the BT5-093 idiom).
//!
//! # Patterns this test covers
//! - G2-adjacent named alt-digivolve source (Lv.5 w/[RizeGreymon] in name,
//!   cost 3)
//! - D1/D-formula: mandatory single-target DP debuff scaled by
//!   `card_count_in_zone` filtered to (own, battle_area, Tamer, yellow/red)
//!   (AD1-016 / BT25-018 idiom)
//! - ST1-14 idiom: `add_player_modifier` w/ `ChangeOwnSecurityDigimonDp`,
//!   `target_player: opponent` (flipped from ST1-14's `you`), scaled formula
//!   value, `expiry: end_of_turn`
//! - D4/BT5-093 idiom: two name-filtered declarative auras (+3000 DP,
//!   <Security A. +1>) gated on `active_when: { your_turn: true }`
//! - E-negative: gate fails with zero yellow/red Tamers on the field (no
//!   selection installs, no security debuff)
//! - Negative: opponent Digimon select does not install when the opponent has
//!   no battle-area Digimon, but the security debuff STILL fires
//! - Negative: blue/green/etc Tamers do not count toward the multiplier or
//!   satisfy the gate

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword, ModifierType, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT12-043";

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn shinegreymon_runner() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-043 YAML in embedded pack")
}

fn compiled() -> digimon_dsl::compiled::CompiledCard {
    digimon_engine::dsl_registry::from_embedded()
        .expect("embedded DSL registry loads")
        .lookup(CARD_ID)
        .unwrap_or_else(|| panic!("{CARD_ID} not found in embedded DSL pack"))
        .clone()
}

fn make_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card
}

fn make_tamer(id: &str, name: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.colors = vec![color];
    card
}

fn make_marcus_damon(id: &str) -> CardData {
    let mut card = make_test_card(id, "Marcus Damon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(5000);
    card.colors = vec![CardColor::Yellow];
    card
}

fn push_security(runner: &mut DebugRunner, player: PlayerId, card_id: &str, count: usize) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card {card_id}"));
    for _ in 0..count {
        let next = runner.game.next_card_index();
        runner.game.players[player as usize]
            .security
            .push(CardSource::new(data_idx, player, next));
    }
}

fn fire_when_digivolving(runner: &mut DebugRunner, perm: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();
}

/// Resolve any pending selections, always picking the first non-PASS action.
fn resolve_all(runner: &mut DebugRunner) {
    while let Some(view) = runner.pending_selection_view() {
        let accept = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != PASS)
            .unwrap_or(view.valid_action_ids[0]);
        runner
            .execute_action(view.selecting_player, accept)
            .expect("resolve pending selection");
    }
    let _ = runner.auto_resolve();
}

// ═══════════════════════════════════════════════════════════════════════════
// SECTION 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt12_043_has_printed_stats_and_traits() {
    let card = compiled();

    assert_eq!(card.card, "BT12-043");
    assert_eq!(card.name, "ShineGreymon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.dp, Some(12000));
    assert_eq!(card.cost, Some(12));
    assert!(
        card.color.contains(&digimon_dsl::compiled::CompiledColor::Yellow)
            && card.color.contains(&digimon_dsl::compiled::CompiledColor::Red),
        "BT12-043 must be Yellow/Red"
    );
    assert!(
        card.traits.iter().any(|t| t == "Light Dragon"),
        "missing trait Light Dragon"
    );
}

#[test]
fn bt12_043_declares_alt_digivolve_from_lv5_rizegreymon_cost3() {
    let card = compiled();
    let has_alt = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(3))
    });
    assert!(
        has_alt,
        "BT12-043 must declare the cost-3 alt digivolve path from a Lv.5 \
         [RizeGreymon]-named source; alt_paths = {:?}",
        card.alt_paths
    );
}

#[test]
fn bt12_043_has_when_digivolving_clause_with_tamer_gate() {
    let card = compiled();
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let wd = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::WhenDigivolving))
        .expect("a [When Digivolving] clause must exist");

    assert_eq!(
        wd.scope,
        CompiledScope::FaceUp,
        "[When Digivolving] is an own-scope (FaceUp) clause"
    );
    assert!(
        !wd.optional,
        "the DP debuff is mandatory once the Tamer gate holds (no \"you may\" in printed text)"
    );
    assert!(
        wd.condition.is_some(),
        "clause must declare the yellow/red Tamer-count gate as a condition"
    );
}

#[test]
fn bt12_043_has_two_your_turn_marcus_damon_auras() {
    let card = compiled();
    let auras: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                active_when,
                target,
                dp_modifier,
                grant_keyword,
                ..
            }) => Some((active_when, target, dp_modifier, grant_keyword)),
            _ => None,
        })
        .collect();

    assert_eq!(
        auras.len(),
        2,
        "BT12-043 must author two separate [Your Turn] Marcus Damon auras \
         (+3000 DP and <Security A. +1>), mirroring DCGO's two static \
         effects; got {:?}",
        auras
    );

    let dp_aura = auras
        .iter()
        .find(|(_, _, dp, _)| dp.is_some())
        .expect("a +3000 DP aura must exist");
    assert_eq!(dp_aura.2, &Some(3000), "DP aura must grant +3000");
    assert_ne!(
        *dp_aura.1,
        digimon_dsl::compiled::CompiledPredicate::default(),
        "DP aura target must be filtered (name_contains Marcus Damon)"
    );
    assert!(
        dp_aura.0.is_some(),
        "DP aura must gate on active_when: your_turn"
    );

    let kw_aura = auras
        .iter()
        .find(|(_, _, _, kw)| kw.is_some())
        .expect("a Security A. +1 aura must exist");
    let gk = kw_aura.3.as_ref().unwrap();
    assert_eq!(gk.keyword, "SecurityAttackPlus");
    assert_eq!(gk.value, Some(1));
    assert!(
        kw_aura.0.is_some(),
        "Security A. +1 aura must gate on active_when: your_turn"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// SECTION 2 — [When Digivolving] condition gating (positive + negative)
// ═══════════════════════════════════════════════════════════════════════════

/// Negative: zero yellow/red Tamers on the field → the whole clause is a
/// no-op (no selection installs, no security debuff).
#[test]
fn bt12_043_no_tamer_gate_fails_no_selection_no_security_debuff() {
    let mut runner = shinegreymon_runner()
        .add_card(make_digimon("OPP", 5, 8000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    fire_when_digivolving(&mut runner, perm);

    assert!(
        runner.pending_selection().is_none(),
        "no yellow/red Tamer → gate fails → no DP-debuff selection installs"
    );
    assert_eq!(
        runner.dp_of(opp),
        Some(8000),
        "opponent Digimon DP must be unchanged"
    );
    assert_eq!(
        runner
            .modifiers()
            .player_modifier_value(1, ModifierType::ChangeOwnSecurityDigimonDp),
        0,
        "no ChangeOwnSecurityDigimonDp modifier should be installed on the opponent"
    );
}

/// Negative: a non-yellow/red (e.g. Blue) Tamer does not satisfy the gate.
#[test]
fn bt12_043_blue_tamer_does_not_satisfy_gate() {
    let mut runner = shinegreymon_runner()
        .add_card(make_tamer("BLUE-TAMER", "BlueTamer", CardColor::Blue))
        .add_card(make_digimon("OPP", 5, 8000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "BLUE-TAMER", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    fire_when_digivolving(&mut runner, perm);

    assert!(
        runner.pending_selection().is_none(),
        "a Blue Tamer must not satisfy the yellow/red gate"
    );
    assert_eq!(runner.dp_of(opp), Some(8000));
}

/// Positive: one yellow Tamer satisfies the gate and installs the mandatory
/// opponent-Digimon selection.
#[test]
fn bt12_043_yellow_tamer_satisfies_gate_installs_selection() {
    let mut runner = shinegreymon_runner()
        .add_card(make_tamer("Y-TAMER", "YellowTamer", CardColor::Yellow))
        .add_card(make_digimon("OPP", 5, 8000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "Y-TAMER", Some(0));
    runner.place_on_field(1, "OPP", Some(0));

    fire_when_digivolving(&mut runner, perm);

    let view = runner
        .pending_selection_view()
        .expect("a yellow Tamer satisfies the gate → mandatory opp-Digimon selection installs");
    assert!(
        !view.valid_action_ids.is_empty(),
        "opponent Digimon must be selectable"
    );
    assert!(
        !runner.pending_is_optional(),
        "the debuff select is mandatory (DCGO canNoSelect: false)"
    );
}

/// Positive: one red Tamer also satisfies the gate.
#[test]
fn bt12_043_red_tamer_satisfies_gate_installs_selection() {
    let mut runner = shinegreymon_runner()
        .add_card(make_tamer("R-TAMER", "RedTamer", CardColor::Red))
        .add_card(make_digimon("OPP", 5, 8000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "R-TAMER", Some(0));
    runner.place_on_field(1, "OPP", Some(0));

    fire_when_digivolving(&mut runner, perm);

    assert!(
        runner.pending_selection().is_some(),
        "a Red Tamer satisfies the gate too"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// SECTION 3 — [When Digivolving] behavioral outcome: DP debuff scaling
// ═══════════════════════════════════════════════════════════════════════════

/// One yellow/red Tamer → -3000 DP to the chosen opponent Digimon.
#[test]
fn bt12_043_one_tamer_applies_minus_3000_to_chosen_opponent_digimon() {
    let mut runner = shinegreymon_runner()
        .add_card(make_tamer("Y-TAMER", "YellowTamer", CardColor::Yellow))
        .add_card(make_digimon("OPP", 6, 12000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "Y-TAMER", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    fire_when_digivolving(&mut runner, perm);
    resolve_all(&mut runner);

    assert_eq!(
        runner.dp_of(opp),
        Some(9000),
        "1 yellow/red Tamer → -3000 DP (12000-3000)"
    );
}

/// Two yellow/red Tamers → -6000 DP to the chosen opponent Digimon.
#[test]
fn bt12_043_two_tamers_scale_debuff_to_minus_6000() {
    let mut runner = shinegreymon_runner()
        .add_card(make_tamer("Y-TAMER", "YellowTamer", CardColor::Yellow))
        .add_card(make_tamer("R-TAMER", "RedTamer", CardColor::Red))
        .add_card(make_digimon("OPP", 6, 12000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "Y-TAMER", Some(0));
    runner.place_on_field(0, "R-TAMER", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    fire_when_digivolving(&mut runner, perm);
    resolve_all(&mut runner);

    assert_eq!(
        runner.dp_of(opp),
        Some(6000),
        "2 yellow/red Tamers → -6000 DP (12000-6000)"
    );
}

/// A Blue Tamer present alongside a Yellow Tamer must NOT add to the
/// multiplier — only yellow/red Tamers count.
#[test]
fn bt12_043_non_matching_tamer_does_not_add_to_multiplier() {
    let mut runner = shinegreymon_runner()
        .add_card(make_tamer("Y-TAMER", "YellowTamer", CardColor::Yellow))
        .add_card(make_tamer("BLUE-TAMER", "BlueTamer", CardColor::Blue))
        .add_card(make_digimon("OPP", 6, 12000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "Y-TAMER", Some(0));
    runner.place_on_field(0, "BLUE-TAMER", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    fire_when_digivolving(&mut runner, perm);
    resolve_all(&mut runner);

    assert_eq!(
        runner.dp_of(opp),
        Some(9000),
        "only the 1 yellow Tamer counts → -3000, not -6000"
    );
}

/// Negative: when the opponent has NO battle-area Digimon, the mandatory
/// select installs nothing (no candidate) — the clause is not blocked.
#[test]
fn bt12_043_no_opponent_digimon_no_selection_installs() {
    let mut runner = shinegreymon_runner()
        .add_card(make_tamer("Y-TAMER", "YellowTamer", CardColor::Yellow))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "Y-TAMER", Some(0));

    fire_when_digivolving(&mut runner, perm);

    assert!(
        runner.pending_selection().is_none(),
        "no opponent Digimon exists → the mandatory select has no candidate and installs nothing"
    );
}

/// The DP debuff expires at end of turn (DCGO `UntilEachTurnEnd`).
#[test]
fn bt12_043_dp_debuff_expires_at_end_of_turn() {
    let mut runner = shinegreymon_runner()
        .add_card(make_tamer("Y-TAMER", "YellowTamer", CardColor::Yellow))
        .add_card(make_digimon("OPP", 6, 12000))
        .add_card(make_digimon("FILLER", 3, 3000))
        .deck(0, &["FILLER", "FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER", "FILLER"])
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "Y-TAMER", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    fire_when_digivolving(&mut runner, perm);
    resolve_all(&mut runner);
    assert_eq!(runner.dp_of(opp), Some(9000), "debuff applied (12000-3000)");

    runner.end_turn();
    assert_eq!(
        runner.dp_of(opp),
        Some(12000),
        "debuff must clear once ShineGreymon's controller's turn ends"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// SECTION 4 — [When Digivolving] behavioral outcome: opponent Security debuff
// ═══════════════════════════════════════════════════════════════════════════

/// One yellow/red Tamer → opponent's security Digimon get -3000 DP,
/// installed as a `ChangeOwnSecurityDigimonDp` player modifier on the
/// opponent (ST1-14 idiom, `target_player: opponent`).
#[test]
fn bt12_043_one_tamer_debuffs_opponent_security_by_3000() {
    let mut runner = shinegreymon_runner()
        .add_card(make_tamer("Y-TAMER", "YellowTamer", CardColor::Yellow))
        .add_card(make_digimon("SEC", 4, 5000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "Y-TAMER", Some(0));
    push_security(&mut runner, 1, "SEC", 3);

    fire_when_digivolving(&mut runner, perm);
    resolve_all(&mut runner);

    let sec_mod = runner
        .modifiers()
        .player_modifier_value(1, ModifierType::ChangeOwnSecurityDigimonDp);
    assert_eq!(
        sec_mod, -3000,
        "opponent (P1) must carry a -3000 ChangeOwnSecurityDigimonDp modifier"
    );
    assert_eq!(
        runner.game.defender_security_dp_adjustment(1),
        -3000,
        "P1's defender security DP adjustment must reflect the -3000 debuff"
    );
}

/// Two yellow/red Tamers → opponent security Digimon get -6000 DP (same
/// per-Tamer scaling as the single-Digimon debuff).
#[test]
fn bt12_043_two_tamers_debuff_opponent_security_by_6000() {
    let mut runner = shinegreymon_runner()
        .add_card(make_tamer("Y-TAMER", "YellowTamer", CardColor::Yellow))
        .add_card(make_tamer("R-TAMER", "RedTamer", CardColor::Red))
        .add_card(make_digimon("SEC", 4, 5000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "Y-TAMER", Some(0));
    runner.place_on_field(0, "R-TAMER", Some(0));
    push_security(&mut runner, 1, "SEC", 3);

    fire_when_digivolving(&mut runner, perm);
    resolve_all(&mut runner);

    assert_eq!(
        runner.game.defender_security_dp_adjustment(1),
        -6000,
        "2 yellow/red Tamers → -6000 to opponent security Digimon"
    );
}

/// Critical faithfulness case (official Q&A): the security debuff fires
/// EVEN WHEN there is no opponent battle-area Digimon to select — the two
/// halves of the clause are independent, not sequentially dependent.
#[test]
fn bt12_043_security_debuff_fires_even_without_opponent_battle_area_digimon() {
    let mut runner = shinegreymon_runner()
        .add_card(make_tamer("Y-TAMER", "YellowTamer", CardColor::Yellow))
        .add_card(make_digimon("SEC", 4, 5000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "Y-TAMER", Some(0));
    push_security(&mut runner, 1, "SEC", 3);
    // No opponent battle-area Digimon at all.

    fire_when_digivolving(&mut runner, perm);
    resolve_all(&mut runner);

    assert_eq!(
        runner.game.defender_security_dp_adjustment(1),
        -3000,
        "the opponent-security debuff must apply even when there is no \
         opponent Digimon to select for the other half of the clause"
    );
}

/// Own security (the controller's own security stack) must be unaffected —
/// the debuff is scoped to the OPPONENT's security only.
#[test]
fn bt12_043_own_security_unaffected() {
    let mut runner = shinegreymon_runner()
        .add_card(make_tamer("Y-TAMER", "YellowTamer", CardColor::Yellow))
        .add_card(make_digimon("SEC", 4, 5000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "Y-TAMER", Some(0));
    push_security(&mut runner, 0, "SEC", 3);

    fire_when_digivolving(&mut runner, perm);
    resolve_all(&mut runner);

    assert_eq!(
        runner
            .modifiers()
            .player_modifier_value(0, ModifierType::ChangeOwnSecurityDigimonDp),
        0,
        "the controller's OWN security must not receive the debuff"
    );
}

/// The security debuff expires at end of turn.
#[test]
fn bt12_043_security_debuff_expires_at_end_of_turn() {
    let mut runner = shinegreymon_runner()
        .add_card(make_tamer("Y-TAMER", "YellowTamer", CardColor::Yellow))
        .add_card(make_digimon("SEC", 4, 5000))
        .add_card(make_digimon("FILLER", 3, 3000))
        .deck(0, &["FILLER", "FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER", "FILLER"])
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "Y-TAMER", Some(0));
    push_security(&mut runner, 1, "SEC", 3);

    fire_when_digivolving(&mut runner, perm);
    resolve_all(&mut runner);
    assert_eq!(runner.game.defender_security_dp_adjustment(1), -3000);

    runner.end_turn();
    assert_eq!(
        runner.game.defender_security_dp_adjustment(1),
        0,
        "security debuff must clear once ShineGreymon's controller's turn ends"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// SECTION 5 — [Your Turn] Marcus Damon auras: +3000 DP
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt12_043_marcus_damon_gains_3000_dp_on_your_turn() {
    let mut runner = shinegreymon_runner()
        .add_card(make_marcus_damon("MD"))
        .memory(20)
        .start();
    let _shine = runner.place_on_field(0, CARD_ID, Some(0));
    let marcus = runner.place_on_field(0, "MD", Some(0));

    runner.game.tick_declarative_effects();

    assert_eq!(
        runner.dp_of(marcus),
        Some(8000),
        "own Marcus Damon must be +3000 DP on your turn (5000 base + 3000)"
    );
}

/// Negative (name filter): a non-Marcus-Damon own Digimon must not receive
/// the +3000 DP bonus.
#[test]
fn bt12_043_non_marcus_damon_own_digimon_unaffected_by_dp_aura() {
    let mut runner = shinegreymon_runner()
        .add_card(make_digimon("ALLY", 4, 5000))
        .memory(20)
        .start();
    let _shine = runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_on_field(0, "ALLY", Some(0));

    runner.game.tick_declarative_effects();

    assert_eq!(
        runner.dp_of(ally),
        Some(5000),
        "non-Marcus-Damon own Digimon must be unaffected by the aura"
    );
}

/// Negative (controller filter): the opponent's Marcus Damon must not
/// receive the +3000 DP bonus from our aura.
#[test]
fn bt12_043_opponents_marcus_damon_unaffected_by_dp_aura() {
    let mut runner = shinegreymon_runner()
        .add_card(make_marcus_damon("OPP-MD"))
        .memory(20)
        .start();
    let _shine = runner.place_on_field(0, CARD_ID, Some(0));
    let opp_marcus = runner.place_on_field(1, "OPP-MD", Some(0));

    runner.game.tick_declarative_effects();

    assert_eq!(
        runner.dp_of(opp_marcus),
        Some(5000),
        "opponent's Marcus Damon must NOT receive the +3000 DP from our aura"
    );
}

/// Negative (turn gate): on the opponent's turn, the DP bonus must clear.
#[test]
fn bt12_043_marcus_damon_dp_aura_clears_on_opponents_turn() {
    let mut runner = shinegreymon_runner()
        .add_card(make_marcus_damon("MD"))
        .memory(20)
        .start();
    let _shine = runner.place_on_field(0, CARD_ID, Some(0));
    let marcus = runner.place_on_field(0, "MD", Some(0));

    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.dp_of(marcus),
        Some(8000),
        "sanity: on your turn the DP bonus is applied"
    );

    runner.end_turn();
    runner.game.tick_declarative_effects();

    assert_eq!(
        runner.dp_of(marcus),
        Some(5000),
        "on opponent's turn the active_when:{{your_turn:true}} gate must clear the DP bonus"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// SECTION 6 — [Your Turn] Marcus Damon auras: <Security A. +1>
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt12_043_marcus_damon_gains_security_attack_plus_one_on_your_turn() {
    let mut runner = shinegreymon_runner()
        .add_card(make_marcus_damon("MD"))
        .memory(20)
        .start();
    let _shine = runner.place_on_field(0, CARD_ID, Some(0));
    let marcus = runner.place_on_field(0, "MD", Some(0));

    runner.game.tick_declarative_effects();

    assert!(
        runner
            .game
            .modifiers
            .has_keyword(marcus, Keyword::SecurityAttackPlus(1)),
        "own Marcus Damon must have SecurityAttackPlus(1) installed by the aura tick"
    );
}

/// Negative (name filter): a non-Marcus-Damon own Digimon must not receive
/// the keyword.
#[test]
fn bt12_043_non_marcus_damon_own_digimon_unaffected_by_keyword_aura() {
    let mut runner = shinegreymon_runner()
        .add_card(make_digimon("ALLY", 4, 5000))
        .memory(20)
        .start();
    let _shine = runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_on_field(0, "ALLY", Some(0));

    runner.game.tick_declarative_effects();

    assert!(
        !runner
            .game
            .modifiers
            .has_keyword(ally, Keyword::SecurityAttackPlus(1)),
        "non-Marcus-Damon own Digimon must NOT receive SecurityAttackPlus"
    );
}

/// Negative (turn gate): on opponent's turn, own Marcus Damon loses the
/// keyword.
#[test]
fn bt12_043_marcus_damon_keyword_aura_clears_on_opponents_turn() {
    let mut runner = shinegreymon_runner()
        .add_card(make_marcus_damon("MD"))
        .memory(20)
        .start();
    let _shine = runner.place_on_field(0, CARD_ID, Some(0));
    let marcus = runner.place_on_field(0, "MD", Some(0));

    runner.game.tick_declarative_effects();
    assert!(runner
        .game
        .modifiers
        .has_keyword(marcus, Keyword::SecurityAttackPlus(1)));

    runner.end_turn();
    runner.game.tick_declarative_effects();

    assert!(
        !runner
            .game
            .modifiers
            .has_keyword(marcus, Keyword::SecurityAttackPlus(1)),
        "on opponent's turn the active_when:{{your_turn:true}} gate must clear the keyword"
    );
}

/// End-to-end: a buffed Marcus Damon attacking the opponent player pops 2
/// security cards (base 1 + aura's SecurityAttackPlus 1).
#[test]
fn bt12_043_buffed_marcus_damon_pops_extra_security_card() {
    let mut runner = shinegreymon_runner()
        .add_card(make_marcus_damon("MD"))
        .add_card(make_digimon("SEC", 1, 0))
        .memory(20)
        .start();
    push_security(&mut runner, 1, "SEC", 3);

    let _shine = runner.place_on_field(0, CARD_ID, Some(0));
    let marcus = runner.place_on_field(0, "MD", Some(0));

    runner.game.tick_declarative_effects();

    let sec_before = runner.game.players[1].security.len();
    let _ = runner.attack_player(marcus, 1, false);
    let sec_after = runner.game.players[1].security.len();

    assert_eq!(
        sec_before - sec_after,
        2,
        "aura-buffed Marcus Damon must pop base 1 + aura 1 = 2 security cards; \
         before={sec_before}, after={sec_after}"
    );
}
