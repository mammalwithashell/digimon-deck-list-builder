//! Official EX13-044 Q7350 / EX13-045 Q7358: effect-selected rule battles
//! can include Mother D-Reaper despite its printed Digi-Egg kind/immunity.
use digimon_engine::action::{build_action_mask,space::{ATTACK_START,PASS}};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{EffectTiming,GamePhase};
use digimon_engine::selection::{SelectionKind,TriggerSource};

fn base()->DebugRunner {
 let mut b=DebugRunner::builder();
 for id in ["EX2-007","EX13-044","EX13-045","ST1-04","ST1-12"] {b=b.dsl_card(id).expect("full actual card");}
 let mut r=b.deck(0,&["ST1-04";12]).deck(1,&["ST1-04";12]).security(1,&["ST1-04";5]).memory(5).start();
 r.skip_mulligan();r.game.current_phase=GamePhase::Main;r
}
fn choose(r:&mut DebugRunner,a:u16){let v=r.pending_selection_view().expect("player decision");assert_eq!(build_action_mask(&r.game,v.selecting_player)[a as usize],1.0);r.game.resolve_selection(v.selecting_player,a).unwrap();}
fn wd(r:&mut DebugRunner){let h=r.place_on_field(0,"EX13-045",Some(0));r.game.enqueue_triggered(EffectTiming::WhenDigivolving,TriggerSource::Permanent(h));r.game.drain_effect_queue();}

#[test]
fn q7358_mother_is_offered_for_effect_battle_and_tie_deletes_both_in_clones(){
 let mut r=base();r.place_on_field(1,"EX2-007",Some(0));wd(&mut r);
 let v=r.pending_selection_view().expect("Mother must be a Digimon field target");assert_eq!(v.kind,SelectionKind::OppField);
 assert_eq!(build_action_mask(&r.game,0)[ATTACK_START as usize],1.0);
 assert_eq!(build_action_mask(&r.game,0)[PASS as usize],1.0);
 let saved=r.game.clone();
 for _ in 0..2 {r.game=saved.clone();choose(&mut r,ATTACK_START);assert!(r.pending_selection().is_some(),"Examon may Evade the battle deletion");choose(&mut r,PASS);assert!(r.pending_selection().is_none());assert_eq!(r.battle_area_size(0),0);assert_eq!(r.battle_area_size(1),0);assert_eq!(r.security_count(1),5);}
}

#[test]
fn q7358_declining_battle_preserves_both_real_cards(){
 let mut r=base();r.place_on_field(1,"EX2-007",Some(0));wd(&mut r);
 assert!(r.pending_selection().is_some(),"Mother is a legal optional battle target");choose(&mut r,PASS);
 assert_eq!(r.battle_area_size(0),1);assert_eq!(r.battle_area_size(1),1);assert_eq!(r.security_count(1),5);
}

#[test]
fn q7350_mother_satisfies_opponent_digimon_activation_and_selection(){
 let mut r=base();r.place_on_field(0,"EX13-044",Some(0));let draco=r.place_on_field(0,"ST1-04",Some(0));r.place_on_field(1,"EX2-007",Some(0));
 // Dracomon qualifies by actual printed name, without needing a synthetic
 // metadata effect-text injection into the real Breakdramon card.
 r.game.suspend(draco);
 let v=r.pending_selection_view().expect("opponent Digimon existence includes Mother");
 if v.kind!=SelectionKind::OwnField {let a=*v.valid_action_ids.iter().find(|&&a|a!=PASS).unwrap();choose(&mut r,a);}
 assert_eq!(r.pending_selection_view().unwrap().kind,SelectionKind::OwnField);
 choose(&mut r,ATTACK_START+draco.index as u16);
 assert_eq!(r.pending_selection_view().unwrap().kind,SelectionKind::OppField);
 choose(&mut r,ATTACK_START);
 assert!(r.pending_selection().is_none());assert_eq!(r.battle_area_size(0),1);assert_eq!(r.battle_area_size(1),1);
 assert!(r.game.players[0].trash.iter().any(|c|c.card_id(&r.game.card_data)=="ST1-04"),"battle rules delete losing Dracomon despite Mother's immunity");
}

#[test]
fn non_digimon_tamer_does_not_satisfy_battle_activation(){
 let mut r=base();r.place_on_field(0,"EX13-044",Some(0));let draco=r.place_on_field(0,"ST1-04",Some(0));r.place_on_field(1,"ST1-12",None);
 r.game.suspend(draco);assert!(r.pending_selection().is_none());assert_eq!(r.battle_area_size(1),1);
}
