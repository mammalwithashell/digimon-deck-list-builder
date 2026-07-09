use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::enums::{CardColor, Keyword};
use digimon_engine::selection::SelectionKind;

use super::support::{
    field_contains, fire_when_digivolving, plain_digimon, select_first_non_pass, top_card_id,
    trash_contains, DebugRunner,
};

const CARD_ID: &str = "EX12-017";

#[test]
fn ex12_017_has_security_attack_plus_decode_counter_and_dna_paths() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-017 YAML loads")
        .start();
    let wargreymon = runner.place_on_field(0, CARD_ID, Some(0));
    let card = runner.compiled_card(CARD_ID).expect("compiled EX12-017");

    assert!(runner
        .game
        .has_keyword(wargreymon, Keyword::SecurityAttackPlus(1)));
    assert!(
        card.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::DnaDigivolve
                && path.cost == Some(CompiledCost::Literal(0))
        }),
        "printed Red/Yellow Lv.5 + Black/Purple Lv.5 DNA digivolve path"
    );

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
        "WarGreymon has Decode on its face and inherited text"
    );
    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
            scope: CompiledScope::Inherited,
            ..
        })
    )));

    let counter = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::Counter) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Counter clause exists");
    assert!(counter.once_per_turn);
    assert!(counter.optional);
    assert!(counter
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::SelectHand { .. })));
    assert!(walk_steps(
        &counter.process.iter().collect::<Vec<_>>(),
        |step| { matches!(step, CompiledStep::SelectDnaPair { .. }) }
    ));
    assert!(walk_steps(
        &counter.process.iter().collect::<Vec<_>>(),
        |step| { matches!(step, CompiledStep::EffectInitiatedDnaDigivolve { .. }) }
    ));
    assert!(walk_steps(
        &counter.process.iter().collect::<Vec<_>>(),
        |step| { matches!(step, CompiledStep::RedirectAttackTarget { .. }) }
    ));
}

#[test]
fn ex12_017_on_play_when_digivolving_when_attacking_clause_is_once_per_turn() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-017 YAML loads")
        .start();
    let card = runner.compiled_card(CARD_ID).expect("compiled EX12-017");

    let clause = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnPlay)
                    && triggered.when.contains(&CompiledTiming::WhenDigivolving)
                    && triggered.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("shared OP/WD/WA clause exists");

    assert!(clause.once_per_turn);
    assert!(
        !clause.optional,
        "printed de-digivolve/delete clause is mandatory"
    );
    assert!(clause
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::DeDigivolve { .. })));
}

#[test]
fn ex12_017_when_digivolving_de_digivolves_two_then_deletes_lowest_dp() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-017 YAML loads")
        .add_card(plain_digimon("OPP-BASE", CardColor::Red, 3, 8000))
        .add_card(plain_digimon("OPP-MID", CardColor::Red, 4, 5000))
        .add_card(plain_digimon("OPP-TOP", CardColor::Red, 5, 9000))
        .add_card(plain_digimon("OPP-LOW", CardColor::Red, 3, 1000))
        .memory(5)
        .start();
    let wargreymon = runner.place_on_field(0, CARD_ID, Some(0));
    let target = runner.place_stack(1, &["OPP-BASE", "OPP-MID", "OPP-TOP"]);
    runner.place_on_field(1, "OPP-LOW", Some(0));

    fire_when_digivolving(&mut runner, wargreymon);
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "first prompt chooses the De-Digivolve target"
    );
    select_first_non_pass(&mut runner);
    assert_eq!(
        top_card_id(&runner, 1, target.index as usize),
        "OPP-BASE",
        "De-Digivolve 2 should stop at the level 3 base"
    );

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "then choose the lowest-DP Digimon to delete"
    );
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("finish EX12-017 WD");

    assert!(trash_contains(&runner, 1, "OPP-TOP"));
    assert!(trash_contains(&runner, 1, "OPP-MID"));
    assert!(!field_contains(&runner, 1, "OPP-LOW"));
    assert!(field_contains(&runner, 1, "OPP-BASE"));
}

fn walk_steps<F>(steps: &[&CompiledStep], pred: F) -> bool
where
    F: Fn(&CompiledStep) -> bool + Copy,
{
    for step in steps {
        if pred(step) {
            return true;
        }

        let nested: Vec<&CompiledStep> = match step {
            CompiledStep::If {
                then, else_branch, ..
            } => then.iter().chain(else_branch.iter()).collect(),
            CompiledStep::Optional(body)
            | CompiledStep::AsSelectingPlayer { body, .. }
            | CompiledStep::ForEach { body, .. }
            | CompiledStep::PerSelected { body, .. }
            | CompiledStep::ScheduleDelayed { body, .. } => body.iter().collect(),
            CompiledStep::SelectOwnPermanent { then, .. }
            | CompiledStep::SelectOpponentPermanent { then, .. }
            | CompiledStep::SelectAnyPermanent { then, .. }
            | CompiledStep::SelectHand { then, .. }
            | CompiledStep::SelectTrash { then, .. }
            | CompiledStep::SelectOwnSources { then, .. }
            | CompiledStep::SelectOpponentSources { then, .. }
            | CompiledStep::SelectOpponentDpBudget { then, .. }
            | CompiledStep::SelectOpponentPlayCostBudget { then, .. }
            | CompiledStep::SelectOwnBreedingPermanent { then, .. }
            | CompiledStep::SelectReveal { then, .. }
            | CompiledStep::SelectSecurity { then, .. }
            | CompiledStep::SelectUnionZone { then, .. } => then.iter().collect(),
            CompiledStep::SearchOwnSecurityStack {
                on_select,
                on_no_match,
                ..
            } => on_select
                .iter()
                .chain(on_no_match.iter().flatten())
                .collect(),
            _ => Vec::new(),
        };

        if !nested.is_empty() && walk_steps(&nested, pred) {
            return true;
        }
    }

    false
}
