"""DIVERGED (driver-only) and TRIAGE (stage `triage`; design D4, D5).

DIVERGED -> TRIAGE always (no model): the state exists so the transition
`ORACLE -> DIVERGED` records the finding before anyone judges it.

TRIAGE, first opinion, routed normally:

    ours_wrong                -> FIX (the fix gate is the arbiter; no second opinion)
    dcgo_quirk | unreachable  -> TERMINATION_CHECK, storing this call and the exact
                                 packet so the other family answers the SAME question
    undetermined              -> ESCALATED

The packet carries no attempt id and no earlier answer, so it can be replayed
verbatim to the second family (D5).
"""
from __future__ import annotations

from ..driver_contracts import ItemRecord, StageOutcome
from . import base, prompts

STAGE = "triage"


class DivergedExecutor:
    state = "DIVERGED"

    def run(self, ctx, item: ItemRecord) -> StageOutcome:
        return StageOutcome(next_state="TRIAGE", reason="triage the divergence")


def subject(ctx, item: ItemRecord) -> str:
    if item.kind == "clause":
        clause = base.clause_of(item)
        card = clause.split("#", 1)[0]
        rec = base.clause_record(ctx, clause) or {}
        return f"clause `{clause}` of {base.card_name(ctx, card)} ({card}): {rec.get('text', '')}".strip()
    if item.kind == "interaction":
        iid = base.interaction_of(item)
        cards = base.interaction_cards(ctx, iid)
        return f"interaction `{iid}` on {', '.join(cards) or 'its cards'}"
    return item.item


def evidence_text(item: ItemRecord) -> str:
    ev = item.data.get("prompt_evidence")
    if not ev:
        return ""
    return (f"- Prompt evidence: at scenario step {ev.get('scenario_step')} the scenario expected "
            f"`{ev.get('expected')}`, DCGO asked `{ev.get('dcgo_asked')}`, and our engine asked "
            f"`{ev.get('ours')}` ({ev.get('explanation')}).\n")


def triage_packet(ctx, item: ItemRecord) -> dict:
    """{"prompt", "references", "prompt_version"}: the packet both families get."""
    cards = base.item_cards(ctx, item)
    scripts = [base.dcgo_script(ctx, c) for c in cards]
    paths = list(item.data.get("scenario_paths") or [])
    oracle = item.data.get("oracle") or {}
    prompt = prompts.render(
        STAGE, item_id=item.item, subject=subject(ctx, item), scenario_paths=base.bullet_list(paths),
        oracle_json=base.json_block(oracle), prompt_evidence=evidence_text(item),
        dcgo_scripts=base.bullet_list([s for s in scripts if s], empty="(no DCGO script found)"),
        bundles=base.bullet_list([base.bundle_path(c) for c in cards]))
    refs = [*paths, *(base.bundle_path(c) for c in cards), *scripts, oracle.get("sidecar"),
            "docs/digimon-rules/rules-index.json", "docs/digimon-rules/keyword-semantics.md"]
    return {"prompt": prompt, "references": [r for r in dict.fromkeys(refs) if r],
            "prompt_version": prompts.version(STAGE)}


class TriageExecutor:
    state = "TRIAGE"

    def run(self, ctx, item: ItemRecord) -> StageOutcome:
        packet = triage_packet(ctx, item)
        call, calls = base.routed_call(ctx, item, stage=STAGE, prompt=packet["prompt"],
                                       references=packet["references"])
        if not call.ok:
            raise base.defer_failed(ctx, item, calls)
        result = call.output
        cls = result.get("classification")
        first = base.argument(call, call_value=cls, citation=result.get("citation"),
                              reasoning=result.get("reasoning", ""))
        data = {"triage_packet": packet, "triage_first": first}
        prior = [c.attempt(ctx) for c in calls[:-1]]
        if cls == "ours_wrong":
            return base.outcome("FIX", item=item, data=data,
                                attempts=prior + [call.attempt(ctx, outcome="accepted")],
                                reason=f"{call.family}: ours_wrong ({base.format_citation(result.get('citation')) or 'no citation'})")
        if cls in ("dcgo_quirk", "unreachable"):
            return base.outcome("TERMINATION_CHECK", item=item, data=data,
                                attempts=prior + [call.attempt(ctx, outcome="accepted")],
                                reason=f"{call.family}: {cls}; needs the other family's agreement (D5)")
        reason = f"{call.family} triage is {cls}: {result.get('reasoning', '')[:300]}"
        return base.outcome("ESCALATED", item=item, data=data, reason=reason,
                            attempts=prior + [call.attempt(ctx, outcome="escalated")],
                            escalation=base.escalation(item, reason, [first],
                                                       extra_history=[c.attempt_id for c in calls]))
