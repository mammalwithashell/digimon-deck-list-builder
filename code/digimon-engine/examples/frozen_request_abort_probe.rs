//! Model-free executable fixture for the shared two-game request boundary.
//! Never accepts a scored request or loads model weights. The explicit reference
//! cases obtain a checked value from actual closed materialization of a separate
//! synthetic starter-deck fixture, then read it after earlier pair-loop work.
use digimon_engine::{
    card_registry::CardRegistry, debug_runner::{DebugRunner, make_test_card},
    game::Game, rules::Rules, permanent::Permanent, enums::GamePhase,
    search::{EvalRequest, Evaluation, PolicyValueFn, UniformEvaluator},
};
use serde_json::{json, Value};
use std::{cell::RefCell, rc::Rc, path::{Path,PathBuf}, io::Write};
#[path="search_quality/frozen_modes.rs"] mod modes;
#[path="search_quality/telemetry.rs"] mod telemetry;
#[path="search_quality/request_driver.rs"] mod request_driver;
#[path="../tests/determinization/support.rs"] #[allow(dead_code)] mod reference_fixture;
fn event_reference(project: bool) -> digimon_engine::trigger_context::EventCardReference {
    use digimon_engine::determinize::{beliefs::{PriorSnapshot, WeightedDeckPrior, CandidateDeck}, closed::materialize_closed};
    let mut root = reference_fixture::new_game_with(77110, reference_fixture::D1, reference_fixture::D1);
    root.enable_belief_knowledge().unwrap();
    let card = root.players[usize::from(project)].hand[0].handle();
    let mut context = digimon_engine::trigger_context::TriggerContext::default();
    context.event_card = Some(card).into(); root.current_trigger_context = Some(context);
    let deck = reference_fixture::owned(reference_fixture::D1);
    let prior = WeightedDeckPrior::new(PriorSnapshot::new("standard", "native-reference-abort-fixture",
        deck.iter().cloned().collect(), vec![CandidateDeck{cards:deck,weight:1.0}], 0.1, 128).unwrap()).unwrap();
    let world = materialize_closed(&root, 0, root.belief_knowledge(0).unwrap(), &prior, 73).unwrap();
    world.current_trigger_context.unwrap().event_card
}
const SCHEMA: &str = "native-pair-abort-fixture-v1";

fn trace(path:&Path, value:Value) {
    let mut file=std::fs::OpenOptions::new().create(true).append(true).open(path).unwrap();
    writeln!(file,"{}",value).unwrap();
    file.flush().unwrap();
}
struct State { game:u8, calls:usize, failure:String }
struct TracedUniform { state:Rc<RefCell<State>>, trace:PathBuf, reference:Option<digimon_engine::trigger_context::EventCardReference> }
impl PolicyValueFn for TracedUniform {
    fn evaluate_batch(&mut self, requests:&[EvalRequest])->Vec<Evaluation> {
        let (game,call,fail)={
            let mut state=self.state.borrow_mut();state.calls+=1;
            let fail=state.calls==2 && ((state.failure=="first-panic" && state.game==0)
                || (state.failure=="second-panic" && state.game==1));
            (state.game,state.calls,fail)
        };
        trace(&self.trace,json!({"event":"eval_begin","game":game,"call":call,"rows":requests.len()}));
        if fail {panic!("native pair fixture evaluator failure: game={game}, call={call}");}
        let reference_read = call==2 && {
            let state=self.state.borrow();
            state.failure=="reference-control" || (state.failure=="first-reference" && game==0)
                || (state.failure=="second-reference" && game==1)
        };
        if reference_read {
            trace(&self.trace,json!({"event":"reference_read_begin","game":game,"call":call}));
            let value=self.reference.unwrap().read();
            assert!(value.is_some());
            trace(&self.trace,json!({"event":"reference_read_complete","game":game,"call":call}));
        }
        let result=UniformEvaluator.evaluate_batch(requests);
        trace(&self.trace,json!({"event":"eval_complete","game":game,"call":call,"rows":result.len()}));
        result
    }
}
fn fixture(actor:u8, empty:bool)->Game {
    let mut attack=make_test_card("BOUND-ATK","Fixture attacker");attack.dp=Some(5000);
    let defend=make_test_card("BOUND-DEF","Fixture defender");
    let data=[(attack.card_id.clone(),attack),(defend.card_id.clone(),defend)]
        .into_iter().collect();
    let mut deck=vec!["BOUND-DEF".to_owned();20];deck.push("BOUND-ATK".into());
    let mut rules=Rules::standard();rules.security_count=0; // Synthetic immediate-lethal setup only.
    let mut game=Game::new(&[deck.clone(),deck],&data,rules,Some(101)).unwrap();
    // Stage only physical cards from the initial deal, before any player action
    // or knowledge initialization. This is a synthetic position, not an admitted deck.
    let store=game.card_data.clone();
    for pid in 0..2u8 {
        let player=&mut game.players[pid as usize];
        let mut cards=std::mem::take(&mut player.hand);
        cards.append(&mut player.deck);cards.append(&mut player.security);
        if !empty {
            let wanted=if pid==actor{"BOUND-ATK"}else{"BOUND-DEF"};
            let index=cards.iter().position(|c|c.card_id(&store)==wanted).unwrap();
            player.battle_area.push(Permanent::new(cards.remove(index),0));
        }
        player.deck=cards;
    }
    game.enable_belief_knowledge().unwrap();
    let mut runner=DebugRunner::wrap(game);
    runner.skip_mulligan();runner.set_first_player(actor);runner.game.set_memory(3);
    runner.set_phase(GamePhase::Main);
    assert!(runner.game.players.iter().all(|p|p.security.is_empty()));
    if !empty {
        let attack=digimon_engine::action::space::encode_attack(0,digimon_engine::action::space::SECURITY_TARGET);
        assert!(digimon_engine::action::build_action_mask(&runner.game,actor)[attack as usize]>0.5,
            "fixture requires a legal direct attack");
    }
    runner.game
}
fn run() {
    let args:Vec<_>=std::env::args().skip(1).collect();assert_eq!(args.len(),2);
    let request:Value=serde_json::from_slice(&std::fs::read(&args[0]).unwrap()).unwrap();
    assert_eq!(request["schema"],SCHEMA,"diagnostic fixture only");
    let actor=request["actor"].as_u64().unwrap() as u8;assert!(actor<2);
    let failure=request["failure"].as_str().unwrap().to_owned();
    assert!(["control","first-panic","second-panic","second-incomplete","first-reference","second-reference","reference-control"].contains(&failure.as_str()));
    let out=Path::new(&args[1]);assert!(!out.exists());
    let trace_path=out.with_extension("trace.jsonl");assert!(!trace_path.exists());
    let sample=fixture(actor,false);
    let data=sample.card_data.iter().map(|c|(c.card_id.clone(),c.clone())).collect();
    let registry=CardRegistry::from_cards(&data);
    let state=Rc::new(RefCell::new(State{game:0,calls:0,failure:failure.clone()}));
    let reference=if failure.contains("reference") {Some(event_reference(failure!="reference-control"))} else {None};
    let mut candidate=TracedUniform{state:state.clone(),trace:trace_path.clone(),reference};
    let mut opponent=TracedUniform{state:state.clone(),trace:trace_path.clone(),reference};
    let plan=json!({"model_sha256":"uniform-fixture-not-a-model",
        "opponent_model_sha256":"uniform-fixture-not-a-model"});
    request_driver::run_pair(out,&request,&plan,"diagnostic-fixture-runtime",
        None,"diagnostic-no-prior",&registry,&mut candidate,&mut opponent,
        modes::InformationRegime::KnownList,modes::InformationRegime::KnownList,
        if failure=="second-incomplete"{1}else{100},SCHEMA,|game|{
            state.borrow_mut().game=game;state.borrow_mut().calls=0;
            trace(&trace_path,json!({"event":"game_start","game":game,"actor":actor}));
            fixture(actor,failure=="second-incomplete" && game==1)
        });
    trace(&trace_path,json!({"event":"pair_returned"}));
}
fn main() {
    std::thread::Builder::new().stack_size(268435456).spawn(run).unwrap().join().unwrap();
}
