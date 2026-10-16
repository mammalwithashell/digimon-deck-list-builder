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
fn multicolor_tamer_keeps_yellow_in_cost_window_and_clones() {
    use digimon_engine::{enums::CardColor, action::space::PASS};
    let mut r=setup(false);
    let h=r.place_on_field(0,"TEST-MULTICOLOR-TAMER",None);
    r.place_on_field(0,"TEST-Q1157-OBSERVER",None);
    r.game.tick_declarative_effects();
    let a=encode_digivolve(0,0);assert_eq!(build_action_mask(&r.game,0)[a as usize],1.0);
    r.game.decode_action(a,0);assert!(r.game.pending_selection.is_some());
    let saved=r.game.clone();
    for _ in 0..2 {
        let mut g=saved.clone();let id=g.players[0].battle_area[0].synth_identity(&g.card_data,&g.modifiers,h);
        assert_eq!(id.colors,vec![CardColor::Red,CardColor::Yellow]);
        assert_eq!(id.kind,CardKind::Digimon);assert_eq!(id.level,Some(3));assert_eq!(id.dp,None);
        assert_eq!(build_action_mask(&g,0)[PASS as usize],1.0);g.resolve_selection(0,PASS).unwrap();
        assert_eq!(g.players[0].battle_area[0].top_card().card_id(&g.card_data),"BT4-011");
        assert_eq!(g.players[0].battle_area[0].synth_identity(&g.card_data,&g.modifiers,h).colors,vec![CardColor::Red]);
    }
}
#[test]
fn yellow_digimon_gate_blocks_multicolor_tamer_without_mutating_it() {
    let gate=r#"card: TEST-Q1157-OBSERVER
name: Yellow Gate
kind: tamer
color: [white]
cost: 2
effects:
  - kind: flood_gate
    modifier: CannotDigivolve
    target: {of: opponent, zone: [battle_area], kind: digimon, color_is: yellow}
"#;
    let mut r=setup_observer(false,gate);let h=r.place_on_field(0,"TEST-MULTICOLOR-TAMER",None);
    r.place_on_field(1,"TEST-Q1157-OBSERVER",None);r.game.tick_declarative_effects();
    let before=r.game.verification_digest();
    for g in [&r.game,&r.game.clone()] {assert_eq!(build_action_mask(g,0)[encode_digivolve(0,0) as usize],0.0);assert_eq!(g.players[0].battle_area[0].synth_identity(&g.card_data,&g.modifiers,h).kind,CardKind::Tamer);}
    assert_eq!(before,r.game.verification_digest());
}
#[test]
fn effect_from_trash_cannot_borrow_hand_only_tamer_route() {
    use digimon_engine::enums::{CardSourceRef,CostDelta,PlaySource};
    let mut r=setup(false);let h=r.place_on_field(0,"ST1-12",None);
    let c=r.game.players[0].hand.remove(0);r.game.players[0].trash.push(c);
    assert!(!r.game.effect_initiated_digivolve_from_source(0,CardSourceRef::Trash(0,0),h,CostDelta::Reduce(0),false,PlaySource::ByEffect));
    assert_eq!(r.memory(),5);assert_eq!(r.game.players[0].trash.len(),1);
    assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data),"ST1-12");
}
#[test]
fn effect_from_trash_retains_ordinary_printed_digimon_route() {
    use digimon_engine::enums::{CardSourceRef,CostDelta,PlaySource};
    let mut r=setup(false);let h=r.place_on_field(0,"ST1-03",None);
    let c=r.game.players[0].hand.remove(0);r.game.players[0].trash.push(c);
    assert!(r.game.effect_initiated_digivolve_from_source(0,CardSourceRef::Trash(0,0),h,CostDelta::Reduce(0),false,PlaySource::ByEffect));
    assert_eq!(r.memory(),3);assert!(r.game.players[0].trash.is_empty());
    assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data),"BT4-011");
}
#[test]
fn effect_from_hand_treated_route_exposes_cost_observer_before_commit() {
    use digimon_engine::enums::{CostDelta,PlaySource};
    let mut r=setup(false);let h=r.place_on_field(0,"ST1-12",None);
    r.place_on_field(0,"TEST-Q1157-OBSERVER",None);r.game.tick_declarative_effects();
    r.game.effect_initiated_digivolve(0,0,h,CostDelta::Reduce(0),false,PlaySource::ByEffect);
    assert!(r.game.pending_selection.is_some(),"Q1157 Digimon-scoped cost observer remains offered for an effect-driven legal hand route");
    let id=r.game.players[0].battle_area[0].synth_identity(&r.game.card_data,&r.game.modifiers,h);
    assert_eq!(id.kind,CardKind::Digimon);assert_eq!(id.level,Some(3));assert_eq!(id.dp,None);
}

#[test]
fn effect_routes_preserve_multicolor_identity_under_normal_level_and_color_waived_paths() {
    use digimon_engine::enums::{CardSourceRef,CostDelta,PlaySource,CardColor};
    for waiver in 0..3 {
        let mut r=setup(false);let h=r.place_on_field(0,"TEST-MULTICOLOR-TAMER",Some(0));r.place_on_field(0,"TEST-Q1157-OBSERVER",Some(0));r.game.tick_declarative_effects();
        if waiver == 1 {r.game.effect_initiated_digivolve_from_source_ignore_level(0,CardSourceRef::Hand(0,0),h,CostDelta::Reduce(0),PlaySource::ByEffect);}
        else {r.game.effect_initiated_digivolve(0,0,h,CostDelta::Reduce(0),waiver == 2,PlaySource::ByEffect);}
        if let Some(rows)=r.game.pending_selection.as_ref().and_then(|p|p.effect_choices.as_ref()) {
            if let Some(row)=rows.iter().find(|row|row.label.contains("treated")) {
                let action=row.action_id;assert_eq!(build_action_mask(&r.game,0)[digimon_engine::action::space::PASS as usize],0.0);r.game.resolve_selection(0,action).unwrap();
            }
        }
        let saved=r.game.clone();
        for _ in 0..2 {
            r.game=saved.clone();assert!(r.pending_selection().is_some());
            let id=r.game.players[0].battle_area[0].synth_identity(&r.game.card_data,&r.game.modifiers,h);
            assert_eq!(id.colors,vec![CardColor::Red,CardColor::Yellow]);assert_eq!(id.kind,CardKind::Digimon);assert_eq!(id.level,Some(3));assert_eq!(id.dp,None);
        }
    }
}
#[test]
fn effect_yellow_digimon_gate_blocks_multicolor_treated_route() {
    use digimon_engine::enums::{CostDelta,PlaySource};
    let gate="card: TEST-Q1157-OBSERVER\nname: Yellow Gate\nkind: tamer\ncolor: [white]\ncost: 2\neffects:\n  - kind: flood_gate\n    modifier: CannotDigivolve\n    target: {of: opponent, zone: [battle_area], kind: digimon, color_is: yellow}\n";
    let mut r=setup_observer(false,gate);let h=r.place_on_field(0,"TEST-MULTICOLOR-TAMER",Some(0));r.place_on_field(1,"TEST-Q1157-OBSERVER",Some(0));r.game.tick_declarative_effects();
    let saved=r.game.clone();
    for _ in 0..2 {r.game=saved.clone();assert!(!r.game.effect_initiated_digivolve(0,0,h,CostDelta::Reduce(0),false,PlaySource::ByEffect));assert_eq!(r.memory(),5);assert!(r.pending_selection().is_none());assert_eq!(r.game.players[0].battle_area[0].synth_identity(&r.game.card_data,&r.game.modifiers,h).kind,CardKind::Tamer);}
}
#[test]
fn security_reveal_and_deck_result_cards_cannot_use_hand_only_tamer_route() {
    use digimon_engine::enums::{CardSourceRef,CostDelta,PlaySource};
    for zone in 0..3 {
        let mut r=setup(false);let h=r.place_on_field(0,"ST1-12",Some(0));let card=r.game.players[0].hand.remove(0);let handle=card.handle();
        let source=match zone {0=>{let i=r.game.players[0].security.len();r.game.players[0].security.push(card);CardSourceRef::Security(0,i)},1=>{r.game.revealed_cards.push(card);CardSourceRef::Reveal(handle)},_=>{r.game.players[0].deck.push(card);CardSourceRef::DeckTop(0)}};
        assert!(!r.game.effect_initiated_digivolve_from_source(0,source,h,CostDelta::Reduce(0),false,PlaySource::ByEffect));assert_eq!(r.memory(),5);assert!(r.pending_selection().is_none());assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data),"ST1-12");
    }
}
#[test]
fn two_red_tamer_hosts_remain_distinct_native_mask_choices() {
    let mut r=setup(false);r.place_on_field(0,"ST1-12",Some(0));r.place_on_field(0,"TEST-MULTICOLOR-TAMER",Some(0));
    let saved=r.game.clone();
    for field in 0usize..2 {
        r.game=saved.clone();let a=encode_digivolve(0,field as u16);assert_eq!(build_action_mask(&r.game,0)[a as usize],1.0);r.game.decode_action(a,0);
        assert_eq!(r.game.players[0].battle_area[field].top_card().card_id(&r.game.card_data),"BT4-011");assert_eq!(r.game.players[0].battle_area[1-field].card_sources.len(),1);
    }
}
