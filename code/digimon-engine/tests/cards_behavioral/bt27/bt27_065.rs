use super::{board, resolve_only_bt27_target, trigger};
use digimon_engine::enums::EffectTiming;

#[test]
fn bt27_065_trashes_security_when_there_is_no_digimon_to_delete() {
    let mut r = board("BT27-065");
    let mamemon = r.place_on_field(0, "BT27-065", Some(0));
    assert_eq!(r.security_count(1), 1);
    trigger(&mut r, EffectTiming::OnDeletion, mamemon);
    assert_eq!(r.security_count(1), 0);
}

#[test]
fn bt27_065_deletes_the_lowest_dp_digimon_instead_of_trashing_security() {
    let mut r = board("BT27-065");
    let mamemon = r.place_on_field(0, "BT27-065", Some(0));
    r.place_on_field(1, "FILLER", Some(0));
    r.place_on_field(1, "LEVEL4", Some(0));
    trigger(&mut r, EffectTiming::OnDeletion, mamemon);
    resolve_only_bt27_target(&mut r);
    assert_eq!(
        r.game.players[1].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "LEVEL4"
    );
    assert_eq!(r.security_count(1), 1);
}
