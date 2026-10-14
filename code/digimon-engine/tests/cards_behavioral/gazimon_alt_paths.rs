//! Real masked breeding-area digivolution, no direct predicate invocation.
//! Runtime origin loads runtime CardData metadata before the same YAML DSL;
//! it is not an independent script/effect implementation.
use digimon_engine::action::{build_action_mask,space::{encode_digivolve,BREEDING_TARGET}};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{DebugRunner,make_test_card};
use digimon_engine::enums::{CardColor,CardKind,GamePhase};
use std::collections::HashMap;
const YAML:&str=include_str!("../../cards/bt25/BT25-078.yaml");
const RUNTIME:&str=include_str!("../fixtures/gazimon_alt_runtime.json");
fn case(s:u8,runtime:bool,kind:&str,expected:bool){
    let mut egg=make_test_card("EGG","Unrelated egg");egg.card_kind=CardKind::DigiEgg;egg.level=Some(2);egg.colors=vec![CardColor::Yellow];egg.dp=None;
    match kind{
        "name"=>egg.card_name="Three Musketeers name only".into(),
        "trait"=>egg.traits.push("Three Musketeers".into()),
        "effect"=>egg.effect_text="Three Musketeers effect only".into(),
        "ts"=>egg.traits.push("TS".into()),
        "none"=>{},
        "wrong_level"=>{egg.card_kind=CardKind::Digimon;egg.level=Some(3);egg.effect_text="Three Musketeers".into();},
        "wrong_level_purple"=>{egg.card_kind=CardKind::Digimon;egg.level=Some(3);egg.colors=vec![CardColor::Purple];egg.traits.push("TS".into());},
        "purple"=>egg.colors=vec![CardColor::Purple],
        _=>panic!("unknown fixture"),
    }
    let mut b=DebugRunner::builder();
    if runtime{let data:HashMap<String,CardData>=CardData::load_from_str(RUNTIME).unwrap();b=b.with_card_data(data);}
    let mut r=b.from_dsl_yaml(YAML).unwrap().add_card(egg).add_card(make_test_card("FILL","Draw card"))
        .hand(s,&["BT25-078"]).deck(s,&["FILL"]).deck(1-s,&["FILL";2]).build();
    r.game.turn_player_idx=s as usize;r.game.memory_pair=(s,1-s);r.game.memory=5;r.game.turn_count=2;r.game.current_phase=GamePhase::Main;
    let source=r.place_in_breeding(s,"EGG").card;
    let action=encode_digivolve(0,BREEDING_TARGET);let before=build_action_mask(&r.game,s)[action as usize];
    println!("ALT_MASK seat={s} runtime={runtime} fixture={kind} expected={expected} actual={before} action={action} phase={:?}",r.game.current_phase);
    assert_eq!(r.game.turn_player_idx,s as usize);
    assert_eq!(before,if expected{1.0}else{0.0},"PRINTED_ALT_IN_TEXT: Lv2 name/traits qualify per Q6379; wrong level must not qualify");
    if !expected{assert_eq!(r.game.players[s as usize].hand.len(),1);assert_eq!(r.game.players[s as usize].breeding_area.as_ref().unwrap().top_card().handle(),source);assert_eq!(r.game.memory,5);return;}
    // Main masks are seat-relative outside a prompt; actor ownership is the
    // explicit turn check above. No out-of-turn action is submitted.
    r.game.decode_action(action,s);
    assert!(r.pending_selection().is_none(),"zero-cost evolution should finish without an unresolved hidden choice");
    let p=&r.game.players[s as usize];let perm=p.breeding_area.as_ref().unwrap();
    assert_eq!(perm.card_sources.len(),2);assert_eq!(perm.card_sources[0].handle(),source);
    assert_eq!(perm.top_card().card_id(&r.game.card_data),"BT25-078");
    assert_eq!(p.hand.len(),1,"evolving draws exactly one card");assert_eq!(p.hand[0].card_id(&r.game.card_data),"FILL");assert!(p.deck.is_empty());
    assert_eq!(r.game.memory,5,"printed route pays exactly0");assert!(r.game.effect_queue.is_empty());
}

#[test]fn name_yaml_s0(){case(0,false,"name",true)}

#[test]fn name_yaml_s1(){case(1,false,"name",true)}

#[test]fn name_runtime_s0(){case(0,true,"name",true)}

#[test]fn name_runtime_s1(){case(1,true,"name",true)}

#[test]fn trait_yaml_s0(){case(0,false,"trait",true)}

#[test]fn trait_yaml_s1(){case(1,false,"trait",true)}

#[test]fn trait_runtime_s0(){case(0,true,"trait",true)}

#[test]fn trait_runtime_s1(){case(1,true,"trait",true)}

#[test]fn effect_yaml_s0(){case(0,false,"effect",true)}

#[test]fn effect_yaml_s1(){case(1,false,"effect",true)}

#[test]fn effect_runtime_s0(){case(0,true,"effect",true)}

#[test]fn effect_runtime_s1(){case(1,true,"effect",true)}

#[test]fn ts_yaml_s0(){case(0,false,"ts",true)}

#[test]fn ts_yaml_s1(){case(1,false,"ts",true)}

#[test]fn ts_runtime_s0(){case(0,true,"ts",true)}

#[test]fn ts_runtime_s1(){case(1,true,"ts",true)}

#[test]fn none_yaml_s0(){case(0,false,"none",false)}

#[test]fn none_yaml_s1(){case(1,false,"none",false)}

#[test]fn none_runtime_s0(){case(0,true,"none",false)}

#[test]fn none_runtime_s1(){case(1,true,"none",false)}

#[test]fn wrong_level_yaml_s0(){case(0,false,"wrong_level",false)}

#[test]fn wrong_level_yaml_s1(){case(1,false,"wrong_level",false)}

#[test]fn wrong_level_runtime_s0(){case(0,true,"wrong_level",false)}

#[test]fn wrong_level_runtime_s1(){case(1,true,"wrong_level",false)}

#[test]fn wrong_level_purple_yaml_s0(){case(0,false,"wrong_level_purple",false)}

#[test]fn wrong_level_purple_yaml_s1(){case(1,false,"wrong_level_purple",false)}

#[test]fn wrong_level_purple_runtime_s0(){case(0,true,"wrong_level_purple",false)}

#[test]fn wrong_level_purple_runtime_s1(){case(1,true,"wrong_level_purple",false)}

#[test]fn purple_yaml_s0(){case(0,false,"purple",true)}

#[test]fn purple_yaml_s1(){case(1,false,"purple",true)}

#[test]fn purple_runtime_s0(){case(0,true,"purple",true)}

#[test]fn purple_runtime_s1(){case(1,true,"purple",true)}
