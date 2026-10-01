use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::enums::CardColor;
use digimon_engine::selection::SelectionKind;

use super::support::{
    bottom_source_id, fire_when_digivolving, select_first_non_pass, vb_digimon, vb_text_digimon,
    DebugRunner,
};

const CARD_ID: &str = "EX12-014";

#[test]
fn ex12_014_has_decode_on_both_faces_and_union_source_attack_clause() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-014 YAML loads")
        .start();
    let card = runner.compiled_card(CARD_ID).expect("compiled EX12-014");

    let replacement_count = card
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
        replacement_count, 2,
        "Canoweissmon has Decode on its face and inherited text"
    );
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
                scope: CompiledScope::Inherited,
                ..
            })
        )),
        "inherited Decode must lower as an inherited replacement"
    );

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
        .expect("On Play/When Digivolving placement clause exists");

    assert!(
        clause
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::SelectUnionZone { .. })),
        "placement must choose from hand/trash with an optional union-zone prompt"
    );
}

#[test]
fn ex12_014_when_digivolving_places_source_then_offers_any_own_digimon_attack() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-014 YAML loads")
        .add_card(vb_digimon("VB-FUEL", CardColor::Red, 4, 5000))
        .add_card(vb_text_digimon("ATTACKER", CardColor::Yellow, 4, 6000))
        .add_card(vb_digimon("SECURITY", CardColor::Red, 3, 2000))
        .hand(0, &["VB-FUEL"])
        .security(1, &["SECURITY"])
        .memory(5)
        .start();
    runner.game.turn_count = 1;

    let canoweiss = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "ATTACKER", Some(0));
    let security_before = runner.game.player(1).security.len();

    fire_when_digivolving(&mut runner, canoweiss);
    let place_prompt = runner
        .pending_selection_view()
        .expect("source placement prompt should be pending");
    assert!(
        matches!(place_prompt.kind, SelectionKind::UnionZone { .. }),
        "must select the source card from hand/trash"
    );
    assert!(
        place_prompt.is_optional,
        "printed 'may place' must allow declining the source placement"
    );
    select_first_non_pass(&mut runner);

    assert_eq!(
        bottom_source_id(&runner, 0, canoweiss.index as usize),
        "VB-FUEL",
        "selected VB card is placed as Canoweissmon's bottom source"
    );

    let choose_attacker = runner
        .pending_selection_view()
        .expect("placing a source should offer an own-Digimon attack choice");
    assert_eq!(
        choose_attacker.kind,
        SelectionKind::OwnField,
        "printed '1 of your Digimon may attack' first chooses the attacker"
    );
    assert!(
        choose_attacker.is_optional,
        "the follow-up attacker choice is optional"
    );
    select_first_non_pass(&mut runner);

    let target_prompt = runner
        .pending_selection_view()
        .expect("accepted attacker should choose an attack target");
    assert_eq!(
        target_prompt.kind,
        SelectionKind::Target,
        "accepted attacker should enter a normal attack-target prompt"
    );
    select_first_non_pass(&mut runner);

    assert_eq!(
        runner.game.player(1).security.len(),
        security_before - 1,
        "accepting the follow-up attack should perform a security check"
    );
}

#[test]
fn ex12_014_declining_source_placement_still_offers_attack() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-014 YAML loads")
        .add_card(vb_digimon("VB-FUEL", CardColor::Red, 4, 5000))
        .add_card(vb_text_digimon("ATTACKER", CardColor::Yellow, 4, 6000))
        .hand(0, &["VB-FUEL"])
        .memory(5)
        .start();
    runner.game.turn_count = 1;

    let canoweiss = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "ATTACKER", Some(0));

    fire_when_digivolving(&mut runner, canoweiss);
    let place_prompt = runner
        .pending_selection_view()
        .expect("source placement prompt should be pending");
    assert!(matches!(place_prompt.kind, SelectionKind::UnionZone { .. }));
    assert!(place_prompt.is_optional);
    runner
        .execute_action(place_prompt.selecting_player, PASS)
        .expect("decline source placement");

    let choose_attacker = runner
        .pending_selection_view()
        .expect("declining source placement should still offer the then-attack");
    assert_eq!(
        choose_attacker.kind,
        SelectionKind::OwnField,
        "the printed attack is after 'Then' and is not conditional on placing a source"
    );
    assert!(choose_attacker.is_optional);
}

// ─── G-DSL-CAN-ATTACK-PREDICATE (2026-10-01) ─────────────────────────────────
//
// DCGO EX12_014.cs `IsYourDigimon`: `IsPermanentExistsOnOwnerBattleAreaDigimon
// && permanent.CanAttack(activateClass)`, and the select is skipped when
// `HasMatchConditionOwnersPermanent` is false.

fn can_attack_skip_placement(runner: &mut DebugRunner) {
    if let Some(v) = runner.pending_selection_view() {
        if matches!(v.kind, SelectionKind::UnionZone { .. }) {
            runner
                .execute_action(v.selecting_player, PASS)
                .expect("decline placement");
        }
    }
}

#[test]
fn ex12_014_attacker_pick_excludes_digimon_that_cannot_attack() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-014 YAML loads")
        .add_card(vb_text_digimon("ATTACKER", CardColor::Yellow, 4, 6000))
        .add_card(vb_text_digimon("TIRED", CardColor::Yellow, 4, 6000))
        .memory(5)
        .start();
    runner.game.turn_count = 1;

    // Canoweissmon is an established Digimon here: it can attack.
    let canoweiss = runner.place_on_field(0, CARD_ID, Some(0));
    let tired = runner.place_on_field(0, "TIRED", Some(0));
    runner.game.players[0].battle_area[tired.index as usize].is_suspended = true;
    let fresh = runner.place_on_field(0, "ATTACKER", None); // summoning sickness

    fire_when_digivolving(&mut runner, canoweiss);
    can_attack_skip_placement(&mut runner);
    let prompt = runner
        .pending_selection_view()
        .expect("Canoweissmon can attack, so the attacker prompt is shown");
    assert_eq!(prompt.kind, SelectionKind::OwnField);
    let offered: Vec<u16> = prompt
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    let slot = |h: digimon_engine::permanent::PermanentHandle| {
        digimon_engine::action::space::encode_attack(0, h.index as u16)
    };
    assert!(offered.contains(&slot(canoweiss)), "{offered:?}");
    assert!(!offered.contains(&slot(tired)), "a suspended Digimon cannot attack");
    assert!(
        !offered.contains(&slot(fresh)),
        "a summoning-sick Digimon cannot attack"
    );
}

#[test]
fn ex12_014_no_attacker_prompt_when_no_digimon_can_attack() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-014 YAML loads")
        .add_card(vb_text_digimon("TIRED", CardColor::Yellow, 4, 6000))
        .memory(5)
        .start();
    runner.game.turn_count = 1;

    let canoweiss = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.players[0].battle_area[canoweiss.index as usize].is_suspended = true;
    let tired = runner.place_on_field(0, "TIRED", Some(0));
    runner.game.players[0].battle_area[tired.index as usize].is_suspended = true;

    fire_when_digivolving(&mut runner, canoweiss);
    can_attack_skip_placement(&mut runner);
    assert!(
        runner.pending_selection_view().is_none(),
        "no Digimon can attack => no (dead) attacker prompt"
    );
}
