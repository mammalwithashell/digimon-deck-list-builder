use super::{board, resolve_only_bt27_target, trigger};
use digimon_engine::enums::EffectTiming;

#[test]
fn bt27_103_recovers_and_can_activate_its_option_face() {
    let mut r = board("BT27-103");
    let alphamon = r.place_on_field(0, "BT27-103", Some(0));
    r.place_on_field(1, "LEVEL4", Some(0));
    trigger(&mut r, EffectTiming::WhenDigivolving, alphamon);
    let order = r
        .pending_selection_view()
        .expect("Order digivolution effects");
    let recovery = order
        .effect_choices
        .expect("Two When Digivolving effects")
        .into_iter()
        .find(|effect| !effect.is_optional)
        .expect("Recovery is mandatory");
    r.execute_action(0, recovery.action_id).unwrap();
    assert_eq!(r.security_count(0), 1);
    r.accept_optional_trigger().unwrap();
    resolve_only_bt27_target(&mut r);
    assert_eq!(r.battle_area_size(1), 0);
}
