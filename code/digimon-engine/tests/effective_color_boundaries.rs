use digimon_engine::{debug_runner::{DebugRunner,make_test_card},enums::{CardColor,ModifierType,Expiry,CardKind},modifiers::{ModifierEntry,ModifierPayload},effect_context::EffectReadContext,dsl_cards::{bindings::Bindings,predicate::{eval_predicate,eval_predicate_with_bindings,PredicateSubject}},permanent::PermanentHandle};
use digimon_dsl::compiled::{CompiledColor,CompiledPredicate,CompiledPlayerRef};
fn setup()->(DebugRunner,PermanentHandle){
 let mut green=make_test_card("COLOR-GREEN","Printed green");green.colors=vec![CardColor::Green];green.play_cost=2;
 let mut blue=make_test_card("COLOR-BLUE","Printed blue");blue.colors=vec![CardColor::Blue];blue.play_cost=2;
 let mut tamer=make_test_card("COLOR-TAMER","Printed green Tamer");tamer.colors=vec![CardColor::Green];tamer.card_kind=CardKind::Tamer;tamer.dp=None;tamer.level=None;
 let mut r=DebugRunner::builder().add_card(green).add_card(blue).add_card(tamer).dsl_card("P-156").unwrap().deck(0,&["COLOR-BLUE";12]).deck(1,&["COLOR-BLUE";12]).memory(10).start();r.skip_mulligan();
 let h=r.place_on_field(0,"COLOR-GREEN",Some(0));overlay(&mut r,h);(r,h)
}
fn overlay(r:&mut DebugRunner,h:PermanentHandle){r.game.modifiers.add(h,ModifierEntry::simple(ModifierType::ChangeBaseCardColor,0,Expiry::Permanent,h.player).with_payload(ModifierPayload::Colors{value:vec![CardColor::Blue],base:true}));}
fn leaf(c:CompiledColor)->CompiledPredicate{CompiledPredicate{color_is:Some(c),..Default::default()}}
#[test]fn nested_permanent_color_combinators_preserve_effective_and_printed_boundaries(){
 let(r,h)=setup();let card=r.top_card(h);
 for game in [&r.game,&r.game.clone()]{let ctx=EffectReadContext::new(game,card,Some(h),0);
 let cases=[(CompiledPredicate{all_of:vec![leaf(CompiledColor::Blue)],..Default::default()},true,false),
 (CompiledPredicate{any_of:vec![leaf(CompiledColor::Blue),leaf(CompiledColor::Red)],..Default::default()},true,false),
 (CompiledPredicate{none_of:vec![leaf(CompiledColor::Green)],..Default::default()},true,false),
 (CompiledPredicate{not:Some(Box::new(leaf(CompiledColor::Green))),..Default::default()},true,false),
 (CompiledPredicate{all_of:vec![CompiledPredicate{not:Some(Box::new(leaf(CompiledColor::Blue))),..Default::default()}],..Default::default()},false,true)];
 for(p,effective,printed)in cases{assert_eq!(eval_predicate(&p,&ctx,PredicateSubject::Permanent(h)),effective);assert_eq!(eval_predicate(&p,&ctx,PredicateSubject::Card(card)),printed);}}
}
#[test]fn permanent_color_comparison_uses_effective_candidate_but_card_search_stays_printed(){
 let(mut r,h)=setup();r.place_on_field(1,"COLOR-BLUE",Some(0));let card=r.top_card(h);let p=CompiledPredicate{color_matches_any_field_digimon:Some(CompiledPlayerRef::Opponent),..Default::default()};
 for game in [&r.game,&r.game.clone()]{let ctx=EffectReadContext::new(game,card,Some(h),0);assert!(!eval_predicate(&p,&ctx,PredicateSubject::Card(card)));assert!(eval_predicate(&p,&ctx,PredicateSubject::Permanent(h)),"field candidate is effectively blue");}
}
#[test]fn bound_field_tamer_color_is_effective_while_hand_candidates_are_printed(){
 let(mut r,h)=setup();let tamer=r.place_on_field(0,"COLOR-TAMER",Some(0));overlay(&mut r,tamer);let blue=r.add_to_hand(0,"COLOR-BLUE");let green=r.add_to_hand(0,"COLOR-GREEN");let blue=r.game.players[0].hand[blue].handle();let green=r.game.players[0].hand[green].handle();let mut binds=Bindings::default();binds.insert_permanent("chosen_tamer",tamer);let p=CompiledPredicate{color_matches_binding:Some("chosen_tamer".into()),..Default::default()};
 for game in [&r.game,&r.game.clone()]{let ctx=EffectReadContext::new(game,r.top_card(h),Some(h),0);assert!(eval_predicate_with_bindings(&p,&ctx,PredicateSubject::Card(blue),Some(&binds)));assert!(!eval_predicate_with_bindings(&p,&ctx,PredicateSubject::Card(green),Some(&binds)));}
}
fn snapshot_check(nested:bool){
 let(mut r,h)=setup();let card=r.top_card(h);let perm=&r.game.players[0].battle_area[0];let snap=digimon_engine::trigger_context::BattleCombatantSnapshot{battle_identity:1,survived:false,controller:0,top_card:card,data_index:perm.top_card().data_index,identity:perm.synth_identity(&r.game.card_data,&r.game.modifiers,h),is_suspended:false};
 assert_eq!(snap.identity.colors,vec![CardColor::Blue]);r.game.return_to_hand(h);let mut event=digimon_engine::trigger_context::TriggerContext::default();event.battle_deleters.push(snap);r.game.current_trigger_context=Some(event);
 for game in [&r.game,&r.game.clone()]{let ctx=EffectReadContext::new(game,card,None,0);
 for(c,want)in[(CompiledColor::Blue,true),(CompiledColor::Green,false)]{let child=if nested{CompiledPredicate{all_of:vec![leaf(c)],..Default::default()}}else{leaf(c)};let p=CompiledPredicate{event_battle_deleter:Some(Box::new(child)),..Default::default()};assert_eq!(eval_predicate(&p,&ctx,PredicateSubject::None),want,"snapshot color is effective at battle time, not unioned with replaced printed color");}}
}
#[test]fn battle_snapshot_color_does_not_restore_replaced_printed_color(){snapshot_check(false)}
#[test]fn battle_snapshot_nested_color_uses_same_effective_identity(){snapshot_check(true)}

#[test]
fn decoy_generated_nested_filter_matches_effective_candidate_color_gate() {
    let (r, h) = setup();
    let card = r.top_card(h);
    for game in [&r.game, &r.game.clone()] {
        let ctx = EffectReadContext::new(game, card, Some(h), 0);
        let perm = &game.players[0].battle_area[h.index as usize];
        for (color, expected) in [(CardColor::Blue, true), (CardColor::Green, false)] {
            let mask = 1u8 << (color as u8);
            let effect = digimon_engine::cards::keyword_effects::keyword_to_auto_effect(
                digimon_engine::enums::Keyword::Decoy(mask), card,
            ).remove(0);
            let filter = effect.paid_prevention_filter.as_ref().unwrap();
            let candidate_color_gate = perm.colors_for_rules(&game.card_data, &game.modifiers, h)
                .iter().any(|c| mask & (1u8 << (*c as u8)) != 0);
            assert_eq!(candidate_color_gate, expected);
            assert_eq!(eval_predicate(filter, &ctx, PredicateSubject::Permanent(h)), expected);
        }
    }
}