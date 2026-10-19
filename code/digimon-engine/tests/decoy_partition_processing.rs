
//! Official 16-17, 15-7-4, 15-8-5-4; Decoy is a paid prevention.
use digimon_engine::{action::{build_action_mask,space::{PASS,REPLACEMENT_ACCEPT}},debug_runner::{DebugRunner,make_test_card},enums::{CardColor,Keyword,ModifierType,Expiry},modifiers::ModifierEntry,permanent::PermanentHandle,selection::SelectionKind};
fn fixture(partition:bool)->DebugRunner {
    let mut decoy=make_test_card("DECOY-COST","Decoy carrier");
    decoy.keywords.push(Keyword::Decoy(1u8 << CardColor::Green as u8));
    let mut ally=make_test_card("DECOY-ALLY","Protected green ally");ally.colors=vec![CardColor::Green];
    let mut green=make_test_card("DECOY-GREEN5","Green source");green.colors=vec![CardColor::Green];green.level=Some(5);
    let mut blue=make_test_card("DECOY-BLUE5","Blue source");blue.colors=vec![CardColor::Blue];blue.level=Some(5);
    let mut r=DebugRunner::builder().dsl_card("BT23-047").unwrap().add_card(decoy).add_card(ally).add_card(green).add_card(blue)
        .deck(0,&["DECOY-ALLY";20]).deck(1,&["DECOY-ALLY";20]).memory(3).start();
    r.skip_mulligan();r.game.enter_main_phase();
    if partition {r.place_stack(0,&["DECOY-GREEN5","DECOY-BLUE5","BT23-047"]);}else{r.place_on_field(0,"DECOY-ALLY",None);}
    r.place_on_field(0,"DECOY-COST",None);r
}
fn act(r:&mut DebugRunner,a:u16){let p=r.game.pending_selection.as_ref().unwrap().selecting_player;assert_eq!(build_action_mask(&r.game,p)[a as usize],1.0);r.game.resolve_selection(p,a).unwrap();}
fn delete_ally(r:&mut DebugRunner){r.game.set_effect_source_player_for_test(Some(1));r.game.delete_permanent_with_effects(PermanentHandle{player:0,index:0});}
fn ids(r:&DebugRunner)->Vec<String>{r.game.players[0].battle_area.iter().map(|p|p.top_card().card_id(&r.game.card_data).to_string()).collect()}
fn choose_order(r:&mut DebugRunner,needle:&str){let v=r.pending_selection_view().unwrap();let rows=v.effect_choices.as_ref().expect("simultaneous immediate order choices");let a=rows.iter().find(|c|c.label.contains(needle)).unwrap().action_id;act(r,a);}
#[test]
fn decoy_optional_attempt_only_prevents_after_actual_own_deletion_and_clones(){
    for blocked in [false,true]{
        let mut r=fixture(false);
        if blocked {r.game.modifiers.add(PermanentHandle{player:0,index:1},ModifierEntry::simple(ModifierType::CannotBeDestroyedByEffect,0,Expiry::Permanent,0));}
        delete_ally(&mut r);
        assert_eq!(r.pending_selection_view().unwrap().kind,SelectionKind::Replacement);
        // 15-7-4 allows the optional processing attempt even when it cannot succeed.
        let mask=build_action_mask(&r.game,0);assert_eq!(mask[PASS as usize],1.0);assert_eq!(mask[REPLACEMENT_ACCEPT as usize],1.0);
        let saved=r.game.clone();
        for accept in [false,true]{r.game=saved.clone();act(&mut r,if accept{REPLACEMENT_ACCEPT}else{PASS});r.auto_resolve().unwrap();
            let paid=accept&&!blocked;
            assert_eq!(ids(&r),if paid{vec!["DECOY-ALLY"]}else{vec!["DECOY-COST"]},"blocked={blocked}, accept={accept}");
        }
    }
}
struct OwnCostWitness;
impl digimon_engine::effect::CardEffect for OwnCostWitness {
 fn effects(&self,c:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect>{vec![digimon_engine::effect::Effect::on_deletion(c).process(|ctx|{
    assert_eq!(ctx.deletion_cause(),Some(digimon_engine::replacement::ReplacementCause::OwnEffect),"Decoy is deleted by its own cost, not the original opponent effect");ctx.draw(0,1);
 }).build()]}
}
#[test]
fn decoy_actual_deletion_uses_own_effect_cause(){let mut r=fixture(false);r.register_effect("DECOY-COST",std::sync::Arc::new(OwnCostWitness));let hand=r.hand_size(0);delete_ally(&mut r);act(&mut r,REPLACEMENT_ACCEPT);r.auto_resolve().unwrap();assert_eq!(ids(&r),vec!["DECOY-ALLY"]);assert_eq!(r.hand_size(0),hand+1);}
#[test]
fn decoy_and_partition_offer_both_orders_and_keep_later_processing_after_prevention(){
 let mut r=fixture(true);delete_ally(&mut r);
 let v=r.pending_selection_view().unwrap();let rows=v.effect_choices.as_ref().expect("Decoy and Partition trigger at same impending deletion");
 assert!(rows.iter().any(|c|c.label.contains("Decoy")));assert!(rows.iter().any(|c|c.label.contains("Partition")));
 let saved=r.game.clone();
 for decoy_first in [false,true]{r.game=saved.clone();
    choose_order(&mut r,if decoy_first{"Decoy"}else{"Partition"});act(&mut r,REPLACEMENT_ACCEPT);
    if !decoy_first {resolve_partition_sources(&mut r);}
    let v=r.pending_selection_view().unwrap_or_else(||panic!("remaining immediate effect: decoy_first={}, field={:?}",decoy_first,ids(&r)));
    if v.effect_choices.is_some(){choose_order(&mut r,if decoy_first{"Partition"}else{"Decoy"});}
    act(&mut r,REPLACEMENT_ACCEPT);
    if decoy_first {resolve_partition_sources(&mut r);}
    r.auto_resolve().unwrap();let mut got=ids(&r);got.sort();assert_eq!(got,vec!["BT23-047","DECOY-BLUE5","DECOY-GREEN5"]);
 }
}

struct OptionalOwnCostPrevention;
impl digimon_engine::effect::CardEffect for OptionalOwnCostPrevention {
 fn effects(&self,c:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect>{vec![digimon_engine::effect::Effect::when_would_be_deleted(c)
    .name("Prevent this Decoy's own-effect payment").optional()
    .replacement_condition(|ctx,subject|ctx.replacement_cause()==Some(digimon_engine::replacement::ReplacementCause::OwnEffect)
      && matches!(subject,digimon_engine::replacement::ReplacementSubject::Permanent(h) if Some(*h)==ctx.source_permanent))
    .replacement_process(|ctx|ctx.cancel()).build()]}
}
#[test]
fn decoy_waits_for_parked_own_cost_actual_result_with_cloned_prevention_choices(){
 let mut r=fixture(false);r.register_effect("DECOY-COST",std::sync::Arc::new(OptionalOwnCostPrevention));delete_ally(&mut r);
 act(&mut r,REPLACEMENT_ACCEPT);
 let v=r.pending_selection_view().expect("own deletion payment has its own replacement choice");assert_eq!(v.kind,SelectionKind::Replacement);
 assert_eq!(ids(&r),vec!["DECOY-ALLY","DECOY-COST"],"neither original nor cost may move before decision");
 let saved=r.game.clone();
 for prevent_cost in [false,true]{r.game=saved.clone();act(&mut r,if prevent_cost{REPLACEMENT_ACCEPT}else{PASS});r.auto_resolve().unwrap();
  assert_eq!(ids(&r),if prevent_cost{vec!["DECOY-COST"]}else{vec!["DECOY-ALLY"]});assert!(r.game.pending_selection.is_none());
 }
}

struct SubstituteDecoyPayment;
impl digimon_engine::effect::CardEffect for SubstituteDecoyPayment {
 fn effects(&self,c:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect>{vec![digimon_engine::effect::Effect::when_would_be_deleted(c)
    .name("Substitute another body for Decoy payment").optional()
    .replacement_condition(|ctx,subject|ctx.replacement_cause()==Some(digimon_engine::replacement::ReplacementCause::OwnEffect)
      && matches!(subject,digimon_engine::replacement::ReplacementSubject::Permanent(h) if Some(*h)==ctx.source_permanent))
    .replacement_process(|ctx|ctx.substitute(digimon_engine::replacement::ReplacementSubject::Permanent(PermanentHandle{player:0,index:2}))).build()]}
}
#[test]
fn a_different_substituted_deletion_does_not_pay_decoy_cost(){
 let mut r=fixture(false);r.place_on_field(0,"DECOY-GREEN5",None);
 r.register_effect("DECOY-COST",std::sync::Arc::new(SubstituteDecoyPayment));delete_ally(&mut r);act(&mut r,REPLACEMENT_ACCEPT);
 assert!(r.game.pending_selection.is_some(),"own-payment replacement must be offered");
 let saved=r.game.clone();
 for substitute in [false,true]{r.game=saved.clone();act(&mut r,if substitute{REPLACEMENT_ACCEPT}else{PASS});r.auto_resolve().unwrap();
   let mut got=ids(&r);got.sort();
   assert_eq!(got,if substitute{vec!["DECOY-COST"]}else{vec!["DECOY-ALLY","DECOY-GREEN5"]});
 }
}

#[test]
fn generic_paid_carrier_deletion_does_not_require_digimon_kind(){
 let yaml=r#"
card: PAID-TAMER
name: Paid Tamer
kind: tamer
color: [green]
cost: 3
effects:
  - kind: replacement
    trigger: when_would_be_deleted
    optional: true
    active_when:
      all_of:
        - replacement_subject_is_mine: true
        - other: true
        - replacement_cause: opponent_effect
    cost: { delete_self: true }
    outcome: prevent
"#;
 let mut ally=make_test_card("PAY-ALLY","Ally");ally.colors=vec![CardColor::Green];
 let mut r=DebugRunner::builder().from_dsl_yaml(yaml).unwrap().add_card(ally)
  .deck(0,&["PAY-ALLY";20]).deck(1,&["PAY-ALLY";20]).memory(3).start();
 r.skip_mulligan();r.game.enter_main_phase();r.place_on_field(0,"PAY-ALLY",None);r.place_on_field(0,"PAID-TAMER",None);
 delete_ally(&mut r);act(&mut r,REPLACEMENT_ACCEPT);r.auto_resolve().unwrap();assert_eq!(ids(&r),vec!["PAY-ALLY"]);
}

struct TwoTimingRiders;
impl digimon_engine::effect::CardEffect for TwoTimingRiders {
 fn effects(&self,c:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect>{
  use digimon_engine::effect::Effect;
  vec![Effect::when_would_leave_battle_area(c).name("Leave rider one").optional()
    .replacement_condition(|ctx,s|matches!(s,digimon_engine::replacement::ReplacementSubject::Permanent(h) if Some(*h)==ctx.source_permanent))
    .replacement_process(|ctx|{ctx.effect.draw(0,1);} ).build(),
   Effect::when_would_be_deleted(c).name("Delete rider three").optional()
    .replacement_condition(|ctx,s|matches!(s,digimon_engine::replacement::ReplacementSubject::Permanent(h) if Some(*h)==ctx.source_permanent))
    .replacement_process(|ctx|{ctx.effect.draw(0,3);} ).build()]
 }
}
fn rider_fixture()->DebugRunner {
 let mut r=DebugRunner::builder().add_card(make_test_card("TIMING-RIDER","Timing rider"))
  .deck(0,&["TIMING-RIDER";30]).deck(1,&["TIMING-RIDER";30]).memory(3).start();
 r.skip_mulligan();r.game.enter_main_phase();r.place_on_field(0,"TIMING-RIDER",None);
 r.register_effect("TIMING-RIDER",std::sync::Arc::new(TwoTimingRiders));r
}
#[test]
fn impending_deletion_keeps_each_original_effect_slot_in_both_orders(){
 let mut r=rider_fixture();let hand=r.hand_size(0);delete_ally(&mut r);let saved=r.game.clone();
 for leave_first in [false,true]{r.game=saved.clone();choose_order(&mut r,if leave_first{"Leave rider"}else{"Delete rider"});act(&mut r,REPLACEMENT_ACCEPT);
  assert_eq!(r.hand_size(0),hand+if leave_first{1}else{3});
  let v=r.pending_selection_view().expect("other simultaneous timing remains");
  if v.effect_choices.is_some(){choose_order(&mut r,if leave_first{"Delete rider"}else{"Leave rider"});}
  act(&mut r,REPLACEMENT_ACCEPT);r.auto_resolve().unwrap();assert_eq!(r.hand_size(0),hand+4);assert!(ids(&r).is_empty());assert!(r.game.pending_selection.is_none());
 }
}
#[test]
fn nontrash_return_does_not_offer_the_deletion_rider(){
 let mut r=rider_fixture();let hand=r.hand_size(0);r.game.set_effect_source_player_for_test(Some(1));
 r.game.return_to_hand(PermanentHandle{player:0,index:0});
 assert!(r.pending_selection_view().unwrap().effect_choices.is_none());act(&mut r,REPLACEMENT_ACCEPT);r.auto_resolve().unwrap();
 assert_eq!(r.hand_size(0),hand+2,"one leave draw plus returned physical card");assert!(ids(&r).is_empty());assert!(r.game.pending_selection.is_none());
}
#[test]
fn passive_prohibition_prevents_the_deletion_event_before_immediate_riders(){
 let mut r=rider_fixture();let hand=r.hand_size(0);
 r.game.modifiers.add(PermanentHandle{player:0,index:0},ModifierEntry::simple(ModifierType::CannotBeDestroyedByEffect,0,Expiry::Permanent,0));
 delete_ally(&mut r);assert!(r.game.pending_selection.is_none());assert_eq!(r.hand_size(0),hand);assert_eq!(ids(&r),vec!["TIMING-RIDER"]);
}

#[test]
fn paid_decoy_exposes_each_simultaneous_victim_and_only_protects_the_chosen_one(){
 let mut r=fixture(false);r.place_on_field(0,"DECOY-GREEN5",None);
 r.game.set_effect_source_player_for_test(Some(1));
 r.game.delete_permanents_batch(vec![PermanentHandle{player:0,index:0},PermanentHandle{player:0,index:2}],digimon_engine::replacement::ReplacementCause::OpponentEffect);
 act(&mut r,REPLACEMENT_ACCEPT);
 let v=r.pending_selection_view().expect("successful Decoy payment must offer which simultaneous victim to protect");
 assert_eq!(v.kind,SelectionKind::OwnField);assert!(!v.valid_action_ids.contains(&PASS));assert_eq!(v.valid_action_ids.len(),2);
 let choices=v.valid_action_ids.clone();let saved=r.game.clone();let mut survivors=Vec::new();
 for a in choices {r.game=saved.clone();act(&mut r,a);r.auto_resolve().unwrap();let got=ids(&r);assert_eq!(got.len(),1);survivors.push(got[0].clone());}
 survivors.sort();assert_eq!(survivors,vec!["DECOY-ALLY","DECOY-GREEN5"]);
}

fn resolve_partition_sources(r:&mut DebugRunner){
 for _ in 0..2 {
  let Some(v)=r.pending_selection_view() else {break;};
  if !matches!(v.kind,SelectionKind::CountCappedMultiSelect{..}){break;}
  assert_eq!(build_action_mask(&r.game,0)[PASS as usize],0.0,"Partition's source choice cannot decline");
  act(r,v.valid_action_ids[0]);
 }
 assert!(ids(r).contains(&"DECOY-GREEN5".to_string()));assert!(ids(r).contains(&"DECOY-BLUE5".to_string()));
}

fn delete_two(r:&mut DebugRunner){
 r.game.set_effect_source_player_for_test(Some(1));
 r.game.delete_permanents_batch(vec![PermanentHandle{player:0,index:0},PermanentHandle{player:0,index:2}],digimon_engine::replacement::ReplacementCause::OpponentEffect);
}
#[test]
fn declined_multi_victim_decoy_is_not_reoffered_for_the_second_member(){
 let mut r=fixture(false);r.place_on_field(0,"DECOY-GREEN5",None);delete_two(&mut r);
 assert_eq!(build_action_mask(&r.game,0)[PASS as usize],1.0);
 let mut clone=r.game.clone();act(&mut r,PASS);clone.resolve_selection(0,PASS).unwrap();
 assert!(r.game.pending_selection.is_none());assert_eq!(ids(&r),vec!["DECOY-COST"]);
 assert_eq!(r.game.verification_digest(),clone.verification_digest());
}
#[test]
fn multi_victim_decoy_waits_for_nested_payment_and_never_credits_prevented_payment(){
 let mut r=fixture(false);r.place_on_field(0,"DECOY-GREEN5",None);
 r.register_effect("DECOY-COST",std::sync::Arc::new(OptionalOwnCostPrevention));delete_two(&mut r);
 act(&mut r,REPLACEMENT_ACCEPT);assert_eq!(r.pending_selection_view().unwrap().kind,SelectionKind::Replacement);
 assert_eq!(ids(&r).len(),3);let saved=r.game.clone();
 for prevent in [false,true]{r.game=saved.clone();act(&mut r,if prevent{REPLACEMENT_ACCEPT}else{PASS});
  if prevent {assert!(r.game.pending_selection.is_none());assert_eq!(ids(&r),vec!["DECOY-COST"]);}
  else {let view=r.pending_selection_view().expect("actual payment offers protected member");
   assert_eq!(view.kind,SelectionKind::OwnField);assert_eq!(view.valid_action_ids.len(),2);
   assert!(!ids(&r).contains(&"DECOY-COST".into()));let paid=r.game.clone();
   for a in view.valid_action_ids{r.game=paid.clone();act(&mut r,a);r.auto_resolve().unwrap();assert_eq!(ids(&r).len(),1);}
  }
 }
}
#[test]
fn multi_victim_decoy_and_partition_keep_both_effect_orders_after_prevention(){
 let mut r=fixture(true);r.place_on_field(0,"DECOY-GREEN5",None);delete_two(&mut r);
 let view=r.pending_selection_view().unwrap();let rows=view.effect_choices.as_ref().unwrap();
 assert!(rows.iter().any(|c|c.label.contains("Decoy")));assert!(rows.iter().any(|c|c.label.contains("Partition")));
 let saved=r.game.clone();
 for decoy_first in [false,true]{r.game=saved.clone();
  choose_order(&mut r,if decoy_first{"Decoy"}else{"Partition"});act(&mut r,REPLACEMENT_ACCEPT);
  if !decoy_first{resolve_partition_sources(&mut r);}
  else{let i=ids(&r).iter().position(|id|id=="BT23-047").unwrap();act(&mut r,digimon_engine::action::space::encode_attack(0,i as u16));}
  if r.pending_selection_view().unwrap().effect_choices.is_some(){choose_order(&mut r,if decoy_first{"Partition"}else{"Decoy"});}
  act(&mut r,REPLACEMENT_ACCEPT);
  if decoy_first{resolve_partition_sources(&mut r);}
  else{let i=ids(&r).iter().position(|id|id=="BT23-047").unwrap();act(&mut r,digimon_engine::action::space::encode_attack(0,i as u16));}
  r.auto_resolve().unwrap();let mut got=ids(&r);got.sort();assert_eq!(got,vec!["BT23-047","DECOY-BLUE5","DECOY-GREEN5"]);
 }
}
struct CrossMemberRider(bool);
impl digimon_engine::effect::CardEffect for CrossMemberRider{
 fn effects(&self,c:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect>{
  let effect = if self.0 { digimon_engine::effect::Effect::when_would_leave_battle_area(c) }
      else { digimon_engine::effect::Effect::when_would_be_deleted(c) };
  vec![effect.name("Protect one actual event member").optional()
   .replacement_condition(|ctx,s|ctx.replacement_cause()==Some(digimon_engine::replacement::ReplacementCause::OpponentEffect)
    && matches!(s,digimon_engine::replacement::ReplacementSubject::Permanent(h) if Some(*h)!=ctx.source_permanent && h.player==ctx.player))
   .replacement_process(|ctx|{ctx.effect.draw(0,1);ctx.cancel();}).build()]
 }
}
#[test]
fn ordinary_cross_member_effect_asks_which_subject_before_processing_once(){
 for leave_only in [false,true] {
 let mut r=fixture(false);
 // Replace the native keyword carrier with a distinct ordinary effect source.
 r.game.players[0].battle_area.pop();r.place_on_field(0,"DECOY-BLUE5",None);
 r.register_effect("DECOY-BLUE5",std::sync::Arc::new(CrossMemberRider(leave_only)));r.place_on_field(0,"DECOY-GREEN5",None);
 let hand=r.hand_size(0);delete_two(&mut r);act(&mut r,REPLACEMENT_ACCEPT);
 let view=r.pending_selection_view().expect("no implicit first event subject");assert_eq!(view.kind,SelectionKind::AnyField);
 assert_eq!(view.valid_action_ids.len(),2);assert_eq!(r.hand_size(0),hand);let saved=r.game.clone();
 let mut protected=Vec::new();for a in view.valid_action_ids{r.game=saved.clone();act(&mut r,a);r.auto_resolve().unwrap();
  assert_eq!(r.hand_size(0),hand+1);let got=ids(&r);assert_eq!(got.len(),2);
  protected.push(got.into_iter().find(|id|id!="DECOY-BLUE5").unwrap());}
 protected.sort();assert_eq!(protected,vec!["DECOY-ALLY","DECOY-GREEN5"]);
 }
}

struct MandatorySubstitute;
impl digimon_engine::effect::CardEffect for MandatorySubstitute{
 fn effects(&self,c:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect>{
  vec![digimon_engine::effect::Effect::when_would_be_deleted(c)
   .replacement_condition(|ctx,s|matches!(s,digimon_engine::replacement::ReplacementSubject::Permanent(h) if Some(*h)==ctx.source_permanent))
   .replacement_process(|ctx|ctx.substitute(digimon_engine::replacement::ReplacementSubject::Permanent(PermanentHandle{player:0,index:2}))).build()]
 }
}
#[test]
fn simultaneous_substitution_preserves_public_completed_cancelled_and_substituted_summary(){
 let mut r=DebugRunner::builder().add_card(make_test_card("SUM-A","Original"))
  .add_card(make_test_card("SUM-B","Other victim")).add_card(make_test_card("SUM-C","Substitute"))
  .deck(0,&["SUM-B";20]).deck(1,&["SUM-B";20]).memory(3).start();
 r.skip_mulligan();r.game.enter_main_phase();r.register_effect("SUM-A",std::sync::Arc::new(MandatorySubstitute));
 for id in ["SUM-A","SUM-B","SUM-C"]{r.place_on_field(0,id,None);}
 let a=PermanentHandle{player:0,index:0};let b=PermanentHandle{player:0,index:1};let c=PermanentHandle{player:0,index:2};
 r.game.set_effect_source_player_for_test(Some(1));
 let outcome=r.game.delete_permanents_batch(vec![a,b],digimon_engine::replacement::ReplacementCause::OpponentEffect);
 assert_eq!(outcome.cancelled,vec![a]);assert_eq!(outcome.substituted_in,vec![c]);
 assert_eq!(outcome.completed,vec![b,c]);assert_eq!(ids(&r),vec!["SUM-A"]);assert!(r.game.pending_selection.is_none());
}

struct CauseBatch;
impl digimon_engine::effect::CardEffect for CauseBatch{
 fn effects(&self,c:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect>{
  vec![digimon_engine::effect::Effect::on_play(c).process(|ctx|{
   let targets=ctx.game.players[0].battle_area.iter().enumerate().filter(|(_,p)|
    ["DECOY-ALLY","DECOY-GREEN5"].contains(&p.top_card().card_id(&ctx.game.card_data)))
    .map(|(i,_)|PermanentHandle{player:0,index:i as u8}).collect();
   ctx.game.delete_permanents_batch(targets,digimon_engine::replacement::ReplacementCause::OpponentEffect);
  }).build()]
 }
}
struct DeletionSourceWitness(digimon_engine::card_source::CardHandle);
impl digimon_engine::effect::CardEffect for DeletionSourceWitness{
 fn effects(&self,c:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect>{
  let source=self.0;
  vec![digimon_engine::effect::Effect::on_deletion(c).process(move |ctx|{
   assert_eq!(ctx.event_source_effect().and_then(|e|e.source_card),Some(source),"causing effect survives synchronous and parked group commit");
   assert_eq!(ctx.event_source_effect().and_then(|e|e.source_permanent),Some(PermanentHandle{player:1,index:0}));
   assert_eq!(ctx.deletion_cause(),Some(digimon_engine::replacement::ReplacementCause::OpponentEffect));
   ctx.draw(0,1);
  }).build()]
 }
}
#[test]
fn simultaneous_deletion_commit_preserves_original_effect_attribution_after_decoy_park(){
 for parked in [false,true]{
  let mut r=fixture(false);if !parked{r.game.players[0].battle_area.pop();}
  r.place_on_field(0,"DECOY-GREEN5",None);let cause=r.place_on_field(1,"DECOY-BLUE5",None);
  let card=r.game.players[1].battle_area[0].top_card().handle();
  r.register_effect("DECOY-BLUE5",std::sync::Arc::new(CauseBatch));
  r.register_effect("DECOY-ALLY",std::sync::Arc::new(DeletionSourceWitness(card)));
  r.register_effect("DECOY-GREEN5",std::sync::Arc::new(DeletionSourceWitness(card)));
  let hand=r.hand_size(0);
  r.game.enqueue_triggered(digimon_engine::enums::EffectTiming::OnPlay,digimon_engine::selection::TriggerSource::Permanent(cause));
  r.game.drain_effect_queue();
  if parked {act(&mut r,REPLACEMENT_ACCEPT);let v=r.pending_selection_view().expect("post-payment victim choice");assert_eq!(v.kind,SelectionKind::OwnField);act(&mut r,v.valid_action_ids[0]);}
  r.auto_resolve().unwrap();assert_eq!(r.hand_size(0),hand+if parked{1}else{2});
 }
}

// DCGO DecoyProcess reevaluates the target predicate inside SuccessProcess.
// A physical victim's continued existence does not retain its earlier color.
#[test]
fn paid_decoy_rechecks_current_color_after_payment_before_offering_victims() {
    for grant_depends_on_payer in [false, true] {
        let mut r = fixture(false);
        // Victim zero is printed green; temporarily make it blue, then grant
        // green only while the Decoy payer is physically present.
        let victim = PermanentHandle { player: 0, index: 0 };
        let payer = r.game.players[0].battle_area[1].top_card().handle();
        r.game.modifiers.add(victim,
            ModifierEntry::simple(ModifierType::ChangeBaseCardColor, 0, Expiry::Permanent, 0)
                .with_payload(digimon_engine::modifiers::ModifierPayload::Colors {
                    value: vec![CardColor::Blue], base: true,
                }));
        r.game.modifiers.add(victim,
            ModifierEntry::simple(ModifierType::AddColor, 0, Expiry::UntilCondition, 0)
                .with_payload(digimon_engine::modifiers::ModifierPayload::Colors {
                    value: vec![CardColor::Green], base: false,
                })
                .with_until_condition(std::sync::Arc::new(move |g, _| {
                    !grant_depends_on_payer || g.players[0].battle_area.iter()
                        .any(|p| p.top_card().handle() == payer)
                })));
        r.place_on_field(0, "DECOY-GREEN5", None);
        assert!(r.game.players[0].battle_area[0]
            .colors_for_rules(&r.game.card_data, &r.game.modifiers, victim)
            .contains(&CardColor::Green));
        delete_two(&mut r);
        let saved = r.game.clone();
        for _ in 0..2 {
            r.game = saved.clone();
            act(&mut r, REPLACEMENT_ACCEPT);
            let view = r.pending_selection_view().expect("post-payment target choice");
            assert_eq!(view.kind, SelectionKind::OwnField);
            let stale = digimon_engine::action::space::encode_attack(0, 0);
            let valid = digimon_engine::action::space::encode_attack(0, 1);
            assert_eq!(r.game.players[0].battle_area[0]
                .colors_for_rules(&r.game.card_data, &r.game.modifiers, victim)
                .contains(&CardColor::Green), !grant_depends_on_payer,
                "the payment has removed the physical source of the temporary grant");
            let mask = build_action_mask(&r.game, 0);
            assert_eq!(mask[stale as usize], if grant_depends_on_payer { 0.0 } else { 1.0 });
            assert_eq!(mask[valid as usize], 1.0);
            assert_eq!(view.valid_action_ids.len(), if grant_depends_on_payer { 1 } else { 2 });
            act(&mut r, valid);
            r.auto_resolve().unwrap();
            assert_eq!(ids(&r), vec!["DECOY-GREEN5"]);
        }
    }
}

// Official BT25-039 Q6307: the paid protection covers all matching members
// without selecting one. This is deliberately distinct from native Decoy.
#[test]
fn sirenmon_paid_protection_preserves_every_matching_simultaneous_victim() {
    let mut ally = make_test_card("SIREN-ALLY", "Shaman ally");
    ally.traits = vec!["Shaman".into()];
    let mut other = make_test_card("SIREN-OTHER", "Iliad ally");
    other.traits = vec!["Iliad".into()];
    let mut r = DebugRunner::builder().dsl_card("BT25-039").unwrap()
        .add_card(ally).add_card(other)
        .deck(0, &["SIREN-ALLY"; 20]).deck(1, &["SIREN-ALLY"; 20]).memory(3).start();
    r.skip_mulligan(); r.game.enter_main_phase();
    r.place_on_field(0, "SIREN-ALLY", None);
    r.place_on_field(0, "BT25-039", None);
    r.place_on_field(0, "SIREN-OTHER", None);
    delete_two(&mut r);
    assert_eq!(r.pending_selection_view().unwrap().kind, SelectionKind::Replacement);
    let saved = r.game.clone();
    for _ in 0..2 {
        r.game = saved.clone();
        act(&mut r, REPLACEMENT_ACCEPT);
        if let Some(view) = r.pending_selection_view() {
            assert_ne!(view.kind, SelectionKind::OwnField,
                "Q6307 protects the complete matching group, without a victim chooser");
        }
        r.auto_resolve().unwrap();
        assert_eq!(ids(&r), vec!["SIREN-ALLY", "SIREN-OTHER"]);
        assert!(r.game.pending_selection.is_none());
    }
}

// Review28 discriminators: scratch only, not yet compiled or executed.
#[test]
fn mixed_controller_batch_decoy_cause_is_relative_to_each_victim() {
 for acting in [0,1] {
  let mut r=fixture(false);
  r.place_on_field(1,"DECOY-ALLY",None);r.place_on_field(1,"DECOY-COST",None);
  r.game.set_effect_source_player_for_test(Some(acting));
  let cause=if acting==0 {digimon_engine::replacement::ReplacementCause::OwnEffect} else {digimon_engine::replacement::ReplacementCause::OpponentEffect};
  r.game.delete_permanents_batch(vec![PermanentHandle{player:0,index:0},PermanentHandle{player:1,index:0}],cause);
  let view=r.pending_selection_view().expect("the opposite controller may attempt Decoy");
  assert_eq!(view.selecting_player,1-acting,"the causing player's own Decoy must not protect from its own effect");
  assert_eq!(view.kind,SelectionKind::Replacement);
  let saved=r.game.clone();let mut digest=None;
  for _ in 0..2 {
   r.game=saved.clone();act(&mut r,REPLACEMENT_ACCEPT);
   let view=r.pending_selection_view().expect("choose the still-valid opposite-side victim after payment");
   assert_eq!(view.selecting_player,1-acting);assert_eq!(view.valid_action_ids.len(),1);
   act(&mut r,view.valid_action_ids[0]);r.auto_resolve().unwrap();
   assert_eq!(r.game.players[acting as usize].battle_area.len(),1);
   assert_eq!(r.game.players[acting as usize].battle_area[0].top_card().card_id(&r.game.card_data),"DECOY-COST");
   assert_eq!(r.game.players[(1-acting) as usize].battle_area.len(),1);
   assert_eq!(r.game.players[(1-acting) as usize].battle_area[0].top_card().card_id(&r.game.card_data),"DECOY-ALLY");
   let d=r.game.verification_digest();if let Some(prior)=digest {assert_eq!(d,prior);}digest=Some(d);
  }
 }
}

#[test]
fn q6308_sirenmon_on_deletion_waits_for_causing_effect_tail_and_its_prompt() {
 let driver=r#"
card: Q6308-DRIVER
name: Causing effect
kind: digimon
color: [red]
level: 6
cost: 12
dp: 12000
effects:
  - when: on_play
    process:
      - delete_all_permanents:
          over: { of: opponent, kind: digimon, zone: [battle_area], name_is: "Shaman target" }
      - select_effect_choice:
          bind_as: tail
          labels: ["Causing effect tail A", "Causing effect tail B"]
          prompt: "Finish the causing effect before On Deletion"
      - draw: { of: you, count: 1 }
"#;
 let mut ally=make_test_card("Q6308-ALLY","Shaman target");ally.traits=vec!["Shaman".into()];
 let mut r=DebugRunner::builder().from_dsl_yaml(driver).unwrap().dsl_card("BT25-039").unwrap().add_card(ally)
  .deck(0,&["Q6308-ALLY";20]).deck(1,&["Q6308-ALLY";20]).memory(3).start();
 r.skip_mulligan();r.game.enter_main_phase();
 r.place_on_field(0,"Q6308-ALLY",None);r.place_on_field(0,"BT25-039",None);r.place_on_field(0,"Q6308-ALLY",None);
 let h=r.place_on_field(1,"Q6308-DRIVER",None);let hand=r.hand_size(1);
 r.game.enqueue_triggered(digimon_engine::enums::EffectTiming::OnPlay,digimon_engine::selection::TriggerSource::Permanent(h));r.game.drain_effect_queue();
 assert_eq!(r.pending_selection_view().unwrap().kind,SelectionKind::Replacement);
 let saved=r.game.clone();let mut digest=None;
 for _ in 0..2 {
  r.game=saved.clone();act(&mut r,REPLACEMENT_ACCEPT);
  let view=r.pending_selection_view().expect("the causing effect's own tail remains first");
  assert_eq!(view.selecting_player,1,"Sirenmon's controller must not get On Deletion before the causing effect finishes");
  assert!(view.prompt.contains("Finish the causing effect"));
  assert_eq!(r.hand_size(1),hand);assert_eq!(view.valid_action_ids.len(),2);
  act(&mut r,view.valid_action_ids[0]);assert_eq!(r.hand_size(1),hand+1);
  let deletion=r.pending_selection_view().expect("real Sirenmon On Deletion optional effect follows");assert_eq!(deletion.selecting_player,0);
  r.auto_resolve().unwrap();
  assert_eq!(r.game.players[0].battle_area.len(),2);
  let d=r.game.verification_digest();if let Some(prior)=digest{assert_eq!(d,prior);}digest=Some(d);
 }
}

struct Review28OwnBatch;
impl digimon_engine::effect::CardEffect for Review28OwnBatch {
 fn effects(&self,c:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect>{
  vec![digimon_engine::effect::Effect::on_play(c).process(|ctx|{
   let targets=ctx.game.players[0].battle_area.iter().enumerate().filter(|(_,p)|
    ["R28-PROTECT","R28-WITNESS"].contains(&p.top_card().card_id(&ctx.game.card_data)))
    .map(|(i,_)|PermanentHandle{player:0,index:i as u8}).collect();
   ctx.game.delete_permanents_batch(targets,digimon_engine::replacement::ReplacementCause::OwnEffect);
  }).build()]
 }
}
struct Review28SourceWitness(digimon_engine::card_source::CardHandle);
impl digimon_engine::effect::CardEffect for Review28SourceWitness {
 fn effects(&self,c:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect>{let source=self.0;
  vec![digimon_engine::effect::Effect::on_deletion(c).process(move|ctx|{
   let current=ctx.game.players[0].battle_area.iter().position(|p|p.top_card().handle()==source).unwrap();
   let event=ctx.event_source_effect().unwrap();assert_eq!(event.source_card,Some(source));
   assert_eq!(event.source_permanent,Some(PermanentHandle{player:0,index:current as u8}),"attribution resolves the causing source after same-controller payer compaction");
   ctx.draw(0,1);
  }).build()]
 }
}
#[test]
fn causing_permanent_attribution_survives_same_controller_payer_compaction() {
 let payer=r#"
card: R28-PAYER
name: Paid own-effect witness
kind: digimon
color: [green]
level: 4
cost: 4
dp: 4000
effects:
  - kind: replacement
    trigger: when_would_be_deleted
    optional: true
    active_when:
      all_of:
        - replacement_subject_is_mine: true
        - name_is: "Protected"
    cost: { delete_self: true }
    outcome: prevent
    prevent_all_matching: true
"#;
 let mut r=DebugRunner::builder().from_dsl_yaml(payer).unwrap()
  .add_card(make_test_card("R28-PROTECT","Protected")).add_card(make_test_card("R28-WITNESS","Deleted witness"))
  .add_card(make_test_card("R28-CAUSE","Cause"))
  .deck(0,&["R28-PROTECT";20]).deck(1,&["R28-PROTECT";20]).memory(3).start();
 r.skip_mulligan();r.game.enter_main_phase();
 for id in ["R28-PROTECT","R28-PAYER","R28-PROTECT","R28-CAUSE","R28-WITNESS"]{r.place_on_field(0,id,None);}
 let source=r.game.players[0].battle_area[3].top_card().handle();
 r.register_effect("R28-CAUSE",std::sync::Arc::new(Review28OwnBatch));
 r.register_effect("R28-WITNESS",std::sync::Arc::new(Review28SourceWitness(source)));
 let hand=r.hand_size(0);
 r.game.enqueue_triggered(digimon_engine::enums::EffectTiming::OnPlay,digimon_engine::selection::TriggerSource::Permanent(PermanentHandle{player:0,index:3}));r.game.drain_effect_queue();
 assert_eq!(r.pending_selection_view().unwrap().kind,SelectionKind::Replacement);let saved=r.game.clone();let mut digest=None;
 for _ in 0..2 {r.game=saved.clone();act(&mut r,REPLACEMENT_ACCEPT);r.auto_resolve().unwrap();assert_eq!(r.hand_size(0),hand+1);
  let d=r.game.verification_digest();if let Some(prior)=digest{assert_eq!(d,prior);}digest=Some(d);
 }
}

struct MixedDeletionCauseWitness(u8);
impl digimon_engine::effect::CardEffect for MixedDeletionCauseWitness {
 fn effects(&self,c:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect>{let actor=self.0;
  vec![digimon_engine::effect::Effect::on_deletion(c).process(move|ctx|{
   let expected=if ctx.player==actor {digimon_engine::replacement::ReplacementCause::OwnEffect} else {digimon_engine::replacement::ReplacementCause::OpponentEffect};
   assert_eq!(ctx.deletion_cause(),Some(expected),"each deleted-object snapshot retains controller-relative effect cause");
   ctx.draw(ctx.player,1);
  }).build()]
 }
}
#[test]
fn mixed_controller_deletion_snapshots_use_each_victim_controller() {
 for actor in [0,1] {
  let mut r=fixture(false);r.game.players[0].battle_area.pop();r.place_on_field(1,"DECOY-ALLY",None);
  r.register_effect("DECOY-ALLY",std::sync::Arc::new(MixedDeletionCauseWitness(actor)));
  let hand0=r.hand_size(0);let hand1=r.hand_size(1);let saved=r.game.clone();let mut digest=None;
  for _ in 0..2 {r.game=saved.clone();r.game.set_effect_source_player_for_test(Some(actor));
   let cause=if actor==0 {digimon_engine::replacement::ReplacementCause::OwnEffect}else{digimon_engine::replacement::ReplacementCause::OpponentEffect};
   r.game.delete_permanents_batch(vec![PermanentHandle{player:0,index:0},PermanentHandle{player:1,index:0}],cause);r.auto_resolve().unwrap();
   assert_eq!(r.hand_size(0),hand0+1);assert_eq!(r.hand_size(1),hand1+1);
   let d=r.game.verification_digest();if let Some(prior)=digest {assert_eq!(d,prior);}digest=Some(d);
  }
 }
}

struct LaterSinglePrevention;
impl digimon_engine::effect::CardEffect for LaterSinglePrevention {
 fn effects(&self,c:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect>{
  vec![digimon_engine::effect::Effect::when_would_be_deleted(c).name("Later single prevention").optional()
   .replacement_condition(|ctx,s|matches!(s,digimon_engine::replacement::ReplacementSubject::Permanent(h) if Some(*h)==ctx.source_permanent))
   .replacement_process(|ctx|ctx.cancel()).build()]
 }
}
fn plural_review_fixture()->DebugRunner {
 let mut ally=make_test_card("PLURAL-ALLY","Shaman ally");ally.traits=vec!["Shaman".into()];
 let mut other=make_test_card("PLURAL-OTHER","Iliad ally");other.traits=vec!["Iliad".into()];
 let mut r=DebugRunner::builder().dsl_card("BT25-039").unwrap().add_card(ally).add_card(other)
  .add_card(make_test_card("PLURAL-NO","No matching trait"))
  .deck(0,&["PLURAL-NO";20]).deck(1,&["PLURAL-NO";20]).memory(3).start();
 r.skip_mulligan();r.game.enter_main_phase();
 for id in ["PLURAL-ALLY","BT25-039","PLURAL-OTHER"]{r.place_on_field(0,id,None);}r
}
#[test]
fn failed_plural_payment_does_not_expand_later_singular_cancellation() {
 let mut r=plural_review_fixture();r.register_effect("PLURAL-ALLY",std::sync::Arc::new(LaterSinglePrevention));
 r.game.modifiers.add(PermanentHandle{player:0,index:1},ModifierEntry::simple(ModifierType::CannotBeDestroyedByEffect,0,Expiry::Permanent,0));
 delete_two(&mut r);let view=r.pending_selection_view().expect("choose either immediate effect");
 let rows=view.effect_choices.as_ref().unwrap();assert_eq!(rows.len(),2);
 let plural=rows.iter().find(|c|c.source_card==Some(r.game.players[0].battle_area[1].top_card().handle())).expect("real Sirenmon physical source").action_id;
 act(&mut r,plural);let saved=r.game.clone();let mut digest=None;
 for _ in 0..2 {r.game=saved.clone();act(&mut r,REPLACEMENT_ACCEPT);
  assert_eq!(r.game.players[0].battle_area.len(),3,"unpayable self-deletion moves no card");
  let view=r.pending_selection_view().expect("the later singular effect remains available");
  assert_eq!(view.kind,SelectionKind::Replacement);act(&mut r,REPLACEMENT_ACCEPT);r.auto_resolve().unwrap();
  assert_eq!(ids(&r),vec!["PLURAL-ALLY","BT25-039"],"unpaid plural candidate cannot lend its member set to a later singular cancel");
  let d=r.game.verification_digest();if let Some(prior)=digest{assert_eq!(d,prior);}digest=Some(d);
 }
}
#[test]
fn sirenmon_own_effect_batch_does_not_offer_plural_protection() {
 let mut r=plural_review_fixture();r.game.set_effect_source_player_for_test(Some(0));
 r.game.delete_permanents_batch(vec![PermanentHandle{player:0,index:0},PermanentHandle{player:0,index:2}],digimon_engine::replacement::ReplacementCause::OwnEffect);
 assert!(r.game.pending_selection.is_none());assert_eq!(ids(&r),vec!["BT25-039"]);
}
#[test]
fn sirenmon_plural_protection_excludes_nonmatching_member() {
 let mut r=plural_review_fixture();r.place_on_field(0,"PLURAL-NO",None);r.game.set_effect_source_player_for_test(Some(1));
 r.game.delete_permanents_batch(vec![PermanentHandle{player:0,index:0},PermanentHandle{player:0,index:2},PermanentHandle{player:0,index:3}],digimon_engine::replacement::ReplacementCause::OpponentEffect);
 let saved=r.game.clone();let mut digest=None;
 for _ in 0..2 {r.game=saved.clone();act(&mut r,REPLACEMENT_ACCEPT);r.auto_resolve().unwrap();assert_eq!(ids(&r),vec!["PLURAL-ALLY","PLURAL-OTHER"]);
  assert!(r.game.players[0].trash.iter().any(|c|c.card_id(&r.game.card_data)=="PLURAL-NO"));
  let d=r.game.verification_digest();if let Some(prior)=digest{assert_eq!(d,prior);}digest=Some(d);
 }
}

#[test]
fn queued_payer_on_deletion_does_not_stale_post_payment_color_eligibility() {
    for grant_depends_on_payer in [false, true] {
        let mut r = fixture(false);
        r.register_effect("DECOY-COST",std::sync::Arc::new(OwnCostWitness));
        let hand_before=r.hand_size(0);
        // Victim zero is printed green; temporarily make it blue, then grant
        // green only while the Decoy payer is physically present.
        let victim = PermanentHandle { player: 0, index: 0 };
        let payer = r.game.players[0].battle_area[1].top_card().handle();
        r.game.modifiers.add(victim,
            ModifierEntry::simple(ModifierType::ChangeBaseCardColor, 0, Expiry::Permanent, 0)
                .with_payload(digimon_engine::modifiers::ModifierPayload::Colors {
                    value: vec![CardColor::Blue], base: true,
                }));
        r.game.modifiers.add(victim,
            ModifierEntry::simple(ModifierType::AddColor, 0, Expiry::UntilCondition, 0)
                .with_payload(digimon_engine::modifiers::ModifierPayload::Colors {
                    value: vec![CardColor::Green], base: false,
                })
                .with_until_condition(std::sync::Arc::new(move |g, _| {
                    !grant_depends_on_payer || g.players[0].battle_area.iter()
                        .any(|p| p.top_card().handle() == payer)
                })));
        r.place_on_field(0, "DECOY-GREEN5", None);
        assert!(r.game.players[0].battle_area[0]
            .colors_for_rules(&r.game.card_data, &r.game.modifiers, victim)
            .contains(&CardColor::Green));
        delete_two(&mut r);
        let saved = r.game.clone();
        for _ in 0..2 {
            r.game = saved.clone();
            act(&mut r, REPLACEMENT_ACCEPT);
            let view = r.pending_selection_view().expect("post-payment target choice");
            assert_eq!(view.kind, SelectionKind::OwnField);
            let stale = digimon_engine::action::space::encode_attack(0, 0);
            let valid = digimon_engine::action::space::encode_attack(0, 1);
            assert_eq!(r.game.players[0].battle_area[0]
                .colors_for_rules(&r.game.card_data, &r.game.modifiers, victim)
                .contains(&CardColor::Green), !grant_depends_on_payer,
                "the payment has removed the physical source of the temporary grant");
            assert_eq!(r.hand_size(0),hand_before,"queued OnDeletion has not resolved before payment choice");
            let mask = build_action_mask(&r.game, 0);
            assert_eq!(mask[stale as usize], if grant_depends_on_payer { 0.0 } else { 1.0 });
            assert_eq!(mask[valid as usize], 1.0);
            assert_eq!(view.valid_action_ids.len(), if grant_depends_on_payer { 1 } else { 2 });
            act(&mut r, valid);
            r.auto_resolve().unwrap();
            assert_eq!(ids(&r), vec!["DECOY-GREEN5"]);
        }
    }
}

// Official rule Q228/A228 permits a Decoy payer also threatened in this event.
#[test]
fn simultaneously_threatened_decoy_payer_can_pay_and_save_matching_ally() {
 let mut r=fixture(false);r.game.set_effect_source_player_for_test(Some(1));
 r.game.delete_permanents_batch(vec![PermanentHandle{player:0,index:0},PermanentHandle{player:0,index:1}],digimon_engine::replacement::ReplacementCause::OpponentEffect);
 assert_eq!(r.pending_selection_view().unwrap().kind,SelectionKind::Replacement);let saved=r.game.clone();
 for _ in 0..2 {r.game=saved.clone();act(&mut r,REPLACEMENT_ACCEPT);
  if let Some(v)=r.pending_selection_view(){if v.kind==SelectionKind::OwnField {assert_eq!(v.valid_action_ids.len(),1);act(&mut r,v.valid_action_ids[0]);}}
  r.auto_resolve().unwrap();assert_eq!(ids(&r),vec!["DECOY-ALLY"]);
  assert_eq!(r.game.players[0].trash.iter().filter(|c|c.card_id(&r.game.card_data)=="DECOY-COST").count(),1);
 }
}
