"""Stage executors of the card-authoring loop (design D1, D4-D7, D10-D14).

The driver looks an item's state up in `EXECUTORS` and calls
`run(ctx, item) -> StageOutcome` (`driver_contracts.StageExecutor`). States
without an entry are the driver's own: PENDING (planning), GATE (the fix gate)
and the terminal states.

    card          IMPLEMENTING -> REVIEW | PARKED          implement.py
                  REVIEW -> IMPLEMENTED | IMPLEMENTING      review.py
    clause /      CLASSIFY -> ENCODE | AUTHORING | TERMINATION_CHECK | ESCALATED   classify.py
    interaction   ENCODE -> AUTHORING | ESCALATED           encode.py
                  AUTHORING -> SIM | ESCALATED              authoring.py
                  SIM -> ORACLE | AUTHORING                 sim.py      (no model)
                  ORACLE -> CONFIRMED | DIVERGED | ORACLE | AUTHORING | ESCALATED
                                                            oracle.py   (no model)
                  DIVERGED -> TRIAGE                        triage.py   (no model)
                  TRIAGE -> FIX | TERMINATION_CHECK | ESCALATED       triage.py
                  TERMINATION_CHECK -> TERMINAL | ESCALATED termination.py
                  FIX -> GATE | ESCALATED                   fix.py

An executor that cannot run now raises `StageDeferred` (see `base`): the item
stays in its state and the attempts it carries are ledgered.
"""
from __future__ import annotations

from .authoring import AuthoringExecutor
from .base import StageDeferred
from .classify import ClassifyExecutor
from .encode import EncodeExecutor
from .fix import FixExecutor
from .implement import ImplementExecutor
from .oracle import OracleExecutor
from .review import ReviewExecutor
from .sim import SimExecutor
from .termination import TerminationCheckExecutor
from .triage import DivergedExecutor, TriageExecutor


def build_executors() -> dict:
    """A fresh `{state: executor}` map (executors are stateless; this is for tests
    that want their own instances)."""
    executors = (ImplementExecutor(), ReviewExecutor(), ClassifyExecutor(), EncodeExecutor(),
                 AuthoringExecutor(), SimExecutor(), OracleExecutor(), DivergedExecutor(),
                 TriageExecutor(), TerminationCheckExecutor(), FixExecutor())
    return {e.state: e for e in executors}


EXECUTORS: dict = build_executors()

__all__ = ["EXECUTORS", "StageDeferred", "build_executors"]
