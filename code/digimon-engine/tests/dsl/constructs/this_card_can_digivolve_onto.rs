//! `this_card_can_digivolve_onto` (G-DSL-THIS-CARD-CAN-DIGIVOLVE-ONTO): a permanent-subject leaf that holds when
//! THIS card (the effect's own card: in the hand for a `[Hand] [Main]`, in the trash for a `{Trash}` effect) can
//! digivolve onto the subject the way the consuming `effect_initiated_digivolve { from_hand: self, cost, ignore_* }`
//! would: the route set the waiver leaves, the "can't digivolve" prohibitions, and the payment at `cost`.
//!
//! DCGO gates both the activation and the pick of these effects on `card.CanPlayCardTargetFrame(
//! permanent.PermanentFrame, payCost, activateClass, fixedCost, ignore)` (BT22_013.cs:50-68, BT22_026.cs:43-60,
//! P_250.cs:54-59, BT20_084.cs:76-118); onto a Digimon that checks the route and the prohibitions (`CanEvolve`,
//! CardSource.cs:1166) but not memory, which DCGO compares only for an empty frame (CardSource.cs:1150-1162). The
//! payment half comes from the rules: a digivolve whose cost can't be paid can't happen (CR 4.3 1-3-11-1; the gauge
//! stops at the opponent's 10, 1-4-2-2). It is a KNOWN DCGO DIVERGENCE (rules over DCGO), so an effect offered with
//! no such target is a choice with no outcome.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Expiry, GamePhase, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;

const CARD: &str = "TEST-THIS-CARD";

/// A Lv.5 red Digimon whose only route is "red Lv.4: cost 3", with a `[Hand] [Main]` that digivolves THIS card onto
/// a Digimon passing `leaf`, by an `effect_initiated_digivolve` with `cost` and `step` (its waiver lines).
fn yaml(leaf: &str, cost: &str, step: &str) -> String {
    format!(
        r#"
card: {CARD}
name: This card
kind: digimon
level: 5
color: [red]
cost: 9
dp: 9000
alt_paths:
  - kind: digivolve
    from: {{ level_eq: 4, color_is: red }}
    cost: 3
effects:
  - when: main_from_hand
    condition:
      any_permanent:
        of: you
        zone: [battle_area]
        kind: digimon
        this_card_can_digivolve_onto: {leaf}
    process:
      - select_own_permanent:
          bind_as: base
          filter:
            all_of:
              - kind: digimon
              - this_card_can_digivolve_onto: {leaf}
          prompt: Choose a Digimon this card digivolves onto
      - effect_initiated_digivolve:
          target: base
          from_hand: self
          cost: {cost}
{step}"#
    )
}

fn host(id: &str, level: u8, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(4000);
    c.colors = vec![color];
    c
}

/// Main phase, player 0 to act at `memory`, THIS card in hand, `hosts` on the field.
fn runner(yaml: &str, hosts: &[&str], memory: i16) -> (DebugRunner, Vec<PermanentHandle>) {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("test card compiles")
        .add_card(host("RED4", 4, CardColor::Red))
        .add_card(host("RED3", 3, CardColor::Red))
        .add_card(host("BLUE4", 4, CardColor::Blue))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &[CARD])
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .start();
    r.set_first_player(0);
    r.skip_mulligan();
    r.set_phase(GamePhase::Main);
    let hs = hosts.iter().map(|id| r.place_on_field(0, id, Some(0))).collect();
    r.game.set_memory(memory);
    (r, hs)
}

fn offered(r: &DebugRunner) -> usize {
    r.pending_selection_view()
        .map(|v| v.valid_action_ids.iter().filter(|a| **a != digimon_engine::action::space::PASS).count())
        .unwrap_or(0)
}

fn top(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.players[0].battle_area[h.index as usize].top_card().card_id(&r.game.card_data).to_string()
}

const IGNORING: &str = "{ cost: 6, ignore_requirements: true }";
const IGNORING_STEP: &str = "          ignore_requirements: true\n";

/// `yaml(IGNORING, "6", IGNORING_STEP)` written out: "1 of your Digimon digivolves into this card for a digivolution
/// cost of 6, ignoring digivolution requirements" (the BT22-013 / BT22-026 shape).
const IGNORING_YAML: &str = r#"
card: TEST-THIS-CARD
name: This card
kind: digimon
level: 5
color: [red]
cost: 9
dp: 9000
alt_paths:
  - kind: digivolve
    from: { level_eq: 4, color_is: red }
    cost: 3
effects:
  - when: main_from_hand
    condition:
      any_permanent:
        of: you
        zone: [battle_area]
        kind: digimon
        this_card_can_digivolve_onto: { cost: 6, ignore_requirements: true }
    process:
      - select_own_permanent:
          bind_as: base
          filter:
            all_of:
              - kind: digimon
              - this_card_can_digivolve_onto: { cost: 6, ignore_requirements: true }
          prompt: Choose a Digimon this card digivolves onto
      - effect_initiated_digivolve:
          target: base
          from_hand: self
          cost: 6
          ignore_requirements: true
"#;

#[test]
fn the_written_out_fixture_matches_the_template() {
    assert_eq!(IGNORING_YAML, yaml(IGNORING, "6", IGNORING_STEP));
}

/// "Digivolves into this card for a digivolution cost of 6, ignoring digivolution requirements": any Digimon is a
/// route (BLUE4 has no circle), and 6 is payable down to memory -4 (to the opponent's 10).
#[test]
fn fixed_cost_ignoring_requirements_is_offered_while_payable() {
    let (mut r, hs) = runner(IGNORING_YAML, &["BLUE4"], -4);
    assert!(r.game.activate_hand_main(0, 0), "6 from -4 reaches -10: payable");
    assert_eq!(offered(&r), 1);
    let v = r.pending_selection_view().unwrap();
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(top(&r, hs[0]), CARD, "digivolved, requirements ignored");
}

/// One below: 6 from -5 would pass the opponent's 10 (CR 4.3 1-4-2-2), so the effect is not offered at all.
#[test]
fn fixed_cost_ignoring_requirements_is_not_offered_when_unpayable() {
    let (mut r, hs) = runner(IGNORING_YAML, &["BLUE4"], -5);
    assert!(!r.game.activate_hand_main(0, 0), "6 from -5 can't be paid (CR 4.3 1-3-11-1)");
    assert!(r.game.pending_selection.is_none());
    assert_eq!(top(&r, hs[0]), "BLUE4");
}

/// The prohibitions: a Digimon that can't digivolve is no target, the others still are.
#[test]
fn a_digimon_that_cannot_digivolve_is_no_target() {
    let (mut r, hs) = runner(IGNORING_YAML, &["BLUE4", "RED3"], 5);
    r.game.modifiers.add(hs[0], ModifierEntry::simple(ModifierType::CannotDigivolve, 0, Expiry::Permanent, 0));
    assert!(r.game.activate_hand_main(0, 0));
    assert_eq!(offered(&r), 1, "only RED3");
    let v = r.pending_selection_view().unwrap();
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(top(&r, hs[1]), CARD);
    assert_eq!(top(&r, hs[0]), "BLUE4");
}

/// A requirement-keeping digivolve at the printed cost: only a Digimon with a route (red Lv.4) is a target, and the
/// route's own cost (3) must be payable.
#[test]
fn printed_cost_keeps_the_route_and_its_cost() {
    let (mut r, hs) = runner(&yaml("{ cost: printed }", "printed", ""), &["RED3", "BLUE4", "RED4"], 5);
    assert!(r.game.activate_hand_main(0, 0));
    assert_eq!(offered(&r), 1, "only RED4 has a route");
    let v = r.pending_selection_view().unwrap();
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(top(&r, hs[2]), CARD);

    let (mut r, _) = runner(&yaml("{ cost: printed }", "printed", ""), &["RED4"], -8);
    assert!(!r.game.activate_hand_main(0, 0), "3 from -8 can't be paid");
}

/// `free` checks the route and the prohibitions, never the gauge.
#[test]
fn free_checks_no_payment() {
    let (mut r, _) = runner(&yaml("{ cost: free }", "free", ""), &["RED4"], -10);
    assert!(r.game.activate_hand_main(0, 0), "a free digivolve needs no memory");
    let (mut r, _) = runner(&yaml("{ cost: free }", "free", ""), &["BLUE4"], 5);
    assert!(!r.game.activate_hand_main(0, 0), "free, but still no route onto BLUE4");
}

/// `ignore_level` and `ignore_requirements` are exclusive, as on the step.
#[test]
fn ignore_level_and_ignore_requirements_together_is_a_compile_error() {
    let yaml = yaml("{ cost: 6, ignore_level: true, ignore_requirements: true }", "6", IGNORING_STEP);
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(&yaml).expect("parses");
    let Err(err) = digimon_dsl::CardRegistry::from_specs("test", &[spec]) else {
        panic!("ignore_level + ignore_requirements must not compile");
    };
    assert!(format!("{err:?}").contains("this_card_can_digivolve_onto"), "{err:?}");
}
