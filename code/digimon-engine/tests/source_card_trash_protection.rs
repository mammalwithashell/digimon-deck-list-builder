use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::CardKind;
use digimon_engine::permanent::PermanentHandle;

const PROTECTION: &str = r#"
card: PROTECT
name: X Antibody
kind: option
color: [white]
cost: 0
effects:
  - kind: aura
    scope: inherited
    target: {}
    source_card_trash_protection: { name_is: X Antibody }
"#;
fn plain_option(id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut card = digimon_engine::debug_runner::make_test_card(id, name);
    card.card_kind = CardKind::Option;
    card.level = None;
    card.dp = None;
    card
}
fn fixture(stack: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(PROTECTION)
        .expect("protection DSL")
        .add_card(plain_option("MATCH", "X Antibody"))
        .add_card(plain_option("NEAR", "X Antibody Proto Form"))
        .add_card(plain_option("OPTION", "Plain Option"))
        .add_card(make_test_card_with_level("PLAIN", "Plain", 4))
        .add_card(make_test_card_with_level("TOP", "Top", 6))
        .memory(5)
        .start();
    let h = r.place_stack(0, stack);
    r.game.tick_declarative_effects();
    (r, h)
}
fn ids(r: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    r.game.players[0].battle_area[h.index as usize]
        .card_sources
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_owned())
        .collect()
}
#[test]
fn bottom_two_keeps_protected_bottom_without_substituting_third() {
    let (mut r, h) = fixture(&["PROTECT", "PLAIN", "PLAIN", "TOP"]);
    let top = r.game.players[0].battle_area[0].top_card().handle();
    assert_eq!(
        EffectContext::new(&mut r.game, top, Some(h), 1).trash_bottom_sources(h, 2),
        1
    );
    assert_eq!(ids(&r, h), vec!["PROTECT", "PLAIN", "TOP"]);
}
#[test]
fn both_players_cannot_trash_specific_protected_card_but_can_trash_other_source() {
    for player in [0, 1] {
        let (mut r, h) = fixture(&["PROTECT", "PLAIN", "TOP"]);
        let cards = r.game.players[0].battle_area[0].card_sources.clone();
        let mut ctx = EffectContext::new(&mut r.game, cards[2].handle(), Some(h), player);
        assert!(!ctx.trash_card_source(h, cards[0].handle()));
        assert!(ctx.trash_card_source(h, cards[1].handle()));
    }
}
#[test]
fn cost_uses_successful_count_and_clone_preserves_protection() {
    let (mut r, h) = fixture(&["PROTECT", "PLAIN", "PLAIN", "TOP"]);
    let mut cloned = r.game.clone();
    for game in [&mut r.game, &mut cloned] {
        let top = game.players[0].battle_area[0].top_card().handle();
        let mut ctx = EffectContext::new(game, top, Some(h), 0);
        let paid = ctx.trash_bottom_sources(h, 2) == 2;
        assert!(
            !paid,
            "protected source cannot pay the second unit of a cost"
        );
        assert_eq!(ctx.game.players[0].trash.len(), 1);
    }
}
#[test]
fn top_two_selects_original_positions_with_multiple_protected_cards() {
    let (mut r, h) = fixture(&["PLAIN", "PROTECT", "PROTECT", "TOP"]);
    let top = r.game.players[0].battle_area[0].top_card().handle();
    assert_eq!(
        EffectContext::new(&mut r.game, top, Some(h), 1).trash_top_n_stacked_sources(h, 2, false),
        0
    );
    assert_eq!(ids(&r, h), vec!["PLAIN", "PROTECT", "PROTECT", "TOP"]);
}

fn run_option_cost(r: &mut DebugRunner, h: PermanentHandle) {
    use digimon_dsl::compiled::{CompiledPlayerRef, CompiledStep};
    use digimon_engine::dsl_cards::{bindings::Bindings, step::run_steps};
    let top = r.game.players[0].battle_area[h.index as usize]
        .top_card()
        .handle();
    let mut ctx = EffectContext::new(&mut r.game, top, Some(h), 0);
    run_steps(
        &[
            CompiledStep::TrashOptionFromOwnStacks {
                of: CompiledPlayerRef::You,
                optional: false,
            },
            CompiledStep::GainMemory(3),
        ],
        &mut ctx,
        &mut Bindings::new(),
    );
}
#[test]
fn protected_only_option_cannot_offer_or_pay_activation_cost() {
    let (mut r, h) = fixture(&["PROTECT", "TOP"]);
    let before = r.memory();
    run_option_cost(&mut r, h);
    assert!(
        r.game.pending_selection.is_none(),
        "unpayable protected-only cost must not offer a choice"
    );
    assert_eq!(
        r.memory(),
        before,
        "unpaid activation must not grant its tail"
    );
    assert_eq!(ids(&r, h), vec!["PROTECT", "TOP"]);
}
#[test]
fn whole_stack_departure_is_valid() {
    let (mut r, h) = fixture(&["PROTECT", "PLAIN", "TOP"]);
    // The ordinary whole-object departure is not an effect that trashes a source.
    r.game.return_to_hand(h);
    assert_eq!(r.game.players[0].trash.len(), 2);
}
#[test]
fn fragment_does_not_offer_unpayable_protected_sources() {
    use digimon_engine::enums::Keyword;
    let mut frag = make_test_card_with_level("FRAG", "Fragment", 6);
    frag.keywords.push(Keyword::Fragment(2));
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(PROTECTION)
        .unwrap()
        .add_card(frag)
        .add_card(make_test_card_with_level("PLAIN", "Plain", 4))
        .start();
    let h = r.place_stack(0, &["PROTECT", "PLAIN", "FRAG"]);
    r.game.tick_declarative_effects();
    r.game.delete_permanent_with_effects(h);
    assert!(
        r.game.pending_selection.is_none(),
        "only one trashable source cannot pay Fragment(2)"
    );
    assert!(r.game.players[0].battle_area.is_empty());
}

#[test]
fn name_filter_does_not_protect_near_name_or_other_host() {
    let (mut r, h) = fixture(&["PROTECT", "NEAR", "TOP"]);
    let other = r.place_stack(0, &["NEAR", "TOP"]);
    r.game.tick_declarative_effects();
    let top = r.game.players[0].battle_area[0].top_card().handle();
    let mut ctx = EffectContext::new(&mut r.game, top, Some(h), 1);
    assert_eq!(ctx.trash_bottom_sources(h, 2), 1);
    assert_eq!(ctx.trash_bottom_sources(other, 1), 1);
}
#[test]
fn option_cost_selection_excludes_protected_card_and_clone_pays_real_option() {
    let (mut r, h) = fixture(&["PROTECT", "OPTION", "TOP"]);
    let before = r.memory();
    run_option_cost(&mut r, h);
    let first = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    r.game.resolve_selection(0, first).unwrap();
    let pending = r.game.pending_selection.as_ref().unwrap();
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "only unprotected Option can pay"
    );
    let pick = pending.valid_action_ids[0];
    let mut cloned = r.game.clone();
    for game in [&mut r.game, &mut cloned] {
        game.resolve_selection(0, pick).unwrap();
        assert_eq!(game.memory, before + 3);
        assert_eq!(game.players[0].trash.len(), 1);
        assert_eq!(game.players[0].trash[0].card_id(&game.card_data), "OPTION");
    }
}

#[test]
fn removing_grantor_removes_protection_from_other_matching_source() {
    let (mut r, h) = fixture(&["PROTECT", "MATCH", "TOP"]);
    let cards = r.game.players[0].battle_area[0].card_sources.clone();
    {
        let mut ctx = EffectContext::new(&mut r.game, cards[2].handle(), Some(h), 0);
        assert!(!ctx.trash_card_source(h, cards[1].handle()));
        assert!(ctx.return_card_source_to_hand(h, cards[0].handle()));
    }
    r.game.tick_declarative_effects();
    assert!(
        EffectContext::new(&mut r.game, cards[2].handle(), Some(h), 0)
            .trash_card_source(h, cards[1].handle())
    );
}
#[test]
fn fragment_mask_excludes_protected_and_requires_exact_payment_on_clone() {
    use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
    use digimon_engine::enums::Keyword;
    let mut frag = make_test_card_with_level("FRAG", "Fragment", 6);
    frag.keywords.push(Keyword::Fragment(2));
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(PROTECTION)
        .unwrap()
        .add_card(frag)
        .add_card(make_test_card_with_level("PLAIN", "Plain", 4))
        .start();
    let h = r.place_stack(0, &["PROTECT", "PLAIN", "PLAIN", "FRAG"]);
    r.game.tick_declarative_effects();
    r.game.delete_permanent_with_effects(h);
    r.game.resolve_selection(0, REPLACEMENT_ACCEPT).unwrap();
    assert_eq!(
        r.game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids
            .len(),
        2
    );
    let mut cloned = r.game.clone();
    for game in [&mut r.game, &mut cloned] {
        let first = game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        game.resolve_selection(0, first).unwrap();
        assert!(!game.pending_selection.as_ref().unwrap().is_optional);
        assert!(!game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids
            .contains(&PASS));
        let second = game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        game.resolve_selection(0, second).unwrap();
        assert_eq!(game.players[0].battle_area.len(), 1);
        assert_eq!(game.players[0].battle_area[0].card_sources.len(), 2);
        assert_eq!(game.players[0].trash.len(), 2);
    }
}

fn digi_burst_fixture() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(PROTECTION)
        .unwrap()
        .from_dsl_yaml(include_str!("../cards/st4/ST4-13.yaml"))
        .unwrap()
        .add_card(make_test_card_with_level("PLAIN", "Plain", 4))
        .memory(5)
        .start();
    r.place_stack(0, &["PROTECT", "PLAIN", "ST4-13"]);
    r.place_on_field(1, "PLAIN", Some(0));
    r.game.enter_main_phase();
    r.game.tick_declarative_effects();
    r
}

#[test]
fn digi_burst_mask_rejects_protected_source_as_second_cost_card() {
    use digimon_engine::action::{
        build_action_mask, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START,
    };
    let r = digi_burst_fixture();
    let action = (FIELD_EFFECT_START + FIELD_EFFECT_SLOT_FOR_MAIN) as usize;
    assert_eq!(
        build_action_mask(&r.game, 0)[action],
        0.0,
        "ST4-13 cannot activate Digi-Burst 2 with only one trashable source"
    );
}

#[test]
fn digi_burst_cannot_suspend_after_only_one_source_was_paid() {
    use digimon_engine::action::{
        build_action_mask, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START,
    };
    let mut r = digi_burst_fixture();
    let action = (FIELD_EFFECT_START + FIELD_EFFECT_SLOT_FOR_MAIN) as usize;
    // Follow only actions the current engine exposes. A corrected preflight
    // may reject the activation entirely; otherwise no unpaid benefit is legal.
    if build_action_mask(&r.game, 0)[action] > 0.0 {
        assert!(r.game.activate_field_main(0, 0));
        for _ in 0..6 {
            let Some(pending) = r.game.pending_selection.as_ref() else {
                break;
            };
            let player = pending.selecting_player;
            let action = pending.valid_action_ids[0];
            r.game.resolve_selection(player, action).unwrap();
        }
    }
    assert!(!r.game.players[1].battle_area[0].is_suspended,
        "Digi-Burst must not suspend an opponent after the protected second cost card refused to trash");
}
#[test]
fn explicit_source_cost_requires_actual_payment_but_ordinary_effect_continues() {
    use digimon_dsl::compiled::CompiledStep;
    use digimon_engine::dsl_cards::{bindings::Bindings, step::run_steps};
    use digimon_engine::selection::SourceSelectionRef;
    for cost in [false, true] {
        let (mut r, h) = fixture(&["PROTECT", "PLAIN", "TOP"]);
        let sources = r.game.players[0].battle_area[0].card_sources.clone();
        let refs = sources[..2]
            .iter()
            .enumerate()
            .map(|(i, c)| SourceSelectionRef {
                permanent: h,
                field_index: 0,
                source_index: i as u8,
                card: c.handle(),
            })
            .collect();
        let mut bindings = Bindings::new();
        bindings.insert_source_refs("chosen", refs);
        let before = r.memory();
        let trash = if cost {
            CompiledStep::TrashSelectedSourcesAsCost {
                source_refs: "chosen".into(),
                min_count: None,
            }
        } else {
            CompiledStep::TrashSelectedSources {
                source_refs: "chosen".into(),
            }
        };
        let mut ctx = EffectContext::new(&mut r.game, sources[2].handle(), Some(h), 0);
        run_steps(
            &[trash, CompiledStep::GainMemory(3)],
            &mut ctx,
            &mut bindings,
        );
        assert_eq!(ctx.game.players[0].trash.len(), 1);
        assert_eq!(ctx.game.memory, before + if cost { 0 } else { 3 });
    }
}

#[test]
fn digi_burst_excludes_protection_and_clone_completes_paid_effect() {
    use digimon_engine::action::{
        build_action_mask, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START,
    };
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(PROTECTION)
        .unwrap()
        .from_dsl_yaml(include_str!("../cards/st4/ST4-13.yaml"))
        .unwrap()
        .add_card(make_test_card_with_level("PLAIN", "Plain", 4))
        .memory(5)
        .start();
    r.place_stack(0, &["PROTECT", "PLAIN", "PLAIN", "ST4-13"]);
    r.place_on_field(1, "PLAIN", Some(0));
    r.game.enter_main_phase();
    r.game.tick_declarative_effects();
    let action = (FIELD_EFFECT_START + FIELD_EFFECT_SLOT_FOR_MAIN) as usize;
    assert_eq!(build_action_mask(&r.game, 0)[action], 1.0);
    assert!(r.game.activate_field_main(0, 0));
    assert_eq!(
        r.game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids
            .len(),
        2
    );
    let mut cloned = r.game.clone();
    for game in [&mut r.game, &mut cloned] {
        for _ in 0..4 {
            let Some(pending) = game.pending_selection.as_ref() else {
                break;
            };
            let player = pending.selecting_player;
            let action = pending.valid_action_ids[0];
            game.resolve_selection(player, action).unwrap();
        }
        assert!(game.players[1].battle_area[0].is_suspended);
        assert_eq!(game.players[0].trash.len(), 2);
        assert_eq!(
            game.players[0].battle_area[0].card_sources[0].card_id(&game.card_data),
            "PROTECT"
        );
    }
}
#[test]
fn optional_zero_source_payment_grants_zero_counted_benefits_on_clone() {
    use digimon_dsl::compiled::{CompiledBindingRef, CompiledPredicate, CompiledStep};
    use digimon_engine::action::space::PASS;
    use digimon_engine::dsl_cards::{bindings::Bindings, step::run_steps};
    let (mut r, h) = fixture(&["PROTECT", "PLAIN", "TOP"]);
    let top = r.game.players[0].battle_area[0].top_card().handle();
    let before = r.memory();
    let steps = vec![CompiledStep::SelectOwnSources {
        target: Some(CompiledBindingRef::Source),
        filter: CompiledPredicate::default(),
        min: 0,
        max: 3,
        bind_as: Some("paid".into()),
        prompt: "Up to 3 source cost".into(),
        then: vec![
            CompiledStep::TrashSelectedSourcesAsCost {
                source_refs: "paid".into(),
                min_count: Some(0),
            },
            CompiledStep::PerSelected {
                selection: "paid".into(),
                bind_as: "each".into(),
                body: vec![CompiledStep::GainMemory(1)],
            },
        ],
    }];
    run_steps(
        &steps,
        &mut EffectContext::new(&mut r.game, top, Some(h), 0),
        &mut Bindings::new(),
    );
    assert!(r.game.pending_selection.as_ref().unwrap().is_optional);
    let mut cloned = r.game.clone();
    for game in [&mut r.game, &mut cloned] {
        game.resolve_selection(0, PASS).unwrap();
        assert_eq!(game.memory, before);
        assert!(game.players[0].trash.is_empty());
        assert!(game.pending_selection.is_none());
    }
}

#[test]
fn cloned_source_cost_rechecks_protection_added_after_first_pick() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::{ModifierEntry, ModifierPayload};
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(PROTECTION)
        .unwrap()
        .from_dsl_yaml(include_str!("../cards/st4/ST4-13.yaml"))
        .unwrap()
        .add_card(make_test_card_with_level("PLAIN", "Plain", 4))
        .memory(5)
        .start();
    let h = r.place_stack(0, &["PROTECT", "PLAIN", "PLAIN", "ST4-13"]);
    r.place_on_field(1, "PLAIN", Some(0));
    r.game.enter_main_phase();
    r.game.tick_declarative_effects();
    let first_card = r.game.players[0].battle_area[0].card_sources[1].handle();
    assert!(r.game.activate_field_main(0, 0));
    let first = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    r.game.resolve_selection(0, first).unwrap();
    let mut cloned = r.game.clone();
    for game in [&mut r.game, &mut cloned] {
        game.modifiers.add(
            h,
            ModifierEntry::simple(ModifierType::CannotTrash, 0, Expiry::Permanent, 0)
                .with_payload(ModifierPayload::ProtectedSourceCards(vec![first_card])),
        );
        let second = game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        game.resolve_selection(0, second).unwrap();
        assert!(!game.players[1].battle_area[0].is_suspended);
        assert_eq!(
            game.players[0].trash.len(),
            1,
            "only the unprotected selected card paid"
        );
        assert!(
            game.pending_selection.is_none(),
            "no benefit target selection after partial payment"
        );
    }
}
#[test]
fn up_to_source_cost_counts_actual_removals_after_late_protection_on_clone() {
    use digimon_dsl::compiled::{CompiledBindingRef, CompiledPredicate, CompiledStep};
    use digimon_engine::dsl_cards::{bindings::Bindings, step::run_steps};
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::{ModifierEntry, ModifierPayload};
    let (mut r, h) = fixture(&["PROTECT", "PLAIN", "PLAIN", "TOP"]);
    let cards = r.game.players[0].battle_area[0].card_sources.clone();
    let before = r.memory();
    let steps = vec![CompiledStep::SelectOwnSources {
        target: Some(CompiledBindingRef::Source),
        filter: CompiledPredicate::default(),
        min: 0,
        max: 2,
        bind_as: Some("paid".into()),
        prompt: "Up to 2 source cost".into(),
        then: vec![
            CompiledStep::TrashSelectedSourcesAsCost {
                source_refs: "paid".into(),
                min_count: Some(0),
            },
            CompiledStep::PerSelected {
                selection: "paid".into(),
                bind_as: "each".into(),
                body: vec![CompiledStep::GainMemory(1)],
            },
        ],
    }];
    run_steps(
        &steps,
        &mut EffectContext::new(&mut r.game, cards[3].handle(), Some(h), 0),
        &mut Bindings::new(),
    );
    let first = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    r.game.resolve_selection(0, first).unwrap();
    let mut cloned = r.game.clone();
    for game in [&mut r.game, &mut cloned] {
        game.modifiers.add(
            h,
            ModifierEntry::simple(ModifierType::CannotTrash, 0, Expiry::Permanent, 0).with_payload(
                ModifierPayload::ProtectedSourceCards(vec![cards[1].handle()]),
            ),
        );
        let second = game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        game.resolve_selection(0, second).unwrap();
        assert_eq!(game.players[0].trash.len(), 1);
        assert_eq!(
            game.memory,
            before + 1,
            "up-to cost benefit counts only successfully trashed refs, including clone resume"
        );
        assert!(game.pending_selection.is_none());
    }
}
#[test]
fn optional_exact_cost_can_decline_zero_or_one_without_trashing_on_clone() {
    use digimon_dsl::compiled::{CompiledBindingRef, CompiledPredicate, CompiledStep};
    use digimon_engine::action::space::{encode_source_select, PASS};
    use digimon_engine::dsl_cards::{bindings::Bindings, step::run_steps};
    for take_one in [false, true] {
        let (mut r, h) = fixture(&["PROTECT", "PLAIN", "TOP"]);
        let top = r.game.players[0].battle_area[0].top_card().handle();
        let before = r.memory();
        let steps = vec![CompiledStep::SelectOwnSources {
            target: Some(CompiledBindingRef::Source),
            filter: CompiledPredicate::default(),
            min: 0,
            max: 2,
            bind_as: Some("paid".into()),
            prompt: "Optional source cost of 2".into(),
            then: vec![
                CompiledStep::TrashSelectedSourcesAsCost {
                    source_refs: "paid".into(),
                    min_count: Some(2),
                },
                CompiledStep::GainMemory(3),
            ],
        }];
        run_steps(
            &steps,
            &mut EffectContext::new(&mut r.game, top, Some(h), 0),
            &mut Bindings::new(),
        );
        let payable = encode_source_select(h.index as u16, 1).unwrap();
        let protected = encode_source_select(h.index as u16, 0).unwrap();
        let pending = r.game.pending_selection.as_ref().unwrap();
        assert_eq!(
            pending.valid_action_ids,
            vec![payable, PASS],
            "only the unprotected source and explicit decline are legal"
        );
        assert!(!pending.valid_action_ids.contains(&protected));
        let choice = if take_one { payable } else { PASS };
        let mut cloned = r.game.clone();
        for game in [&mut r.game, &mut cloned] {
            assert_eq!(
                game.pending_selection.as_ref().unwrap().valid_action_ids,
                vec![payable, PASS]
            );
            game.resolve_selection(0, choice).unwrap();
            assert_eq!(game.memory, before);
            assert!(
                game.players[0].trash.is_empty(),
                "declining a two-card payment must not trash the lone selection"
            );
            assert!(game.pending_selection.is_none());
        }
    }
}
