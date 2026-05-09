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
use digimon_engine::action::space::{encode_digivolve, PLAY_HAND_START};
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::CardColor;

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
    let mut alphamon = make_test_card_with_level("ALPHAMON-MAT", "Alphamon", 6);
    alphamon.colors = vec![CardColor::Black];
    alphamon.dp = Some(10000);
    let mut ouryumon = make_test_card_with_level("OURYUMON-MAT", "Ouryumon", 6);
    ouryumon.colors = vec![CardColor::Red];
    ouryumon.dp = Some(10000);
    let mut attacker = make_test_card_with_level("ATTACKER", "Attacker", 4);
    attacker.dp = Some(4000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-060")
        .expect("BT20-060 YAML loads")
        .add_card(alphamon)
        .add_card(ouryumon)
        .add_card(attacker)
        .hand(1, &["BT20-060", "OURYUMON-MAT"])
        .memory(0)
        .start();

    let atk = runner.place_on_field(0, "ATTACKER", Some(0));
    let target = runner.place_on_field(1, "ALPHAMON-MAT", Some(0));
    assert_eq!(runner.attack_digimon(atk, target, false), AttackResult::InProgress);

    let field_pick = encode_digivolve(0, 0);
    let pending = runner
        .pending_selection()
        .expect("Counter window should offer BT20-060 Blast DNA field material");
    assert!(
        pending.valid_action_ids.contains(&field_pick),
        "field Alphamon should be selectable for BT20-060 Blast DNA"
    );
    let selecting_player = pending.selecting_player;
    runner
        .execute_action(selecting_player, field_pick)
        .expect("select field material");

    let pending = runner
        .pending_selection()
        .expect("Blast DNA should ask for Ouryumon from hand");
    assert_eq!(pending.valid_action_ids, vec![PLAY_HAND_START + 1]);
    let selecting_player = pending.selecting_player;
    runner
        .execute_action(selecting_player, PLAY_HAND_START + 1)
        .expect("select hand material");

    let stack_ids: Vec<_> = runner.game.player(1).battle_area[0]
        .card_sources
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(stack_ids, vec!["ALPHAMON-MAT", "OURYUMON-MAT", "BT20-060"]);
}

#[test]
#[ignore = "pending: DNA-origin gated tail plus security-trash/recovery sequence for BT20-060"]
fn bt20_060_dna_origin_trashes_security_and_recovers() {
    panic!("requires faithful DNA-origin tail sequencing");
}

#[test]
#[ignore = "pending: G-SECURITY-REMOVED-OBSERVER — all-turns security stack removed observer"]
fn bt20_060_security_removed_gain_three_memory_once_per_turn() {
    panic!("requires security-removed event dispatch and OPT handling");
}
