use super::*;
use crate::{
    debug_runner::{make_test_card, DebugRunner},
    enums::{Expiry, Keyword, ModifierType},
    modifiers::{ModifierEntry, ModifierSubject, PlayerModifierEntry, UntilConditionFn},
    trigger_context::TriggerContext,
};
use std::{panic::{catch_unwind, AssertUnwindSafe}, sync::atomic::{AtomicUsize, Ordering}};

fn set_context(game: &mut crate::game::Game, marked: bool, nested: bool) {
    let mut context = TriggerContext::default();
    context.projected_reference_reads_guarded = marked;
    if nested {
        let mut parent = TriggerContext::default();
        parent.simultaneous_play_members.push(context);
        context = parent;
    }
    game.current_trigger_context = Some(context);
}

fn expiry_case(store: u8) {
    for player in [0, 1] { for keep in [false, true] { for nested in [false, true] {
        for marked in [false, true] {
            let mut r = DebugRunner::builder().add_card(make_test_card("MG", "Host")).start();
            let host = r.place_on_field(player, "MG", Some(0));
            let subject = if store == 1 { ModifierSubject::Player(player) }
                else { ModifierSubject::Permanent(host) };
            let calls = Arc::new(AtomicUsize::new(0));
            let counter = calls.clone();
            let predicate: UntilConditionFn = Arc::new(move |_, actual| {
                assert_eq!(actual, subject);
                counter.fetch_add(1, Ordering::SeqCst);
                keep
            });
            match store {
                0 => {
                    let mut entry = ModifierEntry::simple(ModifierType::ChangeDp, 1000, Expiry::UntilCondition, player);
                    entry.until_condition = Some(predicate);
                    r.game.modifiers.add(host, entry);
                }
                1 => {
                    let mut entry = PlayerModifierEntry::simple(ModifierType::CannotSuspend, 0, Expiry::UntilCondition, None, player);
                    entry.until_condition = Some(predicate);
                    r.game.modifiers.add_player_modifier(player, entry);
                }
                _ => r.game.modifiers.grant_keyword_with_until_condition(host, Keyword::Blocker, predicate, player),
            }
            set_context(&mut r.game, marked, nested);
            let before = r.game.modifiers.until_condition_candidates();
            assert_eq!(before.len(), 1);
            // No callback exists at this identity: a marked caller still gets None.
            assert_eq!(r.game.modifiers.evaluate_until_condition(subject, u64::MAX, &r.game), None);
            let cycles = r.game.until_condition_reevaluation_cycles();
            let result = catch_unwind(AssertUnwindSafe(|| r.game.reevaluate_until_condition_modifiers()));
            if marked {
                assert!(result.is_err(), "marked expiry predicate must reject");
                assert_eq!(calls.load(Ordering::SeqCst), 0);
                assert_eq!(r.game.modifiers.until_condition_candidates(), before);
                assert_eq!(r.game.until_condition_reevaluation_cycles(), cycles);
            } else {
                result.unwrap();
                assert_eq!(calls.load(Ordering::SeqCst), 1);
                assert_eq!(r.game.modifiers.until_condition_candidates().len(), usize::from(keep));
                assert_eq!(r.game.until_condition_last_cycle_evaluations(), 1);
                assert_eq!(r.game.until_condition_reevaluation_cycles(), cycles + 1);
            }
        }
    } } }
}

#[test] fn permanent_expiry_checks_before_predicate_and_removal() { expiry_case(0); }
#[test] fn player_expiry_checks_before_predicate_and_removal() { expiry_case(1); }
#[test] fn keyword_expiry_checks_before_predicate_and_removal() { expiry_case(2); }

fn passive_case(player_store: bool) {
    for player in [0, 1] { for allow in [false, true] { for nested in [false, true] {
        for marked in [false, true] {
            let mut r = DebugRunner::builder().add_card(make_test_card("MG", "Host")).start();
            let host = r.place_on_field(player, "MG", Some(0));
            let subject = ReplacementSubject::Permanent(host);
            let calls = Arc::new(AtomicUsize::new(0));
            let counter = calls.clone();
            let predicate: ReplacementConditionFn = Arc::new(move |ctx, actual| {
                assert_eq!(*actual, subject);
                assert_eq!(ctx.source_permanent, Some(host));
                assert_eq!(ctx.replacement_cause(), Some(ReplacementCause::OpponentEffect));
                assert_eq!(ctx.replacement_source_controller(), Some(1 - player));
                assert_eq!(ctx.replacement_subject_controller(), Some(player));
                counter.fetch_add(1, Ordering::SeqCst);
                allow
            });
            if player_store {
                let mut entry = PlayerModifierEntry::simple(ModifierType::CannotBeDestroyed, 0, Expiry::Permanent, Some(host), player);
                entry.replacement_condition = Some(predicate);
                r.game.modifiers.add_player_modifier(player, entry);
            } else {
                let mut entry = ModifierEntry::simple(ModifierType::CannotBeDestroyed, 0, Expiry::Permanent, player);
                entry.replacement_condition = Some(predicate);
                r.game.modifiers.add(host, entry);
            }
            set_context(&mut r.game, marked, nested);
            // Irrelevant timing must skip the callback, including for marked contexts.
            assert!(passive_modifier_candidates(&r.game, EffectTiming::WhenWouldBeReturnedToHand,
                subject, ReplacementCause::OpponentEffect, Some(1 - player)).is_empty());
            assert_eq!(calls.load(Ordering::SeqCst), 0);
            let result = catch_unwind(AssertUnwindSafe(|| passive_modifier_candidates(
                &r.game, EffectTiming::WhenWouldBeDeleted, subject,
                ReplacementCause::OpponentEffect, Some(1 - player))));
            if marked {
                assert!(result.is_err(), "marked passive condition must reject");
                assert_eq!(calls.load(Ordering::SeqCst), 0);
            } else {
                assert_eq!(result.unwrap().len(), usize::from(allow));
                assert_eq!(calls.load(Ordering::SeqCst), 1);
            }
        }
    } } }
}

#[test] fn permanent_passive_checks_before_condition() { passive_case(false); }
#[test] fn player_passive_checks_before_condition() { passive_case(true); }
