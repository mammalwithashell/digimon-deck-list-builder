use super::{board, trigger};
use digimon_engine::enums::EffectTiming;

#[test]
fn bt27_043_trashes_security_to_unsuspend() {
    let mut r = board("BT27-043");
    let kentaurosmon = r.place_on_field(0, "BT27-043", Some(0));
    r.game.players[0].battle_area[0].is_suspended = true;
    trigger(&mut r, EffectTiming::WhenAttacking, kentaurosmon);
    r.accept_optional_trigger().unwrap();
    assert_eq!(r.security_count(0), 0);
    assert!(!r.game.players[0].battle_area[0].is_suspended);
}
