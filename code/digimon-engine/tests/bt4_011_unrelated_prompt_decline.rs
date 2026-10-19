// A post-evolution optional effect is not the enclosing reducer payment.
include!("bt4_011_nested_abort_scope.rs");
const INNER_OPTIONAL:&str=r#"
effects:
  - when: when_digivolving
    optional: true
    process:
      - gain_memory: 1
"#;
fn setup_unrelated_prompt(option: bool, observer: &str) -> DebugRunner {
    let path = std::env::var("BT4_011_TEST_YAML").unwrap_or_else(|_|
        format!("{}/cards/bt4/BT4-011.yaml", env!("CARGO_MANIFEST_DIR")));
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(&std::fs::read_to_string(path).unwrap()).unwrap()
        .from_dsl_yaml(include_str!("../cards/st1/ST1-12.yaml")).unwrap()
        .from_dsl_yaml(include_str!("../cards/st2/ST2-12.yaml")).unwrap()
        .from_dsl_yaml(include_str!("../cards/st1/ST1-03.yaml")).unwrap()
        .from_dsl_yaml(include_str!("../cards/LM/LM-027.yaml")).unwrap()
        .from_dsl_yaml(INNER_COST).unwrap().from_dsl_yaml(observer).unwrap().from_dsl_yaml(&format!("{}{}",INNER,INNER_OPTIONAL)).unwrap().from_dsl_yaml(INNER_WOULD).unwrap()
        .hand(0, if option { &["LM-027", "BT4-011"][..] } else { &["BT4-011"][..] })
        .deck(0, &["ST1-03"; 12]).deck(1, &["ST1-03";12])
        .memory(5).start();
    // Exact printed circle; DebugRunner compiled metadata omits compatibility evo_costs.
    r.game.card_data.iter_mut().find(|c|c.card_id=="BT4-011").unwrap().evo_costs =
        vec![EvoCost {card_color:0, level:3, memory_cost:2}];
    r.game.current_phase = GamePhase::Main;
    r
}
fn unrelated_prompt(native:bool) {
    use digimon_engine::action::space::{ATTACK_START,PLAY_HAND_START,PASS};
    let mut r=setup_unrelated_prompt(false,OBSERVER);let outer=r.place_on_field(0,"ST1-12",Some(0));
    r.place_on_field(0,"ST1-03",Some(0));r.place_on_field(0,"TEST-Q1157-OBSERVER",Some(0));
    let inner=r.add_to_hand(0,"TEST-Q1157-INNER");r.game.tick_declarative_effects();
    if native {let a=encode_digivolve(0,0);assert_eq!(build_action_mask(&r.game,0)[a as usize],1.0);r.game.decode_action(a,0);}
    else {r.game.effect_initiated_digivolve(0,0,outer,digimon_engine::enums::CostDelta::Reduce(0),false,digimon_engine::enums::PlaySource::ByEffect);}
    accept(&mut r);choose(&mut r,ATTACK_START+1);choose(&mut r,PLAY_HAND_START+inner as u16);
    assert_eq!(r.game.players[0].battle_area[1].top_card().card_id(&r.game.card_data),"TEST-Q1157-INNER");
    let parked=r.game.clone();let mut digest=None;
    for _ in 0..2 {
        r.game=parked.clone();choose(&mut r,PASS);
        for _ in 0..2 {choose(&mut r,ATTACK_START+2);}
        assert!(r.pending_selection().is_none());
        assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data),"BT4-011");
        assert_eq!(r.memory(),8,"declining unrelated on-evolution gain1 cannot revoke outer reducer credit");
        let d=r.game.verification_digest();if let Some(previous)=digest {assert_eq!(d,previous);}digest=Some(d);
    }
}
#[test]fn native_outer_credit_survives_unrelated_optional_trigger_decline(){unrelated_prompt(true);}
#[test]fn effect_outer_credit_survives_unrelated_optional_trigger_decline(){unrelated_prompt(false);}
