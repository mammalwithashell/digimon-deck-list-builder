//! BT20-027 official Q4319: protect all matching simultaneous departures without choosing victims.
use digimon_engine::{action::{build_action_mask,space::{PASS,REPLACEMENT_ACCEPT}},card_data::CardData,debug_runner::{DebugRunner,make_test_card},enums::GamePhase,permanent::PermanentHandle,replacement::ReplacementCause,selection::SelectionKind};
fn setup()->DebugRunner {
 let mut cards=CardData::load_from_str(include_str!("../../../data/cards.json")).unwrap();
 let mut b=DebugRunner::builder().dsl_card("BT20-027").unwrap().add_card(cards.remove("BT20-027").unwrap());
 for(id,name)in[("HOST","Examon carrier"),("A","Dracomon A"),("B","Dracomon B"),("NO","Unrelated"),("FILL","Filler")]{let mut c=make_test_card(id,name);c.dp=Some(9000);c.level=Some(7);b=b.add_card(c);}
 let mut r=b.deck(0,&["FILL";20]).deck(1,&["FILL";20]).memory(10).start();r.skip_mulligan();r.game.current_phase=GamePhase::Main;
 r.place_stack(1,&["BT20-027","HOST"]);r.place_on_field(1,"A",None);r.place_on_field(1,"B",None);r
}
fn begin(r:&mut DebugRunner,extra:bool,cause:ReplacementCause){
 let mut victims=vec![PermanentHandle{player:1,index:1},PermanentHandle{player:1,index:2}];
 if extra{victims.push(r.place_on_field(1,"NO",None));victims.push(r.place_on_field(0,"A",None));}
 r.game.set_effect_source_player_for_test(Some(0));r.game.delete_permanents_batch(victims,cause);
}
fn choose(r:&mut DebugRunner,a:u16){let v=r.pending_selection_view().expect("explicit consent");assert_eq!(v.kind,SelectionKind::Replacement,"Q4319 forbids a victim picker");assert_eq!(v.selecting_player,1);assert_eq!(build_action_mask(&r.game,1)[a as usize],1.0);r.game.resolve_selection(1,a).unwrap();r.game.drain_effect_queue();}
fn ids(r:&DebugRunner)->Vec<String>{r.game.players[1].battle_area.iter().map(|p|p.top_card().card_id(&r.game.card_data).to_string()).collect()}
#[test]fn one_host_suspension_protects_both_without_victim_choice_and_clones(){
 let mut r=setup();begin(&mut r,false,ReplacementCause::OpponentEffect);let mut branch=r.game.clone();let mut digest=None;
 for n in 0..2{if n==1{std::mem::swap(&mut r.game,&mut branch);}choose(&mut r,REPLACEMENT_ACCEPT);assert!(r.game.pending_selection.is_none(),"one consent resolves the whole printed set");assert_eq!(ids(&r),["HOST","A","B"]);assert!(r.game.players[1].battle_area[0].is_suspended);assert!(r.game.players[1].trash.is_empty());if let Some(d)=digest{assert_eq!(r.game.verification_digest(),d);}else{digest=Some(r.game.verification_digest());}}
}
#[test]fn nonmatching_and_opponent_members_still_delete(){let mut r=setup();begin(&mut r,true,ReplacementCause::OpponentEffect);choose(&mut r,REPLACEMENT_ACCEPT);assert!(r.game.pending_selection.is_none());assert_eq!(ids(&r),["HOST","A","B"]);assert_eq!(r.game.players[1].trash.len(),1);assert_eq!(r.game.players[0].trash.len(),1);}
#[test]fn decline_preserves_opt_for_second_real_event(){let mut r=setup();begin(&mut r,false,ReplacementCause::OpponentEffect);choose(&mut r,PASS);assert_eq!(ids(&r),["HOST"]);assert!(!r.game.players[1].battle_area[0].is_suspended);r.place_on_field(1,"A",None);r.place_on_field(1,"B",None);begin(&mut r,false,ReplacementCause::OpponentEffect);choose(&mut r,REPLACEMENT_ACCEPT);assert!(r.game.pending_selection.is_none());assert_eq!(ids(&r),["HOST","A","B"]);}
#[test]fn successful_opt_blocks_second_event_even_with_unsuspended_host(){let mut r=setup();begin(&mut r,false,ReplacementCause::OpponentEffect);choose(&mut r,REPLACEMENT_ACCEPT);assert!(r.game.pending_selection.is_none());assert_eq!(ids(&r),["HOST","A","B"]);r.game.players[1].battle_area[0].is_suspended=false;begin(&mut r,false,ReplacementCause::OpponentEffect);assert!(r.game.pending_selection.is_none(),"cost availability restored, consumed OPT still blocks");assert_eq!(ids(&r),["HOST"]);}
#[test]fn already_suspended_host_cannot_pay(){let mut r=setup();r.game.players[1].battle_area[0].is_suspended=true;begin(&mut r,false,ReplacementCause::OpponentEffect);assert!(r.game.pending_selection.is_none());assert_eq!(ids(&r),["HOST"]);}
#[test]fn battle_departures_are_excluded(){let mut r=setup();begin(&mut r,false,ReplacementCause::Battle);assert!(r.game.pending_selection.is_none());assert_eq!(ids(&r),["HOST"]);assert!(!r.game.players[1].battle_area[0].is_suspended);}
