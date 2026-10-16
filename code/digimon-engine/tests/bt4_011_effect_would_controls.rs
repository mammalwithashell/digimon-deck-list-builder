//! Official Q1157: the would-digivolve event must classify the declared Hybrid base as Digimon.
const OBSERVER: &str = r#"
card: TEST-Q1157-OBSERVER
name: Would Evolve Observer
kind: tamer
color: [white]
cost: 2
effects:
  - kind: replacement
    trigger: when_would_digivolve
    optional: true
    active_when: { kind: digimon }
    process:
      - draw: {of: you, count: 1}
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

fn assert_would_event(base: &str) {
    use digimon_engine::action::space::{REPLACEMENT_ACCEPT,PASS};
    let mut r=setup(false);r.place_on_field(0,base,None);
    r.place_on_field(0,"TEST-Q1157-OBSERVER",None);
    r.game.tick_declarative_effects();
    let a=encode_digivolve(0,0);assert_eq!(build_action_mask(&r.game,0)[a as usize],1.0);
    begin_effect(&mut r);
    assert!(r.game.pending_selection.is_some(),"Q1157 Digimon would-evolve observer must be offered for the declared Hybrid base");
    if base=="ST1-12" {
        let h=digimon_engine::permanent::PermanentHandle{player:0,index:0};
        let identity=r.game.players[0].battle_area[0].synth_identity(&r.game.card_data,&r.game.modifiers,h);
        assert_eq!(identity.kind,CardKind::Digimon);assert_eq!(identity.level,Some(3));
        assert_eq!(identity.colors,vec![digimon_engine::enums::CardColor::Red]);
        assert_eq!(identity.dp,None,"treatment does not give the Tamer a fake DP");
    }
    let m=build_action_mask(&r.game,0);
    assert_eq!(m[REPLACEMENT_ACCEPT as usize],1.0);assert_eq!(m[PASS as usize],1.0);
    for (choice,hand) in [(REPLACEMENT_ACCEPT,2),(PASS,1)] {
        let mut g=r.game.clone();let mut twin=g.clone();
        g.resolve_selection(0,choice).unwrap();twin.resolve_selection(0,choice).unwrap();
        assert_eq!(g.verification_digest(),twin.verification_digest());
        assert!(g.pending_selection.is_none());
        let h=digimon_engine::permanent::PermanentHandle{player:0,index:0};
        let identity=g.players[0].battle_area[0].synth_identity(&g.card_data,&g.modifiers,h);
        assert_eq!(identity.level,Some(4));assert_eq!(identity.dp,Some(5000));
        assert!(g.modifiers.get(h,digimon_engine::enums::ModifierType::TreatAsDigimon).is_empty());
        assert!(g.modifiers.get(h,digimon_engine::enums::ModifierType::DontHaveDp).is_empty());

        assert_eq!(g.players[0].hand.len(),hand,"observer draw plus normal evolution bonus; decline pays no observer effect");
        assert_eq!(g.players[0].battle_area[0].top_card().card_id(&g.card_data),"BT4-011");
        assert_eq!(g.card_data[g.players[0].battle_area[0].card_sources[0].data_index].card_kind,
            if base=="ST1-12" {CardKind::Tamer} else {CardKind::Digimon},"printed source kind must remain unchanged after resumed evolution");
    }
}
#[test]
fn effect_q1157_would_event_ordinary_digimon_control() {assert_would_event("ST1-03");}
#[test]
fn effect_q1157_would_event_treated_tamer_accept_decline_and_clones() {assert_would_event("ST1-12");}

#[test]
fn effect_q1157_cancelled_window_restores_tamer_and_preserves_other_modifiers() {
    use digimon_engine::{action::space::REPLACEMENT_ACCEPT, enums::{ModifierType,Expiry},modifiers::ModifierEntry};
    let text=OBSERVER.replace("draw: {of: you, count: 1}","cancel_replacement: {}");
    let mut r=setup_observer(false,&text);let h=r.place_on_field(0,"ST1-12",None);
    r.place_on_field(0,"TEST-Q1157-OBSERVER",None);
    r.game.modifiers.add(h,ModifierEntry::simple(ModifierType::CannotAttack,0,Expiry::EndOfTurn,0));
    r.game.tick_declarative_effects();let a=encode_digivolve(0,0);
    assert_eq!(build_action_mask(&r.game,0)[a as usize],1.0);begin_effect(&mut r);
    assert!(r.game.pending_selection.is_some());
    let mut twin=r.game.clone();r.game.resolve_selection(0,REPLACEMENT_ACCEPT).unwrap();twin.resolve_selection(0,REPLACEMENT_ACCEPT).unwrap();
    assert_eq!(r.game.verification_digest(),twin.verification_digest());
    let id=r.game.players[0].battle_area[0].synth_identity(&r.game.card_data,&r.game.modifiers,h);
    assert_eq!(id.kind,CardKind::Tamer);assert_eq!(id.level,None);assert_eq!(id.dp,None);
    assert!(r.game.modifiers.has(h,ModifierType::CannotAttack));
    assert!(r.game.modifiers.get(h,ModifierType::TreatAsDigimon).is_empty());
    assert!(r.game.modifiers.get(h,ModifierType::DontHaveDp).is_empty());
    assert_eq!(r.memory(),5);assert_eq!(r.game.players[0].hand.len(),1);
}
#[test]
fn effect_q1157_terminal_window_restores_printed_tamer_identity() {
    let mut r=setup(false);let h=r.place_on_field(0,"ST1-12",None);
    r.place_on_field(0,"TEST-Q1157-OBSERVER",None);r.game.tick_declarative_effects();
    begin_effect(&mut r);assert!(r.game.pending_selection.is_some());
    r.game.declare_winner(0);
    let id=r.game.players[0].battle_area[0].synth_identity(&r.game.card_data,&r.game.modifiers,h);
    assert_eq!(id.kind,CardKind::Tamer);assert_eq!(id.level,None);assert_eq!(id.dp,None);
    assert!(r.game.modifiers.get(h,digimon_engine::enums::ModifierType::TreatAsDigimon).is_empty());
    assert_eq!(r.game.verification_digest(),r.game.clone().verification_digest());
}
#[test]
fn effect_q1157_departed_subject_does_not_evolve_or_leak_identity() {
    use digimon_engine::action::space::REPLACEMENT_ACCEPT;
    let text=OBSERVER.replace("draw: {of: you, count: 1}","return_to_hand: {target: replacement_subject}");
    let mut r=setup_observer(false,&text);let h=r.place_on_field(0,"ST1-12",None);
    r.place_on_field(0,"TEST-Q1157-OBSERVER",None);r.game.tick_declarative_effects();
    begin_effect(&mut r);assert!(r.game.pending_selection.is_some());
    let mut twin=r.game.clone();r.game.resolve_selection(0,REPLACEMENT_ACCEPT).unwrap();twin.resolve_selection(0,REPLACEMENT_ACCEPT).unwrap();
    assert_eq!(r.game.verification_digest(),twin.verification_digest());
    assert!(r.game.pending_selection.is_none());assert_eq!(r.memory(),5);
    assert_eq!(r.game.players[0].battle_area.len(),1);
    assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data),"TEST-Q1157-OBSERVER");
    assert!(r.game.players[0].hand.iter().any(|c|c.card_id(&r.game.card_data)=="BT4-011"));
    assert!(r.game.players[0].hand.iter().any(|c|c.card_id(&r.game.card_data)=="ST1-12"));
    assert!(r.game.modifiers.get(h,digimon_engine::enums::ModifierType::TreatAsDigimon).is_empty());
}

fn begin_effect(r:&mut DebugRunner) {
    use digimon_engine::{enums::{CostDelta,PlaySource},permanent::PermanentHandle};
    let _=r.game.effect_initiated_digivolve(0,0,PermanentHandle{player:0,index:0},CostDelta::Reduce(0),false,PlaySource::ByEffect);
}
