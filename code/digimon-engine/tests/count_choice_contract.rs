use digimon_engine::count_choice::{
    encode_count_u64, CountChoiceOperation as Op, CountChoicePage, CountChoiceValue as V,
    COUNT_NEXT_ACTION, COUNT_PREVIOUS_ACTION,
};

fn choices(n: u64) -> CountChoicePage {
    CountChoicePage::new(
        (7..=n)
            .map(|v| V {
                declared: v,
                payable: v,
            })
            .collect(),
        None,
    )
    .unwrap()
}

#[test]
fn every_declared_count_is_reachable_without_thirty_choice_truncation() {
    for n in [7, 8, 36, 37, 42, 300] {
        let mut page = choices(n);
        let mut actual = Vec::new();
        loop {
            let rows = page.rows();
            assert!(rows.len() <= 30);
            for row in rows {
                if let Op::Commit(v) = row.metadata.operation {
                    actual.push(v.declared);
                    assert_eq!(v.payable, v.declared);
                    assert!((30..=57).contains(&row.action_id));
                }
            }
            if page.resolve(COUNT_NEXT_ACTION).is_none() {
                break;
            }
        }
        assert_eq!(actual, (7..=n).collect::<Vec<_>>());
    }
}

#[test]
fn forty_two_sources_have_exact_masks_on_both_pages() {
    let mut page = choices(42);
    assert_eq!(
        page.rows().iter().map(|r| r.action_id).collect::<Vec<_>>(),
        (30..=57).chain([59]).collect::<Vec<_>>()
    );
    assert_eq!(page.resolve(COUNT_PREVIOUS_ACTION), None);
    assert_eq!(page.resolve(COUNT_NEXT_ACTION), Some(Op::NextPage));
    assert_eq!(
        page.rows().iter().map(|r| r.action_id).collect::<Vec<_>>(),
        (30..=37).chain([58]).collect::<Vec<_>>()
    );
    assert_eq!(
        page.resolve(37),
        Some(Op::Commit(V {
            declared: 42,
            payable: 42
        }))
    );
    assert_eq!(page.resolve(38), None);
    assert_eq!(page.resolve(COUNT_NEXT_ACTION), None);
    assert_eq!(page.resolve(62), None); // PASS is handled by optional pending-selection policy.
}

#[test]
fn navigation_is_reversible_and_clone_pages_are_independent() {
    let mut original = choices(300);
    let before = original.clone();
    assert_eq!(original.resolve(COUNT_NEXT_ACTION), Some(Op::NextPage));
    let mut fork = original.clone();
    assert_eq!(
        original.resolve(COUNT_PREVIOUS_ACTION),
        Some(Op::PreviousPage)
    );
    assert_eq!(original, before);
    assert_eq!(fork.page_index(), 1);
    assert_eq!(fork.resolve(COUNT_NEXT_ACTION), Some(Op::NextPage));
    assert_eq!(original.page_index(), 0);
    assert_eq!(original.values(), fork.values());
}

#[test]
fn protected_card_count_is_distinct_from_declared_physical_prefix() {
    let mut page = CountChoicePage::new(
        vec![V {
            declared: 8,
            payable: 7,
        }],
        None,
    )
    .unwrap();
    assert_eq!(
        page.resolve(30),
        Some(Op::Commit(V {
            declared: 8,
            payable: 7
        }))
    );
    assert_eq!(page.rows().len(), 1); // singleton is still explicitly offered.
    assert_eq!(page.resolve(31), None);
    assert_eq!(page.page_count(), 1);
}

#[test]
fn malformed_numeric_domains_are_not_presented_as_legal_choices() {
    assert!(CountChoicePage::new(vec![], None).is_none());
    assert!(CountChoicePage::new(
        vec![V {
            declared: 7,
            payable: 8
        }],
        None
    )
    .is_none());
    assert!(CountChoicePage::new(
        vec![
            V {
                declared: 8,
                payable: 7
            };
            2
        ],
        None
    )
    .is_none());
    assert!(CountChoicePage::new(
        vec![
            V {
                declared: 8,
                payable: 7
            },
            V {
                declared: 7,
                payable: 7
            }
        ],
        None
    )
    .is_none());
}

#[test]
fn observation_limb_encoding_roundtrips_above_u8_and_f32_integer_limits() {
    for value in [
        0,
        7,
        42,
        300,
        65535,
        65536,
        (1 << 24) + 1,
        (1 << 53) + 1,
        u64::MAX,
    ] {
        let limbs = encode_count_u64(value);
        assert!(limbs.iter().all(|v| *v >= 0.0 && *v <= 1.0));
        let restored = limbs.into_iter().enumerate().fold(0u64, |n, (i, v)| {
            n | (((v * 65535.0).round() as u64) << (i * 16))
        });
        assert_eq!(restored, value);
    }
}

use digimon_dsl::compiled::CompiledStep;
use digimon_engine::action::{build_action_mask, space::PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::{bindings::Bindings, step::StepRuntime};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::GamePhase;
use digimon_engine::permanent::PermanentHandle;

fn parked(n: u64, optional: bool) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("COUNT-SOURCE", "Count source"))
        .deck(0, &["COUNT-SOURCE"; 12])
        .deck(1, &["COUNT-SOURCE"; 12])
        .memory(3)
        .start();
    r.game.current_phase = GamePhase::Main;
    r.place_on_field(0, "COUNT-SOURCE", Some(0));
    let source = r.game.player(0).battle_area[0].top_card().handle();
    let mut ctx = EffectContext::new(
        &mut r.game,
        source,
        Some(PermanentHandle {
            player: 0,
            index: 0,
        }),
        0,
    );
    digimon_engine::count_choice::install_count_choice(
        &mut ctx,
        choices(n),
        optional,
        "Choose a declared count".into(),
        Some("count".into()),
        vec![CompiledStep::GainMemory(2)],
        Bindings::new(),
        StepRuntime::default(),
    );
    r
}
fn resolve(r: &mut DebugRunner, id: u16) {
    assert_eq!(build_action_mask(&r.game, 0)[id as usize], 1.0);
    r.game.resolve_selection(0, id).unwrap();
}

#[test]
fn native_prompt_masks_navigation_and_runs_effect_tail_only_on_commit() {
    let mut r = parked(42, false);
    assert_eq!(build_action_mask(&r.game, 0)[58], 0.0);
    assert_eq!(build_action_mask(&r.game, 0)[59], 1.0);
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 0.0);
    let before = r.game.verification_digest();
    resolve(&mut r, 59);
    assert_eq!(r.game.memory, 3);
    assert_eq!(r.game.current_phase, GamePhase::EffectChoice);
    assert_eq!(build_action_mask(&r.game, 0)[38], 0.0);
    assert_eq!(build_action_mask(&r.game, 0)[59], 0.0);
    assert_ne!(r.game.verification_digest(), before);
    resolve(&mut r, 58);
    assert_eq!(r.game.verification_digest(), before);
    resolve(&mut r, 59);
    resolve(&mut r, 37); // exact declaration42
    assert_eq!(r.game.memory, 5);
    assert!(r.game.pending_selection.is_none());
    assert!(r.game.resolve_selection(0, 37).is_err());
    assert_eq!(r.game.memory, 5);
}

#[test]
fn native_clone_can_navigate_independently_and_finish_same_count() {
    let mut r = parked(300, false);
    resolve(&mut r, 59);
    let mut fork = r.game.clone();
    let before = fork.verification_digest();
    resolve(&mut r, 59);
    assert_eq!(fork.verification_digest(), before);
    assert_eq!(fork.memory, 3);
    fork.resolve_selection(0, 30).unwrap();
    resolve(&mut r, 58);
    resolve(&mut r, 30);
    assert_eq!(r.game.memory, 5);
    assert_eq!(fork.memory, 5);
    assert_eq!(r.game.verification_digest(), fork.verification_digest());
}

#[test]
fn count_decline_after_navigation_keeps_memory_and_does_not_run_tail() {
    let mut r = parked(42, true);
    resolve(&mut r, 59);
    resolve(&mut r, PASS);
    assert_eq!(r.game.memory, 3);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.game.player(0).battle_area.len(), 1);
}

#[test]
fn count_metadata_is_explicit_and_foreign_actor_cannot_navigate() {
    let mut r = parked(300, false);
    let pending = r.game.pending_selection.as_ref().unwrap();
    let rows = pending.effect_choices.as_ref().unwrap();
    assert_eq!(
        rows[0].count_choice.unwrap().operation,
        Op::Commit(V {
            declared: 7,
            payable: 7
        })
    );
    assert_eq!(
        rows.last().unwrap().count_choice.unwrap().operation,
        Op::NextPage
    );
    let before = r.game.verification_digest();
    assert_eq!(build_action_mask(&r.game, 1)[59], 0.0);
    assert!(r.game.resolve_selection(1, 59).is_err());
    assert_eq!(r.game.verification_digest(), before);
}

#[test]
fn hidden_later_page_choices_are_part_of_semantic_state() {
    let a = parked(300, false);
    let b = parked(301, false);
    assert_eq!(
        a.game.pending_selection.as_ref().unwrap().valid_action_ids,
        b.game.pending_selection.as_ref().unwrap().valid_action_ids
    );
    assert_ne!(a.game.verification_digest(), b.game.verification_digest());
}

#[test]
fn every_offered_count_fits_the_dsl_literal_binding_domain() {
    assert!(CountChoicePage::new(
        vec![V {
            declared: i64::MAX as u64 + 1,
            payable: 7
        }],
        None
    )
    .is_none());
    assert!(CountChoicePage::new(
        vec![V {
            declared: u64::MAX,
            payable: 7
        }],
        None
    )
    .is_none());
}

#[test]
fn maximum_dsl_count_commits_its_exact_binding_without_wrapping() {
    let mut r = parked(7, false);
    r.game.pending_selection = None;
    r.game.pending_selection_resume = None;
    r.game.current_phase = GamePhase::Main;
    let source = r.game.player(0).battle_area[0].top_card().handle();
    let mut ctx = EffectContext::new(&mut r.game, source, None, 0);
    let page = CountChoicePage::new(
        vec![V {
            declared: i64::MAX as u64,
            payable: 7,
        }],
        None,
    )
    .unwrap();
    digimon_engine::count_choice::install_count_choice(
        &mut ctx,
        page,
        false,
        "Exact count".into(),
        Some("count".into()),
        vec![CompiledStep::GainMemory(2)],
        Bindings::new(),
        StepRuntime::default(),
    );
    resolve(&mut r, 30);
    assert_eq!(r.game.memory, 5);
    assert_eq!(
        r.game
            .dsl_resolved_tail_bindings
            .as_ref()
            .unwrap()
            .get_literal("count"),
        Some(i64::MAX)
    );
}

#[test]
fn later_page_binds_declared_count_rather_than_action_ordinal() {
    let mut r = parked(300, false);
    for _ in 0..10 {
        resolve(&mut r, 59);
    }
    resolve(&mut r, 43);
    assert_eq!(
        r.game
            .dsl_resolved_tail_bindings
            .as_ref()
            .unwrap()
            .get_literal("count"),
        Some(300)
    );
}

fn count_payload(value: i32) -> digimon_engine::dcgo_recording::SelectionRow {
    serde_json::from_value(
        serde_json::json!({"step":0,"actor":0,"prompt":"SelectCountEffect","count":value}),
    )
    .unwrap()
}
fn protected_count_prompt() -> DebugRunner {
    let mut r = parked(8, false);
    let source = r.game.player(0).battle_area[0].top_card().handle();
    let mut ctx = EffectContext::new(
        &mut r.game,
        source,
        Some(PermanentHandle {
            player: 0,
            index: 0,
        }),
        0,
    );
    let page = CountChoicePage::new(
        vec![
            V {
                declared: 7,
                payable: 7,
            },
            V {
                declared: 8,
                payable: 7,
            },
        ],
        None,
    )
    .unwrap();
    digimon_engine::count_choice::install_count_choice(
        &mut ctx,
        page,
        false,
        "Choose count".into(),
        None,
        vec![],
        Bindings::new(),
        StepRuntime::default(),
    );
    r
}
#[test]
fn replay_count_uses_declared_metadata_not_payable_label_numbers() {
    let r = protected_count_prompt();
    for (value, action) in [(7, 30), (8, 31)] {
        assert_eq!(
            digimon_engine::runners::selection_resolve::resolve_next(
                &r.game,
                &count_payload(value),
                0
            )
            .unwrap(),
            Some(action)
        );
    }
}
#[test]
fn replay_count_does_not_depend_on_display_labels() {
    let mut r = protected_count_prompt();
    for e in r
        .game
        .pending_selection
        .as_mut()
        .unwrap()
        .effect_choices
        .as_mut()
        .unwrap()
    {
        e.label = "A translated count choice".into();
    }
    assert_eq!(
        digimon_engine::runners::selection_resolve::resolve_next(&r.game, &count_payload(8), 0)
            .unwrap(),
        Some(31)
    );
}
#[test]
fn replay_count_rejects_payable_only_values_and_off_page_navigation_labels() {
    let mut r = parked(42, false);
    for e in r
        .game
        .pending_selection
        .as_mut()
        .unwrap()
        .effect_choices
        .as_mut()
        .unwrap()
    {
        e.label = "Declare 42".into();
    }
    assert!(digimon_engine::runners::selection_resolve::resolve_next(
        &r.game,
        &count_payload(42),
        0
    )
    .is_err());
    let mut r = protected_count_prompt();
    let pending = r.game.pending_selection.as_mut().unwrap();
    pending.effect_choices.as_mut().unwrap().remove(0);
    pending.valid_action_ids.retain(|id| *id != 30);
    assert!(digimon_engine::runners::selection_resolve::resolve_next(
        &r.game,
        &count_payload(7),
        0
    )
    .is_err());
}
