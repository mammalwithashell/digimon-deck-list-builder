//! Floating effects retain their activation category and carrier across every tick.
use digimon_engine::debug_runner::{make_test_card,DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::modifiers::EffectControllerFilter;
use digimon_engine::{EffectSourceKind,EffectTiming,Expiry,Keyword,TriggerSource};
const BASE:&str=r#"
card: FLOAT-ORIGIN
name: Floating Origin
kind: digimon
level: 6
color: [yellow]
cost: 12
dp: 12000
effects:
  - when: on_play
    process:
      - grant_effect_immunity:
          target: self
          source_kind: digimon
          source_controller: any
          exclude_self: true
          expiry: end_of_opponents_turn
"#;
const DP:&str=r#"
      - add_modifier:
          target: {of: you, kind: digimon}
          modifier: ChangeDp
          value: 2000
          continuous: true
          expiry: end_of_opponents_turn
"#;
const KW:&str=r#"
      - grant_keyword_continuous:
          targets: {of: you, kind: digimon}
          keyword: Blocker
          expiry: end_of_opponents_turn
"#;
fn fixture(body:&str)->(DebugRunner,digimon_engine::permanent::PermanentHandle){
    let yaml=format!("{BASE}{body}");
    let mut r=DebugRunner::builder().from_dsl_yaml(&yaml).unwrap()
        .add_card(make_test_card("FLOAT-FILLER","Filler"))
        .deck(0,&["FLOAT-FILLER";16]).deck(1,&["FLOAT-FILLER";16]).memory(20).start();
    let host=r.place_on_field(0,"FLOAT-ORIGIN",None);
    r.game.enqueue_triggered(EffectTiming::OnPlay,TriggerSource::Permanent(host));
    r.game.drain_effect_queue();r.auto_resolve().unwrap();
    (r,host)
}
#[test]
fn own_floating_dp_is_not_suppressed_by_own_carrier_exclusion(){
    let(r,h)=fixture(DP);
    assert_eq!(r.game.effective_dp(h),Some(14000));
}
#[test]
fn own_floating_keyword_is_not_suppressed_by_own_carrier_exclusion(){
    let(r,h)=fixture(KW);
    assert!(r.game.has_keyword(h,Keyword::Blocker));
}
#[test]
fn floating_window_covers_late_entries_but_does_not_adopt_a_reentered_source(){
    let(mut r,h)=fixture(&format!("{DP}{KW}"));
    let original=r.game.players[0].battle_area[0].top_card().handle();
    let old_identity=r.game.permanent_effect_identity(h).unwrap();
    r.game.delete_permanent_with_effects(h);
    let late=r.place_on_field(0,"FLOAT-FILLER",None);
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(late,Keyword::Blocker),"floating effect survives its source");
    let trash=r.game.players[0].trash.iter().position(|c|c.handle()==original).unwrap();
    let index=r.game.play_from_trash(0,trash).unwrap();
    let returned=digimon_engine::permanent::PermanentHandle{player:0,index:index as u8};
    assert_eq!(r.game.players[0].battle_area[index].top_card().handle(),original);
    // The replayed card's own On Play creates a NEW floating window as well;
    // discard those new windows so this assertion isolates the original one.
    r.game.floating_mass_modifiers.truncate(2);
    let mut ctx=EffectContext::new(&mut r.game,original,Some(returned),0);
    assert_ne!(ctx.source_identity,Some(old_identity));
    ctx.add_effect_immunity_modifier_excluding_self(returned,EffectSourceKind::Digimon,
        EffectControllerFilter::Any,Expiry::EndOfOpponentsTurn,true);
    ctx.game.tick_declarative_effects();
    assert_eq!(ctx.game.effective_dp(returned),Some(12000),"old windows must not become new carrier's own effects");
    assert!(!ctx.game.has_keyword(returned,Keyword::Blocker));
}

#[test]
fn inherited_egg_floating_effect_keeps_digimon_category_after_departure(){
    use digimon_dsl::compiled::{CompiledStep,CompiledPredicate,CompiledModifierTarget,CompiledModifierValue};
    use digimon_engine::dsl_cards::{step::run_steps,bindings::Bindings};
    let mut egg=make_test_card("FLOAT-EGG","Egg");egg.card_kind=digimon_engine::CardKind::DigiEgg;
    let mut r=DebugRunner::builder().add_card(egg)
        .add_card(make_test_card("FLOAT-HOST","Host"))
        .add_card(make_test_card("FLOAT-PROTECTED","Protected"))
        .deck(0,&["FLOAT-HOST";16]).deck(1,&["FLOAT-HOST";16]).start();
    let source=r.place_on_field(0,"FLOAT-HOST",None);
    let target=r.place_on_field(0,"FLOAT-PROTECTED",None);
    let baseline=r.game.effective_dp(target);
    let own=r.game.players[0].battle_area[1].top_card().handle();
    EffectContext::new(&mut r.game,own,Some(target),0).add_effect_immunity_modifier_excluding_self(
        target,EffectSourceKind::Digimon,EffectControllerFilter::Any,Expiry::EndOfOpponentsTurn,true);
    let egg_data=r.game.card_data.iter().position(|c|c.card_id=="FLOAT-EGG").unwrap();
    let unique=r.game.next_card_index();let card=digimon_engine::card_source::CardSource::new(egg_data,0,unique);
    let egg_handle=card.handle();r.game.players[0].battle_area[0].card_sources.insert(0,card);
    let mut ctx=EffectContext::new(&mut r.game,egg_handle,Some(source),0);
    assert_eq!(ctx.source_kind,EffectSourceKind::Digimon);
    run_steps(&[CompiledStep::AddModifier{
        target:CompiledModifierTarget::Filter(CompiledPredicate::default()),modifier:"ChangeDp".into(),
        value:CompiledModifierValue::Literal(2000),expiry:"end_of_opponents_turn".into(),
        synth_identity:None,payload:None,continuous:true,opponent_only:false,
    }],&mut ctx,&mut Bindings::default());
    assert_eq!(ctx.game.effective_dp(target),baseline,"inherited Egg effect is Digimon-category and blocked");
    ctx.game.delete_permanent_with_effects(source);
    let target=digimon_engine::permanent::PermanentHandle{player:0,index:0};
    ctx.game.tick_declarative_effects();
    assert_eq!(ctx.game.effective_dp(target),baseline,"a departed inherited source does not become a native Egg effect");
}
