//! Harness-only association of explicit native declarations to actual entries.
use digimon_engine::{Game,card_source::CardHandle,enums::PlayerId,game::NativePlayEntry,permanent::PermanentHandle};
#[derive(Default)]
pub(super) struct NativePlayTracking {
    active:Option<(String,PlayerId,CardHandle)>,
    committed:Vec<(String,NativePlayEntry)>,
}
impl NativePlayTracking {
    pub(super) fn collect(&mut self,game:&mut Game)->Result<(),String> {
        let entries=game.take_native_play_entries();
        let Some((label,player,card))=self.active.as_ref() else {
            if entries.is_empty(){return Ok(());}
            return Err("native play entry has no authored declaration (NOT MEASURED)".into());
        };
        if entries.is_empty(){return Ok(());}
        if entries.len()!=1||entries[0].player()!=*player||entries[0].card()!=*card {
            self.active=None;
            return Err("native play transaction did not uniquely commit its declared physical card (NOT MEASURED)".into());
        }
        self.committed.push((label.clone(),entries.into_iter().next().unwrap()));
        self.active=None;Ok(())
    }
    pub(super) fn begin(&mut self,label:String,player:PlayerId,card:CardHandle) {
        // Any previous declaration that never committed is closed here. It
        // cannot acquire the entry from a subsequent replay of the same card.
        self.active=Some((label,player,card));
    }
    pub(super) fn reference(&self,game:&Game,handle:PermanentHandle)->Option<String> {
        let hits:Vec<_>=self.committed.iter().filter(|(_,e)|game.resolve_native_play_entry(e)==Some(handle)).collect();
        match hits.as_slice(){[(label,_)]=>Some(label.clone()),_=>None}
    }
}

#[cfg(test)] mod tests {
 use super::*;
 use digimon_engine::{debug_runner::{DebugRunner,make_test_card},enums::GamePhase,action::{build_action_mask,space::PLAY_HAND_START}};
 fn fixture()->DebugRunner {let mut c=make_test_card("NATIVE-TWIN","Twin");c.play_cost=0;let mut r=DebugRunner::builder().add_card(c).hand(0,&["NATIVE-TWIN";3]).deck(0,&["NATIVE-TWIN";5]).memory(5).start();r.game.current_phase=GamePhase::Main;r.game.enable_native_play_observation(true);r}
 fn play(r:&mut DebugRunner,t:&mut NativePlayTracking,label:&str) {let c=r.game.players[0].hand[0].handle();t.begin(label.into(),0,c);assert_eq!(build_action_mask(&r.game,0)[PLAY_HAND_START as usize],1.0);r.game.decode_action(PLAY_HAND_START,0);t.collect(&mut r.game).unwrap();}
 #[test] fn two_identical_plays_bind_actual_distinct_incarnations_and_clones() {let mut r=fixture();let mut t=NativePlayTracking::default();play(&mut r,&mut t,"a");play(&mut r,&mut t,"b");for g in [&r.game,&r.game.clone()] {assert_eq!(t.reference(g,PermanentHandle{player:0,index:0}).as_deref(),Some("a"));assert_eq!(t.reference(g,PermanentHandle{player:0,index:1}).as_deref(),Some("b"));} }
 #[test] fn failed_declaration_label_cannot_acquire_next_native_play() {let mut r=fixture();let mut t=NativePlayTracking::default();t.begin("failed".into(),0,r.game.players[0].hand[0].handle());t.collect(&mut r.game).unwrap();play(&mut r,&mut t,"later");assert_eq!(t.reference(&r.game,PermanentHandle{player:0,index:0}).as_deref(),Some("later"));assert_eq!(t.committed.len(),1);}
 #[test] fn unassociated_entry_is_not_guessed_from_identity() {let mut r=fixture();let mut t=NativePlayTracking::default();r.game.decode_action(PLAY_HAND_START,0);assert!(t.collect(&mut r.game).unwrap_err().contains("no authored declaration"));assert!(t.committed.is_empty());}
}


#[cfg(test)] mod wire_binding_tests {
 use super::*;
 use digimon_engine::{debug_runner::{DebugRunner,make_test_card},enums::GamePhase,action::space::PLAY_HAND_START,effect_context::EffectContext};
 #[test] fn exact_requested_target_gets_its_original_native_reference() {
  let mut c=make_test_card("WIRE-TWIN","Twin");c.play_cost=0;let mut r=DebugRunner::builder().add_card(c).hand(0,&["WIRE-TWIN";3]).hand(1,&["WIRE-TWIN"]).memory(5).start();r.game.current_phase=GamePhase::Main;r.game.enable_native_play_observation(true);let mut tracking=NativePlayTracking::default();
  for label in ["first","second"] {let card=r.game.players[0].hand[0].handle();tracking.begin(label.into(),0,card);r.game.decode_action(PLAY_HAND_START,0);tracking.collect(&mut r.game).unwrap();}
  let source=r.game.players[1].hand[0].handle();{let mut ctx=EffectContext::new(&mut r.game,source,None,1);ctx.select_opponent_permanent("wire pick",false,|_,_|true,|_,_|{});}

  for (slot,label) in [(1,"second"),(0,"first")] {let payload:crate::exam::scenario::SelectPayload=crate::exam::scenario::SelectPayload::Targets(vec![format!("opp.field.{slot}")]);let (_,wire)=super::super::build_selection_row_tracked(&r.game,8,1,&payload,None,&tracking).unwrap();assert_eq!(wire.permanent_targets.len(),1);assert_eq!(wire.permanent_targets[0].version,3);assert_eq!(wire.permanent_targets[0].play_ref.as_deref(),Some(label));let encoded=serde_json::to_string(&wire.permanent_targets[0]).unwrap();let decoded:serde_json::Value=serde_json::from_str(&encoded).unwrap();assert_eq!(decoded["version"],3);assert_eq!(decoded["play_ref"],label);assert_eq!(decoded["owner"],0);assert_eq!(decoded["card_id"],"WIRE-TWIN");}
 }
}


#[cfg(test)] mod parked_native_tests {
 use super::*;
 use digimon_engine::{debug_runner::{DebugRunner,make_test_card},effect::{CardEffect,Effect},card_source::CardHandle,enums::GamePhase,action::{build_action_mask,space::{PLAY_HAND_START,PASS}}};
 struct CancelNative;
 impl CardEffect for CancelNative {fn effects(&self,card:CardHandle)->Vec<Effect>{vec![Effect::when_permanent_would_play(card).optional().once_per_turn().name("native association gate").replacement_condition(|ctx,subject|match subject{digimon_engine::replacement::ReplacementSubject::Card(h,_)=>ctx.game.card_data_for_handle(*h).is_some_and(|d|d.card_id=="PARK-NATIVE"),_=>false}).replacement_process(|ctx|ctx.cancel()).build()]}}
 #[test] fn native_reference_survives_would_play_park_and_cancellation_cannot_bind_later_retry() {
  for cancel in [false,true] {
   let mut card=make_test_card("PARK-NATIVE","Native");card.play_cost=0;let mut r=DebugRunner::builder().add_card(card).add_card(make_test_card("PARK-GATE","Gate")).hand(0,&["PARK-NATIVE"]).memory(5).start();r.register_effect("PARK-GATE",std::sync::Arc::new(CancelNative));r.place_on_field(1,"PARK-GATE",None);r.game.current_phase=GamePhase::Main;r.game.enable_native_play_observation(true);let physical=r.game.players[0].hand[0].handle();let mut tracking=NativePlayTracking::default();tracking.begin("initial".into(),0,physical);assert_eq!(build_action_mask(&r.game,0)[PLAY_HAND_START as usize],1.0);r.game.decode_action(PLAY_HAND_START,0);assert!(r.game.pending_selection.is_some());tracking.collect(&mut r.game).unwrap();assert!(tracking.committed.is_empty());assert!(tracking.active.is_some());let parked=r.game.clone();
   for _ in 0..2 {r.game=parked.clone();let mut local=NativePlayTracking{active:tracking.active.clone(),committed:tracking.committed.clone()};let view=r.pending_selection_view().unwrap();let action=if cancel{view.valid_action_ids.iter().copied().find(|a|*a!=PASS).unwrap()}else{PASS};assert_eq!(build_action_mask(&r.game,view.selecting_player)[action as usize],1.0);r.execute_action(view.selecting_player,action).unwrap();local.collect(&mut r.game).unwrap();assert!(r.game.pending_selection.is_none());
    if cancel {assert!(local.committed.is_empty());assert_eq!(r.game.players[0].hand[0].handle(),physical);local.begin("retry".into(),0,physical);assert_eq!(build_action_mask(&r.game,0)[PLAY_HAND_START as usize],1.0);r.game.decode_action(PLAY_HAND_START,0);assert!(r.game.pending_selection.is_none(),"accepted once-per-turn gate cannot reopen on retry");local.collect(&mut r.game).unwrap();assert_eq!(local.committed.len(),1);assert_eq!(local.reference(&r.game,PermanentHandle{player:0,index:0}).as_deref(),Some("retry"));}
    else {assert_eq!(local.committed.len(),1);assert_eq!(local.reference(&r.game,PermanentHandle{player:0,index:0}).as_deref(),Some("initial"));}
   }
  }
 }
}
