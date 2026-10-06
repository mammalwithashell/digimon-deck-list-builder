"""TERMINATION_CHECK (design D5, spec "Terminating calls need agreement from
both model families").

A call that ends an item without our engine changing needs independent
agreement from the other family on the SAME packet, without sight of the first
answer, and BOTH must cite a source.

* An agreement already reached by two independent calls (CLASSIFY's textual /
  not_examinable, `data["termination"]["agreed"]`) is finalised: TERMINAL.
* Otherwise the first opinion is TRIAGE's (`data["triage_first"]`, packet in
  `data["triage_packet"]`); the other family answers the stored packet:
    same classification, both cited   -> TERMINAL (adjudicated), data["terminal"],
                                          data["citation"]; for a clause ended
                                          `dcgo_quirk` the verdict store is triaged
                                          with `dcgo-harness verdict-triage`
    anything else                      -> ESCALATED with both arguments and a
                                          `family_disagreement` correction pair
    the other family is unavailable    -> ESCALATED (never single-family judgment)

`verdict-triage` accepts only `ours_wrong | dcgo_quirk | undetermined` on a
stored `diverged` CLAUSE row, so an `unreachable` ending and interaction
endings are recorded in `data["verdict_store"]` as not written: the store has
no writer for them yet.
"""
from __future__ import annotations

from .. import corrections as corr
from ..driver_contracts import ItemRecord, StageOutcome
from ..router import SingleFamilyTermination, second_opinion_family
from . import base, harness

STAGE = "triage"


class TerminationCheckExecutor:
    state = "TERMINATION_CHECK"

    def run(self, ctx, item: ItemRecord) -> StageOutcome:
        pre = item.data.get("termination") or {}
        if pre.get("agreed"):
            return base.outcome("TERMINAL", item=item, adjudicated=True,
                                data={"terminal": pre.get("terminal"), "citation": pre.get("citation"),
                                      "citations": pre.get("citations"),
                                      "verdict_store": "not written: Q&A classifications are recorded "
                                                       "by the interaction verdict writer"},
                                reason=f"{pre.get('stage')}: both families agreed on {pre.get('terminal')}")
        first = item.data.get("triage_first")
        packet = item.data.get("triage_packet")
        if not first or not packet:
            reason = "termination check without a first opinion or its packet"
            return base.outcome("ESCALATED", item=item, reason=reason,
                                escalation=base.escalation(item, reason))
        try:
            family = second_opinion_family(first["family"], base.available_families(ctx))
        except SingleFamilyTermination as e:
            reason = f"single family: {e}"
            return base.outcome("ESCALATED", item=item, reason=reason,
                                escalation=base.escalation(item, reason, [first]))

        call = base.call_worker(ctx, item, stage=STAGE, family=family, assignment="forced",
                                prompt=packet["prompt"], references=packet.get("references") or (),
                                prompt_version=packet.get("prompt_version"))
        if call.result.status == "quota_exhausted":
            reason = f"single family: the second opinion ({family}) is exhausted: {call.result.error}"
            return base.outcome("ESCALATED", item=item, reason=reason, attempts=[call.attempt(ctx)],
                                escalation=base.escalation(item, reason, [first],
                                                           extra_history=[call.attempt_id]))
        if not call.ok:
            raise base.defer_failed(ctx, item, [call])

        out = call.output
        second = base.argument(call, call_value=out.get("classification"), citation=out.get("citation"),
                               reasoning=out.get("reasoning", ""))
        cites = [base.format_citation(first.get("citation")), base.format_citation(second["citation"])]
        problems = []
        if second["call"] != first.get("call"):
            problems.append(f"families disagree: {first.get('family')} {first.get('call')} vs "
                            f"{family} {second['call']}")
        for arg, cite in ((first, cites[0]), (second, cites[1])):
            if not cite:
                problems.append(f"{arg.get('family')} gives no citation")
        if problems:
            reason = "; ".join(problems)
            corrections = corr.family_disagreement(
                (first.get("attempt_id"), first.get("call")), (call.attempt_id, second["call"]),
                stage=STAGE, item=item.item, ts=ctx.now()) if first.get("attempt_id") else []
            return base.outcome("ESCALATED", item=item, reason=reason, corrections=corrections,
                                attempts=[call.attempt(ctx, outcome="escalated", parent=first.get("attempt_id"))],
                                data={"triage_second": second},
                                escalation=base.escalation(item, reason, [first, second],
                                                           extra_history=[call.attempt_id]))

        terminal = first["call"]
        data = {"triage_second": second, "terminal": terminal, "citation": cites[0], "citations": cites}
        if item.kind == "clause" and terminal == "dcgo_quirk":
            clause = base.clause_of(item)
            rc, stdout, stderr = harness.run(ctx, harness.verdict_triage_argv(ctx, clause, terminal, cites[0]),
                                             harness.VERDICT_TIMEOUT_S)
            if rc != 0:
                reason = (f"both families agreed on {terminal}, but the verdict store refused the triage: "
                          f"{(stderr or stdout).strip()[-500:]}")
                return base.outcome("ESCALATED", item=item, reason=reason, data=data,
                                    attempts=[call.attempt(ctx, outcome="escalated",
                                                           parent=first.get("attempt_id"))],
                                    escalation=base.escalation(item, reason, [first, second],
                                                               extra_history=[call.attempt_id]))
            data["verdict_store"] = f"triaged {clause} {terminal}"
        else:
            data["verdict_store"] = (f"not written: `dcgo-harness verdict-triage` records only "
                                     f"ours_wrong|dcgo_quirk|undetermined on a diverged clause row "
                                     f"({item.kind} {terminal})")
        return base.outcome("TERMINAL", item=item, adjudicated=True, data=data,
                            attempts=[call.attempt(ctx, outcome="accepted", parent=first.get("attempt_id"))],
                            reason=f"{first.get('family')} and {family} agree: {terminal} ({cites[0]})")
