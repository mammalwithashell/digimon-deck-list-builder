"""card-loop configuration: defaults from the design, overridable by a TOML file.

`load_config(path)` merges a TOML file's top-level keys over `LoopConfig()`;
nested tables (routes, models, attempt caps, prices) merge key by key.
"""
from __future__ import annotations

import tomllib
from dataclasses import dataclass, field, fields, replace

from .contracts import FAMILIES, STAGES


def _default_routes() -> dict:
    # Design D12: defaults until the scorecard has enough evidence to re-route.
    return {
        "implement": "claude",
        "review": "codex",
        "author_clause": "claude",
        "author_interaction": "claude",  # router forces the non-implementer family
        "triage": "claude",              # second opinion: the other family (D5)
        "classify_qa": "claude",
        "encode_ruling": "claude",
        "fix_card": "claude",
        "fix_engine": "codex",
    }


def _default_models() -> dict:
    return {
        "claude": {"model": "sonnet", "effort": None},
        # None = the model configured in the user's ~/.codex/config.toml.
        "codex": {"model": None, "effort": "high"},
    }


def _default_attempt_caps() -> dict:
    return {"implement": 3, "author_clause": 3, "author_interaction": 3,
            "fix_card": 2, "fix_engine": 2, "triage": 1, "classify_qa": 1,
            "encode_ruling": 2, "review": 1}


def _default_prices() -> dict:
    # USD per million tokens, used only when a CLI reports tokens but no cost.
    # None = unpriced: cost is recorded as None and flagged, never guessed.
    return {"codex": {"input": None, "cached_input": None, "output": None}}


@dataclass(frozen=True)
class LoopConfig:
    core_fraction: float = 0.7
    exploration_share: float = 0.2
    audit_rate: float = 0.05            # decided 2026-10-04
    concurrency: int = 3
    worktree_pool_size: int = 3
    plateau_attempts: int = 10
    routing_min_samples: int = 30
    late_correction_weight: float = 3.0
    seed: int = 0
    budget_usd: float | None = None
    wall_clock_hours: float | None = None
    routes: dict = field(default_factory=_default_routes)
    models: dict = field(default_factory=_default_models)
    attempt_caps: dict = field(default_factory=_default_attempt_caps)
    prices: dict = field(default_factory=_default_prices)
    cargo_target_base: str = r"D:\cargo-target"
    sccache_dir: str = r"D:\sccache"
    runs_dir: str = "runs/card-loop"
    attempts_path: str = "qa/card-loop/attempts.jsonl"
    escalations_dir: str = "qa/card-loop/escalations"
    harness_root: str | None = None     # dcgo-harness --root; None = harness default
    player_dir: str | None = None       # dcgo-harness --build
    # The stage executors' optional knobs (stages/base.py): the harness binary
    # (else DCGO_HARNESS_BIN / the per-worktree cargo target / PATH), the per-job
    # oracle wait, a per-call USD cap, the clause-text book, and the base-repo
    # DCGO root for worker references.
    harness_bin: str | None = None
    oracle_timeout_s: int = 300
    stage_budget_usd: float | None = None
    clause_text_json: str = "qa/exam-clause-text.json"
    dcgo_root: str | None = None

    def __post_init__(self):
        unknown = set(self.routes) - set(STAGES)
        if unknown:
            raise ValueError(f"routes name unknown stages: {sorted(unknown)}")
        bad = {s: f for s, f in self.routes.items() if f not in FAMILIES}
        if bad:
            raise ValueError(f"routes name unknown families: {bad}")
        for name in ("core_fraction", "exploration_share", "audit_rate"):
            if not 0.0 <= getattr(self, name) <= 1.0:
                raise ValueError(f"{name} must be within [0, 1]")


_NESTED = ("routes", "models", "attempt_caps", "prices")


def load_config(path: str | None = None) -> LoopConfig:
    base = LoopConfig()
    if not path:
        return base
    with open(path, "rb") as f:
        raw = tomllib.load(f)
    known = {f.name for f in fields(LoopConfig)}
    unknown = set(raw) - known
    if unknown:
        raise ValueError(f"unknown config keys: {sorted(unknown)}")
    overrides = {}
    for key, value in raw.items():
        if key in _NESTED:
            merged = {k: (dict(v) if isinstance(v, dict) else v) for k, v in getattr(base, key).items()}
            for k, v in value.items():
                if isinstance(v, dict) and isinstance(merged.get(k), dict):
                    merged[k] = {**merged[k], **v}
                else:
                    merged[k] = v
            overrides[key] = merged
        else:
            overrides[key] = value
    return replace(base, **overrides)
