//! EX5-070: English card image, official Q3679-3682 and DCGO EX5_070.cs.
//! CR 4.3 8-1-3 (paid evolution), 15-7 (available instructions), and
//! 15-8-5-4 (immediate effects still resolve after another prevents departure).
//! The inherited instruction is mandatory and does not prevent departure.
use digimon_dsl::compiled::{CompiledCardKind, CompiledColor};
use digimon_engine::action::{
    build_action_mask,
    space::{encode_attack, PASS, PLAY_HAND_START, SECURITY_TARGET},
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Expiry, ModifierType};
use digimon_engine::modifiers::{ModifierEntry, PlayerModifierEntry};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

const ID: &str = "EX5-070";
const X: &str = "BT9-109";
const PREVENT: &str = r#"
card: PROTO-PREVENT
name: Prevent fixture
kind: digimon
color: [blue]
level: 3
cost: 3
dp: 6000
effects:
  - kind: replacement
    trigger: when_would_be_deleted
    summary: Prevent fixture departure
    process:
      - cancel_replacement: {}
"#;
const REDUCER: &str = r#"
card: PROTO-REDUCER
name: Proto reducer fixture
kind: tamer
color: [blue]
cost: 2
effects:
  - kind: cost_reduction
    reduction_timing: before_pay_cost
    optional: true
    when_any_ally_digivolves_into: { card_number_is: EVO }
    amount: 2
    pay_cost:
      - suspend: { target: source }
"#;

fn digimon(id: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.colors = vec![CardColor::Blue];
    c.level = Some(level);
    c.dp = Some(10000);
    c
}
fn result(id: &str, cost: u16) -> CardData {
    let mut c = digimon(id, 4);
    c.traits = vec!["X Antibody".into()];
    c.evo_costs = vec![EvoCost {
        card_color: CardColor::Blue as u8,
        level: 3,
        memory_cost: cost,
    }];
    c
}
fn builder() -> DebugRunnerBuilder {
    let mut wrong_trait = result("WRONG-TRAIT", 3);
    wrong_trait.traits = vec!["X Antibody Variant".into()];
    let mut wrong_level = result("WRONG-LEVEL", 3);
    wrong_level.evo_costs[0].level = 5;
    let mut wrong_color = result("WRONG-COLOR", 3);
    wrong_color.evo_costs[0].card_color = CardColor::Red as u8;
    let mut option = result("OPTION", 3);
    option.card_kind = CardKind::Option;
    option.level = None;
    option.dp = None;
    let mut tamer = make_test_card("TAMER", "Blue Tamer");
    tamer.card_kind = CardKind::Tamer;
    tamer.colors = vec![CardColor::Blue];
    tamer.level = None;
    tamer.dp = None;
    let mut egg = digimon("EGG", 2);
    egg.card_kind = CardKind::DigiEgg;
    egg.dp = None;
    let mut near = option.clone();
    near.card_id = "NEAR".into();
    near.card_name = "X Antibody Variant".into();
    let mut filler = option.clone();
    filler.card_id = "FILL".into();
    filler.traits.clear();
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("EX5-070 production YAML required")
        .dsl_card(X)
        .unwrap()
        .from_dsl_yaml(PREVENT)
        .unwrap()
        .from_dsl_yaml(REDUCER)
        .unwrap()
        .add_card(digimon("HOST", 3))
        .add_card(digimon("SOURCE-A", 2))
        .add_card(digimon("SOURCE-B", 3))
        .add_card(result("EVO", 3))
        .add_card(result("EVO-TWO", 4))
        .add_card(result("BOUNDARY", 19))
        .add_card(result("UNPAYABLE", 20))
        .add_card(wrong_trait)
        .add_card(wrong_level)
        .add_card(wrong_color)
        .add_card(option)
        .add_card(tamer)
        .add_card(egg)
        .add_card(near)
        .add_card(filler)
        .deck(0, &["FILL"; 20])
        .deck(1, &["FILL"; 20])
        .security(0, &["FILL"; 2])
        .security(1, &["FILL"; 2])
        .memory(8)
}
fn main(r: &mut DebugRunner) {
    r.skip_mulligan();
    r.game.enter_main_phase();
}
fn choose(r: &mut DebugRunner, a: u16) {
    let v = r.pending_selection_view().expect("player-visible choice");
    assert_eq!(
        build_action_mask(&r.game, v.selecting_player)[a as usize],
        1.0,
        "{v:?}"
    );
    r.execute_action(v.selecting_player, a).unwrap();
}
fn first(r: &mut DebugRunner) {
    let a = r.pending_selection_view().unwrap().valid_action_ids[0];
    choose(r, a);
}
fn mandatory(r: &mut DebugRunner) {
    for _ in 0..12 {
        let Some(v) = r.pending_selection_view() else {
            return;
        };
        assert!(
            !v.is_optional,
            "mandatory inherited instruction cannot be declined: {v:?}"
        );
        assert_eq!(
            build_action_mask(&r.game, v.selecting_player)[PASS as usize],
            0.0
        );
        choose(r, v.valid_action_ids[0]);
    }
    panic!("mandatory instruction did not terminate");
}
fn use_option(r: &mut DebugRunner) {
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(PLAY_HAND_START, 0);
}
fn setup(hand: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = builder().hand(0, hand).start();
    main(&mut r);
    let h = r.place_on_field(0, "HOST", Some(0));
    (r, h)
}
fn stack(r: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .card_sources
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_owned())
        .collect()
}
fn count_hand(r: &DebugRunner, p: u8, id: &str) -> usize {
    r.game.players[p as usize]
        .hand
        .iter()
        .filter(|c| c.card_id(&r.game.card_data) == id)
        .count()
}
fn count_trash(r: &DebugRunner, p: u8, id: &str) -> usize {
    r.game.players[p as usize]
        .trash
        .iter()
        .filter(|c| c.card_id(&r.game.card_data) == id)
        .count()
}
fn security_top(r: &DebugRunner, p: u8) -> &str {
    r.game.players[p as usize]
        .security
        .last()
        .unwrap()
        .card_id(&r.game.card_data)
}
fn delete(r: &mut DebugRunner, h: PermanentHandle, cause: ReplacementCause) {
    r.game.delete_permanent_with_cause(h, cause);
}

#[test]
fn printed_option_metadata_and_rule_name_alias() {
    let r = builder().start();
    let c = r.compiled_card(ID).unwrap();
    assert_eq!(c.name, "X Antibody Proto Form");
    assert_eq!(c.kind, CompiledCardKind::Option);
    assert_eq!(c.color, vec![CompiledColor::White]);
    assert_eq!(c.cost, Some(0));
    assert_eq!(c.traits, vec!["X Antibody"]);
    assert!(c.also_treated_as.iter().any(|name| name == "X Antibody"));
}
#[test]
fn color_bypass_requires_own_battle_digimon() {
    for state in 0..5 {
        let mut r = builder().hand(0, &[ID]).start();
        main(&mut r);
        match state {
            1 => {
                r.place_on_field(0, "TAMER", Some(0));
            }
            2 => {
                r.place_on_field(1, "HOST", Some(0));
            }
            3 => {
                r.place_in_breeding(0, "HOST");
            }
            4 => {
                r.place_on_field(0, "HOST", Some(0));
            }
            _ => {}
        }
        assert_eq!(
            build_action_mask(&r.game, 0)[PLAY_HAND_START as usize],
            if state == 4 { 1.0 } else { 0.0 },
            "state={state}"
        );
    }
}
#[test]
fn main_reduced_evolution_draws_and_places_exact_option_at_bottom_after_clone() {
    let (mut r, h) = setup(&[ID, "EVO", "EVO-TWO"]);
    let identity = r.game.players[0].hand[0].handle();
    use_option(&mut r);
    let v = r.pending_selection_view().unwrap();
    assert_eq!(v.kind, SelectionKind::OwnField);
    assert!(v.is_optional);
    first(&mut r);
    let v = r.pending_selection_view().unwrap();
    assert_eq!(v.kind, SelectionKind::Hand);
    assert!(v.is_optional);
    assert_eq!(
        v.valid_action_ids,
        vec![PLAY_HAND_START, PLAY_HAND_START + 1]
    );
    let saved = r.game.clone();
    for game in [saved.clone(), saved] {
        r.game = game;
        choose(&mut r, PLAY_HAND_START + 1);
        assert_eq!(stack(&r, h), vec![ID, "HOST", "EVO-TWO"]);
        assert_eq!(
            r.game.players[0].battle_area[0].card_sources[0].handle(),
            identity
        );
        assert_eq!(r.memory(), 5);
        assert_eq!(count_hand(&r, 0, "FILL"), 1);
        assert_eq!(count_trash(&r, 0, ID), 0);
    }
}
#[test]
fn main_may_decline_either_host_or_result_without_attaching_option() {
    for decline_result in [false, true] {
        let (mut r, h) = setup(&[ID, "EVO"]);
        use_option(&mut r);
        if decline_result {
            first(&mut r);
        }
        choose(&mut r, PASS);
        assert_eq!(stack(&r, h), vec!["HOST"]);
        assert_eq!(count_trash(&r, 0, ID), 1);
        assert_eq!(count_hand(&r, 0, "EVO"), 1);
        assert_eq!(r.memory(), 8);
        assert!(r.pending_selection().is_none());
    }
}
#[test]
fn main_host_filter_excludes_x_alias_sources_tamers_and_other_zones() {
    let mut r = builder().hand(0, &[ID, "EVO"]).start();
    main(&mut r);
    r.place_stack(0, &[X, "HOST"]);
    r.place_stack(0, &[ID, "HOST"]);
    let eligible = r.place_stack(0, &["NEAR", "HOST"]);
    r.place_on_field(0, "TAMER", Some(0));
    r.place_on_field(1, "HOST", Some(0));
    r.place_in_breeding(0, "HOST");
    use_option(&mut r);
    assert_eq!(r.pending_action_count(), 1);
    first(&mut r);
    first(&mut r);
    assert_eq!(stack(&r, eligible), vec![ID, "NEAR", "HOST", "EVO"]);
}
#[test]
fn main_hand_filter_honors_kind_exact_trait_routes_affordability_and_hand_zone() {
    let (mut r, h) = setup(&[
        ID,
        "EVO",
        "WRONG-TRAIT",
        "WRONG-LEVEL",
        "WRONG-COLOR",
        "OPTION",
        "UNPAYABLE",
    ]);
    r.place_on_field(1, "EVO-TWO", Some(0));
    use_option(&mut r);
    first(&mut r);
    assert_eq!(
        r.pending_selection_view().unwrap().valid_action_ids,
        vec![PLAY_HAND_START]
    );
    first(&mut r);
    assert_eq!(stack(&r, h), vec![ID, "HOST", "EVO"]);
    assert_eq!(r.memory(), 6);
}
#[test]
fn main_has_no_dead_end_host_when_only_result_is_unpayable_or_no_route() {
    for id in ["UNPAYABLE", "WRONG-LEVEL", "WRONG-TRAIT", "OPTION"] {
        let (mut r, h) = setup(&[ID, id]);
        use_option(&mut r);
        assert!(r.pending_selection().is_none(), "{id}");
        assert_eq!(stack(&r, h), vec!["HOST"]);
        assert_eq!(count_trash(&r, 0, ID), 1);
    }
}
#[test]
fn main_cost_reduction_and_evolution_prohibitions_filter_hosts() {
    for restriction in [
        ModifierType::CannotReduceCost,
        ModifierType::CannotReduceDigivolveCost,
        ModifierType::CannotDigivolveDigimonByEffect,
    ] {
        for result in ["EVO", "BOUNDARY"] {
            let (mut r, h) = setup(&[ID, result]);
            r.game.modifiers.add_player_modifier(
                0,
                PlayerModifierEntry::simple(restriction, 0, Expiry::Permanent, None, 0),
            );
            use_option(&mut r);
            if result == "EVO" && restriction != ModifierType::CannotDigivolveDigimonByEffect {
                first(&mut r);
                first(&mut r);
                assert_eq!(stack(&r, h), vec![ID, "HOST", "EVO"]);
                assert_eq!(
                    r.memory(),
                    5,
                    "full printed cost is payable despite reduction prohibition"
                );
            } else {
                assert!(
                    r.pending_selection().is_none(),
                    "{restriction:?}, result={result}"
                );
                assert_eq!(stack(&r, h), vec!["HOST"]);
                assert_eq!(r.memory(), 8);
            }
        }
    }
    let (mut r, h) = setup(&[ID, "EVO"]);
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::CannotDigivolve, 0, Expiry::Permanent, 0),
    );
    use_option(&mut r);
    assert!(r.pending_selection().is_none());
}
#[test]
fn main_reducer_is_player_visible_and_can_be_accepted_or_declined() {
    for accept in [false, true] {
        let (mut r, h) = setup(&[ID, "EVO"]);
        let t = r.place_on_field(0, "PROTO-REDUCER", Some(0));
        use_option(&mut r);
        first(&mut r);
        first(&mut r);
        let v = r.pending_selection_view().expect("optional reducer");
        assert!(v.is_optional);
        choose(&mut r, if accept { v.valid_action_ids[0] } else { PASS });
        assert_eq!(stack(&r, h), vec![ID, "HOST", "EVO"]);
        assert_eq!(r.memory(), if accept { 8 } else { 6 });
        assert_eq!(
            r.game.players[0].battle_area[t.index as usize].is_suspended,
            accept
        );
    }
}
#[test]
fn selected_host_departure_cannot_evolve_or_attach_to_new_same_slot_host() {
    let (mut r, _) = setup(&[ID, "EVO"]);
    use_option(&mut r);
    first(&mut r);
    r.game.players[0].battle_area.remove(0);
    let fresh = r.place_on_field(0, "HOST", Some(0));
    first(&mut r);
    assert_eq!(stack(&r, fresh), vec!["HOST"]);
    assert_eq!(count_hand(&r, 0, "EVO"), 1);
    assert_eq!(count_trash(&r, 0, ID), 1);
}
#[test]
fn inherited_returns_chosen_digimon_and_places_chosen_x_card_then_deletion_continues() {
    let mut r = builder().start();
    main(&mut r);
    let h = r.place_stack(0, &["EGG", "SOURCE-A", "SOURCE-B", X, ID, "HOST"]);
    let source_b = r.game.players[0].battle_area[0].card_sources[2].handle();
    let x = r.game.players[0].battle_area[0].card_sources[3].handle();
    delete(&mut r, h, ReplacementCause::OpponentEffect);
    let v = r
        .pending_selection_view()
        .expect("mandatory Digimon source pick");
    assert!(!v.is_optional);
    assert_eq!(
        v.valid_action_ids.len(),
        2,
        "exclude Digi-Egg and visible top"
    );
    choose(&mut r, v.valid_action_ids[1]);
    assert_eq!(r.game.players[0].hand[0].handle(), source_b);
    let v = r
        .pending_selection_view()
        .expect("mandatory X Antibody source pick");
    assert!(!v.is_optional);
    assert_eq!(v.valid_action_ids.len(), 2, "Proto Form is also X Antibody");
    let saved = r.game.clone();
    for game in [saved.clone(), saved] {
        r.game = game;
        first(&mut r);
        assert_eq!(
            r.battle_area_size(0),
            0,
            "original deletion is not prevented"
        );
        assert_eq!(r.game.players[0].security.last().unwrap().handle(), x);
        assert!(!r.game.players[0].face_up_security.contains(&x.0));
        assert_eq!(count_trash(&r, 0, "HOST"), 1);
        assert_eq!(count_hand(&r, 0, "SOURCE-B"), 1);
        assert!(r.pending_selection().is_none());
    }
}
#[test]
fn inherited_proto_only_source_still_moves_to_security_without_digimon_return() {
    let mut r = builder().start();
    main(&mut r);
    let h = r.place_stack(0, &[ID, "HOST"]);
    delete(&mut r, h, ReplacementCause::Battle);
    mandatory(&mut r);
    assert_eq!(security_top(&r, 0), ID);
    assert_eq!(count_hand(&r, 0, "HOST"), 0);
    assert_eq!(r.battle_area_size(0), 0);
}
#[test]
fn inherited_does_available_return_even_when_security_addition_is_prohibited() {
    let mut r = builder().start();
    main(&mut r);
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(
            ModifierType::CannotAddSecurityByEffect,
            0,
            Expiry::Permanent,
            None,
            0,
        ),
    );
    let h = r.place_stack(0, &["SOURCE-A", ID, "HOST"]);
    delete(&mut r, h, ReplacementCause::OpponentEffect);
    mandatory(&mut r);
    assert_eq!(count_hand(&r, 0, "SOURCE-A"), 1);
    assert_eq!(r.game.players[0].security.len(), 2);
    assert_eq!(count_trash(&r, 0, ID), 1);
    assert_eq!(r.battle_area_size(0), 0);
}
#[test]
fn inherited_own_effect_excluded_but_opponents_effect_and_battle_work() {
    for cause in [
        ReplacementCause::OwnEffect,
        ReplacementCause::OpponentEffect,
        ReplacementCause::Battle,
    ] {
        let mut r = builder().start();
        main(&mut r);
        let h = r.place_stack(0, &["SOURCE-A", ID, "HOST"]);
        delete(&mut r, h, cause);
        mandatory(&mut r);
        let expected = if cause == ReplacementCause::OwnEffect {
            0
        } else {
            1
        };
        assert_eq!(count_hand(&r, 0, "SOURCE-A"), expected, "{cause:?}");
        assert_eq!(r.game.players[0].security.len(), 2 + expected);
    }
}
#[test]
fn inherited_is_local_to_leaving_host_and_does_not_activate_from_other_stack() {
    let mut r = builder().start();
    main(&mut r);
    let keeper = r.place_stack(0, &["SOURCE-A", ID, "HOST"]);
    let other = r.place_on_field(0, "HOST", Some(0));
    delete(&mut r, other, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection().is_none());
    assert_eq!(stack(&r, keeper), vec!["SOURCE-A", ID, "HOST"]);
    assert_eq!(r.game.players[0].security.len(), 2);
}
#[test]
fn inherited_return_to_hand_is_not_restricted_to_deletion() {
    let mut r = builder().start();
    main(&mut r);
    let h = r.place_stack(0, &["SOURCE-A", ID, "HOST"]);
    r.game.return_to_hand_from_effect(h, 1);
    mandatory(&mut r);
    assert_eq!(count_hand(&r, 0, "SOURCE-A"), 1);
    assert_eq!(count_hand(&r, 0, "HOST"), 1);
    assert_eq!(security_top(&r, 0), ID);
    assert_eq!(r.battle_area_size(0), 0);
}
#[test]
fn inherited_resolves_for_nonturn_owner_and_clone_without_moving_opponents_cards() {
    let mut r = builder().start();
    main(&mut r);
    let h = r.place_stack(1, &["SOURCE-A", ID, "HOST"]);
    r.place_stack(0, &["SOURCE-B", X, "HOST"]);
    delete(&mut r, h, ReplacementCause::OpponentEffect);
    assert_eq!(r.pending_selection_view().unwrap().selecting_player, 1);
    let saved = r.game.clone();
    for game in [saved.clone(), saved] {
        r.game = game;
        mandatory(&mut r);
        assert_eq!(count_hand(&r, 1, "SOURCE-A"), 1);
        assert_eq!(count_hand(&r, 0, "SOURCE-A"), 0);
        assert_eq!(security_top(&r, 1), ID);
        assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 3);
    }
}
#[test]
fn inherited_immediate_effect_still_resolves_after_another_effect_prevents_departure() {
    let mut r = builder().start();
    main(&mut r);
    let h = r.place_stack(0, &["SOURCE-A", ID, "PROTO-PREVENT"]);
    delete(&mut r, h, ReplacementCause::OpponentEffect);
    let v = r
        .pending_selection_view()
        .expect("controller chooses immediate effect order");
    let choices = v.effect_choices.expect("order labels");
    let a = choices
        .iter()
        .find(|c| c.label.contains("when_would_be_deleted"))
        .expect("prevention first")
        .action_id;
    choose(&mut r, a);
    mandatory(&mut r);
    assert_eq!(r.battle_area_size(0), 1);
    assert_eq!(stack(&r, h), vec!["PROTO-PREVENT"]);
    assert_eq!(count_hand(&r, 0, "SOURCE-A"), 1);
    assert_eq!(security_top(&r, 0), ID);
}
#[test]
fn facedown_inherited_source_does_not_activate() {
    let mut r = builder().start();
    main(&mut r);
    let h = r.place_stack(0, &["SOURCE-A", ID, "HOST"]);
    r.game.players[0].battle_area[0].card_sources[1].face_down = true;
    delete(&mut r, h, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection().is_none());
    assert_eq!(count_hand(&r, 0, "SOURCE-A"), 0);
    assert_eq!(r.game.players[0].security.len(), 2);
}
#[test]
fn actual_security_check_adds_exact_option_to_owners_hand() {
    let mut r = builder().security(1, &[ID]).start();
    main(&mut r);
    let identity = r.game.players[1].security.last().unwrap().handle();
    let h = r.place_on_field(0, "HOST", Some(0));
    r.game.turn_count = 2;
    let a = encode_attack(h.index as u16, SECURITY_TARGET);
    assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
    r.game.decode_action(a, 0);
    for _ in 0..6 {
        let Some(v) = r.pending_selection_view() else {
            break;
        };
        assert!(v.is_optional);
        choose(&mut r, PASS);
    }
    assert_eq!(count_hand(&r, 1, ID), 1);
    assert_eq!(r.game.players[1].hand[0].handle(), identity);
    assert_eq!(count_trash(&r, 1, ID), 0);
    assert_eq!(r.memory(), 8);
}
