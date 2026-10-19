//! Full EX3-012 printed-card behavior; official Q4669-Q4672/Q6508 authority.
use digimon_engine::{debug_runner::{DebugRunner,DebugRunnerBuilder,make_test_card},enums::{CardColor,CardKind,EffectTiming,Expiry,GamePhase,ModifierType},modifiers::ModifierEntry,permanent::PermanentHandle,selection::TriggerSource};
const ID:&str="EX3-012";
fn base()->DebugRunnerBuilder{
 let path=std::env::var_os("EX3_012_BEHAVIORAL_YAML").map(std::path::PathBuf::from).unwrap_or_else(||std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/ex3/EX3-012.yaml"));
 let text=std::fs::read_to_string(path).expect("EX3-012 full production YAML must exist");
 let mut b=DebugRunner::builder().from_dsl_yaml(&text).expect("complete EX3-012 DSL");
 for (id,dp,level,color) in [("LOW",3000,3,CardColor::Red),("LIMIT",5000,4,CardColor::Red),("HIGH",6000,5,CardColor::Red),("BLACK",6000,5,CardColor::Black),("BLUE",6000,5,CardColor::Blue),("RED4",6000,4,CardColor::Red)] {
  let mut c=make_test_card(id,id);c.dp=Some(dp);c.level=Some(level);c.colors=vec![color];c.play_cost=1;b=b.add_card(c);
 }
 let mut t=make_test_card("TAMER","Tamer");t.card_kind=CardKind::Tamer;t.dp=None;t.level=None;t.play_cost=1;b=b.add_card(t);
 b.hand(0,&["LIMIT","HIGH"]).hand(1,&["LIMIT","HIGH"]).deck(0,&["HIGH";20]).deck(1,&["HIGH";20]).security(0,&["LOW";3]).security(1,&["HIGH","LIMIT","LOW"]).memory(10)
}
fn setup()->(DebugRunner,PermanentHandle){let mut r=base().start();r.skip_mulligan();r.game.current_phase=GamePhase::Main;let h=r.place_on_field(0,ID,None);(r,h)}
fn fire(r:&mut DebugRunner,h:PermanentHandle,timing:EffectTiming){r.game.enqueue_triggered(timing,TriggerSource::Permanent(h));r.game.drain_effect_queue();}
fn may_play(r:&DebugRunner,owner:u8,id:&str,controller:u8)->bool{
 let c=r.game.player(owner).hand.iter().find(|c|c.card_id(&r.game.card_data)==id).unwrap().handle();
 r.game.can_play_card_from_effect_without_cost_for_controller(owner,c,1,controller)
}
#[test]
fn metadata_red_only_and_both_printed_level5_routes(){
 for (source,legal) in [("HIGH",true),("BLACK",true),("BLUE",false),("RED4",false)]{
  let mut r=base().hand(0,&[ID]).start();r.skip_mulligan();r.game.current_phase=GamePhase::Main;let h=r.place_on_field(0,source,None);
  let c=&r.game.card_data[r.game.player(0).hand[0].data_index];assert_eq!(c.colors,vec![CardColor::Red]);assert_eq!(c.dp,Some(12000));assert_eq!(c.level,Some(6));assert_eq!(c.play_cost,12);
  let action=digimon_engine::action::space::encode_digivolve(0,h.index as u16);assert_eq!(digimon_engine::action::build_action_mask(&r.game,0)[action as usize]>0.0,legal,"{source}");
  if legal{r.game.decode_action(action,0);assert_eq!(r.memory(),6);assert_eq!(r.effective_dp(h),Some(12000));}
 }
}
#[test]
fn on_play_deletes_every_tied_lowest_and_uses_effective_dp(){
 for boosted in [false,true]{
  let (mut r,h)=setup();let a=r.place_on_field(1,"LOW",None);let b=r.place_on_field(1,"LOW",None);r.place_on_field(1,"LIMIT",None);r.place_on_field(1,"TAMER",None);
  if boosted{for low in [a,b]{r.game.modifiers.add(low,ModifierEntry::simple(ModifierType::ChangeDp,4000,Expiry::Permanent,1));}}
  fire(&mut r,h,EffectTiming::OnPlay);assert!(r.game.pending_selection.is_none());
  assert_eq!(r.trash_size(1),if boosted{1}else{2});assert_eq!(r.battle_area_size(1),if boosted{3}else{2});
  assert!(may_play(&r,1,"LIMIT",1),"any actual deletion suppresses fallback");
 }
}
#[test]
fn no_targets_installs_captured_player_restriction_and_correct_expiry(){
 let (mut r,h)=setup();fire(&mut r,h,EffectTiming::OnPlay);
 assert!(!may_play(&r,1,"LIMIT",1));assert!(may_play(&r,1,"HIGH",1));assert!(may_play(&r,1,"LIMIT",0));assert!(!may_play(&r,0,"LIMIT",1));
 let clone=r.game.clone();for cloned in [false,true]{r.game=clone.clone();if cloned{r.game=r.game.clone();}
  r.game.modifiers.expire_player_end_of_turn(0);assert!(!may_play(&r,1,"LIMIT",1));
  r.game.modifiers.expire_player_end_of_turn(1);assert!(may_play(&r,1,"LIMIT",1));
 }
}
#[test]
fn protected_lowest_is_not_replaced_by_higher_target_and_fallback_applies(){
 let (mut r,h)=setup();let low=r.place_on_field(1,"LOW",None);r.place_on_field(1,"HIGH",None);
 r.game.modifiers.add(low,ModifierEntry::simple(ModifierType::CannotBeDestroyedByEffect,0,Expiry::Permanent,1));
 fire(&mut r,h,EffectTiming::OnPlay);assert_eq!(r.battle_area_size(1),2);assert_eq!(r.trash_size(1),0);assert!(!may_play(&r,1,"LIMIT",1));
}
#[test]
fn one_successful_tied_deletion_suppresses_fallback_even_when_other_is_protected(){
 let (mut r,h)=setup();let low=r.place_on_field(1,"LOW",None);r.place_on_field(1,"LOW",None);r.place_on_field(1,"HIGH",None);
 r.game.modifiers.add(low,ModifierEntry::simple(ModifierType::CannotBeDestroyedByEffect,0,Expiry::Permanent,1));
 fire(&mut r,h,EffectTiming::OnPlay);assert_eq!(r.battle_area_size(1),2);assert_eq!(r.trash_size(1),1);assert!(may_play(&r,1,"LIMIT",1));
}
struct OptionalProtection;
impl digimon_engine::effect::CardEffect for OptionalProtection{
 fn effects(&self,c:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect>{vec![digimon_engine::effect::Effect::when_would_be_deleted(c).optional().name("Protect the target").replacement_process(|r|r.cancel()).build()]}
}
#[test]
fn replacement_choice_finishes_before_no_deletion_restriction_and_clones(){
 let (mut r,h)=setup();r.register_effect("LOW",std::sync::Arc::new(OptionalProtection));r.place_on_field(1,"LOW",None);fire(&mut r,h,EffectTiming::OnPlay);
 assert!(r.game.pending_selection.is_some());assert!(may_play(&r,1,"LIMIT",1));let snapshot=r.game.clone();
 for protect in [false,true]{for cloned in [false,true]{r.game=snapshot.clone();if cloned{r.game=r.game.clone();}
  let a=if protect{digimon_engine::action::space::REPLACEMENT_ACCEPT}else{digimon_engine::action::space::PASS};let actor=r.game.pending_selection.as_ref().unwrap().selecting_player;
  assert!(digimon_engine::action::build_action_mask(&r.game,actor)[a as usize]>0.0);r.game.resolve_selection(actor,a).unwrap();
  assert_eq!(r.battle_area_size(1),usize::from(protect));assert_eq!(may_play(&r,1,"LIMIT",1),!protect);
 }}
}
#[test]
fn attacking_requires_own_tamer_and_trashes_exactly_top_security_without_check(){
 for owner in [None,Some(0),Some(1)]{
  let (mut r,h)=setup();if let Some(p)=owner{r.place_on_field(p,"TAMER",None);}
  let before:Vec<_>=r.game.player(1).security.iter().map(|c|c.handle()).collect();let top=*before.last().unwrap();
  fire(&mut r,h,EffectTiming::WhenAttacking);
  assert_eq!(r.game.player(1).security.len(),before.len()-usize::from(owner==Some(0)));
  if owner==Some(0){assert!(r.game.player(1).trash.iter().any(|c|c.handle()==top));assert_eq!(r.game.player(1).security.iter().map(|c|c.handle()).collect::<Vec<_>>(),before[..before.len()-1]);}
  assert_eq!(r.game.player(0).security.len(),3);assert!(r.game.pending_selection.is_none());
 }
}
#[test]
fn attacking_has_no_once_per_turn_limit_and_empty_security_is_safe(){
 let (mut r,h)=setup();r.place_on_field(0,"TAMER",None);
 for expected in [2,1,0,0]{fire(&mut r,h,EffectTiming::WhenAttacking);assert_eq!(r.game.player(1).security.len(),expected);assert!(r.game.pending_selection.is_none());}
}
