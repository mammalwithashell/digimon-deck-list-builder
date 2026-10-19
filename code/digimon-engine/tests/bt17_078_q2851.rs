//! Official Q2851: only the bottom-deck sweep requires DNA; Then delete still resolves.
//! Unexecuted TDD draft. Test baseline YAML first, then the reviewed candidate.
use digimon_engine::{action::{build_action_mask,space::{encode_attack,encode_digivolve,PASS}},debug_runner::{DebugRunner,make_test_card},enums::{CardKind,GamePhase},effect_context::EffectContext,selection::SelectionKind};
const YAML:&str=include_str!("../cards/bt17/BT17-078.yaml");
fn setup()->DebugRunner{
 let mut base=make_test_card("BASE","Greymon base");base.level=Some(6);base.dp=Some(11000);
 let mut other=make_test_card("OTHER","Garurumon base");other.level=Some(6);other.dp=Some(11000);
 let mut big=make_test_card("BIG","Big target");big.level=Some(6);big.dp=Some(12000);
 let mut tamer=make_test_card("TAMER","Tamer exclusion");tamer.card_kind=CardKind::Tamer;tamer.level=None;tamer.dp=None;
 let mut r=DebugRunner::builder().from_dsl_yaml(YAML).unwrap().add_card(base).add_card(other).add_card(big).add_card(tamer)
 .add_card(make_test_card("SMALL","Small target")).add_card(make_test_card("FILL","Filler"))
 .hand(0,&["BT17-078"]).deck(0,&["FILL";20]).deck(1,&["FILL";20]).memory(10).start();
 r.skip_mulligan();r.game.current_phase=GamePhase::Main;r
}
fn native(r:&mut DebugRunner,a:u16){assert_eq!(build_action_mask(&r.game,0)[a as usize],1.0);r.game.decode_action(a,0);}
fn choose(r:&mut DebugRunner,index:u16){let v=r.pending_selection_view().expect("printed mandatory target choice");assert_eq!(v.kind,SelectionKind::OppField);assert_eq!(v.selecting_player,0);assert!(!v.is_optional);assert!(!v.valid_action_ids.contains(&PASS));let a=encode_attack(0,index);assert_eq!(build_action_mask(&r.game,0)[a as usize],1.0);r.execute_action(0,a).unwrap();}
fn ids(r:&DebugRunner,p:usize)->Vec<String>{r.game.players[p].battle_area.iter().map(|x|x.top_card().card_id(&r.game.card_data).to_string()).collect()}
#[test]fn normal_play_offers_both_deletion_targets_without_dna_sweep_and_clones(){
 let mut r=setup();r.place_on_field(1,"SMALL",None);r.place_on_field(1,"BIG",None);let deck=r.deck_size(1);native(&mut r,0);
 assert_eq!(r.pending_selection_view().expect("Q2851 non-DNA deletion").valid_action_ids.len(),2);
 let saved=r.game.clone();choose(&mut r,0);assert_eq!(ids(&r,1),["BIG"]);assert_eq!(r.deck_size(1),deck);assert_eq!(r.game.players[1].trash.len(),1);
 r.game=saved;choose(&mut r,1);assert_eq!(ids(&r,1),["SMALL"]);assert_eq!(r.deck_size(1),deck);assert_eq!(r.game.players[1].trash.len(),1);
}
#[test]fn ordinary_evolution_offers_deletion_without_dna_sweep(){
 let mut r=setup();r.place_on_field(0,"BASE",None);r.place_on_field(1,"SMALL",None);r.place_on_field(1,"BIG",None);let deck=r.deck_size(1);
 native(&mut r,encode_digivolve(0,0));choose(&mut r,1);assert_eq!(ids(&r,1),["SMALL"]);assert_eq!(r.deck_size(1),deck);assert_eq!(r.game.players[1].trash.len(),1);
}
#[test]fn no_opponent_digimon_does_not_offer_illegal_choice(){let mut r=setup();r.place_on_field(1,"TAMER",None);native(&mut r,0);assert!(r.pending_selection().is_none());assert_eq!(ids(&r,1),["TAMER"]);}
#[test]fn mandatory_deletion_excludes_opponent_tamer_and_own_digimon(){let mut r=setup();r.place_on_field(0,"BASE",None);r.place_on_field(1,"TAMER",None);r.place_on_field(1,"SMALL",None);native(&mut r,0);let v=r.pending_selection_view().unwrap();assert_eq!(v.valid_action_ids,vec![encode_attack(0,1)]);choose(&mut r,1);assert_eq!(ids(&r,1),["TAMER"]);assert_eq!(ids(&r,0),["BASE","BT17-078"]);}
#[test]fn dna_preserves_sweep_then_separate_mandatory_delete(){
 let mut r=setup();let a=r.place_on_field(0,"BASE",None);let b=r.place_on_field(0,"OTHER",None);r.place_on_field(1,"SMALL",None);r.place_on_field(1,"SMALL",None);r.place_on_field(1,"BIG",None);let deck=r.deck_size(1);let hand=r.game.players[0].hand[0].handle();
 let result={let mut ctx=EffectContext::new(&mut r.game,hand,None,0);ctx.effect_initiated_dna_digivolve(a,b,hand,0,true)};assert!(result.is_some());
 choose(&mut r,0);assert_eq!(ids(&r,1),["BIG"]);assert_eq!(r.deck_size(1),deck+2);assert!(r.game.players[1].trash.is_empty());
 choose(&mut r,0);assert!(ids(&r,1).is_empty());assert_eq!(r.game.players[1].trash.len(),1);assert_eq!(r.deck_size(1),deck+2);
}
