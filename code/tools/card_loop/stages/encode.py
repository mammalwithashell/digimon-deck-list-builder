"""ENCODE (stage `encode_ruling`; design D5, D7): the publisher's answer
becomes the scenario's `expect_ruling:` block -- the EXPECTED VALUE of the
three-way comparison -- authored by the routed family and verified BLIND by
the other (the verifier sees the candidate block, never the author's quote or
reasoning), decided by `packets.agree_encoding`.

    probe:<...>                     -> AUTHORING (nothing to encode)
    verifier agrees                 -> AUTHORING, data["expect_ruling"] (scenario-ready)
                                       and data["base_scenario"] (the line its `at:` counts on)
    verifier disagrees              -> ESCALATED with both arguments
    a family unavailable            -> ESCALATED (single family, D5)
    the author's block is malformed -> StageDeferred, `gate_fail` (encode_validate)
    a call failed                   -> StageDeferred

The block's `at:` steps index a scenario line, and the contract puts ENCODE
before AUTHORING, so the encoding is anchored on a BASE line: an explicit
`data["base_scenario"]`, else the library scenario of a clause the ruling is
about (the interaction author extends it, D7/spec "where possible ... extend an
oracle-confirmed clause line"), else the interaction's planned path, empty.
"""
from __future__ import annotations

from .. import corrections as corr
from ..contracts import FAMILIES
from ..driver_contracts import ItemRecord, StageOutcome
from ..interactions import packets
from ..router import other_family
from . import base, prompts
from .classify import single_family

STAGE = "encode_ruling"

NO_BASE_LINE = ("# No library line exists yet for this ruling's clauses. Encode `at:` as the step index,\n"
                "# counted from 0, right after the situation the answer describes has resolved; the\n"
                "# interaction author must keep your block verbatim and build the line to match it.\n")


def base_line(ctx, item: ItemRecord, iid: str, cards: list[str]) -> tuple[str, str, str]:
    """(path, yaml text, how) of the line the encoding is anchored on."""
    explicit = item.data.get("base_scenario")
    if explicit and base.repo_path(ctx, explicit).is_file():
        return explicit, base.repo_path(ctx, explicit).read_text(encoding="utf-8"), "data"
    examined = list((item.data.get("classification") or {}).get("examined_clauses") or [])
    for clause in examined + [c["id"] for c in base.clauses_for(ctx, cards) if c["id"] not in examined]:
        found = base.find_scenarios(ctx, [clause.split("#", 1)[0]], clause=clause)
        if found:
            return found[0], base.repo_path(ctx, found[0]).read_text(encoding="utf-8"), "library"
    return base.interaction_scenario_path(iid, cards[0] if cards else "X"), NO_BASE_LINE, "planned"


class EncodeExecutor:
    state = "ENCODE"

    def run(self, ctx, item: ItemRecord) -> StageOutcome:
        iid = base.interaction_of(item)
        if not iid.startswith("qa:"):
            return base.outcome("AUTHORING", item=item, reason="probe interactions carry no ruling to encode")
        missing = [f for f in FAMILIES if f not in base.available_families(ctx)]
        if missing:
            return single_family(item, STAGE, missing)
        q = iid[3:]
        qa = base.card_qa(ctx)
        cards = base.interaction_cards(ctx, iid)
        clauses = base.clauses_for(ctx, cards)
        path, text, how = base_line(ctx, item, iid, cards)
        version = prompts.version(STAGE)

        author_family, assignment = base.routed_family(ctx, STAGE, item)
        pa = packets.encode_ruling_inputs(q, qa, clauses, scenario_path=path, scenario_yaml=text, mode="author")
        ca = base.run_packet(ctx, item, self._packet(ctx, pa, author_family, version), assignment=assignment)
        if ca.result.status == "quota_exhausted":
            return single_family(item, STAGE, [author_family], [ca.attempt(ctx)])
        if not ca.ok:
            raise base.defer_failed(ctx, item, [ca])
        problems = packets.validate_encode_result(ca.output, "author")
        if problems:
            detail = "; ".join(problems)
            outcome = StageOutcome(
                next_state=item.state, reason=f"malformed expect_ruling block: {detail}",
                attempts=[ca.attempt(ctx, outcome="gate_failed")],
                corrections=[corr.gate_fail(ca.attempt_id, gate="encode_validate", stage=STAGE,
                                            item=item.item, detail=detail[:2000], ts=ctx.now())],
                data={"history": base.history(item, ca.attempt_id)})
            raise base.StageDeferred(outcome.reason, outcome)

        verifier = other_family(author_family)
        pv = packets.encode_ruling_inputs(q, qa, clauses, scenario_path=path, scenario_yaml=text, mode="verify",
                                          candidate=ca.output["expect_ruling"])
        cv = base.run_packet(ctx, item, self._packet(ctx, pv, verifier, version), assignment="forced")
        if cv.result.status == "quota_exhausted":
            return single_family(item, STAGE, [verifier],
                                 [ca.attempt(ctx, outcome="accepted"), cv.attempt(ctx)])
        if not cv.ok:
            raise base.defer_failed(ctx, item, [ca, cv])

        agreement = packets.agree_encoding(q, ca.output, cv.output)
        args = [base.argument(ca, call_value="encoded", reasoning=ca.output.get("reasoning", ""),
                              citation=f"qa:{q}"),
                base.argument(cv, call_value="agrees" if cv.output.get("agrees") else "does_not_encode",
                              reasoning=f"{cv.output.get('reasoning', '')} | quote: {cv.output.get('answer_quote', '')}",
                              citation=f"qa:{q}")]
        if not agreement.agreed:
            reason = "; ".join(agreement.reasons) or "the encoding was not agreed"
            corrections = corr.family_disagreement((ca.attempt_id, "encoded"), (cv.attempt_id, "not_encoded"),
                                                   stage=STAGE, item=item.item, ts=ctx.now())
            return base.outcome("ESCALATED", item=item, reason=reason, corrections=corrections,
                                attempts=[ca.attempt(ctx, outcome="escalated"),
                                          cv.attempt(ctx, outcome="escalated", parent=ca.attempt_id)],
                                escalation=base.escalation(item, reason, args,
                                                           extra_history=[ca.attempt_id, cv.attempt_id]))
        return base.outcome(
            "AUTHORING", item=item,
            attempts=[ca.attempt(ctx, outcome="accepted"), cv.attempt(ctx, outcome="accepted", parent=ca.attempt_id)],
            data={"expect_ruling": packets.expect_ruling_block(ca.output),
                  "base_scenario": path if how != "planned" else None,
                  "encode_attempts": [ca.attempt_id, cv.attempt_id]},
            reason=f"{author_family} encoded {q}; {verifier} verified it blind")

    def _packet(self, ctx, p: packets.PacketInputs, family: str, version: str):
        text = prompts.render(STAGE, task=p.task, inputs_json=base.json_block(p.inputs))
        return packets.to_task_packet(p, family=family, attempt_id=ctx.new_attempt_id(),
                                      worktree=str(ctx.repo), prompt_version=version, prompt=text)
