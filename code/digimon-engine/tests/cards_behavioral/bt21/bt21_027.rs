use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::replacement::ReplacementCause;

const CARD_ID: &str = "BT21-027";

fn trait_digimon(card_id: &str, name: &str, trait_name: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, name, 4);
    card.colors = vec![CardColor::Red];
    card.traits = vec![trait_name.to_string()];
    card
}

fn plain_digimon(card_id: &str) -> CardData {
    make_test_card_with_level(card_id, card_id, 4)
}

fn tamer(card_id: &str) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.play_cost = 3;
    card
}

#[test]
fn bt21_027_leave_battle_rescues_up_to_four_xros_heart_or_blue_flare_sources_under_tamer() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-027 YAML loads")
        .add_card(trait_digimon("XH-SOURCE", "Xros Source", "Xros Heart"))
        .add_card(trait_digimon(
            "BF-SOURCE",
            "Blue Flare Source",
            "Blue Flare",
        ))
        .add_card(plain_digimon("PLAIN-SOURCE"))
        .add_card(tamer("TAMER"))
        .memory(10)
        .start();

    let dx = runner.place_stack(0, &["XH-SOURCE", "BF-SOURCE", "PLAIN-SOURCE", CARD_ID]);
    runner.place_on_field(0, "TAMER", Some(0));

    runner
        .game
        .delete_permanent_with_cause(dx, ReplacementCause::OpponentEffect);

    let destination_prompt = runner
        .pending_selection()
        .expect("BT21-027 should ask which Tamer receives rescued sources");
    assert!(
        destination_prompt.is_optional,
        "BT21-027 says 'you may', so the rescue prompt must be optional"
    );
    let destination = destination_prompt.valid_action_ids[0];
    runner
        .execute_action(0, destination)
        .expect("choose destination Tamer");

    for _ in 0..4 {
        let Some(prompt) = runner.pending_selection() else {
            break;
        };
        let Some(action) = prompt
            .valid_action_ids
            .iter()
            .copied()
            .find(|action| *action != digimon_engine::action::space::PASS)
        else {
            break;
        };
        runner
            .execute_action(0, action)
            .expect("select trait-filtered source to rescue");
    }

    let tamer = runner.game.players[0]
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "TAMER")
        .expect("destination Tamer remains on field");
    let source_ids: Vec<_> = tamer
        .card_sources
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(source_ids.iter().any(|id| id == "XH-SOURCE"));
    assert!(source_ids.iter().any(|id| id == "BF-SOURCE"));
    assert!(
        !source_ids.iter().any(|id| id == "PLAIN-SOURCE"),
        "BT21-027 rescue must use Xros Heart/Blue Flare trait filters, not a broad source sweep"
    );
}
