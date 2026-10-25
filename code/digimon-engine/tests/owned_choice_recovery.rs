//! Invalidated mandatory pages must not strand a policy or pay an invalid cost.
use digimon_engine::{action::{build_action_mask, space::PASS},
    card_choice_page::{CARD_CHOICE_NEXT, CARD_CHOICE_START},
    debug_runner::{make_test_card, DebugRunner}, effect_context::EffectContext};

fn runner() -> DebugRunner {
    let mut r=DebugRunner::builder()
        .add_card(make_test_card("RECOVERY-SOURCE", "Source"))
        .add_card(make_test_card("RECOVERY-TOP", "Top"))
        .deck(0, &["RECOVERY-SOURCE";20]).deck(1,&["RECOVERY-SOURCE";20])
        .security(0,&["RECOVERY-SOURCE";40]).memory(3).start();
    r.skip_mulligan(); r
}

#[test]
fn mandatory_security_shuffle_offers_refresh_without_committing_an_old_row() {
    let mut r=runner(); let host=r.place_stack(0,&["RECOVERY-TOP"]);
    let top=r.game.player(0).battle_area[0].top_card().handle();
    EffectContext::new(&mut r.game,top,Some(host),0).select_security(
        0,"Mandatory security",false,|_,_| true,|_,_| panic!("refresh must not commit"));
    let old=r.game.pending_selection.as_ref().unwrap().card_choice.as_ref().unwrap().token();
    r.game.shuffle_security(0);
    let before=r.game.verification_digest();
    assert!(r.game.resolve_selection_checked(0,CARD_CHOICE_START,old).is_err());
    assert_eq!(r.game.verification_digest(),before);
    assert_eq!(build_action_mask(&r.game,0)[PASS as usize],0.0,"a live mandatory choice cannot be declined");
    assert_eq!(build_action_mask(&r.game,0)[CARD_CHOICE_NEXT as usize],1.0,
        "invalidated security order requires a presentation-only refresh operation");
    r.game.resolve_selection_checked(0,CARD_CHOICE_NEXT,old).unwrap();
    let current=r.game.pending_selection.as_ref().unwrap().card_choice.as_ref().unwrap().token();
    assert_ne!(old,current,"refresh invalidates previously rendered submissions");
    assert_eq!(build_action_mask(&r.game,0)[CARD_CHOICE_START as usize],1.0);
}

#[test]
fn removed_mandatory_security_candidates_can_finish_without_callback_or_reward() {
    let mut r=runner(); let host=r.place_stack(0,&["RECOVERY-TOP"]);
    let top=r.game.player(0).battle_area[0].top_card().handle();
    EffectContext::new(&mut r.game,top,Some(host),0).select_security(
        0,"Mandatory security",false,|_,_|true,|_,_|panic!("no physical card can pay"));
    r.game.player_mut(0).security.clear();
    assert_eq!(build_action_mask(&r.game,0)[PASS as usize],1.0,
        "an impossible mandatory operation must expose completion, without payment");
    r.game.resolve_selection(0,PASS).unwrap();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.game.memory,3);
}


#[test]
fn refresh_rechecks_live_index_filter_instead_of_preserving_the_old_top_card() {
    let mut r=runner(); let host=r.place_stack(0,&["RECOVERY-TOP"]);
    let top=r.game.player(0).battle_area[0].top_card().handle();
    EffectContext::new(&mut r.game,top,Some(host),0).select_security(0,"Current top",false,
        |_,index| index==0, |ctx,index| { assert_eq!(index,0);ctx.game.memory=7; });
    let old_top=r.game.player(0).security[0].handle();
    for _ in 0..10 {r.game.shuffle_security(0);if r.game.player(0).security[0].handle()!=old_top {break;}}
    assert_ne!(r.game.player(0).security[0].handle(),old_top);
    let expected=r.game.player(0).security[0].handle();
    let before_order:Vec<_>=r.game.player(0).security.iter().map(|card|card.handle()).collect();
    let before_events=r.game.events.len();let mut rng_control=r.game.clone();
    r.game.resolve_selection(0,CARD_CHOICE_NEXT).unwrap();
    assert_eq!(before_order,r.game.player(0).security.iter().map(|card|card.handle()).collect::<Vec<_>>());
    assert_eq!(before_events,r.game.events.len());
    let owner=r.game.pending_selection.as_ref().unwrap().card_choice.as_ref().unwrap();
    assert_eq!(owner.valid_action_ids(&r.game),vec![CARD_CHOICE_START]);
    let mut probe=owner.clone();
    assert!(matches!(probe.resolve(&r.game,CARD_CHOICE_START).unwrap(),
        digimon_engine::card_choice_page::CardChoiceResult::Selected(
            digimon_engine::owned_card_choice::ResolvedCardChoice::Security {card,index:0,..}) if card==expected));
    r.game.resolve_selection(0,CARD_CHOICE_START).unwrap();assert_eq!(r.game.memory,7);
    r.game.shuffle_security(0);rng_control.shuffle_security(0);
    assert_eq!(r.game.player(0).security.iter().map(|card|card.handle()).collect::<Vec<_>>(),
        rng_control.player(0).security.iter().map(|card|card.handle()).collect::<Vec<_>>(),"refresh must not consume RNG");
}

#[test]
fn dsl_security_refresh_retains_typed_filter_and_clone_permissions() {
    use digimon_dsl::compiled::{CompiledPlayerRef,CompiledPredicate,CompiledStep};
    use digimon_engine::dsl_cards::{bindings::Bindings,step::run_steps};
    let mut r=runner();let host=r.place_stack(0,&["RECOVERY-TOP"]);
    let top=r.game.player(0).battle_area[0].top_card().handle();
    let extra=r.place_stack(0,&["RECOVERY-TOP"]);
    let card=r.game.players[0].battle_area.remove(extra.index as usize).card_sources.remove(0);
    r.game.players[0].security.insert(1,card);
    run_steps(&[CompiledStep::SelectSecurity {continue_on_decline:false,of:CompiledPlayerRef::You,
        filter:CompiledPredicate {name_is:Some("Top".into()),..Default::default()},bind_as:Some("chosen".into()),
        prompt:"Only Top".into(),prompt_key:None,optional:false,then:vec![CompiledStep::GainMemory(4)]}],
        &mut EffectContext::new(&mut r.game,top,Some(host),0),&mut Bindings::new());
    r.game.shuffle_security(0);let saved=r.game.clone();
    for game in [&mut r.game,&mut saved.clone()] {
        game.resolve_selection(0,CARD_CHOICE_NEXT).unwrap();
        let owner=game.pending_selection.as_ref().unwrap().card_choice.as_ref().unwrap();
        assert_eq!(owner.valid_action_ids(game),vec![CARD_CHOICE_START]);
        assert!(owner.view(game,1).rows.iter().all(|row|row.card_id.is_none()&&!row.identity_visible));
        game.resolve_selection(0,CARD_CHOICE_START).unwrap();assert_eq!(game.memory,7);
    }
}

#[test]
fn impossible_required_source_prefix_offers_finish_and_never_runs_paid_tail() {
    use digimon_dsl::compiled::{CompiledBindingRef,CompiledCountBound,CompiledPredicate,CompiledStep};
    use digimon_engine::dsl_cards::{bindings::Bindings,step::run_steps};
    let mut r=runner();let host=r.place_stack(0,&["RECOVERY-SOURCE","RECOVERY-SOURCE","RECOVERY-SOURCE","RECOVERY-TOP"]);
    let top=r.game.player(0).battle_area[0].top_card().handle();
    let mut bindings=Bindings::new();bindings.insert_permanent("host",host);
    run_steps(&[CompiledStep::SelectMaterials{named_groups:Vec::new(),require_max:false,
        of_permanent:CompiledBindingRef::Named("host".into()),min:3,max:CompiledCountBound::Literal(3),
        filter:CompiledPredicate::default(),uniqueness:None,bind_as:Some("picked".into()),prompt:"Three".into(),
        prompt_key:None,optional_zero:false,same_by:None},CompiledStep::GainMemory(4)],
        &mut EffectContext::new(&mut r.game,top,Some(host),0),&mut bindings);
    r.game.resolve_selection(0,CARD_CHOICE_START).unwrap();
    r.game.players[0].battle_area[0].card_sources.remove(1);
    let mut clone=r.game.clone();
    for game in [&mut r.game,&mut clone] {
        let mask=build_action_mask(game,0);assert_eq!(mask[PASS as usize],1.0);assert_eq!(mask[CARD_CHOICE_START as usize],0.0);
        assert!(matches!(game.pending_selection.as_ref().unwrap().card_choice.as_ref().unwrap().view(game,0).rows[0].operation,
            digimon_engine::owned_card_choice::CardChoiceOperation::FinishImpossible));
        game.resolve_selection(0,PASS).unwrap();assert!(game.pending_selection.is_none());assert_eq!(game.memory,3);
    }
}

#[test]
fn impossible_named_role_pool_finishes_even_when_enough_physical_cards_remain() {
    use digimon_dsl::compiled::{CompiledBindingRef,CompiledCountBound,CompiledPredicate,CompiledStep};
    use digimon_engine::dsl_cards::{bindings::Bindings,step::run_steps};
    let mut r=runner();let host=r.place_stack(0,&["RECOVERY-SOURCE","RECOVERY-SOURCE","RECOVERY-TOP","RECOVERY-TOP"]);
    let top=r.game.player(0).battle_area[0].top_card().handle();
    run_steps(&[CompiledStep::SelectMaterials{named_groups:vec!["Source".into(),"Top".into()],require_max:true,
        of_permanent:CompiledBindingRef::Source,min:2,max:CompiledCountBound::Literal(2),
        filter:CompiledPredicate::default(),uniqueness:None,bind_as:Some("picked".into()),prompt:"Each role".into(),
        prompt_key:None,optional_zero:false,same_by:None},CompiledStep::GainMemory(4)],
        &mut EffectContext::new(&mut r.game,top,Some(host),0),&mut Bindings::new());
    r.game.players[0].battle_area[0].card_sources.remove(2);
    assert_eq!(build_action_mask(&r.game,0)[PASS as usize],1.0);
    assert_eq!(build_action_mask(&r.game,0)[CARD_CHOICE_START as usize],0.0);
    r.game.resolve_selection(0,PASS).unwrap();assert!(r.game.pending_selection.is_none());assert_eq!(r.game.memory,3);
}


#[test]
fn raw_capped_material_replaces_a_moved_prefix_before_counting_payment() {
    use digimon_engine::effect_context::CountCappedZone;
    let mut r=runner();let host=r.place_stack(0,&["RECOVERY-SOURCE","RECOVERY-SOURCE","RECOVERY-SOURCE","RECOVERY-TOP"]);
    let other=r.place_stack(0,&["RECOVERY-TOP"]);let top=r.game.player(0).battle_area[0].top_card().handle();
    let moved=r.game.player(0).battle_area[0].card_sources[0].handle();
    EffectContext::new(&mut r.game,top,Some(host),0).select_count_capped_multi_min(0,CountCappedZone::Material(host),
        2,2,"Two live original materials",false,None,|_,_|true,move|ctx,cards| {
            assert_eq!(cards.len(),2);assert!(!cards.contains(&moved),"a moved prefix cannot pay");ctx.game.memory=7;
        });
    r.game.resolve_selection(0,CARD_CHOICE_START).unwrap();
    let source=r.game.players[0].battle_area[0].card_sources.remove(0);
    r.game.players[0].battle_area[other.index as usize].card_sources.insert(0,source);
    r.game.resolve_selection(0,CARD_CHOICE_START).unwrap();assert_eq!(r.game.memory,3);
    assert!(r.game.pending_selection.is_some());
    r.game.resolve_selection(0,CARD_CHOICE_START).unwrap();assert_eq!(r.game.memory,7);
}

#[test]
fn actual_raw_play_pipeline_preserves_paid_discount_after_impossible_current_payment() {
    use digimon_engine::{card_source::CardHandle, effect::{CardEffect, Effect, EffectBuilder},
        enums::EffectTiming};
    struct Reducer;
    impl CardEffect for Reducer {
        fn effects(&self, card: CardHandle) -> Vec<Effect> {
            vec![EffectBuilder::new(card, EffectTiming::BeforePayCost)
                    .name("Earlier paid reduction").cost_reduction(2).build(),
                 EffectBuilder::new(card, EffectTiming::BeforePayCost)
                    .name("Current security payment").cost_reduction(4)
                    .pay_cost_fn(|ctx| {
                        ctx.select_security(0, "Pay one security", false, |_, _| true,
                            |_, _| panic!("an impossible payment must not commit"));
                        false
                    }).build()]
        }
    }
    let mut target = make_test_card("RAW-RECOVERY-TARGET", "Recovery target");
    target.play_cost = 10;
    let mut r = DebugRunner::builder().add_card(target)
        .add_card(make_test_card("RAW-RECOVERY-REDUCER", "Reducer"))
        .hand(0, &["RAW-RECOVERY-TARGET"]).security(0, &["RAW-RECOVERY-TARGET"])
        .memory(10).start();
    r.skip_mulligan();
    r.register_effect("RAW-RECOVERY-REDUCER", std::sync::Arc::new(Reducer));
    r.place_stack(0, &["RAW-RECOVERY-REDUCER"]);
    assert_eq!(r.play(0, 0), None, "cost effects must finish before the target enters play");
    for _ in 0..8 {
        let pending = r.game.pending_selection.as_ref().expect("cost selection remains live");
        if pending.card_choice.is_some() { break; }
        let first = pending.effect_choices.as_ref().and_then(|rows| rows.iter()
            .find(|row| row.label.contains("Earlier paid reduction")))
            .map(|row| row.action_id)
            .unwrap_or_else(|| *pending.valid_action_ids.iter().find(|&&id| id != PASS).unwrap());
        r.execute_action(0, first).unwrap();
    }
    assert!(r.game.pending_selection.as_ref().unwrap().card_choice.is_some());
    assert_eq!(r.game.memory, 10, "the play has not paid memory yet");
    r.game.player_mut(0).security.clear();
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 1.0);
    r.execute_action(0, PASS).unwrap();
    assert_eq!(r.game.memory, 2, "pay 10 minus only the earlier completed discount of 2");
    assert!(r.game.player(0).battle_area.iter()
        .any(|permanent| permanent.top_card().card_id(&r.game.card_data) == "RAW-RECOVERY-TARGET"));
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn same_level_choice_masks_a_depleted_group_while_other_groups_remain_payable() {
    use digimon_engine::{effect_context::CountCappedZone,DistinctByMode};
    let mut low=make_test_card("RECOVERY-L3","Low");low.level=Some(3);
    let mut high=make_test_card("RECOVERY-L4","High");high.level=Some(4);
    let mut r=DebugRunner::builder().add_card(low).add_card(high).add_card(make_test_card("RECOVERY-TOP","Top"))
        .deck(0,&["RECOVERY-L3";20]).deck(1,&["RECOVERY-L3";20]).memory(3).start();r.skip_mulligan();
    let host=r.place_stack(0,&["RECOVERY-L3","RECOVERY-L3","RECOVERY-L4","RECOVERY-L4","RECOVERY-TOP"]);
    let top=r.game.player(0).battle_area[0].top_card().handle();
    EffectContext::new(&mut r.game,top,Some(host),0).select_count_capped_multi_min(0,CountCappedZone::Material(host),
        2,2,"Same level",false,Some(DistinctByMode::SameLevel),|_,_|true,|ctx,cards| {
            assert_eq!(cards.len(),2);assert!(cards.iter().all(|&card|ctx.game.card_data_for_handle(card).unwrap().level==Some(4)));
            ctx.game.memory=7;
        });
    r.game.players[0].battle_area[0].card_sources.remove(0);
    let mask=build_action_mask(&r.game,0);assert_eq!(mask[CARD_CHOICE_START as usize+1],0.0);
    assert_eq!(mask[CARD_CHOICE_START as usize+2],1.0);assert_eq!(mask[CARD_CHOICE_START as usize+3],1.0);
    r.game.resolve_selection(0,CARD_CHOICE_START+2).unwrap();
    r.game.resolve_selection(0,CARD_CHOICE_START).unwrap();assert_eq!(r.game.memory,7);
}

#[test]
fn raw_partition_impossible_role_pool_exposes_finish_instead_of_dead_end_commits() {
    use digimon_engine::effect_context::PartitionRequirement;
    let mut r=runner();let host=r.place_stack(0,&["RECOVERY-SOURCE","RECOVERY-SOURCE","RECOVERY-TOP","RECOVERY-TOP"]);
    let top=r.game.player(0).battle_area[0].top_card().handle();
    EffectContext::new(&mut r.game,top,Some(host),0).select_partition_sources(host,"One of each",
        vec![PartitionRequirement::new("Source",|game,source|game.card_data_for_handle(source.card).is_some_and(|data|data.card_name=="Source")),
             PartitionRequirement::new("Top",|game,source|game.card_data_for_handle(source.card).is_some_and(|data|data.card_name=="Top"))],
        |_,_|panic!("an impossible printed role pool cannot pay"));
    r.game.players[0].battle_area[0].card_sources.remove(2);
    let mask=build_action_mask(&r.game,0);assert_eq!(mask[PASS as usize],1.0);
    assert_eq!(mask[CARD_CHOICE_START as usize],0.0);r.game.resolve_selection(0,PASS).unwrap();
    assert!(r.game.pending_selection.is_none());assert_eq!(r.game.memory,3);
}
