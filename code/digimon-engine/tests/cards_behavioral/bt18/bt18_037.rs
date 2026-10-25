//! BT18-037 Lobomon — official observation 50bc757c... and inspected card image.
//! [When Digivolving] May add a Hybrid/Ten Warriors security card to hand;
//! if added recover 1, then shuffle security. Inherited [When Attacking]
//! draw 1 with <=7 cards in hand, without OPT.
//! DCGO: Assets/Scripts/CardEffect/BT18/Yellow/BT18_037.cs.
//! Patterns: filtered optional security choice, result receipt, inherited draw.
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::owned_card_choice::CardChoiceOperation;
use digimon_engine::{EffectTiming, TriggerSource};

fn runner(security: &[&str], hand: usize) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT18-037")
        .unwrap()
        .dsl_card("BT17-020")
        .unwrap()
        .dsl_card("BT21-091")
        .unwrap()
        .dsl_card("BT13-012")
        .unwrap()
        .dsl_card("BT18-093")
        .unwrap()
        .dsl_card("BT4-105")
        .unwrap()
        .security(0, security)
        .hand(0, &vec!["BT13-012"; hand])
        .deck(0, &["BT13-012"; 15])
        .memory(10)
        .start()
}
fn search(r: &mut DebugRunner) {
    let host = r.place_on_field(0, "BT18-037", Some(0));
    r.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(host),
    );
    r.game.drain_effect_queue();
}

fn commits(r: &DebugRunner) -> Vec<(u16, usize, String)> {
    let view = r
        .pending_selection_view()
        .expect("explicit security search");
    let page = view.card_choice.expect("owned physical security choices");
    let mask = digimon_engine::action::build_action_mask(&r.game, 0);
    for row in &page.rows {
        let inspect_only =
            row.position.is_some() && !matches!(row.operation, CardChoiceOperation::CommitCard);
        assert_eq!(
            mask[row.action_id as usize],
            if inspect_only { 0.0 } else { 1.0 }
        );
    }
    page.rows
        .into_iter()
        .filter(|row| matches!(row.operation, CardChoiceOperation::CommitCard))
        .map(|row| {
            assert!(
                row.identity_visible,
                "printed search permits the controller to inspect candidates"
            );
            let index = row
                .position
                .expect("current security position")
                .parse()
                .unwrap();
            let card = row
                .card_id
                .expect("own search must present the candidate card identity");
            (row.action_id, index, card)
        })
        .collect()
}

fn inspect_entire_security(r: &mut DebugRunner, eligible: Option<usize>) -> Option<u16> {
    use digimon_engine::card_choice_page::CARD_CHOICE_NEXT;
    use rand::RngCore;
    let original = security_handles(r);
    let before_rng = r.game.rng.clone().next_u64();
    let mut seen = Vec::new();
    let mut commit = None;
    loop {
        let view = r
            .pending_selection_view()
            .expect("nonempty security remains inspectable even without a match");
        let page = view.card_choice.expect("full-stack security inspection");
        let mask = digimon_engine::action::build_action_mask(&r.game, 0);
        for row in page.rows.iter().filter(|row| row.position.is_some()) {
            let index = row.position.as_ref().unwrap().parse::<usize>().unwrap();
            assert!(row.identity_visible);
            assert_eq!(
                row.card_id.as_deref(),
                Some(if eligible == Some(index) {
                    "BT17-020"
                } else {
                    "BT13-012"
                })
            );
            assert_eq!(
                mask[row.action_id as usize],
                if eligible == Some(index) { 1.0 } else { 0.0 },
                "only an eligible inspected card may be committed"
            );
            if eligible == Some(index) {
                commit = Some(row.action_id);
            }
            seen.push(index);
        }
        let other = r
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .card_choice
            .as_ref()
            .unwrap()
            .view(&r.game, 1);
        assert!(other.rows.iter().all(|row| !row.identity_visible
            && row.card_id.is_none()
            && row.card_registry_index.is_none()));
        if mask[CARD_CHOICE_NEXT as usize] == 0.0 {
            break;
        }
        r.execute_action(0, CARD_CHOICE_NEXT).unwrap();
        assert_eq!(security_handles(r), original);
        assert_eq!(r.game.rng.clone().next_u64(), before_rng);
        assert_eq!(r.hand_size(0), 0);
        assert_eq!(r.deck_size(0), 15);
    }
    assert_eq!(
        seen,
        (0..40).collect::<Vec<_>>(),
        "search permits inspection of every security card, including ineligible cards"
    );
    commit
}

#[test]
fn bt18_037_full_stack_inspection_includes_ineligible_cards() {
    use rand::RngCore;
    let mut cards = vec!["BT13-012"; 40];
    cards[39] = "BT17-020";
    let mut r = runner(&cards, 0);
    let selected = r.game.players[0].security[39].handle();
    let mut expected = r.game.clone();
    expected.players[0].security.remove(39);
    let recovered = expected.players[0].deck.pop().unwrap();
    expected.players[0].put_on_security_top(recovered);
    expected.shuffle_security(0);
    search(&mut r);
    let action = inspect_entire_security(&mut r, Some(39)).unwrap();
    r.execute_action(0, action).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[0].hand[0].handle(), selected);
    assert_eq!(r.deck_size(0), 14);
    assert_eq!(
        security_handles(&r),
        expected.players[0]
            .security
            .iter()
            .map(|c| c.handle())
            .collect::<Vec<_>>()
    );
    assert_eq!(r.game.rng.next_u64(), expected.rng.next_u64());
}

#[test]
fn bt18_037_full_stack_inspection_without_matches_finishes_then_shuffles_once() {
    use rand::RngCore;
    let mut r = runner(&["BT13-012"; 40], 0);
    let mut expected = r.game.clone();
    expected.shuffle_security(0);
    search(&mut r);
    assert!(inspect_entire_security(&mut r, None).is_none());
    assert_eq!(
        digimon_engine::action::build_action_mask(&r.game, 0)[PASS as usize],
        1.0
    );
    r.execute_action(0, PASS).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.hand_size(0), 0);
    assert_eq!(r.deck_size(0), 15);
    assert_eq!(
        security_handles(&r),
        expected.players[0]
            .security
            .iter()
            .map(|c| c.handle())
            .collect::<Vec<_>>()
    );
    assert_eq!(r.game.rng.next_u64(), expected.rng.next_u64());
}

fn security_handles(r: &DebugRunner) -> Vec<digimon_engine::card_source::CardHandle> {
    r.game.players[0]
        .security
        .iter()
        .map(|card| card.handle())
        .collect()
}
#[test]
fn bt18_037_search_filters_traits_and_adds_then_recovers() {
    for pick in 0..2 {
        let mut r = runner(
            &["BT17-020", "BT21-091", "BT13-012", "BT18-093", "BT4-105"],
            0,
        );
        search(&mut r);
        assert!(r.game.pending_selection.as_ref().unwrap().is_optional);
        let choices = commits(&r);
        assert_eq!(
            choices.len(),
            2,
            "Hybrid and Ten Warriors only, including trait-bearing Option"
        );
        assert_eq!(
            choices
                .iter()
                .map(|(_, index, _)| *index)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert_eq!(
            choices
                .iter()
                .map(|(_, _, card)| card.as_str())
                .collect::<Vec<_>>(),
            vec!["BT17-020", "BT21-091"]
        );
        let action = choices[pick].0;
        let mask = digimon_engine::action::build_action_mask(&r.game, 0);
        assert_eq!(mask[action as usize], 1.0);
        assert_eq!(mask[PASS as usize], 1.0);
        assert!(digimon_engine::action::build_action_mask(&r.game, 1)
            .iter()
            .all(|value| *value == 0.0));
        r.execute_action(0, action).unwrap();
        r.auto_resolve().unwrap();
        assert_eq!(r.hand_size(0), 1);
        assert_eq!(
            r.game.players[0].hand[0].card_id(&r.game.card_data),
            choices[pick].2,
            "the explicit Hybrid or Ten Warriors pick must be the card added"
        );
        assert_eq!(r.security_count(0), 5);
        assert_eq!(r.deck_size(0), 14);
    }
}
#[test]
fn bt18_037_decline_does_not_recover() {
    let mut r = runner(&["BT17-020", "BT13-012"], 0);
    search(&mut r);
    r.execute_action(0, PASS).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.hand_size(0), 0);
    assert_eq!(r.security_count(0), 2);
    assert_eq!(r.deck_size(0), 15);
}
#[test]
fn bt18_037_no_matching_card_does_not_recover() {
    let mut r = runner(&["BT13-012", "BT18-093", "BT4-105"], 0);
    search(&mut r);
    r.auto_resolve().unwrap();
    assert_eq!(r.hand_size(0), 0);
    assert_eq!(r.security_count(0), 3);
    assert_eq!(r.deck_size(0), 15);
}
#[test]
fn bt18_037_inherited_seven_eight_boundary_and_repeat() {
    for hand in [6, 7, 8] {
        let mut r = runner(&[], hand);
        let host = r.place_stack(0, &["BT18-037", "BT13-012"]);
        for _ in 0..2 {
            let before = r.hand_size(0);
            r.game
                .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(host));
            r.game.drain_effect_queue();
            r.auto_resolve().unwrap();
            assert_eq!(r.hand_size(0), before + usize::from(before <= 7));
        }
    }
}

#[test]
fn bt18_037_decline_still_shuffles_security_exactly_once() {
    let mut r = runner(
        &["BT17-020", "BT21-091", "BT13-012", "BT18-093", "BT4-105"],
        0,
    );
    let mut expected = r.game.clone();
    expected.shuffle_security(0);
    search(&mut r);
    r.execute_action(0, PASS).unwrap();
    r.auto_resolve().unwrap();
    let actual: Vec<_> = r.game.players[0]
        .security
        .iter()
        .map(|c| c.handle())
        .collect();
    let expected_cards: Vec<_> = expected.players[0]
        .security
        .iter()
        .map(|c| c.handle())
        .collect();
    assert_eq!(
        actual, expected_cards,
        "declining still runs the unconditional shuffle tail"
    );
    use rand::RngCore;
    assert_eq!(
        r.game.rng.next_u64(),
        expected.rng.next_u64(),
        "one shuffle consumes exactly the expected random stream"
    );
}

#[test]
fn bt18_037_real_repeated_attacks_draw_twice_without_opt() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT18-037")
        .unwrap()
        .dsl_card("BT13-012")
        .unwrap()
        .dsl_card("BT7-032")
        .unwrap()
        .hand(0, &["BT13-012"; 6])
        .deck(0, &["BT13-012"; 15])
        .security(1, &["BT7-032"; 5])
        .memory(10)
        .start();
    r.game.turn_count = 3;
    let host = r.place_stack(0, &["BT18-037", "BT13-012"]);
    for expected_hand in [7, 8] {
        r.game.players[0].battle_area[host.index as usize].is_suspended = false;
        let _ = r.attack_player(host, 1, false);
        r.auto_resolve().unwrap();
        assert_eq!(r.hand_size(0), expected_hand);
        assert!(!r.game_over());
    }
}

#[test]
fn bt18_037_native_circle_masks_and_kendo_zero_cost() {
    use digimon_engine::action::{mask::build_action_mask, space::encode_digivolve};
    use digimon_engine::enums::GamePhase;
    for (base, legal, cost) in [
        ("BT17-020", true, 3),
        ("BT7-032", true, 3),
        ("BT18-094", true, 3),
        ("BT1-086", true, 3),
        ("BT18-093", false, 0),
        ("BT13-012", false, 0),
        ("BT17-023", true, 0),
    ] {
        let builder = DebugRunner::builder().dsl_card("BT18-037").unwrap();
        let builder = if base == "BT1-086" {
            let cards = digimon_engine::card_data::CardData::load_from_file(
                &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/cards.json"),
            )
            .unwrap();
            builder.add_card(cards[base].clone())
        } else {
            builder.dsl_card(base).unwrap()
        };
        let mut r = builder
            .hand(0, &["BT18-037"])
            .deck(0, &["BT18-037"; 10])
            .memory(10)
            .start();
        r.place_on_field(0, base, Some(0));
        r.set_phase(GamePhase::Main);
        let action = encode_digivolve(0, 0);
        assert_eq!(
            build_action_mask(&r.game, 0)[action as usize] > 0.5,
            legal,
            "base {base}"
        );
        if legal {
            r.game.decode_action(action, 0);
            r.auto_resolve().unwrap();
            assert_eq!(r.memory(), 10 - cost, "base {base}");
            assert_eq!(
                r.game.players[0].battle_area[0]
                    .top_card()
                    .card_id(&r.game.card_data),
                "BT18-037"
            );
        }
    }
}

#[test]
fn bt18_037_named_koji_route_and_rule_alias_cost_two() {
    use digimon_engine::action::{build_action_mask, space::encode_digivolve};
    use digimon_engine::card_data::CardData;
    use digimon_engine::enums::GamePhase;
    // Companion metadata only: printed named route is independently established
    // by Lobomon's official observation and image, not inferred from this file.
    let cards = CardData::load_from_file(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/cards.json"),
    )
    .unwrap();
    for base in ["BT7-087", "BT18-088"] {
        let mut metadata = cards.get(base).expect("real Koji printing").clone();
        if base == "BT18-088" {
            // Printed Rule name is verified in the frozen deck-source evidence.
            metadata.also_treated_as = vec!["Takuya Kanbara".into(), "Koji Minamoto".into()];
        }
        let mut r = DebugRunner::builder()
            .dsl_card("BT18-037")
            .unwrap()
            .add_card(metadata)
            .hand(0, &["BT18-037"])
            .deck(0, &["BT18-037"; 10])
            .memory(10)
            .start();
        r.place_on_field(0, base, Some(0));
        r.set_phase(GamePhase::Main);
        let action = encode_digivolve(0, 0);
        assert_eq!(
            build_action_mask(&r.game, 0)[action as usize],
            1.0,
            "{base}"
        );
        r.game.decode_action(action, 0);
        // If ordinary Tamer circle also matches, explicitly choose named cost 2.
        if let Some(p) = r.game.pending_selection.as_ref() {
            let pick = p
                .effect_choices
                .as_ref()
                .expect("digivolution routes")
                .iter()
                .find(|entry| entry.label == "Digivolve for cost 2")
                .expect("named Koji cost 2 route")
                .action_id;
            r.execute_action(0, pick).unwrap();
        }
        r.auto_resolve().unwrap();
        assert_eq!(r.memory(), 8, "named/Rule name Koji route: {base}");
    }
}

fn non_tamer_named_koji_route(rule_alias: bool) {
    use digimon_engine::action::{build_action_mask, space::encode_digivolve};
    use digimon_engine::debug_runner::make_test_card;
    use digimon_engine::enums::{CardColor, CardKind, GamePhase};
    // Synthetic route subject, not an implementation claim about another card.
    // Black excludes both ordinary printed blue/yellow circles. The printed
    // named route says [Koji Minamoto], with no Tamer-kind requirement.
    let mut base = make_test_card(
        "TEST-NON-TAMER-KOJI",
        if rule_alias {
            "Distinct base"
        } else {
            "Koji Minamoto"
        },
    );
    base.card_kind = CardKind::Digimon;
    base.level = Some(3);
    base.colors = vec![CardColor::Black];
    if rule_alias {
        base.also_treated_as = vec!["Koji Minamoto".into()];
    }
    let mut r = DebugRunner::builder()
        .dsl_card("BT18-037")
        .unwrap()
        .add_card(base)
        .hand(0, &["BT18-037"])
        .deck(0, &["BT18-037"; 10])
        .memory(10)
        .start();
    r.place_on_field(0, "TEST-NON-TAMER-KOJI", Some(0));
    r.set_phase(GamePhase::Main);
    let action = encode_digivolve(0, 0);
    assert_eq!(
        build_action_mask(&r.game, 0)[action as usize],
        1.0,
        "the named Koji route must not add a Tamer-kind restriction"
    );
    r.game.decode_action(action, 0);
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 8, "the named route pays exactly 2");
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "BT18-037"
    );
}

#[test]
fn bt18_037_named_koji_route_accepts_a_non_tamer_named_subject() {
    non_tamer_named_koji_route(false);
}

#[test]
fn bt18_037_named_koji_route_accepts_a_non_tamer_rule_alias() {
    non_tamer_named_koji_route(true);
}

#[test]
fn bt18_037_no_match_still_shuffles_security() {
    let mut r = runner(&["BT13-012", "BT18-093", "BT4-105"], 0);
    let mut expected = r.game.clone();
    expected.shuffle_security(0);
    search(&mut r);
    r.auto_resolve().unwrap();
    assert_eq!(
        r.game.players[0]
            .security
            .iter()
            .map(|c| c.handle())
            .collect::<Vec<_>>(),
        expected.players[0]
            .security
            .iter()
            .map(|c| c.handle())
            .collect::<Vec<_>>()
    );
    use rand::RngCore;
    assert_eq!(r.game.rng.next_u64(), expected.rng.next_u64());
    assert_eq!(r.deck_size(0), 15);
}

#[test]
fn bt18_037_sparse_deep_security_choices_preserve_physical_identity_and_exact_shuffle() {
    use rand::RngCore;
    for index in [9, 10, 32] {
        let mut security = vec!["BT13-012"; index];
        security.push("BT17-020");
        let mut r = runner(&security, 0);
        let selected = r.game.players[0].security[index].handle();
        let mut expected = r.game.clone();
        expected.players[0].security.remove(index);
        let recovered = expected.players[0].deck.pop().unwrap();
        expected.players[0].put_on_security_top(recovered);
        expected.shuffle_security(0);
        search(&mut r);
        for _ in 0..index / digimon_engine::card_choice_page::CARD_CHOICES_PER_PAGE {
            r.execute_action(0, digimon_engine::card_choice_page::CARD_CHOICE_NEXT).unwrap();
        }
        let choices = commits(&r);
        assert_eq!(choices.len(), 1, "only the deep Hybrid is eligible");
        assert_eq!(choices[0].1, index);
        assert_eq!(choices[0].2, "BT17-020");
        r.execute_action(0, choices[0].0).unwrap();
        r.auto_resolve().unwrap();
        assert_eq!(r.game.players[0].hand[0].handle(), selected);
        assert_eq!(r.security_count(0), index + 1);
        assert_eq!(r.deck_size(0), 14);
        assert_eq!(
            security_handles(&r),
            expected.players[0]
                .security
                .iter()
                .map(|c| c.handle())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            r.game.rng.next_u64(),
            expected.rng.next_u64(),
            "one recovery followed by exactly one shuffle"
        );
    }
}

#[test]
fn bt18_037_paging_only_changes_presentation_until_an_actual_card_is_committed() {
    use digimon_engine::card_choice_page::{CARD_CHOICE_NEXT, CARD_CHOICE_PREVIOUS};
    use rand::RngCore;
    let mut r = runner(&["BT17-020"; 33], 0);
    let selected = r.game.players[0].security[32].handle();
    let original = security_handles(&r);
    let mut expected = r.game.clone();
    expected.players[0].security.remove(32);
    let recovered = expected.players[0].deck.pop().unwrap();
    expected.players[0].put_on_security_top(recovered);
    expected.shuffle_security(0);
    search(&mut r);
    assert_eq!(commits(&r).len(), 28);
    let before_rng = r.game.rng.clone().next_u64();
    for action in [CARD_CHOICE_NEXT, CARD_CHOICE_PREVIOUS, CARD_CHOICE_NEXT] {
        assert_eq!(
            digimon_engine::action::build_action_mask(&r.game, 0)[action as usize],
            1.0
        );
        r.execute_action(0, action).unwrap();
        assert_eq!(security_handles(&r), original);
        assert_eq!(r.hand_size(0), 0);
        assert_eq!(r.deck_size(0), 15);
        assert_eq!(r.game.rng.clone().next_u64(), before_rng);
    }
    let choices = commits(&r);
    assert_eq!(
        choices
            .iter()
            .map(|(_, index, _)| *index)
            .collect::<Vec<_>>(),
        vec![28, 29, 30, 31, 32]
    );
    let action = choices.iter().find(|(_, index, _)| *index == 32).unwrap().0;
    r.execute_action(0, action).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[0].hand[0].handle(), selected);
    assert_eq!(
        security_handles(&r),
        expected.players[0]
            .security
            .iter()
            .map(|c| c.handle())
            .collect::<Vec<_>>()
    );
    assert_eq!(r.game.rng.next_u64(), expected.rng.next_u64());
    assert_eq!(r.deck_size(0), 14);
}

#[test]
fn bt18_037_cloned_second_page_accept_and_decline_preserve_recovery_and_shuffle() {
    use digimon_engine::card_choice_page::CARD_CHOICE_NEXT;
    use rand::RngCore;
    let mut r = runner(&["BT17-020"; 33], 0);
    search(&mut r);
    r.execute_action(0, CARD_CHOICE_NEXT).unwrap();
    let choices = commits(&r);
    let action = choices.iter().find(|(_, index, _)| *index == 32).unwrap().0;
    assert!(
        r.game.pending_selection_resume.is_some(),
        "search continuation must be cloneable data"
    );
    let saved = r.game.clone();
    for decline in [false, true] {
        r.game = saved.clone();
        let selected = r.game.players[0].security[32].handle();
        let mut expected = saved.clone();
        if !decline {
            expected.players[0].security.remove(32);
            let recovered = expected.players[0].deck.pop().unwrap();
            expected.players[0].put_on_security_top(recovered);
        }
        expected.shuffle_security(0);
        r.execute_action(0, if decline { PASS } else { action })
            .unwrap();
        r.auto_resolve().unwrap();
        assert_eq!(r.hand_size(0), usize::from(!decline));
        if !decline {
            assert_eq!(r.game.players[0].hand[0].handle(), selected);
        }
        assert_eq!(r.deck_size(0), if decline { 15 } else { 14 });
        assert_eq!(r.security_count(0), 33);
        assert_eq!(
            security_handles(&r),
            expected.players[0]
                .security
                .iter()
                .map(|c| c.handle())
                .collect::<Vec<_>>()
        );
        assert_eq!(r.game.rng.next_u64(), expected.rng.next_u64());
    }
}

#[test]
fn bt18_037_inspection_grant_does_not_expose_card_identities_to_the_other_seat() {
    let mut r = runner(&["BT17-020", "BT21-091", "BT13-012"], 0);
    search(&mut r);
    assert_eq!(commits(&r).len(), 2);
    let owner = r
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .card_choice
        .as_ref()
        .unwrap();
    let other = owner.view(&r.game, 1);
    for row in other.rows {
        assert!(!row.identity_visible);
        assert!(row.card_id.is_none());
        assert!(row.card_registry_index.is_none());
    }
}
