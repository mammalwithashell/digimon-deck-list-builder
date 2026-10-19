//! Reusable source-play entry orientation required by BT7-063 DarkKnightmon.
//! Official text: play named sources suspended before the would-delete cause resumes.
//! Baseline RED: SOURCE_ENTRY_SUSPENDED_BASELINE=1 omits ONLY the new unsupported
//! YAML field. This deliberately runs the existing unsuspended operation against
//! the same behavioral assertions; it is not a parser fallback or a green mode.
use std::sync::Arc;
use digimon_engine::action::{build_action_mask, space::{PASS, PLAY_HAND_START, REPLACEMENT_ACCEPT}};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{EffectTiming, Expiry, GamePhase, ModifierType};
use digimon_engine::modifiers::PlayerModifierEntry;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const SOURCE: &str = r#"
card: ENTRY-SOURCE
name: Entry Source
kind: digimon
level: 3
color: [black]
cost: 0
dp: 2000
effects:
  - when: on_play
    condition:
      any_permanent: { of: you, is_source: true, is_suspended: true }
    process:
      - draw: { of: you, count: 2 }
  - when: on_suspend
    condition: { event_target_is_source: true }
    process:
      - draw: { of: you, count: 13 }
"#;
const OBSERVER: &str = r#"
card: ENTRY-OBSERVER
name: Entry Observer
kind: digimon
level: 3
color: [black]
cost: 0
dp: 1000
effects:
  - when: on_ally_played
    condition:
      all_of:
        - event_card_name_is: Entry Source
        - any_permanent: { of: you, name_is: Entry Source, is_suspended: true }
    process:
      - draw: { of: you, count: 4 }
"#;
fn carrier(suspended: bool) -> String {
    let flag = if suspended && std::env::var_os("SOURCE_ENTRY_SUSPENDED_BASELINE").is_none() {
        "          suspended: true\n"
    } else { "" };
    format!(r#"
card: ENTRY-CARRIER
name: Entry Carrier
kind: digimon
level: 5
color: [black]
cost: 7
dp: 7000
effects:
  - when: on_play
    process:
      - select_material:
          of_permanent: source
          bind_as: picked
          optional: true
          filter: {{ name_is: Entry Source }}
          prompt: "Choose a source to play"
      - play_from_materials:
          target: source
          source_index: picked
          cost_delta: free
{flag}      - draw: {{ of: you, count: 1 }}
"#)
}
struct OptionalCancelSourcePlay;
impl CardEffect for OptionalCancelSourcePlay {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_permanent_would_play(card)
            .name("Optional source play refusal").optional()
            .replacement_condition(|ctx, subject| match subject {
                digimon_engine::replacement::ReplacementSubject::Card(h, _) => ctx.game
                    .card_data_for_handle(*h).is_some_and(|d| d.card_id == "ENTRY-SOURCE"),
                _ => false,
            })
            .replacement_process(|ctx| ctx.cancel()).build()]
    }
}
fn setup(actor: u8, suspended: bool, gate: bool) -> (DebugRunner, PermanentHandle, usize) {
    let mut r = DebugRunner::builder().from_dsl_yaml(&carrier(suspended)).unwrap()
        .from_dsl_yaml(SOURCE).unwrap().from_dsl_yaml(OBSERVER).unwrap()
        .add_card(make_test_card("ENTRY-FILL", "Filler"))
        .add_card(make_test_card("ENTRY-GATE", "Gate"))
        .deck(0, &["ENTRY-FILL"; 40]).deck(1, &["ENTRY-FILL"; 40])
        .security(0, &["ENTRY-FILL";5]).security(1, &["ENTRY-FILL";5])
        .memory(10).start();
    r.skip_mulligan(); r.set_first_player(actor); r.game.current_phase=GamePhase::Main;
    let host=r.place_stack(actor, &["ENTRY-SOURCE","ENTRY-CARRIER"]);
    r.place_on_field(actor,"ENTRY-OBSERVER",None);
    if gate {
        r.register_effect("ENTRY-GATE",Arc::new(OptionalCancelSourcePlay));
        r.place_on_field(actor,"ENTRY-GATE",None);
    }
    let hand=r.hand_size(actor);
    r.game.enqueue_triggered(EffectTiming::OnPlay,TriggerSource::Permanent(host));
    r.game.drain_effect_queue();
    (r,host,hand)
}
fn act(r: &mut DebugRunner, actor: u8, action: u16) {
    assert!(build_action_mask(&r.game,actor)[action as usize]>0.0,"choice must be legal");
    r.game.resolve_selection(actor,action).unwrap();
}
fn pick(r: &mut DebugRunner, actor: u8) {
    let v=r.pending_selection_view().expect("source choice");
    assert_eq!(v.selecting_player,actor);assert!(v.is_optional);
    assert_eq!(v.valid_action_ids.len(),1);
    act(r,actor,v.valid_action_ids[0]);
}
fn drain_orders(r: &mut DebugRunner) {
    for _ in 0..8 {
        let Some(v)=r.pending_selection_view() else {return;};
        assert_eq!(v.kind,SelectionKind::TriggerOrder,"only observer ordering may remain: {}",v.prompt);
        act(r,v.selecting_player,v.valid_action_ids[0]);
    }
    panic!("observer queue did not settle");
}
fn assert_entry(r: &DebugRunner, actor:u8, before:usize, suspended:bool) {
    let played=r.game.player(actor).battle_area.iter()
        .find(|p|p.top_card().card_id(&r.game.card_data)=="ENTRY-SOURCE").unwrap();
    assert_eq!(played.is_suspended,suspended,"entry orientation must be committed before observers");
    assert_eq!(r.hand_size(actor)-before,if suspended {7}else{1},
        "2 own OnPlay + 4 ally observer + 1 outer tail, and NO 13-card OnSuspend event");
}
#[test]
fn source_play_suspended_before_own_and_global_entry_observers_in_both_seats_and_clones() {
    for actor in [0,1] {
        let (mut r,host,before)=setup(actor,true,false);let snapshot=r.game.clone();
        for cloned in [false,true] {
            if cloned {r.game=snapshot.clone();}
            pick(&mut r,actor);drain_orders(&mut r);assert_entry(&r,actor,before,true);
            assert_eq!(r.game.player(actor).battle_area[host.index as usize].card_sources.len(),1);
        }
    }
}
#[test]
fn omitted_entry_flag_preserves_unsuspended_source_play() {
    let (mut r,_,before)=setup(0,false,false);pick(&mut r,0);drain_orders(&mut r);assert_entry(&r,0,before,false);
}
#[test]
fn declining_source_choice_preserves_source_and_does_not_leak_orientation() {
    let (mut r,host,before)=setup(0,true,false);let snapshot=r.game.clone();
    for cloned in [false,true] {
        if cloned {r.game=snapshot.clone();}
        act(&mut r,0,PASS);drain_orders(&mut r);
        assert_eq!(r.game.player(0).battle_area[host.index as usize].card_sources.len(),2);
        assert_eq!(r.hand_size(0)-before,0,"declining source pick skips its dependent continuation");
        r.add_to_hand(0,"ENTRY-SOURCE");let index=r.hand_size(0)-1;
        assert!(build_action_mask(&r.game,0)[PLAY_HAND_START as usize+index]>0.0);
        r.game.decode_action(PLAY_HAND_START+index as u16,0);drain_orders(&mut r);
        let p=r.game.player(0).battle_area.last().unwrap();assert!(!p.is_suspended);
        assert_eq!(r.hand_size(0)-before,0,"ordinary entry must not trigger suspended observer");
    }
}
#[test]
fn prohibited_source_play_restores_exact_stack_without_orientation_leak() {
    let (mut r,host,before)=setup(0,true,false);
    let sources=r.game.player(0).battle_area[host.index as usize].card_sources.iter().map(|c|c.handle()).collect::<Vec<_>>();
    r.game.modifiers.add_player_modifier(0,PlayerModifierEntry::simple(
        ModifierType::CannotPlayDigimonByEffect,0,Expiry::Permanent,None,0));
    pick(&mut r,0);drain_orders(&mut r);
    assert_eq!(r.game.player(0).battle_area[host.index as usize].card_sources.iter().map(|c|c.handle()).collect::<Vec<_>>(),sources);
    assert_eq!(r.hand_size(0)-before,1);
    r.add_to_hand(0,"ENTRY-SOURCE");let index=r.hand_size(0)-1;
    assert!(build_action_mask(&r.game,0)[index]>0.0,"effect lock does not forbid native play");
    r.game.decode_action(index as u16,0);drain_orders(&mut r);
    assert!(!r.game.player(0).battle_area.last().unwrap().is_suspended);
    assert_eq!(r.hand_size(0)-before,1);
}
#[test]
fn declined_would_play_refusal_keeps_suspended_entry_after_park_and_clone() {
    let (mut r,_,before)=setup(0,true,true);pick(&mut r,0);
    assert_eq!(r.pending_selection_view().unwrap().kind,SelectionKind::Replacement);
    let snapshot=r.game.clone();
    for cloned in [false,true] {
        if cloned {r.game=snapshot.clone();}
        act(&mut r,0,PASS);drain_orders(&mut r);assert_entry(&r,0,before,true);
    }
}
#[test]
fn accepted_would_play_refusal_restores_source_in_cloned_branch() {
    let (mut r,host,before)=setup(0,true,true);
    let sources=r.game.player(0).battle_area[host.index as usize].card_sources.iter().map(|c|c.handle()).collect::<Vec<_>>();
    pick(&mut r,0);let snapshot=r.game.clone();
    for cloned in [false,true] {
        if cloned {r.game=snapshot.clone();}
        act(&mut r,0,REPLACEMENT_ACCEPT);drain_orders(&mut r);
        assert_eq!(r.game.player(0).battle_area[host.index as usize].card_sources.iter().map(|c|c.handle()).collect::<Vec<_>>(),sources);
        assert_eq!(r.battle_area_size(0),3);
        assert_eq!(r.hand_size(0)-before,1,"outer tail resumes exactly once on refused play");
    }
}
