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

// These two fixtures perturb only the host lifecycle while an actual legal
// replacement prompt is parked. They distinguish identity from card/slot reuse.
#[test]
fn remaining_source_tracks_evolved_host_but_rejects_fresh_same_card_permanent(){
    for replace in [false,true]{
        let mut r=DebugRunner::builder().from_dsl_yaml(DRIVER).unwrap()
            .add_card(make_test_card("BATCH-A","Batch Alpha"))
            .add_card(make_test_card("BATCH-B","Batch Beta"))
            .add_card(make_test_card("BATCH-GATE","Batch Gate"))
            .add_card(make_test_card("BATCH-TOP","Batch Evolved"))
            .add_card(make_test_card("BATCH-FILL","Batch Filler"))
            .deck(0,&["BATCH-FILL";20]).deck(1,&["BATCH-FILL";20]).memory(10).start();
        r.skip_mulligan();r.set_first_player(0);r.game.current_phase=GamePhase::Main;
        let host=r.place_stack(0,&["BATCH-A","BATCH-B","BATCH-CARRIER"]);
        r.register_effect("BATCH-GATE",Arc::new(Gate));r.place_on_field(0,"BATCH-GATE",None);
        let hand=r.hand_size(0);
        r.game.enqueue_triggered(EffectTiming::OnPlay,TriggerSource::Permanent(host));r.game.drain_effect_queue();
        for _ in 0..2{let action=r.pending_selection_view().unwrap().valid_action_ids[0];act(&mut r,0,action);}
        assert_eq!(r.pending_selection_view().unwrap().kind,SelectionKind::Replacement);
        if replace{
            let mut old=r.game.player_mut(0).battle_area.remove(0);
            let top=old.card_sources.pop().unwrap();
            let mut fresh=digimon_engine::permanent::Permanent::new(top,r.game.turn_count);
            fresh.card_sources.splice(0..0,old.card_sources);
            r.game.player_mut(0).battle_area.insert(0,fresh);
        }else{
            r.add_to_hand(0,"BATCH-TOP");let top=r.game.player_mut(0).hand.pop().unwrap();let turn=r.game.turn_count;
            r.game.player_mut(0).battle_area[0].digivolve(top,turn);
        }
        let snapshot=r.game.clone();
        for cloned in [false,true]{
            if cloned{r.game=snapshot.clone();}
            act(&mut r,0,PASS);
            if replace{
                assert!(r.pending_selection_view().is_none(),"fresh same-card permanent cannot satisfy captured host identity");
                assert_eq!(r.battle_area_size(0),2);
                assert_eq!(r.game.player(0).battle_area[0].card_sources.len(),3,"both uncommitted sources stay in fresh host; neither captured origin may retarget it");
            }else{
                assert_eq!(r.pending_selection_view().expect("second source after host evolution").kind,SelectionKind::Replacement);
                act(&mut r,0,PASS);
                assert!(r.pending_selection_view().is_none());assert_eq!(r.battle_area_size(0),4);
                assert_eq!(r.game.player(0).battle_area[0].card_sources.len(),2,"evolved carrier retains old and new top");
            }
            assert_eq!(r.hand_size(0),hand+1,"outer tail executes once in either branch");
        }
    }
}

#[test]
fn named_source_play_result_does_not_retarget_same_physical_card_replayed_by_tail(){
    let yaml=DRIVER.replace("max: 2","max: 1")
        .replace("          cost_delta: free","          cost_delta: free\n          bind_as: played")
        .replace("      - draw: { of: you, count: 1 }",r#"      - delete_permanent: { target: played }
      - select_trash:
          of: you
          bind_as: reborn
          filter: { name_is: Batch Alpha }
          prompt: Choose same physical card
      - play_from_trash_free: { of: you, hand_index: reborn }
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
    r.place_on_field(0,"BATCH-GATE",None);let hand=r.hand_size(0);
    r.game.enqueue_triggered(EffectTiming::OnPlay,TriggerSource::Permanent(host));r.game.drain_effect_queue();
    let action=r.pending_selection_view().unwrap().valid_action_ids[0];act(&mut r,0,action);
    let snapshot=r.game.clone();
    for cloned in [false,true]{
        if cloned{r.game=snapshot.clone();}
        assert_eq!(r.pending_selection_view().unwrap().kind,SelectionKind::Replacement);
        act(&mut r,0,PASS);
        let selected=r.pending_selection_view().expect("trash selection after explicit deletion");
        assert_eq!(selected.valid_action_ids.len(),1);
        act(&mut r,0,selected.valid_action_ids[0]);
        assert_eq!(r.pending_selection_view().expect("replay's independent replacement").kind,SelectionKind::Replacement);
        act(&mut r,0,PASS);
        assert!(r.pending_selection_view().is_none());
        assert_eq!(r.hand_size(0)-hand,4,"committed original source records success exactly once");
        assert_eq!(r.battle_area_size(0),3,"old played binding must not bounce same card's new permanent");
        assert_eq!(r.game.player(0).battle_area[2].top_card().card_id(&r.game.card_data),"BATCH-A");
        assert_eq!(r.trash_size(0),0);
    }
}
