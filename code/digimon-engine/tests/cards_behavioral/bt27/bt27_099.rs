use super::{board, resolve_only_bt27_target};

#[test]
fn bt27_099_security_check_places_only_a_lowest_dp_digimon_on_top() {
    let mut r = board("BT27-099");
    r.set_first_player(1);
    r.set_turn(2);
    let attacker = r.place_on_field(1, "LEVEL4", Some(0));
    r.place_on_field(1, "FILLER", Some(0));
    r.attack_player(attacker, 0, false);
    resolve_only_bt27_target(&mut r);
    assert_eq!(r.battle_area_size(1), 1);
    assert_eq!(
        r.game.players[1]
            .security
            .last()
            .unwrap()
            .card_id(&r.game.card_data),
        "FILLER"
    );
}

#[test]
fn bt27_099_trashing_it_from_security_by_an_effect_activates_security() {
    use super::trigger;
    use digimon_engine::enums::EffectTiming;

    let mut r = board("BT27-099");
    let kentaurosmon = r.place_on_field(0, "BT27-043", Some(0));
    r.place_on_field(1, "FILLER", Some(0));
    trigger(&mut r, EffectTiming::WhenDigivolving, kentaurosmon);
    r.accept_optional_trigger().unwrap();
    let order = r
        .pending_selection_view()
        .expect("Order security removal effects");
    let discard_security = order
        .effect_choices
        .unwrap()
        .into_iter()
        .find(|effect| effect.timing == Some(EffectTiming::OnDiscardSecurity))
        .expect("Surtr Brander activates when trashed from security");
    r.execute_action(0, discard_security.action_id).unwrap();
    resolve_only_bt27_target(&mut r);
    assert_eq!(r.security_count(0), 0);
    assert_eq!(r.battle_area_size(1), 0);
    assert_eq!(r.security_count(1), 1);
}

#[test]
fn bt27_099_reduces_dp_and_moves_itself_to_security() {
    let mut r = board("BT27-099");
    let index = r.add_to_hand(0, "BT27-099");
    r.place_on_field(0, "FILLER", Some(0));
    let target = r.place_on_field(1, "LEVEL4", Some(0));
    r.game.use_option_from_hand_without_paying_cost(0, index);
    resolve_only_bt27_target(&mut r);
    assert_eq!(r.effective_dp(target), Some(1000));
    let top = r.game.players[0].security.last().unwrap();
    assert_eq!(top.card_id(&r.game.card_data), "BT27-099");
}
