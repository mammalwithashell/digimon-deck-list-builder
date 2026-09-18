//! G-DSL-DIGIVOLVE-FROM-UNION-WITH-SOURCE-TRASH-COST — the two predicate
//! leaves that close DCGO's `DigivolveIntoHandOrTrashCard` gates for a
//! digivolve whose TARGET is a bound permanent (not the effect's source):
//!
//! - `can_digivolve_onto: <binding>` (card subject) — DCGO `ValidTarget` /
//!   `cardSource.CanPlayCardTargetFrame(selectedPermanent.PermanentFrame, …)`:
//!   the candidate has ≥1 normal-digivolve route onto the bound permanent.
//! - `has_digivolve_candidate: { of, zone: [hand|trash], filter }` (permanent
//!   subject) — DCGO `CanDigivolveDigimon(permanent)`: some hand/trash card
//!   matching `filter` can digivolve onto this permanent.
//!
//! Driver: BT25-092 Asuna Shiroki `[Main]` (`tests/cards_behavioral/bt25/
//! bt25_092.rs` proves the end-to-end flow). This file pins the compile /
//! validate contract and the leaves' semantics on an inline fixture.

use digimon_dsl::compiled::{CompiledClause, CompiledPlayerRef, CompiledStep, CompiledZone};
use digimon_engine::action::space::{encode_attack, PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl::raw_rust_registry::StubRegistry;
use digimon_engine::dsl::validator::{validate, ValidationContext};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};

const FIXTURE: &str = r#"
card: TEST-DIGI-TARGET
name: Digivolve Target Gates
kind: tamer
color: [purple]
cost: 2
effects:
  - when: main_on_field
    summary: "1 of your Digimon may digivolve into a purple card from hand or trash"
    process:
      - select_own_permanent:
          bind_as: target
          optional: true
          filter:
            all_of:
              - kind: digimon
              - has_digivolve_candidate:
                  of: you
                  zone: [hand, trash]
                  filter: { color_is: purple }
          prompt: "Select a Digimon to digivolve"
      - select_union_zone:
          of: you
          zones: [hand, trash]
          optional: true
          filter:
            all_of:
              - color_is: purple
              - can_digivolve_onto: target
          bind_as: evo
          prompt: "Digivolve into"
      - effect_initiated_digivolve:
          target: target
          source: evo
          cost: printed
"#;

fn parse(yaml: &str) -> digimon_dsl::CardSpec {
    serde_yml::from_str(yaml).expect("parse yaml")
}

// ---------------------------------------------------------------------------
// Compile / validate contract
// ---------------------------------------------------------------------------

#[test]
fn compile_lowers_both_leaves() {
    let spec = parse(FIXTURE);
    let reg = StubRegistry::empty();
    validate(&spec, &ValidationContext { raw_rust: &reg }).expect("fixture validates");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile");
    let CompiledClause::Triggered(t) = &compiled.effects[0] else {
        panic!("triggered clause");
    };
    let CompiledStep::SelectOwnPermanent { filter, .. } = &t.process[0] else {
        panic!("select_own_permanent first");
    };
    let hdc = filter.all_of[1]
        .has_digivolve_candidate
        .as_ref()
        .expect("has_digivolve_candidate compiled");
    assert_eq!(hdc.of, CompiledPlayerRef::You);
    assert_eq!(hdc.zones, vec![CompiledZone::Hand, CompiledZone::Trash]);
    assert!(hdc.filter.as_ref().is_some_and(|f| f.color_is.is_some()));

    let CompiledStep::SelectUnionZone { filter, .. } = &t.process[1] else {
        panic!("select_union_zone second");
    };
    assert_eq!(
        filter.all_of[1].can_digivolve_onto.as_deref(),
        Some("target"),
        "can_digivolve_onto keeps the binding name"
    );
}

#[test]
fn validator_rejects_undeclared_can_digivolve_onto_binding() {
    let yaml = FIXTURE.replace("can_digivolve_onto: target", "can_digivolve_onto: nope");
    let spec = parse(&yaml);
    let reg = StubRegistry::empty();
    let errs = validate(&spec, &ValidationContext { raw_rust: &reg })
        .expect_err("undeclared binding must be rejected");
    assert!(
        errs.iter().any(|e| e.path.contains("can_digivolve_onto") && e.message.contains("nope")),
        "expected an undeclared-binding error on can_digivolve_onto, got {errs:?}"
    );
}

#[test]
fn validator_rejects_unsupported_has_digivolve_candidate_zone() {
    let yaml = FIXTURE.replace("zone: [hand, trash]", "zone: [deck]");
    let spec = parse(&yaml);
    let reg = StubRegistry::empty();
    let errs = validate(&spec, &ValidationContext { raw_rust: &reg })
        .expect_err("deck is not a digivolve result zone");
    assert!(
        errs.iter()
            .any(|e| e.path.contains("has_digivolve_candidate.zone") && e.message.contains("Deck")),
        "expected a zone error, got {errs:?}"
    );
}

// ---------------------------------------------------------------------------
// Semantics
// ---------------------------------------------------------------------------

fn digimon(id: &str, level: u8, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.colors = vec![color];
    c
}

/// Purple Lv.N with a "purple Lv.(N-1): cost 2" circle.
fn purple_evo(id: &str, level: u8) -> CardData {
    let mut c = digimon(id, level, CardColor::Purple);
    c.evo_costs = vec![EvoCost {
        card_color: CardColor::Purple as u8,
        level: level - 1,
        memory_cost: 2,
    }];
    c
}

fn tamer() -> CardData {
    let mut c = make_test_card("TEST-DIGI-TARGET", "Digivolve Target Gates");
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 2;
    c
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(FIXTURE)
        .expect("fixture loads")
        .add_card(tamer())
        .add_card(digimon("BASE-LV4", 4, CardColor::Purple))
        .add_card(digimon("BASE-LV3", 3, CardColor::Purple))
        .add_card(purple_evo("EVO-LV5", 5))
        .add_card(purple_evo("EVO-LV5-TRASH", 5))
        .add_card(digimon("RED-LV5", 5, CardColor::Red))
        .add_card(make_test_card("FILLER", "FILLER"))
        .deck(0, &["FILLER"; 5])
        .hand(0, &["EVO-LV5", "RED-LV5"])
        .memory(5)
        .start()
}

fn fire_main(r: &mut DebugRunner, tamer_index: usize) {
    let handle = r.perm_handle(0, tamer_index);
    r.game
        .enqueue_triggered(EffectTiming::MainOnField, TriggerSource::Permanent(handle));
    r.game.drain_effect_queue();
}

#[test]
fn has_digivolve_candidate_only_offers_permanents_with_a_routable_result() {
    let mut r = runner();
    let tamer = r.place_on_field(0, "TEST-DIGI-TARGET", Some(0));
    let lv4 = r.place_on_field(0, "BASE-LV4", Some(0));
    let _lv3 = r.place_on_field(0, "BASE-LV3", Some(0));
    r.inject_trash(0, "EVO-LV5-TRASH");
    r.game.enter_main_phase();

    fire_main(&mut r, tamer.index as usize);
    let view = r.pending_selection_view().expect("target prompt installs");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert_eq!(
        view.valid_action_ids,
        vec![encode_attack(0, lv4.index as u16)],
        "only the Lv.4 base has a purple Lv.5 result (hand or trash); the Lv.3 base does not"
    );
}

#[test]
fn can_digivolve_onto_filters_the_union_result_to_routable_cards() {
    let mut r = runner();
    let tamer = r.place_on_field(0, "TEST-DIGI-TARGET", Some(0));
    let lv4 = r.place_on_field(0, "BASE-LV4", Some(0));
    r.inject_trash(0, "EVO-LV5-TRASH");
    r.game.enter_main_phase();

    fire_main(&mut r, tamer.index as usize);
    r.execute_action(0, encode_attack(0, lv4.index as u16))
        .expect("pick the Lv.4 base");
    let view = r.pending_selection_view().expect("result prompt installs");
    let mut offered = view.valid_action_ids.clone();
    offered.sort_unstable();
    let hand_idx = r
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == "EVO-LV5")
        .expect("EVO-LV5 in hand") as u16;
    let trash_idx = r
        .game
        .player(0)
        .trash
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == "EVO-LV5-TRASH")
        .expect("EVO-LV5-TRASH in trash") as u16;
    let mut expected = vec![PLAY_HAND_START + hand_idx, TRASH_EFFECT_START + trash_idx];
    expected.sort_unstable();
    assert_eq!(
        offered, expected,
        "purple Lv.5s from hand AND trash are offered; RED-LV5 (colour filter, no circle) is not"
    );
}

#[test]
fn no_candidate_permanent_means_no_prompt_at_all() {
    // Only a Lv.3 base on the field: neither result card can digivolve onto
    // it, so `has_digivolve_candidate` leaves the target pick empty and the
    // clause ends silently (DCGO's outer `if HasMatchConditionOwnersPermanent`).
    let mut r = runner();
    let tamer = r.place_on_field(0, "TEST-DIGI-TARGET", Some(0));
    let _lv3 = r.place_on_field(0, "BASE-LV3", Some(0));
    r.inject_trash(0, "EVO-LV5-TRASH");
    r.game.enter_main_phase();

    fire_main(&mut r, tamer.index as usize);
    assert!(r.pending_selection().is_none(), "no routable target → no prompt");
    assert_eq!(r.hand_size(0), 2, "nothing consumed");
}
