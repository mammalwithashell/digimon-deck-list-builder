//! BT24-034 Aegiomon.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledCost, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT24-034";

#[test]
fn bt24_034_yaml_metadata_keywords_alt_paths_and_timings_match_printed_card() {
    let runner = aegiomon_runner().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled BT24-034");

    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(4));
    assert_eq!(card.color, vec![CompiledColor::Yellow]);
    assert_eq!(card.cost, Some(5));
    assert_eq!(card.dp, Some(5000));
    for trait_name in ["Shaman", "Iliad", "TS"] {
        assert!(card.traits.contains(&trait_name.to_string()));
    }
    assert!(
        card.alt_paths
            .iter()
            .any(|path| path.cost == Some(CompiledCost::Literal(2))
                && path
                    .from
                    .as_ref()
                    .is_some_and(|from| from.color_is == Some(CompiledColor::Yellow))),
        "Aegiomon must digivolve from yellow level 3 for cost 2"
    );
    assert!(
        card.alt_paths
            .iter()
            .any(|path| path.cost == Some(CompiledCost::Literal(2))
                && path.from.as_ref().is_some_and(|from| from
                    .all_of
                    .iter()
                    .any(|p| p.trait_has.as_deref() == Some("TS")))),
        "Aegiomon must digivolve from level 3 TS for cost 2"
    );

    let keyword_count = card
        .effects
        .iter()
        .filter(|clause| matches!(clause, CompiledClause::Declarative(_)))
        .count();
    assert_eq!(keyword_count, 2, "face and inherited Barrier grants");

    let trigger = triggered(&card.effects);
    assert_eq!(
        trigger.when,
        vec![
            CompiledTiming::OnMove,
            CompiledTiming::OnPlay,
            CompiledTiming::WhenDigivolving
        ]
    );
    assert!(trigger.condition.is_some());
    assert!(matches!(
        trigger.process.first(),
        Some(CompiledStep::SelectEffectChoice { .. })
    ));
}

#[test]
fn bt24_034_barrier_is_active_on_field_and_as_inherited() {
    let mut runner = aegiomon_runner()
        .add_card(carrier("CARRIER"))
        .memory(0)
        .start();

    let face = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(runner.game.has_keyword(face, Keyword::Barrier));

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    assert!(runner.game.has_keyword(carrier, Keyword::Barrier));
}

#[test]
fn bt24_034_on_play_adds_top_security_and_plays_only_ts_tamer_without_field_name_match() {
    let mut runner = aegiomon_runner()
        .add_card(ts_tamer("TS-TAMER", "Fresh TS Tamer"))
        .add_card(ts_tamer("DUP-TAMER-HAND", "Duplicate Tamer"))
        .add_card(non_ts_tamer("NON-TS", "Plain Tamer"))
        .add_card(ts_tamer("DUP-TAMER-FIELD", "Duplicate Tamer"))
        .add_card(filler("SEC"))
        .hand(0, &[CARD_ID, "TS-TAMER", "DUP-TAMER-HAND", "NON-TS"])
        .security(0, &["SEC"])
        .memory(10)
        .start();
    runner.place_on_field(0, "DUP-TAMER-FIELD", Some(0));
    let security_before = runner.security_count(0);
    let hand_before = runner.hand_size(0);

    runner.play(0, 0).expect("play Aegiomon");
    assert_eq!(runner.pending_kind(), Some(SelectionKind::EffectChoice));
    runner
        .execute_branch(0)
        .expect("choose play TS Tamer branch");

    assert_eq!(
        runner.security_count(0),
        security_before - 1,
        "top security is added to hand as the cost"
    );
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    let valid = runner
        .pending_selection()
        .expect("TS Tamer hand selection")
        .valid_action_ids
        .clone();
    assert_eq!(valid, vec![hand_action_for_id(&runner, "TS-TAMER")]);

    runner
        .execute_action(0, valid[0])
        .expect("play eligible TS Tamer");
    runner.auto_resolve().expect("settle Aegiomon effect");

    assert!(
        runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "TS-TAMER"),
        "eligible TS Tamer must be played"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before - 1,
        "played Aegiomon and Tamer leave hand, security card enters hand"
    );
}

#[test]
fn bt24_034_on_play_decline_branch_does_not_take_security_cost() {
    let mut runner = aegiomon_runner()
        .add_card(ts_tamer("TS-TAMER", "Fresh TS Tamer"))
        .add_card(filler("SEC"))
        .hand(0, &[CARD_ID, "TS-TAMER"])
        .security(0, &["SEC"])
        .memory(10)
        .start();
    let security_before = runner.security_count(0);

    runner.play(0, 0).expect("play Aegiomon");
    assert_eq!(runner.pending_kind(), Some(SelectionKind::EffectChoice));
    runner.execute_branch(1).expect("decline Aegiomon effect");
    runner.auto_resolve().expect("settle decline");

    assert_eq!(
        runner.security_count(0),
        security_before,
        "declining must not add top security to hand"
    );
}

fn aegiomon_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-034 YAML loads")
}

fn triggered(effects: &[CompiledClause]) -> &digimon_dsl::compiled::CompiledTriggeredClause {
    effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("triggered clause exists")
}

fn ts_tamer(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![CardColor::Yellow];
    card.traits = vec!["TS".to_string()];
    card.dp = None;
    card.level = None;
    card.play_cost = 3;
    card
}

fn non_ts_tamer(id: &str, name: &str) -> CardData {
    let mut card = ts_tamer(id, name);
    card.traits.clear();
    card
}

fn carrier(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Yellow];
    card.dp = Some(6000);
    card.level = Some(5);
    card
}

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn hand_action_for_id(runner: &DebugRunner, id: &str) -> u16 {
    runner
        .game
        .player(0)
        .hand
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == id).then_some(PLAY_HAND_START + idx as u16)
        })
        .expect("hand card exists")
}
