use digimon_engine::enums::{CardColor, Keyword, ModifierType};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

use super::support::{
    field_contains, hand_contains, plain_digimon, push_to_trash, select_first_non_pass,
    select_hand_card, tb_digimon, DebugRunner,
};

const CARD_ID: &str = "EX12-047";

fn resolve_first_action(runner: &mut DebugRunner) {
    let (player, action) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("selection must be pending");
        (pending.selecting_player, pending.valid_action_ids[0])
    };
    runner
        .game
        .resolve_selection(player, action)
        .expect("selection resolves");
}

#[test]
fn ex12_047_has_printed_keywords() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-047 YAML loads")
        .start();
    let amaterasumon = runner.place_on_field(0, CARD_ID, Some(0));

    assert!(runner.game.has_keyword(amaterasumon, Keyword::Piercing));
    assert!(runner
        .game
        .has_keyword(amaterasumon, Keyword::SecurityAttackPlus(1)));
    assert!(runner.game.has_keyword(amaterasumon, Keyword::Ascension));
}

#[test]
fn ex12_047_on_play_deletes_lowest_dp_returns_two_trash_and_applies_dp_modifiers() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-047 YAML loads")
        .add_card(plain_digimon("LOW", CardColor::Red, 3, 1000))
        .add_card(plain_digimon("DP-TARGET", CardColor::Purple, 4, 15000))
        .add_card(plain_digimon("TRASH-R", CardColor::Red, 3, 3000))
        .add_card(plain_digimon("TRASH-Y", CardColor::Yellow, 3, 3000))
        .start();

    let amaterasumon = runner.place_on_field(0, CARD_ID, Some(0));
    let dp_target = runner.place_on_field(1, "DP-TARGET", Some(0));
    runner.place_on_field(1, "LOW", Some(0));
    push_to_trash(&mut runner, 1, "TRASH-R");
    push_to_trash(&mut runner, 1, "TRASH-Y");
    assert_eq!(
        runner.game.players[1]
            .trash
            .iter()
            .map(|card| card.card_id(&runner.game.card_data).to_string())
            .collect::<Vec<_>>(),
        vec!["TRASH-R".to_string(), "TRASH-Y".to_string()]
    );

    runner.fire_on_play(0, amaterasumon.index as usize);

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "lowest-DP delete prompt should install first"
    );
    resolve_first_action(&mut runner);

    assert!(!field_contains(&runner, 1, "LOW"));
    assert_eq!(
        runner.game.players[1]
            .trash
            .iter()
            .map(|card| card.card_id(&runner.game.card_data).to_string())
            .collect::<Vec<_>>(),
        vec![
            "TRASH-R".to_string(),
            "TRASH-Y".to_string(),
            "LOW".to_string()
        ]
    );
    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::CountCappedMultiSelect {
                max: 2,
                picked: 0,
                ..
            })
        ),
        "return-2 trash multi-select should follow the delete; got {:?}",
        runner.pending_kind()
    );
    resolve_first_action(&mut runner);
    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::CountCappedMultiSelect {
                max: 2,
                picked: 1,
                ..
            })
        ),
        "return-2 trash multi-select should keep accumulating after one pick; got {:?}",
        runner.pending_kind()
    );
    assert_eq!(
        runner.game.players[1]
            .trash
            .iter()
            .map(|card| card.card_id(&runner.game.card_data).to_string())
            .collect::<Vec<_>>(),
        vec![
            "TRASH-R".to_string(),
            "TRASH-Y".to_string(),
            "LOW".to_string()
        ],
        "multi-pick should not move trash cards before the max is reached"
    );
    resolve_first_action(&mut runner);

    assert_eq!(
        runner.game.players[1]
            .deck
            .iter()
            .map(|card| card.card_id(&runner.game.card_data).to_string())
            .collect::<Vec<_>>(),
        vec!["TRASH-Y".to_string(), "TRASH-R".to_string()],
        "the two selected trash cards should move to opponent deck before the DP target prompt"
    );
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "DP reduction target prompt should install after returning two trash cards"
    );
    resolve_first_action(&mut runner);

    runner.auto_resolve().expect("resolve EX12-047 On Play");

    assert!(
        !field_contains(&runner, 1, "LOW"),
        "the selected lowest-DP Digimon should leave the field"
    );
    assert_eq!(
        runner.game.players[1]
            .deck
            .iter()
            .map(|card| card.card_id(&runner.game.card_data).to_string())
            .collect::<Vec<_>>(),
        vec!["TRASH-Y".to_string(), "TRASH-R".to_string()],
        "the two selected trash cards should remain on the bottom of the opponent's deck"
    );
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(amaterasumon, ModifierType::ChangeDp),
        6000
    );
    assert_eq!(
        runner.game.modifiers.sum(dp_target, ModifierType::ChangeDp),
        -10000,
        "two distinct returned colors should apply -10000 DP"
    );
}

#[test]
fn ex12_047_on_deletion_returns_tb_from_trash_and_plays_level5_tb_from_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-047 YAML loads")
        .add_card(tb_digimon("TB-RETURN", CardColor::Yellow, 4, 5000))
        .add_card(tb_digimon("TB-PLAY", CardColor::Yellow, 5, 7000))
        .hand(0, &["TB-PLAY"])
        .memory(10)
        .start();
    push_to_trash(&mut runner, 0, "TB-RETURN");

    let amaterasumon = runner.place_on_field(0, CARD_ID, Some(0));
    runner
        .game
        .delete_permanents_batch(vec![amaterasumon], ReplacementCause::OpponentEffect);

    select_first_non_pass(&mut runner);
    select_hand_card(&mut runner, 0, "TB-PLAY");
    runner.auto_resolve().expect("resolve EX12-047 On Deletion");

    assert!(hand_contains(&runner, 0, "TB-RETURN"));
    assert!(field_contains(&runner, 0, "TB-PLAY"));
}
