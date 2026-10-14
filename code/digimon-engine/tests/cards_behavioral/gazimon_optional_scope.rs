//! Printed scope regression through public trigger collector/drainer.
//! YAML only, both seats. No direct run_steps or effect closure invocation.
use digimon_engine::action::{build_action_mask,space::PASS};
use digimon_engine::debug_runner::{DebugRunner,make_test_card};
use digimon_engine::enums::{EffectTiming,GamePhase};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind,TriggerSource};
const YAML:&str=include_str!("../../cards/bt25/BT25-078.yaml");

fn setup(seat:u8,target:&str)->(DebugRunner,PermanentHandle){
    let mut text=make_test_card("TEXT","Effect text only");text.effect_text="Three Musketeers".into();
    let mut trait_card=make_test_card("TRAIT","Trait only");trait_card.traits.push("Three Musketeers".into());
    let name=make_test_card("NAME","Three Musketeers name only");
    let f=make_test_card("FILL","Filler");let f2=make_test_card("FILL2","Other filler");
    let mut r=DebugRunner::builder().from_dsl_yaml(YAML).unwrap().add_card(text).add_card(trait_card).add_card(name).add_card(f).add_card(f2)
        .deck(seat,&["FILL2","FILL",target]).deck(1-seat,&["FILL";3]).build();
    r.game.turn_player_idx=seat as usize;r.game.current_phase=GamePhase::Main;r.game.set_memory(10);
    let h=r.place_on_field(seat,"BT25-078",None);(r,h)
}
fn queue(r:&mut DebugRunner,h:PermanentHandle,t:EffectTiming){
    let src=if t==EffectTiming::OnMove{TriggerSource::MovedFromBreeding{player:h.player,permanent:h,card:r.top_card(h)}}else{TriggerSource::Permanent(h)};
    r.game.enqueue_triggered(t,src);assert_eq!(r.game.effect_queue.len(),1);r.game.drain_effect_queue();
    println!("DISPATCH seat={} timing={:?} deck={} revealed={} PASS={} pending={:?}",h.player,t,r.game.players[h.player as usize].deck.len(),r.game.revealed_cards.len(),build_action_mask(&r.game,h.player)[PASS as usize],r.pending_selection_view());
}
fn masked(r:&mut DebugRunner,seat:u8,a:u16){
    assert_eq!(build_action_mask(&r.game,seat)[a as usize],1.0);
    assert_eq!(build_action_mask(&r.game,1-seat)[a as usize],0.0);
    println!("MASK seat={seat} action={a} pending={:?}",r.pending_selection_view());r.game.decode_action(a,seat);
}
// Isolation helper ONLY for non-outer-gate tests. Accept existing erroneous
// Replacement first so another defect cannot hide placement/filter outcomes.
fn accept_legacy_outer_for_isolation(r:&mut DebugRunner,s:u8){
    if r.pending_kind()==Some(SelectionKind::Replacement){let v=r.pending_selection_view().unwrap();assert!(v.is_optional);assert_eq!(v.valid_action_ids.len(),1);masked(r,s,v.valid_action_ids[0]);}
    assert_eq!(r.game.revealed_cards.len(),3);assert_eq!(r.game.players[s as usize].deck.len(),0);
}
fn branch(r:&mut DebugRunner,s:u8,b:usize){
    let v=r.pending_selection_view().unwrap();assert_eq!(v.kind,SelectionKind::EffectChoice);assert_eq!(v.selecting_player,s);assert!(!v.is_optional);assert_eq!(v.valid_action_ids.len(),2);assert_eq!(build_action_mask(&r.game,s)[PASS as usize],0.0);masked(r,s,v.valid_action_ids[b]);
}
fn order(r:&mut DebugRunner,s:u8,count:u8){
    for n in(1..=count).rev(){let v=r.pending_selection_view().expect("all remaining cards need explicit ordering");assert_eq!(v.kind,SelectionKind::OrderedPermutation{remaining:n});assert_eq!(v.selecting_player,s);assert!(!v.is_optional);assert_eq!(v.valid_action_ids.len(),n as usize);assert_eq!(build_action_mask(&r.game,s)[PASS as usize],0.0);masked(r,s,*v.valid_action_ids.last().unwrap());}
    assert!(r.pending_selection().is_none());assert!(r.game.effect_queue.is_empty());assert!(r.game.revealed_cards.is_empty());
}
fn mandatory(s:u8,t:EffectTiming){let(mut r,h)=setup(s,"TEXT");queue(&mut r,h,t);assert_eq!(build_action_mask(&r.game,s)[PASS as usize],0.0,"OUTER_OPTIONAL_GATE: printed reveal cannot be declined before seeing cards");assert_ne!(r.pending_kind(),Some(SelectionKind::Replacement));assert_eq!(r.game.revealed_cards.len(),3);branch(&mut r,s,0);let v=r.pending_selection_view().unwrap();masked(&mut r,s,v.valid_action_ids[0]);order(&mut r,s,2);assert_eq!(r.game.players[s as usize].hand.len(),1);}
fn local_decline(s:u8){let(mut r,h)=setup(s,"TRAIT");queue(&mut r,h,EffectTiming::OnPlay);accept_legacy_outer_for_isolation(&mut r,s);branch(&mut r,s,1);let v=r.pending_selection_view().unwrap();assert_eq!(v.kind,SelectionKind::Reveal);assert_eq!(v.valid_action_ids.len(),1);assert_eq!(build_action_mask(&r.game,s)[PASS as usize],1.0,"PLACEMENT_NOT_OPTIONAL: printed you may place needs branch-local decline");assert!(v.is_optional);masked(&mut r,s,PASS);order(&mut r,s,3);assert_eq!(r.game.players[s as usize].deck.len(),3);assert!(r.game.players[s as usize].hand.is_empty());assert_eq!(r.game.players[s as usize].battle_area[h.index as usize].card_sources.len(),1);}
fn hand(s:u8,target:&str){let(mut r,h)=setup(s,target);queue(&mut r,h,EffectTiming::OnPlay);accept_legacy_outer_for_isolation(&mut r,s);branch(&mut r,s,0);let v=r.pending_selection_view().unwrap();assert_eq!(v.kind,SelectionKind::Reveal,"IN_TEXT_SCOPE: Q6379 includes name/traits, not only effect text");assert_eq!(v.valid_action_ids.len(),1);assert!(!v.is_optional);assert_eq!(build_action_mask(&r.game,s)[PASS as usize],0.0);masked(&mut r,s,v.valid_action_ids[0]);order(&mut r,s,2);assert_eq!(r.game.players[s as usize].hand.len(),1);assert_eq!(r.game.players[s as usize].hand[0].card_id(&r.game.card_data),target);assert_eq!(r.game.players[s as usize].battle_area[h.index as usize].card_sources.len(),1);}
fn source(s:u8){let(mut r,h)=setup(s,"TRAIT");queue(&mut r,h,EffectTiming::OnMove);accept_legacy_outer_for_isolation(&mut r,s);branch(&mut r,s,1);let v=r.pending_selection_view().unwrap();assert_eq!(v.kind,SelectionKind::Reveal);assert_eq!(v.valid_action_ids.len(),1);masked(&mut r,s,v.valid_action_ids[0]);order(&mut r,s,2);let sources=&r.game.players[s as usize].battle_area[h.index as usize].card_sources;assert_eq!(sources.len(),2);assert_eq!(sources[0].card_id(&r.game.card_data),"TRAIT");assert!(r.game.players[s as usize].hand.is_empty());}
fn no_pick(s:u8,target:&str,b:usize){let(mut r,h)=setup(s,target);queue(&mut r,h,EffectTiming::OnPlay);accept_legacy_outer_for_isolation(&mut r,s);branch(&mut r,s,b);assert_ne!(r.pending_kind(),Some(SelectionKind::Reveal),"no eligible card means natural fizzle, not fabricated target");order(&mut r,s,3);assert_eq!(r.game.players[s as usize].deck.len(),3);assert!(r.game.players[s as usize].hand.is_empty());assert_eq!(r.game.players[s as usize].battle_area[h.index as usize].card_sources.len(),1);}

#[test]fn mandatory_reveal_play_s0(){mandatory(0,EffectTiming::OnPlay)}
#[test]fn mandatory_reveal_play_s1(){mandatory(1,EffectTiming::OnPlay)}
#[test]fn mandatory_reveal_move_s0(){mandatory(0,EffectTiming::OnMove)}
#[test]fn mandatory_reveal_move_s1(){mandatory(1,EffectTiming::OnMove)}
#[test]fn placement_decline_s0(){local_decline(0)}
#[test]fn placement_decline_s1(){local_decline(1)}
#[test]fn hand_effect_only_s0(){hand(0,"TEXT")}
#[test]fn hand_effect_only_s1(){hand(1,"TEXT")}
#[test]fn hand_trait_only_s0(){hand(0,"TRAIT")}
#[test]fn hand_trait_only_s1(){hand(1,"TRAIT")}
#[test]fn hand_name_only_s0(){hand(0,"NAME")}
#[test]fn hand_name_only_s1(){hand(1,"NAME")}
#[test]fn source_trait_only_s0(){source(0)}
#[test]fn source_trait_only_s1(){source(1)}
#[test]fn source_rejects_effect_only_s0(){no_pick(0,"TEXT",1)}
#[test]fn source_rejects_effect_only_s1(){no_pick(1,"TEXT",1)}
#[test]fn no_eligible_hand_s0(){no_pick(0,"FILL",0)}
#[test]fn no_eligible_hand_s1(){no_pick(1,"FILL",0)}
#[test]fn no_eligible_source_s0(){no_pick(0,"FILL",1)}
#[test]fn no_eligible_source_s1(){no_pick(1,"FILL",1)}
