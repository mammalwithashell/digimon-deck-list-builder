//! Opt-in payable-result probes for prospective effect evolution hosts.
//! CR 4.3 8-1-3 / 15-7: normal requirements and actual cost remain applicable.
//! EX5-070 and EX13-004 use reduced paid evolution; existing free effects
//! retain route-only semantics when this predicate's cost is omitted.
use digimon_engine::action::{
    build_action_mask,
    space::{PASS, PLAY_HAND_START},
};
use digimon_engine::card_data::EvoCost;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, Expiry, ModifierType};
use digimon_engine::modifiers::PlayerModifierEntry;
use digimon_engine::selection::SelectionKind;

const PROBE: &str = r#"
card: CANDIDATE-COST-PROBE
name: Candidate cost probe
kind: option
color: [blue]
cost: 0
effects:
  - when: main_from_hand
    process:
      - select_own_permanent:
          bind_as: host
          optional: true
          filter:
            all_of:
              - kind: digimon
              - has_digivolve_candidate:
                  of: you
                  zone: [hand]
                  filter: { trait_has: X Antibody }
                  cost: { reduce: 1 }
          prompt: Choose a host with a payable result
      - select_hand:
          of: you
          bind_as: result
          optional: true
          filter:
            all_of:
              - kind: digimon
              - trait_has: X Antibody
              - can_digivolve_onto_with_cost: { target: host, cost: { reduce: 1 } }
          prompt: Choose a payable result
      - effect_initiated_digivolve: { target: host, from_hand: result, cost: { reduce: 1 } }
"#;

fn setup(cost: u16, mode: &str) -> DebugRunner {
    let mut host = make_test_card("COST-HOST", "Host");
    host.colors = vec![CardColor::Blue];
    host.level = Some(3);
    let mut wrong = host.clone();
    wrong.card_id = "WRONG-HOST".into();
    wrong.colors = vec![CardColor::Red];
    let mut result = make_test_card("COST-RESULT", "Result");
    result.colors = vec![CardColor::Blue];
    result.level = Some(4);
    result.traits = vec!["X Antibody".into()];
    result.evo_costs = vec![EvoCost {
        card_color: CardColor::Blue as u8,
        level: 3,
        memory_cost: cost,
    }];
    let yaml = match mode {
        "route" => PROBE.replace("                  cost: { reduce: 1 }\n", ""),
        "free" => PROBE.replace("cost: { reduce: 1 }", "cost: free"),
        _ => PROBE.to_owned(),
    };
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .unwrap()
        .add_card(host)
        .add_card(wrong)
        .add_card(result)
        .hand(0, &["CANDIDATE-COST-PROBE", "COST-RESULT"])
        .deck(0, &["COST-HOST"; 20])
        .deck(1, &["COST-HOST"; 20])
        .memory(8)
        .start();
    r.skip_mulligan();
    r.game.enter_main_phase();
    r.place_on_field(0, "COST-HOST", Some(0));
    r
}

fn use_option(r: &mut DebugRunner) {
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(PLAY_HAND_START, 0);
}
fn choose(r: &mut DebugRunner, a: u16) {
    let p = r.pending_selection_view().unwrap().selecting_player;
    assert_eq!(build_action_mask(&r.game, p)[a as usize], 1.0);
    r.execute_action(p, a).unwrap();
}

#[test]
fn cost_aware_host_rejects_unpayable_result_and_keeps_route_only_compatibility() {
    for mode in ["reduced", "route", "free"] {
        let mut r = setup(20, mode);
        use_option(&mut r);
        assert_eq!(
            r.pending_kind() == Some(SelectionKind::OwnField),
            mode != "reduced",
            "{mode}"
        );
        assert_eq!(r.memory(), 8, "preflight never pays or mutates memory");
    }
}

#[test]
fn reduced_boundary_host_and_hand_choices_agree_and_clone_commit() {
    let mut r = setup(19, "reduced");
    use_option(&mut r);
    let v = r
        .pending_selection_view()
        .expect("19 minus 1 fits 8 plus 10 memory");
    assert_eq!(v.kind, SelectionKind::OwnField);
    assert!(v.is_optional);
    let host = v.valid_action_ids[0];
    choose(&mut r, host);
    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
    let saved = r.game.clone();
    for game in [saved.clone(), saved] {
        r.game = game;
        choose(&mut r, PLAY_HAND_START);
        assert_eq!(
            r.game.players[0].battle_area[0]
                .top_card()
                .card_id(&r.game.card_data),
            "COST-RESULT"
        );
        let payer_memory = if r.game.turn_player() == 0 {
            r.memory()
        } else {
            -r.memory()
        };
        assert_eq!(payer_memory, -10);
    }
}

#[test]
fn host_cost_probe_honors_player_prohibitions() {
    for restriction in [
        ModifierType::CannotReduceCost,
        ModifierType::CannotReduceDigivolveCost,
        ModifierType::CannotDigivolveDigimonByEffect,
    ] {
        for printed_cost in [3, 19] {
            let mut r = setup(printed_cost, "reduced");
            r.game.modifiers.add_player_modifier(
                0,
                PlayerModifierEntry::simple(restriction, 0, Expiry::Permanent, None, 0),
            );
            use_option(&mut r);
            if printed_cost == 3 && restriction != ModifierType::CannotDigivolveDigimonByEffect {
                let host_action = r
                    .pending_selection_view()
                    .expect("full printed cost is payable")
                    .valid_action_ids[0];
                choose(&mut r, host_action);
                choose(&mut r, PLAY_HAND_START);
                assert_eq!(
                    r.memory(),
                    5,
                    "reduction forbidden, evolution pays printed cost"
                );
                assert_eq!(
                    r.game.players[0].battle_area[0]
                        .top_card()
                        .card_id(&r.game.card_data),
                    "COST-RESULT"
                );
            } else {
                assert!(
                    r.pending_selection().is_none(),
                    "{restriction:?}, printed={printed_cost}"
                );
                assert_eq!(r.memory(), 8);
            }
        }
    }
}

#[test]
fn own_host_existence_does_not_substitute_other_hosts_route() {
    let mut r = setup(3, "reduced");
    r.place_on_field(0, "WRONG-HOST", Some(0));
    use_option(&mut r);
    let v = r.pending_selection_view().unwrap();
    assert_eq!(
        v.valid_action_ids.len(),
        1,
        "only the actual blue host has a payable compatible route"
    );
    choose(&mut r, PASS);
    assert!(r.pending_selection().is_none());
}
