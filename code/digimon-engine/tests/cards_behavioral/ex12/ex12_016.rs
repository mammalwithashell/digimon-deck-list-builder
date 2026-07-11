use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
use digimon_engine::enums::{CardColor, EffectTiming, Keyword};
use digimon_engine::selection::SelectionKind;

use super::support::{
    field_contains, hand_index, plain_digimon, select_first_non_pass, DebugRunner,
};

const CARD_ID: &str = "EX12-016";

#[test]
fn ex12_016_has_security_attack_plus_and_decode_on_face_and_inherited() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-016 YAML loads")
        .start();
    let metalgreymon = runner.place_on_field(0, CARD_ID, Some(0));
    let card = runner.compiled_card(CARD_ID).expect("compiled EX12-016");

    assert!(runner
        .game
        .has_keyword(metalgreymon, Keyword::SecurityAttackPlus(1)));

    let decode_replacements = card
        .effects
        .iter()
        .filter(|clause| {
            matches!(
                clause,
                CompiledClause::Declarative(CompiledDeclarativeClause::Replacement { .. })
            )
        })
        .count();
    assert_eq!(
        decode_replacements, 2,
        "MetalGreymon has Decode on its face and inherited text"
    );
    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
            scope: CompiledScope::Inherited,
            ..
        })
    )));
}

#[test]
fn ex12_016_on_play_deletes_6000_dp_or_less_and_grants_forced_attack() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-016 YAML loads")
        .add_card(plain_digimon("OPP-HIGH", CardColor::Red, 4, 7000))
        .add_card(plain_digimon("OPP-LOW", CardColor::Red, 4, 6000))
        .add_card(plain_digimon("SEC", CardColor::Red, 3, 2000))
        .add_card(plain_digimon("FILL-016", CardColor::Red, 3, 2000))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL-016", "FILL-016", "FILL-016"])
        .deck(1, &["FILL-016", "FILL-016", "FILL-016"])
        .security(0, &["SEC"])
        .memory(10)
        .start();
    let high = runner.place_on_field(1, "OPP-HIGH", None);
    runner.place_on_field(1, "OPP-LOW", None);

    let play_slot = hand_index(&runner, 0, CARD_ID);
    runner.play(0, play_slot).expect("play EX12-016");

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "first prompt chooses the <=6000 DP delete target"
    );
    select_first_non_pass(&mut runner);
    assert!(!field_contains(&runner, 1, "OPP-LOW"));
    assert!(field_contains(&runner, 1, "OPP-HIGH"));

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "then choose one of their Digimon to gain the forced attack"
    );
    select_first_non_pass(&mut runner);
    assert_eq!(
        runner
            .game
            .modifiers
            .granted_triggered_for_timing(high, EffectTiming::StartOfYourMainPhase)
            .len(),
        1,
        "selected opponent Digimon should carry the Start-of-Main forced attack"
    );

    runner.end_turn();
    let prompt = runner
        .pending_selection()
        .expect("forced attack should choose a target at opponent main phase");
    assert_eq!(prompt.kind, SelectionKind::Target);
    assert_eq!(prompt.selecting_player, 1);
    assert!(!prompt.is_optional);
    assert!(prompt
        .valid_action_ids
        .contains(&encode_attack(high.index as u16, SECURITY_TARGET)));
}

#[test]
fn ex12_016_shared_on_play_when_digivolving_clause_grants_forced_attack_body() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-016 YAML loads")
        .start();
    let card = runner.compiled_card(CARD_ID).expect("compiled EX12-016");

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
        .expect("shared OP/WD clause exists");

    assert!(clause.process.iter().any(|step| matches!(
        step,
        CompiledStep::GrantTriggeredEffect { body, .. }
            if body.iter().any(|nested| matches!(nested, CompiledStep::ForceAttack { .. }))
    )));
}
