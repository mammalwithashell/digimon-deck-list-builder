"""IMPLEMENTING (stage `implement`): one card's YAML DSL spec and its DebugRunner
tests, DSL-first (CLAUDE.md rules 18, 28; design D4).

    ok, no gaps      -> REVIEW   with a non-engine MergeRequest for the worker diff
    ok, gaps         -> PARKED   data["gap_id"] = the first gap; no merge
    call failed      -> StageDeferred (attempt + schema correction ledgered)

`data["implementer_family"]` records who implemented the card: the reviewer
must be the other family, and so must the author of its interaction exams.
"""
from __future__ import annotations

from ..driver_contracts import ItemRecord, MergeRequest, StageOutcome
from . import base, prompts

STAGE = "implement"


def _directives(item: ItemRecord) -> str:
    rows = item.data.get("review_directives") or []
    if not rows:
        return ""
    lines = [f"- `{d.get('path')}`: {d.get('directive')}" if d.get("path") else f"- {d.get('directive')}"
             for d in rows]
    return ("\n## Reviewer directives (your previous attempt was rejected)\n"
            "Address every one:\n" + "\n".join(lines) + "\n")


def _mode(ctx, card: str) -> str:
    spec = base.spec_path(card)
    if base.repo_path(ctx, spec).is_file():
        return (f"AUDIT the existing spec `{spec}`: compare it with the printed text and DCGO's "
                "behaviour, fix every discrepancy, and make sure its behavioral tests cover every "
                "clause (add the missing ones, failing first).")
    return "IMPLEMENT the card: write its behavioral tests first, then the spec that makes them pass."


class ImplementExecutor:
    state = "IMPLEMENTING"

    def run(self, ctx, item: ItemRecord) -> StageOutcome:
        card = item.ident
        name = base.card_name(ctx, card)
        script = base.dcgo_script(ctx, card)
        examples = base.example_specs(ctx, card)
        spec, tests, bundle = base.spec_path(card), base.test_path(card), base.bundle_path(card)

        def render(attempt_id: str) -> str:
            return prompts.render(
                STAGE, card_id=card, card_name=name, mode=_mode(ctx, card), spec_path=spec,
                test_path=tests, bundle_path=bundle,
                dcgo_script=script or f"(no DCGO script found for {card})",
                examples=base.bullet_list(examples), attempt_id=attempt_id,
                directives=_directives(item))

        refs = [bundle, script, *examples, "docs/RUST_DSL_AGENT_GUIDE.md", "docs/RUST_DSL_TEST_API.md",
                "docs/RUST_ENGINE_API.md", "docs/digimon-rules/keyword-semantics.md"]
        if base.repo_path(ctx, spec).is_file():
            refs.append(spec)
        call, calls = base.routed_call(ctx, item, stage=STAGE, prompt=render, references=refs)
        if not call.ok:
            raise base.defer_failed(ctx, item, calls)

        result = call.output
        attempts = [c.attempt(ctx) for c in calls[:-1]] + [call.attempt(ctx, outcome="accepted")]
        data = {"implementer_family": call.family, "implement_attempt": call.attempt_id,
                "implement_result": result, "artifacts": dict(call.result.artifacts or {}),
                "review_directives": None}
        gaps = [g for g in result.get("gaps") or [] if isinstance(g, dict)]
        if gaps:
            data.update(gap_id=gaps[0].get("id"), gaps=gaps)
            return base.outcome("PARKED", item=item, attempts=attempts, data=data,
                                reason=f"needs substrate: {', '.join(str(g.get('id')) for g in gaps)}",
                                events_data={"gap_id": gaps[0].get("id")})
        data.update(gap_id=None, gaps=None)
        merge = MergeRequest(attempt_id=call.attempt_id, family=call.family, model=call.packet.model,
                             artifacts=dict(call.result.artifacts or {}), engine=False,
                             subject=f"card-loop: implement {name} ({card})")
        return base.outcome("REVIEW", item=item, attempts=attempts, data=data, merge_request=merge,
                            reason=f"implemented by {call.family}; review by the other family")
