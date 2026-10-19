//! Official EX13-024 Q7272: one activation prevents every matching simultaneous departure.
use digimon_engine::{action::{build_action_mask,space::{PASS,PLAY_HAND_START,REPLACEMENT_ACCEPT,encode_attack}},card_source::CardHandle,debug_runner::{DebugRunner,make_test_card},enums::GamePhase,selection::SelectionKind};
const PRODUCER:&str=r#"
card: Q7272-RETURN
name: Simultaneous return witness
kind: digimon
color: [blue]
level: 3
cost: 0
dp: 2000
effects:
  - when: on_play
    process:
      - return_all_to_hand:
          over: {of: opponent, zone: [battle_area], kind: digimon, level_eq: 4}
"#;
fn act(r:&mut DebugRunner,a:u16){let p=r.game.pending_selection.as_ref().expect("public choice").selecting_player;assert_eq!(build_action_mask(&r.game,p)[a as usize],1.0);r.game.resolve_selection(p,a).unwrap();}
fn field(r:&DebugRunner)->Vec<CardHandle>{r.game.players[1].battle_area.iter().map(|p|p.top_card().handle()).collect()}
fn setup_inner(inherited:bool,nonmatching:bool,mixed:bool)->(DebugRunner,Vec<CardHandle>,Option<CardHandle>){
 let mut b=DebugRunner::builder().dsl_card("EX13-024").unwrap().from_dsl_yaml(PRODUCER).unwrap();
 for (id,name,level) in [("Q7272-A","Dracomon A",4),("Q7272-B","Dracomon B",4),("Q7272-PAY","Dracomon payer",3),("Q7272-NO","Nonmatching body",4),("Q7272-HOST","Examon carrier",7),("Q7272-FILL","Filler",3)]{let mut c=make_test_card(id,name);c.level=Some(level);c.dp=Some(4000);b=b.add_card(c);}
 let mut r=b.hand(0,&["Q7272-RETURN"]).deck(0,&["Q7272-FILL";20]).deck(1,&["Q7272-FILL";20]).memory(10).start();r.skip_mulligan();r.game.current_phase=GamePhase::Main;
 if inherited {r.place_stack(1,&["EX13-024","Q7272-HOST"]);}else{r.place_on_field(1,"EX13-024",None);}
 r.game.players[1].battle_area[0].is_suspended=true;
 r.place_on_field(1,"Q7272-PAY",None);
 let mut protected=Vec::new();for id in ["Q7272-A","Q7272-B"]{let h=r.place_on_field(1,id,None);protected.push(r.game.players[1].battle_area[h.index as usize].top_card().handle());r.game.players[1].battle_area[h.index as usize].is_suspended=true;}
 let other=if nonmatching{let h=r.place_on_field(1,"Q7272-NO",None);Some(r.game.players[1].battle_area[h.index as usize].top_card().handle())}else{None};
 if mixed {
  let opp=r.place_on_field(0,"Q7272-A",None);
  r.game.set_effect_source_player_for_test(Some(0));
  r.game.delete_permanents_batch(vec![digimon_engine::permanent::PermanentHandle{player:1,index:2},digimon_engine::permanent::PermanentHandle{player:1,index:3},opp],digimon_engine::replacement::ReplacementCause::OpponentEffect);
 } else {
  assert_eq!(build_action_mask(&r.game,0)[PLAY_HAND_START as usize],1.0);r.game.decode_action(PLAY_HAND_START,0);
 }
 assert_eq!(r.pending_selection_view().unwrap().kind,SelectionKind::Replacement);
 assert_eq!(r.pending_selection_view().unwrap().selecting_player,1);
 (r,protected,other)
}
fn accepted(inherited:bool,nonmatching:bool){
 let (mut r,protected,other)=setup(inherited,nonmatching);let mut cloned=r.game.clone();let mut digest=None;
 for branch in 0..2 {if branch==1{std::mem::swap(&mut r.game,&mut cloned);}
 act(&mut r,REPLACEMENT_ACCEPT);
 let v=r.pending_selection_view().expect("one suspension payment picker");
 assert_eq!(v.kind,SelectionKind::OwnField,"Q7272 protects the complete set; there is no singular victim picker before payment");
 assert_eq!(v.selecting_player,1);assert_eq!(build_action_mask(&r.game,1)[encode_attack(0,1) as usize],1.0);
 act(&mut r,encode_attack(0,1));
 assert!(r.game.pending_selection.is_none(),"one payment finishes one plural activation");
 assert!(r.game.players[1].battle_area.iter().any(|p|p.top_card().card_id(&r.game.card_data)=="Q7272-PAY"&&p.is_suspended));
 assert!(protected.iter().all(|h|field(&r).contains(h)));
 assert!(protected.iter().all(|h|!r.game.players[1].hand.iter().any(|c|c.handle()==*h)));
 if let Some(other)=other{assert!(!field(&r).contains(&other));assert!(r.game.players[1].hand.iter().any(|c|c.handle()==other));}
 if let Some(d)=digest{assert_eq!(r.game.verification_digest(),d);}else{digest=Some(r.game.verification_digest());}
 }
}
#[test]fn face_one_suspension_preserves_all_matching_and_clones(){accepted(false,false);}
#[test]fn inherited_one_suspension_preserves_all_matching_and_clones(){accepted(true,false);}
#[test]fn nonmatching_member_still_returns_after_plural_protection(){accepted(false,true);}
#[test]fn declining_does_not_suspend_and_both_members_return(){let(mut r,group,_)=setup(false,false);act(&mut r,PASS);assert!(r.game.pending_selection.is_none());assert!(group.iter().all(|h|r.game.players[1].hand.iter().any(|c|c.handle()==*h)));assert!(!r.game.players[1].battle_area[1].is_suspended);}

#[test]
fn stale_already_suspended_payment_does_not_protect_the_group(){
 let(mut r,group,_)=setup(false,false);act(&mut r,REPLACEMENT_ACCEPT);
 assert_eq!(r.pending_selection_view().unwrap().kind,SelectionKind::OwnField);
 // Direct stale-state control: the captured payer cannot perform a second suspension.
 r.game.players[1].battle_area[1].is_suspended=true;
 act(&mut r,encode_attack(0,1));
 assert!(r.game.pending_selection.is_none());
 assert!(group.iter().all(|h|r.game.players[1].hand.iter().any(|c|c.handle()==*h)));
}

fn cost_decline_preserves_use(inherited:bool){
 let(mut r,group,_)=setup(inherited,false);act(&mut r,REPLACEMENT_ACCEPT);
 assert_eq!(r.pending_selection_view().unwrap().kind,SelectionKind::OwnField);
 assert_eq!(build_action_mask(&r.game,1)[PASS as usize],1.0,"optional suspension cost can be declined");
 let mut branch=r.game.clone();let mut expected=None;
 for i in 0..2 {if i==1{std::mem::swap(&mut r.game,&mut branch);}
 act(&mut r,PASS);assert!(r.game.pending_selection.is_none());
 assert!(group.iter().all(|h|r.game.players[1].hand.iter().any(|c|c.handle()==*h)));
 assert!(!r.game.players[1].battle_area[1].is_suspended);
 assert!(r.game.players[1].battle_area[0].effect_activations.values().all(|n|*n==0),"declining cost refunds the exact face/inherited OPT");
 r.place_on_field(1,"Q7272-A",None);r.place_on_field(1,"Q7272-B",None);
 let idx=r.add_to_hand(0,"Q7272-RETURN");let action=PLAY_HAND_START+idx as u16;
 assert_eq!(build_action_mask(&r.game,0)[action as usize],1.0);r.game.decode_action(action,0);
 assert_eq!(r.pending_selection_view().unwrap().kind,SelectionKind::Replacement,"same-turn second event can offer the refunded effect");
 if let Some(d)=expected{assert_eq!(r.game.verification_digest(),d);}else{expected=Some(r.game.verification_digest());}
 }
}
#[test]fn face_cost_picker_decline_refunds_opt_and_clones(){cost_decline_preserves_use(false);}
#[test]fn inherited_cost_picker_decline_refunds_opt_and_clones(){cost_decline_preserves_use(true);}

fn setup(inherited:bool,nonmatching:bool)->(DebugRunner,Vec<CardHandle>,Option<CardHandle>){setup_inner(inherited,nonmatching,false)}
fn success_consumes_use(inherited:bool){
 let(mut r,group,_)=setup(inherited,false);act(&mut r,REPLACEMENT_ACCEPT);
 let mut cloned=r.game.clone();let mut digest=None;
 for i in 0..2 {if i==1{std::mem::swap(&mut r.game,&mut cloned);}
 act(&mut r,encode_attack(0,1));assert!(r.game.pending_selection.is_none());
 assert!(group.iter().all(|h|field(&r).contains(h)));
 assert!(r.game.players[1].battle_area[0].effect_activations.values().any(|n|*n>0));
 // A fresh unsuspended payer removes cost unavailability as the explanation for no second offer.
 r.place_on_field(1,"Q7272-PAY",None);
 let idx=r.add_to_hand(0,"Q7272-RETURN");let action=PLAY_HAND_START+idx as u16;
 assert_eq!(build_action_mask(&r.game,0)[action as usize],1.0);r.game.decode_action(action,0);
 assert!(r.game.pending_selection.is_none(),"successful face/inherited OPT cannot be used on a second same-turn event");
 assert!(group.iter().all(|h|r.game.players[1].hand.iter().any(|c|c.handle()==*h)));
 if let Some(d)=digest{assert_eq!(r.game.verification_digest(),d);}else{digest=Some(r.game.verification_digest());}
 }
}
#[test]fn successful_face_payment_consumes_opt_with_another_payer_available(){success_consumes_use(false);}
#[test]fn successful_inherited_payment_consumes_opt_with_another_payer_available(){success_consumes_use(true);}
#[test]fn mixed_controller_group_never_protects_opponent_text_digimon(){
 let(mut r,group,_)=setup_inner(false,false,true);
 let other=r.game.players[0].battle_area[0].top_card().handle();
 act(&mut r,REPLACEMENT_ACCEPT);let mut cloned=r.game.clone();let mut digest=None;
 for i in 0..2 {if i==1{std::mem::swap(&mut r.game,&mut cloned);}
 act(&mut r,encode_attack(0,1));assert!(r.game.pending_selection.is_none());
 assert!(group.iter().all(|h|field(&r).contains(h)));
 assert!(r.game.players[0].trash.iter().any(|c|c.handle()==other));
 assert!(!r.game.players[0].battle_area.iter().any(|p|p.top_card().handle()==other));
 if let Some(d)=digest{assert_eq!(r.game.verification_digest(),d);}else{digest=Some(r.game.verification_digest());}
 }
}
