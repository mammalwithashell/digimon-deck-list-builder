//! Prove the existing public keyword wrappers protect BOTH actual metadata sites.
//! No extra guard is added to the private Partition helper.
use crate::{
    card_source::CardHandle,
    cards::keyword_effects::keyword_to_auto_effect,
    debug_runner::{make_test_card, DebugRunner},
    effect::{CardEffect, Effect},
    effect_context::{EffectContext, EffectReadContext},
    enums::Keyword,
    replacement::{ReplacementCause, ReplacementContext, ReplacementOutcome, ReplacementSubject},
    trigger_context::TriggerContext,
};
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

struct CapturedSlots {
    calls: Arc<AtomicUsize>,
    inherited: bool,
    captured: Option<CardHandle>,
}
impl CardEffect for CapturedSlots {
    fn effects(&self, _: CardHandle) -> Vec<Effect> {
        vec![]
    }
    fn partition_slots(
        &self,
        inherited: bool,
    ) -> Option<Arc<Vec<digimon_dsl::compiled::CompiledPredicate>>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert_eq!(inherited, self.inherited);
        if self.captured.is_some() {
            None
        } else {
            Some(Arc::new(vec![]))
        }
    }
}

fn verify<T: PartialEq + std::fmt::Debug>(reject: bool, run: impl FnOnce() -> T, expected: T) {
    let result = catch_unwind(AssertUnwindSafe(run));
    if reject {
        let error = result.expect_err("keyword callback must reject before partition metadata");
        let message = error
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| error.downcast_ref::<&str>().copied())
            .unwrap_or("");
        assert!(
            message.contains("closed-list projected context reached unproven callback"),
            "{message}"
        );
    } else {
        assert_eq!(result.unwrap(), expected);
    }
}

#[test]
fn partition_condition_and_body_guard_actual_metadata_delegation() {
    for seat in 0..2 {
        for inherited in [false, true] {
            for mode in 0..3 {
                for cloned in [false, true] {
                    let mut runner = DebugRunner::builder()
                        .add_card(make_test_card("SLOT-A", "Source A"))
                        .add_card(make_test_card("SLOT-B", "Source B"))
                        .add_card(make_test_card("SLOT-TOP", "Carrier"))
                        .memory(5)
                        .start();
                    let carrier =
                        runner.place_field_stack(seat, &["SLOT-A", "SLOT-B", "SLOT-TOP"], false, 0);
                    let mut game = runner.game;
                    let source = game.players[seat as usize].battle_area[carrier.index as usize]
                        .card_sources[if inherited { 0 } else { 2 }]
                    .handle();
                    let calls = Arc::new(AtomicUsize::new(0));
                    game.effect_registry.insert(
                        if inherited { "SLOT-A" } else { "SLOT-TOP" },
                        Arc::new(CapturedSlots {
                            calls: calls.clone(),
                            inherited,
                            captured: Some(CardHandle(42)),
                        }),
                    );
                    let effects = keyword_to_auto_effect(Keyword::Partition, source);
                    assert_eq!(effects.len(), 1);
                    if mode == 1 {
                        game.require_projected_world_read_guard_for_test();
                    }
                    if mode == 2 {
                        let mut member = TriggerContext::default();
                        member.projected_reference_reads_guarded = true;
                        let mut parent = TriggerContext::default();
                        parent.simultaneous_play_members.push(member);
                        game.current_trigger_context = Some(parent);
                    }
                    if cloned {
                        game = game.clone();
                    }
                    let before = calls.load(Ordering::SeqCst);
                    verify(
                        mode != 0,
                        || {
                            effects[0].replacement_condition.as_ref().unwrap()(
                                &EffectReadContext::new(&game, source, Some(carrier), seat),
                                &ReplacementSubject::Permanent(carrier),
                            )
                        },
                        true,
                    );
                    assert_eq!(
                        calls.load(Ordering::SeqCst) - before,
                        usize::from(mode == 0)
                    );
                    verify(
                        mode != 0,
                        || {
                            let mut ctx =
                                EffectContext::new(&mut game, source, Some(carrier), seat);
                            let mut replacement = ReplacementContext {
                                effect: &mut ctx,
                                cause: ReplacementCause::OpponentEffect,
                                subject: ReplacementSubject::Permanent(carrier),
                                original_destination: None,
                                outcome: ReplacementOutcome::None,
                                cancel_if_deleted: None,
                            };
                            effects[0].replacement_process.as_ref().unwrap()(&mut replacement);
                        },
                        (),
                    );
                    assert_eq!(
                        calls.load(Ordering::SeqCst) - before,
                        if mode == 0 { 2 } else { 0 }
                    );
                    assert_eq!(
                        game.pending_selection.is_some(),
                        mode == 0,
                        "ordinary slot-blind Partition must park its real choice"
                    );
                    assert_eq!(
                        game.players[seat as usize].battle_area[carrier.index as usize]
                            .card_sources
                            .len(),
                        3
                    );
                    assert_eq!(game.memory, 5);
                }
            }
        }
    }
}
