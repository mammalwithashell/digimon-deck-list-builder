//! Official general_rule.pdf p.23 §12-1-5; DCGO AttackProcess excludes DefendingPermanent.
use digimon_engine::{debug_runner::DebugRunner, action::{build_action_mask,space::{encode_attack,SECURITY_TARGET,PASS}},enums::{GamePhase,Keyword},selection::{EffectChoiceEntry,SelectionKind}};
fn setup(two:bool)->DebugRunner {
 let mut b=DebugRunner::builder();for id in ["EX13-045","ST2-07","ST1-03"] {b=b.dsl_card(id).unwrap();}
 let mut r=b.deck(0,&["ST1-03";20]).deck(1,&["ST1-03";20]).security(1,&["ST1-03";5]).memory(10).start();
 r.skip_mulligan();r.set_phase(GamePhase::Main);r.place_on_field(0,"EX13-045",Some(0));r.place_on_field(1,"ST2-07",Some(0));if two{r.place_on_field(1,"ST2-07",Some(0));}
 let a=encode_attack(0,SECURITY_TARGET);assert_eq!(build_action_mask(&r.game,0)[a as usize],1.0);r.game.decode_action(a,0);
 let p=r.game.pending_selection.as_ref().unwrap();assert!(p.prompt.contains("Raid"));assert_eq!(p.kind,SelectionKind::OppField);r
}
fn choose(r:&mut DebugRunner,p:u8,a:u16){assert_eq!(build_action_mask(&r.game,p)[a as usize],1.0);r.execute_action(p,a).unwrap();}
// EX13-045 (15000 DP) wins the attack battle and deletes the defender, so two of
// its battle-result effects trigger simultaneously (§14-2-4): the printed "[Your
// Turn] [Once Per Turn] When this Digimon wins a battle" clause and <Piercing>,
// which "is a trigger-type effect" (§16-7-2) that "will trigger simultaneously
// with effects triggered by the battle" (§16-7-4; official Q&A Q3931). Both are
// the turn player's, so P0 orders them (§15-4-3-5-1); an [Once Per Turn] effect
// activates even when its processing can't be executed (§15-14-1-4; no
// [Dracomon]/[Examon] text card is available here). Activating <Piercing> only
// makes its one security check pending before the End of Attack (§16-7-4,
// §16-7-5), so either order ends the attack with the same board.
fn battle_pool_entries(r:&DebugRunner)->(u16,u16){
 let p=r.game.pending_selection.as_ref().expect("battle-result TriggerOrder");assert_eq!(p.kind,SelectionKind::TriggerOrder);assert_eq!(p.selecting_player,0);
 let c=p.effect_choices.as_ref().unwrap();assert_eq!(c.len(),2,"the printed win trigger and <Piercing>, nothing else");
 let src=|e:&EffectChoiceEntry|e.source_card.and_then(|h|r.game.card_data_for_handle(h)).map(|d|d.card_id.clone());
 let printed=c.iter().find(|e|e.keyword.is_none()&&src(e).as_deref()==Some("EX13-045")).expect("printed wins-a-battle trigger").action_id;
 let pierce=c.iter().find(|e|e.keyword==Some(Keyword::Piercing)).expect("<Piercing> trigger").action_id;
 assert_eq!(r.security_count(1),5,"<Piercing>'s check is not performed before it activates");(printed,pierce)
}
fn finish_battle_pool(r:&mut DebugRunner,piercing_first:bool){let (printed,pierce)=battle_pool_entries(r);choose(r,0,if piercing_first{pierce}else{printed});}
#[test]
fn sole_current_defender_cannot_install_blocker_choice_after_raid_in_clones(){
 let mut r=setup(false);let g=r.game.clone();for piercing_first in [false,true] {r.game=g.clone();choose(&mut r,0,encode_attack(0,0));
 // No `OwnField` block prompt for P1: the next decision is P0's battle-result order.
 let p=r.game.pending_selection.as_ref().expect("battle-result TriggerOrder");assert_ne!(p.kind,SelectionKind::OwnField,"the sole blocker is already the attack target and cannot block");
 finish_battle_pool(&mut r,piercing_first);
 assert!(r.game.pending_selection.is_none());assert!(r.game.pending_attack.is_none());assert!(r.game.players[1].battle_area.is_empty());assert_eq!(r.security_count(1),3);}
}
#[test]
fn raid_current_target_excluded_other_blocker_preserved_and_clone_accepts(){
 let mut r=setup(true);choose(&mut r,0,encode_attack(0,0));let p=r.game.pending_selection.as_ref().unwrap();assert_eq!(p.kind,SelectionKind::OwnField);assert_eq!(p.selecting_player,1);
 assert_eq!(p.valid_action_ids,vec![encode_attack(0,1)]);let mask=build_action_mask(&r.game,1);assert_eq!(mask[encode_attack(0,0) as usize],0.0);assert_eq!(mask[PASS as usize],1.0);
 let g=r.game.clone();for piercing_first in [false,true] {let mut digest=None;for _ in 0..2 {r.game=g.clone();choose(&mut r,1,encode_attack(0,1));finish_battle_pool(&mut r,piercing_first);assert!(r.game.pending_selection.is_none());assert!(r.game.pending_attack.is_none());assert_eq!(r.game.players[1].battle_area.len(),1);assert!(!r.game.players[1].battle_area[0].is_suspended);assert_eq!(r.security_count(1),3);let d=r.game.verification_digest();if let Some(old)=digest {assert_eq!(old,d);}digest=Some(d);}}
}
#[test]
fn declining_raid_retains_all_blockers_against_player_target(){
 let mut r=setup(true);choose(&mut r,0,PASS);let p=r.game.pending_selection.as_ref().unwrap();assert_eq!(p.kind,SelectionKind::OwnField);assert_eq!(p.valid_action_ids,vec![encode_attack(0,0),encode_attack(0,1)]);choose(&mut r,1,PASS);assert_eq!(r.game.players[1].battle_area.len(),2);assert_eq!(r.security_count(1),3);assert!(r.game.pending_attack.is_none());
}

// Additional rule12-1-5 controls: native (non-Raid) target and Collision.
fn direct_setup(collision:bool,two:bool)->DebugRunner {
 use digimon_engine::{modifiers::ModifierEntry,enums::{ModifierType,Expiry}};
 let mut b=DebugRunner::builder();for id in ["ST1-11","ST2-07","ST1-03","EX7-044"] {b=b.dsl_card(id).unwrap();}
 let mut r=b.deck(0,&["ST1-03";20]).deck(1,&["ST1-03";20]).security(1,&["ST1-03";5]).memory(10).start();r.skip_mulligan();r.set_phase(GamePhase::Main);
 let attacker=if collision {r.place_stack(0,&["EX7-044","ST1-11"])}else{r.place_on_field(0,"ST1-11",Some(0))};
 // Attack-unsuspended permission is explicit and mask-checked; no bypass of declarations.
 r.game.modifiers.add(attacker,ModifierEntry::simple(ModifierType::CanAttackUnsuspended,0,Expiry::Permanent,0));
 r.place_on_field(1,if collision{"ST1-03"}else{"ST2-07"},Some(0));
 if two {r.place_on_field(1,if collision{"ST1-03"}else{"ST2-07"},Some(0));}
 let a=encode_attack(0,0);assert_eq!(build_action_mask(&r.game,0)[a as usize],1.0);r.game.decode_action(a,0);r
}
#[test]
fn native_non_raid_target_cannot_block_but_other_blocker_can(){
 let mut r=direct_setup(false,true);let p=r.game.pending_selection.as_ref().unwrap();assert_eq!(p.kind,SelectionKind::OwnField);assert_eq!(p.valid_action_ids,vec![encode_attack(0,1)]);assert_eq!(build_action_mask(&r.game,1)[encode_attack(0,0) as usize],0.0);
 choose(&mut r,1,PASS);assert!(r.game.pending_attack.is_none());assert_eq!(r.game.players[1].battle_area.len(),1);assert!(!r.game.players[1].battle_area[0].is_suspended);assert_eq!(r.security_count(1),5);
}
#[test]
fn collision_does_not_force_the_sole_current_target_to_block(){
 let r=direct_setup(true,false);assert!(r.game.pending_selection.is_none(),"Collision cannot require an illegal same-target block");assert!(r.game.pending_attack.is_none());assert!(r.game.players[1].battle_area.is_empty());assert_eq!(r.security_count(1),5);
}
#[test]
fn collision_other_digimon_is_mandatory_blocker_and_clone_resolves(){
 let mut r=direct_setup(true,true);let p=r.game.pending_selection.as_ref().unwrap();assert_eq!(p.kind,SelectionKind::OwnField);assert_eq!(p.valid_action_ids,vec![encode_attack(0,1)]);assert!(!p.is_optional);
 let m=build_action_mask(&r.game,1);assert_eq!(m[PASS as usize],0.0);assert_eq!(m[encode_attack(0,0) as usize],0.0);let g=r.game.clone();let mut digest=None;
 for _ in 0..2 {r.game=g.clone();choose(&mut r,1,encode_attack(0,1));assert!(r.game.pending_attack.is_none());assert!(r.game.pending_selection.is_none());assert_eq!(r.game.players[1].battle_area.len(),1);assert!(!r.game.players[1].battle_area[0].is_suspended);assert_eq!(r.security_count(1),5);let d=r.game.verification_digest();if let Some(old)=digest{assert_eq!(old,d);}digest=Some(d);}
}
