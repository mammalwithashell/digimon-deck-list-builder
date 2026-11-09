"""Information provenance is independent of tensor/model shape compatibility."""

from __future__ import annotations
import hashlib
import json
import re
from pathlib import Path


def information_contract(manifest: dict) -> dict:
    value = manifest.get("information")
    if value is None:
        return {"regime": "legacy_exact"}
    if not isinstance(value, dict) or value.get("regime") not in {
        "open_decklist",
        "closed_decklist",
    }:
        raise ValueError("invalid information regime")
    # Read historical schema-1 artifacts without relabelling them. A replay
    # buffer still requires exact equality, including this version.
    if type(value.get("knowledge_schema")) is not int or value["knowledge_schema"] not in {1, 2}:
        raise ValueError("unsupported knowledge schema")
    result = dict(value)
    if value["regime"] == "closed_decklist":
        for field in ("prior_hash", "legality_hash"):
            if not isinstance(value.get(field), str) or not re.fullmatch(
                r"[0-9a-f]{64}", value[field]
            ):
                raise ValueError(f"closed-decklist information missing {field}")
        if (
            not isinstance(value.get("prior_provenance"), str)
            or not value["prior_provenance"].strip()
        ):
            raise ValueError("closed-decklist information missing prior provenance")
    elif value.get("prior_hash") is not None or value.get("legality_hash") is not None:
        raise ValueError("open-decklist data cannot claim an inferred prior")
    return result


def require_same_information(manifests: list[dict]) -> dict:
    contracts = [information_contract(m) for m in manifests]
    if not contracts:
        raise ValueError("no information provenance supplied")
    if any(c != contracts[0] for c in contracts):
        raise ValueError("refusing mixed information regimes or prior snapshots")
    return contracts[0]


def run_information(regime: str, prior: Path | None) -> dict:
    if regime not in {"open_decklist", "closed_decklist"}:
        raise ValueError("unknown information regime")
    if (regime == "closed_decklist") != (prior is not None):
        raise ValueError("closed_decklist requires a prior; open_decklist forbids it")
    return {
        "regime": regime,
        "prior_file_sha256": (
            hashlib.sha256(prior.read_bytes()).hexdigest() if prior else None
        ),
    }


def bind_prepared_run(run_dir: Path, regime: str = "open_decklist", prior: Path | None = None) -> dict:
    """Record the information identity of a run folder a preparer just wrote.

    Campaign preparers write ``state.json`` (the parent checkpoint) before the
    first generation runs, and ``freeze_run_information`` refuses a run folder
    that has ``state.json`` but no identity, treating it as a pre-provenance
    legacy run. Preparers call this right after writing ``state.json``; an
    existing identity must match.
    """
    value = run_information(regime, prior)
    path = run_dir / "information.json"
    if path.exists():
        if json.loads(path.read_text()) != value:
            raise ValueError("prepared run already has a different information identity")
    else:
        run_dir.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(value, indent=2), encoding="utf-8")
    return value


def freeze_run_information(run_dir: Path, regime: str, prior: Path | None) -> dict:
    """Bind full-run resume, not only each scheduler's unfinished chunks."""
    value = run_information(regime, prior)
    path = run_dir / "information.json"
    if path.exists():
        if json.loads(path.read_text()) != value:
            raise ValueError("resume changes information regime or frozen prior")
    elif (run_dir / "state.json").exists():
        raise ValueError(
            "legacy run has no information identity; use a new run directory"
        )
    else:
        path.write_text(json.dumps(value, indent=2), encoding="utf-8")
    return value
