//! Separate capability probe for EX2-007 Q3287: moving a Digimon CARD from
//! the battle area under another Digimon moves only its top; sources trash.
//! Inline PROBE card is test instrumentation, not a partial Mother YAML.
use digimon_engine::action::{
    mask::build_action_mask,
    space::{encode_source_select, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS},
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
const PROBE: &str = r#"
card: PROBE-HOST
name: Source placement capability probe
kind: digimon
color: [white]
level: 3
cost: 3
dp: 2000
effects:
  - when: main_on_field
    process:
      - select_zone_cards:
          zones: [battle_area]
          filter: { name_is: Probe donor }
          min: 1
          max: 1
          optional_zero: true
          exclude: source
          bind_as: selected
          prompt: Place a Digimon card underneath this Digimon
      - place_cards_as_bottom_sources: { cards: selected, target: source }
"#;
fn setup(with_source: bool) -> DebugRunner { setup_with_protection(with_source, None) }
fn setup_with_protection(with_source: bool, protection: Option<bool>) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(PROBE)
        .expect("existing placement DSL")
        .add_card(make_test_card("PROBE-DONOR", "Probe donor"))
        .add_card(make_test_card("PROBE-SOURCE", "Probe ordinary source"))
        .deck(0, &["PROBE-SOURCE"; 8])
        .deck(1, &["PROBE-SOURCE"; 8])
        .memory(3)
        .start();
    if let Some(optional) = protection { protect(&mut r, optional); }
    r.skip_mulligan();
    r.place_on_field(0, "PROBE-HOST", Some(0));
    if with_source {
        r.place_stack(0, &["PROBE-SOURCE", "PROBE-DONOR"]);
    } else {
        r.place_on_field(0, "PROBE-DONOR", Some(0));
    }
    r.game.enter_main_phase();
    let action = FIELD_EFFECT_START + FIELD_EFFECT_SLOT_FOR_MAIN;
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.game.decode_action(action, 0);
    r
}
fn pick(r: &mut DebugRunner, source_index: u16) {
    let action = encode_source_select(1, source_index).unwrap();
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.execute_action(0, action).unwrap();
}
fn assert_seated(r: &DebugRunner, trash_count: usize) {
    assert!(r.pending_kind().is_none());
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "donor leaves as a whole permanent"
    );
    let host = &r.game.players[0].battle_area[0];
    assert_eq!(
        host.card_sources.len(),
        2,
        "only donor top enters receiving stack"
    );
    assert_eq!(
        host.card_sources[0].card_id(&r.game.card_data),
        "PROBE-DONOR"
    );
    assert_eq!(
        r.game.players[0].trash.len(),
        trash_count,
        "donor's old sources are disposed, never promoted"
    );
    if trash_count == 1 {
        assert_eq!(
            r.game.players[0].trash[0].card_id(&r.game.card_data),
            "PROBE-SOURCE"
        );
    }
}
#[test]
fn top_only_field_placement_discards_donor_sources() {
    let mut r = setup(true);
    pick(&mut r, 1);
    assert_seated(&r, 1);
}
#[test]
fn stackless_field_placement_control() {
    let mut r = setup(false);
    pick(&mut r, 0);
    assert_seated(&r, 0);
}
#[test]
fn field_placement_decline_keeps_entire_donor() {
    let mut r = setup(true);
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 1.0);
    r.execute_action(0, PASS).unwrap();
    assert_eq!(r.game.players[0].battle_area.len(), 2);
    assert_eq!(r.game.players[0].battle_area[1].card_sources.len(), 2);
    assert!(r.game.players[0].trash.is_empty());
}
#[test]
fn field_placement_clone_preserves_independent_source_disposal() {
    let mut r = setup(true);
    let saved = r.game.clone();
    pick(&mut r, 1);
    assert_seated(&r, 1);
    r.game = saved;
    r.execute_action(0, PASS).unwrap();
    assert_eq!(r.game.players[0].battle_area.len(), 2);
    assert!(r.game.players[0].trash.is_empty());
}

struct ProtectDeparture {
    optional: bool,
}
impl digimon_engine::effect::CardEffect for ProtectDeparture {
    fn effects(
        &self,
        c: digimon_engine::card_source::CardHandle,
    ) -> Vec<digimon_engine::effect::Effect> {
        let e = digimon_engine::effect::Effect::when_would_leave_battle_area(c)
            .name("Protect donor departure")
            // This fixture protects its own donor, not every donor on the field.
            .replacement_condition(|ctx, subject| {
                subject.permanent().is_some_and(|h| Some(h) == ctx.source_permanent)
            });
        let e = if self.optional { e.optional() } else { e };
        vec![e.replacement_process(|ctx| ctx.cancel()).build()]
    }
}
fn protect(r: &mut DebugRunner, optional: bool) {
    r.register_effect(
        "PROBE-DONOR",
        std::sync::Arc::new(ProtectDeparture { optional }),
    );
}
fn assert_retained(r: &DebugRunner) {
    assert_eq!(r.game.players[0].battle_area.len(), 2);
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 1);
    assert_eq!(r.game.players[0].battle_area[1].card_sources.len(), 2);
    assert!(r.game.players[0].trash.is_empty());
}
#[test]
fn mandatory_leave_replacement_preserves_whole_donor() {
    let mut r = setup_with_protection(true, Some(false));
    pick(&mut r, 1);
    assert_retained(&r);
}
#[test]
fn optional_leave_replacement_accept_cancels_and_clone_decline_commits_once() {
    let mut r = setup_with_protection(true, Some(true));
    pick(&mut r, 1);
    assert!(
        r.pending_kind().is_some(),
        "field-top move must offer actual leave replacement"
    );
    let accept = r
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids
        .iter()
        .copied()
        .find(|a| *a != PASS)
        .unwrap();
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 1.0);
    let saved = r.game.clone();
    r.execute_action(0, accept).unwrap();
    assert_retained(&r);
    r.game = saved;
    r.execute_action(0, PASS).unwrap();
    assert_seated(&r, 1);
}

#[test]
fn multi_donor_batch_waits_for_each_replacement_and_tail_runs_once() {
    let yaml = PROBE
        .replace("min: 1", "min: 2")
        .replace("max: 1", "max: 2")
        + "      - gain_memory: 1\n";
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .unwrap()
        .add_card(make_test_card("PROBE-DONOR", "Probe donor"))
        .add_card(make_test_card("PROBE-SOURCE", "Probe ordinary source"))
        .deck(0, &["PROBE-SOURCE"; 8])
        .deck(1, &["PROBE-SOURCE"; 8])
        .memory(3)
        .start();
    r.skip_mulligan();
    r.place_on_field(0, "PROBE-HOST", Some(0));
    r.place_stack(0, &["PROBE-SOURCE", "PROBE-DONOR"]);
    r.place_stack(0, &["PROBE-SOURCE", "PROBE-DONOR"]);
    protect(&mut r, true);
    r.game.enter_main_phase();
    r.game
        .decode_action(FIELD_EFFECT_START + FIELD_EFFECT_SLOT_FOR_MAIN, 0);
    for field in [1, 2] {
        let a = encode_source_select(field, 1).unwrap();
        assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
        r.execute_action(0, a).unwrap();
    }
    assert_eq!(
        r.game.players[0].battle_area.len(),
        3,
        "second donor cannot move before first leave choice"
    );
    assert_eq!(r.memory(), 3);
    let first = r.game.clone();
    r.execute_action(0, PASS).unwrap();
    assert_eq!(r.game.players[0].battle_area.len(), 2);
    assert_eq!(r.game.players[0].trash.len(), 1);
    assert_eq!(r.memory(), 3, "tail waits for second donor");
    assert!(r.pending_kind().is_some());
    r.execute_action(0, PASS).unwrap();
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 3);
    assert_eq!(r.game.players[0].trash.len(), 2);
    assert_eq!(r.memory(), 4);
    r.game = first;
    for _ in 0..2 {
        let a = r
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids
            .iter()
            .copied()
            .find(|a| *a != PASS)
            .unwrap();
        r.execute_action(0, a).unwrap();
    }
    assert_eq!(r.game.players[0].battle_area.len(), 3);
    assert!(r.game.players[0].trash.is_empty());
    assert_eq!(r.memory(), 4);
}
#[test]
fn donor_before_recipient_uses_same_physical_host_after_compaction() {
    let mut r = setup(true);
    r.execute_action(0, PASS).unwrap();
    r.game.players[0].battle_area.swap(0, 1);
    let host = r.game.players[0].battle_area[1].top_card().handle();
    let donor = r.game.players[0].battle_area[0].top_card().handle();
    let moved = r.game.place_as_bottom_source(
        digimon_engine::enums::CardSourceRef::Material(
            digimon_engine::permanent::PermanentHandle {
                player: 0,
                index: 0,
            },
            1,
        ),
        digimon_engine::permanent::PermanentHandle {
            player: 0,
            index: 1,
        },
        false,
    );
    assert!(moved);
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(r.game.players[0].battle_area[0].top_card().handle(), host);
    assert_eq!(
        r.game.players[0].battle_area[0].card_sources[0].handle(),
        donor
    );
    assert_eq!(r.game.players[0].trash.len(), 1);
}
#[test]
fn invalid_receiving_host_rolls_back_without_disposal() {
    let mut r = setup(true);
    r.execute_action(0, PASS).unwrap();
    let before = r.game.verification_digest();
    assert!(!r.game.place_as_bottom_source(
        digimon_engine::enums::CardSourceRef::Material(
            digimon_engine::permanent::PermanentHandle {
                player: 0,
                index: 1
            },
            1
        ),
        digimon_engine::permanent::PermanentHandle {
            player: 0,
            index: 13
        },
        false
    ));
    assert_retained(&r);
    assert_eq!(r.game.verification_digest(), before);
}
#[test]
fn ordinary_below_top_source_extraction_does_not_remove_donor() {
    let mut r = setup(true);
    r.execute_action(0, PASS).unwrap();
    assert!(r.game.place_as_bottom_source(
        digimon_engine::enums::CardSourceRef::Material(
            digimon_engine::permanent::PermanentHandle {
                player: 0,
                index: 1
            },
            0
        ),
        digimon_engine::permanent::PermanentHandle {
            player: 0,
            index: 0
        },
        false
    ));
    assert_eq!(r.game.players[0].battle_area.len(), 2);
    assert_eq!(r.game.players[0].battle_area[1].card_sources.len(), 1);
    assert!(r.game.players[0].trash.is_empty());
}
#[test]
fn only_disposed_sources_overflow_and_route_to_their_physical_owner() {
    let mut r = setup(true);
    let donor = &mut r.game.players[0].battle_area[1];
    donor.card_sources[0].owner = 1;
    let source_data = donor.card_sources[0].data_index;
    let top_data = donor.top_card().data_index;
    r.game.card_data[source_data].ace_overflow = Some(-2);
    r.game.card_data[top_data].ace_overflow = Some(-5);
    pick(&mut r, 1);
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert!(r.game.players[0].trash.is_empty());
    assert_eq!(r.game.players[1].trash.len(), 1);
    assert_eq!(
        r.memory(),
        5,
        "opponent-owned source loses 2; moved top stays under a card and has no Overflow"
    );
}

fn setup_followup(with_source: bool, registration: Option<(&str, std::sync::Arc<dyn digimon_engine::effect::CardEffect>)>) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(PROBE)
        .expect("existing placement DSL")
        .add_card(make_test_card("PROBE-DONOR", "Probe donor"))
        .add_card(make_test_card("PROBE-OBSERVER", "Leave observer"))
        .add_card(make_test_card("PROBE-SOURCE", "Probe ordinary source"))
        .deck(0, &["PROBE-SOURCE"; 8])
        .deck(1, &["PROBE-SOURCE"; 8])
        .memory(3)
        .start();
    let add_observer=registration.as_ref().is_some_and(|(id,_)|*id=="PROBE-OBSERVER");
    if let Some((id,effect))=registration { r.register_effect(id,effect); }
    r.skip_mulligan();
    r.place_on_field(0, "PROBE-HOST", Some(0));
    if with_source {
        r.place_stack(0, &["PROBE-SOURCE", "PROBE-DONOR"]);
    } else {
        r.place_on_field(0, "PROBE-DONOR", Some(0));
    }
    if add_observer { r.place_on_field(0,"PROBE-OBSERVER",Some(0)); }
    r.game.enter_main_phase();
    let action = FIELD_EFFECT_START + FIELD_EFFECT_SLOT_FOR_MAIN;
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.game.decode_action(action, 0);
    r
}

#[test]
fn receiving_host_leaves_and_same_physical_card_reenters_does_not_receive_donor(){
    let mut r=setup_followup(true,Some(("PROBE-DONOR",std::sync::Arc::new(ProtectDeparture { optional:true }))));pick(&mut r,1);
    let old=r.game.players[0].battle_area.remove(0);
    let reentered=digimon_engine::Permanent::new(old.top_card().clone(), r.game.turn_count);
    r.game.players[0].battle_area.insert(0,reentered);
    r.execute_action(0,PASS).unwrap();
    assert_retained(&r);
    assert_eq!(r.game.players[0].battle_area[0].top_card().handle(),old.top_card().handle());
}
struct LeaveObserver;
impl digimon_engine::effect::CardEffect for LeaveObserver{
 fn effects(&self,c:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect>{
  use digimon_engine::effect::EffectBuilder;
  use digimon_engine::enums::EffectTiming;
  vec![EffectBuilder::new(c,EffectTiming::OnLeaveField).process(|ctx|{
   assert_eq!(ctx.game.players[0].battle_area.len(),2);
   assert_eq!(ctx.game.players[0].battle_area[0].card_sources.len(),2);
   assert_eq!(ctx.game.players[0].trash.len(),1);
   ctx.gain_memory(1);
  }).build()]
 }
}
#[test]
fn leave_observer_sees_final_source_disposal_and_recipient_seating(){
 let mut r=setup_followup(true,Some(("PROBE-OBSERVER",std::sync::Arc::new(LeaveObserver))));
 pick(&mut r,1);assert!(r.pending_kind().is_none());assert_eq!(r.game.players[0].battle_area.len(),2);assert_eq!(r.game.players[0].battle_area[0].card_sources.len(),2);assert_eq!(r.game.players[0].trash.len(),1);assert_eq!(r.memory(),4);
}

struct SourceDisposalObserver;
impl digimon_engine::effect::CardEffect for SourceDisposalObserver {
 fn effects(&self,c:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect>{
  vec![digimon_engine::effect::Effect::on_digivolution_card_trashed(c).inherited().process(|ctx|{
   assert_eq!(ctx.event_cause(),Some(digimon_engine::trigger_context::EventCause::Rule),"old sources are disposed by rule, not directly by placement effect");
   assert_eq!(ctx.game.players[0].battle_area.len(),1);
   assert_eq!(ctx.game.players[0].battle_area[0].card_sources.len(),2);
   assert_eq!(ctx.game.players[0].trash.len(),1);
   ctx.gain_memory(1);
  }).build()]
 }
}
#[test]
fn source_trash_observer_sees_completed_disposition_with_rule_cause(){
 let mut r=setup_followup(true,Some(("PROBE-SOURCE",std::sync::Arc::new(SourceDisposalObserver))));
 pick(&mut r,1);assert_seated(&r,1);assert_eq!(r.memory(),4);
}
