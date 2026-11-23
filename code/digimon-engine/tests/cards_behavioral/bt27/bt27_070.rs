use super::board;
use digimon_engine::enums::Keyword;
use digimon_engine::replacement::ReplacementCause;

#[test]
fn bt27_070_can_delete_all_tied_lowest_dp_digimon() {
    let mut r = board("BT27-070");
    let craniamon = r.place_on_field(0, "BT27-070", Some(0));
    r.place_on_field(1, "FILLER", Some(0));
    r.place_on_field(1, "FILLER", Some(0));
    assert!(r.game.has_keyword(craniamon, Keyword::Blocker));
    r.game.suspend(craniamon);
    let pick = r
        .pending_selection_view()
        .expect("Choose whether to delete");
    r.execute_action(0, pick.valid_action_ids[0]).unwrap();
    assert_eq!(r.battle_area_size(1), 0);
}

#[test]
fn bt27_070_suspends_to_protect_every_digimon_in_one_deletion_batch() {
    let mut r = board("BT27-070");
    let craniamon = r.place_on_field(0, "BT27-070", Some(0));
    let ally = r.place_on_field(0, "FILLER", Some(0));
    r.game.delete_permanents_batch(vec![craniamon, ally], ReplacementCause::OpponentEffect);
    assert!(r.pending_is_optional());
    r.accept_optional_trigger().unwrap();
    let decision = r.pending_selection_view().expect("Suspension triggered Craniamon");
    let decline = decision.effect_choices.unwrap().into_iter()
        .find(|effect| effect.label == "Do not delete").unwrap();
    r.execute_action(0, decline.action_id).unwrap();
    assert_eq!(r.battle_area_size(0), 2);
    assert!(r.game.players[0].battle_area[0].is_suspended);
}
