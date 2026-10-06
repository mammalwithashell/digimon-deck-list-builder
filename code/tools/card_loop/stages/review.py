"""REVIEW (stage `review`): the OTHER family than the implementer audits the
implement attempt's diff, read-only (design D11/D12).

    accept   -> IMPLEMENTED (adjudicated)
    reject   -> IMPLEMENTING with data["review_directives"], and a `review_reject`
                correction of the implement attempt by the review attempt
    no other family available, or the call failed -> StageDeferred
"""
from __future__ import annotations

from .. import corrections as corr
from ..driver_contracts import ItemRecord, StageOutcome
from . import base, prompts

STAGE = "review"


def _report(result: dict) -> str:
    lines = [f"- notes: {result.get('notes') or '(none)'}",
             f"- files: {base.bullet_list(result.get('files') or [])}",
             f"- tests: {base.bullet_list(result.get('tests') or [])}",
             "- test result lines (verbatim):"]
    lines += [f"    {ln}" for ln in result.get("test_result_lines") or []] or ["    (none reported)"]
    return "\n".join(lines)


class ReviewExecutor:
    state = "REVIEW"

    def run(self, ctx, item: ItemRecord) -> StageOutcome:
        card = item.ident
        implementer = item.data.get("implementer_family")
        impl_attempt = item.data.get("implement_attempt")
        arts = item.data.get("artifacts") or {}
        script = base.dcgo_script(ctx, card)
        spec, tests, bundle = base.spec_path(card), base.test_path(card), base.bundle_path(card)
        prompt = prompts.render(
            STAGE, card_id=card, card_name=base.card_name(ctx, card),
            implementer_attempt=impl_attempt or "(unknown)",
            diff_path=arts.get("diff") or "(no diff captured)",
            manifest_path=arts.get("manifest") or "(no manifest captured)",
            spec_path=spec, test_path=tests, bundle_path=bundle,
            dcgo_script=script or f"(no DCGO script found for {card})",
            implementer_report=_report(item.data.get("implement_result") or {}))
        refs = [arts.get("diff"), arts.get("manifest"), spec, tests, bundle, script,
                "docs/RUST_DSL_TEST_API.md"]
        call, calls = base.routed_call(ctx, item, stage=STAGE, prompt=prompt, references=refs,
                                       avoid=implementer)
        if not call.ok:
            raise base.defer_failed(ctx, item, calls)

        result = call.output
        attempts = [c.attempt(ctx) for c in calls[:-1]] + [
            call.attempt(ctx, outcome="accepted", parent=impl_attempt)]
        data = {"review_attempt": call.attempt_id, "review_family": call.family,
                "review_summary": result.get("summary", "")}
        if result.get("verdict") == "accept":
            data["review_directives"] = None
            return base.outcome("IMPLEMENTED", item=item, attempts=attempts, data=data,
                                adjudicated=True, reason=f"accepted by {call.family} review")
        directives = list(result.get("directives") or [])
        data["review_directives"] = directives
        corrections = []
        if impl_attempt:
            corrections.append(corr.review_reject(
                impl_attempt, reviewer_attempt=call.attempt_id, stage="implement", item=item.item,
                detail=(result.get("summary") or "")[:2000], ts=ctx.now()))
        return base.outcome("IMPLEMENTING", item=item, attempts=attempts, data=data,
                            corrections=corrections,
                            reason=f"rejected by {call.family} review: {len(directives)} directive(s)")
