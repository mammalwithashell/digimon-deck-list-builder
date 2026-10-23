//! V3-only selected-permanent identity references. The ordinary Debug view of
//! Bindings deliberately preserves the archival projection. Walk each stored
//! binding environment here, retaining its ordered continuation path.
use super::{debug_field, Sha256};
use crate::{
    dsl_cards::bindings::Bindings,
    enums::PlayerId,
    game::Game,
    permanent::PermanentHandle,
    resume::{AfterSelectionHook, OuterContinuation, ResumeFrame as F, ResumeStack},
    scheduled_effects::PendingProcessing,
    selection::QueuedEffect,
};

type PinRows = Vec<(
    String,
    Vec<(String, PlayerId, u64, Option<PermanentHandle>)>,
)>;

fn bindings(game: &Game, path: &str, value: &Bindings, rows: &mut PinRows) {
    let pins: Vec<_> = value
        .selected_identity_pins_for_verification()
        .into_iter()
        .map(|(name, player, identity)| {
            // Identities are allocated lazily in some engine entry paths. The
            // same number can name different live hosts in different branches.
            let live = value.resolve_selected_identity(game, &name);
            (name, player, identity, live)
        })
        .collect();
    if !pins.is_empty() {
        rows.push((path.to_owned(), pins));
    }
}

fn outer(game: &Game, path: &str, tails: &[OuterContinuation], rows: &mut PinRows) {
    for (index, tail) in tails.iter().enumerate() {
        bindings(
            game,
            &format!("{path}/{index}/bindings"),
            &tail.bindings,
            rows,
        );
    }
}

fn queued(game: &Game, path: &str, effect: &QueuedEffect, rows: &mut PinRows) {
    if let Some(PendingProcessing::Scheduled(effect)) = &effect.pending_processing {
        bindings(
            game,
            &format!("{path}/scheduled"),
            &effect.captured_bindings,
            rows,
        );
    }
}

fn frames(game: &Game, path: &str, stack: &ResumeStack, rows: &mut PinRows) {
    for (index, frame) in stack.frames.iter().enumerate() {
        let path = format!("{path}/{index}");
        macro_rules! tails {
            ($state:expr, $kind:ident) => {
                outer(
                    game,
                    &format!("{path}/{}/outer", stringify!($kind)),
                    &$state.outer_conts,
                    rows,
                )
            };
        }
        macro_rules! bound {
            ($state:expr, $kind:ident) => {{
                bindings(
                    game,
                    &format!("{path}/{}/bindings", stringify!($kind)),
                    &$state.bindings,
                    rows,
                );
                tails!($state, $kind);
            }};
        }
        macro_rules! filtered {
            ($state:expr, $kind:ident) => {{
                bindings(
                    game,
                    &format!("{path}/{}/filter_bindings", stringify!($kind)),
                    &$state.filter_bindings,
                    rows,
                );
                bound!($state, $kind);
            }};
        }
        macro_rules! nested {
            ($inner:expr, $outer:expr, $kind:ident) => {{
                frames(
                    game,
                    &format!("{path}/{}/inner", stringify!($kind)),
                    $inner,
                    rows,
                );
                outer(
                    game,
                    &format!("{path}/{}/outer", stringify!($kind)),
                    $outer,
                    rows,
                );
            }};
        }
        match frame {
            F::RunTail {
                bindings: value,
                outer_conts,
                ..
            } => {
                bindings(game, &format!("{path}/RunTail/bindings"), value, rows);
                outer(game, &format!("{path}/RunTail/outer"), outer_conts, rows);
            }
            F::ReplacementBody { inner, outer_conts } => {
                nested!(inner, outer_conts, ReplacementBody)
            }
            F::FieldTopSourceBatch {
                inner, outer_conts, ..
            } => nested!(inner, outer_conts, FieldTopSourceBatch),
            F::DelayCancelAfterSelection {
                inner, outer_conts, ..
            } => nested!(inner, outer_conts, DelayCancelAfterSelection),
            F::DelayHandDigivolveAfterSelection {
                inner, outer_conts, ..
            } => nested!(inner, outer_conts, DelayHandDigivolveAfterSelection),
            F::DelayDnaAfterSelection {
                inner, outer_conts, ..
            } => nested!(inner, outer_conts, DelayDnaAfterSelection),
            F::DelayFieldDnaAfterPayment {
                inner, outer_conts, ..
            } => nested!(inner, outer_conts, DelayFieldDnaAfterPayment),
            F::DelayPlayFromHandAfterSelection {
                inner, outer_conts, ..
            } => nested!(inner, outer_conts, DelayPlayFromHandAfterSelection),
            F::DelayPlayFromUnionAfterSelection {
                inner, outer_conts, ..
            } => nested!(inner, outer_conts, DelayPlayFromUnionAfterSelection),
            F::DelayBodyAfterCost {
                inner, outer_conts, ..
            } => nested!(inner, outer_conts, DelayBodyAfterCost),
            F::MultiPickStep(s) => bound!(s, MultiPickStep),
            F::PermutationStep(s) => bound!(s, PermutationStep),
            F::BudgetStep(s) => filtered!(s, BudgetStep),
            F::SourceMultiStep(s) => filtered!(s, SourceMultiStep),
            F::PerColorDeleteStep(s) => filtered!(s, PerColorDeleteStep),
            F::CountCappedPermanentsStep(s) => bound!(s, CountCappedPermanentsStep),
            F::RevealBucketStep(s) => bound!(s, RevealBucketStep),
            F::UseOptionFromHandStep(s) => bound!(s, UseOptionFromHandStep),
            F::UseOptionFromTrashStep(s) => bound!(s, UseOptionFromTrashStep),
            F::LinkPickStep(s) => bound!(s, LinkPickStep),
            F::ZoneCardPickStep(s) => bound!(s, ZoneCardPickStep),
            F::TrashLinkCardOfDigimonSelection(s) => bound!(s, TrashLinkCardOfDigimonSelection),
            F::TrashOptionFromStackSelection(s) => bound!(s, TrashOptionFromStackSelection),
            F::AppFuseHostSelection(s) => {
                if let Some(receipt) = &s.receipt {
                    bindings(
                        game,
                        &format!("{path}/AppFuseHostSelection/receipt"),
                        &receipt.bindings,
                        rows,
                    );
                }
                tails!(s, AppFuseHostSelection);
            }
            F::AppFuseResultSelection(s) => {
                if let Some(receipt) = &s.receipt {
                    bindings(
                        game,
                        &format!("{path}/AppFuseResultSelection/receipt"),
                        &receipt.bindings,
                        rows,
                    );
                }
                tails!(s, AppFuseResultSelection);
            }
            F::OuterOptionalTrigger(s) => {
                queued(
                    game,
                    &format!("{path}/OuterOptionalTrigger/queued"),
                    &s.queued_effect,
                    rows,
                );
                tails!(s, OuterOptionalTrigger);
            }
            F::DigivolveReducerPrompt(s) => {
                if let Some(dna) = &s.dna_effect {
                    outer(game, &format!("{path}/DigivolveReducerPrompt/dna/outer"), &dna.outer_conts, rows);
                }
                if let Some(effect) = &s.effect {
                    outer(
                        game,
                        &format!("{path}/DigivolveReducerPrompt/effect/outer"),
                        &effect.outer_conts,
                        rows,
                    );
                }
            }
            F::DigivolveReducerSuspend(s) => {
                if let Some(dna) = &s.dna_effect {
                    outer(game, &format!("{path}/DigivolveReducerSuspend/dna/outer"), &dna.outer_conts, rows);
                }
                if let Some(effect) = &s.effect {
                    outer(
                        game,
                        &format!("{path}/DigivolveReducerSuspend/effect/outer"),
                        &effect.outer_conts,
                        rows,
                    );
                }
            }
            F::NonDslCountCappedStep(s) => tails!(s, NonDslCountCappedStep),
            F::EffectDigivolveCostChoice(s) => tails!(s, EffectDigivolveCostChoice),
            F::EffectDnaHandRoute(s) => tails!(s, EffectDnaHandRoute),
            F::RefireEffectChoice(s) => tails!(s, RefireEffectChoice),
            F::OptionModeSelect(s) => tails!(s, OptionModeSelect),
            F::DigiXrosMaterialSelection(s) => tails!(s, DigiXrosMaterialSelection),
            F::TriggerOrderSelection(s) => tails!(s, TriggerOrderSelection),
            F::OptionalReplacement(s) => tails!(s, OptionalReplacement),
            F::DelayHandDigivolveSelection(s) => tails!(s, DelayHandDigivolveSelection),
            F::DelayDnaCardSelection(s) => tails!(s, DelayDnaCardSelection),
            F::DelayFieldDnaSelection(s) => tails!(s, DelayFieldDnaSelection),
            F::DelayPlayFromHandSelection(s) => tails!(s, DelayPlayFromHandSelection),
            F::ArtsDigivolveSelection(s) => tails!(s, ArtsDigivolveSelection),
            F::LinkOptionHostSelection(s) => tails!(s, LinkOptionHostSelection),
            F::LinkCardTrashChoice(s) => tails!(s, LinkCardTrashChoice),
            F::PlayFromHandCostReductionPrompt(s) => tails!(s, PlayFromHandCostReductionPrompt),
            F::InteractiveDigivolveCostReductionPrompt(s) => {
                tails!(s, InteractiveDigivolveCostReductionPrompt)
            }
            F::InteractiveOptionUseCostReductionPrompt(s) => {
                tails!(s, InteractiveOptionUseCostReductionPrompt)
            }
            F::AllianceSelection(s) => tails!(s, AllianceSelection),
            F::BlockSelection(s) => tails!(s, BlockSelection),
            F::RaidSelection(s) => tails!(s, RaidSelection),
            F::CounterSelection(s) => tails!(s, CounterSelection),
            F::CounterBlastDnaFieldMaterial(s) => tails!(s, CounterBlastDnaFieldMaterial),
            F::CounterBlastDnaHandMaterial(s) => tails!(s, CounterBlastDnaHandMaterial),
            F::AttackDeclarationDecision(s) => tails!(s, AttackDeclarationDecision),
            F::KeywordSaveSelection(s) => tails!(s, KeywordSaveSelection),
            F::KeywordMaterialSaveTamerSelection(s) => tails!(s, KeywordMaterialSaveTamerSelection),
            F::KeywordScapegoatSelection(s) => tails!(s, KeywordScapegoatSelection),
            F::KeywordMindLinkSelection(s) => tails!(s, KeywordMindLinkSelection),
            F::KeywordAscensionChoice(s) => tails!(s, KeywordAscensionChoice),
            F::FamiliarTokenOnDeletionSelection(s) => tails!(s, FamiliarTokenOnDeletionSelection),
            F::LinkCardLeaveSelection(s) => tails!(s, LinkCardLeaveSelection),
            F::PlayOrUseDualChoice(s) => tails!(s, PlayOrUseDualChoice),
            F::MayDnaPartnerSelection(s) => tails!(s, MayDnaPartnerSelection),
            F::MayDnaResultSelection(s) => tails!(s, MayDnaResultSelection),
            // These native decisions carry no DSL binding environments.
            F::DigivolveCostChoice(_)
            | F::DnaDigivolveFirstMaterial(_)
            | F::DnaDigivolveSecondMaterial(_)
            | F::OptionHandLinkHostSelection(_)
            | F::DigimonLinkHostSelection(_)
            | F::OverclockSelection(_)
            | F::PlayOrderSelection => {}
        }
    }
}

pub(super) fn write(hasher: &mut Sha256, game: &Game) {
    let mut rows = Vec::new();
    if let Some(stack) = &game.pending_selection_resume {
        frames(game, "pending", stack, &mut rows);
    }
    if let Some((_, value, _)) = &game.dsl_outer_tail {
        bindings(game, "dsl_outer", value, &mut rows);
    }
    if let Some(value) = &game.dsl_resolved_tail_bindings {
        bindings(game, "dsl_resolved", value, &mut rows);
    }
    for (index, parked) in game.parked_replacement.iter().enumerate() {
        outer(
            game,
            &format!("replacement/{index}/outer"),
            &parked.outer_conts,
            &mut rows,
        );
    }
    for (index, hook) in game.after_selection_resume_hooks.0.iter().enumerate() {
        if let AfterSelectionHook::DigivolveReducerChain { state, .. } = hook {
            if let Some(dna) = &state.dna_effect {
                outer(game, &format!("hook/{index}/dna/outer"), &dna.outer_conts, &mut rows);
            }
            if let Some(effect) = &state.effect {
                outer(
                    game,
                    &format!("hook/{index}/effect/outer"),
                    &effect.outer_conts,
                    &mut rows,
                );
            }
        }
    }
    for (index, scope) in game.effect_digivolve_scopes.iter().enumerate() {
        outer(
            game,
            &format!("effect_scope/{index}/outer"),
            &scope.declaration.outer_conts,
            &mut rows,
        );
    }
    for (index, scope) in game.effect_dna_would_scopes.iter().enumerate() {
        outer(game, &format!("dna_would_scope/{index}/outer"), &scope.declaration.outer_conts, &mut rows);
    }
    for (index, effect) in game.scheduled_effects.iter().enumerate() {
        bindings(
            game,
            &format!("scheduled/{index}"),
            &effect.captured_bindings,
            &mut rows,
        );
    }
    for (index, effect) in game.effect_queue.iter().enumerate() {
        queued(game, &format!("queue/{index}"), effect, &mut rows);
    }
    if let Some(pending) = &game.pending_pay_cost_effect {
        queued(game, "pending_pay_cost", &pending.queued_effect, &mut rows);
    }
    for (index, pending) in game.pending_pay_cost_stack.iter().enumerate() {
        queued(
            game,
            &format!("pending_pay_cost_stack/{index}"),
            &pending.queued_effect,
            &mut rows,
        );
    }
    // Empty/default states keep their existing saved-state goldens.
    if !rows.is_empty() {
        debug_field(hasher, "selected_permanent_identity_pins", &rows);
    }
}
