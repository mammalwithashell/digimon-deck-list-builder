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
use digimon_engine::{action::{build_action_mask, encode_digivolve, space::PLAY_HAND_START},
    card_data::EvoCost, debug_runner::DebugRunner, enums::{CardKind, GamePhase}};
fn setup(option: bool) -> DebugRunner { setup_observer(option, OBSERVER) }
fn setup_observer(option: bool, observer: &str) -> DebugRunner {
    let path = std::env::var("BT4_011_TEST_YAML").unwrap_or_else(|_|
        format!("{}/cards/bt4/BT4-011.yaml", env!("CARGO_MANIFEST_DIR")));
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(&std::fs::read_to_string(path).unwrap()).unwrap()
        .from_dsl_yaml(include_str!("../cards/st1/ST1-12.yaml")).unwrap()
        .from_dsl_yaml(include_str!("../cards/st2/ST2-12.yaml")).unwrap()
        .from_dsl_yaml(include_str!("../cards/st1/ST1-03.yaml")).unwrap()
        .from_dsl_yaml(include_str!("../cards/LM/LM-027.yaml")).unwrap()
        .from_dsl_yaml(observer).unwrap()
        .hand(0, if option { &["LM-027", "BT4-011"][..] } else { &["BT4-011"][..] })
        .deck(0, &["ST1-03"; 12]).deck(1, &["ST1-03";12])
        .memory(5).start();
    // Exact printed circle; DebugRunner compiled metadata omits compatibility evo_costs.
    r.game.card_data.iter_mut().find(|c|c.card_id=="BT4-011").unwrap().evo_costs =
        vec![EvoCost {card_color:0, level:3, memory_cost:2}];
    r.game.current_phase = GamePhase::Main;
    r
}



const HOST:&str=r#"
card: TEST-TAIL-HOST
name: Tail Host
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          bind_as: evo
          prompt: Choose the result
          filter: {card_number_is: BT4-011}
      - effect_initiated_digivolve:
          target: source
          from_hand: evo
          cost: {reduce: 0}
      - if:
          condition: {effect_digivolved_any_digimon: true}
          then:
            - gain_memory: 3
      - draw: {of: you, count: 2}
"#;
const WOULD:&str=r#"
  - kind: replacement
    trigger: when_would_digivolve
    optional: true
    active_when: {kind: digimon}
    process:
      - draw: {of: you, count: 1}
"#;
#[test]
fn initiating_dsl_tail_waits_for_cost_and_would_window_and_runs_once() {
    use digimon_engine::action::space::{PASS,REPLACEMENT_ACCEPT};
    for cancel in [false,true] {
        let would=if cancel {WOULD.replace("draw: {of: you, count: 1}","cancel_replacement: {}")} else {WOULD.to_string()};
        let observer=format!("{OBSERVER}{would}");
        // Add the initiating clause before any field/cached effect construction.
        let path=format!("{}/cards/bt4/BT4-011.yaml",env!("CARGO_MANIFEST_DIR"));
        let mut r=DebugRunner::builder().from_dsl_yaml(&std::fs::read_to_string(path).unwrap()).unwrap()
            .from_dsl_yaml(HOST).unwrap().from_dsl_yaml(&observer).unwrap()
            .from_dsl_yaml(include_str!("../cards/st1/ST1-03.yaml")).unwrap()
            .hand(0,&["BT4-011"]).deck(0,&["ST1-03";12]).deck(1,&["ST1-03";12]).memory(5).start();
        r.game.current_phase=GamePhase::Main;
        r.game.card_data.iter_mut().find(|c|c.card_id=="BT4-011").unwrap().evo_costs=vec![EvoCost{card_color:0,level:3,memory_cost:2}];
        let host=r.place_on_field(0,"TEST-TAIL-HOST",None);r.place_on_field(0,"TEST-Q1157-OBSERVER",None);
        r.fire_play_event_triggers(0,host.index as usize,true,false);
        let hand=*r.game.pending_selection.as_ref().unwrap().valid_action_ids.iter().find(|a|**a!=PASS).unwrap();r.game.resolve_selection(0,hand).unwrap();
        let cost_accept=*r.game.pending_selection.as_ref().expect("cost decision").valid_action_ids.iter().find(|a|**a!=PASS).unwrap();
        assert_eq!(r.memory(),5);assert_eq!(r.game.players[0].hand.len(),1);
        for cost_choice in [PASS,cost_accept] {
            let mut g=r.game.clone();g.resolve_selection(0,cost_choice).unwrap();
            assert!(g.pending_selection.is_some(),"would decision after cost decision");assert_eq!(g.memory,5);assert_eq!(g.players[0].hand.len(),1);
            for would_choice in [PASS,REPLACEMENT_ACCEPT] {
                let mut branch=g.clone();let mut twin=branch.clone();
                assert_eq!(build_action_mask(&branch,0)[would_choice as usize],1.0);
                branch.resolve_selection(0,would_choice).unwrap();twin.resolve_selection(0,would_choice).unwrap();
                assert_eq!(branch.verification_digest(),twin.verification_digest());assert!(branch.pending_selection.is_none());
                let succeeded=!(cancel && would_choice==REPLACEMENT_ACCEPT);
                assert_eq!(branch.memory,if succeeded {6+i16::from(cost_choice!=PASS)}else{5});
                assert_eq!(branch.players[0].hand.len(),3+usize::from(!cancel && would_choice==REPLACEMENT_ACCEPT));
                assert_eq!(branch.players[0].battle_area[0].top_card().card_id(&branch.card_data),if succeeded {"BT4-011"}else{"TEST-TAIL-HOST"});
            }
        }
    }
}
