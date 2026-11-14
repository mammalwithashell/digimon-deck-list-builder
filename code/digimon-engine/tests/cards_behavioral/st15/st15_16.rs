//! Official ST15-16: independent mandatory targets, De-Digivolve 3, temporary forced attack.
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::action::space::PASS;

fn setup() -> DebugRunner {
    let mut b = DebugRunner::builder().dsl_card("ST15-16").expect("Trident Arm registered");
    for (id, level) in [("L3",3),("L4",4),("L5",5),("L6",6),("L7",7)] {
        let mut c = make_test_card(id,id);
        c.card_kind=CardKind::Digimon; c.level=Some(level); c.dp=Some(5000);
        c.colors=vec![CardColor::Black]; b=b.add_card(c);
    }
    let mut filler=make_test_card("FILL","Filler");
    filler.dp=Some(1000);
    b=b.add_card(filler);
    b.hand(0,&["ST15-16"]).deck(0,&["L3";8]).deck(1,&["L3";8])
        .security(0,&["FILL";5]).security(1,&["FILL";5]).memory(10).start()
}

#[test]
fn st15_16_main_dedigivolves_three_then_grants_independently_chosen_attack() {
    let mut r=setup();
    r.place_on_field(0,"L3",None);
    let stack=r.place_stack(1,&["L3","L4","L5","L6","L7"]);
    let other=r.place_on_field(1,"L3",None);
    r.game.activate_hand_main(0,0);
    let p=r.pending_selection_view().expect("first opponent target");
    assert_eq!(p.valid_action_ids.len(),2);
    r.execute_action(p.selecting_player,p.valid_action_ids[0]).unwrap();
    let count=r.pending_selection_view().expect("declare De-Digivolve count");
    assert!(!count.valid_action_ids.contains(&PASS));
    r.execute_action(count.selecting_player,*count.valid_action_ids.last().unwrap()).unwrap();
    assert_eq!(r.game.players[1].battle_area[stack.index as usize].card_sources.len(),2);
    let p=r.pending_selection_view().expect("independent grant target");
    assert_eq!(p.valid_action_ids.len(),2);
    assert!(!p.valid_action_ids.contains(&PASS));
    r.execute_action(p.selecting_player,p.valid_action_ids[1]).unwrap();
    assert_eq!(r.game.modifiers.granted_triggered_for_timing(other,EffectTiming::StartOfYourMainPhase).len(),1);
    assert_eq!(r.game.modifiers.granted_triggered_for_timing(stack,EffectTiming::StartOfYourMainPhase).len(),0);
    r.end_turn();
    let p=r.pending_selection_view().expect("opponent must choose an attack");
    assert_eq!(p.selecting_player,1);
    assert!(!p.valid_action_ids.contains(&PASS));
    let mut fork=r.game.clone();
    fork.decode_action(p.valid_action_ids[0],p.selecting_player);
    assert!(fork.players[1].battle_area[other.index as usize].is_suspended);
    r.execute_action(p.selecting_player,p.valid_action_ids[0]).unwrap();
    r.auto_resolve().unwrap();
    r.end_turn();
    assert!(r.game.modifiers.granted_triggered_for_timing(other,EffectTiming::StartOfYourMainPhase).is_empty());
}

#[test]
fn st15_16_security_stops_at_level_three_and_does_not_grant_attack() {
    use digimon_engine::selection::{PendingSecurity, TriggerSource};
    let mut r=setup();
    let stack=r.place_stack(1,&["L3","L4","L5"]);
    let card=r.game.players[0].hand.remove(0);
    let handle=card.handle();
    r.game.pending_security=Some(PendingSecurity{defender:0,card,played:false});
    r.game.enqueue_triggered(EffectTiming::SecuritySkill,TriggerSource::SecurityRevealed{defender:0,card:handle});
    r.game.drain_effect_queue();
    let p=r.pending_selection_view().expect("security target choice");
    r.execute_action(p.selecting_player,p.valid_action_ids[0]).unwrap();
    let count=r.pending_selection_view().expect("declare De-Digivolve count");
    assert!(!count.valid_action_ids.contains(&PASS));
    r.execute_action(count.selecting_player,*count.valid_action_ids.last().unwrap()).unwrap();
    assert_eq!(r.game.players[1].battle_area[stack.index as usize].card_sources.len(),1);
    assert_eq!(r.game.players[1].trash.len(),2);
    assert!(r.pending_selection_view().is_none());
    assert!(r.game.modifiers.granted_triggered_for_timing(stack,EffectTiming::StartOfYourMainPhase).is_empty());
}

#[test]
fn st15_16_main_with_no_opponent_targets_settles_without_choice() {
    let mut r=setup();
    r.place_on_field(0,"L3",None);
    r.game.activate_hand_main(0,0);
    assert!(r.pending_selection_view().is_none());
    assert_eq!(r.battle_area_size(0),1);
}
