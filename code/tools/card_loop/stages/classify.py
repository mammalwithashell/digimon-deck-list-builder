"""CLASSIFY (stage `classify_qa`; design D5, D6): an official ruling is
classified before any oracle time is spent, by BOTH families independently on
the same packet (`interactions.packets.independent_pair`), decided by
`packets.agree_classification`.

    probe:<...>                         -> AUTHORING (probes need no classification)
    behavioral from either family       -> ENCODE (examining is not terminating)
    both textual / both not_examinable,
      each citing qa:<Q>                -> TERMINATION_CHECK, the agreement recorded in
                                           data["termination"] (TERMINATION_CHECK finalises
                                           it without another call)
    any other disagreement / no citation -> ESCALATED with both arguments
    a family unavailable (before or during) -> ESCALATED: a terminating call never
                                           degrades to one family
    a call failed                       -> StageDeferred

Differing classes are a `family_disagreement` correction pair even when one
says behavioral (D11: triage / classify, "other family disagrees").
"""
from __future__ import annotations

from .. import corrections as corr
from ..contracts import FAMILIES
from ..driver_contracts import ItemRecord, StageOutcome
from ..interactions import packets
from . import base, prompts

STAGE = "classify_qa"


def single_family(item: ItemRecord, stage: str, missing, attempts=(), arguments=()) -> StageOutcome:
    reason = (f"single family: {stage} is a terminating call and {', '.join(missing)} is unavailable; "
              "it is never decided by one family (D5)")
    return base.outcome("ESCALATED", item=item, reason=reason, attempts=list(attempts),
                        escalation=base.escalation(item, reason, arguments,
                                                   extra_history=[a.attempt_id for a in attempts]))


class ClassifyExecutor:
    state = "CLASSIFY"

    def run(self, ctx, item: ItemRecord) -> StageOutcome:
        iid = base.interaction_of(item)
        if not iid.startswith("qa:"):
            return base.outcome("AUTHORING", item=item, reason="probe interactions need no classification")
        q = iid[3:]
        missing = [f for f in FAMILIES if f not in base.available_families(ctx)]
        if missing:
            return single_family(item, STAGE, missing)

        cards = base.interaction_cards(ctx, iid)
        p = packets.classify_qa_inputs(q, base.card_qa(ctx), base.clauses_for(ctx, cards))
        text = prompts.render(STAGE, task=p.task, inputs_json=base.json_block(p.inputs))
        routed = ctx.config.routes.get(STAGE, FAMILIES[0])
        pair = packets.independent_pair(
            p, attempt_ids={f: ctx.new_attempt_id() for f in FAMILIES},
            worktrees={f: str(ctx.repo) for f in FAMILIES}, prompt_version=prompts.version(STAGE), prompt=text)
        calls = [base.run_packet(ctx, item, pkt, assignment="routed" if pkt.family == routed else "forced")
                 for pkt in pair]

        exhausted = [c.family for c in calls if c.result.status == "quota_exhausted"]
        if exhausted:
            attempts = [c.attempt(ctx, outcome="accepted" if c.ok else None) for c in calls]
            return single_family(item, STAGE, exhausted, attempts)
        if not all(c.ok for c in calls):
            raise base.defer_failed(ctx, item, calls)

        a, b = (c.output for c in calls)
        args = [base.argument(c, call_value=c.output.get("classification"), citation=c.output.get("citation"),
                              reasoning=c.output.get("reasoning", "")) for c in calls]
        corrections = corr.family_disagreement(
            (calls[0].attempt_id, a.get("classification")), (calls[1].attempt_id, b.get("classification")),
            stage=STAGE, item=item.item, ts=ctx.now())
        agreement = packets.agree_classification(q, a, b)
        if agreement.escalate:
            reason = "; ".join(agreement.reasons) or "the families did not agree"
            return base.outcome("ESCALATED", item=item, reason=reason, corrections=corrections,
                                attempts=[c.attempt(ctx, outcome="escalated") for c in calls],
                                escalation=base.escalation(item, reason, args,
                                                           extra_history=[c.attempt_id for c in calls]))
        attempts = [c.attempt(ctx, outcome="accepted") for c in calls]
        examined = list(dict.fromkeys([*(a.get("examined_clauses") or []), *(b.get("examined_clauses") or [])]))
        classification = {"q_id": q, "calls": {c.family: c.output.get("classification") for c in calls},
                          "examined_clauses": examined, "agreed": agreement.agreed}
        if agreement.value == "behavioral":
            return base.outcome("ENCODE", item=item, attempts=attempts, corrections=corrections,
                                data={"classification": classification},
                                reason="behavioral: encode the ruling, then examine it")
        cites = [a.get("citation"), b.get("citation")]
        termination = {"agreed": True, "stage": STAGE, "terminal": agreement.value, "citation": cites[0],
                       "citations": cites, "arguments": args}
        return base.outcome("TERMINATION_CHECK", item=item, attempts=attempts, corrections=corrections,
                            data={"classification": classification, "termination": termination,
                                  "terminal": agreement.value, "citation": cites[0], "citations": cites},
                            reason=f"both families classify {q} {agreement.value}, citing it")
