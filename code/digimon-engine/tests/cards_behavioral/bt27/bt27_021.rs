use super::{board, resolve_only_bt27_target, trigger};
use digimon_engine::enums::EffectTiming;

#[test]
fn bt27_021_selects_both_examon_search_buckets() {
    let mut r = board("BT27-021");
    let dracomon = r.place_on_field(0, "BT27-021", Some(0));
    trigger(&mut r, EffectTiming::OnPlay, dracomon);
    resolve_only_bt27_target(&mut r);
    resolve_only_bt27_target(&mut r);
    let hand: Vec<_> = r.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data))
        .collect();
    assert_eq!(hand, ["EXAMON", "EXAMON-OPTION"]);
}
