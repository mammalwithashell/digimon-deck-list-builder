use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::modifiers::EffectControllerFilter;
use digimon_engine::{EffectSourceKind,EffectTiming,Expiry,TriggerSource};

#[test]
fn self_granted_trigger_survives_shield_but_other_friendly_grant_does_not() {
    let mut runner=DebugRunner::builder().add_card(make_test_card("GRANTED-SELF","Self"))
        .add_card(make_test_card("GRANTED-FRIEND","Friend"))
        .deck(0,&["GRANTED-SELF";10]).deck(1,&["GRANTED-FRIEND";10]).memory(0).start();
    let host=runner.place_on_field(0,"GRANTED-SELF",None);
    let friend=runner.place_on_field(0,"GRANTED-FRIEND",None);
    let own_card=runner.game.players[0].battle_area[0].top_card().handle();
    let friend_card=runner.game.players[0].battle_area[1].top_card().handle();
    {
        let mut ctx=EffectContext::new(&mut runner.game,own_card,Some(host),0);
        ctx.grant_triggered_effect(host,EffectTiming::OnAttack,Expiry::EndOfTurn,|ctx| {ctx.gain_memory(2);});
        assert!(ctx.add_effect_immunity_modifier_excluding_self(host,EffectSourceKind::Digimon,
            EffectControllerFilter::Any,Expiry::EndOfOpponentsTurn,true));
    }
    EffectContext::new(&mut runner.game,friend_card,Some(friend),0)
        .grant_triggered_effect(host,EffectTiming::OnAttack,Expiry::EndOfTurn,|ctx| {ctx.gain_memory(7);});
    runner.game.enqueue_triggered(EffectTiming::OnAttack,TriggerSource::Permanent(host));
    runner.game.drain_effect_queue();
    runner.auto_resolve().unwrap();
    assert_eq!(runner.game.memory,2,"only the carrier's own gained effect must fire");
}


#[test]
fn legacy_granted_body_uses_carrier_category_instead_of_option_grantor() {
    let mut option=make_test_card("OPTION-GRANTOR","Option");option.card_kind=digimon_engine::CardKind::Option;
    let mut runner=DebugRunner::builder().add_card(option)
        .add_card(make_test_card("GAINED-CARRIER","Carrier"))
        .hand(0,&["OPTION-GRANTOR"]).memory(0).build();
    let carrier=runner.place_on_field(0,"GAINED-CARRIER",None);
    let source=runner.game.players[0].hand[0].handle();
    EffectContext::new(&mut runner.game,source,None,0).grant_triggered_effect(
        carrier,EffectTiming::OnAttack,Expiry::EndOfTurn,|ctx| {
            if ctx.source_kind==EffectSourceKind::Digimon {ctx.gain_memory(3);}
        });
    runner.game.fire_granted_triggered_effects(carrier,EffectTiming::OnAttack);
    assert_eq!(runner.game.memory,3,"gained body is the carrier's own Digimon effect");
}

#[test]
fn dynamic_grant_predicate_does_not_follow_physical_source_into_new_carrier() {
    let mut runner=DebugRunner::builder().add_card(make_test_card("DYNAMIC-GRANTOR","Grantor"))
        .add_card(make_test_card("FILLER","Filler"))
        .deck(0,&["FILLER";10]).deck(1,&["FILLER";10]).memory(0).start();
    let old=runner.place_on_field(0,"DYNAMIC-GRANTOR",None);
    let source=runner.game.players[0].battle_area[0].top_card().handle();
    EffectContext::new(&mut runner.game,source,Some(old),0).grant_dynamic_triggered_effect(
        EffectTiming::OnAttack,Expiry::EndOfTurn,
        |ctx,carrier|ctx.source_permanent==Some(carrier),|ctx| {ctx.gain_memory(4);});
    runner.game.delete_permanent_with_effects(old);
    let trash=runner.game.players[0].trash.iter().position(|c|c.handle()==source).unwrap();
    let new_index=runner.game.play_from_trash(0,trash).unwrap();
    let returned=digimon_engine::permanent::PermanentHandle{player:0,index:new_index as u8};
    assert_eq!(runner.game.players[0].battle_area[new_index].top_card().handle(),source);
    let before_trigger=runner.game.memory;
    runner.game.enqueue_triggered(EffectTiming::OnAttack,TriggerSource::Permanent(returned));
    runner.game.drain_effect_queue();runner.auto_resolve().unwrap();
    assert_eq!(runner.game.memory,before_trigger,"a source-independent grant retains the original departed carrier, not a new incarnation");
}
