"""ENCODE (stage `encode_ruling`; design D5, D7): the publisher's answer
becomes the scenario's `expect_ruling:` block -- the EXPECTED VALUE of the
three-way comparison -- authored by the routed family and verified BLIND by
the other (the verifier sees the candidate block, never the author's quote or
reasoning), decided by `packets.agree_encoding`.

    probe:<...>                     -> AUTHORING (nothing to encode)
    verifier agrees                 -> SIM, data["expect_ruling"] (the sim step writes it into
                                       the line) and data["base_scenario"] (the line its `at:`
                                       counts on); AUTHORING for an item with no line yet
    the line misses the ruling      -> AUTHORING with the objection (data["encode_feedback"]):
      (encoder or verifier says so)    the author reworks the line
    verifier rejects the block      -> ENCODE again with the objection; the stage cap bounds it
    verifier disagrees, no line     -> ESCALATED with both arguments (the earlier order)
    a family unavailable            -> ESCALATED (single family, D5)
    the author's block is malformed -> ENCODE again, `gate_fail` (encode_validate);
                                       the `encode_ruling` attempt cap bounds the retries
    a call failed                   -> StageDeferred

The block's `at:` steps index the AUTHORED line (`data["scenario_paths"]`):
the second pilot encoded against library lines that never reached the
ruling's situation and 60% of those encodings escalated. An item from before
that reorder (no line yet) still anchors on a BASE line: an explicit
`data["base_scenario"]`, else the library scenario of a clause the ruling is
about, else the interaction's planned path, empty.
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
        # The authored line, when the item has one: the block anchors on steps
        # that exercise the ruling. Items from the earlier order (block before
        # line) still anchor on a library line.
        authored = next((p for p in (item.data.get("scenario_paths") or []) if base.repo_path(ctx, p).is_file()), None)
        if authored:
            path, text, how = authored, base.repo_path(ctx, authored).read_text(encoding="utf-8"), "authored"
        else:
            path, text, how = base_line(ctx, item, iid, cards)
        version = prompts.version(STAGE)
        feedback = item.data.get("encode_feedback")

        author_family, assignment = base.routed_family(ctx, STAGE, item)
        pa = packets.encode_ruling_inputs(q, qa, clauses, scenario_path=path, scenario_yaml=text, mode="author",
                                          feedback=feedback)
        ca = base.run_packet(ctx, item, self._packet(ctx, pa, author_family, version), assignment=assignment)
        if ca.result.status == "quota_exhausted":
            return single_family(item, STAGE, [author_family], [ca.attempt(ctx)])
        if not ca.ok:
            raise base.defer_failed(ctx, item, [ca])
        if how == "authored" and ca.output.get("line_exercises_ruling") is False and not ca.output.get("expect_ruling"):
            # The encoder read the line and found no step at which the ruling's
            # situation happens: the line's fault, not an encoding failure.
            why = f"{author_family} (encoder): {ca.output.get('reasoning', '')}"
            return base.outcome("AUTHORING", item=item, attempts=[ca.attempt(ctx, outcome="accepted")],
                                data={"expect_ruling": None, "encode_feedback": why, "base_scenario": path,
                                      "history": base.history(item, ca.attempt_id)},
                                reason=f"the line does not exercise {q}: {ca.output.get('reasoning', '')[:200]}")
        problems = packets.validate_encode_result(ca.output, "author")
        if problems:
            # The author's correction, not infrastructure: the item stays in
            # ENCODE for another call (the stage cap bounds it). Deferring here
            # escalated Q2671 after one malformed reply in the second pilot.
            detail = "; ".join(problems)
            return StageOutcome(
                next_state=item.state, reason=f"malformed expect_ruling block: {detail}",
                attempts=[ca.attempt(ctx, outcome="gate_failed")],
                corrections=[corr.gate_fail(ca.attempt_id, gate="encode_validate", stage=STAGE,
                                            item=item.item, detail=detail[:2000], ts=ctx.now())],
                data={"history": base.history(item, ca.attempt_id)})

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
            if how == "authored":
                # With a real line the disagreement is actionable: the line never
                # reaches the ruling (back to its author with the objection) or
                # the block misses (re-encode, under the stage cap). Escalation
                # is the cap's, not the first disagreement's.
                why = (f"{verifier} (verifier): {cv.output.get('reasoning', '')} | quote: "
                       f"{cv.output.get('answer_quote', '')}")
                attempts = [ca.attempt(ctx, outcome="gate_failed"),
                            cv.attempt(ctx, outcome="accepted", parent=ca.attempt_id)]
                data = {"expect_ruling": None, "encode_feedback": why, "base_scenario": path,
                        "history": base.history(item, ca.attempt_id, cv.attempt_id)}
                if cv.output.get("line_exercises_ruling") is False:
                    return base.outcome("AUTHORING", item=item, attempts=attempts, corrections=corrections, data=data,
                                        reason=f"the line does not exercise {q}: {cv.output.get('reasoning', '')[:200]}")
                return base.outcome("ENCODE", item=item, attempts=attempts, corrections=corrections, data=data,
                                    reason=f"{verifier} rejected the encoding of {q}; re-encoding against the line")
            return base.outcome("ESCALATED", item=item, reason=reason, corrections=corrections,
                                attempts=[ca.attempt(ctx, outcome="escalated"),
                                          cv.attempt(ctx, outcome="escalated", parent=ca.attempt_id)],
                                escalation=base.escalation(item, reason, args,
                                                           extra_history=[ca.attempt_id, cv.attempt_id]))
        return base.outcome(
            "SIM" if how == "authored" else "AUTHORING", item=item,
            attempts=[ca.attempt(ctx, outcome="accepted"), cv.attempt(ctx, outcome="accepted", parent=ca.attempt_id)],
            data={"expect_ruling": packets.expect_ruling_block(ca.output),
                  "base_scenario": path if how != "planned" else None,
                  "encode_attempts": [ca.attempt_id, cv.attempt_id], "encode_feedback": None},
            reason=f"{author_family} encoded {q}; {verifier} verified it blind")

    def _packet(self, ctx, p: packets.PacketInputs, family: str, version: str):
        text = prompts.render(STAGE, task=p.task, inputs_json=base.json_block(p.inputs))
        return packets.to_task_packet(p, family=family, attempt_id=ctx.new_attempt_id(),
                                      worktree=str(ctx.repo), prompt_version=version, prompt=text)
