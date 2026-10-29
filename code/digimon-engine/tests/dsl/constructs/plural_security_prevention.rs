//! Atomic security-paid plural prevention: CR 4.3 §§3-1-3-3, 15-7-2/4,
//! 15-8-5-2/4. Card-text idiom: BT25-043 / ST23-05 protect matching
//! simultaneous departures with one top-security payment.
use digimon_engine::{
    action::{
        build_action_mask,
        space::{encode_attack, PASS, PLAY_HAND_START, REPLACEMENT_ACCEPT},
    },
    card_source::CardHandle,
    debug_runner::{make_test_card, DebugRunner},
    selection::SelectionKind,
};

const PROTECTION: &str = r#"
card: SECURITY-PAYER
name: Security payer
kind: tamer
color: [yellow]
cost: 3
effects:
  - kind: replacement
    trigger: when_would_leave_battle_area
    optional: true
    once_per_turn: true
    active_when:
      all_of:
        - replacement_subject_is_mine: true
        - kind: digimon
        - trait_has: Guarded
        - replacement_cause: opponent_effect
        - security_count_gte: 1
    process:
      - trash_top_security_and_cancel_replacement:
          of: you
          all_matching:
            all_of:
              - replacement_subject_is_mine: true
              - kind: digimon
              - trait_has: Guarded
              - replacement_cause: opponent_effect
"#;

const DELETE: &str = r#"
card: SECURITY-DELETE
name: Simultaneous deletion
kind: tamer
color: [purple]
cost: 0
effects:
  - when: on_play
    process:
      - delete_all_permanents: { over: { of: any, zone: [battle_area], kind: digimon } }
"#;

const RETURN: &str = r#"
card: SECURITY-RETURN
name: Simultaneous return
kind: tamer
color: [blue]
cost: 0
effects:
  - when: on_play
    process:
      - return_all_to_hand: { over: { of: any, zone: [battle_area], kind: digimon } }
"#;

const LEGACY_PROTECTION: &str = r#"
card: SECURITY-PAYER
name: Security payer
kind: tamer
color: [yellow]
cost: 3
effects:
  - kind: replacement
    trigger: when_would_leave_battle_area
    optional: true
    active_when:
      all_of:
        - replacement_subject_is_mine: true
        - kind: digimon
        - trait_has: Guarded
        - replacement_cause: opponent_effect
    process:
      - trash_top_security_and_cancel_replacement: { of: you }
"#;

struct Fixture {
    runner: DebugRunner,
    protected: Vec<CardHandle>,
    nonmatching: CardHandle,
    opposing: CardHandle,
    security: Vec<CardHandle>,
}

fn fixture(
    protection: &str,
    producer: &str,
    security_count: usize,
    matching_count: usize,
) -> Fixture {
    let mut builder = DebugRunner::builder()
        .from_dsl_yaml(protection)
        .unwrap()
        .from_dsl_yaml(producer)
        .unwrap();
    for (id, guarded) in [("GUARDED", true), ("OTHER", false), ("SECURITY", false)] {
        let mut card = make_test_card(id, id);
        card.level = Some(4);
        card.dp = Some(4000);
        if guarded {
            card.traits = vec!["Guarded".into()];
        }
        builder = builder.add_card(card);
    }
    let producer_id = if producer == DELETE {
        "SECURITY-DELETE"
    } else {
        "SECURITY-RETURN"
    };
    let security_ids = vec!["SECURITY"; security_count];
    let mut runner = builder
        .hand(0, &[producer_id])
        .security(1, &security_ids)
        .deck(0, &["OTHER"; 20])
        .deck(1, &["OTHER"; 20])
        .memory(10)
        .start();
    runner.skip_mulligan();
    runner.game.enter_main_phase();
    runner.place_on_field(1, "SECURITY-PAYER", None);
    let mut protected = Vec::new();
    for _ in 0..matching_count {
        let handle = runner.place_on_field(1, "GUARDED", None);
        protected.push(
            runner.game.players[1].battle_area[handle.index as usize]
                .top_card()
                .handle(),
        );
    }
    let other = runner.place_on_field(1, "OTHER", None);
    let nonmatching = runner.game.players[1].battle_area[other.index as usize]
        .top_card()
        .handle();
    let opponent = runner.place_on_field(0, "GUARDED", None);
    let opposing = runner.game.players[0].battle_area[opponent.index as usize]
        .top_card()
        .handle();
    let security = runner.game.players[1]
        .security
        .iter()
        .map(|card| card.handle())
        .collect();
    Fixture {
        runner,
        protected,
        nonmatching,
        opposing,
        security,
    }
}

fn fire(f: &mut Fixture) {
    assert_eq!(
        build_action_mask(&f.runner.game, 0)[PLAY_HAND_START as usize],
        1.0
    );
    f.runner.game.decode_action(PLAY_HAND_START, 0);
}

fn answer(f: &mut Fixture, action: u16) {
    let selection = f
        .runner
        .game
        .pending_selection
        .as_ref()
        .expect("public replacement decision");
    assert_eq!(selection.kind, SelectionKind::Replacement);
    assert_eq!(selection.selecting_player, 1);
    assert!(selection.is_optional);
    assert_eq!(selection.valid_action_ids, vec![REPLACEMENT_ACCEPT]);
    let mask = build_action_mask(&f.runner.game, 1);
    for (id, legal) in mask.iter().enumerate() {
        assert_eq!(
            *legal,
            if id == PASS as usize || id == REPLACEMENT_ACCEPT as usize {
                1.0
            } else {
                0.0
            },
            "mask {id}"
        );
    }
    assert!(build_action_mask(&f.runner.game, 0)
        .iter()
        .all(|legal| *legal == 0.0));
    f.runner.execute_action(1, action).unwrap();
}

fn remains(f: &Fixture, owner: usize, handle: CardHandle) -> bool {
    f.runner.game.players[owner]
        .battle_area
        .iter()
        .any(|p| p.top_card().handle() == handle)
}

fn in_destination(f: &Fixture, owner: usize, handle: CardHandle, producer: &str) -> bool {
    let player = &f.runner.game.players[owner];
    let cards = if producer == DELETE {
        &player.trash
    } else {
        &player.hand
    };
    cards.iter().any(|card| card.handle() == handle)
}

#[test]
fn one_security_payment_preserves_all_matching_simultaneous_deletions_and_returns() {
    for producer in [DELETE, RETURN] {
        for count in [1, 2] {
            let mut f = fixture(PROTECTION, producer, count, 2);
            fire(&mut f);
            let saved = f.runner.game.clone();
            let mut digest = None;
            for _ in 0..2 {
                f.runner.game = saved.clone();
                answer(&mut f, REPLACEMENT_ACCEPT);
                assert!(
                    f.runner.game.pending_selection.is_none(),
                    "one payment resolves plural prevention without a victim picker"
                );
                assert!(f.protected.iter().all(|handle| remains(&f, 1, *handle)));
                assert!(f
                    .protected
                    .iter()
                    .all(|handle| !in_destination(&f, 1, *handle, producer)));
                assert!(!remains(&f, 1, f.nonmatching));
                assert!(in_destination(&f, 1, f.nonmatching, producer));
                assert!(
                    !remains(&f, 0, f.opposing),
                    "same trait on opponent does not match"
                );
                assert!(in_destination(&f, 0, f.opposing, producer));
                assert_eq!(f.runner.game.players[1].security.len(), count - 1);
                assert!(
                    f.runner.game.players[1]
                        .trash
                        .iter()
                        .any(|card| card.handle() == *f.security.last().unwrap()),
                    "exact top card paid once"
                );
                if count == 2 {
                    assert_eq!(f.runner.game.players[1].security[0].handle(), f.security[0]);
                }
                if let Some(previous) = digest {
                    assert_eq!(previous, f.runner.game.verification_digest());
                } else {
                    digest = Some(f.runner.game.verification_digest());
                }
            }
        }
    }
}

#[test]
fn declining_security_payment_preserves_security_and_prevents_no_departures() {
    for producer in [DELETE, RETURN] {
        let mut f = fixture(PROTECTION, producer, 2, 2);
        fire(&mut f);
        answer(&mut f, PASS);
        assert!(f.runner.game.pending_selection.is_none());
        assert_eq!(f.runner.game.players[1].security.len(), 2);
        assert!(f
            .protected
            .iter()
            .all(|handle| in_destination(&f, 1, *handle, producer)));
        assert!(in_destination(&f, 1, f.nonmatching, producer));
        assert!(in_destination(&f, 0, f.opposing, producer));
    }
}

#[test]
fn no_matching_departure_never_offers_or_pays_plural_prevention() {
    let mut f = fixture(PROTECTION, DELETE, 2, 0);
    fire(&mut f);
    assert!(f.runner.game.pending_selection.is_none());
    assert_eq!(f.runner.game.players[1].security.len(), 2);
    assert!(in_destination(&f, 1, f.nonmatching, DELETE));
    assert!(in_destination(&f, 0, f.opposing, DELETE));
}

#[test]
fn empty_security_cannot_offer_atomic_payment_even_without_authored_security_guard() {
    // The atomic verb itself is a leading cost: CR 4.3 §15-7-4.
    let mut f = fixture(LEGACY_PROTECTION, DELETE, 0, 2);
    fire(&mut f);
    assert!(
        f.runner.game.pending_selection.is_none(),
        "unpayable security cost cannot be offered"
    );
    assert!(f
        .protected
        .iter()
        .all(|handle| in_destination(&f, 1, *handle, DELETE)));
}

#[test]
fn plural_empty_security_cannot_offer_atomic_payment_without_authored_security_guard() {
    let without_guard = PROTECTION.replace("        - security_count_gte: 1\n", "");
    let mut f = fixture(&without_guard, DELETE, 0, 2);
    fire(&mut f);
    assert!(
        f.runner.game.pending_selection.is_none(),
        "unpayable plural security cost cannot be offered"
    );
    assert!(f
        .protected
        .iter()
        .all(|handle| in_destination(&f, 1, *handle, DELETE)));
}

#[test]
fn omitted_plural_filter_keeps_existing_single_subject_atomic_payment() {
    let mut f = fixture(LEGACY_PROTECTION, DELETE, 2, 2);
    fire(&mut f);
    answer(&mut f, REPLACEMENT_ACCEPT);
    let selection = f
        .runner
        .game
        .pending_selection
        .as_ref()
        .expect("singular prevention chooses exactly one impending departure");
    assert_eq!(selection.kind, SelectionKind::AnyField);
    assert_eq!(selection.selecting_player, 1);
    assert!(!selection.is_optional);
    let choices = vec![encode_attack(1, 1), encode_attack(1, 2)];
    assert_eq!(selection.valid_action_ids, choices);
    let mask = build_action_mask(&f.runner.game, 1);
    for (id, legal) in mask.iter().enumerate() {
        assert_eq!(
            *legal,
            if choices.contains(&(id as u16)) {
                1.0
            } else {
                0.0
            },
            "singular subject mask {id}"
        );
    }
    assert!(build_action_mask(&f.runner.game, 0)
        .iter()
        .all(|legal| *legal == 0.0));
    assert_eq!(
        f.runner.game.players[1].security.len(),
        2,
        "choose the singular subject before paying"
    );
    let saved = f.runner.game.clone();
    for (index, action) in choices.into_iter().enumerate() {
        f.runner.game = saved.clone();
        f.runner.execute_action(1, action).unwrap();
        assert!(f.runner.game.pending_selection.is_none());
        assert!(remains(&f, 1, f.protected[index]));
        assert!(in_destination(&f, 1, f.protected[1 - index], DELETE));
        assert!(in_destination(&f, 1, f.nonmatching, DELETE));
        assert_eq!(f.runner.game.players[1].security.len(), 1);
        assert!(f.runner.game.players[1]
            .trash
            .iter()
            .any(|card| card.handle() == f.security[1]));
    }
}

#[test]
fn unpaid_atomic_security_cost_never_cancels_simultaneous_departures() {
    // Force the process path itself without the optional-activation guard:
    // a failed atomic payment must still produce no cancellation outcome.
    let mandatory = PROTECTION
        .replace("    optional: true\n", "    optional: false\n")
        .replace("        - security_count_gte: 1\n", "");
    let mut f = fixture(&mandatory, DELETE, 0, 2);
    fire(&mut f);
    assert!(f.runner.game.pending_selection.is_none());
    assert!(f
        .protected
        .iter()
        .all(|handle| in_destination(&f, 1, *handle, DELETE)));
}

#[test]
fn plural_security_step_roundtrips_in_compiled_pack_and_exports_its_filter() {
    let spec: digimon_dsl::spec::CardSpec = serde_yml::from_str(PROTECTION).unwrap();
    let compiled = digimon_dsl::compile::compile(&spec).unwrap();
    let pack = digimon_dsl::pack::CardPack::new("plural-security", vec![compiled]);
    assert_eq!(
        digimon_dsl::pack::CardPack::from_bytes(&pack.to_bytes().unwrap()).unwrap(),
        pack
    );
    let schema = digimon_dsl::schema::export_json_schema();
    assert!(schema.contains("TrashTopSecurityPreventionArgs"));
    assert!(schema.contains("all_matching"));
}

#[test]
fn plural_security_clauses_reject_disagreeing_subject_filters() {
    let yaml = format!("{PROTECTION}      - trash_top_security_and_cancel_replacement: {{ of: you, all_matching: {{ kind: tamer }} }}\n");
    let spec: digimon_dsl::spec::CardSpec = serde_yml::from_str(&yaml).unwrap();
    let errors = digimon_dsl::compile::compile(&spec).unwrap_err();
    assert!(errors.iter().any(|error| error
        .message
        .contains("one plural prevention subject filter")));
}

#[test]
fn plural_security_filter_binding_is_validated() {
    let yaml = LEGACY_PROTECTION.replace(
        "{ of: you }",
        "{ of: you, all_matching: { binding_exists: missing } }",
    );
    let spec: digimon_dsl::spec::CardSpec = serde_yml::from_str(&yaml).unwrap();
    let registry = digimon_dsl::raw_rust_registry::StubRegistry::empty();
    let result = digimon_dsl::validator::validate(
        &spec,
        &digimon_dsl::validator::ValidationContext {
            raw_rust: &registry,
        },
    );
    assert!(
        result.is_err(),
        "a post-payment subject filter cannot use an undeclared binding"
    );
}

fn local_context_filter_spec(filter: &str, plural: bool) -> digimon_dsl::spec::CardSpec {
    let step = if plural {
        format!("      - trash_top_security_and_cancel_replacement: {{ of: you, all_matching: {{ {filter} }} }}\n")
    } else {
        format!("      - if:\n          condition: {{ {filter} }}\n          then:\n            - gain_memory: 0\n")
    };
    let yaml = LEGACY_PROTECTION.replace(
        "      - trash_top_security_and_cancel_replacement: { of: you }\n",
        &format!("      - select_own_permanent: {{ bind_as: chosen, filter: {{ kind: digimon }}, prompt: Choose a Digimon }}\n{step}"),
    );
    serde_yml::from_str(&yaml).unwrap_or_else(|error| panic!("filter {filter}: {error}"))
}

fn validate_local_context_filter(
    filter: &str,
    plural: bool,
) -> Result<(), Vec<digimon_dsl::errors::ValidationError>> {
    let registry = digimon_dsl::raw_rust_registry::StubRegistry::empty();
    digimon_dsl::validator::validate(
        &local_context_filter_spec(filter, plural),
        &digimon_dsl::validator::ValidationContext {
            raw_rust: &registry,
        },
    )
}

fn assert_plural_filter_rejected(filter: &str) {
    // This is a valid, in-scope ordinary process predicate. Its rejection
    // must specifically identify the narrower post-payment filter context.
    assert!(
        validate_local_context_filter(filter, false).is_ok(),
        "ordinary process filter must remain valid: {filter}"
    );
    let errors = validate_local_context_filter(filter, true)
        .expect_err(&format!("post-payment plural filter must reject: {filter}"));
    assert!(
        errors
            .iter()
            .any(|error| error.path.contains("all_matching")
                && error.message.contains("post-payment plural filter")
                && error.message.contains("local bindings or effect results")),
        "missing context-specific error for {filter}: {errors:?}"
    );
}

#[test]
fn plural_security_rejects_declared_local_binding_dependencies() {
    for filter in [
        "binding_exists: chosen",
        "binding_present: chosen",
        "binding_absent: chosen",
        "binding_count_eq: { binding: chosen, n: 1 }",
        "level_eq_binding: chosen",
        "level_lte_binding: chosen",
        "level_gte_binding: chosen",
        "color_matches_binding: chosen",
        "not_in_binding: chosen",
        "name_not_in_binding: chosen",
        "of_permanent: chosen",
        "binding_owner: { binding: chosen, of: you }",
        "binding_card_kind: { binding: chosen, kind: digimon }",
        "binding_card_color: { binding: chosen, color_is: yellow }",
        "binding_card_name_is: { binding: chosen, name_is: Guarded }",
        "play_cost_eq_binding: { binding: chosen }",
        "can_digivolve_onto: chosen",
        "equals: [chosen, 1]",
        "not_equals: [chosen, 1]",
    ] {
        assert_plural_filter_rejected(filter);
    }
}

#[test]
fn plural_security_rejects_nested_and_formula_binding_dependencies() {
    for filter in [
        "all_of: [{ any_of: [{ not: { binding_exists: chosen } }] }]",
        "none_of: [{ binding_absent: chosen }]",
        "any_permanent: { of: you, binding_exists: chosen }",
        "count_gte: { n: 1, filter: { binding_exists: chosen } }",
        "source_count: { at_least: 1, filter: { binding_exists: chosen } }",
        "has_inherited: { binding_exists: chosen }",
        "dp_lte: { binding_dp: chosen }",
        "play_cost_lte: { binding_play_cost: chosen }",
        "level_gte: { binding_value: chosen }",
        "dp: { op: lte, value: { max: [1000, { binding_value: chosen }] } }",
        "dp_gte: { source_stack_count: { target: chosen } }",
        "dp_gte: { source_stack_dp_sum: { target: chosen } }",
        "dp_gte: { source_stack_count: { target: source, filter: { binding_exists: chosen } } }",
        "dp_lte: { base: 0, delta: 1000, per: { card_count_in_zone: { of: you, zone: battle_area, filter: { binding_exists: chosen } } } }",
    ] {
        assert_plural_filter_rejected(filter);
    }
}

#[test]
fn plural_security_rejects_effect_result_dependencies() {
    for field in [
        "effect_suspended_any_own_digimon",
        "effect_suspended_any_opponent_digimon",
        "effect_returned_any_card",
        "effect_trashed_any_hand_card",
        "effect_deleted_any_own_digimon",
        "effect_deleted_any_opponent_digimon",
        "effect_played_any_digimon",
        "effect_digivolved_any_digimon",
        "effect_added_any_card_to_hand",
        "color_matches_returned_card",
    ] {
        for value in [true, false] {
            assert_plural_filter_rejected(&format!("{field}: {value}"));
        }
    }
    for filter in [
        "returned_card_matching: { kind: digimon }",
        "trashed_hand_card_matching: { kind: digimon }",
        "effect_deleted_opponent_digimon_dp_gte: 1000",
        "all_of: [{ any_of: [{ effect_played_any_digimon: false }] }]",
        "dp_lte: { base: 0, delta: 1000, per: returned_card_color_count }",
        "dp_lte: { base: 0, delta: 1000, per: effect_suspended_count }",
        "dp: { op: gte, value: { max: [1000, { base: 0, delta: 1000, per: effect_suspended_count }] } }",
    ] {
        assert_plural_filter_rejected(filter);
    }
}

#[test]
fn plural_security_permits_subject_source_global_and_literal_predicates() {
    for filter in [
        "all_of: [{ replacement_subject_is_mine: true }, { kind: digimon }, { trait_has: Guarded }, { replacement_cause: opponent_effect }]",
        "all_of: [{ other: true }, { source_is_tamer: true }, { your_turn: true }]",
        "count_gte: { n: 1, filter: { of: you, kind: digimon } }",
        "dp_lte: { source_dp: {} }",
        "dp_lte: { base: 0, delta: 1000, per: { card_count_in_zone: { of: you, zone: battle_area, filter: { kind: digimon } } } }",
        "dp_gte: { source_stack_count: { target: source } }",
        "dp_gte: { source_stack_count: { target: self } }",
        "dp_gte: { source_stack_dp_sum: { target: target } }",
        "can_digivolve_onto: source",
        "equals: [1, 1]",
        "not_equals: [1, 2]",
        "name_is: binding_exists",
        "name_contains: effect_deleted_any_own_digimon",
        "trait_has: returned_card_color_count",
    ] {
        assert!(validate_local_context_filter(filter, true).is_ok(), "independent filter must remain valid: {filter}");
    }
}
