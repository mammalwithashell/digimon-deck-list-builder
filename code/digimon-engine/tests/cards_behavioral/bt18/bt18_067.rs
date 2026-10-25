//! BT18-067: official d0fa3065d737b7c9a44461bbe0d8a4a4eac06164fc825e6365519bef90014293.
//! Image confirms both-color Lv.3 and Tamer circles, named J.P./Beetlemon routes.
use digimon_engine::action::{
    mask::build_action_mask,
    space::{encode_digivolve, PASS},
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword};

fn card(id: &str, level: u8, color: CardColor) -> CardData {
    let mut c = make_test_card_with_level(id, id, level);
    c.colors = vec![color];
    c.dp = Some(6000);
    c
}
fn tamer(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.colors = vec![color];
    c
}
fn runner(base: CardData) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT18-067")
        .expect("BT18-067 production YAML")
        .add_card(base)
        .add_card(card("TEST-L3", 3, CardColor::Red))
        .add_card(card("TEST-L4", 4, CardColor::Red))
        .add_card(card("TEST-L5", 5, CardColor::Red))
        .deck(0, &["TEST-L3"; 15])
        .hand(0, &["BT18-067"])
        .memory(10)
        .start()
}
fn pick_first(r: &mut DebugRunner) {
    let v = r
        .pending_selection_view()
        .expect("mandatory opponent target choice");
    assert!(!v.valid_action_ids.contains(&PASS));
    let action = v.valid_action_ids[0];
    assert!(build_action_mask(&r.game, 0)[action as usize] > 0.5);
    r.game.decode_action(action, 0);
    r.auto_resolve().expect("resolve De-Digivolve");
}
#[test]
fn bt18_067_on_play_de_digivolves_one_selected_opponent() {
    let mut r = runner(card("TEST-BASE", 3, CardColor::Black));
    let target = r.place_stack(1, &["TEST-L3", "TEST-L4", "TEST-L5"]);
    let other = r.place_stack(1, &["TEST-L3", "TEST-L4"]);
    assert!(build_action_mask(&r.game, 0)[0] > 0.5);
    r.game.decode_action(0, 0);
    assert_eq!(
        r.pending_selection_view().unwrap().valid_action_ids.len(),
        2
    );
    pick_first(&mut r);
    assert_eq!(
        r.game.players[1].battle_area[target.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "TEST-L4"
    );
    assert_eq!(
        r.game.players[1].battle_area[other.index as usize]
            .card_sources
            .len(),
        2
    );
}
#[test]
fn bt18_067_when_digivolving_de_digivolves_but_stops_at_level_three() {
    let mut r = runner(card("TEST-BASE", 3, CardColor::Black));
    let host = r.place_on_field(0, "TEST-BASE", Some(0));
    let target = r.place_stack(1, &["TEST-L4", "TEST-L3"]);
    let action = encode_digivolve(0, host.index as u16);
    assert!(build_action_mask(&r.game, 0)[action as usize] > 0.5);
    r.game.decode_action(action, 0);
    pick_first(&mut r);
    assert_eq!(
        r.game.players[1].battle_area[target.index as usize]
            .card_sources
            .len(),
        2,
        "level 3 is not trashed"
    );
}
#[test]
fn bt18_067_no_opponent_digimon_does_not_install_impossible_selection() {
    let mut r = runner(tamer("TEST-TAMER", CardColor::Yellow));
    r.place_on_field(1, "TEST-TAMER", Some(0));
    r.game.decode_action(0, 0);
    assert!(r.pending_selection_view().is_none());
}
#[test]
fn bt18_067_blocker_works_on_top_and_as_inherited_effect() {
    let mut r = runner(card("TEST-BASE", 5, CardColor::Black));
    let own = r.place_on_field(0, "BT18-067", Some(0));
    assert!(r.game.has_keyword(own, Keyword::Blocker));
    let carrier = r.place_stack(0, &["BT18-067", "TEST-BASE"]);
    assert!(r.game.has_keyword(carrier, Keyword::Blocker));
    let plain = r.place_on_field(0, "TEST-BASE", Some(0));
    assert!(!r.game.has_keyword(plain, Keyword::Blocker));
}
#[test]
fn bt18_067_all_printed_evolution_routes_pay_their_costs() {
    let mut cases = vec![
        (card("TEST-BLACK", 3, CardColor::Black), 4),
        (card("TEST-YELLOW", 3, CardColor::Yellow), 4),
        (tamer("TEST-BLACK-TAMER", CardColor::Black), 4),
        (tamer("TEST-YELLOW-TAMER", CardColor::Yellow), 4),
        (tamer("J.P. Shibayama", CardColor::Green), 3),
        (card("Beetlemon", 4, CardColor::Green), 1),
    ];
    let mut alias = tamer("TEST-ALIAS", CardColor::White);
    alias.also_treated_as = vec!["J.P. Shibayama".into()];
    cases.push((alias, 3));
    for (base, cost) in cases {
        let id = base.card_id.clone();
        let mut r = runner(base);
        let host = r.place_on_field(0, &id, Some(0));
        r.game.current_phase = GamePhase::Main;
        let action = encode_digivolve(0, host.index as u16);
        assert!(
            build_action_mask(&r.game, 0)[action as usize] > 0.5,
            "route {id} legal"
        );
        r.game.decode_action(action, 0);
        r.auto_resolve().unwrap();
        assert_eq!(r.memory(), 10 - cost, "route {id}");
        assert_eq!(
            r.game.players[0].battle_area[host.index as usize]
                .top_card()
                .card_id(&r.game.card_data),
            "BT18-067"
        );
    }
}
#[test]
fn bt18_067_wrong_color_or_level_cannot_use_generic_routes() {
    for base in [
        card("TEST-BLUE", 3, CardColor::Blue),
        card("TEST-L4-BLACK", 4, CardColor::Black),
        tamer("TEST-RED-TAMER", CardColor::Red),
    ] {
        let id = base.card_id.clone();
        let mut r = runner(base);
        let host = r.place_on_field(0, &id, Some(0));
        assert_eq!(
            build_action_mask(&r.game, 0)[encode_digivolve(0, host.index as u16) as usize],
            0.0,
            "{id} is not a printed route"
        );
    }
}
