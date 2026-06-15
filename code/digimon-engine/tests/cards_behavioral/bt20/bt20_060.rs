//! BT20-060 Alphamon: Ouryuken - Digimon, Lv.7, Black/Purple/Red.
//!
//! Supported slice:
//! - Printed metadata, ACE Overflow, standard and DNA digivolve routes.
//! - [On Play][When Digivolving] select 1 opponent Digimon and give -15000 DP
//!   until the end of the opponent's turn.
//! - [Hand][Counter] Blast DNA Digivolve ([Alphamon] + [Ouryumon]).
//!
//! Gap-routed:
//! - DNA-gated security trash + Recovery and the security-removed memory
//!   observer need faithful DNA-origin and security-removed dispatch coverage.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledModifierValue, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{DNA_DIGIVOLVE_START, PASS, PLAY_HAND_START};
use digimon_engine::action::{build_action_mask, encode_attack};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level};
use digimon_engine::enums::{CardColor, EffectTiming, GamePhase};
use digimon_engine::selection::TriggerSource;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT20-060")
        .expect("BT20-060 YAML loads")
        .memory(10)
        .start()
}

#[test]
fn bt20_060_has_printed_metadata_ace_overflow_and_routes() {
    let runner = runner();
    let card = runner
        .compiled_card("BT20-060")
        .expect("BT20-060 compiled card present");

    assert_eq!(card.name, "Alphamon: Ouryuken");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(7));
    assert_eq!(card.cost, Some(6));
    assert_eq!(card.dp, Some(16000));
    assert_eq!(
        card.color,
        vec![
            CompiledColor::Black,
            CompiledColor::Purple,
            CompiledColor::Red
        ]
    );
    assert!(card.traits.iter().any(|name| name == "X Antibody"));
    assert!(card.traits.iter().any(|name| name == "Royal Knight"));
    assert!(card.traits.iter().any(|name| name == "Chronicle"));
    assert_eq!(card.attribute.as_deref(), Some("Vaccine"));
    assert_eq!(card.ace_overflow, Some(-5));

    assert!(card.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && path.cost == Some(CompiledCost::Literal(6))
            && path.from.as_ref().is_some_and(|from| {
                from.level_eq == Some(6) && from.color_is == Some(CompiledColor::Black)
            })
    }));
    // Regular DNA Digivolve route: Black Lv.6 + (Yellow|Red) Lv.6, cost 0 — NOT
    // name-gated (only the separate Blast DNA route below is Alphamon+Ouryumon).
    assert!(card.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::DnaDigivolve
            && path.cost == Some(CompiledCost::Literal(0))
            && path.materials.iter().any(|mat| {
                mat.filter.level_eq == Some(6)
                    && mat.filter.color_is == Some(CompiledColor::Black)
            })
            && !path.materials.iter().any(|mat| {
                mat.filter.name_is.is_some() || mat.filter.name_contains.is_some()
            })
    }), "BT20-060 regular DNA route must be Black Lv.6 + Yellow/Red Lv.6 (level/color), not name-restricted");
    assert!(card.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::BlastDnaDigivolve
            && path.cost == Some(CompiledCost::Literal(0))
            && path
                .materials
                .iter()
                .any(|mat| mat.filter.name_is.as_deref() == Some("Alphamon"))
            && path
                .materials
                .iter()
                .any(|mat| mat.filter.name_is.as_deref() == Some("Ouryumon"))
    }));
}

#[test]
fn bt20_060_on_play_when_digivolving_selects_opponent_digimon_for_minus_15000() {
    let runner = runner();
    let card = runner
        .compiled_card("BT20-060")
        .expect("BT20-060 compiled card present");

    let clause = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnPlay)
                    && triggered.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("On Play/When Digivolving DP reduction clause exists");

    assert!(!clause.optional, "printed DP reduction has no 'may'");
    assert!(matches!(
        clause.process.first(),
        Some(CompiledStep::SelectOpponentPermanent { filter, .. })
            if filter.kind == Some(CompiledCardKind::Digimon)
    ));
    assert!(clause.process.iter().any(|step| matches!(
        step,
        CompiledStep::AddDpModifier {
            value: CompiledModifierValue::Literal(-15000),
            ..
        }
    )));
}

#[test]
fn bt20_060_hand_counter_blast_dna_uses_alphamon_and_ouryumon() {
    let mut alphamon = make_test_card_with_level("BT20-056", "Alphamon", 6);
    alphamon.colors = vec![CardColor::Black];
    alphamon.dp = Some(12000);

    let mut ouryumon = make_test_card_with_level("BT20-018", "Ouryumon", 6);
    ouryumon.colors = vec![CardColor::Red, CardColor::Yellow];
    ouryumon.dp = Some(12000);

    let mut attacker = make_test_card_with_level("ATTACKER", "Attacker", 6);
    attacker.colors = vec![CardColor::Red];
    attacker.dp = Some(17000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-060")
        .expect("BT20-060 YAML loads")
        .add_card(alphamon)
        .add_card(ouryumon)
        .add_card(attacker)
        .hand(1, &["BT20-060", "BT20-018"])
        .start();

    let attacking = runner.place_on_field(0, "ATTACKER", Some(0));
    let alphamon = runner.place_on_field(1, "BT20-056", Some(0));

    let result = runner.attack_digimon(attacking, alphamon, false);
    assert_eq!(result, digimon_engine::combat::AttackResult::InProgress);
    assert_eq!(runner.current_phase(), GamePhase::CounterTiming);

    let counter_prompt = runner
        .pending_selection()
        .expect("Counter window should offer Blast DNA");
    assert_eq!(counter_prompt.selecting_player, 1);
    assert!(counter_prompt.is_optional);
    assert!(
        counter_prompt
            .valid_action_ids
            .contains(&(DNA_DIGIVOLVE_START)),
        "BT20-060 in hand slot 0 should be offered as a Counter Blast DNA action: {:?}",
        counter_prompt.valid_action_ids
    );
    let mask = build_action_mask(&runner.game, 1);
    assert_eq!(mask[DNA_DIGIVOLVE_START as usize], 1.0);
    assert_eq!(mask[PASS as usize], 1.0);

    runner
        .execute_action(1, DNA_DIGIVOLVE_START)
        .expect("choose BT20-060 for Counter Blast DNA");
    assert_eq!(runner.current_phase(), GamePhase::SelectMaterial);
    let field_prompt = runner
        .pending_selection()
        .expect("Blast DNA should ask for the field material");
    assert_eq!(field_prompt.selecting_player, 1);
    assert_eq!(field_prompt.valid_action_ids, vec![0]);

    runner
        .execute_action(1, 0)
        .expect("choose Alphamon as the field material");
    let hand_prompt = runner
        .pending_selection()
        .expect("Blast DNA should ask for the hand material");
    assert_eq!(hand_prompt.selecting_player, 1);
    assert_eq!(hand_prompt.valid_action_ids, vec![PLAY_HAND_START + 1]);

    runner
        .execute_action(1, PLAY_HAND_START + 1)
        .expect("choose Ouryumon as the hand material");

    if runner.pending_selection().is_some() {
        runner
            .execute_action(1, encode_attack(0, 0))
            .expect("resolve BT20-060's When Digivolving target selection");
    }

    let evolved = &runner.game.players[1].battle_area[0];
    assert_eq!(evolved.card_sources.len(), 3);
    assert_eq!(
        evolved
            .top_card()
            .card_id(&runner.game.card_data)
            .to_string(),
        "BT20-060"
    );
    assert!(
        evolved
            .card_sources
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "BT20-018"),
        "Ouryumon from hand should be inserted as DNA material"
    );
    assert_eq!(runner.hand_size(1), 0);
}

#[test]
fn bt20_060_dna_origin_trashes_security_and_recovers() {
    let mut alphamon = make_test_card_with_level("BT20-056", "Alphamon", 6);
    alphamon.colors = vec![CardColor::Black];
    alphamon.dp = Some(12000);

    let mut ouryumon = make_test_card_with_level("BT20-018", "Ouryumon", 6);
    ouryumon.colors = vec![CardColor::Red, CardColor::Yellow];
    ouryumon.dp = Some(12000);

    let mut attacker = make_test_card_with_level("ATTACKER", "Attacker", 6);
    attacker.colors = vec![CardColor::Red];
    attacker.dp = Some(17000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-060")
        .expect("BT20-060 YAML loads")
        .add_card(alphamon)
        .add_card(ouryumon)
        .add_card(attacker)
        .add_card(make_test_card("P0-SEC-BOTTOM", "P0 Security Bottom"))
        .add_card(make_test_card("P0-SEC-TOP", "P0 Security Top"))
        .add_card(make_test_card("P1-RECOVER", "P1 Recover"))
        .hand(1, &["BT20-060", "BT20-018"])
        .security(0, &["P0-SEC-BOTTOM", "P0-SEC-TOP"])
        .deck(1, &["P1-RECOVER"])
        .start();

    let attacking = runner.place_on_field(0, "ATTACKER", Some(0));
    let alphamon = runner.place_on_field(1, "BT20-056", Some(0));

    let result = runner.attack_digimon(attacking, alphamon, false);
    assert_eq!(result, digimon_engine::combat::AttackResult::InProgress);
    assert_eq!(runner.current_phase(), GamePhase::CounterTiming);

    runner
        .execute_action(1, DNA_DIGIVOLVE_START)
        .expect("choose BT20-060 for Counter Blast DNA");
    runner
        .execute_action(1, 0)
        .expect("choose Alphamon as the field material");
    runner
        .execute_action(1, PLAY_HAND_START + 1)
        .expect("choose Ouryumon as the hand material");
    runner
        .execute_action(1, encode_attack(0, 0))
        .expect("resolve BT20-060's When Digivolving target selection");

    assert_eq!(
        runner.security_count(0),
        1,
        "DNA-origin rider must trash the opponent's top security"
    );
    assert_eq!(
        runner.security_count(1),
        1,
        "DNA-origin rider must recover 1 for BT20-060's controller"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "P0-SEC-TOP"),
        "the top security card should be the one trashed"
    );
    assert_eq!(
        runner.game.players[1].security[0].card_id(&runner.game.card_data),
        "P1-RECOVER",
        "recovered card should come from the top of the defender's deck"
    );
}

// ─── [All Turns][OPT] security-removed → gain 3 memory (unified observer) ────
//
// Printed: [All Turns][Once Per Turn] When security stacks are removed from,
// gain 3 memory. DCGO BT20_060.cs:256-284 implements this as ONE
// `OnLoseSecurity` clause gated by `CanTriggerWhenLoseSecurity(_, _ => true)`
// (fires for EITHER stack) under a single OPT key `RemovedSec_BT20_060` → one
// fire across both stacks per turn. The DSL expresses this by listing both
// security-removed timings in one triggered clause; the lowering clusters them
// under a single shared once-per-turn lockout (precedent: BT16-102.yaml:94).

/// Helper: fire one security-removed observer for BT20-060's carrier.
fn fire_security_removed(
    runner: &mut DebugRunner,
    carrier: digimon_engine::permanent::PermanentHandle,
    timing: EffectTiming,
) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();
}

/// Structure: the [All Turns][OPT] clause fires on BOTH security-removed
/// timings (own + opponent), is all-turns, once_per_turn, and gains 3 memory.
#[test]
fn bt20_060_security_removed_clause_is_unified_all_turns_opt() {
    let runner = runner();
    let card = runner
        .compiled_card("BT20-060")
        .expect("BT20-060 compiled card present");

    let clause = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered
                    .when
                    .contains(&CompiledTiming::OnOwnSecurityRemoved)
                    || triggered
                        .when
                        .contains(&CompiledTiming::OnOpponentSecurityRemoved) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("security-removed memory clause exists");

    assert!(
        clause
            .when
            .contains(&CompiledTiming::OnOwnSecurityRemoved),
        "must fire when OWN security stack is removed from"
    );
    assert!(
        clause
            .when
            .contains(&CompiledTiming::OnOpponentSecurityRemoved),
        "must fire when OPPONENT's security stack is removed from"
    );
    assert!(
        clause.once_per_turn,
        "printed [Once Per Turn] — single shared lockout across both stacks"
    );
    assert!(
        clause.process.iter().any(|step| matches!(
            step,
            CompiledStep::GainMemory(3)
        )),
        "clause must gain exactly 3 memory; process={:?}",
        clause.process
    );
}

/// Positive: firing the OWN-stack observer gains exactly 3 memory.
#[test]
fn bt20_060_own_security_removed_gains_three_memory() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-060")
        .expect("BT20-060 YAML loads")
        .memory(0)
        .start();
    let carrier = runner.place_on_field(0, "BT20-060", Some(0));

    let before = runner.memory();
    fire_security_removed(&mut runner, carrier, EffectTiming::OnOwnSecurityRemoved);
    let delta = runner.memory() - before;

    assert_eq!(
        delta, 3,
        "own-stack security removal must gain exactly 3 memory; delta={delta}"
    );
}

/// Positive: firing the OPPONENT-stack observer gains exactly 3 memory.
#[test]
fn bt20_060_opponent_security_removed_gains_three_memory() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-060")
        .expect("BT20-060 YAML loads")
        .memory(0)
        .start();
    let carrier = runner.place_on_field(0, "BT20-060", Some(0));

    let before = runner.memory();
    fire_security_removed(
        &mut runner,
        carrier,
        EffectTiming::OnOpponentSecurityRemoved,
    );
    let delta = runner.memory() - before;

    assert_eq!(
        delta, 3,
        "opponent-stack security removal must gain exactly 3 memory; delta={delta}"
    );
}

/// Unified OPT: removing from BOTH stacks in the SAME turn fires the observer
/// only ONCE — total memory gain is EXACTLY 3 (not 6). This is the load-bearing
/// assertion for G-SECURITY-REMOVED-OBSERVER-UNIFIED: the two security-removed
/// timings share a single once-per-turn lockout.
#[test]
fn bt20_060_both_stacks_same_turn_fires_once_gaining_exactly_three() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-060")
        .expect("BT20-060 YAML loads")
        .memory(0)
        .start();
    let carrier = runner.place_on_field(0, "BT20-060", Some(0));

    let before = runner.memory();

    // First: own stack hit → observer fires (+3).
    fire_security_removed(&mut runner, carrier, EffectTiming::OnOwnSecurityRemoved);
    // Then (same turn): opponent stack hit → shared OPT lockout suppresses it.
    fire_security_removed(
        &mut runner,
        carrier,
        EffectTiming::OnOpponentSecurityRemoved,
    );

    let delta = runner.memory() - before;
    assert_eq!(
        delta, 3,
        "unified observer fires once across both stacks per turn (+3, not +6); delta={delta}"
    );
}

/// OPT lockout: a second removal from the SAME stack in the same turn is also
/// suppressed (the [Once Per Turn] gate is not per-stack).
#[test]
fn bt20_060_second_same_stack_removal_is_suppressed_same_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-060")
        .expect("BT20-060 YAML loads")
        .memory(0)
        .start();
    let carrier = runner.place_on_field(0, "BT20-060", Some(0));

    let before = runner.memory();
    fire_security_removed(&mut runner, carrier, EffectTiming::OnOwnSecurityRemoved);
    fire_security_removed(&mut runner, carrier, EffectTiming::OnOwnSecurityRemoved);
    let delta = runner.memory() - before;

    assert_eq!(
        delta, 3,
        "[Once Per Turn] suppresses a second same-stack removal in the same turn; delta={delta}"
    );
}
