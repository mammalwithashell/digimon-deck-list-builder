use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::{CardKind, EffectSourceKind, EffectTiming, TriggerSource};

#[test]
fn native_digi_egg_is_not_a_digimon_effect() {
    assert_ne!(EffectSourceKind::for_card_kind(CardKind::DigiEgg),
               EffectSourceKind::for_card_kind(CardKind::Digimon));
}

#[test]
fn own_carrier_exclusion_is_an_explicit_dsl_choice() {
    let yaml = r#"
card: TST-OWN-IMMUNITY
name: Own Carrier Immunity
kind: digimon
level: 6
color: [yellow]
cost: 12
dp: 12000
effects:
  - when: when_digivolving
    process:
      - grant_effect_immunity:
          target: self
          source_kind: digimon
          source_controller: any
          exclude_self: true
          expiry: end_of_opponents_turn
"#;
    let mut runner=DebugRunner::builder().from_dsl_yaml(yaml).expect("exclude_self is supported").build();
    let host=runner.place_on_field(0,"TST-OWN-IMMUNITY",None);
    runner.game.enqueue_triggered(EffectTiming::WhenDigivolving,TriggerSource::Permanent(host));
    runner.game.drain_effect_queue();
    assert!(runner.game.permanent_is_unaffected_by_effect(host,1,EffectSourceKind::Digimon));
}


struct FormulaAura(i32);
impl digimon_engine::effect::CardEffect for FormulaAura {
    fn effects(&self, card: digimon_engine::card_source::CardHandle) -> Vec<digimon_engine::effect::Effect> {
        let value=self.0;
        vec![digimon_engine::effect::Effect::declarative(card)
            .dp_modifier_fn(move |_,_| Some(value)).build()]
    }
}

#[test]
fn live_formula_aura_uses_original_carrier_identity_at_read_time() {
    use digimon_engine::debug_runner::make_test_card;
    use digimon_engine::effect_context::EffectContext;
    use digimon_engine::modifiers::EffectControllerFilter;
    use digimon_engine::Expiry;
    let mut own=make_test_card("FORMULA-OWN","Own"); own.dp=Some(12000);
    let mut runner=DebugRunner::builder().add_card(own)
        .add_card(make_test_card("FORMULA-FRIEND","Friend")).build();
    let host=runner.place_on_field(0,"FORMULA-OWN",None);
    runner.place_on_field(0,"FORMULA-FRIEND",None);
    let card=runner.game.players[0].battle_area[0].top_card().handle();
    EffectContext::new(&mut runner.game,card,Some(host),0)
        .add_effect_immunity_modifier_excluding_self(host,EffectSourceKind::Digimon,
            EffectControllerFilter::Any,Expiry::EndOfOpponentsTurn,true);
    runner.register_effect("FORMULA-OWN",std::sync::Arc::new(FormulaAura(2000)));
    runner.register_effect("FORMULA-FRIEND",std::sync::Arc::new(FormulaAura(5000)));
    assert_eq!(runner.game.dynamic_dp_aura_bonus(host),2000,
        "own formula is exempt; other friendly formula is suppressed");
    runner.game.modifiers.expire_end_of_turn(1);
    assert_eq!(runner.game.dynamic_dp_aura_bonus(host),7000,
        "other formula applies immediately when immunity expires");
}

#[test]
fn inherited_digi_egg_effect_remains_a_digimon_effect() {
    use digimon_engine::debug_runner::make_test_card;
    use digimon_engine::card_source::CardSource;
    use digimon_engine::effect_context::EffectContext;
    let mut egg=make_test_card("SOURCE-EGG","Egg");egg.card_kind=CardKind::DigiEgg;
    let mut runner=DebugRunner::builder().add_card(egg)
        .add_card(make_test_card("EGG-HOST","Host")).build();
    let host=runner.place_on_field(0,"EGG-HOST",None);
    let data=runner.game.card_data.iter().position(|c|c.card_id=="SOURCE-EGG").unwrap();
    let unique=runner.game.next_card_index();
    let egg=CardSource::new(data,0,unique);let handle=egg.handle();
    runner.game.players[0].battle_area[0].card_sources.insert(0,egg);
    let ctx=EffectContext::new(&mut runner.game,handle,Some(host),0);
    assert_eq!(ctx.source_kind,EffectSourceKind::Digimon);
    assert!(ctx.source_identity.is_some());
}


#[test]
fn same_physical_card_reentering_is_not_the_excluded_old_carrier() {
    use digimon_engine::debug_runner::make_test_card;
    use digimon_engine::effect_context::EffectContext;
    use digimon_engine::modifiers::EffectControllerFilter;
    use digimon_engine::{Expiry,permanent::PermanentHandle};
    let mut runner=DebugRunner::builder().add_card(make_test_card("REENTER","Grantor"))
        .add_card(make_test_card("PROTECTED","Protected"))
        .deck(0,&["PROTECTED";10]).deck(1,&["PROTECTED";10]).start();
    let grantor=runner.place_on_field(0,"REENTER",None);
    let target=runner.place_on_field(0,"PROTECTED",None);
    let physical=runner.game.players[0].battle_area[grantor.index as usize].top_card().handle();
    let original_identity={
        let mut ctx=EffectContext::new(&mut runner.game,physical,Some(grantor),0);
        assert!(ctx.add_effect_immunity_modifier_excluding_self(target,EffectSourceKind::Digimon,
            EffectControllerFilter::Any,Expiry::EndOfOpponentsTurn,true));
        assert!(ctx.can_affect_permanent(target));
        ctx.source_identity
    };
    runner.game.delete_permanent_with_effects(grantor);
    let trash=runner.game.players[0].trash.iter().position(|c|c.handle()==physical).unwrap();
    let index=runner.game.play_from_trash(0,trash).expect("same card returns");
    let returned=PermanentHandle{player:0,index:index as u8};
    assert_eq!(runner.game.players[0].battle_area[index].top_card().handle(),physical);
    let shifted_target=PermanentHandle{player:0,index:0};
    let ctx=EffectContext::new(&mut runner.game,physical,Some(returned),0);
    assert_ne!(ctx.source_identity,original_identity);
    assert!(!ctx.can_affect_permanent(shifted_target),
        "fresh carrier must not inherit the departed carrier's exemption");
}


#[test]
fn self_modifier_binding_resolves_the_carrier_instead_of_silently_skipping() {
    let yaml=r#"
card: SELF-MODIFIER-IDENTITY
name: Self Modifier Identity
kind: digimon
level: 6
color: [yellow]
cost: 12
dp: 12000
effects:
  - when: on_play
    process:
      - add_modifier:
          target: self
          modifier: ChangeDp
          value: 2000
          expiry: end_of_turn
"#;
    let mut runner=DebugRunner::builder().from_dsl_yaml(yaml).unwrap().build();
    let own=runner.place_on_field(0,"SELF-MODIFIER-IDENTITY",None);
    runner.game.enqueue_triggered(EffectTiming::OnPlay,TriggerSource::Permanent(own));
    runner.game.drain_effect_queue();
    assert_eq!(runner.game.effective_dp(own),Some(14000));
}


#[test]
fn own_source_prefix_payment_is_legal_under_own_carrier_exclusion() {
    use digimon_dsl::compiled::{CompiledStep,CompiledPredicate};
    use digimon_engine::debug_runner::make_test_card;
    use digimon_engine::card_source::CardSource;
    use digimon_engine::effect_context::EffectContext;
    use digimon_engine::modifiers::EffectControllerFilter;
    use digimon_engine::dsl_cards::{step::run_steps,bindings::Bindings};
    use digimon_engine::Expiry;
    let mut runner=DebugRunner::builder().add_card(make_test_card("PREFIX-OWN","Own"))
        .add_card(make_test_card("PREFIX-SOURCE","Source"))
        .deck(0,&["PREFIX-SOURCE";10]).deck(1,&["PREFIX-SOURCE";10]).memory(0).start();
    let host=runner.place_on_field(0,"PREFIX-OWN",None);
    let data=runner.game.card_data.iter().position(|c|c.card_id=="PREFIX-SOURCE").unwrap();
    let unique=runner.game.next_card_index();
    runner.game.players[0].battle_area[0].card_sources.insert(0,CardSource::new(data,0,unique));
    let own=runner.game.players[0].battle_area[0].top_card().handle();
    {
        let mut ctx=EffectContext::new(&mut runner.game,own,Some(host),0);
        ctx.add_effect_immunity_modifier_excluding_self(host,EffectSourceKind::Digimon,
            EffectControllerFilter::Any,Expiry::EndOfOpponentsTurn,true);
        run_steps(&[CompiledStep::PaySourcePrefix{host_filter:CompiledPredicate::default(),minimum:1,optional:false},
                    CompiledStep::GainMemory(2)],&mut ctx,&mut Bindings::default());
    }
    let host_choice=runner.pending_selection_view().expect("own payment is payable despite the broad Digimon shield");
    assert_eq!(host_choice.valid_action_ids.len(),1);
    runner.execute_action(host_choice.selecting_player,host_choice.valid_action_ids[0]).unwrap();
    let count=runner.pending_selection_view().expect("physical prefix count is a player choice");
    assert!(!count.valid_action_ids.is_empty());
    runner.execute_action(count.selecting_player,count.valid_action_ids[0]).unwrap();
    runner.auto_resolve().unwrap();
    assert_eq!(runner.game.players[0].battle_area[0].card_sources.len(),1);
    assert_eq!(runner.game.memory,2);
}


#[test]
fn read_context_retains_departed_activation_origin_and_explicit_cardless_origin() {
    use digimon_engine::debug_runner::make_test_card;
    use digimon_engine::effect_context::EffectContext;
    let mut runner=DebugRunner::builder().add_card(make_test_card("READ-ORIGIN","Origin"))
        .deck(0,&["READ-ORIGIN";10]).deck(1,&["READ-ORIGIN";10]).start();
    let host=runner.place_on_field(0,"READ-ORIGIN",None);
    let card=runner.game.players[0].battle_area[0].top_card().handle();
    {
        let mut ctx=EffectContext::new(&mut runner.game,card,Some(host),0);
        let origin=ctx.source_identity.expect("activation allocates an origin identity");
        ctx.game.delete_permanent_with_effects(host);
        assert!(ctx.as_read().source_permanent.is_none(),"departed origin has no live field slot");
        assert_eq!(ctx.as_read().source_identity,Some(origin),
            "readonly payability after departure keeps the activated origin");
    }
    let ctx=EffectContext::new_with_source_kind(&mut runner.game,card,None,EffectSourceKind::Digimon,0);
    assert_eq!(ctx.as_read().source_identity,None,
        "captured cardless origin must not be inferred from a later physical incarnation");
}


#[test]
fn read_context_follows_carrier_when_current_body_deletes_a_lower_slot() {
    use digimon_engine::debug_runner::make_test_card;
    use digimon_engine::effect_context::EffectContext;
    let mut runner=DebugRunner::builder().add_card(make_test_card("READ-LOWER","Lower"))
        .add_card(make_test_card("READ-SELF","Self"))
        .deck(0,&["READ-LOWER";10]).deck(1,&["READ-LOWER";10]).start();
    let lower=runner.place_on_field(0,"READ-LOWER",None);
    let host=runner.place_on_field(0,"READ-SELF",None);
    runner.place_on_field(0,"READ-LOWER",None);
    let source=runner.game.players[0].battle_area[1].top_card().handle();
    let mut ctx=EffectContext::new(&mut runner.game,source,Some(host),0);
    ctx.game.delete_permanent_with_effects(lower);
    assert_eq!(ctx.game.players[0].battle_area.len(),2);
    let read=ctx.as_read();
    let live=read.source_permanent.expect("original carrier still exists");
    assert_eq!(live.index,0,"install-time read predicates must not see the old slot's neighbor");
    assert_eq!(read.game.players[0].battle_area[live.index as usize].top_card().handle(),source);
}
