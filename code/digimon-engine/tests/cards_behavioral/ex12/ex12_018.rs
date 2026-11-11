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
                    && t.process.iter().any(|step| matches!(step, CompiledStep::SelectEffectChoice { .. }))
        )),
        "Digimon face must place source cards from hand/trash (zone menu first, as DCGO)"
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
    // DCGO EX12_018.cs: first "where from?" - only zones holding a candidate are offered (hand here, no trash
    // candidate), plus "Don't place any"; then up to 2 cards from that ONE zone; then top/bottom once for all.
    let zone = runner
        .pending_selection_view()
        .expect("zone menu");
    assert_eq!(zone.kind, SelectionKind::EffectChoice);
    let labels: Vec<String> = zone.effect_choices.as_ref().unwrap().iter().map(|c| c.label.clone()).collect();
    assert_eq!(labels, vec!["From Hand".to_string(), "Don't place any".to_string()], "no trash candidate, no trash option");
    runner.execute_branch(0).expect("from hand");
    select_first_non_pass(&mut runner);
    if runner.pending_selection_view().is_some_and(|p| p.kind != SelectionKind::EffectChoice) {
        // up to 2: end the pick after the only candidate
        runner.game.decode_action(digimon_engine::action::space::PASS, 0);
    }
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

/// CR 4.3 13-1-7 / 4-6-1: a DUAL card checked from security is a Digimon card, so it is a Security Digimon and
/// battles the checking Digimon (13-1-8-3), then goes to the trash (13-1-8-4). Found by playing a sim game back in
/// DCGO, which battles it: our engine revealed Siriusmon and skipped the battle, so the 2000 DP attacker survived.
#[test]
fn ex12_018_checked_from_security_battles_as_a_security_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-018 YAML loads")
        .add_card(plain_digimon("ATTACKER", CardColor::Blue, 3, 2000))
        .security(1, &[CARD_ID])
        .memory(3)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    runner.attack_player(attacker, 1, false);
    let _ = runner.auto_resolve();

    assert!(
        !super::support::field_contains(&runner, 0, "ATTACKER"),
        "2000 DP attacker loses the battle against the 12000 DP Security Digimon"
    );
    assert!(super::support::trash_contains(&runner, 0, "ATTACKER"));
    assert!(super::support::trash_contains(&runner, 1, CARD_ID), "the checked card is trashed (13-1-8-4)");
}
