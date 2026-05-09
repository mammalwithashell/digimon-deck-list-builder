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
use digimon_engine::enums::{CardColor, GamePhase};

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
    assert!(card.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::DnaDigivolve
            && path.cost == Some(CompiledCost::Literal(0))
    }));
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

#[test]
#[ignore = "pending: G-SECURITY-REMOVED-OBSERVER — all-turns security stack removed observer"]
fn bt20_060_security_removed_gain_three_memory_once_per_turn() {
    panic!("requires security-removed event dispatch and OPT handling");
}
