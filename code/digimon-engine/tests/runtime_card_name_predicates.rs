//! Physical runtime aliases must participate in card-subject name matching.
use digimon_engine::debug_runner::{make_test_card,DebugRunner};
use digimon_engine::effect_context::EffectReadContext;
use digimon_engine::dsl_cards::predicate::{eval_predicate,PredicateSubject};
use digimon_dsl::compiled::CompiledPredicate;
fn exercise(leaf:&str){
 for owner in [0,1]{for zone in ["hand","trash","material"]{
  let mut cd=make_test_card("NAME-CAND","Printed Identity");cd.also_treated_as.push("Static Identity".into());
  let mut r=DebugRunner::builder().add_card(cd).add_card(make_test_card("NAME-SOURCE","Name Driver")).start();
  let host=r.place_stack(owner,&["NAME-SOURCE"]);let idx=r.add_to_hand(owner,"NAME-CAND");let mut card=r.game.player_mut(owner).hand.remove(idx);let handle=card.handle();card.also_treated_as.push("SkullKnightmon".into());
  match zone{"hand"=>r.game.player_mut(owner).hand.push(card),"trash"=>r.game.player_mut(owner).trash.push(card),_=>r.game.player_mut(owner).battle_area[0].card_sources.insert(0,card)}
  let snap=r.game.clone();for cloned in [false,true]{r.game=snap.clone();if cloned{r.game=r.game.clone();}
   let source=r.game.player(owner).battle_area[0].top_card().handle();let ctx=EffectReadContext::new(&r.game,source,Some(host),owner);
   let pred=match leaf{"is"=>CompiledPredicate{name_is:Some("SkullKnightmon".into()),..Default::default()},"in"=>CompiledPredicate{name_in:Some(vec!["Missing Identity".into(),"SkullKnightmon".into()]),..Default::default()},_=>CompiledPredicate{name_contains:Some("Knight".into()),..Default::default()}};
   assert!(eval_predicate(&pred,&ctx,PredicateSubject::Card(handle)),"runtime {leaf} alias in {zone} for {owner}");
   for name in ["Printed Identity","Static Identity"]{assert!(eval_predicate(&CompiledPredicate{name_is:Some(name.into()),..Default::default()},&ctx,PredicateSubject::Card(handle)));}
   assert!(!eval_predicate(&CompiledPredicate{name_is:Some("DeadlyAxemon".into()),..Default::default()},&ctx,PredicateSubject::Card(handle)));
  }
 }}
}
#[test]fn runtime_alias_name_is_across_hand_trash_and_material(){exercise("is");}
#[test]fn runtime_alias_name_in_across_hand_trash_and_material(){exercise("in");}
#[test]fn runtime_alias_name_contains_across_hand_trash_and_material(){exercise("contains");}
#[test]
fn union_choice_mask_exposes_runtime_named_physical_candidate(){
 use digimon_engine::action::{build_action_mask,space::PASS};use digimon_engine::enums::{EffectTiming,GamePhase};use digimon_engine::selection::TriggerSource;
 let yaml=r#"
card: NAME-DRIVER
name: Name Driver
kind: tamer
color: [black]
cost: 1
effects:
  - when: on_play
    process:
      - select_union_zone:
          of: you
          zones: [hand, trash]
          filter: { name_is: SkullKnightmon }
          optional: true
          bind_as: picked
          prompt: Choose a SkullKnightmon
          then:
            - gain_memory: 1
"#;
 for owner in [0,1]{let mut r=DebugRunner::builder().from_dsl_yaml(yaml).unwrap().add_card(make_test_card("NAME-CAND","Printed Identity")).memory(3).start();r.skip_mulligan();r.set_first_player(owner);r.game.current_phase=GamePhase::Main;let host=r.place_stack(owner,&["NAME-DRIVER"]);let i=r.add_to_hand(owner,"NAME-CAND");r.game.player_mut(owner).hand[i].also_treated_as.push("SkullKnightmon".into());r.game.enqueue_triggered(EffectTiming::OnPlay,TriggerSource::Permanent(host));r.game.drain_effect_queue();let v=r.pending_selection_view().expect("runtime named hand candidate must be offered");assert_eq!(v.selecting_player,owner);let a=*v.valid_action_ids.iter().find(|a|**a!=PASS).unwrap();assert!(build_action_mask(&r.game,owner)[a as usize]>0.0);let before=r.game.memory;let snap=r.game.clone();for cloned in [false,true]{r.game=snap.clone();if cloned{r.game=r.game.clone();}r.game.resolve_selection(owner,a).unwrap();assert_eq!(r.game.memory,before+1);}}
}
