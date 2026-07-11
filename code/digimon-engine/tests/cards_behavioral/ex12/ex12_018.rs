use super::support::{
    fire_when_digivolving, plain_digimon, select_first_non_pass, vb_digimon, vb_text_digimon,
    DebugRunner,
};
use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledStep, CompiledTiming};
use digimon_engine::enums::CardColor;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX12-018";

#[test]
fn ex12_018_is_dual_with_digimon_and_option_face_effects() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-018 YAML loads")
        .start();
    let card = runner.compiled_card(CARD_ID).expect("compiled EX12-018");
    assert_eq!(card.kind, CompiledCardKind::Dual);
    let dual = card.dual.as_ref().expect("dual metadata");

    for keyword in ["Progress", "Piercing", "SecurityAttackPlus"] {
        assert!(
            dual.digimon.effects.iter().any(|clause| matches!(
                clause,
                CompiledClause::Declarative(d) if format!("{d:?}").contains(keyword)
            )),
            "Digimon face must grant {keyword}"
        );
    }
    assert!(
        dual.digimon.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenDigivolving)
                    && t.when.contains(&CompiledTiming::WhenAttacking)
                    && t.process.iter().any(|step| matches!(step, CompiledStep::SelectUnionZone { .. }))
        )),
        "Digimon face must place source cards from hand/trash"
    );
    assert!(
        dual.option.keywords.iter().any(|k| k == "ArtsDigivolve"),
        "Option face must carry Arts Digivolve"
    );
    assert_eq!(dual.option.security_text, "");
    assert_eq!(
        dual.option.effects.len(),
        1,
        "Option face should only have [Main]"
    );
}

#[test]
fn ex12_018_when_digivolving_places_source_then_debuffs_per_source() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-018 YAML loads")
        .add_card(vb_digimon("BASE", CardColor::Red, 5, 8000))
        .add_card(vb_text_digimon("VB-FUEL", CardColor::Yellow, 4, 5000))
        .add_card(plain_digimon("OPP", CardColor::Purple, 5, 9000))
        .hand(0, &["VB-FUEL"])
        .memory(10)
        .start();

    let sirius = runner.place_stack(0, &["BASE", CARD_ID]);
    let opp = runner.place_on_field(1, "OPP", Some(0));

    fire_when_digivolving(&mut runner, sirius);
    let pick = runner
        .pending_selection_view()
        .expect("source placement prompt");
    assert!(matches!(pick.kind, SelectionKind::UnionZone { .. }));
    select_first_non_pass(&mut runner);
    runner.execute_branch(0).expect("place as top source");

    let dp_prompt = runner
        .pending_selection_view()
        .expect("opponent DP target prompt");
    assert_eq!(dp_prompt.kind, SelectionKind::OppField);
    select_first_non_pass(&mut runner);

    assert_eq!(
        runner.effective_dp(opp),
        Some(5000),
        "BASE plus the placed source means 2 sources, so the target gets -4000 DP"
    );
}
