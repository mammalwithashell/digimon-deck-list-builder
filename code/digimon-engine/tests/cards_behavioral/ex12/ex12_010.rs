use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause,
    CompiledPredicate, CompiledScope,
};
use digimon_engine::enums::{CardColor, Keyword};
use digimon_engine::selection::SelectionKind;

use super::support::{
    fire_when_digivolving, hand_contains, hand_index, plain_digimon, push_to_trash,
    select_first_non_pass, trash_contains, vb_digimon, DebugRunner,
};

const CARD_ID: &str = "EX12-010";

fn pred_any<F: Fn(&CompiledPredicate) -> bool + Copy>(p: &CompiledPredicate, f: F) -> bool {
    f(p) || p.all_of.iter().any(|q| pred_any(q, f))
        || p.any_of.iter().any(|q| pred_any(q, f))
        || p.none_of.iter().any(|q| pred_any(q, f))
        || p.not.as_ref().is_some_and(|q| pred_any(q, f))
}

#[test]
fn ex12_010_has_raid_and_special_agumon_me_vb_path() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-010 YAML loads")
        .add_card(plain_digimon("CARRIER", CardColor::Red, 4, 5000))
        .start();
    let greymon = runner.place_on_field(0, CARD_ID, Some(0));
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let card = runner.compiled_card(CARD_ID).expect("compiled EX12-010");

    assert!(runner.game.has_keyword(greymon, Keyword::Raid));
    assert!(
        runner.game.has_keyword(carrier, Keyword::Raid),
        "inherited <Raid> must grant Raid to the carrier"
    );
    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            scope: CompiledScope::Inherited,
            keyword,
            ..
        }) if keyword == "Raid"
    )));

    let special = card
        .alt_paths
        .iter()
        .find(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(2))
        })
        .expect("special Lv.3 Agumon/ME/VB digivolve path");
    let from = special
        .from
        .as_ref()
        .expect("special path has a from filter");
    assert!(pred_any(from, |p| p.level_eq == Some(3)));
    assert!(pred_any(from, |p| p.name_contains.as_deref() == Some("Agumon")));
    assert!(pred_any(from, |p| p.trait_has.as_deref() == Some("ME")));
    assert!(pred_any(from, |p| p.trait_has.as_deref() == Some("VB")));
}

#[test]
fn ex12_010_on_play_returns_one_greymon_me_or_vb_digimon_from_trash() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-010 YAML loads")
        .add_card(vb_digimon("VB-FUEL", CardColor::Red, 3, 3000))
        .add_card(plain_digimon("PLAIN", CardColor::Red, 3, 3000))
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    push_to_trash(&mut runner, 0, "VB-FUEL");
    push_to_trash(&mut runner, 0, "PLAIN");

    let play_slot = hand_index(&runner, 0, CARD_ID);
    runner.play(0, play_slot).expect("play EX12-010");

    let prompt = runner
        .pending_selection_view()
        .expect("trash recursion prompt");
    assert_eq!(prompt.kind, SelectionKind::Trash);
    assert!(
        prompt.is_optional,
        "printed 'you may return' must expose PASS"
    );
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("finish EX12-010 On Play");

    assert!(hand_contains(&runner, 0, "VB-FUEL"));
    assert!(trash_contains(&runner, 0, "PLAIN"));
}

#[test]
fn ex12_010_when_digivolving_returns_from_trash() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-010 YAML loads")
        .add_card(vb_digimon("VB-FUEL", CardColor::Black, 3, 3000))
        .memory(5)
        .start();
    push_to_trash(&mut runner, 0, "VB-FUEL");
    let greymon = runner.place_on_field(0, CARD_ID, Some(0));

    fire_when_digivolving(&mut runner, greymon);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Trash));
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("finish EX12-010 WD");

    assert!(hand_contains(&runner, 0, "VB-FUEL"));
}
