//! Synthetic two-route declarations on real starter-card multisets. Only the
//! known host/result metadata is changed to isolate the continuation contract.
use crate::{card_data::{CardData,EvoCost}, card_source::{CardHandle,CardSource},
    determinize::{beliefs::*,closed::materialize_closed}, effect_context::EffectContext,
    enums::{CardColor,CardKind,CostDelta,GamePhase,PlaySource,Zone},
    permanent::{Permanent,PermanentHandle}, resume::{OuterContinuation,ResumeFrame},
    dsl_cards::{bindings::Bindings,step::StepRuntime},
    trigger_context::{DeletedObjectSnapshot,EventCause,EventSubject,MovedCardSet,TriggerContext},
    action::space::HAND_EFFECT_START, Game, rules::Rules};
use std::{collections::{BTreeSet,HashSet},sync::Arc,panic::{catch_unwind,AssertUnwindSafe}};
use digimon_dsl::compiled::{CompiledStep,CompiledFormula};

const D1: &[&str] = &[
    "ST1-01", "ST1-01", "ST1-01", "ST1-01", "ST1-02", "ST1-02", "ST1-02", "ST1-02", "ST1-03",
    "ST1-03", "ST1-03", "ST1-03", "ST1-04", "ST1-04", "ST1-04", "ST1-04", "ST1-05", "ST1-05",
    "ST1-05", "ST1-05", "ST1-06", "ST1-06", "ST1-06", "ST1-06", "ST1-07", "ST1-07", "ST1-07",
    "ST1-07", "ST1-08", "ST1-08", "ST1-09", "ST1-09", "ST1-09", "ST1-09", "ST1-10", "ST1-10",
    "ST1-10", "ST1-10", "ST1-11", "ST1-11", "ST1-12", "ST1-12", "ST1-13", "ST1-13", "ST1-14",
    "ST1-14", "ST1-14", "ST1-14", "ST1-15", "ST1-15", "ST1-16", "ST1-16", "ST1-16", "ST1-16",
];

fn take(g:&mut Game, seat:u8,id:&str)->CardSource {
    let data=&g.card_data; let p=&mut g.players[seat as usize];
    for zone in [&mut p.hand,&mut p.deck,&mut p.security] {
        if let Some(i)=zone.iter().position(|c|c.card_id(data)==id) { return zone.remove(i); }
    }
    panic!("fixture card missing {id}")
}
fn fixture(seat:u8,known:bool)->(Game,CardHandle,WeightedDeckPrior) {
    let mut data=CardData::load_from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"),"/../../data/cards.json")).unwrap()).unwrap();
    let host=data.get_mut("ST1-04").unwrap();host.level=Some(4);host.colors=vec![CardColor::Red];
    let result=data.get_mut("ST1-07").unwrap();result.level=Some(5);result.colors=vec![CardColor::Red];
    result.evo_costs=[3,4].into_iter().map(|memory_cost|EvoCost{card_color:0,level:4,memory_cost}).collect();
    let cards:Vec<String>=D1.iter().map(|x|x.to_string()).collect();
    let prior=WeightedDeckPrior::new(PriorSnapshot::new("standard","cost-choice-reference-fixture",cards.iter().cloned().collect(),vec![CandidateDeck{cards:cards.clone(),weight:1.0}],0.1,128).unwrap()).unwrap();
    let mut g=Game::new(&[cards.clone(),cards],&data,Rules::standard(),Some(92401)).unwrap();
    g.enable_belief_knowledge().unwrap();
    g.start_game();g.mulligan_pending.clear();g.current_phase=GamePhase::Main;
    g.turn_order=vec![seat,1-seat];g.turn_player_idx=0;g.turn_count=1;g.memory=4;
    let host=take(&mut g,seat,"ST1-04");let source=host.handle();
    g.players[seat as usize].battle_area.push(Permanent::new(host,0));
    let result=take(&mut g,seat,"ST1-07");g.players[seat as usize].hand.push(result);
    g.enable_belief_knowledge().unwrap();
    let event=g.players[seat as usize].security.last().unwrap().handle();
    if known { g.reveal_security(seat); }
    assert!(EffectContext::new(&mut g,source,None,seat).return_security_card_to_deck(seat,event,true));
    g.effect_queue.clear();g.drain_events();
    let mut context=TriggerContext::default();context.event_card=Some(event).into();
    context.subject=Some(EventSubject::Card{card:event,zone:Zone::Security}).into();
    context.moved_card_sets.push(MovedCardSet{cards:vec![event],from:Some(Zone::Security),to:Some(Zone::Deck)});
    g.current_trigger_context=Some(context.clone());
    let hand=g.players[seat as usize].hand.len()-1;
    assert!(!g.effect_initiated_digivolve(seat,hand,PermanentHandle{player:seat,index:0},CostDelta::Reduce(0),false,PlaySource::ByEffect));
    assert!(matches!(g.pending_selection_resume.as_ref().unwrap().frames.last().unwrap(),ResumeFrame::EffectDigivolveCostChoice(_)));
    assert_eq!(g.pending_selection.as_ref().unwrap().valid_action_ids,vec![HAND_EFFECT_START,HAND_EFFECT_START+1]);
    let mut bindings=Bindings::new();bindings.insert_card_list("retained_position",vec![event]);
    state(&mut g).outer_conts.push(OuterContinuation{tail:Arc::new(vec![CompiledStep::GainMemoryFn{formula:CompiledFormula::EventTargetLevel}]),bindings,runtime:StepRuntime::default(),trigger_context:Some(context),source_card:source,source_permanent:Some(PermanentHandle{player:seat,index:0}),player:seat});
    g.current_trigger_context=None;
    (g,event,prior)
}
fn state(g:&mut Game)->&mut crate::game_actions::digivolve::EffectDigivolveCostChoiceState {
    let ResumeFrame::EffectDigivolveCostChoice(s)=g.pending_selection_resume.as_mut().unwrap().frames.last_mut().unwrap() else{panic!("cost choice")};s
}
fn zones(g:&Game)->Vec<Vec<String>> {
    g.players.iter().flat_map(|p|[&p.hand,&p.deck,&p.security,&p.digitama_deck,&p.trash]).map(|z|z.iter().map(|c|c.card_id(&g.card_data).to_string()).collect()).collect()
}
fn run(known:bool) {
    for seat in [0,1] { for cloned in [false,true] {
        let (g,event,prior)=fixture(seat,known); let root=if cloned{g.clone()}else{g};
        let memory=root.belief_knowledge(seat).unwrap().clone();
        assert_eq!(memory.hidden_instances(seat).iter().any(|x|x.0==event.0),known);
        assert_eq!(memory.concealed_positions().iter().filter(|x|x.instance==event.0).map(|x|(x.owner,x.zone,x.position)).collect::<Vec<_>>(),vec![(seat,KnowledgeZone::Deck,Some(root.players[seat as usize].deck.len()-1))]);
        let mut alternative=root.clone();
        if !known {
            let p=&mut alternative.players[seat as usize];let i=p.deck.iter().position(|c|c.handle()==event).unwrap();
            std::mem::swap(&mut p.deck[i].data_index,&mut p.security[0].data_index);
        }
        let mut sampled=BTreeSet::new();
        for seed in 0..8 { for route in 0..2 {
            let mut a=materialize_closed(&root,seat,&memory,&prior,seed).expect("public position is an entitled reference, not a cached identity");
            let b=materialize_closed(&alternative,seat,&memory,&prior,seed).unwrap();
            assert_eq!(zones(&a),zones(&b));
            assert_eq!(crate::build_action_mask(&a,seat),crate::build_action_mask(&root,seat));
            let level=a.card_data_for_handle(event).unwrap().level.unwrap_or(0) as i16;
            sampled.insert(a.card_data_for_handle(event).unwrap().card_id.clone());
            assert_eq!(state(&mut a).trigger_context.as_ref().unwrap().event_card.read(),Some(event));
            let mut b=b.clone();
            for world in [&mut a,&mut b] {
                assert!(crate::build_action_mask(world,seat)[(HAND_EFFECT_START+route) as usize]>0.5);
                world.decode_action(HAND_EFFECT_START+route,seat);
                assert_eq!(world.memory,4-(3+route as i16)+level);
                assert_eq!(world.players[seat as usize].battle_area[0].top_card().card_id(&world.card_data),"ST1-07");
                assert!(world.pending_selection.is_none());
            }
            assert_eq!(zones(&a),zones(&b));
        } }
        if !known { assert!(sampled.len()>1,"a public position must not reveal its true identity"); }
        assert_eq!(root.belief_knowledge(seat).unwrap(),&memory);
    } }
}
#[test] fn public_position_cost_choice_samples_identity_and_resolves_cloned_routes(){run(false)}
#[test] fn known_identity_cost_choice_preserves_ordinary_routes(){run(true)}
fn snapshot(event:CardHandle,seat:u8)->DeletedObjectSnapshot {
    DeletedObjectSnapshot{former_controller:seat,top_card:event,card_kind:CardKind::Digimon,retained_kind:None,traits:vec![],level:Some(4),dp:Some(4000),cause:EventCause::OwnEffect,dp_just_before:Some(4000),level_just_before:Some(4),cost_just_before:Some(4),names_just_before:vec!["private cached name".into()],traits_just_before:vec![],source_count_just_before:0,digisources_just_before:vec![],is_token:false}
}
#[test] fn cost_choice_keeps_identity_snapshots_and_untracked_references_strict() {
    for seat in [0,1] { for variant in 0..11 {
        let (mut root,event,prior)=fixture(seat,false);let memory=root.belief_knowledge(seat).unwrap().clone();
        let other=root.players[seat as usize].deck.iter().find(|c|c.handle()!=event).unwrap().handle();
        let s=state(&mut root);s.trigger_context=None;s.outer_conts[0].trigger_context=None;s.outer_conts[0].bindings=Bindings::new();
        match variant {
            0=>s.result_card=event,1=>s.target_top=event,2=>s.source_carrier_top=Some(event),
            3=>s.result_clause=Some((event,seat)),4=>s.outer_conts[0].source_card=event,
            5|6|7=>{let mut c=TriggerContext::default();if variant==5{c.deleted_object=Some(snapshot(event,seat));}else if variant==6{c.event_snapshot=Some(Arc::new(snapshot(event,seat)));}else{c.event_card=Some(other).into();}s.trigger_context=Some(c);},
            8=>s.outer_conts[0].bindings.insert_card_list("unknown",vec![other]),
            9=>s.acting_player=1-seat,
            10=>{let mut c=TriggerContext::default();c.deleted_object=Some(snapshot(event,seat));s.outer_conts[0].trigger_context=Some(c);},_=>unreachable!()
        }
        assert!(materialize_closed(&root,seat,&memory,&prior,2).is_err(),"variant {variant}");
    } }
}
#[test] fn cost_choice_checked_context_requires_world_fence_and_retains_read_failure() {
    for seat in [0,1] { for outer in [false,true] { for fenced in [false,true] {
        let (mut root,event,prior)=fixture(seat,false);let memory=root.belief_knowledge(seat).unwrap().clone();
        let s=state(&mut root);s.trigger_context=None;s.outer_conts[0].trigger_context=None;s.outer_conts[0].bindings=Bindings::new();
        let mut c=TriggerContext::default();c.event_card=Some(event).into();assert!(c.event_card.project_if_unentitled(&HashSet::new()));
        if outer{s.outer_conts[0].trigger_context=Some(c)}else{s.trigger_context=Some(c)}
        if fenced{root.require_event_card_read_fence_for_test();}
        let world=materialize_closed(&root,seat,&memory,&prior,2);
        if fenced {let mut world=world.unwrap();let s=state(&mut world);let c=if outer{s.outer_conts[0].trigger_context.as_ref().unwrap()}else{s.trigger_context.as_ref().unwrap()};assert!(catch_unwind(AssertUnwindSafe(||c.event_card.read())).is_err());}
        else{assert!(world.is_err(),"checked value cannot be admitted without its world fence");}
    } } }
}
