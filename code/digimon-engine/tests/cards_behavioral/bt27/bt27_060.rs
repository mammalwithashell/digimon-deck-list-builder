use super::{board, resolve_only_bt27_target, trigger};
use digimon_engine::enums::{EffectTiming, Keyword};

#[test]
fn bt27_060_matches_its_mamemon_name_rule() {
    let mut r = board("BT27-060");
    let thundermon = r.place_on_field(0, "BT27-060", Some(0));
    r.place_on_field(0, "FILLER", Some(0));
    trigger(&mut r, EffectTiming::OnPlay, thundermon);
    resolve_only_bt27_target(&mut r);
    assert!(r.game.has_keyword(thundermon, Keyword::Blocker));
}
