//! Synthetic primitive regression; no printed-card or oracle admission.
//! Dispatcher -> masked branch -> optional pick -> tail. Closure mode only
//! disables the coexisting data frame to exercise the retained normal callback.
use digimon_engine::action::{build_action_mask,space::PASS};
use digimon_engine::debug_runner::{DebugRunner,make_test_card};
use digimon_engine::enums::{EffectTiming,GamePhase};
use digimon_engine::selection::{SelectionKind,TriggerSource};
#[derive(Clone,Copy,Debug)]enum Mode{Data,Snapshot,Closure}
const YAML:&str=r#"
card: TEST-REVEAL-CONTINUATION
name: Reveal Continuation Fixture
kind: digimon
level: 3
color: [purple]
cost: 3
dp: 1000
traits: [Fixture]
effects:
  - when: on_move
    process:
      - reveal_top_deck: { of: you, count: 2, bind_as: revealed }
      - select_effect_choice:
          labels: ["Continue", "Other"]
          bind_as: earlier
          prompt: "Bind earlier branch"
      - as_selecting_player:
          of: opponent
          body:
            - choose_from_reveal:
                of: you
                filter: { trait_has: Material }
                destination: { bottom_source_of: { target: this } }
                optional: true
                bind_as: material
                prompt: "You may place a Material"
            - if:
                condition:
                  all_of:
                    - event_permanent_is_source: true
                    - equals: [earlier, 0]
                then:
                  - gain_memory: 2
                else:
                  - gain_memory: 9
            - place_remainder_on_deck: { of: you, position: bottom }
"#;
fn masked(r:&mut DebugRunner,p:u8,a:u16){assert_eq!(build_action_mask(&r.game,p)[a as usize],1.0);assert_eq!(build_action_mask(&r.game,1-p)[a as usize],0.0);println!("MASK chooser={p} action={a} pending={:?}",r.pending_selection_view());r.game.decode_action(a,p);}
fn case(s:u8,mode:Mode,optional:bool,eligible:bool,decline:bool){
    let text=if optional{YAML.to_owned()}else{YAML.replace("optional: true","optional: false")};
    let mut mat=make_test_card("MAT","Material");mat.traits.push("Material".into());let f=make_test_card("FILL","Filler");
    let mut r=DebugRunner::builder().from_dsl_yaml(&text).unwrap().add_card(mat).add_card(f)
        .deck(s,&["FILL",if eligible{"MAT"}else{"FILL"}]).deck(1-s,&["FILL";2]).build();
    r.game.turn_player_idx=s as usize;r.game.current_phase=GamePhase::Main;r.game.set_memory(5);
    let h=r.place_on_field(s,"TEST-REVEAL-CONTINUATION",None);let card=r.top_card(h);
    r.game.enqueue_triggered(EffectTiming::OnMove,TriggerSource::MovedFromBreeding{player:s,permanent:h,card});assert_eq!(r.game.effect_queue.len(),1);r.game.drain_effect_queue();
    let v=r.pending_selection_view().unwrap();assert_eq!(v.kind,SelectionKind::EffectChoice);assert_eq!(v.selecting_player,s);assert_eq!(v.valid_action_ids.len(),2);masked(&mut r,s,v.valid_action_ids[0]);
    let chooser=1-s;
    if eligible{
        let v=r.pending_selection_view().unwrap();assert_eq!(v.kind,SelectionKind::Reveal);assert_eq!(v.selecting_player,chooser);assert_eq!(v.source_card,card);assert_eq!(v.source_permanent,Some(h));assert_eq!(v.valid_action_ids.len(),1);
        assert_eq!(build_action_mask(&r.game,chooser)[PASS as usize],if optional{1.0}else{0.0});
        match mode{Mode::Data=>assert!(r.game.pending_selection_resume.is_some()),Mode::Snapshot=>{assert!(r.game.pending_selection_resume.is_some());r.game=r.game.clone();},Mode::Closure=>{assert!(r.game.pending_selection_resume.take().is_some());}}
        masked(&mut r,chooser,if decline{PASS}else{v.valid_action_ids[0]});
    }
    let n=if eligible&&!decline{1}else{2};
    println!("AFTER mode={mode:?} seat={s} memory={} revealed={} deck={} pending={:?}",r.memory(),r.game.revealed_cards.len(),r.game.players[s as usize].deck.len(),r.pending_selection_view());
    assert_eq!(r.memory(),7,"DECLINE_TAIL_CONTEXT: continuation must preserve prior binding and OnMove source context, not skip or choose else");
    for remaining in(1..=n).rev(){let v=r.pending_selection_view().expect("DECLINE_TAIL_REMAINDER: optional pick cannot abort mandatory tail");assert_eq!(v.kind,SelectionKind::OrderedPermutation{remaining});assert_eq!(v.selecting_player,chooser,"selecting-player override must survive decline");assert_eq!(v.source_card,card);assert_eq!(v.source_permanent,Some(h));assert!(!v.is_optional);assert_eq!(build_action_mask(&r.game,chooser)[PASS as usize],0.0);masked(&mut r,chooser,*v.valid_action_ids.last().unwrap());}
    assert!(r.pending_selection().is_none());assert!(r.game.revealed_cards.is_empty());assert_eq!(r.game.players[s as usize].deck.len(),n as usize);assert_eq!(r.game.players[chooser as usize].deck.len(),2);
    let sources=&r.game.players[s as usize].battle_area[h.index as usize].card_sources;assert_eq!(sources.len(),if eligible&&!decline{2}else{1});if eligible&&!decline{assert_eq!(sources[0].card_id(&r.game.card_data),"MAT");}assert_eq!(r.top_card(h),card);
}
#[test]fn decline_data_s0(){case(0,Mode::Data,true,true,true)}
#[test]fn decline_data_s1(){case(1,Mode::Data,true,true,true)}
#[test]fn decline_snapshot_s0(){case(0,Mode::Snapshot,true,true,true)}
#[test]fn decline_snapshot_s1(){case(1,Mode::Snapshot,true,true,true)}
#[test]fn decline_closure_s0(){case(0,Mode::Closure,true,true,true)}
#[test]fn decline_closure_s1(){case(1,Mode::Closure,true,true,true)}
#[test]fn mandatory_data_s0(){case(0,Mode::Data,false,true,false)}
#[test]fn mandatory_data_s1(){case(1,Mode::Data,false,true,false)}
#[test]fn mandatory_snapshot_s0(){case(0,Mode::Snapshot,false,true,false)}
#[test]fn mandatory_snapshot_s1(){case(1,Mode::Snapshot,false,true,false)}
#[test]fn optional_success_s0(){case(0,Mode::Data,true,true,false)}
#[test]fn optional_success_s1(){case(1,Mode::Data,true,true,false)}
#[test]fn no_eligible_s0(){case(0,Mode::Data,true,false,false)}
#[test]fn no_eligible_s1(){case(1,Mode::Data,true,false,false)}
