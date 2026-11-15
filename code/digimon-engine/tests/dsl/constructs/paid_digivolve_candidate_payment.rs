//! A PAID effect digivolve's card pick offers only cards whose digivolution cost can be paid
//! (G-DSL-PAID-DIGIVOLVE-PICK-PAYMENT).
//!
//! "This Digimon may digivolve into [X] in the hand (with the digivolution cost reduced by N)" charges memory. A card
//! whose cost can't be paid can't be digivolved: CR 4.3 1-3-11-1 (a card can't be declared if its cost can't be
//! paid), 8-1-2-6 (it goes back and memory doesn't move), 4-2-2 / 1-4-2-2 (the gauge stops at 10, so a cost above
//! memory + 10 can't be paid). Offering it is a dead choice. It is reachable whenever memory is already low when the
//! effect resolves, e.g. an [On Play] after paying the play cost into the opponent's side.
//!
//! DCGO passes `payCost: true` for these digivolves (`DigivolveIntoHandOrTrashCard`, CardEffectCommons.cs:820-878;
//! AD1_011.cs:141-147, BT13_013.cs:59-65, EX13_074.cs:234-240, P_092.cs:75-81), but
//! `CardSource.CanPlayCardTargetFrame` honours `PayCost` only for an empty frame (CardSource.cs:1150-1161), so for a
//! digivolve onto a permanent DCGO checks the route alone. The rules manual governs.
//!
//! The compile pass (`digimon_dsl::digivolve_candidates`) adds `can_digivolve_onto_with_cost { target, cost: <the
//! step's cost> }` with the step's waiver to the pick, next to any route-only leaf the card authored. Each shape
//! below is one the card pool uses: the implicit leaf (BT22-098), an authored `can_digivolve_onto: source`
//! (AD1-011), `can_digivolve_from_source` (BT13-013), a target bound by an earlier pick (LM-027), a hand-or-trash
//! union pick at the printed cost (EX13-055) and a fixed cost "ignoring digivolution requirements" (EX13-074, P-092).
use digimon_dsl::compiled::{CompiledClause, CompiledStep};
use digimon_engine::action::{build_action_mask, space};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase};

const HOST: &str = "TEST-PAID-HOST";

/// A Lv.3 yellow Digimon costing `cost` whose [On Play] runs `process`.
fn host_card(cost: u16, process: &str) -> String {
    format!(
        r#"
card: {HOST}
name: Paid digivolve host
kind: digimon
level: 3
color: [yellow]
cost: {cost}
dp: 3000
effects:
  - when: on_play
    process:
{process}
"#
    )
}

/// The pick and digivolve, `filter` on the pick, `step` the digivolve's extra lines.
fn hand_pick(filter: &str, cost: &str, step: &str) -> String {
    format!(
        r#"      - select_hand:
          of: you
          bind_as: result
          optional: true
          filter: {filter}
          prompt: This Digimon may digivolve into a card in your hand
      - effect_initiated_digivolve:
          target: source
          from_hand: result
          cost: {cost}
{step}"#
    )
}

/// Lv.4 yellow result whose only printed circle is yellow Lv.3 at `cost`.
fn result(id: &str, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.colors = vec![CardColor::Yellow];
    c.play_cost = 9;
    c.evo_costs = vec![EvoCost { card_color: CardColor::Yellow as u8, level: 3, memory_cost: cost }];
    c
}

/// Lv.6 [Wingdramon]-like result with NO circle onto the host: only an "ignoring digivolution requirements"
/// digivolve can use it.
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

/// A runner in its main phase, memory `memory`, the host first in hand, `hand` after it and `trash` in the trash.
fn runner(yaml: &str, hand: &[&str], trash: &[&str], memory: i16) -> DebugRunner {
    let mut cards = vec![HOST];
    cards.extend_from_slice(hand);
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("paid digivolve host compiles")
        .add_card(result("RES-7", 7))
        .add_card(result("RES-8", 8))
        .add_card(result("RES-9", 9))
        .add_card(no_route("IGNORER"))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &cards)
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .start();
    r.set_first_player(0);
    r.skip_mulligan();
    r.set_phase(GamePhase::Main);
    for id in trash {
        r.inject_trash(0, id);
    }
    r.game.set_memory(memory);
    r
}

/// Play the host (hand[0]) so its [On Play] runs with the memory left after paying its play cost.
fn play_host(r: &mut DebugRunner) {
    assert_eq!(build_action_mask(&r.game, 0)[space::PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(space::PLAY_HAND_START, 0);
}

/// The card ids the pending hand pick offers.
fn offered_hand(r: &DebugRunner) -> Vec<String> {
    let Some(v) = r.pending_selection_view() else { return Vec::new() };
    assert!(v.is_optional, "\"may digivolve\" is optional");
    v.valid_action_ids
        .iter()
        .filter(|a| **a != space::PASS)
        .map(|a| r.game.players[0].hand[*a as usize].card_id(&r.game.card_data).to_string())
        .collect()
}

fn pick_first(r: &mut DebugRunner) {
    let v = r.pending_selection_view().expect("a pick");
    let a = *v.valid_action_ids.iter().find(|a| **a != space::PASS).expect("a candidate");
    r.execute_action(0, a).unwrap();
}

fn host_top(r: &DebugRunner) -> String {
    r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data).to_string()
}

/// Player 0's memory (the gauge reads from the turn player's side).
fn mine(r: &DebugRunner) -> i16 {
    if r.game.turn_player() == 0 { r.memory() } else { -r.memory() }
}

/// The host costs 3, played from memory 0: its [On Play] resolves at -3, where at most 7 can be paid. With the cost
/// reduced by 1, RES-8 (8-1=7) is payable and RES-9 (9-1=8) is one above the limit.
fn assert_offers_only_the_payable_card(yaml: &str) {
    let mut r = runner(yaml, &["RES-8", "RES-9"], &[], 0);
    play_host(&mut r);
    assert_eq!(mine(&r), -3, "the play cost is paid before [On Play]");
    assert_eq!(offered_hand(&r), vec!["RES-8"], "9-1=8 can't be paid from -3 (CR 4.3 1-3-11-1)");
    pick_first(&mut r);
    assert_eq!(host_top(&r), "RES-8");
    assert_eq!(mine(&r), -10, "7 paid from -3: the gauge stops at the opponent's 10");
}

#[test]
fn implicit_candidate_leaf_checks_the_payment() {
    // BT22-098 shape: no authored leaf; the compile pass supplies route and payment.
    assert_offers_only_the_payable_card(&host_card(3, &hand_pick("{kind: digimon}", "{reduce: 1}", "")));
}

#[test]
fn authored_route_only_leaf_still_gets_the_payment_check() {
    // AD1-011 / BT12-030 / EX13-018 shape: `can_digivolve_onto: source` authored on the pick.
    let filter = "{all_of: [{kind: digimon}, {can_digivolve_onto: source}]}";
    assert_offers_only_the_payable_card(&host_card(3, &hand_pick(filter, "{reduce: 1}", "")));
}

#[test]
fn authored_can_digivolve_from_source_still_gets_the_payment_check() {
    // BT13-013 / BT20-023 / BT25-005 shape.
    let filter = "{all_of: [{kind: digimon}, {can_digivolve_from_source: true}]}";
    assert_offers_only_the_payable_card(&host_card(3, &hand_pick(filter, "{reduce: 1}", "")));
}

#[test]
fn bound_target_pick_checks_the_payment_onto_the_chosen_digimon() {
    // LM-027 / LM-055 / BT22-098 shape: the target is a Digimon chosen by an earlier pick.
    let process = r#"      - select_own_permanent:
          bind_as: target
          optional: true
          filter: {kind: digimon}
          prompt: Choose 1 of your Digimon
      - select_hand:
          of: you
          bind_as: result
          optional: true
          filter: {all_of: [{kind: digimon}, {can_digivolve_onto: target}]}
          prompt: It may digivolve into a card in your hand
      - effect_initiated_digivolve:
          target: target
          from_hand: result
          cost: {reduce: 1}
"#;
    let mut r = runner(&host_card(3, process), &["RES-8", "RES-9"], &[], 0);
    play_host(&mut r);
    pick_first(&mut r); // the host itself
    assert_eq!(offered_hand(&r), vec!["RES-8"], "9-1=8 can't be paid from -3 (CR 4.3 1-3-11-1)");
    pick_first(&mut r);
    assert_eq!(host_top(&r), "RES-8");
    assert_eq!(mine(&r), -10);
}

#[test]
fn hand_or_trash_pick_at_the_printed_cost_checks_the_payment() {
    // EX13-055 / EX13-057 shape: `select_union_zone` over hand and trash, the printed cost paid.
    let process = r#"      - select_union_zone:
          of: you
          zones: [hand, trash]
          optional: true
          bind_as: evo
          filter: {all_of: [{kind: digimon}, {can_digivolve_onto: source}]}
          prompt: This Digimon may digivolve into a card in your hand or trash
      - effect_initiated_digivolve:
          target: source
          source: evo
          cost: printed
"#;
    // From -3 at most 7 can be paid: RES-7 is payable, RES-8 is one above the limit.
    let mut r = runner(&host_card(3, process), &[], &["RES-8", "RES-7"], 0);
    play_host(&mut r);
    let v = r.pending_selection_view().expect("the payable trash card is offered");
    assert_eq!(v.valid_action_ids.iter().filter(|a| **a != space::PASS).count(), 1, "RES-8 can't be paid from -3");
    pick_first(&mut r);
    assert_eq!(host_top(&r), "RES-7");
    assert_eq!(mine(&r), -10);
}

/// EX13-074 / P-092 shape: "digivolve into [X] for a digivolution cost of 3, ignoring digivolution requirements".
fn ignoring_requirements(host_cost: u16) -> String {
    host_card(host_cost, &hand_pick("{kind: digimon}", "3", "          ignore_requirements: true\n"))
}

#[test]
fn fixed_cost_ignoring_requirements_pick_checks_the_payment() {
    // Host cost 7 from memory 0: -7, so 3 is payable (to -10).
    let mut r = runner(&ignoring_requirements(7), &["IGNORER"], &[], 0);
    play_host(&mut r);
    assert_eq!(offered_hand(&r), vec!["IGNORER"], "requirements ignored, 3 payable from -7");
    pick_first(&mut r);
    assert_eq!(host_top(&r), "IGNORER");
    assert_eq!(mine(&r), -10);
    // Host cost 8 from memory 0: -8, so 3 is one above the limit.
    let mut r = runner(&ignoring_requirements(8), &["IGNORER"], &[], 0);
    play_host(&mut r);
    assert!(offered_hand(&r).is_empty(), "3 can't be paid from -8 (CR 4.3 1-3-11-1)");
    assert!(r.pending_selection().is_none(), "no candidate: nothing to choose");
    assert_eq!(host_top(&r), HOST);
}

#[test]
fn free_digivolve_pick_checks_no_payment() {
    // "Digivolve ... without paying the cost" (DCGO `payCost: false`, P_092.cs:138-144): nothing is paid, so even
    // a card whose printed cost is far above the limit is a candidate.
    let yaml = host_card(3, &hand_pick("{kind: digimon}", "free", ""));
    let mut r = runner(&yaml, &["RES-9"], &[], 0);
    play_host(&mut r);
    assert_eq!(offered_hand(&r), vec!["RES-9"]);
    pick_first(&mut r);
    assert_eq!(host_top(&r), "RES-9");
    assert_eq!(mine(&r), -3);
}

#[test]
fn compile_pass_emits_the_payment_leaf_with_the_steps_cost() {
    for (filter, cost, step, want_route) in [
        ("{kind: digimon}", "{reduce: 1}", "", false),
        ("{all_of: [{kind: digimon}, {can_digivolve_onto: source}]}", "{reduce: 1}", "", true),
        ("{kind: digimon}", "3", "          ignore_requirements: true\n", false),
    ] {
        let spec: digimon_dsl::CardSpec =
            serde_yml::from_str(&host_card(3, &hand_pick(filter, cost, step))).expect("parses");
        let card = digimon_dsl::compile::compile(&spec).expect("compiles");
        let CompiledClause::Triggered(t) = &card.effects[0] else { panic!("triggered clause") };
        let CompiledStep::SelectHand { filter: compiled, .. } = &t.process[0] else { panic!("select_hand") };
        let CompiledStep::EffectInitiatedDigivolve { cost: step_cost, ignore_requirements, .. } = &t.process[1] else {
            panic!("digivolve step")
        };
        let json = serde_json::to_value(compiled).unwrap();
        let leaf = &json["can_digivolve_onto_with_cost"];
        assert_eq!(leaf["target"], "source", "{json}");
        assert_eq!(leaf["cost"], serde_json::to_value(step_cost).unwrap(), "the step's own cost: {json}");
        assert_eq!(leaf["ignore_requirements"], *ignore_requirements, "{json}");
        assert_eq!(leaf["ignore_level"], false, "{json}");
        // An authored route-only leaf stays in conjunction.
        assert_eq!(json.to_string().contains("\"can_digivolve_onto\":\"source\""), want_route, "{json}");
    }
}

fn compile_errors(yaml: &str) -> Vec<String> {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parses");
    match digimon_dsl::compile::compile(&spec) {
        Ok(_) => Vec::new(),
        Err(errors) => errors.into_iter().map(|e| format!("{}: {}", e.path, e.message)).collect(),
    }
}

#[test]
fn requirement_waiving_leaf_on_a_requirement_keeping_pick_is_a_compile_error() {
    let leaf = "{can_digivolve_onto_with_cost: {target: source, cost: {reduce: 1}, ignore_requirements: true}}";
    let errors = compile_errors(&host_card(3, &hand_pick(leaf, "{reduce: 1}", "")));
    assert!(
        errors.iter().any(|e| e.contains("ignore_requirements")),
        "a requirement-waiving leaf offers cards the digivolve refuses: {errors:?}"
    );
    let both = "{can_digivolve_onto_with_cost: {target: source, cost: 3, ignore_level: true, ignore_requirements: true}}";
    let errors = compile_errors(&host_card(3, &hand_pick(both, "3", "          ignore_requirements: true\n")));
    assert!(errors.iter().any(|e| e.contains("exclusive")), "{errors:?}");
}
