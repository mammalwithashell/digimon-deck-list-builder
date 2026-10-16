 use super::*;
 use digimon_engine::{debug_runner::{DebugRunner,make_test_card},permanent::PermanentHandle,selection::{PendingAttack,AttackTarget,AttackState},enums::GamePhase};
 fn fixture()->Game {
  let mut r=DebugRunner::builder().add_card(make_test_card("ATTACKER","Attacker")).add_card(make_test_card("TWIN","Twin")).add_card(make_test_card("LOWER","Lower"))
   .deck(0,&["ATTACKER";12]).deck(1,&["TWIN";12]).start();r.skip_mulligan();r.place_on_field(0,"ATTACKER",Some(0));r.place_on_field(1,"TWIN",Some(0));r.place_on_field(1,"TWIN",Some(0));
  for p in &mut r.game.players[1].battle_area{p.is_suspended=true;}
  let h=PermanentHandle{player:0,index:0};let target=AttackTarget::Digimon(PermanentHandle{player:1,index:0});
  r.game.pending_attack=Some(PendingAttack{attacker:h,original_target:target,effective_target:target,effective_target_removed_by_dna:false,is_blocked:false,blocker:None,is_vortex:false,is_overclock:false,declaration_committed:true,cancelled:false,battle_occurred:false,battle_defender_deleted:false,effect_battle_piercing_pending:false,security_check_started:false,return_phase:GamePhase::Main,state:AttackState::CounterOpen,counter_depth:0});r.game
 }

 #[test] fn descriptor_actor_one_uses_absolute_seat_not_viewpoint(){
  let g=fixture();let(_,own)=build_selection_row(&g,0,1,&SelectPayload::Targets(vec!["own.field.1".into()]),None).unwrap();let(_,opp)=build_selection_row(&g,0,1,&SelectPayload::Targets(vec!["opp.field.0".into()]),None).unwrap();assert_eq!(own.permanent_targets[0].owner,1);assert_eq!(opp.permanent_targets[0].owner,0);assert!(opp.permanent_targets[0].attacker);
 }
 #[test] fn precommit_descriptor_has_no_attack_roles(){
  let mut g=fixture();g.pending_attack.as_mut().unwrap().declaration_committed=false;
  for (actor,target) in [(0,"own.field.0"),(1,"own.field.0")] {let(_,wire)=build_selection_row(&g,0,actor,&SelectPayload::Targets(vec![target.into()]),None).unwrap();assert!(!wire.permanent_targets[0].attacker);assert!(!wire.permanent_targets[0].attack_target);}
 }
 #[test] fn blocked_descriptor_follows_effective_target_only(){
  let mut g=fixture();let blocker=PermanentHandle{player:1,index:1};let a=g.pending_attack.as_mut().unwrap();a.effective_target=AttackTarget::Digimon(blocker);a.is_blocked=true;a.blocker=Some(blocker);
  let(_,wire)=build_selection_row(&g,0,0,&SelectPayload::Targets(vec!["opp.field.0".into(),"opp.field.1".into()]),None).unwrap();assert!(!wire.permanent_targets[0].attack_target);assert!(wire.permanent_targets[1].attack_target);
  g.pending_attack.as_mut().unwrap().battle_defender_deleted=true;let(_,wire)=build_selection_row(&g,0,0,&SelectPayload::Targets(vec!["opp.field.1".into()]),None).unwrap();assert!(!wire.permanent_targets[0].attack_target);
 }
 #[test] fn two_link_and_source_orders_are_preserved(){
  let mut g=fixture();let a=g.players[0].battle_area[0].top_card().clone();let b=g.players[1].battle_area[0].top_card().clone();let p=&mut g.players[1].battle_area[1];p.card_sources.insert(0,b.clone());p.card_sources.insert(0,a.clone());p.linked_cards=vec![b,a];let(_,wire)=build_selection_row(&g,0,0,&SelectPayload::Targets(vec!["opp.field.1".into()]),None).unwrap();assert_eq!(wire.permanent_targets[0].sources,vec!["ATTACKER","TWIN"]);assert_eq!(wire.permanent_targets[0].linked,vec!["TWIN","ATTACKER"]);
 }
