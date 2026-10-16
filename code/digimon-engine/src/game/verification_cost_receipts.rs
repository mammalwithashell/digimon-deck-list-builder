//! Alpha-equivalent outstanding payment tokens share a digest; wrong references do not.
use super::{debug_field, Sha256};
use crate::{
    game::Game,
    resume::{AfterSelectionHook, OuterContinuation, ResumeDecline, ResumeFrame as F, ResumeStack},
};
use digimon_dsl::compiled::CompiledStep;
#[derive(Debug)]
pub(super) enum ReceiptRef {
    None,
    Live(usize),
    Missing(u64),
}
fn reference(g: &Game, id: Option<u64>) -> ReceiptRef {
    match id {
        None => ReceiptRef::None,
        Some(id) => g
            .cost_payment_receipts
            .iter()
            .position(|r| r.id == id)
            .map(ReceiptRef::Live)
            .unwrap_or(ReceiptRef::Missing(id)),
    }
}
pub(super) fn adjustment(
    g: &Game,
    a: &crate::game_actions::PlayCostAdjustments,
) -> (Option<u16>, crate::enums::CostDelta, i32, ReceiptRef) {
    (
        a.assigned_base,
        a.effect_cost,
        a.reduction,
        reference(g, a.pending_payment),
    )
}
fn value_refs(g: &Game, path: &str, v: &serde_json::Value, out: &mut Vec<(String, ReceiptRef)>) {
    match v {
        serde_json::Value::Object(map) => {
            if let Some(id) = map
                .get("CompleteCostPayment")
                .and_then(|v| v.get("receipt"))
                .and_then(|v| v.as_u64())
            {
                out.push((path.into(), reference(g, Some(id))));
                return;
            }
            for (key, v) in map {
                value_refs(g, &format!("{path}/{key}"), v, out);
            }
        }
        serde_json::Value::Array(values) => {
            for (i, v) in values.iter().enumerate() {
                value_refs(g, &format!("{path}/{i}"), v, out);
            }
        }
        _ => {}
    }
}
fn steps(g: &Game, path: &str, steps: &[CompiledStep], out: &mut Vec<(String, ReceiptRef)>) {
    value_refs(
        g,
        path,
        &serde_json::to_value(steps).expect("compiled step serializes"),
        out,
    );
}
fn outer(g: &Game, path: &str, conts: &[OuterContinuation], out: &mut Vec<(String, ReceiptRef)>) {
    for (i, c) in conts.iter().enumerate() {
        steps(g, &format!("{path}/{i}"), &c.tail, out);
    }
}
fn frames(g: &Game, path: &str, stack: &ResumeStack, out: &mut Vec<(String, ReceiptRef)>) {
    for (i, f) in stack.frames.iter().enumerate() {
        let path = format!("{path}/{i}");
        match f {
            F::RunTail {
                inner_tail,
                outer_conts,
                decline,
                ..
            } => {
                steps(g, &format!("{path}/success"), inner_tail, out);
                outer(g, &format!("{path}/outer"), outer_conts, out);
                if let ResumeDecline::RunTail { tail, .. } = decline {
                    steps(g, &format!("{path}/decline"), tail, out);
                }
            }
            F::ReplacementBody { inner, outer_conts }
            | F::FieldTopSourceBatch {
                inner, outer_conts, ..
            }
            | F::DelayCancelAfterSelection {
                inner, outer_conts, ..
            }
            | F::DelayHandDigivolveAfterSelection {
                inner, outer_conts, ..
            }
            | F::DelayDnaAfterSelection {
                inner, outer_conts, ..
            }
            | F::DelayFieldDnaAfterPayment {
                inner, outer_conts, ..
            }
            | F::DelayPlayFromHandAfterSelection {
                inner, outer_conts, ..
            }
            | F::DelayPlayFromUnionAfterSelection {
                inner, outer_conts, ..
            }
            | F::DelayBodyAfterCost {
                inner, outer_conts, ..
            } => {
                frames(g, &format!("{path}/inner"), inner, out);
                outer(g, &format!("{path}/outer"), outer_conts, out);
            }
            F::MultiPickStep(s) => {
                steps(
                    g,
                    &format!("{path}/MultiPickStep/inner_tail"),
                    &s.inner_tail,
                    out,
                );
                outer(
                    g,
                    &format!("{path}/MultiPickStep/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::PermutationStep(s) => {
                steps(
                    g,
                    &format!("{path}/PermutationStep/inner_tail"),
                    &s.inner_tail,
                    out,
                );
                outer(
                    g,
                    &format!("{path}/PermutationStep/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::BudgetStep(s) => {
                steps(
                    g,
                    &format!("{path}/BudgetStep/inner_tail"),
                    &s.inner_tail,
                    out,
                );
                outer(g, &format!("{path}/BudgetStep/outer"), &s.outer_conts, out);
            }
            F::SourceMultiStep(s) => {
                steps(
                    g,
                    &format!("{path}/SourceMultiStep/inner_tail"),
                    &s.inner_tail,
                    out,
                );
                outer(
                    g,
                    &format!("{path}/SourceMultiStep/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::CountCappedPermanentsStep(s) => {
                steps(
                    g,
                    &format!("{path}/CountCappedPermanentsStep/inner_tail"),
                    &s.inner_tail,
                    out,
                );
                outer(
                    g,
                    &format!("{path}/CountCappedPermanentsStep/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::NonDslCountCappedStep(s) => {
                outer(
                    g,
                    &format!("{path}/NonDslCountCappedStep/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::RevealBucketStep(s) => {
                steps(
                    g,
                    &format!("{path}/RevealBucketStep/inner_tail"),
                    &s.inner_tail,
                    out,
                );
                outer(
                    g,
                    &format!("{path}/RevealBucketStep/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::UseOptionFromHandStep(s) => {
                steps(
                    g,
                    &format!("{path}/UseOptionFromHandStep/tail"),
                    &s.tail,
                    out,
                );
                outer(
                    g,
                    &format!("{path}/UseOptionFromHandStep/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::UseOptionFromTrashStep(s) => {
                steps(
                    g,
                    &format!("{path}/UseOptionFromTrashStep/tail"),
                    &s.tail,
                    out,
                );
                outer(
                    g,
                    &format!("{path}/UseOptionFromTrashStep/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::LinkPickStep(s) => {
                steps(g, &format!("{path}/LinkPickStep/tail"), &s.tail, out);
                outer(
                    g,
                    &format!("{path}/LinkPickStep/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::ZoneCardPickStep(s) => {
                steps(
                    g,
                    &format!("{path}/ZoneCardPickStep/inner_tail"),
                    &s.inner_tail,
                    out,
                );
                outer(
                    g,
                    &format!("{path}/ZoneCardPickStep/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::EffectDigivolveCostChoice(s) => {
                outer(
                    g,
                    &format!("{path}/EffectDigivolveCostChoice/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::RefireEffectChoice(s) => {
                outer(
                    g,
                    &format!("{path}/RefireEffectChoice/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::OptionModeSelect(s) => {
                outer(
                    g,
                    &format!("{path}/OptionModeSelect/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::DigiXrosMaterialSelection(s) => {
                outer(
                    g,
                    &format!("{path}/DigiXrosMaterialSelection/outer"),
                    &s.outer_conts,
                    out,
                );
                out.push((
                    format!("{path}/digixros"),
                    reference(g, s.total_reduction.pending_payment),
                ));
            }
            F::OuterOptionalTrigger(s) => {
                outer(
                    g,
                    &format!("{path}/OuterOptionalTrigger/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::TriggerOrderSelection(s) => {
                outer(
                    g,
                    &format!("{path}/TriggerOrderSelection/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::OptionTrashOrder(s) => {
                outer(
                    g,
                    &format!("{path}/OptionTrashOrder/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::OptionalReplacement(s) => {
                outer(
                    g,
                    &format!("{path}/OptionalReplacement/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::DelayDnaCardSelection(s) => {
                outer(
                    g,
                    &format!("{path}/DelayDnaCardSelection/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::DelayFieldDnaSelection(s) => {
                outer(
                    g,
                    &format!("{path}/DelayFieldDnaSelection/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::AppFuseHostSelection(s) => {
                outer(
                    g,
                    &format!("{path}/AppFuseHostSelection/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::AppFuseResultSelection(s) => {
                outer(
                    g,
                    &format!("{path}/AppFuseResultSelection/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::ArtsDigivolveSelection(s) => {
                outer(
                    g,
                    &format!("{path}/ArtsDigivolveSelection/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::LinkOptionHostSelection(s) => {
                outer(
                    g,
                    &format!("{path}/LinkOptionHostSelection/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::PlayFromHandCostReductionPrompt(s) => {
                outer(
                    g,
                    &format!("{path}/PlayFromHandCostReductionPrompt/outer"),
                    &s.outer_conts,
                    out,
                );
                out.push((
                    format!("{path}/play_cost"),
                    reference(g, s.accumulated_reduction.pending_payment),
                ));
            }
            F::AllianceSelection(s) => {
                outer(
                    g,
                    &format!("{path}/AllianceSelection/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::BlockSelection(s) => {
                outer(
                    g,
                    &format!("{path}/BlockSelection/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::RaidSelection(s) => {
                outer(
                    g,
                    &format!("{path}/RaidSelection/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::CounterSelection(s) => {
                outer(
                    g,
                    &format!("{path}/CounterSelection/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::CounterBlastDnaFieldMaterial(s) => {
                outer(
                    g,
                    &format!("{path}/CounterBlastDnaFieldMaterial/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::CounterBlastDnaHandMaterial(s) => {
                outer(
                    g,
                    &format!("{path}/CounterBlastDnaHandMaterial/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::KeywordSaveSelection(s) => {
                outer(
                    g,
                    &format!("{path}/KeywordSaveSelection/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::KeywordMaterialSaveTamerSelection(s) => {
                outer(
                    g,
                    &format!("{path}/KeywordMaterialSaveTamerSelection/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::KeywordScapegoatSelection(s) => {
                outer(
                    g,
                    &format!("{path}/KeywordScapegoatSelection/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::KeywordMindLinkSelection(s) => {
                outer(
                    g,
                    &format!("{path}/KeywordMindLinkSelection/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::KeywordAscensionChoice(s) => {
                outer(
                    g,
                    &format!("{path}/KeywordAscensionChoice/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::FamiliarTokenOnDeletionSelection(s) => {
                outer(
                    g,
                    &format!("{path}/FamiliarTokenOnDeletionSelection/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::LinkCardLeaveSelection(s) => {
                outer(
                    g,
                    &format!("{path}/LinkCardLeaveSelection/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::TrashLinkCardOfDigimonSelection(s) => {
                steps(
                    g,
                    &format!("{path}/TrashLinkCardOfDigimonSelection/tail"),
                    &s.tail,
                    out,
                );
                outer(
                    g,
                    &format!("{path}/TrashLinkCardOfDigimonSelection/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::TrashOptionFromStackSelection(s) => {
                steps(
                    g,
                    &format!("{path}/TrashOptionFromStackSelection/tail"),
                    &s.tail,
                    out,
                );
                outer(
                    g,
                    &format!("{path}/TrashOptionFromStackSelection/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::PlayOrUseDualChoice(s) => {
                outer(
                    g,
                    &format!("{path}/PlayOrUseDualChoice/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::MayDnaPartnerSelection(s) => {
                outer(
                    g,
                    &format!("{path}/MayDnaPartnerSelection/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::MayDnaResultSelection(s) => {
                outer(
                    g,
                    &format!("{path}/MayDnaResultSelection/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::PerColorDeleteStep(s) => {
                steps(
                    g,
                    &format!("{path}/PerColorDeleteStep/inner_tail"),
                    &s.inner_tail,
                    out,
                );
                outer(
                    g,
                    &format!("{path}/PerColorDeleteStep/outer"),
                    &s.outer_conts,
                    out,
                );
            }

            F::DelayHandDigivolveSelection(s) => {
                outer(
                    g,
                    &format!("{path}/DelayHandDigivolveSelection/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::DelayPlayFromHandSelection(s) => {
                outer(
                    g,
                    &format!("{path}/DelayPlayFromHandSelection/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::InteractiveDigivolveCostReductionPrompt(s) => {
                outer(
                    g,
                    &format!("{path}/InteractiveDigivolveCostReductionPrompt/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::InteractiveOptionUseCostReductionPrompt(s) => {
                outer(
                    g,
                    &format!("{path}/InteractiveOptionUseCostReductionPrompt/outer"),
                    &s.outer_conts,
                    out,
                );
            }
            F::DigivolveReducerPrompt(s) | F::DigivolveReducerSuspend(s) => {
                if let Some(effect) = &s.effect {
                    outer(g, &format!("{path}/effect/outer"), &effect.outer_conts, out);
                }
            }
            _ => {}
        }
    }
}
// Full effect-owned declarations remain part of the state hash, but payment
// completion tokens are hashed separately as live-table references below.
fn erase_completion_tokens(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(marker)=map.get_mut("CompleteCostPayment") {
                if let Some(receipt)=marker.get_mut("receipt") { *receipt=serde_json::Value::from(0u64); }
            }
            for child in map.values_mut() { erase_completion_tokens(child); }
        }
        serde_json::Value::Array(values)=>for child in values { erase_completion_tokens(child); },
        _=>{}
    }
}
pub(super) fn effect_declaration(state:&crate::game_actions::digivolve::EffectDigivolveCostChoiceState)->crate::game_actions::digivolve::EffectDigivolveCostChoiceState {
    let mut normalized=state.clone();
    for cont in &mut normalized.outer_conts {
        let mut steps=serde_json::to_value(cont.tail.as_ref()).expect("compiled tail serializes");
        erase_completion_tokens(&mut steps);
        cont.tail=std::sync::Arc::new(serde_json::from_value(steps).expect("normalized compiled tail deserializes"));
    }
    normalized
}
pub(super) fn write(h: &mut Sha256, g: &Game) {
    let mut refs = vec![("active".into(), reference(g, g.active_cost_payment_receipt))];
    if let Some(stack) = &g.pending_selection_resume {
        frames(g, "pending", stack, &mut refs);
    }
    if let Some((tail, _, _)) = &g.dsl_outer_tail {
        steps(g, "dsl_outer", tail, &mut refs);
    }
    for (i, p) in g.parked_replacement.iter().enumerate() {
        outer(g, &format!("replacement/{i}"), &p.outer_conts, &mut refs);
    }
    for (i, hook) in g.after_selection_resume_hooks.0.iter().enumerate() {
        match hook {
            AfterSelectionHook::PlayCostContinuation {
                accumulated_reduction,
                ..
            } => refs.push((
                format!("hook/{i}"),
                reference(g, accumulated_reduction.pending_payment),
            )),
            AfterSelectionHook::DigiXrosLeaveContinuation { cont } => refs.push((
                format!("hook/{i}/digixros"),
                reference(g, cont.pending_cost_payment()),
            )),
            AfterSelectionHook::DigivolveReducerChain { state, .. } => {
                if let Some(effect) = &state.effect {
                    outer(
                        g,
                        &format!("hook/{i}/effect/outer"),
                        &effect.outer_conts,
                        &mut refs,
                    );
                }
            }
            _ => {}
        }
    }
    for (index, scope) in g.effect_digivolve_scopes.iter().enumerate() {
        outer(g, &format!("effect_scope/{index}/outer"), &scope.declaration.outer_conts, &mut refs);
    }
    if g.cost_payment_receipts.is_empty() && refs.iter().all(|(_, r)| matches!(r, ReceiptRef::None))
    {
        return;
    }
    let rows: Vec<_> = g
        .cost_payment_receipts
        .iter()
        .map(|r| {
            (
                r.target,
                &r.key,
                r.assignment,
                r.completed,
                r.record_activation,
                r.source_identity,
            )
        })
        .collect();
    debug_field(h, "cost_payment_receipts", &rows);
    debug_field(h, "cost_payment_references", &refs);
}
