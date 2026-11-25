use super::{board, resolve_only_bt27_target};

#[test]
fn bt27_100_security_deletes_all_lowest_play_cost_digimon_then_returns_to_hand() {
    let mut r = board("BT27-100");
    r.set_first_player(1);
    r.set_turn(2);
    let attacker = r.place_on_field(1, "LEVEL4", Some(0));
    r.place_on_field(1, "FILLER", Some(0));
    r.attack_player(attacker, 0, false);
    assert_eq!(r.battle_area_size(1), 0);
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(
        r.game.players[0].hand[0].card_id(&r.game.card_data),
        "BT27-100"
    );
}

#[test]
fn bt27_100_places_itself_in_the_battle_area() {
    let mut r = board("BT27-100");
    let index = r.add_to_hand(0, "BT27-100");
    r.place_on_field(0, "FILLER", Some(0));
    r.place_on_field(1, "FILLER", Some(0));
    r.game.use_option_from_hand_without_paying_cost(0, index);
    resolve_only_bt27_target(&mut r);
    assert_eq!(
        r.game.players[0].battle_area[1]
            .top_card()
            .card_id(&r.game.card_data),
        "BT27-100"
    );
}

#[test]
fn bt27_100_craniamon_suspension_enables_delay() {
    let mut r = board("BT27-100");
    let index = r.add_to_hand(0, "BT27-100");
    r.place_on_field(0, "FILLER", Some(0));
    let craniamon = r.place_on_field(0, "CRANIAMON", Some(0));
    r.place_stack(1, &["FILLER", "LEVEL4"]);
    r.game.use_option_from_hand_without_paying_cost(0, index);
    resolve_only_bt27_target(&mut r);
    r.set_turn(2);
    r.game.suspend(craniamon);
    r.accept_optional_trigger().unwrap();
    assert_eq!(r.battle_area_size(1), 0);
    assert_eq!(r.battle_area_size(0), 2);
}
