use super::{board, resolve_only_bt27_target, trigger};
use digimon_engine::enums::{EffectTiming, GamePhase, Keyword};

#[test]
fn bt27_034_offers_both_players_digimon_for_suspension() {
    let mut r = board("BT27-034");
    let examon = r.place_on_field(0, "BT27-034", Some(0));
    r.place_on_field(1, "FILLER", Some(0));
    assert!(r.game.has_keyword(examon, Keyword::Raid));
    assert!(r.game.has_keyword(examon, Keyword::Evade));
    trigger(&mut r, EffectTiming::WhenDigivolving, examon);
    let pick = r
        .pending_selection_view()
        .expect("Choose Digimon or Tamers to suspend");
    assert!(pick.is_optional);
    assert_eq!(pick.valid_action_ids.len(), 2);
}

#[test]
fn bt27_034_counter_is_available_from_the_battle_area() {
    let mut r = board("BT27-034");
    let examon = r.place_on_field(1, "BT27-034", Some(0));
    let attacker = r.place_on_field(0, "FILLER", Some(0));
    r.set_turn(2);
    r.attack_player(attacker, 1, false);
    assert_eq!(r.current_phase(), GamePhase::CounterTiming);
    assert_eq!(r.pending_selection_view().unwrap().selecting_player, 1);
    resolve_only_bt27_target(&mut r);
    let pick = r.pending_selection_view().expect("Suspend up to 3");
    assert!(pick.is_optional);
    r.decline_prompt().unwrap();
    assert!(!r.game.players[1].battle_area[examon.index as usize].is_suspended);
}
