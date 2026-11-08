use digimon_engine::{action::{build_action_mask,encode_digivolve},
    debug_runner::DebugRunner,enums::GamePhase,
    policies::{greedy_action,DevelopmentPolicy,tamer_first::{greedy_tamer_first_action,legal_tamer_digivolutions}}};

fn fixture() -> DebugRunner {
    let mut r=DebugRunner::builder()
        .from_dsl_yaml("card: TF-TAMER\nname: Tamer Base\nkind: tamer\ncolor: [red]\ncost: 3\n").unwrap()
        .from_dsl_yaml("card: TF-HYBRID\nname: Hybrid\nkind: digimon\nlevel: 4\ncolor: [red]\ncost: 5\ndp: 5000\ntraits: [Hybrid]\nalt_paths:\n  - kind: digivolve\n    from: {kind: tamer, color_is: red}\n    cost: 2\n").unwrap()
        .hand(0,&["TF-HYBRID","TF-HYBRID"])
        .deck(0,&["TF-HYBRID";12]).deck(1,&["TF-HYBRID";12]).memory(1).start();
    r.skip_mulligan();r.place_on_field(0,"TF-TAMER",None);r.place_on_field(0,"TF-TAMER",None);
    r.game.current_phase=GamePhase::Main;r.game.memory=1;r
}

fn run(f:impl FnOnce()+Send+'static){std::thread::Builder::new().stack_size(268435456).spawn(f).unwrap().join().unwrap();}

#[test]
fn both_variants_choose_only_tamer_routes_even_when_passing_memory(){run(||{
    let r=fixture();let mask=build_action_mask(&r.game,0);let before=r.game.verification_digest();
    let eligible=legal_tamer_digivolutions(&r.game,&mask);
    assert!(eligible.contains(&encode_digivolve(0,0)));assert!(eligible.contains(&encode_digivolve(1,1)));
    let greedy=greedy_tamer_first_action(&r.game,&mask);assert_eq!(greedy,*eligible.iter().min().unwrap());
    let mut dev=DevelopmentPolicy::default();let chosen=dev.decide_tamer_first(&r.game,&mask).unwrap();
    assert!(eligible.contains(&chosen.action));assert_eq!(chosen.version,"development_tamer_first_v1");
    assert_eq!(before,r.game.verification_digest());
    let mut after=r.game.clone();after.decode_action(chosen.action,0);
    assert!(after.players[0].battle_area.iter().any(|p|after.card_data[p.top_card().data_index].card_id=="TF-HYBRID"));
    // The engine may already rotate the seesaw to the next player.
    let relative=if after.memory_pair.0==0 {after.memory}else{-after.memory};
    assert_eq!(relative,-1);
});}

#[test]
fn masked_routes_are_never_selected_and_fallback_is_unchanged(){run(||{
    let r=fixture();let mut mask=build_action_mask(&r.game,0);
    let eligible=legal_tamer_digivolutions(&r.game,&mask);
    for a in eligible {mask[a as usize]=0.;}
    assert!(legal_tamer_digivolutions(&r.game,&mask).is_empty());
    assert_eq!(greedy_tamer_first_action(&r.game,&mask),greedy_action(&r.game,&mask));
    let a=DevelopmentPolicy::default().decide(&r.game,&mask).unwrap();
    let b=DevelopmentPolicy::default().decide_tamer_first(&r.game,&mask).unwrap();
    assert_eq!(a.action,b.action);assert_eq!(a.candidates,b.candidates);assert_eq!(a.version,"development_v1");
});}

#[test]
fn non_tamer_targets_and_reused_action_ids_do_not_trigger(){run(||{
    let mut r=fixture();r.game.players[0].battle_area.clear();r.place_on_field(0,"TF-HYBRID",None);
    assert!(legal_tamer_digivolutions(&r.game,&build_action_mask(&r.game,0)).is_empty());
    r.game.current_phase=GamePhase::Mulligan;
    let mut mask=vec![0.;digimon_engine::action::ACTION_SPACE_SIZE];mask[0]=1.;mask[1]=1.;mask[encode_digivolve(0,0) as usize]=1.;
    assert!(legal_tamer_digivolutions(&r.game,&mask).is_empty());
});}

#[test]
fn development_repeated_state_guard_remains_and_reset_restores_choice(){run(||{
    use digimon_engine::policies::Policy;
    let r=fixture();let mask=build_action_mask(&r.game,0);let eligible=legal_tamer_digivolutions(&r.game,&mask);
    let mut dev=DevelopmentPolicy::default();let first=dev.decide_tamer_first(&r.game,&mask).unwrap().action;
    let mut seen=std::collections::BTreeSet::from([first]);
    for _ in 1..eligible.len(){let a=dev.decide_tamer_first(&r.game,&mask).unwrap().action;assert!(eligible.contains(&a));assert!(seen.insert(a));}
    assert!(dev.decide_tamer_first(&r.game,&mask).is_err());
    dev.reset();assert_eq!(dev.decide_tamer_first(&r.game,&mask).unwrap().action,first);
});}
