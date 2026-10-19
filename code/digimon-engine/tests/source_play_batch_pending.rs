//! Two chosen source cards must retain remaining plays across replacement parking.
use std::sync::Arc;
use digimon_engine::action::{build_action_mask, space::{PASS,REPLACEMENT_ACCEPT}};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card,DebugRunner};
use digimon_engine::effect::{CardEffect,Effect};
use digimon_engine::enums::{EffectTiming,GamePhase};
use digimon_engine::selection::{SelectionKind,TriggerSource};
const DRIVER:&str=r#"
card: BATCH-CARRIER
name: Batch Carrier
kind: digimon
level: 6
color: [black]
cost: 10
dp: 10000
effects:
  - when: on_play
    process:
      - select_materials:
          of_permanent: source
          max: 2
          uniqueness: name
          bind_as: picked
          prompt: Choose source pair
      - play_from_materials:
          target: source
          source_index: picked
          cost_delta: free
      - draw: { of: you, count: 1 }
"#;
struct Gate;
impl CardEffect for Gate {
    fn effects(&self,card:CardHandle)->Vec<Effect>{vec![Effect::when_permanent_would_play(card)
        .optional().name("Refuse selected source play")
        .replacement_condition(|ctx,subject|match subject{
            digimon_engine::replacement::ReplacementSubject::Card(h,_)=>ctx.game.card_data_for_handle(*h).is_some_and(|d|d.card_id=="BATCH-A" || d.card_id=="BATCH-B"),_=>false})
        .replacement_process(|ctx|ctx.cancel()).build()]}
}
fn act(r:&mut DebugRunner,actor:u8,action:u16){
    assert!(build_action_mask(&r.game,actor)[action as usize]>0.0);r.game.resolve_selection(actor,action).unwrap();
}
fn exercise_source_batch(refuse_first:bool,refuse_second:bool,cost:u16){
    let yaml=DRIVER.replace("cost_delta: free",&format!("cost_delta: {}\n          suspended: true",cost));
    for actor in [0,1]{
        let mut r=DebugRunner::builder().from_dsl_yaml(&yaml).unwrap()
            .add_card(make_test_card("BATCH-A","Batch Alpha"))
            .add_card(make_test_card("BATCH-B","Batch Beta"))
            .add_card(make_test_card("BATCH-GATE","Batch Gate"))
            .add_card(make_test_card("BATCH-FILL","Batch Filler"))
            .deck(0,&["BATCH-FILL";20]).deck(1,&["BATCH-FILL";20]).memory(10).start();
        r.skip_mulligan();r.set_first_player(actor);r.game.current_phase=GamePhase::Main;
        let host=r.place_stack(actor,&["BATCH-A","BATCH-B","BATCH-CARRIER"]);
        r.register_effect("BATCH-GATE",Arc::new(Gate));r.place_on_field(actor,"BATCH-GATE",None);
        let hand=r.hand_size(actor);
        r.game.enqueue_triggered(EffectTiming::OnPlay,TriggerSource::Permanent(host));r.game.drain_effect_queue();
        for _ in 0..2 {
            let v=r.pending_selection_view().expect("source multiselect");
            let action=*v.valid_action_ids.iter().find(|a|**a!=PASS).expect("source action");
            act(&mut r,actor,action);
        }
        assert_eq!(r.pending_selection_view().unwrap().kind,SelectionKind::Replacement);
        // Only the first selected source may have left its stack while its
        // replacement is pending. The second source remains uncommitted.
        assert_eq!(r.game.player(actor).battle_area[host.index as usize].card_sources.len(),3,
            "both selected physical sources stay in their carrier during preparation");
        assert_eq!(r.hand_size(actor),hand,"source preparation must not expose a selected card as a hand card");
        let snapshot=r.game.clone();
        for cloned in [false,true]{
            if cloned {r.game=snapshot.clone();}
            act(&mut r,actor,if refuse_first {REPLACEMENT_ACCEPT}else{PASS});
            assert_eq!(r.pending_selection_view().expect("second source replacement").kind,SelectionKind::Replacement);
            assert_eq!(r.battle_area_size(actor),2,"neither simultaneous source enters before the second pre-play window finishes");
            assert_eq!(r.hand_size(actor),hand,"prepared sources are not hand candidates");
            assert_eq!(r.game.player(actor).battle_area[host.index as usize].card_sources.len(),3,"all sources remain in origin until entry");
            assert_eq!(r.game.memory,10-if refuse_first {0}else{cost as i16},"first preparation pays once before the second window");
            act(&mut r,actor,if refuse_second {REPLACEMENT_ACCEPT}else{PASS});
            assert!(r.pending_selection_view().is_none());
            assert_eq!(r.battle_area_size(actor),4-usize::from(refuse_first)-usize::from(refuse_second));
            assert_eq!(r.game.memory,10-cost as i16*(2-i16::from(refuse_first)-i16::from(refuse_second)),"each successful preparation pays exactly once");
            assert!(r.game.player(actor).battle_area.iter().filter(|p|matches!(p.top_card().card_id(&r.game.card_data),"BATCH-A"|"BATCH-B")).all(|p|p.is_suspended));
            assert_eq!(r.hand_size(actor),hand+1,"outer tail executes exactly once after both plays");
            assert_eq!(r.game.player(actor).battle_area[host.index as usize].card_sources.len(),1+usize::from(refuse_first)+usize::from(refuse_second));
            for (index,id) in ["BATCH-A","BATCH-B"].iter().enumerate(){
                assert_eq!(r.game.player(actor).battle_area.iter().filter(|p|p.top_card().card_id(&r.game.card_data)==*id).count(),if (refuse_first && index==0) || (refuse_second && index==1) {0}else{1});
            }
        }
    }
}

#[test]
fn two_selected_sources_park_each_play_before_remaining_batch_and_tail_in_clones(){exercise_source_batch(false,false,0);}
#[test]
fn refusing_first_source_restores_it_and_still_resolves_second_play_and_outer_tail(){exercise_source_batch(true,false,0);}

#[test]
fn parked_source_play_result_survives_explicit_tail_deletion_but_refusal_records_no_success(){
    let yaml=DRIVER.replace("max: 2","max: 1")
        .replace("          cost_delta: free","          cost_delta: free\n          bind_as: played")
        .replace("      - draw: { of: you, count: 1 }",r#"      - delete_permanent: { target: played }
      - if:
          condition: { effect_played_any_digimon: true }
          then:
            - draw: { of: you, count: 4 }
      - return_to_hand: { target: played }
"#);
    let mut r=DebugRunner::builder().from_dsl_yaml(&yaml).unwrap()
        .add_card(make_test_card("BATCH-A","Batch Alpha"))
        .add_card(make_test_card("BATCH-GATE","Batch Gate"))
        .add_card(make_test_card("BATCH-FILL","Batch Filler"))
        .deck(0,&["BATCH-FILL";20]).deck(1,&["BATCH-FILL";20]).memory(10).start();
    r.skip_mulligan();r.set_first_player(0);r.game.current_phase=GamePhase::Main;
    let host=r.place_stack(0,&["BATCH-A","BATCH-CARRIER"]);
    r.register_effect("BATCH-GATE",Arc::new(Gate));
    r.place_on_field(0,"BATCH-GATE",None);
    let hand=r.hand_size(0);
    r.game.enqueue_triggered(EffectTiming::OnPlay,TriggerSource::Permanent(host));r.game.drain_effect_queue();
    let action=r.pending_selection_view().unwrap().valid_action_ids[0];act(&mut r,0,action);
    assert_eq!(r.pending_selection_view().unwrap().kind,SelectionKind::Replacement);
    let snapshot=r.game.clone();
    for refuse in [false,true]{
        for cloned in [false,true]{
            r.game=snapshot.clone();
            if cloned {r.game=r.game.clone();}
            act(&mut r,0,if refuse {REPLACEMENT_ACCEPT}else{PASS});
            assert!(r.pending_selection_view().is_none());
            assert_eq!(r.hand_size(0)-hand,if refuse {0}else{4},"actual commit drives result log after explicit tail deletion");
            assert_eq!(r.battle_area_size(0),2,"played binding must not return unrelated survivor");
            assert_eq!(r.trash_size(0),if refuse {0}else{1});
            assert_eq!(r.game.player(0).battle_area[host.index as usize].card_sources.len(),if refuse {2}else{1});
        }
    }
}

#[test]
fn refusing_second_restores_only_second_after_first_prepared_in_clones(){exercise_source_batch(false,true,0);}
#[test]
fn refusing_both_preserves_both_physical_sources_in_clones(){exercise_source_batch(true,true,0);}

#[test]
fn prepared_source_costs_are_paid_once_across_all_replacement_choices(){for a in [false,true]{for b in [false,true]{exercise_source_batch(a,b,2);}}}

struct NestedPlay;
impl CardEffect for NestedPlay {
    fn effects(&self,card:CardHandle)->Vec<Effect>{vec![Effect::when_permanent_would_play(card)
        .optional().name("Play independent card during second preparation")
        .replacement_condition(|ctx,subject|match subject{
            digimon_engine::replacement::ReplacementSubject::Card(h,_)=>ctx.game.card_data_for_handle(*h).is_some_and(|d|d.card_id=="BATCH-B"),_=>false})
        .replacement_process(|ctx|{
            let player=ctx.effect.player;
            let index=ctx.effect.game.player(player).hand.iter().position(|c|c.card_id(&ctx.effect.game.card_data)=="BATCH-NESTED").expect("independent card");
            ctx.effect.play_from_hand_free(player,index);
        }).build()]}
}
#[test]
fn nested_ordinary_play_does_not_capture_or_erase_prepared_source_declarations(){
    let mut r=DebugRunner::builder().from_dsl_yaml(DRIVER).unwrap()
        .add_card(make_test_card("BATCH-A","Batch Alpha"))
        .add_card(make_test_card("BATCH-B","Batch Beta"))
        .add_card(make_test_card("BATCH-NESTED","Independent Play"))
        .add_card(make_test_card("BATCH-GATE","Batch Gate"))
        .add_card(make_test_card("BATCH-FILL","Batch Filler"))
        .deck(0,&["BATCH-FILL";20]).deck(1,&["BATCH-FILL";20]).memory(10).start();
    r.skip_mulligan();r.set_first_player(0);r.game.current_phase=GamePhase::Main;
    let host=r.place_stack(0,&["BATCH-A","BATCH-B","BATCH-CARRIER"]);
    r.register_effect("BATCH-GATE",Arc::new(NestedPlay));r.place_on_field(0,"BATCH-GATE",None);
    r.add_to_hand(0,"BATCH-NESTED");
    let hand=r.hand_size(0);
    r.game.enqueue_triggered(EffectTiming::OnPlay,TriggerSource::Permanent(host));r.game.drain_effect_queue();
    for _ in 0..2 {
        let action=*r.pending_selection_view().unwrap().valid_action_ids.iter().find(|a|**a!=PASS).unwrap();
        act(&mut r,0,action);
    }
    assert_eq!(r.pending_selection_view().unwrap().kind,SelectionKind::Replacement);
    let snapshot=r.game.clone();
    for cloned in [false,true]{
        r.game=snapshot.clone();if cloned{r.game=r.game.clone();}
        act(&mut r,0,REPLACEMENT_ACCEPT);
        assert!(r.pending_selection_view().is_none());
        for id in ["BATCH-A","BATCH-B","BATCH-NESTED"]{
            assert_eq!(r.game.player(0).battle_area.iter().filter(|p|p.top_card().card_id(&r.game.card_data)==id).count(),1,"each independent physical play commits: {id}");
        }
        assert_eq!(r.hand_size(0),hand,"independent play consumes one hand card, outer tail draws once");
        assert_eq!(r.game.player(0).battle_area[host.index as usize].card_sources.len(),1);
    }
}

#[test]
fn single_selected_source_retains_actual_origin_through_cloned_would_play_choices(){
 let yaml=DRIVER.replace("max: 2","max: 1");
 let mut r=DebugRunner::builder().from_dsl_yaml(&yaml).unwrap().add_card(make_test_card("BATCH-A","Batch Alpha"))
  .add_card(make_test_card("BATCH-GATE","Gate")).add_card(make_test_card("BATCH-FILL","Filler"))
  .deck(0,&["BATCH-FILL";20]).deck(1,&["BATCH-FILL";20]).memory(10).start();
 r.skip_mulligan();r.game.enter_main_phase();let host=r.place_stack(0,&["BATCH-A","BATCH-CARRIER"]);
 r.register_effect("BATCH-GATE",Arc::new(Gate));r.place_on_field(0,"BATCH-GATE",None);let hand=r.hand_size(0);
 r.game.enqueue_triggered(EffectTiming::OnPlay,TriggerSource::Permanent(host));r.game.drain_effect_queue();
 let a=r.pending_selection_view().unwrap().valid_action_ids[0];act(&mut r,0,a);
 assert_eq!(r.pending_selection_view().unwrap().kind,SelectionKind::Replacement);
 assert_eq!(r.hand_size(0),hand,"a pending source play is not a hand card");
 assert_eq!(r.game.player(0).battle_area[host.index as usize].card_sources.len(),2,"the source remains under its carrier until entry");
 let saved=r.game.clone();for refuse in [false,true]{r.game=saved.clone();act(&mut r,0,if refuse{REPLACEMENT_ACCEPT}else{PASS});
  assert_eq!(r.game.player(0).battle_area[host.index as usize].card_sources.len(),if refuse{2}else{1});
  assert_eq!(r.hand_size(0),hand+1);assert_eq!(r.battle_area_size(0),if refuse{2}else{3});
 }
}
struct BatchBroadcastWitness;
impl CardEffect for BatchBroadcastWitness {
 fn effects(&self,c:CardHandle)->Vec<Effect>{vec![
  Effect::on_enter_field_anyone(c).process(|ctx|{ctx.draw(0,1);}).build(),
  Effect::on_ally_played(c).process(|ctx|{ctx.draw(0,3);}).build(),
 ]}
}
#[test]
fn simultaneous_source_pair_broadcasts_each_condition_once_after_all_enter_including_parked_path(){
 for parked in [false,true]{
 let mut b=DebugRunner::builder().from_dsl_yaml(DRIVER).unwrap();
 for id in ["BATCH-A","BATCH-B","BATCH-GATE","BATCH-OBSERVER","BATCH-FILL"]{b=b.add_card(make_test_card(id,id));}
 let mut r=b.deck(0,&["BATCH-FILL";40]).deck(1,&["BATCH-FILL";40]).memory(10).start();
 r.skip_mulligan();r.game.enter_main_phase();let host=r.place_stack(0,&["BATCH-A","BATCH-B","BATCH-CARRIER"]);
 r.register_effect("BATCH-OBSERVER",Arc::new(BatchBroadcastWitness));r.place_on_field(0,"BATCH-OBSERVER",None);
 if parked {r.register_effect("BATCH-GATE",Arc::new(Gate));r.place_on_field(0,"BATCH-GATE",None);}
 let hand=r.hand_size(0);r.game.enqueue_triggered(EffectTiming::OnPlay,TriggerSource::Permanent(host));r.game.drain_effect_queue();
 for _ in 0..2 {let a=r.pending_selection_view().unwrap().valid_action_ids[0];act(&mut r,0,a);}
 let saved=r.game.clone();for cloned in [false,true]{r.game=saved.clone();if cloned{r.game=r.game.clone();}
  if parked {act(&mut r,0,PASS);act(&mut r,0,PASS);}r.auto_resolve().unwrap();
  assert_eq!(r.hand_size(0),hand+5,"one outer draw, one OnEnterFieldAnyone, one OnAllyPlayed: parked={parked}");
  for id in ["BATCH-A","BATCH-B"]{assert_eq!(r.game.player(0).battle_area.iter().filter(|p|p.top_card().card_id(&r.game.card_data)==id).count(),1);}
 }
 }
}

struct EntrantBroadcastWitness;
impl CardEffect for EntrantBroadcastWitness {
 fn effects(&self,c:CardHandle)->Vec<Effect>{vec![Effect::on_ally_played(c)
  .condition(|ctx|ctx.game.players[0].battle_area.iter().filter(|p|matches!(p.top_card().card_id(&ctx.game.card_data),"BATCH-A"|"BATCH-B")).count()==2)
  .process(|ctx|{ctx.draw(0,1);}).build()]}
}
struct SecondEntrantWitness;
impl CardEffect for SecondEntrantWitness {
 fn effects(&self,c:CardHandle)->Vec<Effect>{vec![Effect::on_ally_played(c)
  .condition(|ctx|ctx.event_card_name_contains("BATCH-B"))
  .process(|ctx|{ctx.draw(0,3);}).build()]}
}
#[test]
fn both_entrant_observers_see_whole_pair_and_second_only_event_filter_is_preserved(){
 let mut b=DebugRunner::builder().from_dsl_yaml(DRIVER).unwrap();
 for id in ["BATCH-A","BATCH-B","BATCH-OBSERVER","BATCH-FILL"]{b=b.add_card(make_test_card(id,id));}
 let mut r=b.deck(0,&["BATCH-FILL";40]).deck(1,&["BATCH-FILL";40]).memory(10).start();
 r.skip_mulligan();r.game.enter_main_phase();let host=r.place_stack(0,&["BATCH-A","BATCH-B","BATCH-CARRIER"]);
 r.register_effect("BATCH-A",Arc::new(EntrantBroadcastWitness));r.register_effect("BATCH-B",Arc::new(EntrantBroadcastWitness));
 r.register_effect("BATCH-OBSERVER",Arc::new(SecondEntrantWitness));r.place_on_field(0,"BATCH-OBSERVER",None);
 let hand=r.hand_size(0);r.game.enqueue_triggered(EffectTiming::OnPlay,TriggerSource::Permanent(host));r.game.drain_effect_queue();
 for _ in 0..2 {let a=r.pending_selection_view().unwrap().valid_action_ids[0];act(&mut r,0,a);}
 r.auto_resolve().unwrap();
 assert_eq!(r.hand_size(0),hand+6,"outer draw + one per entrant observer + the observer matching only the second entrant");
}

#[test]
fn simultaneous_source_entrant_floodgate_does_not_retroactively_block_its_sibling(){
 const FLOOD:&str=r#"
card: BATCH-A
name: Batch Alpha floodgate
kind: digimon
level: 3
color: [yellow]
cost: 3
dp: 2000
effects:
  - kind: flood_gate
    scope: face_up
    active_when: { all_turns: true }
    modifier: CannotPlayDigimonByEffect
    target_player: any
"#;
 let mut r=DebugRunner::builder().from_dsl_yaml(DRIVER).unwrap().from_dsl_yaml(FLOOD).unwrap()
  .add_card(make_test_card("BATCH-B","Batch Beta")).add_card(make_test_card("BATCH-FILL","Filler"))
  .deck(0,&["BATCH-FILL";20]).deck(1,&["BATCH-FILL";20]).memory(10).start();
 r.skip_mulligan();r.game.enter_main_phase();let host=r.place_stack(0,&["BATCH-A","BATCH-B","BATCH-CARRIER"]);
 r.game.enqueue_triggered(EffectTiming::OnPlay,TriggerSource::Permanent(host));r.game.drain_effect_queue();
 for _ in 0..2 {let a=r.pending_selection_view().unwrap().valid_action_ids[0];act(&mut r,0,a);}
 r.auto_resolve().unwrap();
 for id in ["BATCH-A","BATCH-B"]{assert_eq!(r.game.player(0).battle_area.iter().filter(|p|p.top_card().card_id(&r.game.card_data)==id).count(),1,"simultaneous member {id} enters before newly active continuous prohibition can affect later events");}
 assert_eq!(r.game.player(0).battle_area[host.index as usize].card_sources.len(),1);
}

#[test]
fn simultaneous_subject_body_exposes_both_physical_entrants_and_cloned_choices(){
 const OBS:&str=r#"
card: BATCH-OBSERVER
name: Subject chooser
kind: digimon
level: 3
color: [yellow]
cost: 3
dp: 2000
effects:
  - when: on_ally_played
    condition: { event_card_name_contains: Batch }
    process:
      - return_to_hand: { target: event_target }
"#;
 let mut r=DebugRunner::builder().from_dsl_yaml(DRIVER).unwrap().from_dsl_yaml(OBS).unwrap()
  .add_card(make_test_card("BATCH-A","Batch Alpha")).add_card(make_test_card("BATCH-B","Batch Beta"))
  .add_card(make_test_card("BATCH-FILL","Filler"))
  .deck(0,&["BATCH-FILL";20]).deck(1,&["BATCH-FILL";20]).memory(10).start();
 r.skip_mulligan();r.game.enter_main_phase();let host=r.place_stack(0,&["BATCH-A","BATCH-B","BATCH-CARRIER"]);
 r.place_on_field(0,"BATCH-OBSERVER",None);let hand=r.hand_size(0);
 r.game.enqueue_triggered(EffectTiming::OnPlay,TriggerSource::Permanent(host));r.game.drain_effect_queue();
 for _ in 0..2 {let a=r.pending_selection_view().unwrap().valid_action_ids[0];act(&mut r,0,a);}
 let selection=r.pending_selection_view().expect("one effect chooses its simultaneous event subject");
 assert_eq!(selection.kind,SelectionKind::EffectChoice);
 let actions=selection.valid_action_ids.clone();assert_eq!(actions.len(),2);assert!(!actions.contains(&PASS));
 let saved=r.game.clone();let mut outcomes=Vec::new();
 for a in actions {r.game=saved.clone();act(&mut r,0,a);r.auto_resolve().unwrap();
  assert_eq!(r.hand_size(0),hand+2,"one outer draw and exactly one returned event subject");
  let remaining:Vec<_>=r.game.player(0).battle_area.iter().filter_map(|p|{let id=p.top_card().card_id(&r.game.card_data);matches!(id,"BATCH-A"|"BATCH-B").then_some(id.to_string())}).collect();
  assert_eq!(remaining.len(),1);outcomes.push(remaining[0].clone());
 }
 outcomes.sort();assert_eq!(outcomes,vec!["BATCH-A","BATCH-B"]);
}

struct SecondSubjectOptionalWitness;
impl CardEffect for SecondSubjectOptionalWitness {
 fn effects(&self,c:CardHandle)->Vec<Effect>{vec![Effect::on_ally_played(c).optional().needs_outer_optional_prompt()
  .outer_optional_guard(|ctx|ctx.event_card_name_contains("Beta"))
  .process(|ctx|{ctx.draw(0,3);}).build()]}
}
#[test]
fn grouped_optional_guard_can_match_second_subject_and_decline_remains_a_choice(){
 let mut b=DebugRunner::builder().from_dsl_yaml(DRIVER).unwrap();
 for (id,name) in [("BATCH-A","Batch Alpha"),("BATCH-B","Batch Beta"),("BATCH-OBSERVER","Observer"),("BATCH-FILL","Filler")]{b=b.add_card(make_test_card(id,name));}
 let mut r=b.deck(0,&["BATCH-FILL";40]).deck(1,&["BATCH-FILL";40]).memory(10).start();
 r.skip_mulligan();r.game.enter_main_phase();let host=r.place_stack(0,&["BATCH-A","BATCH-B","BATCH-CARRIER"]);
 r.register_effect("BATCH-OBSERVER",Arc::new(SecondSubjectOptionalWitness));r.place_on_field(0,"BATCH-OBSERVER",None);
 let hand=r.hand_size(0);r.game.enqueue_triggered(EffectTiming::OnPlay,TriggerSource::Permanent(host));r.game.drain_effect_queue();
 for _ in 0..2{let a=r.pending_selection_view().unwrap().valid_action_ids[0];act(&mut r,0,a);}
 let view=r.pending_selection_view().expect("the second subject makes optional effect actionable");
 assert!(build_action_mask(&r.game,0)[PASS as usize]>0.0);let accept=*view.valid_action_ids.iter().find(|a|**a!=PASS).unwrap();
 let saved=r.game.clone();for decline in [false,true]{r.game=saved.clone();act(&mut r,0,if decline{PASS}else{accept});r.auto_resolve().unwrap();assert_eq!(r.hand_size(0),hand+if decline{1}else{4});}
}
