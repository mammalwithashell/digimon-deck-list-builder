use super::{board, resolve_only_bt27_target, trigger};
use digimon_engine::enums::EffectTiming;

#[test]
fn bt27_013_de_digivolves_without_a_deletion_outside_an_attack() {
    let mut r = board("BT27-013");
    let hisyaryumon = r.place_on_field(0, "BT27-013", Some(0));
    r.place_stack(1, &["FILLER", "LEVEL4"]);
    trigger(&mut r, EffectTiming::WhenDigivolving, hisyaryumon);
    resolve_only_bt27_target(&mut r);
    assert_eq!(r.battle_area_size(1), 1);
    assert_eq!(r.game.players[1].battle_area[0].card_sources.len(), 1);
    assert!(r.pending_selection_view().is_none());
}
