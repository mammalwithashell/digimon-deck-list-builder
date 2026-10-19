//! Q1161/Q1162: Tamer source inherited and security scopes remain distinct.
//! SecuritySkill injection below is a dispatcher negative control, not an oracle line.
use digimon_engine::{debug_runner::DebugRunner,action::{build_action_mask,encode_digivolve},enums::{EffectTiming,GamePhase},selection::TriggerSource,card_data::EvoCost};
const TAMER:&str=r#"
card: TEST-HYBRID-TAMER
name: Hybrid Tamer Scope Fixture
kind: tamer
color: [red]
cost: 2
effects:
  - scope: inherited
    when: when_attacking
    process:
      - gain_memory: 1
  - when: on_security
    process:
      - gain_memory: 7
"#;
fn evolved()->(DebugRunner,digimon_engine::permanent::PermanentHandle) {
    let mut r=DebugRunner::builder().from_dsl_yaml(TAMER).unwrap().dsl_card("BT4-011").unwrap().dsl_card("ST1-03").unwrap()
        .hand(0,&["BT4-011"]).deck(0,&["ST1-03";12]).deck(1,&["ST1-03";12]).security(0,&["TEST-HYBRID-TAMER"]).memory(5).start();
    r.game.card_data.iter_mut().find(|c|c.card_id=="BT4-011").unwrap().evo_costs=vec![EvoCost{card_color:0,level:3,memory_cost:2}];
    r.skip_mulligan();r.game.current_phase=GamePhase::Main;let h=r.place_on_field(0,"TEST-HYBRID-TAMER",Some(0));let a=encode_digivolve(0,0);assert_eq!(build_action_mask(&r.game,0)[a as usize],1.0);r.game.decode_action(a,0);assert_eq!(r.memory(),3);(r,h)
}
#[test]
fn q1162_tamer_source_inherited_clause_dispatches_after_hybrid_evolution() {
    let (mut r,h)=evolved();let saved=r.game.clone();
    for _ in 0..2 {r.game=saved.clone();r.game.enqueue_triggered(EffectTiming::WhenAttacking,TriggerSource::Permanent(h));r.game.drain_effect_queue();assert_eq!(r.memory(),4);}
}
#[test]
fn q1161_tamer_source_security_clause_is_not_gained_but_security_dispatch_remains_valid() {
    let (mut r,h)=evolved();r.game.enqueue_triggered(EffectTiming::SecuritySkill,TriggerSource::Permanent(h));r.game.drain_effect_queue();assert_eq!(r.memory(),3,"buried security effect is not inherited");
    let revealed=r.game.players[0].security.pop().unwrap();let card=revealed.handle();r.game.pending_security=Some(digimon_engine::selection::PendingSecurity{defender:0,card:revealed,played:false});r.game.enqueue_triggered(EffectTiming::SecuritySkill,TriggerSource::SecurityRevealed{defender:0,card});r.game.drain_effect_queue();assert_eq!(r.memory(),10,"the same clause works in its actual security scope");
}
