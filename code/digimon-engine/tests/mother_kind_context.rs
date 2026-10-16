//! Card category remains printed in card bindings; field-event subjects use permanent identity.
use digimon_dsl::compiled::{
    CompiledBindingCardKindPredicate, CompiledCardKind, CompiledPredicate,
};
use digimon_engine::{
    action::{build_action_mask, space::PASS},
    debug_runner::DebugRunner,
    dsl_cards::{
        bindings::Bindings,
        predicate::{eval_predicate_with_bindings, PredicateSubject},
    },
    effect_context::EffectReadContext,
    selection::UnionZoneOrigin,
    trigger_context::TriggerContext,
};
fn base() -> DebugRunner {
    let mut token=digimon_engine::debug_runner::make_test_card("CONTEXT-TOKEN","Context Token");
    token.card_kind=digimon_engine::enums::CardKind::Token;token.dp=Some(3000);
    let mut b = DebugRunner::builder().add_card(token);
    for id in ["EX2-007", "EX13-044", "EX3-074", "ST1-04", "ST1-12", "ST1-16", "BT12-092", "ST1-03"] {
        b = b.dsl_card(id).unwrap();
    }
    let mut r = b
        .deck(0, &["ST1-04"; 12])
        .deck(1, &["ST1-04"; 12])
        .memory(5)
        .start();
    r.skip_mulligan();
    r
}
fn check(r: &DebugRunner, p: &CompiledPredicate, b: Option<&Bindings>) -> bool {
    let src = r.game.players[0].battle_area[0].top_card().handle();
    eval_predicate_with_bindings(
        p,
        &EffectReadContext::new(&r.game, src, None, 0),
        PredicateSubject::None,
        b,
    )
}
#[test]
fn material_card_bindings_do_not_promote_printed_egg_to_digimon() {
    let mut r = base();
    let host = r.place_on_field(0, "EX3-074", None);
    let egg = r.push_source(host, "EX2-007");
    for mode in 0..3 {
        let mut b = Bindings::default();
        match mode {
            0 => b.insert_card("pick", egg),
            1 => b.insert_card_list("pick", vec![egg]),
            _ => b.insert_union_card(
                "pick",
                egg,
                UnionZoneOrigin::Material {
                    carrier: host,
                    source_index: 0,
                },
                0,
            ),
        };
        let pred = CompiledPredicate {
            binding_card_kind: Some(CompiledBindingCardKindPredicate {
                binding: "pick".into(),
                kind: CompiledCardKind::Digimon,
            }),
            ..Default::default()
        };
        assert!(
            !check(&r, &pred, Some(&b)),
            "printed Digi-Egg in material card binding {mode} is not a Digimon card"
        );
        let egg_pred = CompiledPredicate {
            binding_card_kind: Some(CompiledBindingCardKindPredicate {
                binding: "pick".into(),
                kind: CompiledCardKind::DigiEgg,
            }),
            ..Default::default()
        };
        assert!(check(&r, &egg_pred, Some(&b)));
    }
}
#[test]
fn living_field_egg_is_digimon_event_target_in_clones() {
    let mut r = base();
    let h = r.place_on_field(0, "EX2-007", None);
    let top = r.top_card(h);
    r.game.current_trigger_context = Some({ let mut __fixture_trigger_context = TriggerContext::default(); __fixture_trigger_context.event_permanent = Some(h); __fixture_trigger_context.event_card = Some(top); __fixture_trigger_context });
    let saved = r.game.clone();
    for _ in 0..2 {
        r.game = saved.clone();
        assert!(check(
            &r,
            &CompiledPredicate {
                event_target_kind: Some(CompiledCardKind::Digimon),
                ..Default::default()
            },
            None
        ));
        assert!(
            !check(
                &r,
                &CompiledPredicate {
                    event_card_kind: Some(CompiledCardKind::Digimon),
                    ..Default::default()
                },
                None
            ),
            "event-card identity stays printed"
        );
    }
}
#[test]
fn source_card_event_and_tamer_permanent_are_not_digimon_targets() {
    let mut r = base();
    let host = r.place_on_field(0, "EX3-074", None);
    let egg = r.push_source(host, "EX2-007");
    r.game.current_trigger_context = Some({ let mut __fixture_trigger_context = TriggerContext::default(); __fixture_trigger_context.event_card = Some(egg); __fixture_trigger_context });
    let p = CompiledPredicate {
        event_target_kind: Some(CompiledCardKind::Digimon),
        ..Default::default()
    };
    assert!(!check(&r, &p, None));
    let t = r.place_on_field(0, "ST1-12", None);
    r.game.current_trigger_context = Some({ let mut __fixture_trigger_context = TriggerContext::default(); __fixture_trigger_context.event_permanent = Some(t); __fixture_trigger_context.event_card = Some(r.top_card(t)); __fixture_trigger_context });
    assert!(!check(&r, &p, None));
}
#[test]
fn digimon_card_binding_remains_positive() {
    let mut r = base();
    let h = r.place_on_field(0, "EX3-074", None);
    let mut b = Bindings::default();
    b.insert_card("pick", r.top_card(h));
    assert!(check(
        &r,
        &CompiledPredicate {
            binding_card_kind: Some(CompiledBindingCardKindPredicate {
                binding: "pick".into(),
                kind: CompiledCardKind::Digimon
            }),
            ..Default::default()
        },
        Some(&b)
    ));
}
#[test]
fn own_mother_suspension_triggers_real_breakdramon_optional_battle() {
    let mut r = base();
    r.place_on_field(0, "EX13-044", Some(0));
    r.place_on_field(0, "EX3-074", Some(0));
    let mother = r.place_on_field(0, "EX2-007", Some(0));
    r.place_on_field(1, "ST1-04", Some(0));
    r.game.suspend(mother);
    let v = r
        .pending_selection_view()
        .expect("Mother is a field Digimon suspension event");
    assert!(v.is_optional);
    assert_eq!(
        build_action_mask(&r.game, v.selecting_player)[PASS as usize],
        1.0
    );
    let actor = v.selecting_player;
    let saved = r.game.clone();
    for _ in 0..2 {
        r.game = saved.clone();
        r.game.resolve_selection(actor, PASS).unwrap();
        assert!(r.pending_selection().is_none());
        assert_eq!(r.battle_area_size(0), 3);
        assert_eq!(r.battle_area_size(1), 1);
    }
}


use std::sync::{Arc, Mutex};
use digimon_engine::{
    card_source::CardHandle,
    effect::{CardEffect, Effect},
    enums::{CardColor, CardKind, Expiry, ModifierType},
    modifiers::{ModifierEntry, ModifierPayload},
    permanent::PermanentHandle,
    replacement::ReplacementCause,
    trigger_context::DeletedObjectSnapshot,
};
fn treat(r: &mut DebugRunner, h: PermanentHandle) {
    r.game.modifiers.add(h, ModifierEntry::simple(ModifierType::TreatAsDigimon,0,Expiry::EndOfTurn,0)
        .with_payload(ModifierPayload::SynthIdentity {kind:CardKind::Digimon,level:3,colors:vec![CardColor::Red],traits:vec![],dp:3000}));
}
fn event_kind(r:&DebugRunner,kind:CompiledCardKind)->bool {
    check(r,&CompiledPredicate{event_target_kind:Some(kind),..Default::default()},None)
}
fn bind_kind(r:&DebugRunner,card:CardHandle,kind:CompiledCardKind)->bool {
    let mut b=Bindings::default();b.insert_card("pick",card);
    check(r,&CompiledPredicate{binding_card_kind:Some(CompiledBindingCardKindPredicate{binding:"pick".into(),kind}),..Default::default()},Some(&b))
}
#[test]
fn live_treated_tamer_and_expiry_use_effective_event_identity_but_printed_card_kind() {
    let mut r=base();let h=r.place_on_field(0,"ST1-12",Some(0));let card=r.top_card(h);treat(&mut r,h);
    r.game.current_trigger_context=Some({ let mut __fixture_trigger_context = TriggerContext::default(); __fixture_trigger_context.event_permanent = Some(h); __fixture_trigger_context.event_card = Some(card); __fixture_trigger_context });
    let saved=r.game.clone();
    for _ in 0..2 {
        r.game=saved.clone();assert!(event_kind(&r,CompiledCardKind::Digimon));
        assert!(bind_kind(&r,card,CompiledCardKind::Tamer));assert!(!bind_kind(&r,card,CompiledCardKind::Digimon));
        r.game.modifiers.expire_end_of_turn(0);
        assert!(!event_kind(&r,CompiledCardKind::Digimon));assert!(event_kind(&r,CompiledCardKind::Tamer));
    }
}
#[test]
fn field_egg_card_binding_stays_printed_and_hand_trash_eggs_are_not_digimon_cards() {
    let mut r=base();let h=r.place_on_field(0,"EX2-007",Some(0));let c=r.top_card(h);
    assert!(bind_kind(&r,c,CompiledCardKind::DigiEgg));assert!(!bind_kind(&r,c,CompiledCardKind::Digimon));
    let i=r.add_to_hand(0,"EX2-007");let c=r.game.players[0].hand[i].handle();
    assert!(!bind_kind(&r,c,CompiledCardKind::Digimon));assert!(bind_kind(&r,c,CompiledCardKind::DigiEgg));
    let card=r.game.players[0].hand.remove(i);r.game.players[0].trash.push(card);
    assert!(!bind_kind(&r,c,CompiledCardKind::Digimon));assert!(bind_kind(&r,c,CompiledCardKind::DigiEgg));
}
struct CaptureDeleted(Arc<Mutex<Vec<DeletedObjectSnapshot>>>);
impl CardEffect for CaptureDeleted {
    fn effects(&self,card:CardHandle)->Vec<Effect> {
        let seen=self.0.clone();vec![Effect::on_any_deletion(card).process(move|ctx| {
            seen.lock().unwrap().push(ctx.deleted_object_snapshot().expect("real dispatched deletion snapshot").clone());
        }).build()]
    }
}
fn assert_deleted_kind(id:&str,treated:bool,expired:bool,want:CompiledCardKind) {
    let mut r=base();let seen=Arc::new(Mutex::new(vec![]));
    r.register_effect("ST1-04",Arc::new(CaptureDeleted(seen.clone())));
    r.place_on_field(0,"ST1-04",Some(0));let h=r.place_on_field(0,id,Some(0));
    if treated {treat(&mut r,h);}
    if expired {r.game.modifiers.expire_end_of_turn(0);}
    let saved=r.game.clone();
    for _ in 0..2 {
        r.game=saved.clone();seen.lock().unwrap().clear();
        r.game.delete_permanent_with_cause(h,ReplacementCause::OwnEffect);
        let snapshots=seen.lock().unwrap().clone();assert_eq!(snapshots.len(),1);
        r.game.current_trigger_context=Some({ let mut __fixture_trigger_context = TriggerContext::default(); __fixture_trigger_context.deleted_object = Some(snapshots[0].clone()); __fixture_trigger_context });
        assert!(event_kind(&r,want),"deleted {id}, treated={treated}, expired={expired}");
        assert_eq!(r.game.verification_digest(),r.game.clone().verification_digest());
        if id=="ST1-12" {
            assert!(!check(&r,&CompiledPredicate{event_card_kind:Some(CompiledCardKind::Digimon),..Default::default()},None),"deleted card classification remains printed");
            if treated && !expired {}
        }
    }
}
#[test]
fn deleted_treated_tamer_retains_effective_digimon_identity() {assert_deleted_kind("ST1-12",true,false,CompiledCardKind::Digimon);}
#[test]
fn deleted_expired_tamer_remains_tamer() {assert_deleted_kind("ST1-12",true,true,CompiledCardKind::Tamer);}
#[test]
fn deleted_mother_remains_field_digimon() {assert_deleted_kind("EX2-007",false,false,CompiledCardKind::Digimon);}

#[test]
fn digimon_token_is_field_subject_but_not_printed_digimon_card() {
    let mut r=base();let h=r.place_on_field(0,"CONTEXT-TOKEN",Some(0));let c=r.top_card(h);
    r.game.current_trigger_context=Some({ let mut __fixture_trigger_context = TriggerContext::default(); __fixture_trigger_context.event_permanent = Some(h); __fixture_trigger_context.event_card = Some(c); __fixture_trigger_context });
    assert!(event_kind(&r,CompiledCardKind::Digimon));assert!(bind_kind(&r,c,CompiledCardKind::Token));
    assert!(!bind_kind(&r,c,CompiledCardKind::Digimon));
}
#[test]
fn option_and_tamer_cards_are_excluded_from_digimon_card_bindings() {
    let mut r=base();r.place_on_field(0,"ST1-04",Some(0));
    for id in ["ST1-12","ST1-16"] {let i=r.add_to_hand(0,id);assert!(!bind_kind(&r,r.game.players[0].hand[i].handle(),CompiledCardKind::Digimon));}
}
#[test]
fn captured_source_card_does_not_inherit_current_host_treated_identity() {
    let mut r=base();let h=r.place_on_field(0,"ST1-12",Some(0));let source=r.push_source(h,"ST1-12");treat(&mut r,h);
    r.game.current_trigger_context=Some({ let mut __fixture_trigger_context = TriggerContext::default(); __fixture_trigger_context.event_permanent = Some(h); __fixture_trigger_context.event_card = Some(source); __fixture_trigger_context });
    assert!(!event_kind(&r,CompiledCardKind::Digimon));assert!(event_kind(&r,CompiledCardKind::Tamer));
}

struct CaptureLeave(Arc<Mutex<Vec<DeletedObjectSnapshot>>>);
impl CardEffect for CaptureLeave {
    fn effects(&self,card:CardHandle)->Vec<Effect> {
        let seen=self.0.clone();vec![digimon_engine::effect::EffectBuilder::new(card,digimon_engine::enums::EffectTiming::OnLeaveField).process(move|ctx| {
            seen.lock().unwrap().push(ctx.deleted_object_snapshot().expect("real leaving snapshot").clone());
        }).build()]
    }
}
#[test]
fn returned_treated_tamer_retains_field_kind_but_card_query_is_printed() {
    let mut r=base();let seen=Arc::new(Mutex::new(vec![]));
    r.register_effect("ST1-04",Arc::new(CaptureLeave(seen.clone())));
    r.place_on_field(0,"ST1-04",Some(0));let h=r.place_on_field(0,"ST1-12",Some(0));treat(&mut r,h);
    let saved=r.game.clone();
    for _ in 0..2 {
        r.game=saved.clone();seen.lock().unwrap().clear();
        assert!(r.game.return_to_hand(h).is_some());
        let snapshots=seen.lock().unwrap().clone();assert_eq!(snapshots.len(),1);
        r.game.current_trigger_context=Some({ let mut __fixture_trigger_context = TriggerContext::default(); __fixture_trigger_context.deleted_object = Some(snapshots[0].clone()); __fixture_trigger_context });
        assert!(event_kind(&r,CompiledCardKind::Digimon));
        assert!(bind_kind(&r,snapshots[0].top_card,CompiledCardKind::Tamer));
    }
}

fn activate_real_marcus(r:&mut DebugRunner)->PermanentHandle {
    let h=r.place_on_field(0,"BT12-092",Some(0));r.place_on_field(0,"ST1-03",Some(0));
    r.game.enter_main_phase();
    r.accept_optional_trigger().expect("actual Marcus pay-1 optional activation");
    assert!(r.pending_selection().is_none());assert_eq!(r.effective_dp(h),Some(3000));h
}
#[test]
fn official_q2228_live_marcus_remains_both_digimon_and_tamer() {
    let mut r=base();let h=activate_real_marcus(&mut r);let card=r.top_card(h);
    r.game.current_trigger_context=Some({ let mut __fixture_trigger_context = TriggerContext::default(); __fixture_trigger_context.event_permanent = Some(h); __fixture_trigger_context.event_card = Some(card); __fixture_trigger_context });
    assert!(event_kind(&r,CompiledCardKind::Digimon));
    assert!(event_kind(&r,CompiledCardKind::Tamer),"Q2228: also treated means BOTH categories");
    let saved=r.game.clone();
    for _ in 0..2 {
        r.game=saved.clone();r.game.modifiers.expire_end_of_turn(0);
        assert!(!event_kind(&r,CompiledCardKind::Digimon));assert!(event_kind(&r,CompiledCardKind::Tamer));
    }
}
#[test]
fn official_q2228_deleted_marcus_retains_both_last_known_categories() {
    let mut r=base();let seen=Arc::new(Mutex::new(vec![]));r.register_effect("ST1-04",Arc::new(CaptureDeleted(seen.clone())));
    r.place_on_field(0,"ST1-04",Some(0));let h=activate_real_marcus(&mut r);
    r.game.delete_permanent_with_cause(h,ReplacementCause::OwnEffect);
    let snapshots=seen.lock().unwrap().clone();assert_eq!(snapshots.len(),1);
    r.game.current_trigger_context=Some({ let mut __fixture_trigger_context = TriggerContext::default(); __fixture_trigger_context.deleted_object = Some(snapshots[0].clone()); __fixture_trigger_context });
    assert!(event_kind(&r,CompiledCardKind::Digimon));assert!(event_kind(&r,CompiledCardKind::Tamer),"Q2228 last-known dual category");
    let clone=r.game.clone();assert_eq!(r.game.verification_digest(),clone.verification_digest());
    let mut wrong=clone.clone();wrong.current_trigger_context.as_mut().unwrap().deleted_object.as_mut().unwrap().retained_kind=None;
    assert_ne!(clone.verification_digest(),wrong.verification_digest(),"meaningful additional membership is hashed");
}

#[test]
fn official_q2228_source_host_queries_see_both_marcus_categories() {
    let mut r=base();let h=activate_real_marcus(&mut r);let c=r.push_source(h,"ST1-04");
    let subject=PredicateSubject::Source(digimon_engine::selection::SourceSelectionRef{permanent:h,field_index:h.index,source_index:0,card:c});
    let ctx=EffectReadContext::new(&r.game,r.top_card(h),None,0);
    for kind in [CompiledCardKind::Digimon,CompiledCardKind::Tamer] {
        assert!(eval_predicate_with_bindings(&CompiledPredicate{host_kind_is:Some(kind),..Default::default()},&ctx,subject,None),"Q2228 source host must retain {kind:?}");
    }
}

#[test]
fn replacement_identity_does_not_retain_printed_tamer_membership() {
    let mut r=base();let h=r.place_on_field(0,"ST1-12",Some(0));treat(&mut r,h);
    r.game.current_trigger_context=Some({ let mut __fixture_trigger_context = TriggerContext::default(); __fixture_trigger_context.event_permanent = Some(h); __fixture_trigger_context.event_card = Some(r.top_card(h)); __fixture_trigger_context });
    assert!(event_kind(&r,CompiledCardKind::Digimon));assert!(!event_kind(&r,CompiledCardKind::Tamer));
    assert!(r.game.players[0].battle_area[h.index as usize].synth_identity(&r.game.card_data,&r.game.modifiers,h).retained_kind.is_none());
}

#[test]
fn actual_marcus_retains_printed_colors_and_no_level_when_effect_grants_only_dp() {
    use digimon_engine::enums::CardColor;
    let mut r=base();let h=activate_real_marcus(&mut r);
    for g in [&r.game,&r.game.clone()] {
        let identity=g.players[0].battle_area[h.index as usize].synth_identity(&g.card_data,&g.modifiers,h);
        assert_eq!(identity.colors,vec![CardColor::Yellow,CardColor::Red]);
        assert_eq!(identity.level,None,"BT12-092 grants DP but no level");
    }
}
#[test]
fn actual_marcus_is_an_effective_field_color_match_then_expires() {
    use digimon_dsl::compiled::CompiledPlayerRef;
    let mut r=base();let h=activate_real_marcus(&mut r);
    let agumon=digimon_engine::permanent::PermanentHandle{player:0,index:1};
    r.game.delete_permanent_with_cause(agumon,ReplacementCause::OwnEffect);
    let index=r.add_to_hand(0,"BT12-092");let card=r.game.players[0].hand[index].handle();
    let pred=CompiledPredicate{color_matches_any_field_digimon:Some(CompiledPlayerRef::You),..Default::default()};
    let matches=|r:&DebugRunner| eval_predicate_with_bindings(&pred,&EffectReadContext::new(&r.game,r.top_card(h),None,0),PredicateSubject::Card(card),None);
    assert!(matches(&r),"sole field subject is the actual yellow/red Marcus treated as a Digimon");
    r.game.modifiers.expire_end_of_turn(0);
    assert!(!matches(&r),"ordinary yellow/red Tamer is not a field Digimon");
}

#[test]
fn additive_and_replacement_memberships_require_different_game_digests() {
    use digimon_engine::{enums::{CardKind,CardColor,ModifierType,Expiry},modifiers::{ModifierPayload,ModifierEntry}};
    let mut r=base();let h=r.place_on_field(0,"BT12-092",Some(0));
    r.game.current_trigger_context=Some({ let mut __fixture_trigger_context = TriggerContext::default(); __fixture_trigger_context.event_permanent = Some(h); __fixture_trigger_context.event_card = Some(r.top_card(h)); __fixture_trigger_context });
    let mut replacement=r.game.clone();
    let make=|additive| if additive {ModifierPayload::AdditiveSynthIdentity{kind:CardKind::Digimon,level:0,colors:vec![CardColor::Yellow,CardColor::Red],traits:vec![],dp:3000}}
        else {ModifierPayload::SynthIdentity{kind:CardKind::Digimon,level:0,colors:vec![CardColor::Yellow,CardColor::Red],traits:vec![],dp:3000}};
    r.game.modifiers.add(h,ModifierEntry::simple(ModifierType::TreatAsDigimon,0,Expiry::EndOfTurn,0).with_payload(make(true)));
    replacement.modifiers.add(h,ModifierEntry::simple(ModifierType::TreatAsDigimon,0,Expiry::EndOfTurn,0).with_payload(make(false)));
    assert!(event_kind(&r,CompiledCardKind::Tamer));
    let additive_digest=r.game.verification_digest();
    let replacement_digest=replacement.verification_digest();
    assert_ne!(additive_digest,replacement_digest,"semantically different Tamer membership must not collide after serialization");
    assert_eq!(additive_digest,r.game.clone().verification_digest());
    r.game=replacement;assert!(!event_kind(&r,CompiledCardKind::Tamer));
}
#[test]
fn ordinary_and_replacement_payload_serializations_keep_their_legacy_spelling() {
    use digimon_engine::{enums::{CardKind,CardColor},modifiers::ModifierPayload};
    assert_eq!(format!("{:?}",ModifierPayload::None),"None");
    assert_eq!(format!("{:?}",ModifierPayload::SynthIdentity{kind:CardKind::Digimon,level:3,colors:vec![CardColor::Red],traits:vec![],dp:0}),
        "SynthIdentity { kind: Digimon, level: 3, colors: [Red], traits: [], dp: 0 }");
}

// Shared snapshot controls deliberately use explicit modifier payloads; they
// exercise last-known field identity, not a claim that Marcus grants traits.
fn assert_effective_snapshot_for_departure(mode:u8) {
    use digimon_engine::enums::{CardSourceRef,StackPosition};
    let mut r=base();let seen=Arc::new(Mutex::new(vec![]));
    if mode==0 {r.register_effect("ST1-04",Arc::new(CaptureDeleted(seen.clone())));}
    else {r.register_effect("ST1-04",Arc::new(CaptureLeave(seen.clone())));}
    let receiver=r.place_on_field(0,"ST1-04",Some(0));
    let h=r.place_on_field(0,"ST1-12",Some(0));let top=r.top_card(h);
    r.game.modifiers.add(h,ModifierEntry::simple(ModifierType::TreatAsDigimon,0,Expiry::EndOfTurn,0)
        .with_payload(ModifierPayload::AdditiveSynthIdentity {kind:CardKind::Digimon,level:4,colors:vec![],traits:vec!["Dragonkin".into()],dp:5000}));
    r.game.modifiers.add(h,ModifierEntry::simple(ModifierType::ChangeBaseCardName,0,Expiry::EndOfTurn,0)
        .with_payload(ModifierPayload::Name{value:"Snapshot Alias".into(),base:true}));
    let saved=r.game.clone();let mut digests=vec![];
    for _ in 0..2 {
        r.game=saved.clone();seen.lock().unwrap().clear();
        match mode {
            0=>r.game.delete_permanent_with_cause(h,ReplacementCause::OwnEffect),
            1=>{assert!(r.game.return_to_hand(h).is_some());},
            2=>{assert!(r.game.return_to_deck(h,StackPosition::Bottom));},
            3=>{let mut ctx=digimon_engine::effect_context::EffectContext::new(&mut r.game,top,Some(h),0);assert!(ctx.place_permanent_on_security(0,h,StackPosition::Bottom,false));},
            4=>{assert!(r.game.place_as_bottom_source(CardSourceRef::Material(h,0),receiver,false));},
            _=>unreachable!(),
        }
        let snapshots=seen.lock().unwrap().clone();assert_eq!(snapshots.len(),1,"one actual departure event");
        let snap=&snapshots[0];
        assert_eq!(snap.level,Some(4));assert_eq!(snap.level_just_before,Some(4));
        assert_eq!(snap.traits,vec!["Dragonkin".to_string()]);assert_eq!(snap.traits_just_before,snap.traits);
        assert_eq!(snap.names_just_before,vec!["Snapshot Alias".to_string()]);
        assert_eq!(snap.card_kind,CardKind::Digimon);assert_eq!(snap.retained_kind,Some(CardKind::Tamer));
        assert_eq!(snap.dp,Some(6000),"printed ST1-12 grants all its controller's Digimon +1000 DP");
        assert_eq!(r.game.card_data_for_handle(top).unwrap().card_kind,CardKind::Tamer);
        digests.push(r.game.verification_digest());
    }
    assert_eq!(digests[0],digests[1]);
}
#[test]fn deleted_snapshot_uses_effective_level_traits_and_names(){assert_effective_snapshot_for_departure(0);}
#[test]fn returned_hand_snapshot_uses_effective_level_traits_and_names(){assert_effective_snapshot_for_departure(1);}
#[test]fn returned_deck_snapshot_uses_effective_level_traits_and_names(){assert_effective_snapshot_for_departure(2);}
#[test]fn security_placement_snapshot_uses_effective_level_traits_and_names(){assert_effective_snapshot_for_departure(3);}
#[test]fn field_to_source_snapshot_uses_effective_level_traits_and_names(){assert_effective_snapshot_for_departure(4);}

// Internal typed-modifier ordering controls: replacement/as-if remains distinct
// from additive treatment when the payloads coexist on one physical host.
fn assert_mixed_identity_order(additive_last:bool) {
 let mut r=base();let h=r.place_on_field(0,"BT12-092",Some(0));
 let additive=ModifierPayload::AdditiveSynthIdentity{kind:CardKind::Digimon,level:0,colors:vec![],traits:vec![],dp:3000};
 let replacement=ModifierPayload::SynthIdentity{kind:CardKind::Digimon,level:3,colors:vec![CardColor::Red],traits:vec![],dp:0};
 let payloads=if additive_last{vec![replacement,additive]}else{vec![additive,replacement]};
 for payload in payloads {r.game.modifiers.add(h,ModifierEntry::simple(ModifierType::TreatAsDigimon,0,Expiry::EndOfTurn,0).with_payload(payload));}
 for g in [&r.game,&r.game.clone()] {
  let identity=g.players[0].battle_area[0].synth_identity(&g.card_data,&g.modifiers,h);
  assert_eq!(identity.kind,CardKind::Digimon);
  assert_eq!(identity.retained_kind,if additive_last{Some(CardKind::Tamer)}else{None});
  assert_eq!(identity.level,if additive_last{None}else{Some(3)});
  assert_eq!(identity.dp,Some(if additive_last{3000}else{0}));
  assert_eq!(identity.colors,vec![CardColor::Red]);
 }
 r.game.modifiers.expire_end_of_turn(0);
 let identity=r.game.players[0].battle_area[0].synth_identity(&r.game.card_data,&r.game.modifiers,h);
 assert_eq!(identity.kind,CardKind::Tamer);assert_eq!(identity.level,None);assert_eq!(identity.dp,None);assert_eq!(identity.colors,vec![CardColor::Yellow,CardColor::Red]);
}
#[test]fn replacement_after_additive_uses_replacement_membership(){assert_mixed_identity_order(false);}
#[test]fn additive_after_replacement_restores_additional_printed_membership(){assert_mixed_identity_order(true);}
