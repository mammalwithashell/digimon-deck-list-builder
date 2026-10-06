"""FIX (`fix_card`, or `fix_engine` when the substrate is what is wrong;
design D4, D13, spec "Fixes pass the fix gate and engine fixes stay off main").

Which stage: `data["engine_fix"]` is decided and recorded here. It is True when
a previous fix attempt reported gaps, an earlier FIX already chose the engine,
or the triage reasoning names an engine / DSL gap (`ENGINE_HINTS`); otherwise a
card / YAML fix. A `fix_card` result that reports gaps turns into a
`fix_engine` call in the same step (the card cannot be fixed without the
substrate, and the exam state machine has no PARKED).

    ok, cited, no remaining gaps -> GATE with MergeRequest(engine=data["engine_fix"])
    no citation                  -> ESCALATED (spec: a logged finding, not a fix)
    fix_engine still has gaps    -> ESCALATED naming them
    call failed                  -> StageDeferred
The GATE itself (citation, fail-before/pass-after test, scoped suite) is the
driver's.
"""
from __future__ import annotations

import re

from ..driver_contracts import ItemRecord, MergeRequest, StageOutcome
from . import base, prompts
from .triage import subject as item_subject

ENGINE_HINTS = re.compile(
    r"\b(?:engine|dsl)[ -](?:gap|primitive|bug|fix|change|code|limitation|support)\b"
    r"|\bdsl vocabulary\b|\bsubstrate\b|\braw_rust\b"
    r"|code/digimon-(?:engine|dsl)/src\b|\bsrc/[\w/]+\.rs\b|\bG-(?:ENG|DSL)[A-Z0-9-]*",
    re.IGNORECASE)


def decide_engine_fix(item: ItemRecord) -> tuple[bool, str]:
    if item.data.get("engine_fix") is True:
        return True, item.data.get("engine_fix_reason") or "an earlier fix chose the engine"
    prev = item.data.get("fix_result") or {}
    if prev.get("gaps"):
        return True, "the previous fix reported gaps: " + ", ".join(str(g.get("id")) for g in prev["gaps"])
    reasoning = (item.data.get("triage_first") or {}).get("reasoning") or ""
    m = ENGINE_HINTS.search(reasoning)
    if m:
        return True, f"triage names the substrate: {m.group(0)!r}"
    return False, "card / YAML fix: triage names no engine or DSL gap"


def _triage_text(item: ItemRecord) -> str:
    t = item.data.get("triage_first") or {}
    return (f"- classification: {t.get('call')} (by {t.get('family')})\n"
            f"- citation: {base.format_citation(t.get('citation')) or '(none)'}\n"
            f"- reasoning: {t.get('reasoning') or '(none)'}")


def _previous(item: ItemRecord) -> str:
    gate = item.data.get("gate_result") or item.data.get("gate")
    if not gate:
        return ""
    reasons = gate.get("reasons") if isinstance(gate, dict) else gate
    lines = reasons if isinstance(reasons, list) else [str(reasons)]
    return "\n## The previous fix failed the fix gate\n" + "\n".join(f"- {ln}" for ln in lines) + "\n"


def _gaps_text(gaps) -> str:
    return ", ".join(f"`{g.get('id')}` ({g.get('kind')}: {g.get('summary')})" for g in gaps or []) or "(none)"


class FixExecutor:
    state = "FIX"

    def run(self, ctx, item: ItemRecord) -> StageOutcome:
        engine_fix, why = decide_engine_fix(item)
        calls_all = []
        known_gaps = list((item.data.get("fix_result") or {}).get("gaps") or [])
        attempts = []
        if not engine_fix:
            call, calls = self._call(ctx, item, "fix_card", known_gaps)
            calls_all += calls
            gaps = list(call.output.get("gaps") or [])
            if not gaps:
                return self._finish(ctx, item, call, calls_all, engine_fix=False, why=why, attempts=attempts)
            attempts += [c.attempt(ctx) for c in calls[:-1]] + [
                call.attempt(ctx, outcome="accepted", parent=self._parent(item))]
            engine_fix, why, known_gaps = True, "fix_card reported gaps: " + _gaps_text(gaps), gaps
        call, calls = self._call(ctx, item, "fix_engine", known_gaps)
        calls_all += calls
        return self._finish(ctx, item, call, calls_all, engine_fix=True, why=why, attempts=attempts,
                            known_gaps=known_gaps, prior_calls=len(calls_all) - len(calls))

    def _parent(self, item: ItemRecord) -> str | None:
        return (item.data.get("triage_first") or {}).get("attempt_id")

    def _call(self, ctx, item: ItemRecord, stage: str, known_gaps):
        cards = base.item_cards(ctx, item)
        card = cards[0] if cards else item.ident
        paths = list(item.data.get("scenario_paths") or [])
        oracle = item.data.get("oracle") or {}
        scripts = [base.dcgo_script(ctx, c) for c in cards]
        common = dict(item_id=item.item, triage=_triage_text(item), divergence_json=base.json_block(oracle),
                      scenario_paths=base.bullet_list(paths), previous=_previous(item))
        if stage == "fix_card":
            def render(attempt_id: str) -> str:
                return prompts.render(stage, card_id=card, card_name=base.card_name(ctx, card),
                                      spec_path=base.spec_path(card), test_path=base.test_path(card),
                                      bundle_path=base.bundle_path(card),
                                      dcgo_script=scripts[0] or f"(no DCGO script found for {card})",
                                      attempt_id=attempt_id, **common)
        else:
            def render(attempt_id: str) -> str:
                return prompts.render(stage, subject=item_subject(ctx, item), gaps=_gaps_text(known_gaps),
                                      dcgo_scripts=base.bullet_list([s for s in scripts if s],
                                                                    empty="(no DCGO script found)"),
                                      attempt_id=attempt_id, **common)
        refs = [*paths, *(base.spec_path(c) for c in cards), *(base.test_path(c) for c in cards),
                *(base.bundle_path(c) for c in cards), *scripts, oracle.get("sidecar"),
                "docs/RUST_DSL_TEST_API.md", "docs/RUST_ENGINE_API.md", "docs/digimon-rules/rules-index.json"]
        if stage == "fix_engine":
            refs += ["qa/dsl-vocab-gaps.md", "docs/RUST_ENGINE_GAPS.md"]
        call, calls = base.routed_call(ctx, item, stage=stage, prompt=render, references=refs)
        if not call.ok:
            raise base.defer_failed(ctx, item, calls)
        return call, calls

    def _finish(self, ctx, item, call, calls_all, *, engine_fix, why, attempts, known_gaps=(),
                prior_calls=0) -> StageOutcome:
        result = call.output
        gaps = list(result.get("gaps") or [])
        citation = base.format_citation(result.get("citation"))
        parent = self._parent(item)
        tail = calls_all[prior_calls:]
        data = {"engine_fix": engine_fix, "engine_fix_reason": why, "fix_attempt": call.attempt_id,
                "fix_family": call.family, "fix_stage": call.stage, "fix_result": result,
                "fix_citation": citation, "gap_id": (gaps or list(known_gaps) or [{}])[0].get("id")}
        problem = None
        if not citation:
            problem = (f"{call.stage} by {call.family} has no citation (rule, ruling or DCGO source): "
                       "logged as a finding, not a fix")
        elif engine_fix and gaps:
            problem = f"{call.stage} by {call.family} still needs substrate: {_gaps_text(gaps)}"
        if problem:
            attempts = attempts + [c.attempt(ctx) for c in tail[:-1]] + [
                call.attempt(ctx, outcome="escalated", parent=parent)]
            return base.outcome("ESCALATED", item=item, reason=problem, data=data, attempts=attempts,
                                escalation=base.escalation(item, problem, [base.argument(
                                    call, call_value=call.stage, citation=result.get("citation"),
                                    reasoning=result.get("notes", ""))],
                                    extra_history=[c.attempt_id for c in calls_all]))
        attempts = attempts + [c.attempt(ctx) for c in tail[:-1]] + [
            call.attempt(ctx, outcome="accepted", parent=parent)]
        merge = MergeRequest(attempt_id=call.attempt_id, family=call.family, model=call.packet.model,
                             artifacts=dict(call.result.artifacts or {}), engine=engine_fix,
                             gap_id=data["gap_id"],
                             subject=f"card-loop: {call.stage} for {item.item} ({citation})")
        return base.outcome("GATE", item=item, data=data, attempts=attempts, merge_request=merge,
                            reason=f"{call.stage} by {call.family}: {why}")
