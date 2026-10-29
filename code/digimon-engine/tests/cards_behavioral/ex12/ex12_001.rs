use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledStep, CompiledTiming};
use digimon_engine::debug_runner::make_test_dna_card;
use digimon_engine::enums::CardColor;
use digimon_engine::selection::SelectionKind;

use super::support::{select_first_non_pass, select_hand_card, vb_digimon, DebugRunner};

const CARD_ID: &str = "EX12-001";

fn dna_fixture(has_result: bool, partner_first: bool) -> DebugRunner {
    let mut target = make_test_dna_card("VB-DNA", "VB DNA Result", 3, 3, 0);
    target.traits = vec!["VB".to_string()];
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .unwrap()
        .add_card(vb_digimon("CARRIER", CardColor::Yellow, 3, 3000))
        .add_card(vb_digimon("PARTNER", CardColor::Red, 3, 3000))
        .add_card(target)
        .hand(0, if has_result { &["VB-DNA"] } else { &[] })
        .memory(3)
        .start();
    runner.game.turn_count = 1;
    if partner_first {
        runner.place_on_field(0, "PARTNER", Some(0));
    }
    runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    if !partner_first {
        runner.place_on_field(0, "PARTNER", Some(0));
    }
    runner
}

#[test]
fn ex12_001_no_dna_target_must_not_offer_attack() {
    let mut runner = dna_fixture(false, false);
    runner.game.fire_end_of_your_turn(0);
    assert!(
        runner.pending_selection_view().is_none(),
        "No successful DNA means no attack offer"
    );
}

#[test]
fn ex12_001_no_partner_must_not_offer_attack() {
    let mut runner = dna_fixture(true, false);
    runner.game.player_mut(0).battle_area.truncate(1);
    runner.game.fire_end_of_your_turn(0);
    assert!(runner.pending_selection_view().is_none());
}

#[test]
fn ex12_001_rejected_dna_commit_must_not_offer_attack() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let mut runner = dna_fixture(true, false);
    runner.game.fire_end_of_your_turn(0);
    select_first_non_pass(&mut runner);
    runner.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(
            ModifierType::CannotDigivolveDigimonByEffect,
            0,
            Expiry::Permanent,
            None,
            0,
        ),
    );
    runner
        .execute_action(0, digimon_engine::action::space::PLAY_HAND_START)
        .unwrap();
    assert!(runner.pending_selection_view().is_none());
    assert_eq!(runner.game.player(0).battle_area.len(), 2);
}

#[test]
fn ex12_001_declining_partner_or_result_must_not_offer_attack() {
    for decline_result in [false, true] {
        let mut runner = dna_fixture(true, false);
        runner.game.fire_end_of_your_turn(0);
        if decline_result {
            select_first_non_pass(&mut runner);
        }
        runner
            .execute_action(0, digimon_engine::action::space::PASS)
            .unwrap();
        assert!(
            runner.pending_selection_view().is_none(),
            "Declined DNA must not offer an attack"
        );
    }
}

#[test]
fn ex12_001_invalidated_result_must_not_offer_attack() {
    let mut runner = dna_fixture(true, false);
    runner.game.fire_end_of_your_turn(0);
    select_first_non_pass(&mut runner);
    // The result can become unavailable after the prompt was installed.
    runner.game.player_mut(0).hand.clear();
    runner
        .execute_action(0, digimon_engine::action::space::PLAY_HAND_START)
        .unwrap();
    assert!(
        runner.pending_selection_view().is_none(),
        "A failed commit must not earn an attack"
    );
}

#[test]
fn ex12_001_result_attack_survives_compaction_and_cloning() {
    let mut runner = dna_fixture(true, true);
    runner.game.fire_end_of_your_turn(0);
    select_first_non_pass(&mut runner);
    // Search clones the game while the DNA result choice is pending.
    runner.game = runner.game.clone();
    select_hand_card(&mut runner, 0, "VB-DNA");
    let context = digimon_engine::evaluation_context::decision_context(&runner.game);
    assert_eq!(context["effects"][0]["stage"], "followup_attack");
    assert_eq!(context["effects"][0]["attacker_slot"], 0);
    assert_eq!(
        runner.game.player(0).battle_area[0]
            .top_card()
            .card_id(&runner.game.card_data),
        "VB-DNA"
    );
}

#[test]
fn ex12_001_has_inherited_eot_vb_dna_then_may_attack_steps() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-001 YAML loads")
        .start();
    let card = runner.compiled_card(CARD_ID).expect("compiled EX12-001");

    let clause = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::EndOfYourTurn)
                    && t.optional =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("Nyaromon must expose inherited optional EOT DNA digivolve");

    assert!(
        clause.process.iter().any(|step| matches!(
            step,
            CompiledStep::MayDnaDigivolveNow {
                ignore_requirements: false,
                ..
            }
        )),
        "EOT inherited body must use MayDnaDigivolveNow and honor printed DNA requirements"
    );
    assert!(
        clause
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::If { condition, then: then_steps, .. }
                if condition.effect_digivolved_any_digimon == Some(true)
                    && then_steps.iter().any(|s| matches!(s, CompiledStep::MayAttackNow { optional: true, .. })))),
        "The optional attack must be guarded by this effect's completed DNA evolution"
    );
}

#[test]
fn ex12_001_inherited_eot_dna_then_result_may_attack() {
    let mut target = make_test_dna_card("VB-DNA", "VB DNA Result", 3, 3, 0);
    target.traits = vec!["VB".to_string()];

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-001 YAML loads")
        .add_card(vb_digimon("CARRIER", CardColor::Yellow, 3, 3000))
        .add_card(vb_digimon("PARTNER", CardColor::Red, 3, 3000))
        .add_card(target)
        .add_card(vb_digimon("SECURITY", CardColor::Red, 3, 2000))
        .hand(0, &["VB-DNA"])
        .security(1, &["SECURITY"])
        .memory(3)
        .start();
    runner.game.turn_count = 1;

    runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    runner.place_on_field(0, "PARTNER", Some(0));
    let security_before = runner.game.player(1).security.len();

    runner.game.fire_end_of_your_turn(0);
    select_hand_card(&mut runner, 0, "VB-DNA");

    let top_ids: Vec<_> = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        top_ids.iter().any(|id| id == "VB-DNA"),
        "DNA target must become the top card after the EOT DNA selection; field={top_ids:?}"
    );

    let attack_prompt = runner
        .pending_selection_view()
        .expect("after DNA, the DNA-digivolved Digimon should get a may-attack prompt");
    assert_eq!(
        attack_prompt.kind,
        SelectionKind::Target,
        "follow-up must be a fixed-attacker attack-target prompt for the DNA result"
    );
    assert!(
        attack_prompt.is_optional,
        "printed 'may attack' must leave the attack optional"
    );
    select_first_non_pass(&mut runner);

    assert_eq!(
        runner.game.player(1).security.len(),
        security_before - 1,
        "accepting the follow-up attack should run the normal security attack flow"
    );
}
