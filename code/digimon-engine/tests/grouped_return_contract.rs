//! Contract coverage for explicit simultaneous return; synthetic effects isolate reusable semantics.
use digimon_engine::{action::{build_action_mask,space::{encode_attack,PLAY_HAND_START}},card_source::CardHandle,debug_runner::{DebugRunner,make_test_card},enums::GamePhase};
fn act(r:&mut DebugRunner,a:u16) {
    let p=r.game.pending_selection.as_ref().expect("required public choice").selecting_player;
    assert_eq!(build_action_mask(&r.game,p)[a as usize],1.0);
    r.game.resolve_selection(p,a).unwrap();
}
fn fixture(position:&str,include_sources:bool)->DebugRunner {
    let yaml=format!(r#"
card: GROUP-CONTRACT
name: Group contract fixture
kind: digimon
color: [blue]
level: 3
cost: 0
dp: 2000
effects:
  - when: on_play
    process:
      - return_all_to_deck:
          over: {{of: opponent, zone: [battle_area], kind: digimon, level_eq: 4}}
          position: {position}
          include_sources: {include_sources}
"#);
    let mut b=DebugRunner::builder().from_dsl_yaml(&yaml).unwrap();
    for id in ["TOP-A","TOP-B","TOP-C","SOURCE-A","SOURCE-B","FILL"] {
        let mut c=make_test_card(id,id);c.level=Some(if id.starts_with("TOP-"){4}else{3});c.dp=Some(4000);b=b.add_card(c);
    }
    let mut r=b.hand(0,&["GROUP-CONTRACT"]).deck(0,&["FILL";20]).deck(1,&["FILL";20]).memory(10).start();
    r.skip_mulligan();r.game.current_phase=GamePhase::Main;r
}
fn begin(r:&mut DebugRunner) {
    assert_eq!(build_action_mask(&r.game,0)[PLAY_HAND_START as usize],1.0);
    r.game.decode_action(PLAY_HAND_START,0);
}
fn choose(r:&mut DebugRunner,index:usize,remaining:usize) {
    let v=r.pending_selection_view().expect("public ordering prompt");
    assert_eq!(v.selecting_player,0,"causing player orders the public cards");
    assert_eq!(v.valid_action_ids.len(),remaining);
    assert!(!r.game.pending_selection.as_ref().unwrap().is_optional);
    act(r,encode_attack(1,index as u16));
}
#[test]
fn top_three_members_expose_all_six_permutations_and_original_clone_agree() {
    for order in [[0,1,2],[0,2,1],[1,0,2],[1,2,0],[2,0,1],[2,1,0]] {
        let mut r=fixture("top",false);
        let mut cards=Vec::<CardHandle>::new();
        for id in ["TOP-A","TOP-B","TOP-C"] {
            let h=r.place_on_field(1,id,None);cards.push(r.game.players[1].battle_area[h.index as usize].top_card().handle());
        }
        begin(&mut r);
        let mut clone=r.game.clone();
        choose(&mut r,order[0],3);choose(&mut r,order[1],2);
        assert!(r.game.pending_selection.is_none());
        assert!(r.game.players[1].battle_area.is_empty());
        let top=r.game.players[1].deck.iter().rev().take(3).map(|c|c.handle()).collect::<Vec<_>>();
        assert_eq!(top,order.iter().map(|i|cards[*i]).collect::<Vec<_>>());
        let expected=r.game.verification_digest();
        std::mem::swap(&mut r.game,&mut clone);
        choose(&mut r,order[0],3);choose(&mut r,order[1],2);
        assert_eq!(r.game.verification_digest(),expected);
    }
}
#[test]
fn included_sources_stay_in_each_selected_stack_for_both_public_orders() {
    for position in ["top","bottom"] {for first in [0usize,1] {
        let mut r=fixture(position,true);
        r.place_stack(1,&["SOURCE-A","TOP-A"]);
        r.place_stack(1,&["SOURCE-B","TOP-B"]);
        let stacks=r.game.players[1].battle_area.iter().map(|p|p.card_sources.iter().map(|c|c.handle()).collect::<Vec<_>>()).collect::<Vec<_>>();
        begin(&mut r);choose(&mut r,first,2);
        assert!(r.game.pending_selection.is_none());
        assert!(r.game.players[1].battle_area.is_empty());
        assert!(r.game.players[1].trash.is_empty(),"included source cards move to deck, not trash");
        let raw=r.game.players[1].deck.iter().map(|c|c.handle()).collect::<Vec<_>>();
        if position=="top" {
            let expected=stacks[first].iter().rev().chain(stacks[1-first].iter().rev()).copied().collect::<Vec<_>>();
            assert_eq!(raw.iter().rev().take(4).copied().collect::<Vec<_>>(),expected);
        } else {
            let expected=stacks[first].iter().chain(stacks[1-first].iter()).copied().collect::<Vec<_>>();
            assert_eq!(raw[..4],expected);
        }
    }}
}
#[test]
fn included_sources_are_routed_to_physical_owners_without_changing_controller_order() {
    let mut r=fixture("bottom",true);
    r.place_stack(1,&["SOURCE-A","TOP-A"]);
    r.place_stack(1,&["SOURCE-B","TOP-B"]);
    r.game.players[1].battle_area[0].card_sources[0].owner=0;
    let foreign=r.game.players[1].battle_area[0].card_sources[0].handle();
    let top=r.game.players[1].battle_area[0].top_card().handle();
    begin(&mut r);choose(&mut r,0,2);
    assert!(r.game.players[0].deck.iter().any(|c|c.handle()==foreign));
    assert!(!r.game.players[1].deck.iter().any(|c|c.handle()==foreign));
    assert_eq!(r.game.players[1].deck[0].handle(),top);
    assert!(r.game.players.iter().all(|p|p.trash.is_empty()));
}


const INHERITED_LOOP:&str=r#"
card: INHERITED-LOOP
name: Inherited continuation witness
kind: digimon
color: [blue]
level: 3
cost: 0
dp: 2000
effects:
  - scope: inherited
    when: when_attacking
    process:
      - for_each:
          over: {of: you, zone: [battle_area], kind: digimon, level_eq: 4}
          bind_as: member
          body:
            - select_effect_choice:
                labels: ["Return selected member", "Return selected member also"]
                prompt: "Park each actual member"
                bind_as: branch
            - return_to_hand: {target: member}
      - suspend: {target: source}
      - draw: {of: you, count: 1}
"#;
fn inherited_fixture(departed:bool)->DebugRunner {
    let mut b=DebugRunner::builder().from_dsl_yaml(INHERITED_LOOP).unwrap();
    for (id,level) in [("LOOP-FIRST",4),("LOOP-HOST",if departed {4}else{5}),("LOOP-LAST",4),("LOOP-OTHER",5)] {
        let mut c=make_test_card(id,id);c.level=Some(level);c.dp=Some(5000);b=b.add_card(c);
    }
    let mut r=b.deck(0,&["LOOP-OTHER";20]).deck(1,&["LOOP-OTHER";20]).memory(10).start();
    r.skip_mulligan();r.game.current_phase=GamePhase::Main;
    if !departed {r.place_on_field(1,"LOOP-FIRST",None);}
    let host=r.place_stack(1,&["INHERITED-LOOP","LOOP-HOST"]);
    r.place_on_field(1,"LOOP-LAST",None);
    r.place_on_field(1,"LOOP-OTHER",None);
    r.game.enqueue_triggered(digimon_engine::enums::EffectTiming::WhenAttacking,digimon_engine::selection::TriggerSource::Permanent(host));
    r.game.drain_effect_queue();r
}
#[test]
fn inherited_loop_continuation_re_resolves_compacted_carrier_and_keeps_actor() {
    let mut r=inherited_fixture(false);
    let before=r.game.players[1].hand.len();
    let mut cloned=r.game.clone();let mut expected=None;
    for run in 0..2 {
        if run==1 {std::mem::swap(&mut r.game,&mut cloned);}
        for remaining in [2,1] {
            let v=r.pending_selection_view().expect("each iteration retains its player choice");
            assert_eq!(v.selecting_player,1);assert_eq!(v.valid_action_ids.len(),2);
            let a=v.valid_action_ids[0];act(&mut r,a);
            if remaining==2 {assert_eq!(r.game.players[1].hand.len(),before+1);}
        }
        assert!(r.game.pending_selection.is_none());
        assert_eq!(r.game.players[1].battle_area.len(),2);
        assert_eq!(r.game.players[1].battle_area[0].top_card().card_id(&r.game.card_data),"LOOP-HOST");
        assert!(r.game.players[1].battle_area[0].is_suspended);
        assert!(!r.game.players[1].battle_area[1].is_suspended);
        assert_eq!(r.game.players[1].hand.len(),before+3,"two returns and exactly one actor-relative draw");
        let d=r.game.verification_digest();if let Some(prior)=expected {assert_eq!(d,prior);}else{expected=Some(d);}
    }
}
#[test]
fn departed_inherited_source_keeps_controller_tail_without_retargeting_new_slot() {
    let mut r=inherited_fixture(true);
    let before=r.game.players[1].hand.len();let mut cloned=r.game.clone();let mut expected=None;
    for run in 0..2 {
        if run==1 {std::mem::swap(&mut r.game,&mut cloned);}
        for _ in 0..2 {let v=r.pending_selection_view().expect("later member still resolves after source departure");assert_eq!(v.selecting_player,1);let a=v.valid_action_ids[0];act(&mut r,a);}
        assert!(r.game.pending_selection.is_none());
        assert_eq!(r.game.players[1].battle_area.len(),1);
        assert_eq!(r.game.players[1].battle_area[0].top_card().card_id(&r.game.card_data),"LOOP-OTHER");
        assert!(!r.game.players[1].battle_area[0].is_suspended,"departed source binding never points at compacted unrelated permanent");
        assert_eq!(r.game.players[1].hand.len(),before+3);
        assert_eq!(r.game.players[1].trash.iter().filter(|c|c.card_id(&r.game.card_data)=="INHERITED-LOOP").count(),1);
        let d=r.game.verification_digest();if let Some(prior)=expected {assert_eq!(d,prior);}else{expected=Some(d);}
    }
}


#[test]
fn captured_return_does_not_follow_same_physical_card_into_fresh_incarnation() {
    let mut r=fixture("bottom",false);
    r.place_on_field(1,"TOP-A",None);r.place_on_field(1,"TOP-B",None);
    let first=r.game.players[1].battle_area[0].top_card().handle();
    let second=r.game.players[1].battle_area[1].top_card().handle();
    begin(&mut r);
    // A direct state transition isolates captured-incarnation validation; no
    // claim that a native Main action can be taken during an ordering prompt.
    let departed=r.game.players[1].battle_area.remove(1);
    let card=departed.card_sources.into_iter().last().unwrap();
    r.game.players[1].battle_area.push(digimon_engine::permanent::Permanent::new(card,r.game.turn_count));
    let mut clone=r.game.clone();let mut expected=None;
    for run in 0..2 {
        if run==1 {std::mem::swap(&mut r.game,&mut clone);}
        let v=r.pending_selection_view().unwrap();assert_eq!(v.selecting_player,0);
        act(&mut r,encode_attack(1,0));
        assert!(r.game.pending_selection.is_none());
        assert_eq!(r.game.players[1].battle_area.len(),1);
        assert_eq!(r.game.players[1].battle_area[0].top_card().handle(),second);
        assert!(r.game.players[1].deck.iter().any(|c|c.handle()==first));
        assert!(!r.game.players[1].deck.iter().any(|c|c.handle()==second));
        let d=r.game.verification_digest();if let Some(prior)=expected {assert_eq!(d,prior);}else{expected=Some(d);}
    }
}
#[test]
fn own_group_hand_return_removes_token_and_routes_real_mother_to_egg_deck() {
    let yaml=r#"
card: GROUP-HAND-CONTRACT
name: Group hand routing fixture
kind: digimon
color: [blue]
level: 3
cost: 0
dp: 2000
effects:
  - when: on_play
    process:
      - return_all_to_hand:
          over:
            of: you
            zone: [battle_area]
            any_of:
              - name_is: Mother D-Reaper
              - name_is: Routing token
"#;
    let mut token=make_test_card("ROUTING-TOKEN","Routing token");
    token.card_kind=digimon_engine::enums::CardKind::Token;
    let mut r=DebugRunner::builder().from_dsl_yaml(yaml).unwrap().dsl_card("EX2-007").unwrap().add_card(token)
        .hand(0,&["GROUP-HAND-CONTRACT"]).deck(0,&["GROUP-HAND-CONTRACT";20]).deck(1,&["GROUP-HAND-CONTRACT";20]).memory(10).start();
    r.skip_mulligan();r.game.current_phase=GamePhase::Main;
    let m=r.place_on_field(0,"EX2-007",None);let mother=r.game.players[0].battle_area[m.index as usize].top_card().handle();
    let t=r.place_on_field(0,"ROUTING-TOKEN",None);let token=r.game.players[0].battle_area[t.index as usize].top_card().handle();
    begin(&mut r);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.game.players[0].battle_area.len(),1);
    assert!(r.game.players[0].digitama_deck.iter().any(|c|c.handle()==mother));
    assert!(!r.game.players[0].hand.iter().any(|c|c.handle()==mother||c.handle()==token));
    assert!(!r.game.players[0].trash.iter().any(|c|c.handle()==token));
    assert!(!r.game.players[0].deck.iter().any(|c|c.handle()==token));
}
