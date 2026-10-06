"""Shared machinery for the stage executors.

Every worker call goes through `call_worker`: routed with `router.route`, run
with `workers.base.run_with_retry(worker, packet, health=ctx.health)`, in a
leased pool worktree (`ctx.pool.lease()`) or the run's own tree, with the model
and effort from `ctx.config.models[family]`. The executor then decides the
attempt's outcome and turns the call into a `ledger.attempt_from_call` row.

`ItemRecord.data` keys the executors share (JSON-serialisable; `None` means
"cleared"):

    history              attempt ids of this item, oldest first (Escalation.history)
    implementer_family   card items: the family whose implement attempt is current
    implement_attempt    card items: that attempt's id; implement_result, artifacts
    gap_id, gaps         card items parked on a gap
    review_directives    card items: the reviewer's directives after a reject
    scenario_paths       exam items: repo-relative scenarios (AUTHORING -> SIM)
    author_attempt, author_family
    deck_books           exam items: {scenario path: deck book SIM lowered it with}
    sim_failure          exam items: the failing sim-only lines (SIM -> AUTHORING)
    oracle, oracle_results, oracle_retry_paths, prompt_route, prompt_evidence
    classification, expect_ruling, base_scenario       Q&A interactions
    triage_packet, triage_first                        TRIAGE -> TERMINATION_CHECK
    termination          a two-family agreement reached before TERMINATION_CHECK
    terminal, citation, citations                      TERMINAL items
    engine_fix, engine_fix_reason, fix_attempt, fix_result

`StageDeferred` is raised when a stage cannot run NOW and must not move: no
model family can take a non-terminating call (quota exhausted), or the worker
call failed (error, or schema-invalid after its one retry). It carries the
attempts and corrections already made; the driver ledgers them, counts the
attempt against the stage's cap, and leaves the item in its state (D4: the cap,
not a single failure, escalates). Terminating calls never defer on a missing
family: D5 escalates them.
"""
from __future__ import annotations

import contextlib
import dataclasses
import json
import os
import re
from dataclasses import dataclass, field
from functools import lru_cache
from pathlib import Path
from typing import Any, Iterable, Iterator, Mapping, Sequence

from .. import corrections as corr
from ..contracts import FAMILIES, TaskPacket, WorkerResult, split_item_id
from ..driver_contracts import Escalation, ItemRecord, StageOutcome
from ..ledger import Attempt, attempt_from_call
from ..router import RoutingUnavailable, other_family, route
from ..workers.base import run_with_retry, schema_path
from . import prompts


class StageDeferred(RuntimeError):
    """The stage cannot run now; leave the item where it is (see module doc)."""

    def __init__(self, reason: str, outcome: StageOutcome | None = None):
        super().__init__(reason)
        self.reason = reason
        self.outcome = outcome


# ---------------------------------------------------------------------------
# ctx helpers


def available_families(ctx) -> tuple[str, ...]:
    """Families with a registered worker that `ctx.health` has not disabled."""
    health = getattr(ctx, "health", None)
    fams = health.available() if health is not None else FAMILIES
    workers = getattr(ctx, "workers", None) or {}
    return tuple(f for f in FAMILIES if f in fams and f in workers)


def model_for(ctx, family: str) -> tuple[str | None, str | None]:
    models = getattr(getattr(ctx, "config", None), "models", None) or {}
    m = models.get(family) or {}
    return m.get("model"), m.get("effort")


def repo_path(ctx, rel: str | os.PathLike) -> Path:
    p = Path(rel)
    return p if p.is_absolute() else Path(ctx.repo) / p


@contextlib.contextmanager
def worktree(ctx) -> Iterator[str]:
    """A leased pool worktree, else the run's working tree."""
    pool = getattr(ctx, "pool", None)
    if pool is None:
        yield str(ctx.repo)
        return
    with pool.lease() as wt:
        yield str(getattr(wt, "path", wt))


def history(item: ItemRecord, *new: str | None) -> list[str]:
    return list(item.data.get("history") or []) + [a for a in new if a]


# ---------------------------------------------------------------------------
# worker calls


@dataclass
class WorkerCall:
    stage: str
    family: str
    assignment: str
    packet: TaskPacket
    result: WorkerResult

    @property
    def attempt_id(self) -> str:
        return self.packet.attempt_id

    @property
    def ok(self) -> bool:
        return self.result.status == "ok" and isinstance(self.result.result, dict)

    @property
    def output(self) -> dict:
        return self.result.result if isinstance(self.result.result, dict) else {}

    def attempt(self, ctx, *, outcome: str | None = None, parent: str | None = None,
                notes: str | None = None) -> Attempt:
        """The ledger row. `outcome` is required for an `ok` call; a failed
        call's outcome follows its status (schema_invalid / error / quota)."""
        if self.result.status != "ok":
            outcome = None
        return attempt_from_call(self.packet, self.result, run_id=ctx.run_id,
                                 assignment=self.assignment, outcome=outcome,
                                 parent_attempt=parent, notes=notes, ts=ctx.now())


Prompt = "str | Callable[[str], str]"   # text, or attempt_id -> text (provenance stamps)


def call_worker(ctx, item: ItemRecord, *, stage: str, family: str, assignment: str, prompt,
                references: Iterable[str] = (), prompt_version: str | None = None,
                schema: str | os.PathLike | None = None) -> WorkerCall:
    """One worker call. `prompt` is the rendered text, or a function of the
    new attempt id for prompts that stamp provenance (`produced_by`)."""
    if family not in (getattr(ctx, "workers", None) or {}):
        raise RoutingUnavailable(f"no {family} worker registered for this run")
    model, effort = model_for(ctx, family)
    attempt_id = ctx.new_attempt_id()
    text = prompt(attempt_id) if callable(prompt) else prompt
    retry_kwargs = {}
    if callable(getattr(ctx, "sleep", None)):
        retry_kwargs["sleep"] = ctx.sleep
    with worktree(ctx) as wt:
        packet = TaskPacket(
            stage=stage, family=family, item=item.item, attempt_id=attempt_id, prompt=text,
            prompt_version=prompt_version or prompts.version(stage),
            schema_path=str(schema or schema_path(stage)), worktree=wt,
            references=tuple(dict.fromkeys(r for r in references if r)),
            model=model, effort=effort, budget_usd=_budget(ctx, stage),
        )
        result = run_with_retry(ctx.workers[family], packet, health=getattr(ctx, "health", None),
                                **retry_kwargs)
    return WorkerCall(stage=stage, family=family, assignment=assignment, packet=packet, result=result)


def _budget(ctx, stage: str) -> float | None:
    budgets = getattr(getattr(ctx, "config", None), "stage_budget_usd", None)
    if isinstance(budgets, Mapping):
        return budgets.get(stage)
    return None


def routed_family(ctx, stage: str, item: ItemRecord, *, implementer: str | None = None,
                  ) -> tuple[str, str]:
    return route(stage, item.item, config=ctx.config, scorecard=getattr(ctx, "scorecard", None),
                 implementer_family=implementer, available=available_families(ctx),
                 prompt_version=prompts.version(stage))


def routed_call(ctx, item: ItemRecord, *, stage: str, prompt: str, references: Iterable[str] = (),
                implementer: str | None = None, avoid: str | None = None,
                ) -> tuple[WorkerCall, list[WorkerCall]]:
    """One routed call, re-routed once if its vendor turns out exhausted.

    `implementer` forces the other family (author_interaction, D11 `forced`).
    `avoid` names a family that must not take the call (a reviewer may not be
    the implementer): a route landing on it is moved to the other family as
    `forced`. Raises `StageDeferred` when no allowed family can take it.
    Returns (final call, every call made).
    """
    refs = tuple(references)
    calls: list[WorkerCall] = []
    for _ in range(2):
        try:
            family, assignment = routed_family(ctx, stage, item, implementer=implementer)
        except RoutingUnavailable as e:
            raise deferred(ctx, item, calls, str(e)) from None
        if avoid and family == avoid:
            family, assignment = other_family(avoid), "forced"
            if family not in available_families(ctx):
                raise deferred(ctx, item, calls,
                               f"{stage} for {item.item} needs {family} (the {avoid} attempt may "
                               f"not be judged by its own family) and {family} is unavailable")
        call = call_worker(ctx, item, stage=stage, family=family, assignment=assignment,
                           prompt=prompt, references=refs)
        calls.append(call)
        if call.result.status != "quota_exhausted":
            return call, calls
    return calls[-1], calls


def deferred(ctx, item: ItemRecord, calls: Sequence[WorkerCall], reason: str) -> StageDeferred:
    """A `StageDeferred` carrying the attempts (and schema corrections) made so far."""
    corrections = [
        corr.schema_invalid(c.attempt_id, stage=c.stage, item=item.item,
                            detail=(c.result.error or "")[:2000], ts=ctx.now())
        for c in calls if c.result.status == "schema_invalid"
    ]
    outcome = StageOutcome(next_state=item.state, reason=reason,
                           attempts=[c.attempt(ctx) for c in calls], corrections=corrections,
                           data={"history": history(item, *(c.attempt_id for c in calls))})
    return StageDeferred(reason, outcome)


def defer_failed(ctx, item: ItemRecord, calls: Sequence[WorkerCall]) -> StageDeferred:
    """The final call failed (error, quota, or schema-invalid after its one
    retry): ledger it, correct a schema failure, and leave the item in place."""
    final = calls[-1]
    return deferred(ctx, item, calls,
                    f"{final.stage} worker {final.family} returned {final.result.status}: "
                    f"{(final.result.error or '')[:300]}")


def escalation(item: ItemRecord, reason: str, arguments: Iterable[Mapping] = (),
               extra_history: Iterable[str] = ()) -> Escalation:
    hist = tuple(item.data.get("history") or ()) + tuple(a for a in extra_history if a)
    return Escalation(item=item.item, reason=reason, arguments=tuple(dict(a) for a in arguments),
                      history=tuple(dict.fromkeys(hist)))


def argument(call: WorkerCall | None, *, call_value, citation=None, reasoning: str = "",
             family: str | None = None, attempt_id: str | None = None) -> dict:
    """An Escalation argument: {"family", "attempt_id", "call", "citation", "reasoning"}."""
    return {"family": call.family if call else family,
            "attempt_id": call.attempt_id if call else attempt_id,
            "call": call_value, "citation": citation, "reasoning": reasoning}


# ---------------------------------------------------------------------------
# paths (canonical, repo-relative, POSIX)


def card_set(card_id: str) -> str:
    """`BT7-056` -> `BT7`, `P-130` -> `P`, `ST23-04` -> `ST23`."""
    return card_id.rsplit("-", 1)[0]


def bundle_path(card_id: str) -> str:
    return f"data/card_bundles/{card_id}.md"


def spec_path(card_id: str) -> str:
    return f"code/digimon-engine/cards/{card_set(card_id).lower()}/{card_id}.yaml"


def test_path(card_id: str) -> str:
    return (f"code/digimon-engine/tests/cards_behavioral/{card_set(card_id).lower()}/"
            f"{card_id.lower().replace('-', '_')}.rs")


def scenario_dir(card_id: str) -> str:
    return f"qa/dcgo-exams/{card_set(card_id)}"


def clause_parts(clause_id: str) -> tuple[str, str, str]:
    card, zone, idx = clause_id.split("#")
    return card, zone, idx


def clause_scenario_path(clause_id: str) -> str:
    """`ST23-04#effect#0` -> `qa/dcgo-exams/ST23/ST23-04-effect0.yaml` (the library's naming)."""
    card, zone, idx = clause_parts(clause_id)
    return f"{scenario_dir(card)}/{card}-{zone}{idx}.yaml"


def _slug(text: str) -> str:
    return re.sub(r"[^A-Za-z0-9]+", "-", text).strip("-")


def interaction_scenario_path(interaction_id: str, primary_card: str) -> str:
    """`qa:Q1601` -> `<dir>/<card>-qa-Q1601.yaml`;
    `probe:BT7-056#effect#0:scope:neg` -> `<dir>/BT7-056-effect0-scope-neg.yaml`."""
    if interaction_id.startswith("probe:"):
        _, clause, *rest = interaction_id.split(":")
        card, zone, idx = clause_parts(clause)
        return f"{scenario_dir(card)}/{card}-{zone}{idx}-{'-'.join(_slug(r) for r in rest)}.yaml"
    return f"{scenario_dir(primary_card)}/{primary_card}-{_slug(interaction_id)}.yaml"


def notes_files(ctx, card_id: str) -> list[str]:
    d = repo_path(ctx, scenario_dir(card_id))
    if not d.is_dir():
        return []
    return sorted(f"{scenario_dir(card_id)}/{p.name}" for p in d.glob("NOTES*.md"))


def card_name(ctx, card_id: str) -> str:
    """From the official bundle's title line (`# Dorumon  (BT7-056)`)."""
    p = repo_path(ctx, bundle_path(card_id))
    try:
        first = p.read_text(encoding="utf-8").splitlines()[0]
    except (OSError, IndexError):
        return card_id
    m = re.match(r"#\s*(.+?)\s*\(" + re.escape(card_id) + r"\)", first)
    return m.group(1).strip() if m else card_id


def _card_number(card_id: str) -> int:
    m = re.search(r"-(\d+)$", card_id)
    return int(m.group(1)) if m else 0


def example_specs(ctx, card_id: str, n: int = 2) -> list[str]:
    """The `n` nearest existing YAML specs of the card's set (by card number),
    falling back to `cards/_examples/`."""
    set_dir = repo_path(ctx, f"code/digimon-engine/cards/{card_set(card_id).lower()}")
    mine = _card_number(card_id)
    found = []
    if set_dir.is_dir():
        for p in set_dir.glob("*.yaml"):
            if p.stem == card_id:
                continue
            found.append((abs(_card_number(p.stem) - mine), p.stem,
                          f"code/digimon-engine/cards/{card_set(card_id).lower()}/{p.name}"))
    out = [rel for _, _, rel in sorted(found)[:n]]
    if len(out) < n:
        ex = repo_path(ctx, "code/digimon-engine/cards/_examples")
        if ex.is_dir():
            out += [f"code/digimon-engine/cards/_examples/{p.name}" for p in sorted(ex.glob("*.yaml"))
                    ][: n - len(out)]
    return out


def dcgo_script(ctx, card_id: str) -> str | None:
    """Absolute path of the card's DCGO C# in the BASE repo's DCGO (rule 29)."""
    from .. import preflight

    root = getattr(getattr(ctx, "config", None), "dcgo_root", None) or getattr(ctx, "dcgo_root", None)
    root = preflight.resolve_dcgo_root(root)
    if root is None:
        return None
    rel = preflight.dcgo_script_presence([card_id], root).get(card_id)
    return str(Path(root) / rel) if rel else None


def dcgo_script_text(ctx, card_id: str) -> str:
    return dcgo_script(ctx, card_id) or f"(no DCGO script found for {card_id})"


# ---------------------------------------------------------------------------
# committed data (overridable through `ctx.sources` for tests)


DEFAULT_CLAUSE_TEXT_JSON = "qa/exam-clause-text.json"


def _sources(ctx):
    return getattr(ctx, "sources", None)


@lru_cache(maxsize=8)
def _clause_text_file(path: str, mtime: float) -> dict:
    data = json.loads(Path(path).read_text(encoding="utf-8"))
    return {c["id"]: c for c in data.get("clauses") or []}


def clause_text_json(ctx) -> str:
    """The clause-text file the oracle records verdicts against: the run's own
    (`ctx.clause_text_json` / `plan["clause_text_json"]`), else the committed one."""
    plan = getattr(ctx, "plan", None)
    for v in (getattr(ctx, "clause_text_json", None),
              plan.get("clause_text_json") if isinstance(plan, Mapping) else None,
              getattr(getattr(ctx, "config", None), "clause_text_json", None)):
        if v:
            return str(v)
    return DEFAULT_CLAUSE_TEXT_JSON


def clauses_for(ctx, card_ids: Sequence[str]) -> list[dict]:
    """Extracted clause records (`tools.clause_coverage`) for `card_ids`."""
    src = _sources(ctx)
    if src is not None and getattr(src, "clauses", None):
        return list(src.clauses(list(card_ids)))
    out: list[dict] = []
    p = repo_path(ctx, clause_text_json(ctx))
    known: dict = {}
    if p.is_file():
        known = _clause_text_file(str(p), p.stat().st_mtime)
    missing = []
    for cid in card_ids:
        rows = [c for c in known.values() if c.get("card_id") == cid]
        if rows:
            out.extend(rows)
        else:
            missing.append(cid)
    if missing:
        from ..interactions.denominator import extract_clauses

        out.extend(extract_clauses(missing))
    return out


def clause_record(ctx, clause_id: str) -> dict | None:
    card = clause_id.split("#", 1)[0]
    for c in clauses_for(ctx, [card]):
        if c.get("id") == clause_id:
            return c
    return None


def card_qa(ctx) -> Mapping:
    src = _sources(ctx)
    if src is not None and getattr(src, "card_qa", None):
        return src.card_qa()
    from ..interactions.denominator import load_card_qa

    return _cached_json_loader(str(repo_path(ctx, "data/card_qa.json")), load_card_qa)


def denominator(ctx) -> Mapping:
    src = _sources(ctx)
    if src is not None and getattr(src, "denominator", None):
        return src.denominator()
    p = repo_path(ctx, "data/interaction_denominator.json")
    if not p.is_file():
        return {}
    from ..interactions.denominator import load_denominator

    return _cached_json_loader(str(p), load_denominator)


_LOADED: dict = {}


def _cached_json_loader(path: str, loader):
    p = Path(path)
    key = (path, p.stat().st_mtime if p.exists() else None)
    if key not in _LOADED:
        _LOADED[key] = loader(p)
    return _LOADED[key]


# ---------------------------------------------------------------------------
# items


def clause_of(item: ItemRecord) -> str:
    kind, ident = split_item_id(item.item)
    if kind != "clause":
        raise ValueError(f"{item.item} is not a clause item")
    return ident


def interaction_of(item: ItemRecord) -> str:
    kind, ident = split_item_id(item.item)
    if kind != "interaction":
        raise ValueError(f"{item.item} is not an interaction item")
    return ident


def interaction_cards(ctx, interaction_id: str) -> list[str]:
    """Every card an interaction counts for, primary first."""
    if interaction_id.startswith("probe:"):
        return [interaction_id.split(":")[1].split("#")[0]]
    if interaction_id.startswith("qa:"):
        q = interaction_id[3:]
        ruling = (card_qa(ctx).get("qa") or {}).get(q) or {}
        cards = list(ruling.get("card_ids") or [])
        entry = (denominator(ctx).get("interactions") or {}).get(interaction_id) or {}
        for c in entry.get("card_ids") or []:
            if c not in cards:
                cards.append(c)
        return cards
    entry = (denominator(ctx).get("interactions") or {}).get(interaction_id) or {}
    return list(entry.get("card_ids") or [])


def item_cards(ctx, item: ItemRecord) -> list[str]:
    kind, ident = split_item_id(item.item)
    if kind == "card":
        return [ident]
    if kind == "clause":
        return [ident.split("#", 1)[0]]
    return interaction_cards(ctx, ident)


def implementer_family(ctx, card_id: str) -> str | None:
    """The family that implemented `card_id`: the run's card item
    (`ctx.items["card:<id>"].data["implementer_family"]`), else the attempt
    ledger's last accepted `implement` for it, else the spec's
    `# produced_by:` header resolved through the ledger."""
    items = getattr(ctx, "items", None)
    if items:
        rec = items.get(f"card:{card_id}")
        data = getattr(rec, "data", None) if rec is not None else None
        if data is None and isinstance(rec, Mapping):
            data = rec.get("data")
        fam = (data or {}).get("implementer_family")
        if fam in FAMILIES:
            return fam
    attempts = _ledger_attempts(ctx)
    last = None
    for a in attempts:
        if a.stage == "implement" and a.item == f"card:{card_id}" and a.outcome == "accepted":
            last = a
    if last is not None and last.family in FAMILIES:
        return last.family
    from ..provenance import yaml_produced_by

    p = repo_path(ctx, spec_path(card_id))
    if p.is_file():
        aid = yaml_produced_by(p.read_text(encoding="utf-8"))
        for a in attempts:
            if a.attempt_id == aid and a.family in FAMILIES:
                return a.family
    return None


def _ledger_attempts(ctx) -> list:
    ledger = getattr(ctx, "ledger", None)
    if ledger is None:
        return []
    try:
        attempts, _ = ledger.read()
    except (OSError, ValueError):
        return []
    return list(attempts)


def json_block(value) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False, default=str)


def bullet_list(values: Iterable[str], *, empty: str = "(none)") -> str:
    vals = [f"`{v}`" for v in values if v]
    return ", ".join(vals) if vals else empty


def outcome(next_state: str, *, item: ItemRecord, reason: str = "",
            attempts: Sequence[Attempt] = (), data: Mapping | None = None, **kwargs) -> StageOutcome:
    """A StageOutcome whose `data["history"]` gains every attempt's id."""
    d = dict(data or {})
    d["history"] = history(item, *(a.attempt_id for a in attempts))
    return StageOutcome(next_state=next_state, reason=reason, attempts=list(attempts), data=d, **kwargs)
