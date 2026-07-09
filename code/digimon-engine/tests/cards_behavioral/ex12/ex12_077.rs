use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledStep, CompiledTiming};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::make_test_card;
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::selection::SelectionKind;

use super::support::{
    field_contains, fire_when_digivolving, push_to_trash, select_first_non_pass, vb_digimon,
    vb_text_digimon, DebugRunner,
};

const CARD_ID: &str = "EX12-077";

fn drain_trigger_order(runner: &mut DebugRunner) {
    if runner.pending_kind() == Some(SelectionKind::TriggerOrder) {
        select_first_non_pass(runner);
    }
}

fn decline_source_multi_if_pending(runner: &mut DebugRunner) {
    drain_trigger_order(runner);
    if matches!(
        runner.pending_kind(),
        Some(SelectionKind::SourceMulti { .. })
    ) {
        let view = runner
            .pending_selection_view()
            .expect("SourceMulti prompt should be pending");
        runner
            .execute_action(view.selecting_player, PASS)
            .expect("decline source play/use prompt");
    }
}

fn tamer(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.play_cost = 3;
    card.colors = vec![CardColor::Yellow];
    card
}

fn source_ids(runner: &DebugRunner, player: u8, field_index: usize) -> Vec<String> {
    runner.game.players[player as usize].battle_area[field_index]
        .card_sources
        .iter()
        .map(|source| source.card_id(&runner.game.card_data).to_string())
        .collect()
}

#[test]
fn ex12_077_has_keywords_paths_and_source_play_or_use_clause() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-077 YAML loads")
        .start();
    let prox = runner.place_on_field(0, CARD_ID, Some(0));
    let card = runner.compiled_card(CARD_ID).expect("compiled EX12-077");

    assert!(runner.game.has_keyword(prox, Keyword::Blocker));
    assert!(runner
        .game
        .has_keyword(prox, Keyword::SecurityAttackPlus(1)));
    assert!(
        card.alt_paths
            .iter()
            .any(|path| path.kind == CompiledAltPathKind::DnaDigivolve),
        "Proximamon must have the printed DNA digivolve path"
    );
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::Counter)
                    && t.process.iter().any(|step| matches!(step, CompiledStep::SelectOwnSources { .. }))
        )),
        "play/use-from-sources clause must be present on all printed timings"
    );
}

#[test]
fn ex12_077_on_play_places_two_cards_then_deletes_opponent_tamer() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-077 YAML loads")
        .add_card(vb_digimon("HAND-FUEL", CardColor::Red, 4, 5000))
        .add_card(vb_text_digimon("TRASH-FUEL", CardColor::Yellow, 4, 5000))
        .add_card(tamer("OPP-TAMER"))
        .hand(0, &["HAND-FUEL"])
        .memory(10)
        .start();
    push_to_trash(&mut runner, 0, "TRASH-FUEL");
    let prox = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-TAMER", Some(0));

    fire_when_digivolving(&mut runner, prox);
    drain_trigger_order(&mut runner);
    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::UnionZone { .. })
    ));
    select_first_non_pass(&mut runner);

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField),
        "the cost chooses which Digimon receives the sources"
    );
    select_first_non_pass(&mut runner);
    runner
        .execute_branch(1)
        .expect("place first as bottom source");

    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::UnionZone { .. })
    ));
    select_first_non_pass(&mut runner);
    runner
        .execute_branch(0)
        .expect("place second as top source");

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "after both source placements, choose the delete target"
    );
    select_first_non_pass(&mut runner);
    decline_source_multi_if_pending(&mut runner);

    let sources = source_ids(&runner, 0, prox.index as usize);
    assert!(sources.contains(&"HAND-FUEL".to_string()));
    assert!(sources.contains(&"TRASH-FUEL".to_string()));
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "opponent Tamer should be deleted"
    );
}

#[test]
fn ex12_077_when_digivolving_plays_cost_10_vb_source_free() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-077 YAML loads")
        .add_card(vb_digimon("HOST", CardColor::Red, 5, 8000))
        .add_card(vb_digimon("SRC-DIGI", CardColor::Yellow, 4, 5000))
        .memory(10)
        .start();
    let host = runner.place_stack(0, &["SRC-DIGI", "HOST"]);
    let prox = runner.place_on_field(0, CARD_ID, Some(0));
    let field_before = runner.battle_area_size(0);

    fire_when_digivolving(&mut runner, prox);
    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::SourceMulti { .. })
        ),
        "source play/use prompt must be offered"
    );
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("resolve source play");

    assert_eq!(runner.battle_area_size(0), field_before + 1);
    assert!(field_contains(&runner, 0, "SRC-DIGI"));
    assert_eq!(
        source_ids(&runner, 0, host.index as usize),
        vec!["HOST".to_string()],
        "played source should leave its original stack"
    );
}
