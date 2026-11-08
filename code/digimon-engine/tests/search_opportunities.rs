#[path="search/support.rs"] mod support;
#[path="search/paging.rs"] mod paging;
use digimon_engine::{action::{build_action_mask, encode_digivolve}, card_registry::CardRegistry,
    debug_runner::DebugRunner, enums::GamePhase,
    search::{opportunity::{describe, record_execution}, Mcts, SearchConfig, SearchResult, UniformEvaluator}};

const TAMER: &str = "card: TELE-TAMER\nname: Telemetry Tamer\nkind: tamer\ncolor: [red]\ncost: 3\n";
const HYBRID: &str = "card: TELE-HYBRID\nname: Telemetry Hybrid\nkind: digimon\nlevel: 4\ncolor: [red]\ncost: 5\ndp: 5000\ntraits: [Hybrid]\nalt_paths:\n  - kind: digivolve\n    from: {kind: tamer, color_is: red}\n    cost: 2\n";

fn result(plans: Vec<(Vec<u16>,u32,f32)>) -> SearchResult {
    SearchResult {root_visits:plans.iter().map(|(r,n,_)|(r[0],*n)).collect(),
        root_q:plans.iter().map(|(r,_,q)|(r[0],*q)).collect(),simulations_run:plans.iter().map(|p|p.1).sum(),
        root_plans:plans,root_priors:vec![],root_value:0.}
}

#[test]
fn tamer_opportunity_records_actual_choice_and_preserves_source() { support::run_big_stack(|| {
    let mut runner=DebugRunner::builder().from_dsl_yaml(TAMER).unwrap().from_dsl_yaml(HYBRID).unwrap()
        .hand(0,&["TELE-HYBRID"]).deck(0,&["TELE-HYBRID";12]).deck(1,&["TELE-HYBRID";12]).memory(5).start();
    runner.skip_mulligan(); runner.place_on_field(0,"TELE-TAMER",None); runner.game.current_phase=GamePhase::Main;
    let evo=encode_digivolve(0,0); let play=0;
    let mask=build_action_mask(&runner.game,0);
    assert_eq!(mask[evo as usize],1.); assert_eq!(mask[play as usize],1.);
    let mut result=result(vec![(vec![evo],9,0.7),(vec![play],1,0.2)]);
    result.root_priors=vec![(vec![evo],0.1,0.3),(vec![play],0.9,0.7)];
    let before=runner.game.verification_digest();
    let mut row=describe(&runner.game,0,&result,&[play],"visit_sample_tau1").unwrap();
    assert_eq!(row["plans"][0]["category"],"tamer_digivolve");
    assert_eq!(row["plans"][1]["category"],"hard_play");
    assert_eq!(row["plans"][1]["selected"],true);
    assert_eq!(row["plans"][1]["selection_probability"],0.1);
    assert_eq!(row["commitment_executed"],false);
    record_execution(&mut row,&[]).unwrap(); assert_eq!(row["commitment_executed"],false);
    assert!(record_execution(&mut row,&[evo]).is_err());
    record_execution(&mut row,&[play]).unwrap(); assert_eq!(row["commitment_executed"],true);
    assert_eq!(before,runner.game.verification_digest());
}); }

#[test]
fn priors_survive_serial_batch_and_world_aggregation() { support::run_big_stack(|| {
    use digimon_engine::{game::Game,rules::Rules,search::{determinized_search_batched,DeterminizedConfig}};
    let cards=support::load_card_data(); let registry=CardRegistry::from_cards(&cards);
    let decks=vec![support::ST1_DECK.iter().map(|s|s.to_string()).collect(),support::ST5_DECK.iter().map(|s|s.to_string()).collect()];
    let root=Game::new(&decks,&cards,Rules::standard(),Some(123)).unwrap();
    let config=SearchConfig{simulations:16,dirichlet_epsilon:0.25,seed:51,..Default::default()};
    let mcts=Mcts::new(&registry);
    let serial=mcts.search(&root,&mut UniformEvaluator,&config);
    let batch=mcts.search_batch(&[root.clone()],&mut UniformEvaluator,&[config.clone()]);
    assert_eq!(serial,batch[0]);
    assert_eq!(serial.root_priors.len(),serial.root_plans.len());
    assert!(serial.root_priors.iter().any(|(_,a,b)|a!=b));
    for column in [1,2] { let sum:f32=serial.root_priors.iter().map(|p|if column==1 {p.1}else{p.2}).sum(); assert!((sum-1.).abs()<1e-5); }
    let cfg=DeterminizedConfig{search:config,worlds:4,..Default::default()};
    let combined=determinized_search_batched(&registry,&root,root.mulligan_current_player().unwrap(),&mut UniformEvaluator,&cfg,4);
    assert_eq!(combined.root_priors.len(),combined.root_plans.len());
    assert!((combined.root_priors.iter().map(|p|p.1).sum::<f32>()-1.).abs()<1e-5);
    let digest=root.verification_digest();
    let selected=combined.best_plan().unwrap();
    let row=describe(&root,root.mulligan_current_player().unwrap(),&combined,selected,"argmax_tau0").unwrap();
    assert_eq!(row["plans"].as_array().unwrap().iter().filter(|p|p["selection_probability"]==1.).count(),1);
    assert_eq!(digest,root.verification_digest());
}); }

#[test]
fn zero_visits_and_missing_priors_are_not_fabricated() { support::run_big_stack(|| {
    use digimon_engine::{game::Game,rules::Rules};
    let cards=support::load_card_data();
    let decks=vec![support::ST1_DECK.iter().map(|s|s.to_string()).collect(),support::ST5_DECK.iter().map(|s|s.to_string()).collect()];
    let root=Game::new(&decks,&cards,Rules::standard(),Some(123)).unwrap();
    let actor=root.mulligan_current_player().unwrap();
    let result=result(vec![(vec![0],0,0.),(vec![1],0,0.)]);
    let row=describe(&root,actor,&result,&[1],"uniform_zero_visits").unwrap();
    assert_eq!(row["plans"][0]["selection_probability"],0.5);
    assert!(row["plans"][0]["prior_before_noise"].is_null());
    assert!(row["plans"][0]["q"].is_null());
    assert!(describe(&root,actor,&result,&[1],"visit_sample_tau1").is_err());
}); }

#[test]
fn selfplay_telemetry_preserves_shard_bytes_and_temperature_schedule() { support::run_big_stack(|| {
    use digimon_engine::selfplay::{run_self_play,SelfPlayConfig};
    use std::{fs,path::PathBuf};
    let cards=support::load_card_data();
    let decks=vec![support::ST1_DECK.iter().map(|s|s.to_string()).collect(),support::ST5_DECK.iter().map(|s|s.to_string()).collect()];
    let names=vec!["ST1".into(),"ST5".into()];
    let mut config=SelfPlayConfig{games:1,decision_cap:24,temperature_decisions:2,full_search_fraction:0.5,cheap_simulations:4,..Default::default()};
    config.search.worlds=2;config.search.search.simulations=8;config.search.search.max_decisions=3;
    let base=std::env::temp_dir().join(format!("opportunity-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    let previous=std::env::var_os("DIGIMON_SEARCH_OPPORTUNITY_TELEMETRY");
    struct Restore(Option<std::ffi::OsString>); impl Drop for Restore {fn drop(&mut self){match &self.0 {Some(v)=>std::env::set_var("DIGIMON_SEARCH_OPPORTUNITY_TELEMETRY",v),None=>std::env::remove_var("DIGIMON_SEARCH_OPPORTUNITY_TELEMETRY")}}}
    let _restore=Restore(previous);
    std::env::remove_var("DIGIMON_SEARCH_OPPORTUNITY_TELEMETRY");
    run_self_play(&cards,&decks,&names,&mut UniformEvaluator,&config,&base.join("off"),"uniform-test").unwrap();
    std::env::set_var("DIGIMON_SEARCH_OPPORTUNITY_TELEMETRY","1");
    run_self_play(&cards,&decks,&names,&mut UniformEvaluator,&config,&base.join("on"),"uniform-test").unwrap();
    fn arrays(dir:&std::path::Path,relative:PathBuf,out:&mut Vec<(PathBuf,Vec<u8>)>){for entry in fs::read_dir(dir.join(&relative)).unwrap(){let e=entry.unwrap();let p=relative.join(e.file_name());if e.file_type().unwrap().is_dir(){arrays(dir,p,out)}else if p.extension().is_some_and(|s|s=="npy"){out.push((p,fs::read(e.path()).unwrap()));}}out.sort_by(|a,b|a.0.cmp(&b.0));}
    let mut off=vec![];let mut on=vec![];arrays(&base.join("off"),PathBuf::new(),&mut off);arrays(&base.join("on"),PathBuf::new(),&mut on);
    assert!(!off.is_empty());assert_eq!(off,on,"telemetry changed training arrays");
    let records:Vec<serde_json::Value>=fs::read_to_string(base.join("on/search_opportunities.jsonl")).unwrap().lines().map(|s|serde_json::from_str(s).unwrap()).collect();
    assert!(records.iter().any(|r|r["sampling"]=="visit_sample_tau1"));
    assert!(records.iter().any(|r|r["sampling"]=="argmax_tau0"));
    assert!(records.iter().any(|r|r["context"]["full_search"]==false));
    for r in records {assert_eq!(r["commitment_executed"],true); if r["context"]["full_search"]==false {assert_eq!(r["context"]["dirichlet_epsilon"],0.0);}}
}); }
