//! Official general_rule.pdf p.7 3-1-3-9-1/-2; Mother Q3281/Q3284.
//! Real Mother metadata, no incomplete card-effect implementation.
use digimon_engine::debug_runner::{DebugRunner,make_test_egg,make_test_card};
use digimon_engine::enums::{CardColor,CardSourceRef,StackPosition};
use digimon_engine::permanent::PermanentHandle;
fn setup()->DebugRunner {
 let mut mother=make_test_egg("EX2-007","Mother D-Reaper");mother.level=None;mother.dp=Some(15000);mother.colors=vec![CardColor::White];
 let mut r=DebugRunner::builder().add_card(mother).add_card(make_test_egg("PROBE-EGG","Egg order marker"))
 .add_card(make_test_card("PROBE-SOURCE","Source"))
 .hand(0,&["PROBE-EGG"]).hand(1,&["PROBE-EGG"]).deck(0,&["PROBE-SOURCE";10]).deck(1,&["PROBE-SOURCE";10]).build();
 for owner in 0..2 {let egg=r.game.players[owner].hand.pop().unwrap();r.game.players[owner].digitama_deck.push(egg);}
 r.place_stack(0,&["PROBE-SOURCE","EX2-007"]);r
}
fn h()->PermanentHandle {PermanentHandle{player:0,index:0}}
#[test]
fn trash_to_hand_redirects_egg_to_owner_bottom_without_reporting_hand_addition() {
    let mut r=setup();
    let mut egg=r.game.players[0].battle_area[0].card_sources.pop().unwrap();
    egg.owner=1;
    let handle=egg.handle();
    r.game.players[0].trash.push(egg);
    let mut cloned=r.game.clone();
    for game in [&mut r.game,&mut cloned] {
        assert!(!game.add_to_hand_from_trash(0,handle),"no card actually added to hand");
        assert_eq!(game.players[1].digitama_deck[0].handle(),handle);
        assert_eq!(game.players[1].digitama_deck.len(),2);
        assert!(game.players[0].trash.is_empty());
        assert!(game.players.iter().all(|p|p.hand.iter().all(|c|c.handle()!=handle)));
    }
}
#[test]
fn misplaced_egg_in_hand_is_never_advertised_as_playable() {
    let mut r=setup();
    let egg=r.game.players[0].digitama_deck.pop().unwrap();
    r.game.players[0].hand.push(egg);
    r.set_first_player(0);r.set_phase(digimon_engine::enums::GamePhase::Main);
    assert_eq!(digimon_engine::action::build_action_mask(&r.game,0)[0],0.0);
}
struct HandAdditionObserver;
impl digimon_engine::effect::CardEffect for HandAdditionObserver {
    fn effects(&self,c:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect> {
        vec![digimon_engine::effect::EffectBuilder::new(c,digimon_engine::enums::EffectTiming::OnAddToHand)
            .process(|ctx| {ctx.gain_memory(1);}).build()]
    }
}
#[test]
fn redirected_egg_does_not_trigger_hand_addition_but_ordinary_card_does() {
    for egg in [false,true] {
        let mut r=setup();
        r.register_effect("PROBE-SOURCE",std::sync::Arc::new(HandAdditionObserver));
        r.place_stack(0,&["PROBE-SOURCE"]);
        let source=r.game.players[0].battle_area[1].top_card().handle();
        let card=if egg {r.game.players[0].digitama_deck.pop().unwrap()}
            else {r.game.players[0].deck.pop().unwrap()};
        let handle=card.handle();r.game.players[0].trash.push(card);
        let mut ctx=digimon_engine::effect_context::EffectContext::new(&mut r.game,source,None,0);
        assert_eq!(ctx.add_to_hand_from_trash(0,handle),!egg);
        let queued=r.game.effect_queue.iter().filter(|e|e.timing==digimon_engine::enums::EffectTiming::OnAddToHand).count();
        assert_eq!(queued,usize::from(!egg));
    }
}
fn assert_routed(r:&DebugRunner,owner:usize,top:bool){
 assert!(r.game.players[0].battle_area.is_empty());assert_eq!(r.game.players[0].trash.len(),1);
 let eggs=&r.game.players[owner].digitama_deck;assert_eq!(eggs.len(),2);assert_eq!(eggs[usize::from(top)].card_id(&r.game.card_data),"EX2-007");
 for p in &r.game.players {assert!(p.hand.iter().chain(p.deck.iter()).chain(p.security.iter()).all(|c|c.card_id(&r.game.card_data)!="EX2-007"));}
}
#[test]fn mother_return_to_hand_routes_bottom_egg_deck(){let mut r=setup();r.game.return_to_hand(h());assert_routed(&r,0,false);}
#[test]fn mother_return_to_deck_top_preserves_specified_top_in_egg_deck(){let mut r=setup();r.game.return_to_deck(h(),StackPosition::Top);assert_routed(&r,0,true);}
#[test]fn mother_return_to_deck_bottom_routes_bottom_egg_deck(){let mut r=setup();r.game.return_to_deck(h(),StackPosition::Bottom);assert_routed(&r,0,false);}
#[test]fn mother_security_placement_routes_bottom_egg_deck_and_discards_sources(){let mut r=setup();r.game.place_on_security(0,CardSourceRef::Material(h(),1),StackPosition::Top,false);assert_routed(&r,0,false);}
#[test]fn mother_private_routing_follows_physical_owner(){let mut r=setup();r.game.players[0].battle_area[0].card_sources[1].owner=1;r.game.return_to_hand(h());assert_routed(&r,1,false);}
#[test]fn ordinary_digimon_hand_destination_control(){let mut r=setup();r.game.players[0].battle_area[0].card_sources.pop();r.game.return_to_hand(h());assert_eq!(r.game.players[0].hand.len(),1);assert_eq!(r.game.players[0].digitama_deck.len(),1);}

struct EggDepartureProtection;
impl digimon_engine::effect::CardEffect for EggDepartureProtection {
 fn effects(&self,c:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect>{
  vec![digimon_engine::effect::Effect::when_would_leave_battle_area(c).optional().replacement_process(|ctx|ctx.cancel()).build()]
 }
}
#[test]fn egg_security_departure_optional_replacement_preserves_stack_or_clone_disposes_once(){
 let mut mother=make_test_egg("EX2-007","Mother D-Reaper");mother.level=None;mother.dp=Some(15000);
 let mut r=DebugRunner::builder().add_card(mother).add_card(make_test_card("PROBE-SOURCE","Source"))
  .deck(0,&["PROBE-SOURCE";10]).deck(1,&["PROBE-SOURCE";10]).build();
 r.register_effect("EX2-007",std::sync::Arc::new(EggDepartureProtection));
 r.place_stack(0,&["PROBE-SOURCE","EX2-007"]);
 r.game.place_on_security(0,CardSourceRef::Material(h(),1),StackPosition::Top,false);
 let pending=r.game.pending_selection.as_ref().expect("actual field departure protection");
 let yes=*pending.valid_action_ids.iter().find(|a|**a!=digimon_engine::action::space::PASS).unwrap();
 let saved=r.game.clone();r.execute_action(0,yes).unwrap();
 assert_eq!(r.game.players[0].battle_area.len(),1);assert_eq!(r.game.players[0].battle_area[0].card_sources.len(),2);assert!(r.game.players[0].trash.is_empty());assert!(r.game.players[0].digitama_deck.is_empty());
 r.game=saved;r.execute_action(0,digimon_engine::action::space::PASS).unwrap();
 assert!(r.game.players[0].battle_area.is_empty());assert_eq!(r.game.players[0].trash.len(),1);assert_eq!(r.game.players[0].digitama_deck.len(),1);assert!(r.game.players[0].security.is_empty());
}
#[test]fn ordinary_field_top_security_moves_only_top_and_discards_sources(){
 let mut r=DebugRunner::builder().add_card(make_test_card("SOURCE","Source")).add_card(make_test_card("TOP","Top"))
 .deck(0,&["SOURCE";10]).deck(1,&["SOURCE";10]).build();
 r.place_stack(0,&["SOURCE","TOP"]);let top=r.game.players[0].battle_area[0].top_card().handle();
 assert!(r.game.place_on_security(0,CardSourceRef::Material(h(),1),StackPosition::Top,false));
 assert!(r.game.players[0].battle_area.is_empty());assert_eq!(r.game.players[0].security.last().unwrap().handle(),top);
 assert_eq!(r.game.players[0].trash.len(),1);assert_eq!(r.game.players[0].trash[0].card_id(&r.game.card_data),"SOURCE");
}
#[test]fn parked_security_move_preserves_target_bottom_faceup_and_clone_cancel(){
 let mut r=DebugRunner::builder().add_card(make_test_card("SOURCE","Source")).add_card(make_test_card("TOP","Top"))
 .hand(1,&["SOURCE"]).deck(0,&["SOURCE";10]).deck(1,&["SOURCE";10]).build();
 r.register_effect("TOP",std::sync::Arc::new(EggDepartureProtection));
 r.place_stack(0,&["SOURCE","TOP"]);let top=r.game.players[0].battle_area[0].top_card().handle();
 let marker=r.game.players[1].hand.pop().unwrap();r.game.players[1].security.push(marker);
 r.game.place_on_security(1,CardSourceRef::Material(h(),1),StackPosition::Bottom,true);
 let pending=r.game.pending_selection.as_ref().expect("actual leave choice");
 let yes=*pending.valid_action_ids.iter().find(|a|**a!=digimon_engine::action::space::PASS).unwrap();
 let saved=r.game.clone();r.execute_action(0,digimon_engine::action::space::PASS).unwrap();
 assert!(r.game.players[0].battle_area.is_empty());assert!(r.game.players[0].security.is_empty());
 assert_eq!(r.game.players[1].security.len(),2);assert_eq!(r.game.players[1].security[0].handle(),top);
 assert!(r.game.players[1].face_up_security.contains(&r.game.players[1].security[0].card_index));
 assert_eq!(r.game.players[0].trash.len(),1);
 r.game=saved;r.execute_action(0,yes).unwrap();assert_eq!(r.game.players[0].battle_area.len(),1);
 assert_eq!(r.game.players[1].security.len(),1);assert!(r.game.players[0].trash.is_empty());
}

struct NestedDifferentSecurityDestination;
impl digimon_engine::effect::CardEffect for NestedDifferentSecurityDestination {
 fn effects(&self,c:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect>{
  use digimon_engine::effect::{Effect,EffectBuilder};
  use digimon_engine::enums::EffectTiming;
  vec![Effect::when_would_leave_battle_area(c).name("Nested destination probe")
   .replacement_process(|ctx|{
    let donor=ctx.subject.permanent().expect("field departure");
    let index=ctx.effect.game.players[donor.player as usize].battle_area[donor.index as usize].card_sources.len()-1;
    ctx.effect.game.place_on_security(1,CardSourceRef::Material(donor,index),StackPosition::Bottom,true);
   }).build(),
   EffectBuilder::new(c,EffectTiming::WhenWouldPlaceInSecurity).optional().name("Cancel inner security placement")
    .replacement_process(|ctx|ctx.cancel()).build()]
 }
}
#[test]fn nested_same_donor_security_cancel_inner_restores_outer_payload_and_clone_decline_uses_inner(){
 let mut r=DebugRunner::builder().add_card(make_test_card("SOURCE","Source")).add_card(make_test_card("TOP","Top"))
  .hand(0,&["SOURCE"]).hand(1,&["SOURCE"]).deck(0,&["SOURCE";10]).deck(1,&["SOURCE";10]).build();
 r.register_effect("TOP",std::sync::Arc::new(NestedDifferentSecurityDestination));
 r.place_stack(0,&["SOURCE","TOP"]);let top=r.game.players[0].battle_area[0].top_card().handle();
 for player in 0..2 {let marker=r.game.players[player].hand.pop().unwrap();r.game.players[player].security.push(marker);}
 r.game.place_on_security(0,CardSourceRef::Material(h(),1),StackPosition::Top,false);
 let pending=r.game.pending_selection.as_ref().expect("nested security-placement choice");
 let actor=pending.selecting_player;let yes=*pending.valid_action_ids.iter().find(|a|**a!=digimon_engine::action::space::PASS).unwrap();
 let saved=r.game.clone();r.execute_action(actor,yes).unwrap();
 assert!(r.pending_kind().is_none());assert!(r.game.players[0].battle_area.is_empty());
 assert_eq!(r.game.players[0].security.len(),2);assert_eq!(r.game.players[0].security.last().unwrap().handle(),top);
 assert!(!r.game.players[0].face_up_security.contains(&r.game.players[0].security.last().unwrap().card_index));
 assert_eq!(r.game.players[1].security.len(),1);assert_eq!(r.game.players[0].trash.len(),1);
 r.game=saved;r.execute_action(actor,digimon_engine::action::space::PASS).unwrap();
 assert!(r.pending_kind().is_none());assert!(r.game.players[0].battle_area.is_empty());
 assert_eq!(r.game.players[0].security.len(),1);assert_eq!(r.game.players[1].security.len(),2);
 assert_eq!(r.game.players[1].security[0].handle(),top);
 assert!(r.game.players[1].face_up_security.contains(&r.game.players[1].security[0].card_index));
 assert_eq!(r.game.players[0].trash.len(),1);
}
