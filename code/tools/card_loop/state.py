"""A card-loop run's item state (design D4, task 6.1).

**Items.** Built from the frozen `plan.json`'s work set and the committed ledgers:

- one `card:<ID>` per pool card with no YAML spec (the planner's `implement`);
- one `clause:<clause id>` per extracted clause of every pool card -- the
  planner's `exam` and `skipped` entries for cards with a spec
  (`tools.clause_coverage.campaign.build_plan_for_pool`), plus the clauses of
  the cards still to implement (`exam_binding.bind`), which wait on their card
  item (`ItemMeta.requires`) so the denominator is honest from the start;
- one `interaction:<id>` per GATING interaction of a pool card
  (`data/interaction_denominator.json`); a shared ruling is ONE item counted for
  every pool card that prints it. Non-gating interactions are not items.

**Initial states** come from the committed ledgers: a stored `confirmed`
verdict -> CONFIRMED; `diverged` triaged `dcgo_quirk` with a citation, or
`unreachable` with a reason -> TERMINAL; a stored `unavailable` -> UNAVAILABLE;
an open escalation file -> ESCALATED; any other `diverged` -> DIVERGED (its
triage carried in `data`); otherwise PENDING. A card in the plan's
`dcgo.unavailable` makes its not-yet-adjudicated clause items -- and the
interactions whose every pool card is unavailable -- UNAVAILABLE.

**Events.** Every transition is one `driver_contracts.Event` row in
`<run_dir>/events.jsonl`, appended BEFORE memory changes. Driver bookkeeping
rides in `Event.data["_driver"]`: `meta`/`data` on an item's creation row,
`item_data` (the delta merged into `ItemRecord.data`), `count` (attempt
counters to bump) and `reset_attempts` on later rows. A row whose `src == dst`
is a note (a merge failure, a refused outcome): the item stayed put but an
attempt may have been spent.

**Resume** = ledgers first, events second: the ledger build gives each item an
origin state, the event log replays on top (an item's creation row restores
the run's starting point), then reconciliation applies cross-run truth --
a committed adjudication beats a stale in-run state, a deleted escalation file
withdraws the escalation (attempts reset), a PENDING card whose spec now exists
is IMPLEMENTED. A torn or damaged line is skipped and reported in `problems`.
"""
from __future__ import annotations

import json
import os
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable, Iterable, Mapping

from ._jsonl import append_row, read_rows, utc_now_iso
from .config import LoopConfig
from .contracts import split_item_id
from .driver_contracts import (
    ADJUDICATED_STATES,
    Event,
    ItemRecord,
    check_transition,
    is_adjudicated,
)
from .escalations import load_escalated_items
from .interactions.probes import natural_key

STATE_VERSION = 1
DRIVER_KEY = "_driver"
EVENTS_NAME = "events.jsonl"
SNAPSHOT_NAME = "state.json"
#: Item data that belongs to one pass through the exam stages and is cleared
#: when a withdrawn escalation re-enters the item (resume reconciliation).
EXAM_SCOPED_DATA = ("scenario_paths", "pool_files", "author_attempt", "author_family", "author_stage",
                    "expect_ruling", "encode_feedback", "triage_feedback", "encode_attempts", "base_scenario",
                    "sim_failure", "sim_notes", "deck_books", "ruling_contradicted", "ruling_block_written",
                    "prompt_evidence", "prompt_route", "oracle", "oracle_results", "oracle_retry_paths",
                    "triage_packet", "triage_first", "termination", "escalation", "escalation_reason")
KIND_ORDER = {"card": 0, "clause": 1, "interaction": 2}
COUNT_KEYS = ("confirmed", "terminal", "unavailable", "escalated", "unmeasured",
              "implemented", "parked")


# ---------------------------------------------------------------------------
# where the committed ledgers live
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class LedgerPaths:
    cards_dir: Path            # code/digimon-engine/cards (YAML specs)
    scenarios_dir: Path        # qa/dcgo-exams
    verdicts_dir: Path         # qa/qa-reports/exam-verdicts
    denominator_path: Path     # data/interaction_denominator.json
    escalations_dir: Path      # qa/card-loop/escalations

    @classmethod
    def for_repo(cls, repo: str | os.PathLike, config: LoopConfig | None = None) -> "LedgerPaths":
        repo = Path(repo)
        esc = Path((config or LoopConfig()).escalations_dir)
        return cls(
            cards_dir=repo / "code" / "digimon-engine" / "cards",
            scenarios_dir=repo / "qa" / "dcgo-exams",
            verdicts_dir=repo / "qa" / "qa-reports" / "exam-verdicts",
            denominator_path=repo / "data" / "interaction_denominator.json",
            escalations_dir=esc if esc.is_absolute() else repo / esc,
        )


# ---------------------------------------------------------------------------
# per-item metadata and ledger origin
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class ItemMeta:
    """Scheduling facts (frozen per run; logged on the item's creation row)."""

    cards: tuple = ()          # the pool cards the item counts for
    is_core: bool = False
    rank: int = 0              # best plan-ranking position among `cards`
    requires: tuple = ()       # card items that must be IMPLEMENTED first
    source: str | None = None  # interactions: "qa" | "probe" | ...

    def to_dict(self) -> dict:
        return {"cards": list(self.cards), "is_core": self.is_core, "rank": self.rank,
                "requires": list(self.requires), "source": self.source}

    @classmethod
    def from_dict(cls, d: Mapping) -> "ItemMeta":
        return cls(cards=tuple(d.get("cards") or ()), is_core=bool(d.get("is_core")),
                   rank=int(d.get("rank") or 0), requires=tuple(d.get("requires") or ()),
                   source=d.get("source"))


@dataclass(frozen=True)
class Origin:
    """What the committed ledgers say about an item. `strong` = a cross-run
    fact (a committed adjudication, an open escalation) that beats a stale
    in-run state on resume; a weak origin only seeds a new item."""

    state: str
    reason: str
    data: dict = field(default_factory=dict)
    strong: bool = False


def _nonblank(v) -> bool:
    return isinstance(v, str) and bool(v.strip())


def exam_origin(verdict: str, stored: Mapping | None, *, escalation: Path | None = None,
                plan_unavailable: bool = False) -> Origin:
    """A clause / interaction item's ledger origin (see the module docstring)."""
    stored = dict(stored or {})
    data: dict = {}
    if stored.get("scenario_path"):
        data["scenario_path"] = stored["scenario_path"]
    reason = stored.get("reason")
    origin: Origin | None = None
    if verdict == "confirmed":
        origin = Origin("CONFIRMED", "ledger: confirmed", {**data, "verdict": "confirmed"}, True)
    elif verdict == "unavailable":
        origin = Origin("UNAVAILABLE", f"ledger: unavailable -- {reason or 'no reason recorded'}",
                        {**data, "verdict": "unavailable", "reason": reason,
                         "unavailable_source": "verdict"}, True)
    elif verdict == "unreachable" and _nonblank(reason):
        origin = Origin("TERMINAL", "ledger: unreachable with a measured reason",
                        {**data, "verdict": "unreachable", "terminal": "unreachable",
                         "reason": reason}, True)
    elif verdict == "diverged":
        triage, citation = stored.get("triage"), stored.get("citation")
        if triage == "dcgo_quirk" and _nonblank(citation):
            origin = Origin("TERMINAL", "ledger: diverged, triaged dcgo_quirk with a citation",
                            {**data, "verdict": "diverged", "terminal": "dcgo_quirk",
                             "triage": triage, "citation": citation, "reason": reason}, True)
        else:
            data.update(verdict="diverged", triage=triage, citation=citation, reason=reason)
    elif verdict == "unreachable":
        data.update(verdict="unreachable", note="stored unreachable without a reason")
    if origin is not None:
        return origin
    if escalation is not None:
        return Origin("ESCALATED", f"ledger: open escalation {escalation.name}",
                      {**data, "escalation": escalation.name}, True)
    if plan_unavailable:
        return Origin("UNAVAILABLE", "plan: DCGO has no script for this item's card(s)",
                      {**data, "unavailable_source": "plan",
                       "unavailable_reason": "DCGO has no script (plan.dcgo.unavailable)"})
    if data.get("verdict") == "diverged":
        return Origin("DIVERGED", "ledger: diverged, not adjudicated", data)
    return Origin("PENDING", f"ledger: {verdict or 'unmeasured'}", data)


def card_origin(card: str, *, has_spec: bool, escalation: Path | None = None) -> Origin:
    if escalation is not None:
        return Origin("ESCALATED", f"ledger: open escalation {escalation.name}",
                      {"escalation": escalation.name}, True)
    if has_spec:
        return Origin("IMPLEMENTED", "ledger: the card has a YAML spec")
    return Origin("PENDING", "ledger: no YAML spec")


def _yaml_ids(cards_dir: Path) -> set[str]:
    return {p.stem for p in cards_dir.rglob("*.yaml")} if cards_dir.is_dir() else set()


# ---------------------------------------------------------------------------
# building items from the plan + ledgers
# ---------------------------------------------------------------------------


@dataclass
class ItemSeed:
    item: str
    meta: ItemMeta
    origin: Origin
    data: dict = field(default_factory=dict)   # static facts for executors


def build_items(plan: Mapping, paths: LedgerPaths, *,
                escalated: Mapping[str, Path] | None = None) -> list[ItemSeed]:
    """Every item the plan implies, with its ledger origin (deterministic order)."""
    from tools.clause_coverage.campaign import build_plan_for_pool
    from tools.clause_coverage.exam_binding import (
        bind,
        bind_interactions,
        load_interaction_verdicts,
        load_verdict_store,
    )

    escalated = dict(escalated or {})
    ws = plan["work_set"]
    pool = list(ws["pool"])
    core = set(ws.get("core") or ())
    ranking = list(ws.get("ranking") or pool)
    rank = {c: i for i, c in enumerate(ranking)}
    unranked = len(rank)
    plan_unavailable = set((plan.get("dcgo") or {}).get("unavailable") or ())

    def card_rank(cards: Iterable[str]) -> int:
        return min((rank.get(c, unranked) for c in cards), default=unranked)

    exam_plan = build_plan_for_pool(pool, sorted(core), cards_dir=paths.cards_dir,
                                    scenarios_dir=paths.scenarios_dir,
                                    verdicts_path=paths.verdicts_dir, ranking=ranking)
    implement = list(exam_plan["implement"])
    to_implement = set(implement)
    store = load_verdict_store(paths.verdicts_dir)
    seeds: list[ItemSeed] = []

    for card in implement:
        item = f"card:{card}"
        seeds.append(ItemSeed(item, ItemMeta(cards=(card,), is_core=card in core, rank=card_rank([card])),
                              card_origin(card, has_spec=False, escalation=escalated.get(item)),
                              {"card_ids": [card]}))

    clause_rows: list[tuple[str, str, str]] = []           # (card, clause id, verdict)
    for e in exam_plan["exam"]:
        clause_rows.append((e["card_id"], e["clause_id"], e.get("verdict") or "unmeasured"))
    for e in exam_plan["skipped"]:
        clause_rows.append((e["card_id"], e["clause_id"],
                            "confirmed" if e.get("reason") == "confirmed" else "unavailable"))
    if implement:
        bound = bind(implement, paths.scenarios_dir, paths.verdicts_dir)
        for card in implement:
            for c in bound["cards"].get(card, {}).get("clauses", []):
                clause_rows.append((card, c["clause_id"], c.get("verdict") or "unmeasured"))
    for card, clause_id, verdict in clause_rows:
        item = f"clause:{clause_id}"
        stored = store.get(clause_id) if verdict != "unmeasured" else None
        if verdict == "unavailable" and not stored:
            stored = {"reason": "DCGO has no script for this card"}
        origin = exam_origin(verdict, stored, escalation=escalated.get(item),
                             plan_unavailable=card in plan_unavailable)
        meta = ItemMeta(cards=(card,), is_core=card in core, rank=card_rank([card]),
                        requires=(f"card:{card}",) if card in to_implement else ())
        seeds.append(ItemSeed(item, meta, origin, {"card_ids": [card]}))

    bound_i = bind_interactions(pool, paths.denominator_path, paths.verdicts_dir, gating_only=True)
    raw_i = load_interaction_verdicts(paths.verdicts_dir)
    cards_of: dict[str, list[str]] = {}
    row_of: dict[str, dict] = {}
    for card in pool:
        for x in bound_i["cards"].get(card, {}).get("interactions", []):
            cards_of.setdefault(x["interaction_id"], []).append(card)
            row_of[x["interaction_id"]] = x
    for iid, cards in cards_of.items():
        item = f"interaction:{iid}"
        x = row_of[iid]
        verdict = x.get("verdict") or "unmeasured"
        stored = raw_i.get(iid) if verdict != "unmeasured" else None
        origin = exam_origin(verdict, stored, escalation=escalated.get(item),
                             plan_unavailable=all(c in plan_unavailable for c in cards))
        meta = ItemMeta(cards=tuple(cards), is_core=any(c in core for c in cards),
                        rank=card_rank(cards),
                        requires=tuple(f"card:{c}" for c in cards if c in to_implement),
                        source=x.get("source"))
        seeds.append(ItemSeed(item, meta, origin, {"card_ids": list(cards), "source": x.get("source")}))

    seeds.sort(key=lambda s: priority_key(s.item, s.meta))
    return seeds


def priority_key(item: str, meta: ItemMeta) -> tuple:
    """Core first, then plan ranking, then card before clause before
    interaction, then the id in natural order (`#effect#2` before `#effect#10`)."""
    kind, ident = split_item_id(item)
    return (not meta.is_core, meta.rank, KIND_ORDER[kind], natural_key(ident))


# ---------------------------------------------------------------------------
# the run state
# ---------------------------------------------------------------------------


class RunState:
    def __init__(self, run_id: str, run_dir: str | os.PathLike, *,
                 now: Callable[[], str] = utc_now_iso, write: bool = True):
        self.run_id = run_id
        self.run_dir = Path(run_dir)
        self.events_path = self.run_dir / EVENTS_NAME
        self.now = now
        self.write = write
        self.records: dict[str, ItemRecord] = {}
        self.meta: dict[str, ItemMeta] = {}
        self.origin: dict[str, Origin] = {}
        self.problems: list[str] = []
        self._logged: set[str] = set()
        self._order: list[str] | None = None
        self._escalated: dict[str, Path] = {}
        self._has_spec: set[str] = set()

    # ------------------------------------------------------------------ build

    @classmethod
    def open(cls, plan: Mapping, paths: LedgerPaths, *, run_dir: str | os.PathLike,
             now: Callable[[], str] = utc_now_iso, write: bool = True,
             run_id: str | None = None) -> "RunState":
        """Build from the ledgers, replay `events.jsonl`, reconcile, and log a
        creation row for every item the log has not seen. `write=False` is a
        read-only view (nothing is appended)."""
        escalated = load_escalated_items(paths.escalations_dir)
        seeds = build_items(plan, paths, escalated=escalated)
        return cls.from_seeds(run_id or plan["run_id"], run_dir, seeds, now=now, write=write,
                              escalated=escalated, has_spec=_yaml_ids(paths.cards_dir))

    @classmethod
    def from_seeds(cls, run_id: str, run_dir: str | os.PathLike, seeds: Iterable[ItemSeed], *,
                   now: Callable[[], str] = utc_now_iso, write: bool = True,
                   escalated: Mapping[str, Path] | None = None,
                   has_spec: Iterable[str] = ()) -> "RunState":
        """`open` without the ledger build: items given directly (driver tests,
        the end-to-end fake run). `escalated` = the open escalation files,
        `has_spec` = card ids with a YAML spec, both as resume reconciliation sees them."""
        s = cls(run_id, run_dir, now=now, write=write)
        s._escalated = dict(escalated or {})
        s._has_spec = set(has_spec)
        for seed in seeds:
            s.records[seed.item] = ItemRecord(item=seed.item, state=seed.origin.state,
                                              data={**seed.data, **seed.origin.data})
            s.meta[seed.item] = seed.meta
            s.origin[seed.item] = seed.origin
        s._replay()
        s._reconcile()
        return s

    def _orphan_origin(self, item: str) -> Origin:
        """Origin for an item known only from the event log."""
        kind, ident = split_item_id(item)
        esc = self._escalated.get(item)
        if kind == "card":
            return card_origin(ident, has_spec=ident in self._has_spec, escalation=esc)
        if esc is not None:
            return Origin("ESCALATED", f"ledger: open escalation {esc.name}", {"escalation": esc.name}, True)
        return Origin("PENDING", "ledger: not in the current plan build")

    # ------------------------------------------------------------------ events

    def _log(self, ev: Event) -> None:
        if self.write:
            append_row(self.events_path, ev.to_row())

    @staticmethod
    def _apply(rec: ItemRecord, ev: Event) -> None:
        drv = ev.data.get(DRIVER_KEY) or {}
        rec.state = ev.dst
        if drv.get("reset_attempts"):
            rec.attempts = {}
        for key in drv.get("reset") or ():          # restart dependent caps, then count
            rec.attempts.pop(key, None)
        for key in drv.get("count") or ():
            rec.attempts[key] = rec.attempts.get(key, 0) + 1
        rec.data.update(drv.get("item_data") or {})

    def _event(self, rec: ItemRecord, dst: str, *, reason: str, stage=None, attempt_id=None,
               item_data=None, event_data=None, count=(), reset_attempts=False,
               attempt_ids: Iterable[str] = (), reset_keys: Iterable[str] = (),
               extra_driver: Mapping | None = None) -> Event:
        drv = dict(extra_driver or {})
        attempt_ids = list(attempt_ids)
        if len(attempt_ids) > 1 or (attempt_ids and attempt_ids[0] != attempt_id):
            drv["attempt_ids"] = attempt_ids
        if item_data:
            drv["item_data"] = dict(item_data)
        reset_keys = [k for k in reset_keys if k in rec.attempts]
        if reset_keys:
            drv["reset"] = reset_keys
        if count:
            drv["count"] = list(count)
        if reset_attempts:
            drv["reset_attempts"] = True
        data = dict(event_data or {})
        if drv:
            data[DRIVER_KEY] = drv
        ev = Event(ts=self.now(), run_id=self.run_id, item=rec.item, src=rec.state, dst=dst,
                   reason=reason, stage=stage, attempt_id=attempt_id, data=data)
        self._log(ev)                     # durable first, then memory
        self._apply(rec, ev)
        return ev

    def transition(self, item: str, dst: str, *, reason: str, stage: str | None = None,
                   attempt_id: str | None = None, item_data: Mapping | None = None,
                   event_data: Mapping | None = None, count: Iterable[str] = (),
                   reset_attempts: bool = False, attempt_ids: Iterable[str] = (),
                   reset_keys: Iterable[str] = ()) -> Event:
        """Validate (design D4), log, then apply one transition. `attempt_ids`
        lists every worker attempt behind the step when there is more than one
        (a two-family termination check); `attempt_id` is the first.
        `reset_keys` drops attempt counters before `count` bumps others."""
        rec = self.records[item]
        check_transition(rec.kind, rec.state, dst)
        return self._event(rec, dst, reason=reason, stage=stage, attempt_id=attempt_id,
                           item_data=item_data, event_data=event_data, count=tuple(count),
                           reset_attempts=reset_attempts, attempt_ids=attempt_ids,
                           reset_keys=tuple(reset_keys))

    def note(self, item: str, *, reason: str, stage: str | None = None,
             attempt_id: str | None = None, item_data: Mapping | None = None,
             event_data: Mapping | None = None, count: Iterable[str] = (),
             attempt_ids: Iterable[str] = (), reset_keys: Iterable[str] = ()) -> Event:
        """A `src == dst` row: the item stayed put (merge failure, refused
        outcome, executor error) but data or attempt counters changed."""
        rec = self.records[item]
        return self._event(rec, rec.state, reason=reason, stage=stage, attempt_id=attempt_id,
                           item_data=item_data, event_data=event_data, count=tuple(count),
                           attempt_ids=attempt_ids, reset_keys=tuple(reset_keys))

    def _force(self, item: str, dst: str, *, reason: str, item_data=None,
               reset_attempts: bool = False) -> Event:
        """A resume reconciliation: cross-run truth, logged but not D4-checked."""
        rec = self.records[item]
        return self._event(rec, dst, reason=reason, item_data=item_data,
                           reset_attempts=reset_attempts, extra_driver={"reconciled": True})

    def _replay(self) -> None:
        rows, problems = read_rows(self.events_path)
        last_good = max((n for n, _ in rows), default=0)
        for p in problems:
            if p.line > last_good:
                self.problems.append(f"{EVENTS_NAME}: damaged last line {p.line} ignored "
                                     f"(a torn write from a crash?): {p.message}")
            else:
                self.problems.append(f"{EVENTS_NAME}: line {p.line} ignored ({p.kind}): {p.message}")
        for line, row in rows:
            try:
                ev = Event.from_row(row)
                split_item_id(ev.item)
            except (KeyError, TypeError, ValueError, AttributeError) as e:
                self.problems.append(f"{EVENTS_NAME}: line {line} is not an event ({e!r}); ignored")
                continue
            if ev.run_id != self.run_id:
                self.problems.append(f"{EVENTS_NAME}: line {line} belongs to run {ev.run_id!r}; ignored")
                continue
            drv = ev.data.get(DRIVER_KEY) or {}
            rec = self.records.get(ev.item)
            if ev.src is None:
                if rec is None:
                    rec = self.records[ev.item] = ItemRecord(item=ev.item, state=ev.dst)
                    self.origin[ev.item] = self._orphan_origin(ev.item)
                if drv.get("meta"):
                    self.meta[ev.item] = ItemMeta.from_dict(drv["meta"])
                else:
                    self.meta.setdefault(ev.item, ItemMeta())
                rec.state, rec.attempts = ev.dst, {}
                rec.data = dict(drv.get("data") or rec.data)
                self._logged.add(ev.item)
                continue
            if rec is None or ev.item not in self._logged:
                self.problems.append(f"{EVENTS_NAME}: line {line} moves {ev.item}, which has no "
                                     "creation row; ignored")
                continue
            if ev.src != rec.state:
                self.problems.append(f"{EVENTS_NAME}: line {line} moves {ev.item} from {ev.src} "
                                     f"but the replayed state is {rec.state}; applied anyway")
            self._apply(rec, ev)
        self._order = None

    def _reconcile(self) -> None:
        escalated = self._escalated
        for item in self.ordered_items():
            rec = self.records[item]
            origin = self.origin.get(item)
            if item not in self._logged:
                ev = Event(ts=self.now(), run_id=self.run_id, item=item, src=None, dst=rec.state,
                           reason=origin.reason if origin else "created",
                           data={DRIVER_KEY: {"meta": self.meta[item].to_dict(), "data": dict(rec.data)}})
                self._log(ev)
                self._logged.add(item)
                continue
            if origin is None:
                continue
            if rec.state == "ESCALATED" and item not in escalated:
                target = origin.state if origin.state != "ESCALATED" else "PENDING"
                # The item starts its exam over: the stage-scoped data of the
                # escalated attempt (an agreed block, feedback, oracle rows) must
                # not steer the new attempt. A re-entered Q&A item kept its old
                # `expect_ruling` in the second pilot and its author was told to
                # satisfy a block that no longer matched the line.
                cleared = {k: None for k in EXAM_SCOPED_DATA}
                self._force(item, target, item_data={**cleared, **dict(origin.data)}, reset_attempts=True,
                            reason="resume: escalation withdrawn (its file was deleted); "
                                   f"back to the ledger state {target}")
            elif origin.strong and rec.state != origin.state and not is_adjudicated(rec.kind, rec.state):
                self._force(item, origin.state, item_data=origin.data,
                            reason=f"resume: committed ledger says {origin.state} ({origin.reason})")
            elif rec.kind == "card" and rec.state == "PENDING" and origin.state == "IMPLEMENTED":
                self.transition(item, "IMPLEMENTED", reason="resume: the card's YAML spec exists")

    # ------------------------------------------------------------------ queries

    def ordered_items(self) -> list[str]:
        if self._order is None or len(self._order) != len(self.records):
            self._order = sorted(self.records, key=lambda i: priority_key(i, self.meta.get(i, ItemMeta())))
        return list(self._order)

    def ordered(self) -> list[ItemRecord]:
        return [self.records[i] for i in self.ordered_items()]

    def priority(self, item: str) -> tuple:
        return priority_key(item, self.meta.get(item, ItemMeta()))

    def unmet_requirements(self, item: str) -> list[str]:
        """Card items this item waits on that are not IMPLEMENTED yet. A
        required card with no item in the run (it had a spec) is met."""
        out = []
        for req in self.meta.get(item, ItemMeta()).requires:
            rec = self.records.get(req)
            if rec is not None and rec.state != "IMPLEMENTED":
                out.append(req)
        return out

    def requirements_met(self, item: str) -> bool:
        return not self.unmet_requirements(item)

    def reenter_available(self, cards: Iterable[str]) -> list[str]:
        """Items UNAVAILABLE only because the plan found no DCGO script, one of
        whose cards now has one: UNAVAILABLE -> PENDING (design D3 resume)."""
        cards = set(cards)
        moved = []
        for item in self.ordered_items():
            rec = self.records[item]
            if rec.state != "UNAVAILABLE" or rec.data.get("unavailable_source") != "plan":
                continue
            if any(c in cards for c in self.meta[item].cards):
                self.transition(item, "PENDING", reason="DCGO gained the card's script",
                                item_data={"unavailable_source": None, "unavailable_reason": None})
                moved.append(item)
        return moved

    def counts(self) -> dict:
        """`{"total"|kind: {confirmed, terminal, unavailable, escalated,
        unmeasured, implemented, parked, adjudicated, items}}`. `unmeasured` is
        everything not otherwise counted; `adjudicated` never includes ESCALATED."""
        out = {k: Counter({c: 0 for c in COUNT_KEYS}) for k in ("total", "card", "clause", "interaction")}
        for rec in self.records.values():
            s = rec.state
            if rec.kind == "card":
                key = {"IMPLEMENTED": "implemented", "PARKED": "parked",
                       "ESCALATED": "escalated"}.get(s, "unmeasured")
            else:
                key = {"CONFIRMED": "confirmed", "TERMINAL": "terminal", "UNAVAILABLE": "unavailable",
                       "ESCALATED": "escalated"}.get(s, "unmeasured")
            for k in ("total", rec.kind):
                out[k][key] += 1
        result = {}
        for k, c in out.items():
            d = {key: c[key] for key in COUNT_KEYS}
            d["adjudicated"] = d["confirmed"] + d["terminal"] + d["unavailable"] + d["implemented"]
            d["items"] = sum(c[key] for key in COUNT_KEYS)
            result[k] = d
        return result

    def by_state(self) -> dict:
        """`{kind: {state: n}}` for the report's breakdown of open work."""
        out: dict[str, Counter] = {}
        for rec in self.records.values():
            out.setdefault(rec.kind, Counter())[rec.state] += 1
        return {k: dict(sorted(v.items())) for k, v in sorted(out.items(), key=lambda kv: KIND_ORDER[kv[0]])}

    # ------------------------------------------------------------------ snapshot

    def snapshot(self, extra: Mapping | None = None) -> dict:
        doc = {"version": STATE_VERSION, "run_id": self.run_id, "written_at": self.now(),
               "counts": self.counts(), "by_state": self.by_state(), "problems": list(self.problems)}
        doc.update(dict(extra or {}))
        doc["items"] = [{**self.records[i].to_dict(), "meta": self.meta[i].to_dict()}
                        for i in self.ordered_items()]
        return doc

    def write_snapshot(self, extra: Mapping | None = None) -> Path:
        path = self.run_dir / SNAPSHOT_NAME
        path.parent.mkdir(parents=True, exist_ok=True)
        tmp = path.with_suffix(".json.tmp")
        with open(tmp, "w", encoding="utf-8", newline="\n") as f:
            json.dump(self.snapshot(extra), f, indent=2, default=str)
            f.write("\n")
        os.replace(tmp, path)
        return path


__all__ = [
    "ADJUDICATED_STATES", "DRIVER_KEY", "EVENTS_NAME", "SNAPSHOT_NAME", "ItemMeta", "ItemSeed",
    "LedgerPaths", "Origin", "RunState", "build_items", "card_origin", "exam_origin", "priority_key",
]
