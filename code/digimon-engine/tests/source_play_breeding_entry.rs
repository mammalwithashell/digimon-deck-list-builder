//! Breeding source sibling: orientation and refusal must retain the real origin.
//! Same explicit baseline mode as source_play_enters_suspended.rs.
use std::sync::Arc;
use digimon_engine::action::{build_action_mask, space::{PASS,REPLACEMENT_ACCEPT}};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card,DebugRunner};
use digimon_engine::effect::{CardEffect,Effect};
use digimon_engine::enums::{EffectTiming,GamePhase};
use digimon_engine::selection::{SelectionKind,TriggerSource};
const SOURCE:&str=r#"
card: BENTRY-SOURCE
name: Breeding Entry Source
kind: digimon
level: 3
color: [black]
cost: 0
dp: 1000
effects:
  - when: on_play
    condition:
      any_permanent: { of: you, is_source: true, is_suspended: true }
    process:
      - draw: { of: you, count: 2 }
"#;
fn driver()->String {
    let flag=if std::env::var_os("SOURCE_ENTRY_SUSPENDED_BASELINE").is_none(){"                  suspended: true\n"}else{""};
    format!(r#"
card: BENTRY-DRIVER
name: Breeding Entry Driver
kind: digimon
level: 6
color: [black]
cost: 10
dp: 10000
effects:
  - when: on_play
    process:
      - select_own_breeding_permanent:
          bind_as: breeding_host
          prompt: Choose breeding carrier
          then:
            - select_material:
                of_permanent: breeding_host
                bind_as: picked
                optional: true
                filter: {{ name_is: Breeding Entry Source }}
                prompt: Choose breeding source
            - play_from_materials:
                target: breeding_host
                source_index: picked
                cost_delta: free
{flag}"#).replace("                  suspended:","                suspended:")
}
struct Gate;
impl CardEffect for Gate {
    fn effects(&self,card:CardHandle)->Vec<Effect>{vec![Effect::when_permanent_would_play(card)
        .optional().name("Refuse breeding source play")
        .replacement_condition(|ctx,subject|match subject{
            digimon_engine::replacement::ReplacementSubject::Card(h,_)=>ctx.game.card_data_for_handle(*h).is_some_and(|d|d.card_id=="BENTRY-SOURCE"),_=>false})
        .replacement_process(|ctx|ctx.cancel()).build()]}
}
fn setup(actor:u8)->DebugRunner{
    let mut r=DebugRunner::builder().from_dsl_yaml(&driver()).unwrap().from_dsl_yaml(SOURCE).unwrap()
        .add_card(make_test_card("BENTRY-CARRIER","Breeding Carrier"))
        .add_card(make_test_card("BENTRY-GATE","Breeding Gate"))
        .add_card(make_test_card("BENTRY-FILL","Breeding Filler"))
        .deck(0,&["BENTRY-FILL";20]).deck(1,&["BENTRY-FILL";20]).memory(10).start();
    r.skip_mulligan();r.set_first_player(actor);r.game.current_phase=GamePhase::Main;
    r.place_stack(actor,&["BENTRY-SOURCE","BENTRY-CARRIER"]);
    let carrier=r.game.player_mut(actor).battle_area.remove(0);
    r.game.player_mut(actor).breeding_area=Some(carrier);
    let driver=r.place_on_field(actor,"BENTRY-DRIVER",None);
    r.register_effect("BENTRY-GATE",Arc::new(Gate));r.place_on_field(actor,"BENTRY-GATE",None);
    r.game.enqueue_triggered(EffectTiming::OnPlay,TriggerSource::Permanent(driver));r.game.drain_effect_queue();r
}
fn act(r:&mut DebugRunner,actor:u8,action:u16){
    assert!(build_action_mask(&r.game,actor)[action as usize]>0.0);r.game.resolve_selection(actor,action).unwrap();
}
fn pick_only(r:&mut DebugRunner,actor:u8){let v=r.pending_selection_view().expect("pending choice");assert_eq!(v.valid_action_ids.len(),1);assert_eq!(v.selecting_player,actor);act(r,actor,v.valid_action_ids[0]);}
#[test]
fn breeding_source_enters_suspended_after_replacement_decline_and_clone(){
    for actor in [0,1]{let mut r=setup(actor);let hand=r.hand_size(actor);pick_only(&mut r,actor);pick_only(&mut r,actor);
        assert_eq!(r.pending_selection_view().unwrap().kind,SelectionKind::Replacement);let snapshot=r.game.clone();
        for cloned in [false,true]{if cloned{r.game=snapshot.clone();}act(&mut r,actor,PASS);
            assert!(r.pending_selection_view().is_none());let p=r.game.player(actor).battle_area.last().unwrap();
            assert_eq!(p.top_card().card_id(&r.game.card_data),"BENTRY-SOURCE");assert!(p.is_suspended);
            assert_eq!(r.hand_size(actor)-hand,2,"OnPlay saw suspended entry");
            assert_eq!(r.game.player(actor).breeding_area.as_ref().unwrap().card_sources.len(),1);
        }
    }
}
#[test]
fn refused_breeding_source_returns_to_exact_breeding_stack_in_clones(){
    for actor in [0,1]{let mut r=setup(actor);let hand=r.hand_size(actor);
        let sources=r.game.player(actor).breeding_area.as_ref().unwrap().card_sources.iter().map(|c|c.handle()).collect::<Vec<_>>();
        pick_only(&mut r,actor);pick_only(&mut r,actor);let snapshot=r.game.clone();
        for cloned in [false,true]{if cloned{r.game=snapshot.clone();}act(&mut r,actor,REPLACEMENT_ACCEPT);
            assert!(r.pending_selection_view().is_none());
            assert_eq!(r.game.player(actor).breeding_area.as_ref().unwrap().card_sources.iter().map(|c|c.handle()).collect::<Vec<_>>(),sources);
            assert_eq!(r.hand_size(actor),hand);assert_eq!(r.trash_size(actor),0);assert_eq!(r.battle_area_size(actor),2);
        }
    }
}
