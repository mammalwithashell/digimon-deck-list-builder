"""AUTHORING (`author_clause` for clause items, `author_interaction` for
interaction items; design D4, D6-D8, D11).

    library scenario exists, nothing failed yet -> SIM, no model call (iteration 0)
    ok, scenario paths                          -> SIM, with a non-engine MergeRequest
                                                   so the run tree has the file(s)
    ok, no paths (no legal line)                -> ESCALATED with the author's reason
    call failed / no allowed family             -> StageDeferred

An interaction exam is authored adversarially (spec "Interaction exams are
authored adversarially"): the router FORCES the family that did not implement
the card (`router.route(..., implementer_family=...)`), and a missing family
defers the item rather than letting the implementer examine itself. The
implementer comes from the run's card item, else the attempt ledger.
"""
from __future__ import annotations

import yaml

from ..driver_contracts import ItemRecord, MergeRequest, StageOutcome
from . import base, prompts


def _previous(item: ItemRecord) -> str:
    """Why the last scenario did not hold: sim failures, or the prompt sequence
    both engines observed when they contradicted the scenario's `expect:`."""
    parts = []
    for path, lines in (item.data.get("sim_failure") or {}).items():
        parts.append(f"- `{path}` failed sim-only:\n" + "\n".join(f"    {ln}" for ln in lines))
    ev = item.data.get("prompt_evidence")
    if ev:
        parts.append(
            f"- `{ev.get('scenario')}`: at step {ev.get('scenario_step')} the scenario expected "
            f"`{ev.get('expected')}`, DCGO asked `{ev.get('dcgo_asked')}`, our engine asked "
            f"`{ev.get('ours')}` ({ev.get('explanation')}). Fix the `expect:` and the line, not the engines.")
    fb = item.data.get("encode_feedback")
    if fb:
        parts.append(f"- the publisher's ruling could not be encoded against your line: {fb}\n"
                     "  Rework the line so the situation the question describes happens on it, and stop "
                     "right after it resolves.")
    tf = item.data.get("triage_feedback")
    if tf:
        parts.append(f"- triage of the oracle run judged the exam itself wrong: {tf}\n"
                     "  Change what it names; do not touch the engines.")
    if not parts:
        return ""
    return ("\n## Your previous scenario did not hold\n" + "\n".join(parts)
            + "\nRevise the same file rather than starting over.\n")


def _clause_lines(records) -> str:
    return "\n".join(f"- `{c['id']}` ({c.get('label', '')}): {c.get('text', '')}" for c in records) or "- (none)"


class AuthoringExecutor:
    state = "AUTHORING"

    def run(self, ctx, item: ItemRecord) -> StageOutcome:
        if item.kind == "interaction":
            return self._interaction(ctx, item)
        return self._clause(ctx, item)

    # ------------------------------------------------------------------ clause

    def _clause(self, ctx, item: ItemRecord) -> StageOutcome:
        clause = base.clause_of(item)
        card = clause.split("#", 1)[0]
        reused = self._reuse(ctx, item, base.find_scenarios(ctx, [card], clause=clause))
        if reused is not None:
            return reused
        rec = base.clause_record(ctx, clause) or {"id": clause, "label": "", "text": "(clause text not found)"}
        path = (item.data.get("scenario_paths") or [base.clause_scenario_path(clause)])[0]
        script = base.dcgo_script(ctx, card)
        notes = base.notes_files(ctx, card)

        def render(attempt_id: str) -> str:
            return prompts.render(
                "author_clause", clause_id=clause, card_id=card, card_name=base.card_name(ctx, card),
                clause_label=rec.get("label") or "", clause_text=rec.get("text") or "",
                scenario_path=path, scenario_dir=base.scenario_dir(card),
                notes_files=base.bullet_list(notes), bundle_path=base.bundle_path(card),
                dcgo_script=script or f"(no DCGO script found for {card})",
                keywords=base.bullet_list(base.keywords_of([rec]), empty="(none)"),
                attempt_id=attempt_id, previous=_previous(item))

        refs = [base.bundle_path(card), script, base.scenario_dir(card), *notes, "qa/exam-authoring-guide.json",
                "docs/digimon-rules/keyword-semantics.md"]
        if base.repo_path(ctx, path).is_file():
            refs.append(path)
        return self._finish(ctx, item, "author_clause", render, refs, implementer=None, subject=clause)

    # ------------------------------------------------------------------ interaction

    def _interaction(self, ctx, item: ItemRecord) -> StageOutcome:
        iid = base.interaction_of(item)
        cards = base.interaction_cards(ctx, iid) or [iid.split(":")[1].split("#")[0]]
        primary = cards[0]
        reused = self._reuse(ctx, item, base.find_scenarios(ctx, cards, interaction=iid))
        if reused is not None:
            return reused
        implementer = next((f for f in (base.implementer_family(ctx, c) for c in cards) if f), None)
        entry = (base.denominator(ctx).get("interactions") or {}).get(iid) or {}
        negative = iid.endswith(":neg") or entry.get("kind") == "negative"
        clause_ids, target, ruling_rule = self._target(ctx, item, iid, entry, cards)
        records = [r for r in (base.clause_record(ctx, c) for c in clause_ids) if r]
        path = (item.data.get("scenario_paths") or [base.interaction_scenario_path(iid, primary)])[0]
        base_line = item.data.get("base_scenario") or next(
            (s for c in clause_ids for s in base.find_scenarios(ctx, [c.split("#")[0]], clause=c)), None)
        notes = base.notes_files(ctx, primary)
        kind_rule = ("- This is a NEGATIVE probe: the line must reach the situation the clause guards and "
                     "show that the clause does NOT apply there (a permanent outside its scope is "
                     "unaffected, or it does not fire on the wrong turn)." if negative else
                     "- The line must make the interaction actually happen, not merely set it up.")

        def render(attempt_id: str) -> str:
            return prompts.render(
                "author_interaction", interaction_id=iid, target=target,
                cards=", ".join(f"{base.card_name(ctx, c)} ({c})" for c in cards),
                clauses=_clause_lines(records), scenario_path=path,
                source="qa" if iid.startswith("qa:") else "probe",
                kind="negative" if negative else "positive", kind_rule=kind_rule, ruling_rule=ruling_rule,
                scenario_dir=base.scenario_dir(primary), notes_files=base.bullet_list(notes),
                base_scenario=f"`{base_line}`" if base_line else "(none in the library yet)",
                keywords=base.bullet_list(base.keywords_of(records), empty="(none)"),
                attempt_id=attempt_id, previous=_previous(item))

        refs = [*(base.bundle_path(c) for c in cards), *(base.dcgo_script(ctx, c) for c in cards),
                base_line, base.scenario_dir(primary), *notes, "qa/exam-authoring-guide.json",
                "docs/digimon-rules/keyword-semantics.md"]
        # A ruling not yet encoded is encoded against THIS line next (ENCODE ->
        # SIM); a block agreed earlier rides along to the sim check directly.
        next_state = "ENCODE" if iid.startswith("qa:") and not item.data.get("expect_ruling") else "SIM"
        return self._finish(ctx, item, "author_interaction", render, refs, implementer=implementer,
                            subject=iid, extra={"base_scenario": base_line, "encode_feedback": None,
                                                "triage_feedback": None},
                            next_state=next_state)

    def _target(self, ctx, item, iid, entry, cards):
        if iid.startswith("qa:"):
            q = iid[3:]
            r = (base.card_qa(ctx).get("qa") or {}).get(q) or {}
            target = (f"Official ruling {q} ({r.get('date') or 'undated'}), `data/card_qa.json`:\n"
                      f"Q: {r.get('question', '')}\nA: {r.get('answer', '')}")
            cls = item.data.get("classification") or {}
            clause_ids = list(cls.get("examined_clauses") or []) or [
                c["id"] for c in base.clauses_for(ctx, cards)]
            block = item.data.get("expect_ruling")
            if block:
                # An item encoded under the earlier order (block before line).
                ruling_rule = (
                    "- Copy this `expect_ruling:` block into the scenario verbatim. Two model families "
                    "agreed it encodes the publisher's answer: do not change its values. Its `at:` "
                    "steps count from the base line below, so keep that line's steps up to the last "
                    "`at:` unchanged.\n```yaml\nexpect_ruling:\n"
                    + "".join(f"  {ln}\n" for ln in yaml.safe_dump(block, sort_keys=False).splitlines())
                    + "```")
            else:
                ruling_rule = (
                    "- Build the line so the situation the question describes actually happens -- the "
                    "card, timing and choice the answer rules on -- and stop right after it resolves. "
                    "Do not write an `expect_ruling:` block and leave `assert:` empty: once your line "
                    "passes, another model family encodes the publisher's answer against its steps and "
                    "a third checks that encoding blind.")
            return clause_ids, target, ruling_rule
        clause = entry.get("clause_id") or iid.split(":")[1]
        family = entry.get("family") or (iid.split(":")[2] if iid.count(":") >= 2 else "?")
        why = entry.get("why") or ""
        target = f"Risk probe `{family}` on clause `{clause}`" + (f": {why}" if why else ".")
        return [clause], target, ""

    # ------------------------------------------------------------------ shared

    def _reuse(self, ctx, item: ItemRecord, found: list[str]) -> StageOutcome | None:
        """A committed scenario and no failed attempt yet: examine it as is. A
        Q&A item without an agreed block has its ruling encoded against that
        line first (ENCODE), as a freshly authored line would."""
        tried = (item.data.get("author_attempt") or item.data.get("sim_failure") or item.data.get("prompt_evidence")
                 or item.data.get("encode_feedback") or item.data.get("triage_feedback"))
        if not found or tried or item.data.get("scenario_paths"):
            return None
        qa = item.kind == "interaction" and base.interaction_of(item).startswith("qa:")
        next_state = "ENCODE" if qa and not item.data.get("expect_ruling") else "SIM"
        return base.outcome(next_state, item=item, data={"scenario_paths": found, "author_attempt": None,
                                                         "author_family": None, "sim_failure": None,
                                                         "prompt_evidence": None},
                            reason=f"reusing library scenario(s) {', '.join(found)}")

    def _finish(self, ctx, item, stage, render, refs, *, implementer, subject, extra=None,
                next_state: str = "SIM") -> StageOutcome:
        call, calls = base.routed_call(ctx, item, stage=stage, prompt=render, references=refs,
                                       implementer=implementer)
        if not call.ok:
            raise base.defer_failed(ctx, item, calls)
        result = call.output
        listed = list(dict.fromkeys(base.posix(p) for p in result.get("scenario_paths") or [] if p))
        # A deck-pool JSON a worker added rides along as a pool file; only the
        # YAML lines are scenarios the sim and oracle steps run.
        paths = [p for p in listed if p.lower().endswith((".yaml", ".yml"))]
        pool_files = [p for p in listed if p not in paths]
        if not paths:
            attempts = [c.attempt(ctx) for c in calls[:-1]] + [call.attempt(ctx, outcome="escalated")]
            reason = f"{call.family} found no legal line for {subject}: {result.get('notes') or '(no reason)'}"
            return base.outcome("ESCALATED", item=item, attempts=attempts, reason=reason,
                                escalation=base.escalation(item, reason, [base.argument(
                                    call, call_value="no_legal_line", reasoning=result.get("notes", ""))],
                                    extra_history=[c.attempt_id for c in calls]))
        attempts = [c.attempt(ctx) for c in calls[:-1]] + [call.attempt(ctx, outcome="accepted")]
        data = {"scenario_paths": paths, "pool_files": pool_files, "covers": list(result.get("covers") or []),
                "author_attempt": call.attempt_id, "author_family": call.family, "author_stage": stage,
                "sim_failure": None, "prompt_evidence": None, "prompt_route": None,
                "deck_books": None, "oracle_results": None, "oracle_retry_paths": None, **(extra or {})}
        merge = MergeRequest(attempt_id=call.attempt_id, family=call.family, model=call.packet.model,
                             artifacts=dict(call.result.artifacts or {}), engine=False,
                             subject=f"card-loop: exam scenario for {subject}")
        return base.outcome(next_state, item=item, attempts=attempts, data=data, merge_request=merge,
                            reason=f"{stage} by {call.family} ({call.assignment})")
