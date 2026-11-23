use super::{board, resolve_only_bt27_target, trigger};
use digimon_engine::enums::{EffectTiming, Keyword};

#[test]
fn bt27_072_activates_a_mamemon_sources_on_deletion_effect() {
    let mut r = board("BT27-072");
    let filler = r.place_on_field(0, "FILLER", Some(0));
    assert!(!r.game.has_keyword(filler, Keyword::Blocker));
    let carrier = r.place_stack(0, &["BT27-065", "BT27-072"]);
    let thundermon = r.place_on_field(0, "BT27-060", Some(0));
    r.place_on_field(1, "FILLER", Some(0));
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(thundermon, Keyword::Alliance));
    assert!(r.game.has_keyword(thundermon, Keyword::Blocker));
    trigger(&mut r, EffectTiming::WhenDigivolving, carrier);
    resolve_only_bt27_target(&mut r);
    resolve_only_bt27_target(&mut r);
    assert_eq!(r.battle_area_size(1), 0);
    assert_eq!(r.battle_area_size(0), 3);
}
