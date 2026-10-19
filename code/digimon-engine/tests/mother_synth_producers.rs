//! Actual DSL producer boundary controls, not full-card or oracle confirmation.
use digimon_engine::{debug_runner::DebugRunner,action::{build_action_mask,space::*},enums::{CardColor,CardKind,EffectTiming},permanent::PermanentHandle,selection::TriggerSource};
fn setup()->DebugRunner {
 let mut b=DebugRunner::builder();
 for id in ["BT12-092","BT13-008","BT13-020","BT21-044","AD1-021","BT25-104","ST1-04"] {b=b.dsl_card(id).unwrap();}
 let mut r=b.deck(0,&["ST1-04";16]).deck(1,&["ST1-04";16]).memory(10).start();r.skip_mulligan();r.game.enter_main_phase();r
}
fn select(r:&mut DebugRunner,action:u16){let p=r.pending_selection().expect("source clause chooser").selecting_player;assert_eq!(build_action_mask(&r.game,p)[action as usize],1.0);r.execute_action(p,action).unwrap();}
fn assert_identity(r:&DebugRunner,h:PermanentHandle,dp:i32){
 for g in [&r.game,&r.game.clone()] {
  let id=g.players[h.player as usize].battle_area[h.index as usize].synth_identity(&g.card_data,&g.modifiers,h);
  assert_eq!(id.kind,CardKind::Digimon);assert_eq!(id.retained_kind,Some(CardKind::Tamer));
  assert_eq!(id.level,None);assert_eq!(id.colors,vec![CardColor::Yellow,CardColor::Red]);
  assert!(id.traits.contains(&"DATA SQUAD".to_string()));assert_eq!(g.effective_dp(h),Some(dp));
 }
}
fn expire(r:&mut DebugRunner,h:PermanentHandle){r.game.modifiers.expire_end_of_turn(0);let id=r.game.players[0].battle_area[h.index as usize].synth_identity(&r.game.card_data,&r.game.modifiers,h);assert_eq!(id.kind,CardKind::Tamer);assert_eq!(id.dp,None);assert_eq!(id.level,None);assert_eq!(id.colors,vec![CardColor::Yellow,CardColor::Red]);}
#[test]fn bt13_008_actual_main_retains_marcus_printed_properties(){
 let mut r=setup();let src=r.place_on_field(0,"BT13-008",Some(0));let m=r.place_on_field(0,"AD1-021",Some(0));
 let a=FIELD_EFFECT_START+src.index as u16*EFFECTS_PER_PERMANENT+FIELD_EFFECT_SLOT_FOR_MAIN;assert_eq!(build_action_mask(&r.game,0)[a as usize],1.0);r.game.decode_action(a,0);select(&mut r,ATTACK_START+m.index as u16);assert_identity(&r,m,3000);expire(&mut r,m);
}
#[test]fn bt13_020_actual_evolution_clause_play_retains_marcus_printed_properties(){
 let mut r=setup();let src=r.place_on_field(0,"BT13-020",Some(0));let hand=r.add_to_hand(0,"AD1-021");
 r.game.enqueue_triggered(EffectTiming::WhenDigivolving,TriggerSource::Permanent(src));r.game.drain_effect_queue();
 select(&mut r,hand as u16);let m=PermanentHandle{player:0,index:1};assert_identity(&r,m,12000);expire(&mut r,m);
}
#[test]fn bt21_044_actual_play_clause_retains_marcus_printed_properties(){
 let mut r=setup();let src=r.place_on_field(0,"BT21-044",Some(0));let m=r.place_on_field(0,"AD1-021",Some(0));r.fire_play_event_triggers(0,src.index as usize,false,false);
 select(&mut r,ATTACK_START+m.index as u16);assert_identity(&r,m,3000);if r.pending_selection().is_some(){select(&mut r,PASS);}expire(&mut r,m);
}
#[test]fn ad1_021_actual_end_turn_clause_retains_its_printed_properties(){
 let mut r=setup();let m=r.place_on_field(0,"AD1-021",Some(0));r.place_on_field(0,"BT13-008",Some(0));
 r.game.enqueue_triggered(EffectTiming::EndOfYourTurn,TriggerSource::Permanent(m));r.game.drain_effect_queue();select(&mut r,ATTACK_START+m.index as u16);assert_identity(&r,m,6000);if r.pending_selection().is_some(){select(&mut r,PASS);}expire(&mut r,m);
}
#[test]fn bt25_104_actual_aura_retains_marcus_properties_and_tracks_turn(){
 let mut r=setup();r.place_on_field(0,"BT25-104",Some(0));let m=r.place_on_field(0,"AD1-021",Some(0));r.game.tick_declarative_effects();assert_identity(&r,m,12000);
 r.game.turn_player_idx=1;r.game.tick_declarative_effects();let id=r.game.players[0].battle_area[m.index as usize].synth_identity(&r.game.card_data,&r.game.modifiers,m);assert_eq!(id.kind,CardKind::Tamer);assert_eq!(id.dp,None);
}

#[test]fn q6019_newer_actual_dp_treatment_overwrites_earlier_grant(){
 let mut r=setup();let a=r.place_on_field(0,"BT13-008",Some(0));let m=r.place_on_field(0,"AD1-021",Some(0));
 let action=FIELD_EFFECT_START+a.index as u16*EFFECTS_PER_PERMANENT+FIELD_EFFECT_SLOT_FOR_MAIN;assert_eq!(build_action_mask(&r.game,0)[action as usize],1.0);r.game.decode_action(action,0);select(&mut r,ATTACK_START+m.index as u16);assert_identity(&r,m,3000);
 let saved=r.game.clone();let mut hashes=vec![];
 for _ in 0..2 {r.game=saved.clone();r.game.enqueue_triggered(EffectTiming::EndOfYourTurn,TriggerSource::Permanent(m));r.game.drain_effect_queue();select(&mut r,ATTACK_START+m.index as u16);assert_identity(&r,m,6000);hashes.push(r.game.verification_digest());}
 assert_eq!(hashes[0],hashes[1]);
}
