//! BT18-076: mandatory draw/trash, chosen trash Hybrid evolution,
//! and a non-cancelling inherited would-leave Tamer play.
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::owned_card_choice::CardChoiceOperation;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::TriggerSource;

fn base() -> DebugRunnerBuilder {
    let mut tamer = make_test_card("TAMER", "Tamer");
    tamer.card_kind = CardKind::Tamer;
    tamer.colors = vec![CardColor::Yellow];
    tamer.inherited_text = "[Your Turn] This Digimon gets +1000 DP.".into();
    let mut blank_tamer = tamer.clone();
    blank_tamer.card_id = "NO-INHERITED".into();
    blank_tamer.inherited_text.clear();
    blank_tamer.security_text = "[Security] Play this card.".into();
    let mut inherited_digimon = make_test_card("WRONG-KIND", "Wrong Kind");
    inherited_digimon.inherited_text = tamer.inherited_text.clone();
    let mut yellow_hybrid = make_test_card("YELLOW-HYBRID", "Yellow Hybrid");
    yellow_hybrid.colors = vec![CardColor::Yellow];
    yellow_hybrid.traits = vec!["Hybrid".into()];
    yellow_hybrid.level = Some(5);
    yellow_hybrid.evo_costs = vec![digimon_engine::card_data::EvoCost {
        card_color: 2,
        level: 4,
        memory_cost: 4,
    }];
    let mut red_hybrid = yellow_hybrid.clone();
    red_hybrid.card_id = "RED-HYBRID".into();
    red_hybrid.colors = vec![CardColor::Red];
    let mut non_hybrid = yellow_hybrid.clone();
    non_hybrid.card_id = "NON-HYBRID".into();
    non_hybrid.traits.clear();
    DebugRunner::builder()
        .dsl_card("BT18-076")
        .expect("BT18-076 loads")
        .add_card(tamer)
        .add_card(blank_tamer)
        .add_card(inherited_digimon)
        .add_card(yellow_hybrid)
        .add_card(red_hybrid)
        .add_card(non_hybrid)
        .add_card(make_test_card("TOP", "Top"))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 20])
        .deck(1, &["PAD"; 20])
        .memory(10)
}
fn pick(r: &mut DebugRunner) {
    let view = r.pending_selection_view().expect("player-visible choice");
    let player = view.selecting_player;
    let action = *view
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("legal pick");
    assert!(
        digimon_engine::action::mask::build_action_mask(&r.game, player)[action as usize] > 0.5
    );
    r.execute_action(player, action)
        .expect("choose legal action");
}
fn field(r: &DebugRunner, id: &str) -> bool {
    r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == id)
}

fn source_pick(r: &mut DebugRunner, position: usize) {
    let view = r
        .pending_selection_view()
        .expect("explicit source-card choice");
    let player = view.selecting_player;
    let page = view
        .card_choice
        .expect("source choice uses the physical paging owner");
    let position = position.to_string();
    let row = page
        .rows
        .iter()
        .find(|row| {
            matches!(row.operation, CardChoiceOperation::CommitCard)
                && row.position.as_deref() == Some(position.as_str())
        })
        .expect("the requested physical source remains reachable");
    assert_eq!(row.card_id.as_deref(), Some("TAMER"));
    let mask = digimon_engine::action::mask::build_action_mask(&r.game, player);
    for &action in &view.valid_action_ids {
        assert!(
            mask[action as usize] > 0.5,
            "every presented action is legal"
        );
    }
    assert!(mask[row.action_id as usize] > 0.5);
    r.execute_action(player, row.action_id).unwrap();
}

fn source_choice_count(r: &DebugRunner) -> usize {
    r.pending_selection_view()
        .expect("source prompt")
        .card_choice
        .expect("typed source paging")
        .rows
        .iter()
        .filter(|row| matches!(row.operation, CardChoiceOperation::CommitCard))
        .count()
}
#[test]
fn bt18_076_metadata_and_routes() {
    let r = base().start();
    let c = r.compiled_card("BT18-076").unwrap();
    assert_eq!(c.name, "Loweemon");
    assert_eq!(c.level, Some(4));
    assert_eq!(c.dp, Some(5000));
    assert_eq!(c.alt_paths.len(), 4);
    assert!(c.all_traits().contains(&"Hybrid".into()));
    assert!(c.all_traits().contains(&"Variable".into()));
    assert!(c.all_traits().contains(&"Warrior".into()));
}
#[test]
fn bt18_076_inherited_opponent_and_battle_play_tamer_before_original_leaves() {
    for cause in [ReplacementCause::OpponentEffect, ReplacementCause::Battle] {
        let mut r = base().start();
        let host = r.place_stack(
            0,
            &["TAMER", "NO-INHERITED", "WRONG-KIND", "BT18-076", "TOP"],
        );
        r.game.delete_permanent_with_cause(host, cause);
        assert!(r.pending_is_optional());
        assert!(field(&r, "TOP"), "original stack waits for choice");
        pick(&mut r);
        assert_eq!(
            source_choice_count(&r),
            1,
            "only Tamer with inherited text eligible"
        );
        pick(&mut r);
        r.auto_resolve().unwrap();
        assert!(field(&r, "TAMER"));
        assert!(!field(&r, "TOP"), "play does not prevent original leave");
        assert_eq!(r.memory(), 10, "free play");
    }
}
#[test]
fn bt18_076_inherited_decline_still_leaves() {
    let mut r = base().start();
    let host = r.place_stack(0, &["TAMER", "BT18-076", "TOP"]);
    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    assert!(r.pending_is_optional());
    r.execute_action(0, PASS).unwrap();
    r.auto_resolve().unwrap();
    assert!(!field(&r, "TAMER"));
    assert!(!field(&r, "TOP"));
}
#[test]
fn bt18_076_inherited_excludes_own_effect_and_ineligible_sources() {
    for (sources, cause) in [
        (
            vec!["TAMER", "BT18-076", "TOP"],
            ReplacementCause::OwnEffect,
        ),
        (
            vec!["NO-INHERITED", "WRONG-KIND", "BT18-076", "TOP"],
            ReplacementCause::OpponentEffect,
        ),
        (vec!["BT18-076", "TOP"], ReplacementCause::Battle),
    ] {
        let mut r = base().start();
        let host = r.place_stack(0, &sources);
        r.game.delete_permanent_with_cause(host, cause);
        assert!(r.pending_selection().is_none());
        assert!(!field(&r, "TOP"));
        assert!(!field(&r, "TAMER"));
    }
}

fn attack_setup() -> DebugRunner {
    let mut r = base().start();
    for id in ["BT18-076", "WRONG-KIND", "PAD"] {
        r.inject_trash(0, id);
    }
    let attacker = r.place_stack(0, &["BT18-076"]);
    r.place_stack(0, &["TAMER"]);
    r.place_stack(0, &["PAD"]);
    r.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(attacker),
    );
    r.game.drain_effect_queue();
    r
}
#[test]
fn bt18_076_attack_can_choose_other_tamer_and_pays_printed_cost() {
    let mut r = attack_setup();
    assert!(r.pending_is_optional());
    assert_eq!(r.pending_action_count(), 1, "one eligible Tamer");
    pick(&mut r);
    assert_eq!(r.pending_action_count(), 1, "one legal Hybrid");
    assert!(r.pending_is_optional());
    assert!(digimon_engine::action::mask::build_action_mask(&r.game, 0)[PASS as usize] > 0.5);
    pick(&mut r);
    r.auto_resolve().unwrap();
    assert_eq!(
        r.game.players[0]
            .battle_area
            .iter()
            .filter(|p| p.top_card().card_id(&r.game.card_data) == "BT18-076")
            .count(),
        2
    );
    assert_eq!(r.memory(), 7, "printed Tamer cost 3");
}
#[test]
fn bt18_076_attack_may_decline_host_or_card() {
    for decline_card in [false, true] {
        let mut r = attack_setup();
        if decline_card {
            pick(&mut r);
        }
        r.execute_action(0, PASS).unwrap();
        r.auto_resolve().unwrap();
        assert!(field(&r, "TAMER"));
        assert_eq!(r.memory(), 10);
    }
}
#[test]
fn bt18_076_inherited_pending_choice_clones_independently() {
    let mut r = base().start();
    let host = r.place_stack(0, &["TAMER", "BT18-076", "TOP"]);
    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    let mut cloned = r.game.clone();
    pick(&mut r);
    pick(&mut r);
    r.auto_resolve().unwrap();
    assert!(
        cloned.pending_selection.is_some(),
        "clone retains its independent unresolved choice"
    );
    assert!(field(&r, "TAMER"));
    cloned.drain_effect_queue();
}
#[test]
fn bt18_076_attack_filters_result_color_trait_and_routes() {
    let mut r = base().start();
    for id in ["YELLOW-HYBRID", "RED-HYBRID", "NON-HYBRID", "BT18-076"] {
        r.inject_trash(0, id);
    }
    let attacker = r.place_stack(0, &["BT18-076"]);
    r.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(attacker),
    );
    r.game.drain_effect_queue();
    assert_eq!(
        r.pending_action_count(),
        1,
        "attacker is the only legal host; optional PASS remains available through the mask"
    );
    pick(&mut r);
    assert_eq!(
        r.pending_action_count(),
        1,
        "only yellow Hybrid level 5 has a legal route"
    );
    pick(&mut r);
    r.auto_resolve().unwrap();
    assert!(field(&r, "YELLOW-HYBRID"));
    assert_eq!(r.memory(), 6);
}
#[test]
fn bt18_076_attack_has_no_unpayable_host_candidates() {
    let mut r = base().memory(-8).start();
    for id in ["BT18-076"] {
        r.inject_trash(0, id);
    }
    let attacker = r.place_stack(0, &["BT18-076"]);
    r.place_stack(0, &["TAMER"]);
    r.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(attacker),
    );
    r.game.drain_effect_queue();
    assert!(
        r.pending_selection().is_none(),
        "cannot pay printed 3 across memory boundary"
    );
    assert!(field(&r, "TAMER"));
}

#[test]
fn bt18_076_draw_then_mandatory_hand_trash_choice() {
    let mut r = base().hand(0, &["TAMER", "WRONG-KIND"]).start();
    let host = r.place_stack(0, &["BT18-076"]);
    let before = r.game.players[0].hand.len();
    r.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(host),
    );
    r.game.drain_effect_queue();
    assert_eq!(r.game.players[0].hand.len(), before + 1);
    assert!(!r.pending_is_optional());
    assert_eq!(r.pending_action_count(), before + 1);
    let view = r.pending_selection_view().unwrap();
    assert!(!view.valid_action_ids.contains(&PASS));
    for &action in &view.valid_action_ids {
        assert!(digimon_engine::action::mask::build_action_mask(&r.game, 0)[action as usize] > 0.5);
    }
    let drawn = *view.valid_action_ids.last().unwrap();
    r.execute_action(0, drawn).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[0].hand.len(), before);
    assert_eq!(r.game.players[0].trash.len(), 1);
    assert_eq!(
        r.game.players[0].trash[0].card_id(&r.game.card_data),
        "PAD",
        "the newly drawn card is selectable for mandatory discard"
    );
}

#[test]
fn bt18_076_attack_never_uses_hand_or_wrong_trash_cards() {
    let mut r = base().hand(0, &["BT18-076"]).start();
    r.inject_trash(0, "RED-HYBRID");
    r.inject_trash(0, "NON-HYBRID");
    let attacker = r.place_stack(0, &["BT18-076"]);
    r.place_stack(0, &["TAMER"]);
    r.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(attacker),
    );
    r.game.drain_effect_queue();
    assert!(r.pending_selection().is_none());
    assert_eq!(r.memory(), 10);
}

#[test]
fn bt18_076_native_tamer_and_koichi_routes_expose_and_pay_printed_costs() {
    use digimon_engine::action::{mask::build_action_mask, space::encode_digivolve};
    use digimon_engine::enums::GamePhase;
    for (name, kind, level, color, cost) in [
        ("Generic Tamer", CardKind::Tamer, None, CardColor::Purple, 3),
        (
            "Generic Yellow Tamer",
            CardKind::Tamer,
            None,
            CardColor::Yellow,
            3,
        ),
        ("Koichi Kimura", CardKind::Tamer, None, CardColor::Purple, 2),
        (
            "KaiserLeomon",
            CardKind::Digimon,
            Some(4),
            CardColor::Purple,
            0,
        ),
        (
            "Purple Rookie",
            CardKind::Digimon,
            Some(3),
            CardColor::Purple,
            3,
        ),
        (
            "Yellow Rookie",
            CardKind::Digimon,
            Some(3),
            CardColor::Yellow,
            3,
        ),
    ] {
        let mut tamer = make_test_card("ROUTE-TAMER", name);
        tamer.card_kind = kind;
        tamer.level = level;
        tamer.colors = vec![color];
        let mut r = base().add_card(tamer).hand(0, &["BT18-076"]).start();
        r.place_on_field(0, "ROUTE-TAMER", Some(0));
        r.game.current_phase = GamePhase::Main;
        let action = encode_digivolve(0, 0);
        assert!(build_action_mask(&r.game, 0)[action as usize] > 0.5);
        r.game.decode_action(action, 0);
        if let Some(view) = r.pending_selection_view() {
            if let Some(choices) = view.effect_choices {
                let pick = choices
                    .iter()
                    .find(|e| e.label == format!("Digivolve for cost {cost}"))
                    .unwrap();
                r.execute_action(0, pick.action_id).unwrap();
            }
        }
        r.auto_resolve().unwrap();
        assert_eq!(r.memory(), 10 - cost);
        assert!(field(&r, "BT18-076"));
    }
}

#[test]
fn bt18_076_koichi_rule_alias_can_choose_named_cost_two_route() {
    use digimon_engine::action::{mask::build_action_mask, space::encode_digivolve};
    use digimon_engine::enums::GamePhase;
    let mut alias = make_test_card("KOICHI-ALIAS", "Combined Tamer");
    alias.card_kind = CardKind::Tamer;
    alias.level = None;
    alias.colors = vec![CardColor::Purple];
    alias.also_treated_as = vec!["Koichi Kimura".into()];
    let mut r = base().add_card(alias).hand(0, &["BT18-076"]).start();
    r.place_on_field(0, "KOICHI-ALIAS", Some(0));
    r.game.current_phase = GamePhase::Main;
    let action = encode_digivolve(0, 0);
    assert!(build_action_mask(&r.game, 0)[action as usize] > 0.5);
    r.game.decode_action(action, 0);
    let view = r
        .pending_selection_view()
        .expect("generic and named route choices");
    let choices = view.effect_choices.expect("printed evolution costs");
    assert!(choices.iter().any(|e| e.label == "Digivolve for cost 3"));
    let named = choices
        .iter()
        .find(|e| e.label == "Digivolve for cost 2")
        .expect("Rule alias offers Koichi cost 2");
    assert!(build_action_mask(&r.game, 0)[named.action_id as usize] > 0.5);
    r.execute_action(0, named.action_id).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 8);
    assert!(field(&r, "BT18-076"));
}

#[test]
fn bt18_076_named_routes_do_not_add_unprinted_card_kind_requirements() {
    use digimon_engine::action::{mask::build_action_mask, space::encode_digivolve};
    use digimon_engine::enums::GamePhase;
    for (name, kind, cost) in [
        ("Koichi Kimura", CardKind::Digimon, 2),
        ("KaiserLeomon", CardKind::Tamer, 0),
    ] {
        for alias in [false, true] {
            let mut host = make_test_card("NAMED-HOST", if alias { "Rule Alias" } else { name });
            host.card_kind = kind;
            host.level = if kind == CardKind::Digimon {
                Some(6)
            } else {
                None
            };
            host.colors = vec![CardColor::Red];
            if alias {
                host.also_treated_as = vec![name.into()];
            }
            let mut r = base().add_card(host).hand(0, &["BT18-076"]).start();
            r.place_on_field(0, "NAMED-HOST", Some(0));
            r.game.current_phase = GamePhase::Main;
            let action = encode_digivolve(0, 0);
            assert!(
                build_action_mask(&r.game, 0)[action as usize] > 0.5,
                "printed named route must allow {name}, {kind:?}, alias={alias}"
            );
            r.game.decode_action(action, 0);
            r.auto_resolve().unwrap();
            assert_eq!(
                r.memory(),
                10 - cost,
                "exact named cost for {name}, alias={alias}"
            );
            assert!(field(&r, "BT18-076"));
        }
    }
}

#[test]
fn bt18_076_deep_inherited_tamer_choices_use_the_exact_physical_source() {
    for index in [11usize, 12, 39] {
        let mut r = base().start();
        let mut stack = vec!["BT18-076", "NO-INHERITED", "WRONG-KIND"];
        stack.resize(index, "PAD");
        stack.extend(["TAMER", "TOP"]);
        let host = r.place_stack(0, &stack);
        let selected = r.game.players[0].battle_area[0].card_sources[index].handle();
        r.game
            .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
        assert!(r.pending_is_optional());
        assert!(field(&r, "TOP"), "original leave waits for the decision");
        pick(&mut r);
        assert_eq!(
            source_choice_count(&r),
            1,
            "blank Tamers and Digimon stay excluded"
        );
        source_pick(&mut r, index);
        r.auto_resolve().unwrap();
        assert!(r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().handle() == selected));
        assert!(!field(&r, "TOP"), "play never cancels the original leave");
        assert_eq!(r.memory(), 10, "free play");
    }
}

#[test]
fn bt18_076_cloned_deep_prompt_can_choose_different_copies_with_the_same_printed_id() {
    let mut r = base().start();
    let mut stack = vec!["BT18-076", "NO-INHERITED", "WRONG-KIND"];
    stack.resize(12, "PAD");
    stack.push("TAMER");
    stack.resize(39, "PAD");
    stack.extend(["TAMER", "TOP"]);
    let host = r.place_stack(0, &stack);
    let first = r.game.players[0].battle_area[0].card_sources[12].handle();
    let second = r.game.players[0].battle_area[0].card_sources[39].handle();
    assert_ne!(first, second);
    r.game
        .delete_permanent_with_cause(host, ReplacementCause::Battle);
    pick(&mut r);
    assert_eq!(source_choice_count(&r), 2);
    let saved = r.game.clone();
    for (index, selected, unselected) in [(12, first, second), (39, second, first)] {
        r.game = saved.clone();
        source_pick(&mut r, index);
        r.auto_resolve().unwrap();
        assert!(r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().handle() == selected));
        assert!(!r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().handle() == unselected));
        assert!(r.game.players[0]
            .trash
            .iter()
            .any(|c| c.handle() == unselected));
        assert!(!field(&r, "TOP"));
        assert_eq!(r.memory(), 10);
    }
    assert!(
        saved.pending_selection.is_some(),
        "saved prompt remains unresolved"
    );
}

#[test]
fn bt18_076_source_pages_do_not_move_cards_before_the_selected_tamer_is_played() {
    use digimon_engine::card_choice_page::CARD_CHOICE_NEXT;
    use rand::RngCore;
    let mut r = base().start();
    let mut stack = vec!["BT18-076"];
    stack.extend(std::iter::repeat("TAMER").take(65));
    stack.push("TOP");
    let host = r.place_stack(0, &stack);
    let selected = r.game.players[0].battle_area[0].card_sources[65].handle();
    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    pick(&mut r);
    let before = r.game.players[0].battle_area[0]
        .card_sources
        .iter()
        .map(|c| c.handle())
        .collect::<Vec<_>>();
    let mut expected_rng = r.game.rng.clone();
    for page in [1, 2] {
        assert!(
            digimon_engine::action::mask::build_action_mask(&r.game, 0)[CARD_CHOICE_NEXT as usize]
                > 0.5
        );
        r.execute_action(0, CARD_CHOICE_NEXT).unwrap();
        let owner = r.pending_selection_view().unwrap().card_choice.unwrap();
        assert_eq!(owner.page_index, page.to_string());
        assert_eq!(owner.page_count, "3");
        assert_eq!(
            r.game.players[0].battle_area[0]
                .card_sources
                .iter()
                .map(|c| c.handle())
                .collect::<Vec<_>>(),
            before
        );
        assert_eq!(r.memory(), 10);
        assert!(r.game.players[0].trash.is_empty());
    }
    assert_eq!(
        r.game.rng.next_u64(),
        expected_rng.next_u64(),
        "paging consumes no RNG"
    );
    source_pick(&mut r, 65);
    r.auto_resolve().unwrap();
    assert!(r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().handle() == selected));
    assert!(!field(&r, "TOP"));
}

#[test]
fn bt18_076_may_decline_play_processing_after_accepting_the_inherited_activation() {
    let mut r = base().start();
    let mut stack = vec!["BT18-076"];
    stack.resize(12, "PAD");
    stack.extend(["TAMER", "TOP"]);
    let host = r.place_stack(0, &stack);
    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    pick(&mut r);
    assert!(
        r.pending_is_optional(),
        "printed may-play processing remains optional"
    );
    assert!(digimon_engine::action::mask::build_action_mask(&r.game, 0)[PASS as usize] > 0.5);
    r.execute_action(0, PASS).unwrap();
    r.auto_resolve().unwrap();
    assert!(!field(&r, "TAMER"));
    assert!(
        !field(&r, "TOP"),
        "declining the play does not cancel the original leave"
    );
    assert_eq!(r.memory(), 10);
}

#[test]
fn bt18_076_deep_source_choice_is_owned_by_the_controller_in_the_second_seat() {
    let mut r = base().start();
    let mut stack = vec!["BT18-076"];
    stack.resize(12, "PAD");
    stack.extend(["TAMER", "TOP"]);
    let host = r.place_stack(1, &stack);
    let selected = r.game.players[1].battle_area[0].card_sources[12].handle();
    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    assert_eq!(r.pending_selection_view().unwrap().selecting_player, 1);
    pick(&mut r);
    let view = r.pending_selection_view().unwrap();
    assert_eq!(view.selecting_player, 1);
    assert!(digimon_engine::action::mask::build_action_mask(&r.game, 0)
        .iter()
        .all(|&v| v == 0.0));
    let owner = view.card_choice.unwrap();
    assert!(owner
        .rows
        .iter()
        .filter(|row| matches!(row.operation, CardChoiceOperation::CommitCard))
        .all(|row| row.owner == Some(1)));
    source_pick(&mut r, 12);
    r.auto_resolve().unwrap();
    assert!(r.game.players[1]
        .battle_area
        .iter()
        .any(|p| p.top_card().handle() == selected));
    assert!(!r.game.players[1]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "TOP"));
    assert!(r.game.players[0].battle_area.is_empty());
}

#[path = "light_hybrid_inherited_order.rs"]
mod inherited_order_regression;

#[test]
fn bt18_076_q3021_played_tamer_and_deleted_digimon_share_player_ordering() {
    inherited_order_regression::prove_simultaneous_order("BT18-076");
}
