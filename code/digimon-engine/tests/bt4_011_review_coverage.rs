const OBSERVER: &str = r#"
card: TEST-Q1157-OBSERVER
name: Would Evolve Cost Observer
kind: tamer
color: [white]
cost: 2
effects:
  - kind: cost_reduction
    reduction_timing: before_pay_cost
    optional: true
    when_any_ally_digivolves_into: {card_number_is: BT4-011}
    condition:
      any_permanent: {of: you, zone: [battle_area], kind: digimon}
    amount: 1
    pay_cost:
      - suspend: {target: source}
"#;
use digimon_engine::{
    action::{build_action_mask, encode_digivolve, space::PLAY_HAND_START},
    card_data::EvoCost,
    debug_runner::DebugRunner,
    enums::{CardKind, GamePhase},
};
fn setup(option: bool) -> DebugRunner {
    setup_observer(option, OBSERVER)
}
fn setup_observer(option: bool, observer: &str) -> DebugRunner {
    let path = std::env::var("BT4_011_TEST_YAML")
        .unwrap_or_else(|_| format!("{}/cards/bt4/BT4-011.yaml", env!("CARGO_MANIFEST_DIR")));
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(&std::fs::read_to_string(path).unwrap())
        .unwrap()
        .from_dsl_yaml(include_str!("../cards/st1/ST1-12.yaml"))
        .unwrap()
        .from_dsl_yaml(include_str!("../cards/st2/ST2-12.yaml"))
        .unwrap()
        .from_dsl_yaml(include_str!("../cards/st1/ST1-03.yaml"))
        .unwrap()
        .from_dsl_yaml(include_str!("../cards/LM/LM-027.yaml"))
        .unwrap()
        .from_dsl_yaml("card: TEST-MULTICOLOR-TAMER\nname: Multicolor Fixture\nkind: tamer\ncolor: [red, yellow]\ncost: 4\n")
        .unwrap()
        .from_dsl_yaml(observer)
        .unwrap()
        .hand(
            0,
            if option {
                &["LM-027", "BT4-011"][..]
            } else {
                &["BT4-011"][..]
            },
        )
        .deck(0, &["ST1-03"; 12])
        .deck(1, &["ST1-03"; 12])
        .memory(5)
        .start();
    // Exact printed circle; DebugRunner compiled metadata omits compatibility evo_costs.
    r.game
        .card_data
        .iter_mut()
        .find(|c| c.card_id == "BT4-011")
        .unwrap()
        .evo_costs = vec![EvoCost {
        card_color: 0,
        level: 3,
        memory_cost: 2,
    }];
    r.game.current_phase = GamePhase::Main;
    r
}



#[test]
fn q1159_fresh_tamer_remains_unable_to_attack_after_evolution() {
    use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
    for fresh in [false,true] {
        let mut r=setup(false);let turn=r.game.turn_count;
        r.place_on_field(0,"ST1-12",Some(if fresh {turn} else {0}));
        let evo=encode_digivolve(0,0);assert_eq!(build_action_mask(&r.game,0)[evo as usize],1.0);
        r.game.decode_action(evo,0);assert!(r.game.pending_selection.is_none());
        assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data),"BT4-011");
        assert_eq!(build_action_mask(&r.game,0)[encode_attack(0,SECURITY_TARGET) as usize],if fresh {0.0} else {1.0});
    }
}
#[test]
fn direct_prohibition_rejects_tamer_route_in_mask_and_declaration() {
    use digimon_engine::{enums::{ModifierType,Expiry,PlaySource},modifiers::ModifierEntry};
    let mut r=setup(false);let h=r.place_on_field(0,"ST1-12",None);
    r.game.modifiers.add(h,ModifierEntry::simple(ModifierType::CannotDigivolve,0,Expiry::Permanent,0));
    assert_eq!(build_action_mask(&r.game,0)[encode_digivolve(0,0) as usize],0.0);
    assert!(!r.game.digivolve_from_hand(0,0,0,PlaySource::ByHand));
    assert_eq!(r.memory(),5);assert_eq!(r.game.players[0].hand.len(),1);
}
#[test]
fn opponent_tamer_is_not_an_own_evolution_target() {
    let mut r=setup(false);r.place_on_field(1,"ST1-12",None);
    assert_eq!(build_action_mask(&r.game,0)[encode_digivolve(0,0) as usize],0.0);
    assert_eq!(r.game.players[1].battle_area[0].top_card().card_id(&r.game.card_data),"ST1-12");
}
#[test]
fn production_circle_matches_official_red_level_three_cost_two() {
    let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/cards.json");
    let cards:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert!(cards["BT4-011"]["evo_costs"].as_array().unwrap().iter().any(|c|c["card_color"]==0 && c["level"]==3 && c["memory_cost"]==2));
}
#[test]
fn q1157_post_digivolve_observer_accept_decline_preserves_bonus_draw() {
    use digimon_engine::action::space::PASS;
    let observer=r#"card: TEST-Q1157-OBSERVER
name: Post Evolution Observer
kind: tamer
color: [white]
cost: 2
effects:
  - when: on_digivolve
    optional: true
    process:
      - draw: {of: you, count: 1}
"#;
    let mut r=setup_observer(false,observer);r.place_on_field(0,"ST1-12",None);r.place_on_field(0,"TEST-Q1157-OBSERVER",None);
    let evo=encode_digivolve(0,0);assert_eq!(build_action_mask(&r.game,0)[evo as usize],1.0);r.game.decode_action(evo,0);
    let pick=r.game.pending_selection.as_ref().expect("post-digivolve observer");let accept=*pick.valid_action_ids.iter().find(|a|**a!=PASS).unwrap();
    assert_eq!(r.game.players[0].hand.len(),1,"result leaves hand and bonus draw already resolves");
    for (action,count) in [(PASS,1),(accept,2)] {let mut g=r.game.clone();assert_eq!(build_action_mask(&g,0)[action as usize],1.0);g.resolve_selection(0,action).unwrap();assert_eq!(g.players[0].hand.len(),count);assert_eq!(g.players[0].battle_area[0].top_card().card_id(&g.card_data),"BT4-011");}
}

#[test]
fn effect_cannot_borrow_own_tamer_route_on_opponent_tamer() {
    use digimon_engine::enums::{CostDelta,PlaySource};
    let mut r=setup(false);let h=r.place_on_field(1,"ST1-12",None);
    assert!(!r.game.effect_initiated_digivolve(0,0,h,CostDelta::Reduce(0),false,PlaySource::ByEffect));
    assert_eq!(r.memory(),5);assert_eq!(r.game.players[0].hand.len(),1);
    assert_eq!(r.game.players[1].battle_area[0].top_card().card_id(&r.game.card_data),"ST1-12");
}


#[test]
fn production_metadata_drives_native_tamer_and_ordinary_routes_without_manual_evo_costs() {
    use digimon_engine::card_data::CardData;
    let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/cards.json");
    let all=CardData::load_from_file(&path).unwrap();
    for base in ["ST1-12", "ST1-03"] {
        let data=all.iter().filter(|(id,_)|["BT4-011","ST1-12","ST1-03"].contains(&id.as_str())).map(|(k,v)|(k.clone(),v.clone())).collect();
        let mut r=DebugRunner::builder()
            .from_dsl_yaml(include_str!("../cards/bt4/BT4-011.yaml")).unwrap()
            .from_dsl_yaml(include_str!("../cards/st1/ST1-12.yaml")).unwrap()
            .from_dsl_yaml(include_str!("../cards/st1/ST1-03.yaml")).unwrap()
            .with_card_data(data).hand(0,&["BT4-011"]).deck(0,&["ST1-03";12]).deck(1,&["ST1-03";12]).memory(5).start();
        r.game.current_phase=GamePhase::Main;r.place_on_field(0,base,None);
        let a=encode_digivolve(0,0);assert_eq!(build_action_mask(&r.game,0)[a as usize],1.0);
        r.game.decode_action(a,0);assert!(r.game.pending_selection.is_none());assert_eq!(r.memory(),3);
        assert_eq!(r.game.players[0].hand.len(),1);assert_eq!(r.game.players[0].deck.len(),11);
        let ids:Vec<_>=r.game.players[0].battle_area[0].card_sources.iter().map(|c|c.card_id(&r.game.card_data)).collect();
        assert_eq!(ids,vec![base,"BT4-011"]);
    }
}

#[test]
fn q1160_q1163_tamer_source_is_trashed_on_return_and_deletion() {
    for delete in [false,true] {
        let mut r=setup(false);let host=r.place_on_field(0,"ST1-12",None);
        let a=encode_digivolve(0,0);assert_eq!(build_action_mask(&r.game,0)[a as usize],1.0);r.game.decode_action(a,0);
        if delete {r.game.delete_permanent_with_effects(host);} else {assert!(r.game.return_to_hand(host).is_some());}
        assert!(r.game.players[0].battle_area.is_empty());
        let trash:Vec<_>=r.game.players[0].trash.iter().map(|c|c.card_id(&r.game.card_data)).collect();
        assert!(trash.contains(&"ST1-12"));assert_eq!(trash.contains(&"BT4-011"),delete);
        assert_eq!(r.game.players[0].hand.iter().any(|c|c.card_id(&r.game.card_data)=="BT4-011"),!delete);
    }
}

#[test]
fn q4035_mixed_board_offers_red_digimon_but_excludes_red_tamer() {
    use digimon_engine::action::space::{ATTACK_START,PLAY_HAND_START};
    let mut r=setup(true);r.place_on_field(0,"ST1-12",None);r.place_on_field(0,"ST1-03",None);
    assert_eq!(build_action_mask(&r.game,0)[PLAY_HAND_START as usize],1.0);r.game.decode_action(PLAY_HAND_START,0);
    assert!(r.game.pending_selection.is_some());let mask=build_action_mask(&r.game,0);
    assert_eq!(mask[ATTACK_START as usize],0.0,"red Tamer remains outside Scramble subject scope");
    assert_eq!(mask[(ATTACK_START+1) as usize],1.0,"ordinary red Digimon is a real eligible control");
    r.game.resolve_selection(0,ATTACK_START+1).unwrap();
    assert_eq!(build_action_mask(&r.game,0)[PLAY_HAND_START as usize],1.0);r.game.resolve_selection(0,PLAY_HAND_START).unwrap();
    assert!(r.game.pending_selection.is_none());assert_eq!(r.memory(),3);
    assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data),"ST1-12");
    assert_eq!(r.game.players[0].battle_area[1].top_card().card_id(&r.game.card_data),"BT4-011");
}
