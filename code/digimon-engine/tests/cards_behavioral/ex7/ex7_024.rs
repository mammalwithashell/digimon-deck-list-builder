//! EX7-024 Shoemon — Digimon, Lv.3, Yellow, DP 1000, Cost 3.
//! Traits: Puppet, LIBERATOR. Attribute: Virus.
//! Evo cost: from yellow Lv.2 for 0 memory.
//!
//! # Card text (data/cards.json — verbatim)
//!
//! **Effect:**
//! [Your Turn] When this Digimon would digivolve into a Digimon card with the
//! [Puppet] trait, reduce the digivolution cost by 1.
//!
//! **Inherited:**
//! [Your Turn] All of your opponent's Security Digimon get -3000 DP.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX7/Yellow/EX7_024.cs
//!   - "Your Turn": ChangeDigivolutionCostStaticEffect(changeValue: -1,
//!     permanentCondition: targetPermanent == card.PermanentOfThisCard(),
//!     cardCondition: cardSource.IsDigimon && cardSource.ContainsTraits("Puppet"),
//!     condition: IsExistOnBattleAreaDigimon(card) && IsOwnerTurn(card)).
//!   - "Your Turn - ESS" (inherited): ChangeSecurityDigimonCardDPStaticEffect(
//!     cardCondition: cardSource.Owner == card.Owner.Enemy, changeValue: -3000,
//!     isInheritedEffect: true, condition: IsExistOnBattleAreaDigimon(card)
//!     && IsOwnerTurn(card)).
//!
//! # Patterns this test covers
//! - D2 — BeforePayCost cost reduction (clause 1; source-scoped digivolve-into-trait)
//! - D4 — declarative inherited aura (clause 2; opponent-security-DP adjustment)
//! - G-BEFORE-PAY-COST-DIGIVOLVE-TARGET — `cost_target` + `source_is_cost_target_permanent`
//! - G-OPPONENT-SECURITY-DP-AURA / PUPPETS-G008 — `applies_to_opponent_security_dp`
//!
//! # Gap-closure status
//!
//! Both printed effects were previously deferred. Both blocking gaps are now
//! CLOSED:
//! - Clause 1: `G-BEFORE-PAY-COST-DIGIVOLVE-TARGET` RESOLVED 2026-05-17
//!   (Phase 2 Track H). The DSL exposes `cost_target: { ... }` (evaluated
//!   against the digivolve-target card) and `source_is_cost_target_permanent`
//!   (gates to "THIS permanent is being digivolved into"). Proof-of-substrate:
//!   `code/digimon-engine/cards/p/P-117.yaml`.
//! - Clause 2: `G-OPPONENT-SECURITY-DP-AURA` / `PUPPETS-G008` RESOLVED
//!   2026-05-17 (Phase 2 Track I). The DSL `aura` clause carries
//!   `applies_to_opponent_security_dp: true`, lowered to
//!   `EffectBuilder::applies_to_opponent_security_dp()`.
//!
//! Note on `[Your Turn]` gating: a Digimon can only digivolve and only attack
//! on its controller's own turn, so the printed `[Your Turn]` qualifier on
//! both clauses is never reachable through an opponent-turn path in standard
//! rules. The combat path `Game::attacker_security_dp_adjustment` and the
//! digivolve cost-calc both run exclusively during the controller's turn.
//! The `[Your Turn]` qualifier is therefore asserted structurally (the
//! `active_when` predicate is encoded on each clause) rather than via an
//! impossible opponent-turn behavioral negative.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledDeclarativeClause,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};

// ─── Fixture builders ─────────────────────────────────────────────────────────

/// A Lv.4 yellow **Puppet** Digimon that digivolves from a Lv.3 yellow for a
/// printed 2-memory cost. Used as the digivolve target for clause 1's positive
/// branch: EX7-024's cost reduction must drop 2 → 1.
fn puppet_lv4(id: &str) -> CardData {
    let mut c = make_test_card(id, "PuppetLv4");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Yellow];
    c.traits = vec!["Puppet".to_string(), "LIBERATOR".to_string()];
    c.evo_costs = vec![EvoCost {
        level: 3,
        card_color: 2, // Yellow (mirrors action::mask::evo_color)
        memory_cost: 2,
    }];
    c
}

/// A Lv.4 yellow Digimon WITHOUT the [Puppet] trait, otherwise identical to
/// `puppet_lv4`. Used as the negative branch for clause 1: the cost reduction
/// must NOT fire because the digivolve target lacks [Puppet].
fn non_puppet_lv4(id: &str) -> CardData {
    let mut c = make_test_card(id, "NonPuppetLv4");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Yellow];
    c.traits = vec!["Beast".to_string()];
    c.evo_costs = vec![EvoCost {
        level: 3,
        card_color: 2, // Yellow
        memory_cost: 2,
    }];
    c
}

/// A generic Lv.3 yellow Digimon with no effects, able to digivolve into a
/// Lv.4 Puppet. Used as an *unrelated* digivolve base for clause 1's
/// source-scope negative branch: EX7-024's cost reduction is gated by
/// `source_is_cost_target_permanent`, so digivolving this permanent (which is
/// not EX7-024) must NOT trigger the reduction.
fn other_lv3(id: &str) -> CardData {
    let mut c = make_test_card(id, "Other Lv3");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.colors = vec![CardColor::Yellow];
    c.evo_costs = vec![EvoCost {
        level: 2,
        card_color: 2, // Yellow
        memory_cost: 0,
    }];
    c
}

/// A high-DP attacker for player 0 that always wins the security DP battle so
/// the security pipeline runs to completion.
fn strong_attacker(id: &str) -> CardData {
    let mut c = make_test_card(id, "Strong Attacker");
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(8000);
    c.play_cost = 8;
    c.colors = vec![CardColor::Red];
    c
}

/// A Digimon placed into a player's security stack with a DP just high enough
/// that, *without* the -3000 aura, it would survive the attacker — and *with*
/// the aura it loses. The security battle is the observable test point.
fn security_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, "Security Digimon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(dp);
    c.play_cost = 6;
    c.colors = vec![CardColor::Blue];
    c
}

fn filler(id: &str) -> CardData {
    make_test_card(id, "Filler")
}

/// Push a registered card directly onto a battle-area permanent's
/// `card_sources` (as a digivolution source under the top card).
fn insert_source_under(runner: &mut DebugRunner, perm: digimon_engine::permanent::PermanentHandle, card_id: &str) {
    let game = runner.game_mut();
    let data_idx = game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("insert_source_under: card {card_id} not registered"));
    let next = game.next_card_index();
    let mut src = CardSource::new(data_idx, perm.player, next);
    src.card_index = next;
    let p = &mut game.players[perm.player as usize].battle_area[perm.index as usize];
    p.card_sources.insert(0, src);
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

/// EX7-024 compiles with its printed stats and the yellow Lv.2 cost-0
/// digivolution path.
#[test]
fn ex7_024_compiles_with_printed_stats_and_lv2_yellow_path() {
    let runner = DebugRunner::builder()
        .dsl_card("EX7-024")
        .expect("EX7-024 found in embedded DSL pack")
        .start();

    let compiled = runner
        .compiled_card("EX7-024")
        .expect("EX7-024 in compiled cards");

    assert_eq!(compiled.card, "EX7-024");
    assert_eq!(compiled.name, "Shoemon");
    assert_eq!(compiled.kind, CompiledCardKind::Digimon);
    assert_eq!(compiled.level, Some(3));
    assert_eq!(compiled.color, vec![CompiledColor::Yellow]);
    assert_eq!(compiled.cost, Some(3));
    assert_eq!(compiled.dp, Some(1000));
    for trait_name in ["Puppet", "LIBERATOR"] {
        assert!(
            compiled.traits.iter().any(|t| t == trait_name),
            "missing trait {trait_name}"
        );
    }

    assert!(
        compiled.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(0))
                && path.from.as_ref().is_some_and(|from| {
                    (from.level_eq == Some(2)
                        || from.all_of.iter().any(|pred| pred.level_eq == Some(2)))
                        && (from.color_is == Some(CompiledColor::Yellow)
                            || from
                                .all_of
                                .iter()
                                .any(|pred| pred.color_is == Some(CompiledColor::Yellow)))
                })
        }),
        "Shoemon must digivolve from a yellow Lv2 for cost 0"
    );
}

/// EX7-024 encodes exactly two effect clauses: clause 1 the source-scoped
/// digivolve-into-Puppet cost reduction, clause 2 the inherited
/// opponent-security DP aura.
#[test]
fn ex7_024_has_cost_reduction_and_inherited_aura_clauses() {
    let runner = DebugRunner::builder()
        .dsl_card("EX7-024")
        .expect("EX7-024 found in embedded DSL pack")
        .start();
    let compiled = runner
        .compiled_card("EX7-024")
        .expect("EX7-024 in compiled cards");

    assert_eq!(
        compiled.effects.len(),
        2,
        "EX7-024 has exactly 2 clauses (cost reduction + inherited aura); got {}",
        compiled.effects.len()
    );

    let has_cost_reduction = compiled.effects.iter().any(|clause| {
        matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
        )
    });
    assert!(
        has_cost_reduction,
        "EX7-024 must encode the [Your Turn] digivolve-into-Puppet cost reduction"
    );

    let has_security_aura = compiled.effects.iter().any(|clause| match clause {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            applies_to_opponent_security_dp,
            ..
        }) => *applies_to_opponent_security_dp,
        _ => false,
    });
    assert!(
        has_security_aura,
        "EX7-024 must encode the inherited opponent-security-DP aura"
    );
}

/// Clause 1: the cost-reduction clause is NOT [Once Per Turn] — the printed
/// text has no such qualifier (cf. P-117 which IS OPT). It reduces by 1, has
/// `before_pay_cost` timing, and carries a non-empty `active_when` gate (the
/// `[Your Turn]` qualifier).
#[test]
fn ex7_024_cost_reduction_clause_structural_shape() {
    let runner = DebugRunner::builder()
        .dsl_card("EX7-024")
        .expect("EX7-024 found in embedded DSL pack")
        .start();
    let compiled = runner
        .compiled_card("EX7-024")
        .expect("EX7-024 in compiled cards");

    let (reduction_timing, active_when, once_per_turn, amount) = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
                reduction_timing,
                active_when,
                once_per_turn,
                amount,
                ..
            }) => Some((
                reduction_timing.clone(),
                active_when.clone(),
                *once_per_turn,
                *amount,
            )),
            _ => None,
        })
        .expect("EX7-024 must encode a CostReduction clause");

    assert_eq!(
        reduction_timing.as_deref(),
        Some("before_pay_cost"),
        "cost reduction must fire at BeforePayCost cost-calc"
    );
    assert_eq!(amount, Some(1), "printed text reduces the cost by exactly 1");
    assert!(
        !once_per_turn,
        "printed text carries no [Once Per Turn] — reduction fires for every \
         qualifying digivolution (DCGO ChangeDigivolutionCostStaticEffect has no max-count)"
    );
    assert!(
        active_when.is_some(),
        "the [Your Turn] qualifier must be encoded as an active_when gate"
    );
}

/// Clause 2: the inherited aura is `Inherited`-scoped, applies a -3000 DP
/// adjustment to the opposing security Digimon, and carries the `[Your Turn]`
/// active_when gate.
#[test]
fn ex7_024_inherited_aura_clause_structural_shape() {
    let runner = DebugRunner::builder()
        .dsl_card("EX7-024")
        .expect("EX7-024 found in embedded DSL pack")
        .start();
    let compiled = runner
        .compiled_card("EX7-024")
        .expect("EX7-024 in compiled cards");

    let (scope, dp_modifier, active_when) = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope,
                dp_modifier,
                active_when,
                applies_to_opponent_security_dp,
                ..
            }) if *applies_to_opponent_security_dp => {
                Some((*scope, *dp_modifier, active_when.clone()))
            }
            _ => None,
        })
        .expect("EX7-024 must encode an opponent-security-DP aura");

    assert_eq!(
        scope,
        digimon_dsl::compiled::CompiledScope::Inherited,
        "the opponent-security DP aura is an inherited effect"
    );
    assert_eq!(
        dp_modifier,
        Some(-3000),
        "the aura lowers opponent Security Digimon DP by 3000"
    );
    assert!(
        active_when.is_some(),
        "the [Your Turn] qualifier must be encoded as an active_when gate"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — Clause 1: cost-reduction behavior (digivolve-into-Puppet)
// ─────────────────────────────────────────────────────────────────────────────

/// POSITIVE: EX7-024 on field digivolves into a Lv.4 [Puppet] Digimon. The
/// printed digivolution cost is 2; EX7-024's [Your Turn] effect reduces it by
/// 1, so memory drops by only 1.
#[test]
fn ex7_024_cost_reduction_reduces_digivolving_into_puppet_by_one() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-024")
        .expect("EX7-024 found in embedded DSL pack")
        .add_card(puppet_lv4("PUPPET-LV4"))
        .add_card(filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(8)
        .start();
    runner.game.turn_count = 1;

    // EX7-024 on the field as the digivolve source.
    let shoemon = runner.place_on_field(0, "EX7-024", Some(0));
    // PUPPET-LV4 in player 0's hand.
    {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "PUPPET-LV4")
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, card_index));
    }
    let hand_idx = runner.game.player(0).hand.len() - 1;

    let memory_before = runner.game.memory;
    let digivolved = runner.game.digivolve_from_hand(
        0,
        hand_idx,
        shoemon.index as usize,
        PlaySource::ByHand,
    );
    assert!(
        digivolved,
        "EX7-024 must digivolve into the Lv.4 Puppet (effective cost 2 - 1 = 1)"
    );

    // Printed cost 2, reduced by 1 → 1. Memory drops by exactly 1.
    assert_eq!(
        runner.game.memory,
        memory_before - 1,
        "digivolving into a [Puppet] Digimon must cost 1 memory (2 printed - 1 reduction)"
    );
}

/// NEGATIVE: EX7-024 digivolves into a Lv.4 Digimon that lacks the [Puppet]
/// trait. The cost-target-trait predicate fails, so the reduction must NOT
/// fire — memory drops by the full printed 2.
#[test]
fn ex7_024_cost_reduction_does_not_apply_to_non_puppet_target() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-024")
        .expect("EX7-024 found in embedded DSL pack")
        .add_card(non_puppet_lv4("NONPUPPET-LV4"))
        .add_card(filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(8)
        .start();
    runner.game.turn_count = 1;

    let shoemon = runner.place_on_field(0, "EX7-024", Some(0));
    {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "NONPUPPET-LV4")
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, card_index));
    }
    let hand_idx = runner.game.player(0).hand.len() - 1;

    let memory_before = runner.game.memory;
    let digivolved = runner.game.digivolve_from_hand(
        0,
        hand_idx,
        shoemon.index as usize,
        PlaySource::ByHand,
    );
    assert!(
        digivolved,
        "EX7-024 must still be able to digivolve into the non-Puppet Lv.4 Digimon"
    );

    // Target lacks [Puppet] — reduction must NOT fire. Memory drops by the full 2.
    assert_eq!(
        runner.game.memory,
        memory_before - 2,
        "digivolving into a non-[Puppet] Digimon must cost the full 2 memory"
    );
}

/// NEGATIVE (source scope): EX7-024 sits on the field as a standalone
/// permanent, and a SEPARATE, unrelated Lv.3 yellow Digimon digivolves into a
/// Lv.4 [Puppet]. EX7-024's cost reduction is gated by
/// `source_is_cost_target_permanent` (DCGO
/// `permanentCondition: targetPermanent == card.PermanentOfThisCard()`) — it
/// applies ONLY when EX7-024's OWN permanent digivolves. Because the
/// digivolving permanent here is the unrelated Lv.3, the reduction must NOT
/// fire and memory drops by the full printed 2. This catches a regression that
/// drops the source-scope predicate (which would leak the reduction onto every
/// digivolution while a bare EX7-024 sits on the field).
#[test]
fn ex7_024_cost_reduction_does_not_apply_to_unrelated_permanent_digivolution() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-024")
        .expect("EX7-024 found in embedded DSL pack")
        .add_card(other_lv3("OTHER-LV3"))
        .add_card(puppet_lv4("PUPPET-LV4"))
        .add_card(filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(8)
        .start();
    runner.game.turn_count = 1;

    // EX7-024 on the field as a STANDALONE permanent — NOT the digivolve source.
    let _shoemon = runner.place_on_field(0, "EX7-024", Some(0));
    // A separate, unrelated Lv.3 yellow Digimon — this is what digivolves.
    let other = runner.place_on_field(0, "OTHER-LV3", Some(0));

    // PUPPET-LV4 into player 0's hand.
    {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "PUPPET-LV4")
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, card_index));
    }
    let hand_idx = runner.game.player(0).hand.len() - 1;

    let memory_before = runner.game.memory;
    let digivolved = runner.game.digivolve_from_hand(
        0,
        hand_idx,
        other.index as usize,
        PlaySource::ByHand,
    );
    assert!(
        digivolved,
        "the unrelated Lv.3 must digivolve into the Lv.4 Puppet"
    );

    // The digivolving permanent is NOT the EX7-024 carrier — the source-scoped
    // reduction must NOT fire. Memory drops by the full printed 2.
    assert_eq!(
        runner.game.memory,
        memory_before - 2,
        "EX7-024's cost reduction is scoped to its OWN permanent; an unrelated \
         permanent digivolving into a Puppet pays the full 2 memory"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — Clause 2: inherited opponent-security-DP aura behavior
// ─────────────────────────────────────────────────────────────────────────────

/// POSITIVE: an attacker carrying EX7-024 in its digivolution sources attacks
/// the opponent directly. EX7-024's inherited aura lowers the opposing
/// Security Digimon's DP by 3000, flipping the security battle so the attacker
/// survives a security Digimon it would otherwise lose to.
///
/// SEC's base DP is 9000 vs. an 8000-DP attacker — without the aura the
/// attacker would be deleted. With EX7-024's -3000 aura, SEC drops to 6000 and
/// the attacker wins.
#[test]
fn ex7_024_inherited_security_dp_aura_drops_opponent_security_digimon_by_3000() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-024")
        .expect("EX7-024 found in embedded DSL pack")
        .add_card(strong_attacker("ATK")) // 8000 DP
        .add_card(security_digimon("SEC", 9000))
        .security(1, &["SEC"])
        .start();

    let atk = runner.place_on_field(0, "ATK", Some(0));
    // EX7-024 sits underneath the attacker as a digivolution source — its
    // inherited effect contributes during the security DP battle.
    insert_source_under(&mut runner, atk, "EX7-024");

    // Sanity: without the aura, an 8000 attacker loses to a 9000 security
    // Digimon. The aura's -3000 adjustment is what flips the result.
    let result = runner.attack_player(atk, 1, false);

    assert_eq!(
        result,
        AttackResult::SecurityCheckSurvived,
        "8000-DP attacker must survive: EX7-024's -3000 aura drops SEC 9000 -> 6000"
    );
    assert_eq!(
        runner.battle_area_size(0),
        1,
        "attacker must remain on the field after winning the security battle"
    );
}

/// NEGATIVE: an attacker with NO EX7-024 in its digivolution sources gets no
/// security-DP adjustment, so the same 8000-DP attacker loses to the 9000-DP
/// security Digimon. This confirms the -3000 swing is sourced specifically
/// from EX7-024's inherited effect and is not an unconditional combat bonus.
#[test]
fn ex7_024_no_security_dp_adjustment_without_ex7_024_in_stack() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-024")
        .expect("EX7-024 found in embedded DSL pack")
        .add_card(strong_attacker("ATK")) // 8000 DP
        .add_card(security_digimon("SEC", 9000))
        .security(1, &["SEC"])
        .start();

    let atk = runner.place_on_field(0, "ATK", Some(0));
    // No EX7-024 source inserted — the attacker is a bare 8000-DP Digimon.

    let result = runner.attack_player(atk, 1, false);

    assert_eq!(
        result,
        AttackResult::AttackerDeletedBySecurity,
        "without EX7-024's aura, the 8000-DP attacker loses to the 9000-DP security Digimon"
    );
    assert_eq!(
        runner.battle_area_size(0),
        0,
        "the attacker must be deleted after losing the security battle"
    );
}
