//! EX8-019 Penguinmon — Digimon, Lv.3, Blue/Yellow, DP 1000, Cost 3.
//! Traits: Avian, LIBERATOR. Attribute: Vaccine. Rarity: C.
//! Standard digivolution: Lv.2 blue for 1 memory (printed evo box).
//! Alt digivolution (printed evo box): "[Digivolve] [Hiyarimon]: Cost 0".
//! Rule box (card image): "Rule: Trait: Has [Ice-Snow] Type." — the card has
//! the [Ice-Snow] trait everywhere, in addition to Avian/LIBERATOR.
//!
//! # Card text (card image EX8-019 — authoritative; cards.json agrees)
//!
//! **Effect:**
//! [Your Turn] When this Digimon would digivolve into a Digimon card with the
//! [Ice-Snow] trait, reduce the digivolution cost by 1.
//!
//! **Inherited:**
//! [When Attacking] Give 1 of your opponent's Digimon <Security A. -1> (This
//! Digimon checks 1 fewer security card.) until the end of their turn.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX8/Blue/EX8_019.cs
//!   - Alternate Digivolution Requirement (None):
//!     AddSelfDigivolutionRequirementStaticEffect(permanentCondition:
//!     TopCard.ContainsCardName("Hiyarimon"), digivolutionCost: 0,
//!     ignoreDigivolutionRequirement: false).
//!   - "Your Turn" (None): ChangeDigivolutionCostStaticEffect(changeValue: -1,
//!     permanentCondition: targetPermanent == card.PermanentOfThisCard(),
//!     cardCondition: cardSource.EqualsTraits("Ice-Snow"),
//!     condition: IsOwnerTurn(card) && IsExistOnBattleArea(card)).
//!   - "When Attacking - ESS" (OnAllyAttack, inherited): mandatory
//!     SelectPermanentEffect(maxCount: 1, canNoSelect: false) over opponent
//!     battle-area Digimon → ChangeDigimonSAttack(changeValue: -1,
//!     effectDuration: EffectDuration.UntilOpponentTurnEnd).
//!     CanActivateCondition gates on an opponent Digimon existing.
//!
//! # Patterns this test covers
//! - D2 — BeforePayCost digivolution cost reduction (source-scoped,
//!   digivolve-into-trait via `cost_target` + `source_is_cost_target_permanent`;
//!   same shape as EX7-024 Shoemon).
//! - Inherited [When Attacking] mandatory single-target selection →
//!   `SecurityAttackChange -1` with `end_of_opponents_turn` expiry
//!   (cf. P-134's on_play clause + Nyaromon's inherited WA pattern).
//!
//! Note on `[Your Turn]` gating: a Digimon can only digivolve during its
//! controller's own turn, so the printed `[Your Turn]` qualifier on clause 1
//! is never reachable through an opponent-turn path in standard rules (same
//! reasoning as EX7-024's tests). It is asserted structurally (the
//! `active_when` predicate is encoded on the clause) rather than via an
//! impossible opponent-turn behavioral negative.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, ModifierType, PlaySource};
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Fixture builders ─────────────────────────────────────────────────────────

/// A Lv.4 blue **Ice-Snow** Digimon that digivolves from a Lv.3 blue for a
/// printed 2-memory cost. Used as the digivolve target for clause 1's positive
/// branch: EX8-019's cost reduction must drop 2 → 1.
fn ice_snow_lv4(id: &str) -> CardData {
    let mut c = make_test_card(id, "IceSnowLv4");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["Ice-Snow".to_string(), "LIBERATOR".to_string()];
    c.evo_costs = vec![EvoCost {
        level: 3,
        card_color: 1, // Blue (mirrors action::mask::evo_color)
        memory_cost: 2,
    }];
    c
}

/// A Lv.4 blue Digimon WITHOUT the [Ice-Snow] trait, otherwise identical to
/// `ice_snow_lv4`. Negative branch for clause 1: the cost reduction must NOT
/// fire because the digivolve target lacks [Ice-Snow].
fn non_ice_snow_lv4(id: &str) -> CardData {
    let mut c = make_test_card(id, "NonIceSnowLv4");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["Beast".to_string()];
    c.evo_costs = vec![EvoCost {
        level: 3,
        card_color: 1, // Blue
        memory_cost: 2,
    }];
    c
}

/// A generic Lv.3 blue Digimon with no effects, able to digivolve into the
/// Lv.4 Ice-Snow target. Used as an *unrelated* digivolve base for clause 1's
/// source-scope negative branch: EX8-019's cost reduction is gated by
/// `source_is_cost_target_permanent`, so digivolving this permanent (which is
/// not EX8-019) must NOT trigger the reduction.
fn other_lv3(id: &str) -> CardData {
    let mut c = make_test_card(id, "Other Lv3");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.colors = vec![CardColor::Blue];
    c.evo_costs = vec![EvoCost {
        level: 2,
        card_color: 1, // Blue
        memory_cost: 0,
    }];
    c
}

/// A carrier Digimon for player 0 — EX8-019 sits underneath it as a
/// digivolution source so its inherited [When Attacking] effect fires.
fn carrier(id: &str) -> CardData {
    let mut c = make_test_card(id, "Carrier");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Blue];
    c
}

/// An opponent (player 1) attacker used to observe the Security A. -1
/// behaviorally: with the -1 modifier its base 1 security check drops to 0.
fn opp_attacker(id: &str) -> CardData {
    let mut c = make_test_card(id, "Opp Attacker");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(6000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Red];
    c
}

/// A weak Digimon used as security stack filler — if a security check happens
/// it is consumed, so `security.len()` detects whether any check ran.
fn security_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, "Security Filler");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(0);
    c.play_cost = 3;
    c.colors = vec![CardColor::Red];
    c
}

fn filler(id: &str) -> CardData {
    make_test_card(id, "Filler")
}

/// Push a registered card into player 0's hand; returns the hand index.
fn put_in_hand(runner: &mut DebugRunner, player: u8, card_id: &str) -> usize {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("put_in_hand: card {card_id} not registered"));
    let card_index = runner.game.next_card_index();
    runner.game.players[player as usize]
        .hand
        .push(CardSource::new(data_idx, player, card_index));
    runner.game.player(player).hand.len() - 1
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

/// EX8-019 compiles with its printed stats: Lv.3 blue/yellow Digimon, cost 3,
/// DP 1000, traits Avian + LIBERATOR + Ice-Snow (the rule box "Trait: Has
/// [Ice-Snow] Type." is printed card text — the card has the trait in every
/// zone), the standard Lv.2-blue cost-1 path, and the printed
/// "[Digivolve] [Hiyarimon]: Cost 0" alternate path (DCGO checks
/// `ContainsCardName("Hiyarimon")` only — no level/color restriction).
#[test]
fn ex8_019_compiles_with_printed_stats_and_digivolve_paths() {
    let runner = DebugRunner::builder()
        .dsl_card("EX8-019")
        .expect("EX8-019 found in embedded DSL pack")
        .start();

    let compiled = runner
        .compiled_card("EX8-019")
        .expect("EX8-019 in compiled cards");

    assert_eq!(compiled.card, "EX8-019");
    assert_eq!(compiled.name, "Penguinmon");
    assert_eq!(compiled.kind, CompiledCardKind::Digimon);
    assert_eq!(compiled.level, Some(3));
    assert_eq!(
        compiled.color,
        vec![CompiledColor::Blue, CompiledColor::Yellow],
        "cards.json card_colors [1, 2] = blue + yellow (multi-color)"
    );
    assert_eq!(compiled.cost, Some(3));
    assert_eq!(compiled.dp, Some(1000));
    for trait_name in ["Avian", "LIBERATOR", "Ice-Snow"] {
        assert!(
            compiled.traits.iter().any(|t| t == trait_name),
            "missing trait {trait_name} (Ice-Snow comes from the printed rule box)"
        );
    }

    // Printed alt path: "[Digivolve] [Hiyarimon]: Cost 0" — name-gated only,
    // mirroring DCGO's ContainsCardName("Hiyarimon").
    assert!(
        compiled.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(0))
                && path.from.as_ref().is_some_and(|from| {
                    from.name_contains.as_deref() == Some("Hiyarimon")
                        || from
                            .all_of
                            .iter()
                            .any(|pred| pred.name_contains.as_deref() == Some("Hiyarimon"))
                })
        }),
        "Penguinmon must digivolve from [Hiyarimon] for cost 0"
    );

    // Standard printed evo box: Lv.2 blue for 1 memory (encoded as an explicit
    // alt path like neighbor EX8 specs so DSL-only loads carry it).
    assert!(
        compiled.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(1))
                && path.from.as_ref().is_some_and(|from| {
                    (from.level_eq == Some(2)
                        || from.all_of.iter().any(|pred| pred.level_eq == Some(2)))
                        && (from.color_is == Some(CompiledColor::Blue)
                            || from
                                .all_of
                                .iter()
                                .any(|pred| pred.color_is == Some(CompiledColor::Blue)))
                })
        }),
        "Penguinmon must digivolve from a blue Lv.2 for cost 1 (printed evo box)"
    );
}

/// Clause 1: the cost-reduction clause fires at `before_pay_cost`, reduces by
/// exactly 1, is NOT [Once Per Turn] (no printed qualifier; DCGO's
/// ChangeDigivolutionCostStaticEffect has no max-count), and carries an
/// `active_when` gate ([Your Turn] + source/cost-target scoping).
#[test]
fn ex8_019_cost_reduction_clause_structural_shape() {
    let runner = DebugRunner::builder()
        .dsl_card("EX8-019")
        .expect("EX8-019 found in embedded DSL pack")
        .start();
    let compiled = runner
        .compiled_card("EX8-019")
        .expect("EX8-019 in compiled cards");

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
        .expect("EX8-019 must encode a CostReduction clause");

    assert_eq!(
        reduction_timing.as_deref(),
        Some("before_pay_cost"),
        "cost reduction must fire at BeforePayCost cost-calc"
    );
    assert_eq!(amount, Some(1), "printed text reduces the cost by exactly 1");
    assert!(
        !once_per_turn,
        "printed text carries no [Once Per Turn] — the reduction applies to every \
         qualifying digivolution"
    );
    assert!(
        active_when.is_some(),
        "the [Your Turn] qualifier + cost-target gates must be encoded as active_when"
    );
}

/// Clause 2: the inherited [When Attacking] clause is Inherited-scoped,
/// triggered at when_attacking, and NOT optional — the printed text has no
/// "you may", so the target selection is mandatory.
#[test]
fn ex8_019_inherited_when_attacking_clause_structural_shape() {
    let runner = DebugRunner::builder()
        .dsl_card("EX8-019")
        .expect("EX8-019 found in embedded DSL pack")
        .start();
    let compiled = runner
        .compiled_card("EX8-019")
        .expect("EX8-019 in compiled cards");

    let triggered = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("EX8-019 must encode a triggered inherited clause");

    assert_eq!(
        triggered.scope,
        CompiledScope::Inherited,
        "the Security A. -1 grant is an inherited (ESS) effect"
    );
    assert!(
        triggered.when.contains(&CompiledTiming::WhenAttacking),
        "the inherited clause fires at [When Attacking]"
    );
    assert!(
        !triggered.optional,
        "no printed 'you may' — the effect (and its selection) is mandatory"
    );
    assert!(
        !triggered.once_per_turn,
        "printed text carries no [Once Per Turn]"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — Clause 1: cost-reduction behavior (digivolve-into-Ice-Snow)
// ─────────────────────────────────────────────────────────────────────────────

/// POSITIVE: EX8-019 on the field digivolves into a Lv.4 [Ice-Snow] Digimon.
/// The printed digivolution cost is 2; EX8-019's [Your Turn] effect reduces it
/// by 1, so memory drops by only 1.
#[test]
fn ex8_019_cost_reduction_reduces_digivolving_into_ice_snow_by_one() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-019")
        .expect("EX8-019 found in embedded DSL pack")
        .add_card(ice_snow_lv4("ICESNOW-LV4"))
        .add_card(filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(8)
        .start();
    runner.game.turn_count = 1;

    let penguinmon = runner.place_on_field(0, "EX8-019", Some(0));
    let hand_idx = put_in_hand(&mut runner, 0, "ICESNOW-LV4");

    let memory_before = runner.game.memory;
    let digivolved = runner.game.digivolve_from_hand(
        0,
        hand_idx,
        penguinmon.index as usize,
        PlaySource::ByHand,
    );
    assert!(
        digivolved,
        "EX8-019 must digivolve into the Lv.4 Ice-Snow Digimon (effective cost 2 - 1 = 1)"
    );

    assert_eq!(
        runner.game.memory,
        memory_before - 1,
        "digivolving into an [Ice-Snow] Digimon must cost 1 memory (2 printed - 1 reduction)"
    );
}

/// NEGATIVE: EX8-019 digivolves into a Lv.4 Digimon that lacks the [Ice-Snow]
/// trait. The cost-target-trait predicate fails, so the reduction must NOT
/// fire — memory drops by the full printed 2.
#[test]
fn ex8_019_cost_reduction_does_not_apply_to_non_ice_snow_target() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-019")
        .expect("EX8-019 found in embedded DSL pack")
        .add_card(non_ice_snow_lv4("NONICE-LV4"))
        .add_card(filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(8)
        .start();
    runner.game.turn_count = 1;

    let penguinmon = runner.place_on_field(0, "EX8-019", Some(0));
    let hand_idx = put_in_hand(&mut runner, 0, "NONICE-LV4");

    let memory_before = runner.game.memory;
    let digivolved = runner.game.digivolve_from_hand(
        0,
        hand_idx,
        penguinmon.index as usize,
        PlaySource::ByHand,
    );
    assert!(
        digivolved,
        "EX8-019 must still digivolve into the non-Ice-Snow Lv.4 Digimon"
    );

    assert_eq!(
        runner.game.memory,
        memory_before - 2,
        "digivolving into a non-[Ice-Snow] Digimon must cost the full 2 memory"
    );
}

/// NEGATIVE (source scope): EX8-019 sits on the field as a standalone
/// permanent while a SEPARATE, unrelated Lv.3 blue Digimon digivolves into a
/// Lv.4 [Ice-Snow]. EX8-019's cost reduction is gated by
/// `source_is_cost_target_permanent` (DCGO `permanentCondition:
/// targetPermanent == card.PermanentOfThisCard()`) — it applies ONLY when
/// EX8-019's OWN permanent digivolves. The unrelated digivolution must pay the
/// full printed 2. This catches a regression that drops the source-scope
/// predicate (which would leak the reduction onto every digivolution while a
/// bare EX8-019 sits on the field).
#[test]
fn ex8_019_cost_reduction_does_not_apply_to_unrelated_permanent_digivolution() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-019")
        .expect("EX8-019 found in embedded DSL pack")
        .add_card(other_lv3("OTHER-LV3"))
        .add_card(ice_snow_lv4("ICESNOW-LV4"))
        .add_card(filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(8)
        .start();
    runner.game.turn_count = 1;

    let _penguinmon = runner.place_on_field(0, "EX8-019", Some(0));
    let other = runner.place_on_field(0, "OTHER-LV3", Some(0));
    let hand_idx = put_in_hand(&mut runner, 0, "ICESNOW-LV4");

    let memory_before = runner.game.memory;
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_idx, other.index as usize, PlaySource::ByHand);
    assert!(
        digivolved,
        "the unrelated Lv.3 must digivolve into the Lv.4 Ice-Snow Digimon"
    );

    assert_eq!(
        runner.game.memory,
        memory_before - 2,
        "EX8-019's cost reduction is scoped to its OWN permanent; an unrelated \
         permanent digivolving into an Ice-Snow Digimon pays the full 2 memory"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — Clause 2: inherited [When Attacking] Security A. -1 behavior
// ─────────────────────────────────────────────────────────────────────────────

/// POSITIVE: a carrier stacked on EX8-019 attacks; the inherited [When
/// Attacking] effect installs a MANDATORY single-target opponent-Digimon
/// selection (no PASS in the mask — the printed text has no "you may"), and
/// the chosen Digimon gets SecurityAttackChange -1.
#[test]
fn ex8_019_inherited_when_attacking_mandatory_select_gives_security_attack_minus_1() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-019")
        .expect("EX8-019 found in embedded DSL pack")
        .add_card(carrier("CARRIER"))
        .add_card(opp_attacker("OPP"))
        .start();
    let stack = runner.place_stack(0, &["EX8-019", "CARRIER"]);
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(stack));
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("inherited [When Attacking] must install an opponent-target selection");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(
        !runner.pending_is_optional(),
        "printed text has no 'you may' — the selection must not expose PASS"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose the opponent Digimon");
    runner.auto_resolve().expect("finish the Security A. -1 grant");

    assert_eq!(
        runner
            .game
            .modifiers
            .sum(opp, ModifierType::SecurityAttackChange),
        -1,
        "the chosen opponent Digimon gets <Security A. -1>"
    );
}

/// BEHAVIORAL + EXPIRY: the granted Security A. -1 persists through the end of
/// the granting player's turn ("until the end of THEIR turn" = the opponent's
/// turn end; DCGO EffectDuration.UntilOpponentTurnEnd), suppresses the marked
/// Digimon's security check during the opponent's turn (base 1 - 1 = 0 checks),
/// and expires when the opponent's turn ends.
#[test]
fn ex8_019_security_attack_minus_1_zero_checks_and_expires_at_end_of_their_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-019")
        .expect("EX8-019 found in embedded DSL pack")
        .add_card(carrier("CARRIER"))
        .add_card(opp_attacker("OPP"))
        .add_card(security_filler("SEC"))
        .add_card(filler("FILL"))
        // Both players need draw fuel: the test rotates through two turn ends,
        // and an empty deck would deck-out the drawing player (game_over makes
        // end_turn a no-op, masking the expiry under test).
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .security(0, &["SEC", "SEC", "SEC"])
        .start();
    let stack = runner.place_stack(0, &["EX8-019", "CARRIER"]);
    let opp = runner.place_on_field(1, "OPP", Some(0));

    // Player 0's turn: fire the inherited WA and mark the opponent Digimon.
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(stack));
    runner.game.drain_effect_queue();
    let view = runner.pending_selection_view().expect("opponent target");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose the opponent Digimon");
    runner.auto_resolve().expect("finish the Security A. -1 grant");
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(opp, ModifierType::SecurityAttackChange),
        -1
    );

    // End of player 0's turn: "until the end of THEIR turn" must SURVIVE the
    // granting player's own turn end.
    runner.end_turn();
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(opp, ModifierType::SecurityAttackChange),
        -1,
        "Security A. -1 must persist into the opponent's turn (their turn has not ended)"
    );

    // Opponent's turn: the marked Digimon attacks player 0. Base 1 security
    // check - 1 = 0 checks, so no security card is consumed and the attacker
    // survives without battling security.
    let security_before = runner.game.players[0].security.len();
    assert_eq!(security_before, 3);
    let result = runner.attack_player(opp, 0, false);
    assert_eq!(
        result,
        AttackResult::SecurityCheckSurvived,
        "0 checks: the marked attacker survives without consuming security"
    );
    assert_eq!(
        runner.game.players[0].security.len(),
        3,
        "<Security A. -1> cancels the base check — no security card consumed"
    );

    // End of the opponent's turn: the modifier expires.
    runner.end_turn();
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(opp, ModifierType::SecurityAttackChange),
        0,
        "Security A. -1 expires at the end of the opponent's turn"
    );
}

/// NEGATIVE (gate): with NO opponent Digimon on the field, the inherited
/// [When Attacking] effect must not install a selection (DCGO
/// CanActivateCondition requires an opponent battle-area Digimon; in the DSL
/// an empty mandatory select is silently skipped).
#[test]
fn ex8_019_inherited_when_attacking_skips_with_no_opponent_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-019")
        .expect("EX8-019 found in embedded DSL pack")
        .add_card(carrier("CARRIER"))
        .start();
    let stack = runner.place_stack(0, &["EX8-019", "CARRIER"]);

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(stack));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no opponent Digimon — the mandatory selection must be silently skipped, \
         not parked as an empty prompt"
    );
}
