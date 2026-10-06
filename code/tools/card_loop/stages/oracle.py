"""ORACLE (driver-only, no model): the one-call `dcgo-harness exam --oracle`
(readiness Plan 3) per scenario -- preflight, submit, wait for THIS job, diff,
record the verdict, backfill (design D4, spec "Oracle round trip involves no
model").

Per scenario result:

    confirmed                         -> counts toward CONFIRMED (adjudicated)
    diverged                          -> DIVERGED, the whole result in data["oracle"]
    unmeasured (timeout, quarantine,  -> ORACLE again, only for the unmeasured
      DCGO stopped, no JSON line)        scenarios (data["oracle_retry_paths"]);
                                         the driver caps the round trips
    refused (verdict store refused)   -> ESCALATED (a configuration error, never "confirmed")
    DCGO `failed` with a prompt mismatch, no earlier divergence:
        `--sim-only --inspect N` shows what OUR engine asked at that step
        (spec "Prompt-sequence failures are routed by who disagreed"):
        ours also contradicts the scenario's expect -> AUTHORING (scenario_wrong),
            with the observed prompts and a `gate_fail` (gate "oracle_scenario")
            correction of the author: a scenario-caused oracle round trip (D11)
        ours matches the scenario, DCGO does not    -> DIVERGED (engines_disagree);
            the contract routes ORACLE -> DIVERGED -> TRIAGE. A missing decline
            looks exactly like this.
        ours cannot be mapped onto DCGO's prompts    -> DIVERGED (undetermined),
            the evidence goes to triage

Several scenarios: a refusal escalates; else any divergence (or engine
disagreement) goes to DIVERGED; else any scenario_wrong to AUTHORING; else any
unmeasured to ORACLE; all confirmed -> CONFIRMED.
"""
from __future__ import annotations

from typing import Mapping

from .. import corrections as corr
from ..driver_contracts import ItemRecord, StageOutcome
from . import base, harness


def _unmeasured(path: str, reason: str, **extra) -> dict:
    return {"scenario": path, "verdict": "unmeasured", "reason": reason, **extra}


class OracleExecutor:
    state = "ORACLE"

    def run(self, ctx, item: ItemRecord) -> StageOutcome:
        paths = list(item.data.get("scenario_paths") or [])
        if not paths:
            return base.outcome("AUTHORING", item=item, reason="no scenario to submit to the oracle",
                                data={"sim_failure": {"(none)": ["no scenario was authored for this item"]}})
        retry = item.data.get("oracle_retry_paths")
        todo = [p for p in paths if p in retry] if retry else paths
        results = dict(item.data.get("oracle_results") or {}) if retry else {}
        books = item.data.get("deck_books") or {}
        clause_text = base.clause_text_json(ctx)
        for path in todo:
            book = books.get(path)
            if book is None:
                cands = harness.deck_books_for(ctx.repo, path)
                book = cands[0] if cands else None
            argv = harness.oracle_argv(ctx, path, book, clause_text_json=clause_text)
            rc, out, err = harness.run(ctx, argv, harness.oracle_timeout(ctx) + 180)
            rows = harness.parse_oracle_output(rc, out, err)
            if not rows:
                tail = (err or out or "").strip().splitlines()[-3:]
                row = _unmeasured(path, f"oracle printed no result (exit {rc}): {' | '.join(tail)}")
            elif "error" in rows[0] and "verdict" not in rows[0]:
                row = _unmeasured(path, f"oracle error: {rows[0]['error']}", error=rows[0]["error"])
            else:
                row = rows[0]
            if isinstance(row.get("stall"), Mapping):
                # The player is wedged (DCGO enforces no job timeout). Restart it
                # here, once per fresh report, so the lane is not dead for every
                # later item -- but only while the heartbeat still names the
                # stalled job: another run may have restarted it already.
                row = {**row, "player_restart": self._restart_if_still_stalled(ctx, row["stall"])}
            results[path] = row

        routed = {p: self._classify(ctx, item, p, results[p]) for p in paths if p in results}
        return self._decide(ctx, item, paths, results, routed)

    # ------------------------------------------------------------------ the player

    def _restart_if_still_stalled(self, ctx, stall: Mapping) -> str:
        beat = harness.heartbeat_job(ctx)
        if beat != stall.get("job_id"):
            return f"not restarted: the heartbeat now reads {beat!r}; the player has moved on"
        return harness.restart_player(ctx)

    @staticmethod
    def _stall_evidence(path: str, row: Mapping, stall: Mapping) -> dict:
        """Triage evidence for a line DCGO never finished: no prompt name to
        compare, so the route is `undetermined` and a model reads both sides."""
        last = stall.get("last_row")
        explanation = (f"DCGO stalled on {stall.get('job_id')}: claimed {stall.get('claimed_for_s')}s ago against "
                       f"its {stall.get('limit_s')}s limit while the heartbeat still named it. The scripted answer "
                       f"after the last recorded row did not complete DCGO's prompt"
                       + (f" (last recorded row: {last})" if last else "")
                       + "; our engine ran the same line to completion sim-only. Compare what each engine "
                         "asks at that step: a selection DCGO needs more picks (or a cancel) for than ours does.")
        return {"scenario": path, "dcgo_row": None, "scenario_step": None, "step_mapping": None,
                "expected": None, "dcgo_asked": None, "ours": None, "route": "undetermined",
                "explanation": explanation, "stall": dict(stall), "player_restart": row.get("player_restart")}

    # ------------------------------------------------------------------ per scenario

    def _classify(self, ctx, item: ItemRecord, path: str, row: dict) -> tuple[str, dict | None]:
        """(`confirmed` | `diverged` | `engines_disagree` | `undetermined` |
        `scenario_wrong` | `unmeasured` | `refused`, prompt evidence)."""
        if row.get("refused"):
            return "refused", None
        verdict = row.get("verdict")
        if verdict == "confirmed":
            return "confirmed", None
        if isinstance(row.get("stall"), Mapping) and row["stall"].get("job_id") == row.get("job_id"):
            # Wedged on THIS job: DCGO could not finish the line. (Wedged on
            # another run's job, ours was simply never claimed: unmeasured.)
            return "undetermined", self._stall_evidence(path, row, row["stall"])
        pm = harness.prompt_mismatch(row)
        hm = row.get("mismatch") if isinstance(row.get("mismatch"), Mapping) else None
        if pm is None and hm is not None and isinstance(hm.get("row"), int):
            # DCGO stopped on a mismatch its message does not spell out as
            # prompts (an actor mismatch): the engines disagreed on WHO acts at
            # that step, which triage must see spelled out -- the differ's
            # "TRUNCATED, no divergence found" read as nothing to classify.
            step = hm.get("step")
            am = harness.actor_mismatch(row)
            expected = f"actor {am[0]}" if am else hm.get("expected")
            asked = f"actor {am[1]}" if am else hm.get("asked")
            explanation = (
                f"DCGO stopped at its row {hm['row']} (scenario step {step}): the scenario, which our engine runs "
                f"sim-only, has {expected} act there, but DCGO asked {asked} -- the engines disagree on who acts at "
                f"step {step} (a prompt one engine asks and the other skips, or a turn that ends differently). "
                f"Nothing diverged in the {row.get('denominator') or 'compared rows'} before it."
                if am else str(row.get("reason") or ""))
            evidence = {"scenario": path, "dcgo_row": hm["row"], "scenario_step": step,
                        "step_mapping": "harness", "expected": expected, "dcgo_asked": asked,
                        "ours": expected, "route": "engines_disagree", "explanation": explanation}
            return "engines_disagree", evidence
        cm = harness.candidate_mismatch(row) if pm is None else None
        if cm is not None:
            # DCGO's prompt matched but offered other cards than the pick our
            # engine accepted sim-only: the engines disagree on that selection
            # (BT26-005#inherited#0 was retried three times as "unmeasured").
            evidence = {"scenario": path, "dcgo_row": None, "scenario_step": None, "step_mapping": None,
                        "expected": cm.prompt, "dcgo_asked": cm.prompt, "ours": cm.prompt, "route": "engines_disagree",
                        "explanation": (f"DCGO's {cm.prompt} offered {cm.offered} where the scenario picks {cm.wanted} "
                                        f"(pick {cm.pick} of {cm.wanted_list}); our engine accepted that pick sim-only, "
                                        f"so the two engines offer different candidates at that selection")}
            return "engines_disagree", evidence
        if pm is None:
            return ("diverged" if verdict == "diverged" else "unmeasured"), None
        steps = base.scenario_steps(ctx, path)
        if hm is not None and isinstance(hm.get("step"), int):
            # The harness maps DCGO's wire row to the scenario step exactly
            # (it knows the wire rows per step); the heuristic is the fallback.
            step, how = int(hm["step"]), "harness"
        else:
            step, how = harness.scenario_step_for_row(steps, pm)
        div = row.get("divergence") or {}
        if verdict == "diverged" and isinstance(div.get("step"), int) and div["step"] < step:
            return "diverged", None          # a real divergence happened first
        book = (item.data.get("deck_books") or {}).get(path)
        rc, out, err = harness.run(ctx, harness.sim_argv(ctx, path, book, inspect=step), harness.SIM_TIMEOUT_S)
        payload = harness.parse_inspect(out)
        snapshot = payload.get("snapshot") if payload else None
        decision = harness.decide_prompt_route(pm, snapshot)
        evidence = {"scenario": path, "dcgo_row": pm.row, "scenario_step": step, "step_mapping": how,
                    "expected": pm.expected, "dcgo_asked": pm.asked, "ours": decision.ours,
                    "ours_snapshot": snapshot, "route": decision.route, "explanation": decision.explanation}
        if payload is None:
            evidence["inspect_error"] = harness.inspect_error(out) or (err or "").strip()[-500:] or f"exit {rc}"
        return decision.route, evidence

    # ------------------------------------------------------------------ aggregate

    def _decide(self, ctx, item, paths, results, routed) -> StageOutcome:
        def first(*kinds):
            return next(((p, routed[p]) for p in paths if p in routed and routed[p][0] in kinds), None)

        base_data = {"oracle_results": results, "oracle_retry_paths": None}
        hit = first("refused")
        if hit:
            p, _ = hit
            reason = f"verdict store refused {p}: {'; '.join(results[p].get('refused') or [])}"
            return base.outcome("ESCALATED", item=item, reason=reason, data={**base_data, "oracle": results[p]},
                                escalation=base.escalation(item, reason))
        hit = first("diverged", "engines_disagree", "undetermined")
        if hit:
            p, (kind, evidence) = hit
            data = {**base_data, "oracle": results[p],
                    "prompt_route": None if kind == "diverged" else kind, "prompt_evidence": evidence}
            if kind == "diverged":
                reason = results[p].get("first_divergence") or results[p].get("reason") or "diverged"
            elif evidence.get("stall"):
                reason = f"player stalled, {kind}: {evidence['explanation']}"
            else:
                reason = f"prompt mismatch, {kind}: {evidence['explanation']}"
            return base.outcome("DIVERGED", item=item, reason=reason, data=data)
        hit = first("scenario_wrong")
        if hit:
            p, (_, evidence) = hit
            corrections = []
            author = item.data.get("author_attempt")
            if author:
                stage = item.data.get("author_stage") or (
                    "author_interaction" if item.kind == "interaction" else "author_clause")
                corrections.append(corr.gate_fail(author, gate="oracle_scenario", stage=stage, item=item.item,
                                                  detail=evidence["explanation"][:2000], ts=ctx.now()))
            data = {**base_data, "oracle": results[p], "prompt_route": "scenario_wrong",
                    "prompt_evidence": evidence, "sim_failure": None}
            return base.outcome("AUTHORING", item=item, corrections=corrections, data=data,
                                reason=f"both engines contradict the scenario: {evidence['explanation']}")
        pending = [p for p in paths if p not in routed or routed[p][0] == "unmeasured"]
        if pending:
            reasons = "; ".join(str(results.get(p, {}).get("reason") or "not run") for p in pending)
            return base.outcome("ORACLE", item=item, reason=f"unmeasured: {reasons}",
                                data={**base_data, "oracle_retry_paths": pending,
                                      "oracle": results.get(pending[0])})
        return base.outcome("CONFIRMED", item=item, adjudicated=True, data={**base_data,
                            "oracle": results[paths[0]] if paths else None, "prompt_route": None,
                            "prompt_evidence": None},
                            reason=f"confirmed by the oracle ({len(paths)} scenario(s))")
