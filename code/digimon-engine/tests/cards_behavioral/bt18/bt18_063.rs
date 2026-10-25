//! BT18-063: opponent-only deletion protection, chosen Hybrid evolution,
//! and a non-cancelling inherited would-leave Tamer play.
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
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
    let mut inherited_digimon = make_test_card("WRONG-KIND", "Wrong Kind");
    inherited_digimon.inherited_text = tamer.inherited_text.clone();
    let mut yellow_hybrid = make_test_card("YELLOW-HYBRID", "Yellow Hybrid");
    yellow_hybrid.colors = vec![CardColor::Yellow];
    yellow_hybrid.traits = vec!["Hybrid".into()];
    yellow_hybrid.level = Some(5);
    yellow_hybrid.evo_costs = vec![digimon_engine::card_data::EvoCost {
        card_color: 5,
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
        .dsl_card("BT18-063")
        .expect("BT18-063 loads")
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
    let mask = digimon_engine::action::mask::build_action_mask(&r.game, player);
    for &id in &view.valid_action_ids {
        assert!(
            mask[id as usize] > 0.5,
            "every presented choice is legal in the action mask"
        );
    }
    let action = *view
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("legal pick");
    r.execute_action(player, action)
        .expect("choose legal action");
}
fn field(r: &DebugRunner, id: &str) -> bool {
    r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == id)
}
#[test]
fn bt18_063_metadata_and_routes() {
    let r = base().start();
    let c = r.compiled_card("BT18-063").unwrap();
    assert_eq!(c.name, "Beetlemon");
    assert_eq!(c.level, Some(4));
    assert_eq!(c.dp, Some(5000));
    assert!(c.all_traits().contains(&"Hybrid".into()));
    assert!(c.all_traits().contains(&"Variable".into()));
    assert!(c.all_traits().contains(&"Cyborg".into()));
}
#[test]
fn bt18_063_printed_routes_have_legal_actions_and_exact_payments() {
    use digimon_engine::action::{build_action_mask, space::encode_digivolve};
    use digimon_engine::enums::GamePhase;

    // BT18-063.webp prints black/yellow Lv.3: 3, black/yellow Tamer: 3,
    // [J.P. Shibayama]: 2, and [MetalKabuterimon]: 0.
    for (label, kind, color, level, name, alias, cost) in [
        (
            "black level 3",
            CardKind::Digimon,
            CardColor::Black,
            Some(3),
            "Black Rookie",
            false,
            3,
        ),
        (
            "yellow level 3",
            CardKind::Digimon,
            CardColor::Yellow,
            Some(3),
            "Yellow Rookie",
            false,
            3,
        ),
        (
            "black Tamer",
            CardKind::Tamer,
            CardColor::Black,
            None,
            "Black Tamer",
            false,
            3,
        ),
        (
            "yellow Tamer",
            CardKind::Tamer,
            CardColor::Yellow,
            None,
            "Yellow Tamer",
            false,
            3,
        ),
        (
            "named J.P.",
            CardKind::Tamer,
            CardColor::Green,
            None,
            "J.P. Shibayama",
            false,
            2,
        ),
        (
            "Rule-name J.P.",
            CardKind::Tamer,
            CardColor::Red,
            None,
            "Rule Alias",
            true,
            2,
        ),
        (
            "MetalKabuterimon",
            CardKind::Digimon,
            CardColor::Black,
            Some(4),
            "MetalKabuterimon",
            false,
            0,
        ),
    ] {
        let mut metadata = make_test_card("ROUTE-BASE", name);
        metadata.card_kind = kind;
        metadata.colors = vec![color];
        metadata.level = level;
        if kind == CardKind::Tamer {
            metadata.dp = None;
        }
        if alias {
            metadata.also_treated_as = vec!["J.P. Shibayama".into()];
        }
        let mut r = base().add_card(metadata).hand(0, &["BT18-063"]).start();
        r.place_on_field(0, "ROUTE-BASE", Some(0));
        r.set_phase(GamePhase::Main);
        let action = encode_digivolve(0, 0);
        assert_eq!(
            build_action_mask(&r.game, 0)[action as usize],
            1.0,
            "{label}"
        );
        r.game.decode_action(action, 0);
        if let Some(p) = r.game.pending_selection.as_ref() {
            let route = p
                .effect_choices
                .as_ref()
                .expect("route choice")
                .iter()
                .find(|choice| choice.label == format!("Digivolve for cost {cost}"))
                .expect("printed cost offered")
                .action_id;
            assert!(build_action_mask(&r.game, 0)[route as usize] > 0.5);
            r.execute_action(0, route).unwrap();
        }
        r.auto_resolve().unwrap();
        assert!(field(&r, "BT18-063"), "{label}: evolution committed");
        assert_eq!(r.memory(), 10 - cost, "{label}: exact printed cost");
    }
}
#[test]
fn bt18_063_protects_only_opponent_effect_deletion() {
    for (cause, survives) in [
        (ReplacementCause::OpponentEffect, true),
        (ReplacementCause::OwnEffect, false),
        (ReplacementCause::Battle, false),
    ] {
        let mut r = base().start();
        let host = r.place_stack(0, &["BT18-063"]);
        r.game.enqueue_triggered(
            EffectTiming::WhenDigivolving,
            TriggerSource::Permanent(host),
        );
        r.game.drain_effect_queue();
        r.game.delete_permanent_with_cause(host, cause);
        assert_eq!(field(&r, "BT18-063"), survives, "{cause:?}");
    }
}

fn named_jp_without_a_tamer_requirement(rule_alias: bool) {
    use digimon_engine::action::{build_action_mask, space::encode_digivolve};
    use digimon_engine::enums::GamePhase;
    // Synthetic named subjects establish the printed condition's scope. Green
    // Lv.6 misses both ordinary circles; the named route has no kind/level gate.
    for actor in [0, 1] {
        let mut subject = make_test_card(
            "NAMED-NON-TAMER-JP",
            if rule_alias {
                "Rule Alias"
            } else {
                "J.P. Shibayama"
            },
        );
        subject.card_kind = CardKind::Digimon;
        subject.level = Some(6);
        subject.colors = vec![CardColor::Green];
        if rule_alias {
            subject.also_treated_as = vec!["J.P. Shibayama".into()];
        }
        let mut r = base().add_card(subject).hand(actor, &["BT18-063"]).start();
        r.skip_mulligan();
        r.place_on_field(actor, "NAMED-NON-TAMER-JP", Some(0));
        r.set_first_player(actor);
        r.set_phase(GamePhase::Main);
        // set_first_player also makes this actor the positive seesaw seat.
        r.game.memory = 10;
        let action = encode_digivolve(0, 0);
        assert_eq!(
            build_action_mask(&r.game, actor)[action as usize],
            1.0,
            "the named J.P. route must not require a Tamer: actor={actor}, alias={rule_alias}"
        );
        r.game.decode_action(action, actor);
        r.auto_resolve().unwrap();
        assert_eq!(
            r.game.players[actor as usize].battle_area[0]
                .top_card()
                .card_id(&r.game.card_data),
            "BT18-063",
            "named route commits for actor={actor}, alias={rule_alias}"
        );
        assert_eq!(r.game.memory, 8, "named route pays exactly two");
    }
}

#[test]
fn bt18_063_named_jp_route_accepts_a_non_tamer_subject() {
    named_jp_without_a_tamer_requirement(false);
}

#[test]
fn bt18_063_named_jp_route_accepts_a_non_tamer_rule_name_alias() {
    named_jp_without_a_tamer_requirement(true);
}
#[test]
fn bt18_063_inherited_opponent_and_battle_play_tamer_before_original_leaves() {
    for cause in [ReplacementCause::OpponentEffect, ReplacementCause::Battle] {
        let mut r = base().start();
        let host = r.place_stack(
            0,
            &["TAMER", "NO-INHERITED", "WRONG-KIND", "BT18-063", "TOP"],
        );
        r.game.delete_permanent_with_cause(host, cause);
        assert!(r.pending_is_optional());
        assert!(field(&r, "TOP"), "original stack waits for choice");
        pick(&mut r);
        assert_eq!(
            source_target_count(&r),
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
fn bt18_063_inherited_decline_still_leaves() {
    let mut r = base().start();
    let host = r.place_stack(0, &["TAMER", "BT18-063", "TOP"]);
    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    assert!(r.pending_is_optional());
    r.execute_action(0, PASS).unwrap();
    r.auto_resolve().unwrap();
    assert!(!field(&r, "TAMER"));
    assert!(!field(&r, "TOP"));
}
#[test]
fn bt18_063_inherited_excludes_own_effect_and_ineligible_sources() {
    for (sources, cause) in [
        (
            vec!["TAMER", "BT18-063", "TOP"],
            ReplacementCause::OwnEffect,
        ),
        (
            vec!["NO-INHERITED", "WRONG-KIND", "BT18-063", "TOP"],
            ReplacementCause::OpponentEffect,
        ),
        (vec!["BT18-063", "TOP"], ReplacementCause::Battle),
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
    let mut r = base().hand(0, &["BT18-063", "WRONG-KIND", "PAD"]).start();
    let attacker = r.place_stack(0, &["BT18-063"]);
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
fn bt18_063_attack_can_choose_other_tamer_and_pays_reduced_cost() {
    let mut r = attack_setup();
    assert!(
        r.pending_is_optional(),
        "pending {:?}; hand {:?}",
        r.pending_selection_view(),
        r.game.players[0]
            .hand
            .iter()
            .map(|c| c.card_id(&r.game.card_data))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        r.pending_action_count(),
        1,
        "one eligible Tamer; PASS supplied by optional mask"
    );
    pick(&mut r);
    assert_eq!(
        r.pending_action_count(),
        1,
        "one legal Hybrid; optional mask adds PASS"
    );
    pick(&mut r);
    r.auto_resolve().unwrap();
    assert_eq!(
        r.game.players[0]
            .battle_area
            .iter()
            .filter(|p| p.top_card().card_id(&r.game.card_data) == "BT18-063")
            .count(),
        2
    );
    assert_eq!(r.memory(), 8, "printed Tamer cost 3 reduced to 2");
}
#[test]
fn bt18_063_attack_may_decline_host_or_card() {
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
fn bt18_063_inherited_pending_choice_clones_independently() {
    let mut r = base().start();
    let host = r.place_stack(0, &["TAMER", "BT18-063", "TOP"]);
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
fn bt18_063_protection_expires_after_opponent_turn() {
    let mut r = base().start();
    r.skip_mulligan();
    r.set_first_player(0);
    let host = r.place_stack(0, &["BT18-063"]);
    r.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(host),
    );
    r.game.drain_effect_queue();
    r.pass_turn();
    assert_eq!(r.turn_player(), 1);
    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    assert!(field(&r, "BT18-063"));
    r.pass_turn();
    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    assert!(!field(&r, "BT18-063"));
}
#[test]
fn bt18_063_protection_does_not_stop_return_to_hand() {
    let mut r = base().start();
    let host = r.place_stack(0, &["BT18-063"]);
    r.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(host),
    );
    r.game.drain_effect_queue();
    r.game.return_to_hand(host);
    assert!(!field(&r, "BT18-063"));
}
#[test]
fn bt18_063_attack_filters_result_color_trait_and_routes() {
    let mut r = base()
        .hand(
            0,
            &["YELLOW-HYBRID", "RED-HYBRID", "NON-HYBRID", "BT18-063"],
        )
        .start();
    let attacker = r.place_stack(0, &["BT18-063"]);
    r.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(attacker),
    );
    r.game.drain_effect_queue();
    assert_eq!(
        r.pending_action_count(),
        1,
        "attacker offered explicitly; optional mask adds PASS"
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
    assert_eq!(r.memory(), 7);
}
#[test]
fn bt18_063_attack_has_no_unpayable_host_candidates() {
    let mut r = base().hand(0, &["BT18-063"]).memory(-9).start();
    let attacker = r.place_stack(0, &["BT18-063"]);
    r.place_stack(0, &["TAMER"]);
    r.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(attacker),
    );
    r.game.drain_effect_queue();
    assert!(
        r.pending_selection().is_none(),
        "cannot pay reduced 2 across memory boundary"
    );
    assert!(field(&r, "TAMER"));
}

fn source_choice(r: &DebugRunner, position: usize) -> u16 {
    use digimon_engine::owned_card_choice::CardChoiceOperation;
    let view = r.pending_selection_view().expect("explicit source prompt");
    let page = view.card_choice.expect("owned physical-source page");
    let position = position.to_string();
    let row = page
        .rows
        .iter()
        .find(|row| {
            matches!(row.operation, CardChoiceOperation::CommitCard)
                && row.position.as_deref() == Some(position.as_str())
        })
        .expect("requested physical source is visible on this page");
    assert_eq!(row.card_id.as_deref(), Some("TAMER"));
    assert_eq!(
        digimon_engine::action::build_action_mask(&r.game, 0)[row.action_id as usize],
        1.0,
        "physical source commit is legal",
    );
    row.action_id
}

fn source_target_count(r: &DebugRunner) -> usize {
    use digimon_engine::owned_card_choice::CardChoiceOperation;
    r.pending_selection_view()
        .expect("source prompt")
        .card_choice
        .expect("owned source page")
        .rows
        .iter()
        .filter(|row| matches!(row.operation, CardChoiceOperation::CommitCard))
        .count()
}

#[test]
fn bt18_063_deep_inherited_tamer_is_a_masked_physical_choice_and_clones() {
    for position in [12, 39] {
        let mut r = base().start();
        let mut stack = vec!["BT18-063"];
        stack.extend(std::iter::repeat("PAD").take(position - 1));
        stack.extend(["TAMER", "NO-INHERITED", "WRONG-KIND", "TOP"]);
        let host = r.place_stack(0, &stack);
        let chosen = r.game.players[0].battle_area[0].card_sources[position].handle();
        r.game
            .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
        assert!(
            r.pending_is_optional(),
            "the inherited play must first be accepted"
        );
        pick(&mut r);
        let view = r
            .pending_selection_view()
            .expect("explicit Tamer processing choice after acceptance");
        assert!(
            view.is_optional,
            "printed may-play processing remains optional"
        );
        assert_eq!(
            source_target_count(&r),
            1,
            "wrong-kind and blank inherited cards stay excluded"
        );
        let pending = r.game.clone();
        for game in [r.game.clone(), pending] {
            r.game = game;
            let action = source_choice(&r, position);
            r.execute_action(0, action).unwrap();
            r.auto_resolve().unwrap();
            let played = r.game.players[0]
                .battle_area
                .iter()
                .find(|p| p.top_card().card_id(&r.game.card_data) == "TAMER")
                .expect("the deep Tamer was played");
            assert_eq!(
                played.top_card().handle(),
                chosen,
                "chosen physical copy, not an aliased source slot"
            );
            assert!(
                !field(&r, "TOP"),
                "the inherited play never prevents the original departure"
            );
            assert_eq!(r.memory(), 10, "Tamer play is free");
        }
    }
}

#[test]
fn bt18_063_paged_inherited_tamers_keep_cloned_choices_independent() {
    use digimon_engine::card_choice_page::CARD_CHOICE_NEXT;
    use rand::RngCore;
    let mut r = base().start();
    let mut stack = vec!["BT18-063"];
    stack.extend(std::iter::repeat("TAMER").take(32));
    stack.extend(["NO-INHERITED", "WRONG-KIND", "TOP"]);
    let host = r.place_stack(0, &stack);
    let first = r.game.players[0].battle_area[0].card_sources[1].handle();
    let last = r.game.players[0].battle_area[0].card_sources[32].handle();
    r.game
        .delete_permanent_with_cause(host, ReplacementCause::Battle);
    pick(&mut r);
    let pending = r.game.clone();
    let first_action = source_choice(&r, 1);
    r.execute_action(0, first_action).unwrap();
    r.auto_resolve().unwrap();
    let original_done = r.game.clone();
    assert!(original_done.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().handle() == first));

    r.game = pending;
    let source_handles: Vec<_> = r.game.players[0].battle_area[0]
        .card_sources
        .iter()
        .map(|c| c.handle())
        .collect();
    let before_rng = r.game.rng.clone();
    assert_eq!(
        digimon_engine::action::build_action_mask(&r.game, 0)[CARD_CHOICE_NEXT as usize],
        1.0
    );
    assert_eq!(
        digimon_engine::action::build_action_mask(&r.game, 0)[PASS as usize],
        1.0,
        "accepting activation does not force the printed may-play processing"
    );
    r.execute_action(0, CARD_CHOICE_NEXT).unwrap();
    assert_eq!(r.memory(), 10);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .card_sources
            .iter()
            .map(|c| c.handle())
            .collect::<Vec<_>>(),
        source_handles
    );
    assert_eq!(
        r.game.rng.clone().next_u64(),
        before_rng.clone().next_u64(),
        "navigation consumes no RNG"
    );
    let last_action = source_choice(&r, 32);
    r.execute_action(0, last_action).unwrap();
    r.auto_resolve().unwrap();
    assert!(r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().handle() == last));
    assert!(!r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().handle() == first));
    assert!(!field(&r, "TOP"));
    assert_eq!(r.memory(), 10);
    assert!(original_done.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().handle() == first));
    assert!(!original_done.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().handle() == last));
}

#[test]
fn bt18_063_inherited_play_processing_can_be_declined_after_activation() {
    let mut r = base().start();
    let mut stack = vec!["BT18-063"];
    stack.extend(std::iter::repeat("PAD").take(11));
    stack.extend(["TAMER", "TOP"]);
    let host = r.place_stack(0, &stack);
    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    pick(&mut r);
    assert_eq!(source_target_count(&r), 1);
    assert!(r.pending_is_optional(), "the inherited play says you may");
    assert_eq!(
        digimon_engine::action::build_action_mask(&r.game, 0)[PASS as usize],
        1.0
    );
    let pending = r.game.clone();
    for game in [r.game.clone(), pending] {
        r.game = game;
        r.execute_action(0, PASS).unwrap();
        r.auto_resolve().unwrap();
        assert!(!field(&r, "TAMER"), "declining processing plays nothing");
        assert!(!field(&r, "TOP"), "the original departure still completes");
        assert_eq!(r.memory(), 10);
    }
}

#[test]
fn bt18_063_deep_inherited_choice_is_owned_by_the_second_player() {
    let mut r = base().start();
    let mut stack = vec!["BT18-063"];
    stack.extend(std::iter::repeat("PAD").take(11));
    stack.extend(["TAMER", "TOP"]);
    let host = r.place_stack(1, &stack);
    let chosen = r.game.players[1].battle_area[0].card_sources[12].handle();
    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    assert_eq!(r.pending_selection_view().unwrap().selecting_player, 1);
    pick(&mut r);
    let view = r
        .pending_selection_view()
        .expect("player two's source prompt");
    assert_eq!(view.selecting_player, 1);
    assert!(digimon_engine::action::build_action_mask(&r.game, 0)
        .iter()
        .all(|&v| v == 0.0));
    let row = view
        .card_choice
        .unwrap()
        .rows
        .into_iter()
        .find(|row| row.position.as_deref() == Some("12"))
        .expect("deep source is reachable");
    assert_eq!(row.owner, Some(1));
    assert_eq!(row.card_id.as_deref(), Some("TAMER"));
    assert_eq!(
        digimon_engine::action::build_action_mask(&r.game, 1)[row.action_id as usize],
        1.0
    );
    r.execute_action(1, row.action_id).unwrap();
    r.auto_resolve().unwrap();
    assert!(r.game.players[1]
        .battle_area
        .iter()
        .any(|p| p.top_card().handle() == chosen));
    assert!(!r.game.players[1]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "TOP"));
    assert!(r.game.players[0].battle_area.is_empty());
    assert_eq!(r.memory(), 10);
}

#[path = "light_hybrid_inherited_order.rs"]
mod inherited_order_regression;

#[test]
fn bt18_063_q2994_played_tamer_and_deleted_digimon_share_player_ordering() {
    inherited_order_regression::prove_simultaneous_order("BT18-063");
}
