// Official BT12-092 Q2229: an activated effect of a Tamer also treated
// as a Digimon is both a Digimon effect and a Tamer effect.
use digimon_engine::{debug_runner::DebugRunner,effect_context::{EffectContext,EffectReadContext},enums::{EffectSourceKind,Expiry},modifiers::ModifierEntry,permanent::PermanentHandle};
fn setup(treated:bool)->(DebugRunner,PermanentHandle){ setup_with_extra(treated,None) }
fn setup_with_extra(treated:bool, extra:Option<&str>)->(DebugRunner,PermanentHandle){
 let mut b=DebugRunner::builder();if let Some(yaml)=extra {b=b.from_dsl_yaml(yaml).unwrap();}for id in ["BT12-092","ST1-03","ST1-04"] {b=b.dsl_card(id).unwrap();}
 let mut r=b.deck(0,&["ST1-04";12]).deck(1,&["ST1-04";12]).memory(10).start();r.skip_mulligan();
 let h=r.place_on_field(0,"BT12-092",Some(0));r.place_on_field(0,"ST1-03",Some(0));r.game.enter_main_phase();
 if treated {r.accept_optional_trigger().unwrap();assert_eq!(r.effective_dp(h),Some(3000));}else{r.decline_optional_trigger().unwrap();}
 (r,h)
}
#[test]fn q2229_actual_marcus_context_is_both_effect_kinds(){
 let (mut r,h)=setup(true);let card=r.top_card(h);
 {let c=EffectReadContext::new(&r.game,card,Some(h),0);assert!(c.source_is_tamer());assert!(c.source_is_digimon());assert!(!c.source_is_option());}
 let c=EffectContext::new(&mut r.game,card,Some(h),0);assert!(c.source_is_tamer());assert!(c.source_is_digimon());
}
#[test]fn ordinary_and_expired_marcus_are_only_tamer_effects(){
 for treated in [false,true]{let(mut r,h)=setup(treated);if treated{r.game.modifiers.expire_end_of_turn(0);}let c=EffectReadContext::new(&r.game,r.top_card(h),Some(h),0);assert!(c.source_is_tamer());assert!(!c.source_is_digimon());}
}
fn immunity(kind:EffectSourceKind,blocked:bool){
 let(mut r,h)=setup(true);let target=r.place_on_field(1,"ST1-04",Some(0));
 r.game.modifiers.add(target,ModifierEntry::cannot_be_affected_by_opponents_source_kind(kind,Expiry::EndOfTurn,1));
 let card=r.top_card(h);let saved=r.game.clone();
 for _ in 0..2 {r.game=saved.clone();let mut c=EffectContext::new(&mut r.game,card,Some(h),0);assert_eq!(c.can_affect_permanent(target),!blocked);c.suspend(target);assert_eq!(c.game.players[1].battle_area[0].is_suspended,!blocked);}
}
#[test]fn actual_treated_marcus_effect_is_blocked_by_digimon_effect_immunity(){immunity(EffectSourceKind::Digimon,true);}
#[test]fn actual_treated_marcus_effect_is_blocked_by_tamer_effect_immunity(){immunity(EffectSourceKind::Tamer,true);}
#[test]fn actual_treated_marcus_effect_is_not_blocked_by_option_effect_immunity(){immunity(EffectSourceKind::Option,false);}
#[test]fn real_marcus_suspend_clause_captures_both_kinds_in_pending_selection(){
 let(mut r,h)=setup(true);r.game.suspend(h);let kind=r.pending_selection().expect("actual optional evolution clause").source_kind;let card=r.top_card(h);
 for g in [&r.game,&r.game.clone()]{let c=EffectReadContext::new_with_source_kind(g,card,Some(h),kind,0);assert!(c.source_is_tamer());assert!(c.source_is_digimon());}
}
#[test]fn activated_effect_capture_survives_expiry_and_source_departure(){
 let(mut r,h)=setup(true);r.game.suspend(h);r.accept_optional_trigger().unwrap();let kind=r.pending_selection().expect("activated clause target chooser").source_kind;let card=r.top_card(h);let saved=r.game.clone();
 for leave in [false,true]{r.game=saved.clone();r.auto_resolve().unwrap();assert!(r.pending_selection().is_none());if leave{assert!(r.game.return_to_hand(h).is_some());}else{r.game.modifiers.expire_end_of_turn(0);}let c=EffectReadContext::new_with_source_kind(&r.game,card,None,kind,0);assert!(c.source_is_tamer());assert!(c.source_is_digimon());}
}
#[test]fn card_zone_inference_does_not_borrow_another_live_marcus_identity(){
 let(mut r,_)=setup(true);let i=r.add_to_hand(0,"BT12-092");let card=r.game.players[0].hand[i].handle();let c=EffectReadContext::new(&r.game,card,None,0);assert!(c.source_is_tamer());assert!(!c.source_is_digimon());
}

// Source-kind immunity controls invoke a generic mutation through the actual
// treated host's EffectContext; they do not invent a printed Marcus suspend effect.
use std::sync::{Arc,Mutex};
struct OriginProbe{seen:Arc<Mutex<Vec<(bool,bool,bool)>>>,linked:bool}
impl digimon_engine::effect::CardEffect for OriginProbe {
 fn effects(&self,c:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect>{let seen=self.seen.clone();let b=digimon_engine::effect::Effect::when_attacking(c);let b=if self.linked{b.linked()}else{b.inherited()};vec![b.process(move|ctx|seen.lock().unwrap().push((ctx.source_is_digimon(),ctx.source_is_tamer(),ctx.source_is_option()))).build()]}
}
fn origin_control_with_host(linked:bool, dual:bool){
 let mut source=digimon_engine::debug_runner::make_test_card("ORIGIN-TAMER","Origin Tamer");source.card_kind=digimon_engine::enums::CardKind::Tamer;source.dp=None;source.level=None;
 let mut r=DebugRunner::builder().add_card(source).dsl_card("BT12-092").unwrap().dsl_card("ST1-03").unwrap().deck(0,&["ST1-03";12]).deck(1,&["ST1-03";12]).memory(10).start();r.skip_mulligan();let seen=Arc::new(Mutex::new(vec![]));r.register_effect("ORIGIN-TAMER",Arc::new(OriginProbe{seen:seen.clone(),linked}));
 let h=r.place_on_field(0,if dual {"BT12-092"} else {"ST1-03"},Some(0));r.place_on_field(0,"ST1-03",Some(0));if dual {r.game.enter_main_phase();r.accept_optional_trigger().unwrap();}
 if linked {let i=r.add_to_hand(0,"ORIGIN-TAMER");let card=r.game.players[0].hand.remove(i);r.game.players[0].battle_area[0].linked_cards.push(card);}else{r.push_source(h,"ORIGIN-TAMER");}
 r.game.enqueue_triggered(digimon_engine::enums::EffectTiming::WhenAttacking,digimon_engine::selection::TriggerSource::Permanent(h));r.game.drain_effect_queue();assert_eq!(*seen.lock().unwrap(),vec![(true,false,false)],"inherited/link origin is Digimon-only on either ordinary or dual-kind carrier");
}
fn origin_control(linked:bool){origin_control_with_host(linked,true);}
#[test]fn inherited_tamer_source_is_digimon_only_on_dual_kind_carrier(){origin_control(false);}
#[test]fn linked_tamer_source_is_digimon_only_on_dual_kind_carrier(){origin_control(true);}
#[test]fn explicit_option_effect_override_is_not_promoted_by_field_identity(){
 let(r,h)=setup(true);let c=EffectReadContext::new_with_source_kind(&r.game,r.top_card(h),Some(h),EffectSourceKind::Option,0);assert!(c.source_is_option());assert!(!c.source_is_tamer());assert!(!c.source_is_digimon());
}

#[test]fn captured_dual_origin_has_distinct_digest_from_tamer_only_and_clones(){
 let(mut r,h)=setup(true);r.game.suspend(h);let saved=r.game.clone();assert_eq!(saved.verification_digest(),saved.clone().verification_digest());let mut wrong=saved.clone();wrong.pending_selection.as_mut().unwrap().source_kind=EffectSourceKind::Tamer;assert_ne!(saved.verification_digest(),wrong.verification_digest(),"different source immunity semantics must remain hash-distinct");
}
#[test]fn ordinary_source_kind_debug_contract_remains_unchanged(){
 for (kind,text) in [(EffectSourceKind::Digimon,"Digimon"),(EffectSourceKind::Tamer,"Tamer"),(EffectSourceKind::Option,"Option"),(EffectSourceKind::Rule,"Rule")] {assert_eq!(format!("{kind:?}"),text);}
}

// Model a second simultaneous effect changing the carrier between triggering
// and activation, without claiming that a player may mutate a parked decision.
fn queued_then_identity_changes(gain:bool){
 let(mut r,h)=setup(!gain);let card=r.top_card(h);
 // Model the state/event halves of Game::enqueue_on_suspend without draining,
 // so another simultaneous effect can change identity before this activates.
 r.game.players[0].battle_area[h.index as usize].is_suspended=true;
 r.game.enqueue_triggered(digimon_engine::enums::EffectTiming::OnSuspend,digimon_engine::selection::TriggerSource::EventObserved{player:0,permanent:h,card,effect_initiated:true});
 if gain {r.game.modifiers.add(h,ModifierEntry::simple(digimon_engine::enums::ModifierType::TreatAsDigimon,0,Expiry::EndOfTurn,0).with_payload(digimon_engine::modifiers::ModifierPayload::AdditiveSynthIdentity{kind:digimon_engine::enums::CardKind::Digimon,level:0,colors:vec![],traits:vec![],dp:3000}));}else{r.game.modifiers.expire_end_of_turn(0);}
 r.game.drain_effect_queue();r.accept_optional_trigger().unwrap();let kind=r.pending_selection().expect("actual activated Marcus target chooser").source_kind;let c=EffectReadContext::new_with_source_kind(&r.game,card,Some(h),kind,0);assert!(c.source_is_tamer());assert_eq!(c.source_is_digimon(),gain,"classification follows activation-time field identity");
}
#[test]fn native_effect_gains_dual_kind_between_trigger_and_activation(){queued_then_identity_changes(true);}
#[test]fn native_effect_loses_dual_kind_between_trigger_and_activation(){queued_then_identity_changes(false);}

fn physical_source_context(linked: bool) {
    let (mut r, h) = setup(true);
    let index = r.add_to_hand(0, "BT12-092");
    let source = r.game.players[0].hand.remove(index);
    let card = source.handle();
    if linked {
        r.game.players[0].battle_area[h.index as usize].linked_cards.push(source);
    } else {
        r.game.players[0].battle_area[h.index as usize].card_sources.insert(0, source);
    }
    for game in [&r.game, &r.game.clone()] {
        let ctx = EffectReadContext::new(game, card, Some(h), 0);
        assert!(ctx.source_is_digimon(), "15-3-2 physical inherited/link origin");
        assert!(!ctx.source_is_tamer());
        assert!(!ctx.source_is_option());
    }
}
#[test]
fn inherited_physical_tamer_context_is_digimon_only() { physical_source_context(false); }
#[test]
fn linked_physical_tamer_context_is_digimon_only() { physical_source_context(true); }

// These synthetic complete-metadata carriers test the continuation capability;
// they do not add an invented effect to Marcus's printed card implementation.
struct ParkedOriginProbe {
    observed: Arc<Mutex<Vec<EffectSourceKind>>>,
    cost: bool,
}
impl digimon_engine::effect::CardEffect for ParkedOriginProbe {
    fn effects(&self, card: digimon_engine::card_source::CardHandle) -> Vec<digimon_engine::effect::Effect> {
        let observed = self.observed.clone();
        let effect = digimon_engine::effect::Effect::on_play(card);
        if self.cost {
            vec![effect.pay_cost(|ctx| {
                ctx.select_opponent_permanent("Pay cost choice", false, |_, _| true, |_, _| {});
                true
            }).process(move |ctx| observed.lock().unwrap().push(ctx.source_kind())).build()]
        } else {
            vec![effect.process(move |ctx| {
                let observed = observed.clone();
                ctx.select_opponent_permanent("Captured body choice", false, |_, _| true,
                    move |ctx, target| {
                        observed.lock().unwrap().push(ctx.source_kind());
                        ctx.suspend(target);
                    });
            }).build()]
        }
    }
}
fn parked_origin(cost: bool, leave: bool) {
    use digimon_engine::enums::{CardKind, ModifierType, EffectTiming};
    let mut source = digimon_engine::debug_runner::make_test_card("PARKED-DUAL", "Parked Dual Origin");
    source.card_kind = CardKind::Tamer; source.dp = None; source.level = None;
    let mut r = DebugRunner::builder().add_card(source).dsl_card("ST1-03").unwrap()
        .deck(0, &["ST1-03"; 12]).deck(1, &["ST1-03"; 12]).memory(5).start();
    r.skip_mulligan();
    let observed = Arc::new(Mutex::new(Vec::new()));
    r.register_effect("PARKED-DUAL", Arc::new(ParkedOriginProbe{observed:observed.clone(), cost}));
    let host = r.place_on_field(0, "PARKED-DUAL", Some(0));
    let target = r.place_on_field(1, "ST1-03", Some(0));
    r.game.modifiers.add(host, ModifierEntry::simple(ModifierType::TreatAsDigimon,0,Expiry::EndOfTurn,0)
        .with_payload(digimon_engine::modifiers::ModifierPayload::AdditiveSynthIdentity {
            kind: CardKind::Digimon, level:0, colors:vec![], traits:vec![], dp:3000 }));
    r.game.modifiers.add(target, ModifierEntry::cannot_be_affected_by_opponents_source_kind(
        EffectSourceKind::Digimon, Expiry::Permanent, 1));
    // Legacy native callbacks are intentionally not cloneable. Clone before
    // activation, then independently install and resolve the real callback.
    // Typed parked-selection cloning is covered by the real Marcus controls.
    let before_activation = r.game.clone();
    let mut digest = None;
    for _ in 0..2 {
        observed.lock().unwrap().clear(); r.game = before_activation.clone();
        r.game.enqueue_triggered(EffectTiming::OnPlay, digimon_engine::selection::TriggerSource::Permanent(host));
        r.game.drain_effect_queue();
        if leave {
            // Park the outer chooser while exercising the real movement path;
            // no gameplay action is issued through an unrelated active chooser.
            let selection = r.game.pending_selection.take();
            let resume = r.game.pending_selection_resume.take();
            assert!(r.game.return_to_hand(host).is_some());
            assert!(r.game.pending_selection.is_none());
            r.game.pending_selection = selection;
            r.game.pending_selection_resume = resume;
        } else { r.game.modifiers.expire_end_of_turn(0); }
        let selection = r.pending_selection().unwrap();
        let action = selection.valid_action_ids[0]; let player = selection.selecting_player;
        assert_eq!(digimon_engine::action::build_action_mask(&r.game, player)[action as usize], 1.0);
        r.game.resolve_selection(player, action).unwrap();
        assert_eq!(*observed.lock().unwrap(), vec![EffectSourceKind::DigimonAndTamer]);
        assert!(!r.game.players[1].battle_area[target.index as usize].is_suspended,
            "resumed dual body retains Digimon-effect immunity after identity changes");
        let current = r.game.verification_digest();
        if let Some(previous) = digest { assert_eq!(current, previous); } digest = Some(current);
    }
}
#[test] fn q2229_followup_parked_body_retains_origin_after_expiry_and_departure() {
    parked_origin(false, false); parked_origin(false, true);
}
#[test] fn q2229_followup_parked_cost_retains_activated_origin_after_expiry() { parked_origin(true, false); }

#[test] fn q2229_followup_tamer_only_memory_gate_accepts_dual_and_rejects_other_origins() {
    use digimon_engine::{enums::ModifierType, modifiers::PlayerModifierEntry};
    let (mut r, host) = setup(true); let card = r.top_card(host);
    r.game.modifiers.add_player_modifier(0, PlayerModifierEntry::simple(
        ModifierType::CannotGainMemoryExceptFromTamers, 0, Expiry::Permanent, None, 1));
    for (kind, expected) in [(EffectSourceKind::DigimonAndTamer,2), (EffectSourceKind::Tamer,2),
        (EffectSourceKind::Digimon,0), (EffectSourceKind::Option,0)] {
        let mut g = r.game.clone(); g.set_memory(0);
        EffectContext::new_with_source_kind(&mut g, card, Some(host), kind, 0).gain_memory(2);
        assert_eq!(g.memory, expected, "Q5979 {kind:?}");
    }
}

#[test] fn q2229_followup_compound_metadata_does_not_change_pending_masks() {
    let (mut r, host) = setup(true); r.game.suspend(host);
    for choices in [false, true] {
        let mut dual = r.game.clone();
        if choices {
            let selection = dual.pending_selection.as_mut().unwrap();
            selection.effect_choices = Some(vec![digimon_engine::selection::EffectChoiceEntry {
                count_choice: None, label:"same offered choice".into(), action_id:selection.valid_action_ids[0],
                source_card:Some(selection.source_card), source_kind:Some(EffectSourceKind::DigimonAndTamer),
                timing:Some(digimon_engine::enums::EffectTiming::OnSuspend), is_optional:true,
                keyword:None, observation_metadata:Default::default(),
            }]);
        }
        let mut single = dual.clone(); let pending = single.pending_selection.as_mut().unwrap();
        pending.source_kind = EffectSourceKind::Tamer;
        if let Some(choices) = &mut pending.effect_choices { for choice in choices { choice.source_kind = Some(EffectSourceKind::Tamer); } }
        assert_eq!(digimon_engine::action::build_action_mask(&dual,0), digimon_engine::action::build_action_mask(&single,0));
        assert_ne!(dual.verification_digest(), single.verification_digest());
        if choices {
            let cards = dual.card_data.iter().map(|card| (card.card_id.clone(), card.clone())).collect();
            let registry = digimon_engine::card_registry::CardRegistry::from_cards(&cards);
            let encode = |game: &digimon_engine::game::Game| digimon_engine::observation::build_observation_tensor(
                game, 0, &registry, digimon_engine::observation::parse_observation_profile("standard_lite_v2").unwrap());
            let compound = encode(&dual); let ordinary = encode(&single);
            let offset = digimon_engine::tensor_profiles::standard::v2_lite::OFF_PENDING_CHOICE_FEATURES + 34;
            assert_eq!(&compound[offset..offset+4], &[0.0,0.0,0.0,0.0]);
            assert_eq!(&ordinary[offset..offset+4], &[0.0,1.0,0.0,0.0]);
            assert_eq!(compound.len(), ordinary.len());
        }
    }
}

#[test] fn q2229_followup_category_membership_truth_table_and_queued_digest() {
    use EffectSourceKind::*;
    let kinds = [Digimon,Tamer,Option,Rule,DigimonAndTamer];
    let expected = [
        [true,false,false,false,false],
        [false,true,false,false,false],
        [false,false,true,false,false],
        [false,false,false,true,false],
        [true,true,false,false,true],
    ];
    for (i, actual) in kinds.iter().enumerate() {
        for (j, queried) in kinds.iter().enumerate() {
            assert_eq!(actual.includes(*queried), expected[i][j]);
        }
    }
    let (mut r, host) = setup(true); let card = r.top_card(host);
    r.game.enqueue_triggered(digimon_engine::enums::EffectTiming::OnSuspend,
        digimon_engine::selection::TriggerSource::EventObserved {player:0, permanent:host, card, effect_initiated:true});
    assert_eq!(r.game.effect_queue.front().unwrap().source_kind, DigimonAndTamer);
    let mut single = r.game.clone(); single.effect_queue.front_mut().unwrap().source_kind=Tamer;
    assert_ne!(r.game.verification_digest(), single.verification_digest());
}

#[test] fn q2229_followup_ordinary_carrier_inherited_and_link_sources_are_digimon_only(){origin_control_with_host(false,false);origin_control_with_host(true,false);}


struct RemovedOriginProbe(Arc<Mutex<Vec<EffectSourceKind>>>);
impl digimon_engine::effect::CardEffect for RemovedOriginProbe {
 fn effects(&self,c:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect>{
  let seen=self.0.clone(); vec![digimon_engine::effect::Effect::on_deletion(c).process(move|ctx|{
   assert!(ctx.game.players[0].battle_area.is_empty(), "actual post-removal processing");
   seen.lock().unwrap().push(ctx.source_kind());
  }).build()]
 }
}
#[test] fn q2229_boundary_deletion_retains_dual_native_origin() {
 use digimon_engine::enums::{CardKind,ModifierType};
 let mut data=digimon_engine::debug_runner::make_test_card("DELETED-DUAL","Deleted Dual");
 data.card_kind=CardKind::Tamer;data.level=None;data.dp=None;
 let mut r=DebugRunner::builder().add_card(data).dsl_card("ST1-03").unwrap()
 .deck(0,&["ST1-03";12]).deck(1,&["ST1-03";12]).memory(5).start();r.skip_mulligan();
 let seen=Arc::new(Mutex::new(vec![]));r.register_effect("DELETED-DUAL",Arc::new(RemovedOriginProbe(seen.clone())));
 let h=r.place_on_field(0,"DELETED-DUAL",Some(0));
 r.game.modifiers.add(h,ModifierEntry::simple(ModifierType::TreatAsDigimon,0,Expiry::EndOfTurn,0)
 .with_payload(digimon_engine::modifiers::ModifierPayload::AdditiveSynthIdentity{kind:CardKind::Digimon,level:0,colors:vec![],traits:vec![],dp:3000}));
 let before=r.game.clone(); let mut digest=None;
 for _ in 0..2 {r.game=before.clone();seen.lock().unwrap().clear();r.game.delete_permanent_with_effects(h);
 assert_eq!(*seen.lock().unwrap(),vec![EffectSourceKind::DigimonAndTamer]);
 let d=r.game.verification_digest();if let Some(old)=digest{assert_eq!(old,d);}digest=Some(d);}
}
struct OptionInheritedProbe(Arc<Mutex<Vec<EffectSourceKind>>>);
impl digimon_engine::effect::CardEffect for OptionInheritedProbe {
 fn effects(&self,c:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect>{
 let seen=self.0.clone();vec![digimon_engine::effect::Effect::when_attacking(c).inherited().process(move|ctx|{
 seen.lock().unwrap().push(ctx.source_kind());ctx.suspend(digimon_engine::permanent::PermanentHandle{player:1,index:0});
 }).build()]}
}
#[test] fn q2229_boundary_lower_option_inherited_is_not_blocked_by_option_immunity(){
 use digimon_engine::enums::CardKind;
 let mut data=digimon_engine::debug_runner::make_test_card("LOWER-OPTION","Lower Option");data.card_kind=CardKind::Option;data.dp=None;data.level=None;
 let mut r=DebugRunner::builder().add_card(data).dsl_card("ST1-03").unwrap().deck(0,&["ST1-03";12]).deck(1,&["ST1-03";12]).memory(5).start();r.skip_mulligan();
 let seen=Arc::new(Mutex::new(vec![]));r.register_effect("LOWER-OPTION",Arc::new(OptionInheritedProbe(seen.clone())));
 let host=r.place_on_field(0,"ST1-03",Some(0));r.push_source(host,"LOWER-OPTION");let target=r.place_on_field(1,"ST1-03",Some(0));
 r.game.modifiers.add(target,ModifierEntry::cannot_be_affected_by_opponents_source_kind(EffectSourceKind::Option,Expiry::Permanent,1));
 r.game.enqueue_triggered(digimon_engine::enums::EffectTiming::WhenAttacking,digimon_engine::selection::TriggerSource::Permanent(host));r.game.drain_effect_queue();
 assert_eq!(*seen.lock().unwrap(),vec![EffectSourceKind::Digimon]);assert!(r.game.players[1].battle_area[0].is_suspended);
}
#[test] fn q2229_boundary_reverse_additive_category_keeps_both_memberships(){
 use digimon_engine::enums::{CardKind,ModifierType};
 let (mut r,_)=setup(false);let h=r.place_on_field(0,"ST1-03",Some(0));
 r.game.modifiers.add(h,ModifierEntry::simple(ModifierType::TreatAsDigimon,0,Expiry::EndOfTurn,0)
 .with_payload(digimon_engine::modifiers::ModifierPayload::AdditiveSynthIdentity{kind:CardKind::Tamer,level:0,colors:vec![],traits:vec![],dp:3000}));
 let c=EffectReadContext::new(&r.game,r.top_card(h),Some(h),0);assert!(c.source_is_digimon());assert!(c.source_is_tamer());assert!(!c.source_is_option());
 r.game.modifiers.expire_end_of_turn(0);let c=EffectReadContext::new(&r.game,r.top_card(h),Some(h),0);assert!(c.source_is_digimon());assert!(!c.source_is_tamer());
}

#[test] fn q2229_boundary_granted_effect_uses_receiving_dual_carrier_category(){
 let (mut r,host)=setup(true);let seen=Arc::new(Mutex::new(vec![]));let witness=seen.clone();
 let grantor=digimon_engine::permanent::PermanentHandle{player:0,index:1};let card=r.top_card(grantor);
 EffectContext::new(&mut r.game,card,Some(grantor),0).grant_triggered_effect(host,
 digimon_engine::enums::EffectTiming::WhenAttacking,Expiry::EndOfTurn,move|ctx|witness.lock().unwrap().push(ctx.source_kind()));
 r.game.enqueue_triggered(digimon_engine::enums::EffectTiming::WhenAttacking,digimon_engine::selection::TriggerSource::Permanent(host));r.game.drain_effect_queue();
 assert_eq!(*seen.lock().unwrap(),vec![EffectSourceKind::DigimonAndTamer]);
}

#[test] fn q2229_boundary_granted_keyword_body_retains_dual_category(){
 const SAVE_AURA:&str=r#"
card: TEST-SAVE-GRANTER
name: Keyword Category Probe
kind: tamer
color: [white]
cost: 2
effects:
  - kind: aura
    target: {of: you, kind: digimon, card_number_is: BT12-092}
    grant_keyword: {keyword: Save}
"#;
 let (mut r,host)=setup_with_extra(true,Some(SAVE_AURA));
 r.place_on_field(0,"TEST-SAVE-GRANTER",Some(0));r.game.tick_declarative_effects();
 r.game.delete_permanent_with_effects(host);
 assert_eq!(r.pending_kind(),Some(digimon_engine::selection::SelectionKind::OwnField));
 assert_eq!(r.pending_selection().expect("eligible granted Save body").source_kind,EffectSourceKind::DigimonAndTamer);
 let parked=r.game.clone();let mut digest=None;
 for _ in 0..2{r.game=parked.clone();let p=r.pending_selection().unwrap().selecting_player;
 assert_eq!(digimon_engine::action::build_action_mask(&r.game,p)[digimon_engine::action::space::PASS as usize],1.0);
 r.game.resolve_selection(p,digimon_engine::action::space::PASS).unwrap();
 assert_eq!(r.game.players[0].battle_area.len(),2);assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data),"ST1-03");
 let d=r.game.verification_digest();if let Some(old)=digest{assert_eq!(old,d);}digest=Some(d);}
}
