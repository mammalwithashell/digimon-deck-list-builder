use digimon_engine::{debug_runner::DebugRunner,card_data::EvoCost,enums::GamePhase,action::{build_action_mask,encode_digivolve,space::PASS}};
const CARD:&str=r#"
card: TEST-TREATED-ROUTES
name: Treated Route Fixture
kind: digimon
level: 4
color: [red]
cost: 5
dp: 5000
alt_paths:
  - kind: digivolve
    from: {kind: tamer, color_is: red}
    cost: 2
  - kind: digivolve
    from: {kind: tamer, color_is: red}
    source_treated_as: level_3_red_digimon
    cost: 2
"#;
const OBS:&str=r#"
card: TEST-ROUTE-OBSERVER
name: Observer
kind: tamer
color: [white]
cost: 2
effects:
  - kind: replacement
    trigger: when_would_digivolve
    active_when: {kind: digimon}
    optional: true
    process:
      - draw: {of: you, count: 1}
"#;
#[test]
fn effect_fixed_cost_routes_keep_distinct_identity_and_visible_labels() {
    let mut r=DebugRunner::builder().from_dsl_yaml(CARD).unwrap().from_dsl_yaml(OBS).unwrap()
        .from_dsl_yaml(include_str!("../cards/st1/ST1-12.yaml")).unwrap()
        .from_dsl_yaml(include_str!("../cards/st1/ST1-03.yaml")).unwrap()
        .hand(0,&["TEST-TREATED-ROUTES"]).deck(0,&["ST1-03";12]).deck(1,&["ST1-03";12]).memory(5).start();
    r.game.card_data.iter_mut().find(|c|c.card_id=="TEST-TREATED-ROUTES").unwrap().evo_costs=vec![EvoCost{card_color:0,level:3,memory_cost:2}];
    r.place_on_field(0,"ST1-12",None);r.place_on_field(0,"TEST-ROUTE-OBSERVER",None);
    r.game.current_phase=GamePhase::Main;r.game.tick_declarative_effects();
    let a=encode_digivolve(0,0);assert_eq!(build_action_mask(&r.game,0)[a as usize],1.0);let _=r.game.effect_initiated_digivolve(0,0,digimon_engine::permanent::PermanentHandle{player:0,index:0},digimon_engine::enums::CostDelta::Fixed(1),false,digimon_engine::enums::PlaySource::ByEffect);
    let rows=r.game.pending_selection.as_ref().expect("same price does not erase distinct source identity routes").effect_choices.as_ref().unwrap().clone();
    assert_eq!(rows.len(),2);assert_ne!(rows[0].label,rows[1].label);
    assert_eq!(build_action_mask(&r.game,0)[PASS as usize],0.0);
    for row in rows {
        let mut g=r.game.clone();let mut twin=g.clone();assert_eq!(build_action_mask(&g,0)[row.action_id as usize],1.0);
        g.resolve_selection(0,row.action_id).unwrap();twin.resolve_selection(0,row.action_id).unwrap();
        assert_eq!(g.verification_digest(),twin.verification_digest());
        if row.label.contains("treated") {assert!(g.pending_selection.is_some(),"Digimon observer belongs to treated route only");}
        else {assert!(g.pending_selection.is_none());assert_eq!(g.memory,4);}
    }
}

#[test]
fn native_interleaved_equal_cost_routes_deduplicate_only_equal_identity() {
    let yaml=format!("{}{}",CARD,"  - kind: digivolve\n    from: {kind: tamer, color_is: red}\n    cost: 2\n");
    let mut r=DebugRunner::builder().from_dsl_yaml(&yaml).unwrap()
        .from_dsl_yaml(include_str!("../cards/st1/ST1-12.yaml")).unwrap()
        .from_dsl_yaml(include_str!("../cards/st1/ST1-03.yaml")).unwrap()
        .hand(0,&["TEST-TREATED-ROUTES"]).deck(0,&["ST1-03";12]).deck(1,&["ST1-03";12]).memory(5).start();
    r.game.card_data.iter_mut().find(|c|c.card_id=="TEST-TREATED-ROUTES").unwrap().evo_costs=vec![EvoCost{card_color:0,level:3,memory_cost:2}];
    r.place_on_field(0,"ST1-12",Some(0));r.game.current_phase=GamePhase::Main;
    let a=encode_digivolve(0,0);assert_eq!(build_action_mask(&r.game,0)[a as usize],1.0);r.game.decode_action(a,0);
    let rows=r.pending_selection_view().expect("two semantically different routes").effect_choices.unwrap();
    assert_eq!(rows.len(),2,"plain, treated, plain must coalesce the duplicate plain route");
    assert_eq!(build_action_mask(&r.game,0)[PASS as usize],0.0,"Q4633: declared evolution cannot be declined");
    for row in rows {let mut g=r.game.clone();assert_eq!(build_action_mask(&g,0)[row.action_id as usize],1.0);g.resolve_selection(0,row.action_id).unwrap();assert_eq!(g.memory,3);assert!(g.pending_selection.is_none());}
}
