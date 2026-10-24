//! Cost-aware effect evolution is opt-in. Compatibility-only predicates must
//! keep their meaning for existing free/reduced callers. CR 4.3 §§4-2-2,
//! 8-1-2-1/6 and 8-1-3 require the real payer, chosen route and complete cost.

use digimon_engine::action::{
    build_action_mask,
    space::{HAND_EFFECT_START, PASS, PLAY_HAND_START},
};
use digimon_engine::card_data::EvoCost;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Expiry, GamePhase, ModifierType};
use digimon_engine::modifiers::{ModifierEntry, PlayerModifierEntry};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;
use digimon_engine::TriggerSource;

const REDUCER: &str = r#"
card: AFFORD-REDUCER
name: Affordable evolution reducer
kind: tamer
color: [red]
cost: 2
effects:
  - kind: cost_reduction
    reduction_timing: before_pay_cost
    optional: true
    when_any_ally_digivolves_into: {card_number_is: AFFORD-RESULT}
    amount: 2
    pay_cost:
      - suspend: {target: source}
"#;

const HOST: &str = r#"
card: AFFORD-HOST
name: Affordable evolution host
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          bind_as: result
          optional: true
          filter:
            all_of:
              - kind: digimon
              - name_is: Evolution result
              - can_digivolve_onto_with_cost: { target: source, cost: printed }
          prompt: Choose payable evolution
      - effect_initiated_digivolve: { target: source, from_hand: result, cost: printed }
"#;

fn host_yaml(cost: &str, route_only: bool, bound: bool) -> String {
    let mut yaml = HOST.to_owned();
    if route_only {
        yaml = yaml.replace(
            "can_digivolve_onto_with_cost: { target: source, cost: printed }",
            "can_digivolve_onto: source",
        );
    }
    yaml = yaml.replace("cost: printed", &format!("cost: {cost}"));
    if bound {
        yaml = yaml.replace("target: source", "target: chosen")
            .replace("can_digivolve_onto: source", "can_digivolve_onto: chosen")
            .replace("    process:\n", "    process:\n      - select_own_permanent: { bind_as: chosen, filter: { kind: digimon }, prompt: Choose evolution host }\n");
    }
    yaml
}

fn fixture(
    owner: u8,
    current: u8,
    memory: i16,
    costs: &[u16],
    cost: &str,
    route_only: bool,
    bound: bool,
) -> (DebugRunner, PermanentHandle) {
    let mut result = make_test_card("AFFORD-RESULT", "Evolution result");
    result.level = Some(4);
    result.evo_costs = costs
        .iter()
        .map(|&memory_cost| EvoCost {
            card_color: 0,
            level: 3,
            memory_cost,
        })
        .collect();
    let mut neighbor = make_test_card("AFFORD-NEIGHBOR", "Neighbor host");
    neighbor.dp = Some(8000);
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(&host_yaml(cost, route_only, bound))
        .unwrap()
        .from_dsl_yaml(REDUCER)
        .unwrap()
        .add_card(result)
        .add_card(neighbor)
        .hand(owner, &["AFFORD-RESULT"])
        .deck(0, &["AFFORD-NEIGHBOR"; 12])
        .deck(1, &["AFFORD-NEIGHBOR"; 12])
        .start();
    r.skip_mulligan();
    r.set_first_player(current);
    r.set_phase(GamePhase::Main);
    r.game.set_memory(memory);
    let h = r.place_on_field(owner, "AFFORD-HOST", None);
    (r, h)
}

fn fire(r: &mut DebugRunner, h: PermanentHandle) {
    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}

fn top(r: &DebugRunner, h: PermanentHandle) -> &str {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
}

fn hand_choice(r: &DebugRunner, owner: u8) {
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("affordable candidate choice");
    assert_eq!(pending.kind, SelectionKind::Hand);
    assert_eq!(pending.selecting_player, owner);
    assert_eq!(pending.valid_action_ids, vec![PLAY_HAND_START]);
    assert!(pending.is_optional);
    let mask = build_action_mask(&r.game, owner);
    let expected = [PLAY_HAND_START, PASS];
    assert_eq!(
        mask.iter().enumerate().filter(|(_, v)| **v != 0.0).count(),
        expected.len()
    );
    for action in expected {
        assert_eq!(mask[action as usize], 1.0);
    }
    assert!(build_action_mask(&r.game, 1 - owner)
        .iter()
        .all(|&v| v == 0.0));
}

fn choose(r: &mut DebugRunner, owner: u8, action: u16) {
    assert_eq!(build_action_mask(&r.game, owner)[action as usize], 1.0);
    r.execute_action(owner, action).unwrap();
}

#[test]
fn cost_aware_hand_filter_uses_printed_free_reduced_and_fixed_costs() {
    for (cost, expected_cost, offered) in [
        ("printed", 19, false),
        ("free", 0, true),
        ("{ reduce: 1 }", 18, true),
        ("2", 2, true),
        ("19", 19, false),
    ] {
        let (mut r, h) = fixture(0, 0, 8, &[19], cost, false, false);
        fire(&mut r, h);
        if offered {
            hand_choice(&r, 0);
            assert_eq!(r.memory(), 8, "candidate probe is read-only: {cost}");
            choose(&mut r, 0, PLAY_HAND_START);
            assert_eq!(top(&r, h), "AFFORD-RESULT");
            let expected_own_memory = 8 - expected_cost;
            let own_memory = if r.turn_player() == 0 {
                r.memory()
            } else {
                -r.memory()
            };
            assert_eq!(own_memory, expected_own_memory);
            assert_eq!(
                r.turn_player(),
                if expected_own_memory < 0 { 1 } else { 0 },
                "the complete effect ends the turn after crossing zero"
            );
        } else {
            assert!(
                r.game.pending_selection.is_none(),
                "unpayable {cost} must not be offered"
            );
            assert_eq!(top(&r, h), "AFFORD-HOST");
            assert_eq!(r.memory(), 8);
            assert_eq!(r.game.players[0].hand.len(), 1);
        }
    }
}

#[test]
fn inactive_candidate_affordability_uses_the_target_controllers_gauge_side() {
    for owner in [0, 1] {
        for memory in [7, 8] {
            let (mut r, h) = fixture(owner, 1 - owner, memory, &[3], "printed", false, false);
            fire(&mut r, h);
            if memory == 7 {
                hand_choice(&r, owner);
                choose(&mut r, owner, PLAY_HAND_START);
                assert_eq!(r.memory(), 10);
                assert_eq!(top(&r, h), "AFFORD-RESULT");
            } else {
                assert!(r.game.pending_selection.is_none());
                assert_eq!(r.memory(), 8);
                assert_eq!(top(&r, h), "AFFORD-HOST");
            }
        }
    }
}

#[test]
fn optional_reducer_is_probed_without_spending_and_remains_an_explicit_choice() {
    for owner in [0, 1] {
        let (mut r, h) = fixture(owner, 1 - owner, 8, &[4], "printed", false, false);
        let reducer = r.place_on_field(owner, "AFFORD-REDUCER", None);
        let hand = r.game.players[owner as usize].hand[0].handle();
        let deck = r.game.players[owner as usize].deck.len();
        fire(&mut r, h);
        hand_choice(&r, owner);
        assert!(!r.game.players[owner as usize].battle_area[reducer.index as usize].is_suspended);
        assert_eq!(r.memory(), 8);
        assert_eq!(r.game.players[owner as usize].hand[0].handle(), hand);
        assert_eq!(r.game.players[owner as usize].deck.len(), deck);
        let parked = r.game.clone();
        choose(&mut r, owner, PASS);
        assert_eq!(r.memory(), 8);
        assert!(!r.game.players[owner as usize].battle_area[reducer.index as usize].is_suspended);
        r.game = parked;
        choose(&mut r, owner, PLAY_HAND_START);
        let p = r
            .game
            .pending_selection
            .as_ref()
            .expect("pay the optional reducer");
        assert_eq!(p.kind, SelectionKind::EffectChoice);
        assert_eq!(p.selecting_player, owner);
        assert_eq!(p.valid_action_ids, vec![HAND_EFFECT_START]);
        assert!(p.is_optional);
        let payment = r.game.clone();
        choose(&mut r, owner, PASS);
        assert_eq!(r.memory(), 8);
        assert_eq!(top(&r, h), "AFFORD-HOST");
        assert_eq!(r.game.players[owner as usize].hand[0].handle(), hand);
        r.game = payment;
        choose(&mut r, owner, HAND_EFFECT_START);
        assert_eq!(r.memory(), 10);
        assert_eq!(top(&r, h), "AFFORD-RESULT");
        assert!(r.game.players[owner as usize].battle_area[reducer.index as usize].is_suspended);
        assert!(r.game.pending_selection.is_none());
    }
}

#[test]
fn an_unpayable_reducer_cannot_make_a_candidate_legal() {
    let (mut r, h) = fixture(0, 1, 8, &[4], "printed", false, false);
    let reducer = r.place_on_field(0, "AFFORD-REDUCER", None);
    r.game.players[0].battle_area[reducer.index as usize].is_suspended = true;
    fire(&mut r, h);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.memory(), 8);
    assert_eq!(top(&r, h), "AFFORD-HOST");
    assert!(r.game.players[0].battle_area[reducer.index as usize].is_suspended);
}

#[test]
fn one_payable_route_keeps_candidate_but_does_not_offer_an_unpayable_route() {
    let (mut r, h) = fixture(0, 1, 8, &[2, 4], "printed", false, false);
    fire(&mut r, h);
    hand_choice(&r, 0);
    choose(&mut r, 0, PLAY_HAND_START);
    assert!(
        r.game.pending_selection.is_none(),
        "only cost two is payable"
    );
    assert_eq!(top(&r, h), "AFFORD-RESULT");
    assert_eq!(r.memory(), 10);
}

#[test]
fn both_payable_routes_remain_visible_after_candidate_selection() {
    for choice in [0, 1] {
        let (mut r, h) = fixture(0, 1, 5, &[2, 4], "printed", false, false);
        fire(&mut r, h);
        hand_choice(&r, 0);
        choose(&mut r, 0, PLAY_HAND_START);
        let p = r
            .game
            .pending_selection
            .as_ref()
            .expect("distinct route decision");
        assert_eq!(
            p.valid_action_ids,
            vec![HAND_EFFECT_START, HAND_EFFECT_START + 1]
        );
        assert!(!p.is_optional);
        for action in &p.valid_action_ids {
            assert_eq!(build_action_mask(&r.game, 0)[*action as usize], 1.0);
        }
        assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 0.0);
        choose(&mut r, 0, HAND_EFFECT_START + choice);
        assert_eq!(r.memory(), if choice == 0 { 7 } else { 9 });
    }
}

#[test]
fn free_cost_does_not_waive_route_compatibility_or_host_prohibitions() {
    for mode in ["wrong_level", "wrong_color", "option", "host_prohibited"] {
        let (mut r, h) = fixture(0, 0, 8, &[19], "free", false, false);
        if mode == "host_prohibited" {
            r.game.modifiers.add(
                h,
                ModifierEntry::simple(ModifierType::CannotDigivolve, 0, Expiry::Permanent, 0),
            );
        } else {
            let card = r
                .game
                .card_data
                .iter_mut()
                .find(|c| c.card_id == "AFFORD-RESULT")
                .unwrap();
            match mode {
                "wrong_level" => card.evo_costs[0].level = 5,
                "wrong_color" => card.evo_costs[0].card_color = CardColor::Blue as u8,
                "option" => card.card_kind = CardKind::Option,
                _ => unreachable!(),
            }
        }
        fire(&mut r, h);
        assert!(r.game.pending_selection.is_none(), "{mode}");
        assert_eq!(top(&r, h), "AFFORD-HOST");
        assert_eq!(r.memory(), 8);
    }
}

#[test]
fn cost_probe_honors_effect_evolution_and_reduction_prohibitions() {
    for prohibition in [
        ModifierType::CannotReduceCost,
        ModifierType::CannotReduceDigivolveCost,
        ModifierType::OpponentCannotReduceDigivolveCost,
        ModifierType::CannotDigivolveDigimonByEffect,
    ] {
        for cost in ["{ reduce: 2 }", "free"] {
            let (mut r, h) = fixture(0, 1, 8, &[4], cost, false, false);
            let modifier_owner = if prohibition == ModifierType::OpponentCannotReduceDigivolveCost {
                1
            } else {
                0
            };
            r.game.modifiers.add_player_modifier(
                modifier_owner,
                PlayerModifierEntry::simple(
                    prohibition,
                    0,
                    Expiry::Permanent,
                    None,
                    modifier_owner,
                ),
            );
            fire(&mut r, h);
            if cost == "free" && prohibition != ModifierType::CannotDigivolveDigimonByEffect {
                hand_choice(&r, 0);
                choose(&mut r, 0, PLAY_HAND_START);
                assert_eq!(top(&r, h), "AFFORD-RESULT");
            } else {
                assert!(
                    r.game.pending_selection.is_none(),
                    "{prohibition:?}, {cost}"
                );
                assert_eq!(top(&r, h), "AFFORD-HOST");
            }
            assert_eq!(r.memory(), 8);
        }
    }
}

#[test]
fn existing_route_only_predicate_keeps_free_reduced_and_unpayable_compatibility() {
    for cost in ["free", "{ reduce: 18 }", "printed"] {
        let (mut r, h) = fixture(0, 0, 8, &[19], cost, true, false);
        fire(&mut r, h);
        hand_choice(&r, 0);
        choose(&mut r, 0, PLAY_HAND_START);
        if cost == "printed" {
            assert_eq!(top(&r, h), "AFFORD-HOST");
            assert_eq!(r.memory(), 8);
        } else {
            assert_eq!(top(&r, h), "AFFORD-RESULT");
            assert_eq!(r.memory(), if cost == "free" { 8 } else { 7 });
        }
    }
}

#[test]
fn selected_host_identity_survives_compaction_and_a_removed_host_never_retargets() {
    for remove_selected in [false, true] {
        let (mut r, actor) = fixture(0, 0, 8, &[3], "printed", false, true);
        r.place_on_field(0, "AFFORD-NEIGHBOR", None);
        fire(&mut r, actor);
        let p = r.game.pending_selection.as_ref().unwrap();
        assert_eq!(p.valid_action_ids.len(), 2);
        let selected = if remove_selected {
            p.valid_action_ids[0]
        } else {
            p.valid_action_ids[1]
        };
        choose(&mut r, 0, selected);
        hand_choice(&r, 0);
        let removed = r.game.players[0].battle_area.remove(0);
        r.game.players[0].trash.extend(removed.card_sources);
        let parked = r.game.clone();
        for _ in 0..2 {
            r.game = parked.clone();
            choose(&mut r, 0, PLAY_HAND_START);
            assert_eq!(r.game.players[0].battle_area.len(), 1);
            assert_eq!(
                top(
                    &r,
                    PermanentHandle {
                        player: 0,
                        index: 0
                    }
                ),
                if remove_selected {
                    "AFFORD-NEIGHBOR"
                } else {
                    "AFFORD-RESULT"
                }
            );
            assert_eq!(r.memory(), if remove_selected { 8 } else { 5 });
        }
    }
}

#[test]
fn formula_cost_reads_effect_source_context_instead_of_the_selected_host() {
    let cost = "{ reduce: { floor_div: [{ source_dp: {} }, 1000] } }";
    let (mut r, actor) = fixture(0, 1, 5, &[7], cost, false, true);
    let target = r.place_on_field(0, "AFFORD-NEIGHBOR", None);
    fire(&mut r, actor);
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[1];
    choose(&mut r, 0, action);
    hand_choice(&r, 0);
    choose(&mut r, 0, PLAY_HAND_START);
    assert_eq!(top(&r, target), "AFFORD-RESULT");
    assert_eq!(
        r.memory(),
        10,
        "source DP2000 reduces seven by two; target DP8000 is irrelevant"
    );
}

fn validate(yaml: &str) -> Result<(), Vec<digimon_dsl::errors::ValidationError>> {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).unwrap();
    let raw = digimon_dsl::raw_rust_registry::StubRegistry::empty();
    digimon_dsl::validator::validate(
        &spec,
        &digimon_dsl::validator::ValidationContext { raw_rust: &raw },
    )
}

#[test]
fn new_predicate_validates_target_and_cost_formula_binding_dependencies() {
    assert!(validate(&host_yaml("printed", false, false)).is_ok());
    assert!(validate(&host_yaml("printed", false, true)).is_ok());
    for (yaml, path) in [
        (
            host_yaml("printed", false, false).replace("target: source", "target: missing"),
            "target",
        ),
        (
            host_yaml("{ reduce: { binding_value: missing } }", false, false),
            "cost",
        ),
    ] {
        let errors =
            validate(&yaml).expect_err("undeclared target/formula binding must be rejected");
        assert!(
            errors
                .iter()
                .any(|e| e.path.contains("can_digivolve_onto_with_cost") && e.path.contains(path)),
            "{errors:?}"
        );
    }
    for cost in [
        "{ reduce: { raw_rust: arbitrary_callback } }",
        "{ reduce: { max: [1, { raw_rust: arbitrary_callback }] } }",
    ] {
        let errors = validate(&host_yaml(cost, false, false)).expect_err(
            "raw callbacks cannot be assumed equivalent in read-only and commit contexts",
        );
        assert!(
            errors
                .iter()
                .any(|e| e.path.contains("can_digivolve_onto_with_cost.cost")
                    && e.message.contains("declarative cost formulas")),
            "{errors:?}"
        );
    }
}

#[test]
fn cost_aware_predicate_cannot_smuggle_local_bindings_into_plural_post_payment_filters() {
    for filter in [
        "can_digivolve_onto_with_cost: { target: chosen, cost: printed }",
        "can_digivolve_onto_with_cost: { target: source, cost: { reduce: { binding_value: chosen } } }",
        "all_of: [{ can_digivolve_onto_with_cost: { target: chosen, cost: free } }]",
    ] {
        let yaml = format!(r#"
card: AFFORD-PLURAL
name: Plural predicate context
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
effects:
  - kind: replacement
    trigger: when_would_leave_battle_area
    optional: true
    process:
      - select_own_permanent: {{ bind_as: chosen, filter: {{ kind: digimon }}, prompt: Choose a Digimon }}
      - trash_top_security_and_cancel_replacement: {{ of: you, all_matching: {{ {filter} }} }}
"#);
        let errors = validate(&yaml).expect_err("post-payment filter loses local bindings");
        assert!(errors.iter().any(|e| e.path.contains("all_matching") && e.message.contains("post-payment plural filter")), "{errors:?}");
    }
}
