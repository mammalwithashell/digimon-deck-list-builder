//! The HOST pick (and the activation gate) of a PAID effect digivolve checks the payment, not only the route
//! (G-DSL-PAID-DIGIVOLVE-HOST-PICK-ROUTE-ONLY).
//!
//! "1 of your Digimon may digivolve into a card in the hand with the digivolution cost reduced by N" is authored as a
//! host pick filtered by `has_digivolve_candidate { zone: [hand] }`, a card pick and an `effect_initiated_digivolve`.
//! After G-DSL-PAID-DIGIVOLVE-PICK-PAYMENT the card pick checks the payment, so a host whose only candidates can't be
//! paid was still offered and choosing it left an empty card pick: a dead choice. A card whose cost can't be paid can't
//! be declared (CR 4.3 1-3-11-1; 8-1-2-6: it goes back and memory doesn't move; 4-2-2 / 1-4-2-2: the gauge stops at
//! 10). DCGO's gates have the same shape (`CanPlayCardTargetFrame` on an occupied frame never checks payment,
//! CardSource.cs:1150-1161), so this is a rules fix, not a DCGO divergence; the rules manual governs.
//!
//! The compile pass (`digimon_dsl::digivolve_candidates`) now gives the host leaf the digivolve's `cost` and waiver, and
//! does the same for a clause gate (`condition` / `active_when`) that counts route-only candidates onto the effect's
//! own `source` (`can_digivolve_onto: source`, `can_digivolve_from_source: true`, a `has_digivolve_candidate` on the
//! `is_source` permanent).
//!
//! `has_digivolve_candidate { ignore_requirements: true }` is the host-pick sibling of an `effect_initiated_digivolve
//! { ignore_requirements: true }` ("1 of your [Gabumon] may digivolve into [MetalGarurumon] in the hand, ignoring
//! digivolution requirements", BT22-013; DCGO `CanPlayCardTargetFrame(frame, false, ..., IgnoreRequirement.All)`,
//! BT22_013.cs:145-150): only prohibitions apply (CR 4.3 15-7-5 `CannotDigivolve`), no route is needed.
use digimon_dsl::compiled::{CompiledClause, CompiledStep};
use digimon_engine::action::{build_action_mask, space};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Expiry, GamePhase, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::selection::SelectionKind;

const HOST: &str = "TEST-HOST-PICK";

/// A Lv.3 yellow Digimon costing 3 whose [On Play] runs `process` (optionally behind `condition`).
fn host_card(condition: &str, process: &str) -> String {
    format!(
        r#"
card: {HOST}
name: Host pick source
kind: digimon
level: 3
color: [yellow]
cost: 3
dp: 3000
effects:
  - when: on_play
{condition}    process:
{process}
"#
    )
}

/// The host pick (leaf authored by the card, route-only), the card pick and the digivolve.
fn host_pick(leaf: &str, cost: &str, step: &str) -> String {
    format!(
        r#"      - select_own_permanent:
          bind_as: host
          optional: true
          filter:
            all_of:
              - kind: digimon
              - has_digivolve_candidate: {leaf}
          prompt: 1 of your Digimon may digivolve
      - select_hand:
          of: you
          bind_as: result
          optional: true
          filter: {{kind: digimon}}
          prompt: Choose the card to digivolve into
      - effect_initiated_digivolve:
          target: host
          from_hand: result
          cost: {cost}
{step}"#
    )
}

fn digimon(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.colors = vec![color];
    c
}

/// Lv.4 result with a yellow Lv.3 circle at `yellow` and a red Lv.3 circle at `red`.
fn result(id: &str, yellow: u16, red: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.colors = vec![CardColor::Yellow];
    c.play_cost = 9;
    c.evo_costs = vec![
        EvoCost { card_color: CardColor::Yellow as u8, level: 3, memory_cost: yellow },
        EvoCost { card_color: CardColor::Red as u8, level: 3, memory_cost: red },
    ];
    c
}

/// Lv.6 blue card with NO circle onto any host: only an "ignoring digivolution requirements" digivolve can use it.
fn no_route(id: &str) -> CardData {
    let mut c = make_test_card(id, "Ignorer");
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(12000);
    c.colors = vec![CardColor::Blue];
    c.play_cost = 13;
    c.evo_costs = Vec::new();
    c
}

/// A runner in its main phase at memory 0, the host first in hand and `hand` after it. Two own Digimon stand on the
/// field first: a yellow one (`YEL`, battle area 0) and a red one (`RED`, battle area 1).
fn runner(yaml: &str, hand: &[&str]) -> DebugRunner {
    let mut cards = vec![HOST];
    cards.extend_from_slice(hand);
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("host pick source compiles")
        // Payable from -3 (at most 7) only onto the red Digimon once reduced by 1; onto yellow 9-1=8 is one too many.
        .add_card(result("RES-A", 9, 2))
        .add_card(result("RES-B", 8, 9))
        .add_card(result("RES-FREE", 9, 9))
        .add_card(no_route("IGNORER"))
        .add_card(digimon("YEL", CardColor::Yellow))
        .add_card(digimon("RED", CardColor::Red))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &cards)
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .start();
    r.set_first_player(0);
    r.skip_mulligan();
    r.set_phase(GamePhase::Main);
    r.place_on_field(0, "YEL", Some(0));
    r.place_on_field(0, "RED", Some(0));
    r.game.set_memory(0);
    r
}

/// Play the host (hand[0]) so its [On Play] runs with the memory left after paying its play cost (-3).
fn play_host(r: &mut DebugRunner) {
    assert_eq!(build_action_mask(&r.game, 0)[space::PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(space::PLAY_HAND_START, 0);
}

fn candidates(r: &DebugRunner) -> usize {
    r.pending_selection_view()
        .map(|v| v.valid_action_ids.iter().filter(|a| **a != space::PASS).count())
        .unwrap_or(0)
}

fn pick_first(r: &mut DebugRunner) {
    let v = r.pending_selection_view().expect("a pick");
    let a = *v.valid_action_ids.iter().find(|a| **a != space::PASS).expect("a candidate");
    r.execute_action(0, a).unwrap();
}

fn top_of(r: &DebugRunner, index: usize) -> String {
    r.game.players[0].battle_area[index].top_card().card_id(&r.game.card_data).to_string()
}

fn mine(r: &DebugRunner) -> i16 {
    if r.game.turn_player() == 0 { r.memory() } else { -r.memory() }
}

fn compile_errors(yaml: &str) -> Vec<String> {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parses");
    match digimon_dsl::compile::compile(&spec) {
        Ok(_) => Vec::new(),
        Err(errors) => errors.into_iter().map(|e| format!("{}: {}", e.path, e.message)).collect(),
    }
}

// ---------------------------------------------------------------------------------------------------------------
// The host pick of a paid digivolve
// ---------------------------------------------------------------------------------------------------------------

/// The leaf is authored route-only (the LM-027 / LM-055 / BT25-092 shape). From -3 at most 7 can be paid: RES-A onto
/// the yellow Digimon (and the yellow host itself) costs 9-1=8, onto the red one 2-1=1.
#[test]
fn route_only_host_leaf_offers_only_hosts_with_a_payable_candidate() {
    let yaml = host_card("", &host_pick("{of: you, zone: [hand]}", "{reduce: 1}", ""));
    let mut r = runner(&yaml, &["RES-A"]);
    play_host(&mut r);
    assert_eq!(mine(&r), -3, "the play cost is paid before [On Play]");
    assert_eq!(candidates(&r), 1, "only the red Digimon can afford its candidate (CR 4.3 1-3-11-1)");
    pick_first(&mut r);
    assert_eq!(candidates(&r), 1, "the card pick offers RES-A");
    pick_first(&mut r);
    assert_eq!(top_of(&r, 1), "RES-A", "the red Digimon digivolved");
    assert_eq!(top_of(&r, 0), "YEL");
    assert_eq!(mine(&r), -4, "1 paid from -3");
}

#[test]
fn route_only_host_leaf_offers_no_host_when_no_candidate_is_payable() {
    let yaml = host_card("", &host_pick("{of: you, zone: [hand]}", "{reduce: 1}", ""));
    // RES-FREE costs 9 onto either colour: 8 after the reduction, one above the limit from -3.
    let mut r = runner(&yaml, &["RES-FREE"]);
    play_host(&mut r);
    assert!(r.pending_selection().is_none(), "no host has a payable candidate: nothing to choose");
}

#[test]
fn free_digivolve_host_pick_keeps_checking_the_route_only() {
    // "without paying the cost" (DCGO `payCost: false`): nothing is paid, so even the 9-cost card is a candidate for
    // every host with a circle.
    let yaml = host_card("", &host_pick("{of: you, zone: [hand]}", "free", ""));
    let mut r = runner(&yaml, &["RES-FREE"]);
    play_host(&mut r);
    assert_eq!(candidates(&r), 3, "yellow, red and the host itself, all with a printed circle");
}

#[test]
fn authored_host_cost_is_kept() {
    // An explicit cost on the leaf wins over the pass (AD1-022 / BT21-082 shape): reduce 3 makes 9-3=6 payable too.
    let yaml = host_card("", &host_pick("{of: you, zone: [hand], cost: {reduce: 3}}", "{reduce: 3}", ""));
    let mut r = runner(&yaml, &["RES-FREE"]);
    play_host(&mut r);
    assert_eq!(candidates(&r), 3, "9-3=6 is payable from -3");
}

#[test]
fn compile_pass_gives_the_host_leaf_the_steps_cost() {
    let yaml = host_card("", &host_pick("{of: you, zone: [hand]}", "{reduce: 1}", ""));
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(&yaml).expect("parses");
    let card = digimon_dsl::compile::compile(&spec).expect("compiles");
    let CompiledClause::Triggered(t) = &card.effects[0] else { panic!("triggered clause") };
    let CompiledStep::SelectOwnPermanent { filter, .. } = &t.process[0] else { panic!("host pick") };
    let json = serde_json::to_value(filter).unwrap();
    let leaf = &json["all_of"][1]["has_digivolve_candidate"];
    let step_cost = match &t.process[2] {
        CompiledStep::EffectInitiatedDigivolve { cost, .. } => serde_json::to_value(cost).unwrap(),
        other => panic!("digivolve step, got {other:?}"),
    };
    assert_eq!(leaf["cost"], step_cost, "the host leaf carries the digivolve's own cost: {json}");
}

// ---------------------------------------------------------------------------------------------------------------
// `has_digivolve_candidate { ignore_requirements: true }`
// ---------------------------------------------------------------------------------------------------------------

/// "1 of your Digimon may digivolve into a card in the hand, ignoring digivolution requirements and without paying
/// the cost" (BT22-013 branch 0).
fn free_ignoring(leaf: &str) -> String {
    host_card("", &host_pick(leaf, "free", "          ignore_requirements: true\n"))
}

#[test]
fn ignore_requirements_host_leaf_needs_no_route_but_honours_prohibitions() {
    let yaml = free_ignoring("{of: you, zone: [hand], ignore_requirements: true}");
    let mut r = runner(&yaml, &["IGNORER"]);
    play_host(&mut r);
    assert_eq!(candidates(&r), 3, "IGNORER has no circle, but requirements are ignored: yellow, red and the host");
    // `CannotDigivolve` still applies (DCGO `CanEvolve`, CardSource.cs:1269-1290).
    let mut r = runner(&yaml, &["IGNORER"]);
    let locked = digimon_engine::permanent::PermanentHandle { player: 0, index: 0 };
    r.game.modifiers.add(locked, ModifierEntry::simple(ModifierType::CannotDigivolve, 0, Expiry::Permanent, 0));
    play_host(&mut r);
    assert_eq!(candidates(&r), 2, "the locked Digimon can't digivolve, even ignoring requirements");
    pick_first(&mut r);
    pick_first(&mut r);
    assert!(
        top_of(&r, 0) == "YEL" && (top_of(&r, 1) == "IGNORER" || top_of(&r, 2) == "IGNORER"),
        "the locked yellow Digimon was never offered"
    );
}

#[test]
fn ignore_requirements_host_leaf_offers_no_host_without_a_hand_card() {
    let yaml = free_ignoring("{of: you, zone: [hand], ignore_requirements: true}");
    let mut r = runner(&yaml, &[]);
    play_host(&mut r);
    assert!(r.pending_selection().is_none(), "nothing in the hand can digivolve: no host is offered (BT22_013.cs:145-150)");
}

#[test]
fn requirement_keeping_host_leaf_on_an_ignoring_free_digivolve_still_needs_a_route() {
    // Without the flag the leaf keeps the printed route, so the circle-less card is no candidate.
    let yaml = free_ignoring("{of: you, zone: [hand]}");
    let mut r = runner(&yaml, &["IGNORER"]);
    play_host(&mut r);
    // No host is offered (the card pick that follows an unbound host is a separate, free pick).
    assert_ne!(r.pending_kind(), Some(SelectionKind::OwnField), "no printed route onto any host");
}

#[test]
fn paid_ignore_requirements_host_pick_checks_the_payment() {
    // "for a digivolution cost of 8, ignoring digivolution requirements": 8 is one above the limit from -3.
    let leaf = "{of: you, zone: [hand], ignore_requirements: true}";
    let step = "          ignore_requirements: true\n";
    let mut r = runner(&host_card("", &host_pick(leaf, "8", step)), &["IGNORER"]);
    play_host(&mut r);
    assert!(r.pending_selection().is_none(), "8 can't be paid from -3 (CR 4.3 1-3-11-1)");
    let mut r = runner(&host_card("", &host_pick(leaf, "7", step)), &["IGNORER"]);
    play_host(&mut r);
    assert_eq!(candidates(&r), 3, "7 is payable from -3");
}

#[test]
fn ignore_requirements_host_leaf_on_a_requirement_keeping_digivolve_is_a_compile_error() {
    let errors = compile_errors(&host_card(
        "",
        &host_pick("{of: you, zone: [hand], ignore_requirements: true}", "free", ""),
    ));
    assert!(
        errors.iter().any(|e| e.contains("ignore_requirements")),
        "a requirement-waiving host leaf offers hosts the digivolve refuses: {errors:?}"
    );
}

#[test]
fn ignore_level_and_ignore_requirements_on_one_host_leaf_are_exclusive() {
    let errors = compile_errors(&free_ignoring("{of: you, zone: [hand], ignore_level: true, ignore_requirements: true}"));
    assert!(errors.iter().any(|e| e.contains("exclusive")), "{errors:?}");
}

// ---------------------------------------------------------------------------------------------------------------
// The activation gate of a paid digivolve onto `source`
// ---------------------------------------------------------------------------------------------------------------

/// "[On Play] This Digimon may digivolve into a card in the hand, cost -1", gated on a route-only candidate. The first
/// step gains 1 memory, so the gauge shows whether the effect was offered at all.
fn gated(gate: &str) -> String {
    host_card(
        &format!("    condition:\n{gate}\n"),
        r#"      - gain_memory: 1
      - select_hand:
          of: you
          bind_as: result
          optional: true
          filter: {kind: digimon}
          prompt: This Digimon may digivolve into a card in your hand
      - effect_initiated_digivolve:
          target: source
          from_hand: result
          cost: {reduce: 1}
"#,
    )
}

const GATE_ONTO: &str = "      count_gte:
        filter:
          of: you
          zone: [hand]
          all_of:
            - kind: digimon
            - can_digivolve_onto: source
        n: 1";

const GATE_FROM_SOURCE: &str = "      count_gte:
        filter:
          of: you
          zone: [hand]
          all_of:
            - kind: digimon
            - can_digivolve_from_source: true
        n: 1";

const GATE_HOST_LEAF: &str = "      any_permanent:
        of: you
        zone: [battle_area]
        all_of:
          - is_source: true
          - has_digivolve_candidate:
              of: you
              zone: [hand]
              filter: {kind: digimon}";

/// From -3 at most 7 can be paid. The host is yellow Lv.3, so RES-A costs 9-1=8 onto it: unpayable. RES-A's red
/// circle is no route onto a yellow Digimon.
fn assert_gate_needs_a_payable_candidate(gate: &str) {
    let yaml = gated(gate);
    let mut r = runner(&yaml, &["RES-A"]);
    play_host(&mut r);
    assert_eq!(mine(&r), -3, "the effect was not offered: the only candidate costs 8 from -3 (CR 4.3 1-3-11-1)");
    assert!(r.pending_selection().is_none());
}

#[test]
fn can_digivolve_onto_source_gate_checks_the_payment() {
    assert_gate_needs_a_payable_candidate(GATE_ONTO);
}

#[test]
fn can_digivolve_from_source_gate_checks_the_payment() {
    assert_gate_needs_a_payable_candidate(GATE_FROM_SOURCE);
}

#[test]
fn is_source_host_leaf_gate_checks_the_payment() {
    assert_gate_needs_a_payable_candidate(GATE_HOST_LEAF);
}

#[test]
fn gate_still_opens_for_a_payable_candidate() {
    for gate in [GATE_ONTO, GATE_FROM_SOURCE, GATE_HOST_LEAF] {
        // A yellow circle of 8 printed, 7 after the reduction: exactly the limit from -3.
        let mut r = runner(&gated(gate), &["RES-B"]);
        play_host(&mut r);
        assert_eq!(mine(&r), -2, "the gate opened and the first step gained 1 memory ({gate})");
        assert_eq!(candidates(&r), 1, "RES-B is offered ({gate})");
    }
}

#[test]
fn free_digivolve_gate_stays_route_only() {
    let yaml = gated(GATE_ONTO).replace("cost: {reduce: 1}", "cost: free");
    let mut r = runner(&yaml, &["RES-A"]);
    play_host(&mut r);
    assert_eq!(mine(&r), -2, "a free digivolve pays nothing: the printed 9 is no obstacle");
}
