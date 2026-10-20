//! Gazimon YAML regression: real public collector/drainer, no run_steps.
//! These are qualifying timing-event unit fixtures, not a claimed reachable
//! full card-game sequence of two natural moves/plays. Same object is retained.
use digimon_engine::action::{build_action_mask, space::PASS};
use digimon_engine::debug_runner::{DebugRunner, make_test_card};
use digimon_engine::enums::{EffectTiming, GamePhase};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const YAML: &str = include_str!("../../cards/bt25/BT25-078.yaml");

fn setup(seat: u8) -> (DebugRunner, PermanentHandle) {
    let mut a=make_test_card("MATCH-A","Match A");a.effect_text="Three Musketeers".into();a.traits.push("Three Musketeers".into());
    let mut b=make_test_card("MATCH-B","Match B");b.effect_text="Three Musketeers".into();b.traits.push("Three Musketeers".into());
    let fill=make_test_card("FILL","Filler");
    let mut r=DebugRunner::builder().from_dsl_yaml(YAML).unwrap().add_card(a).add_card(b).add_card(fill)
        .deck(seat,&["FILL","FILL","MATCH-B","FILL","FILL","MATCH-A"])
        .deck(1-seat,&["FILL";6]).build();
    r.game.turn_player_idx=seat as usize;r.game.current_phase=GamePhase::Main;r.game.set_memory(10);
    let host=r.place_on_field(seat,"BT25-078",None);
    (r,host)
}

fn queue(r: &mut DebugRunner, host: PermanentHandle, timing: EffectTiming) {
    let source=if timing==EffectTiming::OnMove {
        TriggerSource::MovedFromBreeding {player:host.player,permanent:host,card:r.top_card(host)}
    } else {TriggerSource::Permanent(host)};
    r.game.enqueue_triggered(timing,source);
    println!("QUEUED seat={} timing={:?} entries={}",host.player,timing,r.game.effect_queue.len());
    assert_eq!(r.game.effect_queue.len(),1,"qualifying source must actually collect its shared trigger");
    r.game.drain_effect_queue();
}

fn masked(r: &mut DebugRunner,seat: u8, action: u16) {
    let own=build_action_mask(&r.game,seat);let other=build_action_mask(&r.game,1-seat);
    assert_eq!(own[action as usize],1.0,"legal action required: {:?}",r.pending_selection_view());
    assert_eq!(other[action as usize],0.0,"opponent cannot select this prompt");
    println!("MASK seat={seat} action={action} pending={:?}",r.pending_selection_view());
    r.game.decode_action(action,seat);
}

fn finish(r: &mut DebugRunner,seat: u8,branch: usize) {
    // Preserve the currently scripted whole-effect optional gate; accept it.
    // Whether the printed effect's whole body is optional is a separate gap.
    if r.pending_selection_view().unwrap().kind==SelectionKind::Replacement {
        let v=r.pending_selection_view().unwrap();assert!(v.is_optional);assert_eq!(v.valid_action_ids.len(),1);
        masked(r,seat,v.valid_action_ids[0]);
    }
    let v=r.pending_selection_view().expect("shared effect body must offer both branches");
    assert_eq!(v.kind,SelectionKind::EffectChoice);assert_eq!(v.selecting_player,seat);assert_eq!(v.valid_action_ids.len(),2);
    masked(r,seat,v.valid_action_ids[branch]);
    let v=r.pending_selection_view().expect("eligible revealed-card selection required");
    assert_eq!(v.kind,SelectionKind::Reveal);assert_eq!(v.selecting_player,seat);assert_eq!(v.valid_action_ids.len(),1);
    masked(r,seat,v.valid_action_ids[0]);
    // The two remaining revealed cards have a real player-controlled order.
    // Resolve both prompts explicitly, including the singleton final pick.
    for remaining in [2u8,1u8] {
        let v=r.pending_selection_view().expect("remainder ordering must be surfaced");
        assert_eq!(v.kind,SelectionKind::OrderedPermutation {remaining});
        assert_eq!(v.selecting_player,seat);assert!(!v.is_optional);
        assert_eq!(v.valid_action_ids.len(),remaining as usize);
        assert_eq!(build_action_mask(&r.game,seat)[PASS as usize],0.0);
        let index=if remaining==2{1}else{0};masked(r,seat,v.valid_action_ids[index]);
    }
    assert!(r.pending_selection().is_none());assert!(r.game.effect_queue.is_empty());
}

fn repeated(seat: u8,first: EffectTiming,second: EffectTiming,branch: usize) {
        let (mut r,host)=setup(seat);let card=r.top_card(host);let turn=r.game.turn_count;
        let object=&r.game.players[seat as usize].battle_area[host.index as usize]as *const _ as usize;
        queue(&mut r,host,first);finish(&mut r,seat,branch);
        assert_eq!(r.top_card(host),card);assert_eq!(r.game.turn_count,turn);
        assert_eq!(&r.game.players[seat as usize].battle_area[host.index as usize]as *const _ as usize,object);
        assert_eq!(r.game.players[seat as usize].deck.len(),5);
        assert_eq!(r.game.players[seat as usize].hand.len(),usize::from(branch==0));
        assert_eq!(r.game.players[seat as usize].battle_area[host.index as usize].card_sources.len(),1+usize::from(branch==1));
        println!("FIRST_DONE seat={seat} card={card:?} turn={turn} activations={:?}",r.game.players[seat as usize].battle_area[host.index as usize].effect_activations);
        queue(&mut r,host,second);
        assert_eq!(r.top_card(host),card);assert_eq!(r.game.turn_count,turn);
        assert_eq!(&r.game.players[seat as usize].battle_area[host.index as usize]as *const _ as usize,object);
        assert!(r.pending_selection().is_some(),"UNPRINTED_OPT: same live object/current turn queued {:?} after {:?}, but second effect was suppressed",second,first);
        finish(&mut r,seat,branch);
        assert_eq!(r.game.players[seat as usize].deck.len(),4);
        assert_eq!(r.game.players[seat as usize].hand.len(),2*usize::from(branch==0));
        assert_eq!(r.game.players[seat as usize].battle_area[host.index as usize].card_sources.len(),1+2*usize::from(branch==1));
}

#[test]fn repeated_play_hand_s0(){repeated(0,EffectTiming::OnPlay,EffectTiming::OnPlay,0)}
#[test]fn repeated_play_hand_s1(){repeated(1,EffectTiming::OnPlay,EffectTiming::OnPlay,0)}
#[test]fn repeated_move_hand_s0(){repeated(0,EffectTiming::OnMove,EffectTiming::OnMove,0)}
#[test]fn repeated_move_hand_s1(){repeated(1,EffectTiming::OnMove,EffectTiming::OnMove,0)}
#[test]fn play_then_move_hand_s0(){repeated(0,EffectTiming::OnPlay,EffectTiming::OnMove,0)}
#[test]fn play_then_move_hand_s1(){repeated(1,EffectTiming::OnPlay,EffectTiming::OnMove,0)}
#[test]fn move_then_play_hand_s0(){repeated(0,EffectTiming::OnMove,EffectTiming::OnPlay,0)}
#[test]fn move_then_play_hand_s1(){repeated(1,EffectTiming::OnMove,EffectTiming::OnPlay,0)}
#[test]fn repeated_play_source_s0(){repeated(0,EffectTiming::OnPlay,EffectTiming::OnPlay,1)}
#[test]fn repeated_play_source_s1(){repeated(1,EffectTiming::OnPlay,EffectTiming::OnPlay,1)}
#[test]fn repeated_move_source_s0(){repeated(0,EffectTiming::OnMove,EffectTiming::OnMove,1)}
#[test]fn repeated_move_source_s1(){repeated(1,EffectTiming::OnMove,EffectTiming::OnMove,1)}
#[test]fn play_then_move_source_s0(){repeated(0,EffectTiming::OnPlay,EffectTiming::OnMove,1)}
#[test]fn play_then_move_source_s1(){repeated(1,EffectTiming::OnPlay,EffectTiming::OnMove,1)}
#[test]fn move_then_play_source_s0(){repeated(0,EffectTiming::OnMove,EffectTiming::OnPlay,1)}
#[test]fn move_then_play_source_s1(){repeated(1,EffectTiming::OnMove,EffectTiming::OnPlay,1)}

#[test]fn first_activation_hand_control(){for seat in[0,1]{let(mut r,h)=setup(seat);queue(&mut r,h,EffectTiming::OnPlay);finish(&mut r,seat,0);assert_eq!(r.game.players[seat as usize].hand.len(),1);}}
#[test]fn first_activation_source_control(){for seat in[0,1]{let(mut r,h)=setup(seat);queue(&mut r,h,EffectTiming::OnMove);finish(&mut r,seat,1);assert_eq!(r.game.players[seat as usize].battle_area[h.index as usize].card_sources.len(),2);}}
#[test]fn unrelated_move_does_not_spend_gazimon_control(){for seat in[0,1]{let(mut r,h)=setup(seat);let other=r.place_on_field(seat,"FILL",None);let card=r.top_card(h);r.game.enqueue_triggered(EffectTiming::OnMove,TriggerSource::MovedFromBreeding{player:seat,permanent:other,card:r.top_card(other)});r.game.drain_effect_queue();assert!(r.pending_selection().is_none());assert_eq!(r.game.players[seat as usize].deck.len(),6);assert!(r.game.players[seat as usize].battle_area[h.index as usize].effect_activations.is_empty());assert_eq!(r.top_card(h),card);queue(&mut r,h,EffectTiming::OnMove);finish(&mut r,seat,0);}}
