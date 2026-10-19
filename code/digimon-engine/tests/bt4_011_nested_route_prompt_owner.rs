// Synthetic declaration-ownership capability control, not a new printed card rule.
include!("bt4_011_synchronous_inner_park.rs");

#[test]
fn nested_multi_route_prompt_keeps_inner_payment_source_after_outer_unwinds() {
    use digimon_engine::action::space::PLAY_HAND_START;
    let inner = format!("{}{}", INNER, "  - kind: digivolve\n    from: {kind: digimon, level_eq: 3, color_is: red}\n    cost: 3\n");
    let mut r=DebugRunner::builder()
        .from_dsl_yaml(include_str!("../cards/bt4/BT4-011.yaml")).unwrap()
        .from_dsl_yaml(include_str!("../cards/st1/ST1-03.yaml")).unwrap()
        .from_dsl_yaml(&inner).unwrap().from_dsl_yaml(SYNC_INITIATOR).unwrap()
        .from_dsl_yaml(SYNC_PAYER).unwrap()
        .hand(0,&["BT4-011"]).deck(0,&["ST1-03";12])
        .deck(1,&["ST1-03";12]).memory(5).start();
    r.skip_mulligan();
    let outer=r.place_on_field(0,"TEST-SYNC-INITIATOR",Some(0));
    let payer=r.place_on_field(0,"TEST-SYNC-PAYER",Some(0));
    let payer_card=r.top_card(payer);
    let i=r.add_to_hand(0,"TEST-Q1157-INNER");
    let result=r.game.players[0].hand.remove(i);r.game.players[0].deck.push(result);
    r.game.current_phase=GamePhase::Main;r.game.tick_declarative_effects();
    r.fire_play_event_triggers(0,outer.index as usize,true,false);
    choose(&mut r,PLAY_HAND_START);
    assert_eq!(r.memory(),5);
    let parked=r.game.clone();
    for game in [&parked,&parked.clone()] {
        let pending=game.pending_selection.as_ref().expect("inner route decision remains public");
        let entries=pending.effect_choices.as_ref().expect("two distinct payable route costs");
        assert_eq!(entries.len(),2);
        assert_eq!(pending.source_card,payer_card,"outer declaration must not relabel inner route prompt");
        assert_eq!(pending.source_permanent,Some(payer));
        assert_eq!(pending.source_kind,digimon_engine::enums::EffectSourceKind::Digimon);
        for entry in entries {
            assert_eq!(entry.source_card,Some(payer_card));
            assert_eq!(entry.source_kind,Some(digimon_engine::enums::EffectSourceKind::Digimon));
            assert_eq!(build_action_mask(game,0)[entry.action_id as usize],1.0);
        }
    }
}