//! Official Bandai BT27-047 image and category 522040, checked 2026-11-22.
//! No card-specific Q&A appears in that category's 047 entry.
//! Synthetic board fixtures; the subject always loads its production DSL.
use digimon_engine::action::{
    build_action_mask,
    space::{encode_attack, encode_digivolve, PASS, SECURITY_TARGET},
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword};
use digimon_engine::selection::SelectionKind;
const ID: &str = "BT27-047";
fn fixture(id: &str, name: &str, level: u8, color: CardColor, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(3000);
    c.colors = vec![color];
    c.traits = traits.iter().map(|s| s.to_string()).collect();
    c
}
fn runner() -> DebugRunner {
    let mut host = fixture("HOST", "Host", 5, CardColor::Green, &[]);
    host.dp = Some(12000);
    let mut tamer = make_test_card("TAMER", "Tamer");
    tamer.card_kind = CardKind::Tamer;
    let mut r = DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT27-047 production YAML")
        .add_card(fixture("FILL", "Filler", 3, CardColor::Red, &[]))
        .add_card(host)
        .add_card(tamer)
        .deck(0, &["FILL"; 12])
        .deck(1, &["FILL"; 12])
        .security(1, &["FILL"; 5])
        .build();
    r.set_first_player(0);
    r.set_turn(2);
    r.set_phase(GamePhase::Main);
    r.game.set_memory(10);
    r
}
fn act(r: &mut DebugRunner, player: u8, action: u16) {
    assert_eq!(build_action_mask(&r.game, player)[action as usize], 1.0);
    r.game.decode_action(action, player);
}
#[test]
fn bt27_047_printed_identity() {
    let r = runner();
    let c = r.compiled_card(ID).unwrap();
    assert_eq!(c.name, "WezenGammamon");
    assert_eq!(c.level, Some(4));
    assert_eq!(c.cost, Some(4));
    assert_eq!(c.dp, Some(5000));
    assert_eq!(c.all_traits(), ["Champion", "Data", "Ceratopsian", "VB"]);
}
#[test]
fn bt27_047_on_play_mandatory_opponent_digimon_choice_and_masks() {
    for pick in 0..2 {
        let mut r = runner();
        r.place_on_field(0, "FILL", Some(0));
        r.place_on_field(1, "FILL", Some(0));
        r.place_on_field(1, "FILL", Some(1));
        r.place_on_field(1, "TAMER", Some(2));
        r.place_in_breeding(1, "FILL");
        r.add_to_hand(0, ID);
        r.play(0, 0).unwrap();
        let v = r.pending_selection_view().expect("mandatory target choice");
        assert_eq!(v.kind, SelectionKind::OppField);
        assert!(!v.is_optional);
        let mut ids = v.valid_action_ids;
        ids.sort();
        assert_eq!(ids.len(), 2);
        assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 0.0);
        assert!(!r.game.players[1].battle_area[0].is_suspended);
        assert!(!r.game.players[1].battle_area[1].is_suspended);
        act(&mut r, 0, ids[pick]);
        for i in 0..2 {
            assert_eq!(r.game.players[1].battle_area[i].is_suspended, i == pick);
        }
        assert!(!r.game.players[0].battle_area[0].is_suspended);
        assert!(!r.game.players[1].battle_area[2].is_suspended);
        assert_eq!(r.memory(), 6);
    }
}
#[test]
fn bt27_047_no_opponent_digimon_does_not_park() {
    let mut r = runner();
    r.place_on_field(1, "TAMER", Some(0));
    r.place_in_breeding(1, "FILL");
    r.add_to_hand(0, ID);
    r.play(0, 0).unwrap();
    assert!(r.pending_selection().is_none());
    assert!(!r.game.players[1].battle_area[0].is_suspended);
}
#[test]
fn bt27_047_actual_breeding_move_triggers_but_another_move_does_not() {
    for own_move in [false, true] {
        let mut r = runner();
        r.place_on_field(1, "FILL", Some(0));
        if !own_move {
            r.place_on_field(0, ID, Some(0));
        }
        r.place_in_breeding(0, if own_move { ID } else { "FILL" });
        assert!(r.move_from_breeding(0));
        if own_move {
            let v = r.pending_selection_view().expect("when moving");
            assert!(!v.is_optional);
            assert_eq!(v.valid_action_ids.len(), 1);
            act(&mut r, 0, v.valid_action_ids[0]);
        } else {
            assert!(r.pending_selection().is_none());
        }
        assert_eq!(r.game.players[1].battle_area[0].is_suspended, own_move);
    }
}
#[test]
fn bt27_047_when_attacking_dp_once_per_turn_and_expires() {
    let mut r = runner();
    let h = r.place_on_field(0, ID, Some(0));
    assert_eq!(r.effective_dp(h), Some(5000));
    act(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    assert_eq!(r.effective_dp(h), Some(10000));
    assert_eq!(r.security_count(1), 4);
    // Stage a second legal attack in the same turn; no second +5000.
    r.game.players[0].battle_area[0].is_suspended = false;
    act(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    assert_eq!(r.effective_dp(h), Some(10000));
    assert_eq!(r.security_count(1), 3);
    r.end_turn();
    assert_eq!(r.effective_dp(h), Some(5000));
}
#[test]
fn bt27_047_piercing_only_inherited_real_attack_and_no_direct_battle_check() {
    for inherited in [false, true] {
        for attack in [false, true] {
            let mut r = runner();
            let h = if inherited {
                r.place_stack(0, &[ID, "HOST"])
            } else {
                r.place_on_field(0, ID, Some(0))
            };
            let target = r.place_on_field(1, "FILL", Some(0));
            r.game.players[1].battle_area[0].is_suspended = true;
            assert_eq!(r.game.has_keyword(h, Keyword::Piercing), inherited);
            if attack {
                act(&mut r, 0, encode_attack(0, 0));
            } else {
                r.battle_digimon(h, target);
            }
            assert_eq!(r.battle_area_size(1), 0);
            assert_eq!(r.security_count(1), if inherited && attack { 4 } else { 5 });
            if inherited {
                assert_eq!(
                    r.effective_dp(h),
                    Some(12000),
                    "face attack bonus is not inherited"
                );
            }
        }
    }
}
fn route(base: CardData, expected_costs: &[i16]) {
    for chosen_cost in expected_costs
        .iter()
        .copied()
        .chain(if expected_costs.is_empty() {
            Some(0)
        } else {
            None
        })
    {
        let base_id = base.card_id.clone();
        let mut r = DebugRunner::builder()
            .dsl_card(ID)
            .unwrap()
            .add_card(base.clone())
            .add_card(fixture("FILL", "Filler", 3, CardColor::Red, &[]))
            .deck(0, &["FILL"; 8])
            .deck(1, &["FILL"; 8])
            .build();
        r.set_first_player(0);
        r.set_turn(2);
        r.set_phase(GamePhase::Main);
        r.game.set_memory(10);
        r.place_on_field(0, &base_id, Some(0));
        r.add_to_hand(0, ID);
        let action = encode_digivolve(0, 0);
        assert_eq!(
            build_action_mask(&r.game, 0)[action as usize] > 0.0,
            !expected_costs.is_empty()
        );
        if expected_costs.is_empty() {
            continue;
        }
        act(&mut r, 0, action);
        if expected_costs.len() > 1 {
            let v = r
                .pending_selection_view()
                .expect("both printed costs exposed");
            assert_eq!(v.kind, SelectionKind::EffectChoice);
            let opts = v.effect_choices.unwrap();
            assert_eq!(opts.len(), expected_costs.len());
            let opt = opts
                .iter()
                .find(|c| {
                    c.label
                        .to_lowercase()
                        .contains(&format!("cost {chosen_cost}"))
                })
                .unwrap();
            act(&mut r, 0, opt.action_id);
        }
        assert_eq!(r.memory(), 10 - chosen_cost);
        assert_eq!(
            r.game.players[0].battle_area[0]
                .top_card()
                .card_id(&r.game.card_data),
            ID
        );
        assert_eq!(r.hand_size(0), 1, "digivolution bonus draw");
        assert!(
            r.pending_selection().is_none(),
            "digivolving does not fire on-play"
        );
    }
}
#[test]
fn bt27_047_normal_and_special_routes_expose_both_costs() {
    for color in [CardColor::Green, CardColor::Yellow] {
        route(fixture("BASE", "Other", 3, color, &[]), &[3]);
        route(fixture("BASE", "Other", 3, color, &["VB"]), &[2, 3]);
        route(fixture("BASE", "Gammamon", 3, color, &[]), &[2, 3]);
    }
    route(fixture("BASE", "Gammamon", 3, CardColor::Red, &[]), &[2]);
    route(fixture("BASE", "Other", 3, CardColor::Blue, &["VB"]), &[2]);
}
#[test]
fn bt27_047_evolution_rejects_partial_name_wrong_level_wrong_trait() {
    route(fixture("BASE", "Gammamon X", 3, CardColor::Blue, &[]), &[]);
    route(fixture("BASE", "Other", 4, CardColor::Green, &["VB"]), &[]);
    route(
        fixture("BASE", "Other", 3, CardColor::Blue, &["VB-like"]),
        &[],
    );
}
