//! Synthetic Counter transform isolates DNA target incarnation from ordinary evolution.
//! These tests intentionally use no proposed-field name so they compile on baseline.
use std::sync::Arc;
use digimon_engine::{debug_runner::{DebugRunner,make_test_card,make_test_dna_card}, effect::{CardEffect,Effect}, card_source::CardHandle, enums::{EffectTiming,CostDelta}, action::{mask::build_action_mask,space::encode_attack}, selection::AttackTarget, permanent::PermanentHandle};
struct Transform(u8);
impl CardEffect for Transform {
 fn effects(&self,card:CardHandle)->Vec<Effect>{
  let mode=self.0;
  vec![Effect::on_play(card).name("Test Counter transform").counter().timing(EffectTiming::CounterEffect)
   .process(move |ctx| {
    let target=ctx.source_permanent.expect("field Counter source");
    if mode==0 {
     assert!(ctx.effect_initiated_digivolve_ignore_requirements(1,0,target,CostDelta::Free));
    }else{
     let result=ctx.game.players[1].hand[0].handle();
     let partner=ctx.game.players[1].hand[1].handle();
     let merged=ctx.effect_initiated_dna_digivolve_with_hand_partner(target,partner,result,0,false)
      .expect("valid field+hand DNA commits");
     if mode==2 {ctx.redirect_attack(AttackTarget::Digimon(merged)).unwrap();}
     if mode==3 {
      let index=ctx.game.players[1].battle_area.iter().position(|p|p.top_card().card_id(&ctx.game.card_data)=="BYST").unwrap();
      ctx.redirect_attack(AttackTarget::Digimon(PermanentHandle{player:1,index:index as u8})).unwrap();
     }
    }
   }).build()]
 }
}
fn exercise(mode:u8){
 let mut mat=make_test_card("MAT","Material");mat.level=Some(6);mat.dp=Some(2000);
 let mut partner=make_test_card("PARTNER","Partner");partner.level=Some(6);
 let mut result=make_test_dna_card("RESULT","Result",6,6,0);result.level=Some(7);result.dp=Some(5000);
 let mut attacker=make_test_card("ATT","Attacker");attacker.dp=Some(12000);
 let mut r=DebugRunner::builder().add_card(mat).add_card(partner).add_card(result).add_card(attacker)
  .add_card(make_test_card("BYST","Bystander")).add_card(make_test_card("DRAW","Draw"))
  .hand(1,&["RESULT","PARTNER"]).deck(0,&["DRAW";4]).deck(1,&["DRAW";4]).memory(5).start();
 r.register_effect("MAT",Arc::new(Transform(mode)));
 assert!(r.game.mulligan_current_player().is_none());
 r.game.current_phase=digimon_engine::enums::GamePhase::Main;
 let att=r.place_on_field(0,"ATT",Some(0));let target=r.place_on_field(1,"MAT",Some(0));
 r.place_on_field(1,"BYST",Some(0));r.game.players[1].battle_area[0].is_suspended=true;
 let declaration=encode_attack(att.index as u16,target.index as u16);
 assert!(build_action_mask(&r.game,0)[declaration as usize]>0.0);
 r.game.decode_action(declaration,0);
 let action=encode_attack(0,target.index as u16);
 let selection=r.game.pending_selection.as_ref().expect("Counter choice");
 assert_eq!(selection.selecting_player,1);assert!(selection.valid_action_ids.contains(&action));
 assert!(build_action_mask(&r.game,1)[action as usize]>0.0);
 let mut clone=r.game.clone();
 for g in [&mut r.game,&mut clone] {
  assert!(build_action_mask(g,1)[action as usize]>0.0);
  g.resolve_selection(1,action).unwrap();
  for _ in 0..8 {if g.pending_attack.is_none(){break;}assert!(g.pending_selection.is_none(),"unexpected interrupt");g.advance_pending_attack();}
  assert!(g.pending_attack.is_none(),"attack must finish");
  let result_alive=g.players[1].battle_area.iter().any(|p|p.top_card().card_id(&g.card_data)=="RESULT");
  let bystander_alive=g.players[1].battle_area.iter().any(|p|p.top_card().card_id(&g.card_data)=="BYST");
  assert_eq!(result_alive,mode==1||mode==3,"DNA loses old target; ordinary evolution and explicit new target do not");
  assert_eq!(bystander_alive,mode!=3,"redirect must hit selected replacement target");
  assert_eq!(g.players[0].battle_area.len(),1);
 }
 assert_eq!(r.game.verification_digest(),clone.verification_digest());
}
#[test] fn dna_target_becomes_new_object_in_same_slot(){exercise(1)}
#[test] fn ordinary_evolution_preserves_attack_target(){exercise(0)}
#[test] fn explicit_redirect_can_select_new_dna_result_in_same_slot(){exercise(2)}
#[test] fn explicit_redirect_selects_bystander_after_target_dna(){exercise(3)}
