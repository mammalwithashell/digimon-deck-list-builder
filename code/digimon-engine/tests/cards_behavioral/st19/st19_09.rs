//! ST19-09 Pandamon.
//! Printed text covered here: <Blocker>. [On Deletion] You may play 1 level 3
//! Puppet Digimon card from your hand without paying the cost.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::Keyword;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

#[test]
fn st19_09_has_blocker_while_face_up() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-09")
        .expect("ST19-09 YAML loads")
        .start();
    let panda = runner.place_on_field(0, "ST19-09", Some(0));

    assert!(
        runner.game.has_keyword(panda, Keyword::Blocker),
        "ST19-09 has printed Blocker"
    );
}

#[test]
fn st19_09_on_deletion_may_play_level3_puppet_from_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-09")
        .expect("ST19-09 YAML loads")
        .add_card(make_puppet_level3("PUPPET-L3"))
        .add_card(make_puppet_level4("PUPPET-L4"))
        .hand(0, &["PUPPET-L3", "PUPPET-L4"])
        .start();
    let panda = runner.place_on_field(0, "ST19-09", Some(0));

    runner
        .game
        .delete_permanent_with_cause(panda, ReplacementCause::OpponentEffect);

    assert!(runner.pending_is_optional(), "printed play effect says may");
    let view = runner
        .pending_selection_view()
        .expect("level 3 Puppet prompt");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the level 3 Puppet is listed as a candidate"
    );
    let play_action = view.valid_action_ids[0];

    runner
        .execute_action(0, play_action)
        .expect("choose level 3 Puppet");
    runner.auto_resolve().expect("finish play");

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "PUPPET-L3"),
        "selected level 3 Puppet enters battle area"
    );
    let hand_ids = runner.game.players[0]
        .hand
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect::<Vec<_>>();
    assert_eq!(hand_ids, vec!["PUPPET-L4".to_string()]);
}

fn make_puppet_level3(id: &str) -> digimon_engine::card_data::CardData {
    make_puppet(id, 3)
}

fn make_puppet_level4(id: &str) -> digimon_engine::card_data::CardData {
    make_puppet(id, 4)
}

fn make_puppet(id: &str, level: u8) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.level = Some(level);
    card.traits = vec!["Puppet".to_string()];
    card
}
