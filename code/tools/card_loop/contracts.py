"""Shared types every card-loop module agrees on.

Kept deliberately small: stage and family names, item ids, and the worker
packet / result / usage shapes (design D10). Anything module-specific lives in
that module.
"""
from __future__ import annotations

from dataclasses import dataclass, field

FAMILIES = ("claude", "codex")

# Judgment tasks a worker can be asked to do (design D1, D4, D6, D7).
STAGES = (
    "implement",           # author a card's YAML DSL spec + behavioral tests (rule 28)
    "review",              # review another attempt's artifacts
    "author_clause",       # one exam scenario per printed clause (breadth)
    "author_interaction",  # adversarial interaction exam (depth)
    "triage",              # classify a divergence: ours_wrong | dcgo_quirk | undetermined
    "classify_qa",         # official ruling: behavioral | textual | not_examinable
    "encode_ruling",       # write the expect_ruling assertion for a Q&A interaction
    "fix_card",            # card / YAML fix under the fix gate
    "fix_engine",          # engine / DSL fix under the fix gate (own branch, human merge)
)

# Calls that can END an item without our engine changing need agreement from
# both families (design D5): a dcgo_quirk / unreachable triage, a textual or
# not_examinable Q&A classification, and the ruling encoding itself.
TERMINATING_STAGES = ("triage", "classify_qa", "encode_ruling")

ITEM_KINDS = ("card", "clause", "interaction")


def item_id(kind: str, ident: str) -> str:
    """`card:BT7-056`, `clause:BT7-056#effect#0`, `interaction:qa:Q1601`."""
    if kind not in ITEM_KINDS:
        raise ValueError(f"unknown item kind {kind!r}; expected one of {ITEM_KINDS}")
    return f"{kind}:{ident}"


def split_item_id(item: str) -> tuple[str, str]:
    kind, _, ident = item.partition(":")
    if kind not in ITEM_KINDS or not ident:
        raise ValueError(f"malformed item id {item!r}")
    return kind, ident


@dataclass(frozen=True)
class Usage:
    input_tokens: int = 0
    output_tokens: int = 0
    cache_read_tokens: int = 0
    cache_write_tokens: int = 0
    cost_usd: float | None = None
    # True when cost was computed from the config price table rather than
    # reported by the CLI (Codex reports tokens only).
    cost_derived: bool = False
    wall_seconds: float = 0.0


@dataclass(frozen=True)
class TaskPacket:
    stage: str
    family: str
    item: str
    attempt_id: str
    prompt: str                       # rendered prompt template text
    prompt_version: str
    schema_path: str                  # JSON schema the result must satisfy
    worktree: str                     # driver-managed worktree the worker runs in
    references: tuple[str, ...] = ()  # files the worker should read (not inlined)
    model: str | None = None
    effort: str | None = None
    budget_usd: float | None = None

    def __post_init__(self):
        if self.stage not in STAGES:
            raise ValueError(f"unknown stage {self.stage!r}")
        if self.family not in FAMILIES:
            raise ValueError(f"unknown family {self.family!r}")


WORKER_STATUSES = ("ok", "schema_invalid", "error", "quota_exhausted")


@dataclass(frozen=True)
class WorkerResult:
    status: str
    result: dict | None = None
    # {"diff": <path to a binary git diff vs the pinned base>,
    #  "manifest": <path to a JSON manifest of touched files + pasted test lines>}
    artifacts: dict = field(default_factory=dict)
    usage: Usage = field(default_factory=Usage)
    error: str | None = None
    transcript_path: str | None = None

    def __post_init__(self):
        if self.status not in WORKER_STATUSES:
            raise ValueError(f"unknown worker status {self.status!r}")
