#[test]
fn bt27_068_ally_leaving_activates_the_option_face_without_using_it() {
    use digimon_engine::enums::EffectSourceKind;

    let mut r = board("BT27-068");
    r.place_on_field(0, "BT27-068", Some(0));
    let ally = r.place_on_field(0, "FILLER", Some(0));
    r.place_on_field(1, "LEVEL4", Some(0));
    r.game.delete_permanent_with_effects(ally);
    r.accept_optional_trigger().unwrap();
    let pick = r.pending_selection_view().expect("Delete a lowest-play-cost Digimon");
    assert_eq!(pick.source_kind, EffectSourceKind::Option);
    resolve_only_bt27_target(&mut r);
    assert_eq!(r.battle_area_size(0), 1);
    assert_eq!(r.battle_area_size(1), 0);
}
use super::{board, resolve_only_bt27_target};

#[test]
fn bt27_068_deletes_when_no_digimon_can_be_returned() {
    let mut r = board("BT27-068");
    let index = r.add_to_hand(0, "BT27-068");
    r.place_on_field(0, "HUCKMON", Some(0));
    r.place_on_field(1, "FILLER", Some(0));
    r.game.use_option_from_hand_without_paying_cost(0, index);
    resolve_only_bt27_target(&mut r);
    assert_eq!(r.battle_area_size(1), 0);
}
