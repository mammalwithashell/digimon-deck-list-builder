use digimon_dsl::compiled::{CompiledStep as S,CompiledPredicate as P,CompiledPlayerRef};
use digimon_engine::{debug_runner::{DebugRunner,make_test_card},dsl_cards::{bindings::Bindings,step::run_steps},effect_context::EffectContext,action::{build_action_mask,space::PASS}};
fn draw(n:u8)->S{S::Draw{of:CompiledPlayerRef::You,count:n}}
fn pick(name:&str)->S {S::SelectOpponentPermanent {then:vec![],filter:P::default(),bind_as:Some(name.into()),selector:None,prompt:name.into(),prompt_key:None,optional:true,continue_on_decline:true}}
fn check(name:&str,yes:u8,no:u8)->S {S::If{condition:P{binding_exists:Some(name.into()),..P::default()},then:vec![draw(yes)],else_branch:vec![draw(no)]}}
#[test]
fn nested_if_optional_binding_accept_decline_and_repark_clone() {
 for first_accept in [false,true] {for second_accept in [false,true] {
  let mut r=DebugRunner::builder().add_card(make_test_card("SRC","SRC")).add_card(make_test_card("TGT","TGT")).hand(0,&["SRC"]).deck(0,&["SRC";40]).deck(1,&["SRC";10]).memory(5).start();
  r.place_on_field(1,"TGT",None);let c=r.game.players[0].hand[0].handle();let initial=r.game.players[0].hand.len();
  let steps=vec![S::If{condition:P::default(),then:vec![S::Optional(vec![S::If{condition:P::default(),then:vec![pick("first"),draw(1)],else_branch:vec![]}]),check("first",2,3)],else_branch:vec![]},pick("second"),check("first",4,5),check("second",6,7)];
  {let mut ctx=EffectContext::new(&mut r.game,c,None,0);run_steps(&steps,&mut ctx,&mut Bindings::new());}
  assert_eq!(r.game.players[0].hand.len(),initial);let g=r.game.clone();
  for _ in 0..2 {r.game=g.clone();let v=r.pending_selection_view().unwrap();let a=if first_accept{v.valid_action_ids.iter().copied().find(|a|*a!=PASS).unwrap()}else{PASS};assert_eq!(build_action_mask(&r.game,v.selecting_player)[a as usize],1.0);r.execute_action(v.selecting_player,a).unwrap();
   assert_eq!(r.game.players[0].hand.len(),initial+1+if first_accept{2}else{3});assert_eq!(r.pending_selection_view().unwrap().prompt,"second");let parked=r.game.clone();
   for _ in 0..2 {r.game=parked.clone();let v=r.pending_selection_view().unwrap();let a=if second_accept{v.valid_action_ids.iter().copied().find(|a|*a!=PASS).unwrap()}else{PASS};assert_eq!(build_action_mask(&r.game,v.selecting_player)[a as usize],1.0);r.execute_action(v.selecting_player,a).unwrap();assert_eq!(r.game.players[0].hand.len(),initial+1+(if first_accept{6}else{8})+(if second_accept{6}else{7}));assert!(r.game.pending_selection.is_none());}
  }
 }}
}
