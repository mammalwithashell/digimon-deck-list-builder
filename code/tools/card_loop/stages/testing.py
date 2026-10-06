"""Test doubles for the stage executors: a `RunContext` built from a
`SimpleNamespace`, canned worker results, and a scripted `run_command`.

Nothing here starts a process: workers are `workers.fake.FakeWorker`s and the
harness is a `FakeCommands` that answers argv patterns with canned output.
"""
from __future__ import annotations

import itertools
from dataclasses import dataclass, field
from pathlib import Path
from types import SimpleNamespace
from typing import Iterable, Mapping

from ..config import LoopConfig
from ..contracts import Usage, WorkerResult
from ..corrections import CorrectionLog
from ..ledger import AttemptLedger
from ..workers.fake import FakeWorker
from ..workers.health import VendorHealth

NOW = "2026-10-05T12:00:00Z"


def ok(result: Mapping, *, artifacts: Mapping | None = None, cost: float = 0.01) -> WorkerResult:
    return WorkerResult(status="ok", result=dict(result), artifacts=dict(artifacts or {}),
                        usage=Usage(input_tokens=100, output_tokens=10, cost_usd=cost))


def failed(status: str = "error", error: str = "boom") -> WorkerResult:
    return WorkerResult(status=status, error=error)


@dataclass
class _Rule:
    subs: tuple
    responses: list = field(default_factory=list)

    def matches(self, argv: list) -> bool:
        text = " ".join(argv)
        return all(s in text for s in self.subs)


class FakeCommands:
    """`ctx.run_command` double. `on(pattern, rc, stdout, stderr)` answers an
    argv whose joined text contains every substring of `pattern` (a str or a
    tuple of str). Rules are tried in the order first registered. Responses
    queue per rule and the last one repeats. An argv nothing matches raises
    `AssertionError`; `raise_on` makes a pattern raise `OSError`."""

    def __init__(self):
        self.calls: list[dict] = []
        self._rules: list[_Rule] = []

    def on(self, pattern, rc: int | None = 0, stdout: str = "", stderr: str = "") -> "FakeCommands":
        subs = (pattern,) if isinstance(pattern, str) else tuple(pattern)
        rule = next((r for r in self._rules if r.subs == subs), None)
        if rule is None:
            rule = _Rule(subs)
            self._rules.append(rule)
        rule.responses.append((rc, stdout, stderr))
        return self

    def raise_on(self, pattern, error: str = "not found") -> "FakeCommands":
        return self.on(pattern, rc=OSError(error))  # type: ignore[arg-type]

    def __call__(self, argv, cwd, timeout):
        self.calls.append({"argv": list(argv), "cwd": cwd, "timeout": timeout})
        for r in self._rules:
            if r.matches(list(argv)):
                resp = r.responses.pop(0) if len(r.responses) > 1 else r.responses[0]
                if isinstance(resp[0], OSError):
                    raise resp[0]
                return resp
        raise AssertionError(f"FakeCommands: no canned response for {argv}")

    def argvs(self, *subs: str) -> list[list]:
        return [c["argv"] for c in self.calls if all(s in " ".join(c["argv"]) for s in subs)]


def make_ctx(repo: str | Path, *, workers: Mapping | Iterable[FakeWorker] | None = None,
             run_command=None, config: LoopConfig | None = None, health: VendorHealth | None = None,
             items: Mapping | None = None, sources=None, plan: Mapping | None = None,
             ledger=None, pool=None, **extra) -> SimpleNamespace:
    repo = Path(repo)
    if workers is None:
        workers = {"claude": FakeWorker(family="claude"), "codex": FakeWorker(family="codex")}
    elif not isinstance(workers, Mapping):
        workers = {w.family: w for w in workers}
    counter = itertools.count(1)
    cfg = config or LoopConfig(exploration_share=0.0, harness_root="H:/root", player_dir="P:/player")
    ns = SimpleNamespace(
        run_id="run-test",
        run_dir=str(repo / "runs" / "card-loop" / "run-test"),
        repo=str(repo),
        base_sha="0" * 40,
        plan=dict(plan or {}),
        config=cfg,
        workers=dict(workers),
        health=health or VendorHealth(clock=lambda: 0.0),
        ledger=ledger if ledger is not None else AttemptLedger(repo / "attempts.jsonl"),
        corrections=CorrectionLog(repo / "corrections.jsonl"),
        scorecard=None,
        pool=pool,
        run_command=run_command if run_command is not None else FakeCommands(),
        now=lambda: NOW,
        new_attempt_id=lambda: f"att-{next(counter):04d}",
        items=dict(items or {}),
        sources=sources,
        harness_bin="dcgo-harness",
        sleep=lambda s: None,
    )
    for k, v in extra.items():
        setattr(ns, k, v)
    return ns


def sources(*, clauses: Iterable[Mapping] = (), card_qa: Mapping | None = None,
            denominator: Mapping | None = None) -> SimpleNamespace:
    """A `ctx.sources` double: committed data without reading the repo."""
    rows = [dict(c) for c in clauses]
    return SimpleNamespace(
        clauses=lambda ids: [c for c in rows if c.get("card_id") in set(ids)],
        card_qa=(lambda: card_qa) if card_qa is not None else None,
        denominator=(lambda: denominator) if denominator is not None else None,
    )
