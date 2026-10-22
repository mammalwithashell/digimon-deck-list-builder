"""MetaGauntlet: meta-weighted opponent deck sampling for RL training.

Loads the deck library (produced by tools/meta_loader.py), keeps the
training-ready decklists, computes a Threat Index for each archetype, and samples
opponent decks weighted by TI or uniformly.

**Training gate (two layers, spec 2026-10-15 §5b):**
  1. Mechanics -- `data/mechanics_readiness.json` must be `ready` (no
     mechanics obligation hand-flagged `blocks_training`; unflagged open gaps
     do not close it). Checked once per load, before any deck is examined;
     `MechanicsNotReadyError` otherwise.
  2. Light per-card -- every card of a decklist must be registered in the Rust
     engine and its card database, and admissible in
     `data/card_admissibility.json`: in the committed registry, every DSL
     construct its YAML uses behaviourally tested by a mechanics test
     (lowering coverage), its own behavioural test when it has 4+ clauses or a
     protection construct, no failed authoring verdict, no untriaged
     divergence. Reasons: `unregistered`, `untested_lowering`,
     `untested_card`, `failed_verdict`, `diverged_no_quirk`. An unmeasured
     clause blocks nothing.
  Only those two committed `data/` artifacts are read at run time. No override
  admits a non-admissible deck; `legacy_clause_gate=True` only ADDS the old
  per-card oracle-readiness requirement for comparison runs.

**Survivorship Bias Fix (v2):**
  - Statistical weights (Threat Index) are derived ONLY from DigiLab tournament
    log data, which has full field participation counts.  Scraper-only sources
    (Digimon Meta, Egman Events) are treated as *deck pool* providers: they
    contribute optimised decklists but NOT the meta_share / conversion_rate
    used to weight sampling.
  - A confidence threshold ensures conversion_rate is only factored into TI
    when an archetype has >= `confidence_min_appearances` in the DigiLab data.
    Below that threshold, TI = meta_share only.
  - The Sleeper rule now uses DigiLab conversion_rate (not scraper stats).

**Deck Pool Routing:**
  When an archetype is sampled, the individual decklist is drawn preferentially
  from Digimon Meta (highly-optimised top-cut lists), then Egman, then any
  remaining source.

GauntletWrapper integrates into the Gymnasium wrapper chain, injecting sampled
opponent decks on reset() and applying bounty reward bonuses on terminal wins
against high-threat opponents.

Usage:
    gauntlet = MetaGauntlet(alpha=1.0, beta=2.0)
    gauntlet.load()
    deck = gauntlet.sample_opponent()
    # deck.card_ids is ready for HeadlessGame
    # deck.threat_index for reward shaping
"""

from __future__ import annotations

import json
import logging
import os
import hashlib
import warnings
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Mapping, Optional, Set

import numpy as np
import gymnasium

from digimon_engine import CardDatabase, load_implemented_card_ids, parse_tts

logger = logging.getLogger(__name__)

from data_paths import (
    ARCHETYPE_ALIASES as _ARCHETYPE_ALIASES_PATH,
    CARD_ADMISSIBILITY as _CARD_ADMISSIBILITY_PATH,
    DECK_LIBRARY as _DECK_LIBRARY_PATH,
    MECHANICS_READINESS as _MECHANICS_READINESS_PATH,
    ORACLE_READINESS as _ORACLE_READINESS_PATH,
)
# `card_admissibility` owns the ledger status set and folds it (plus FAILED /
# FAIL / QA-FAIL) into the committed per-card artifact, whose `failed_verdict`
# reason subsumes the retired run-time ledger gate. `_NOT_READY_DSL_STATUSES`
# is re-exported under its old name for one release. The import edge is one-way
# (gauntlet -> card_admissibility).
from digimon_gym.agents.card_admissibility import (
    CardAdmissibility,
    NOT_READY_LEDGER_STATUSES as _NOT_READY_DSL_STATUSES,
)

DECK_LIBRARY_PATH = str(_DECK_LIBRARY_PATH)
ARCHETYPE_ALIASES_PATH = str(_ARCHETYPE_ALIASES_PATH)


def _load_alias_map() -> Dict[str, str]:
    path = os.environ.get("ARCHETYPE_ALIASES_PATH", ARCHETYPE_ALIASES_PATH)
    try:
        with open(path, "r", encoding="utf-8") as f:
            raw = json.load(f)
    except FileNotFoundError:
        return {}
    result: Dict[str, str] = {}
    for canonical, aliases in raw.items():
        if canonical.startswith("_"):
            continue
        for alias in aliases:
            result[alias.lower()] = canonical
    return result


_ALIAS_MAP: Optional[Dict[str, str]] = None


def _normalize_archetype_quotes(name: str) -> str:
    """Fold typographic apostrophes/quotes to ASCII.

    Data sources disagree on apostrophe style — e.g. `data/deck_library.json`
    has "ST-3 Heaven's Yellow" (straight U+0027) while other sources write
    "ST-3 Heaven’s Yellow" (curly U+2019). Folding lets both spellings, and a
    normal apostrophe typed in `--archetypes`, resolve to the same archetype.
    """
    return (
        name.replace("’", "'")
        .replace("‘", "'")
        .replace("“", '"')
        .replace("”", '"')
    )


def canonicalize_archetype(name: str) -> str:
    """Resolve an archetype name to its canonical form via the alias index."""
    global _ALIAS_MAP
    if _ALIAS_MAP is None:
        _ALIAS_MAP = _load_alias_map()
    name = _normalize_archetype_quotes(name)
    return _ALIAS_MAP.get(name.lower(), name)


# ─── Training gate: two layers (spec 2026-10-15 §5b) ─────────────────
# Layer 1, mechanics: `data/mechanics_readiness.json` must be `ready` (no
#   obligation flagged `blocks_training`). Checked once per load before any
#   deck is examined (`require_mechanics_ready`).
# Layer 2, light per-card: every card of a decklist is admissible in
#   `data/card_admissibility.json` (`validate_admissible_deck`): lowering-covered
#   plus the hybrid own-test rule, no failed verdict, no untriaged divergence;
#   rejection reason `not_admissible`, logged with the artifact's per-card reason
#   (`unregistered` / `untested_lowering` / `untested_card` / `failed_verdict` /
#   `diverged_no_quirk`). The Architect TRAINING entrypoints read the same
#   artifacts through `architect_admission` (`training_gate=True`); the hosted
#   deck tools sharing those modules stay ungated.
# Run time reads ONLY those two committed `data/` artifacts (training images
# ship no `qa/`, ruling R5); a missing one stops training. Always on, for every
# library path and every pool snapshot; no CLI or env override admits a
# non-admissible deck. `legacy_clause_gate=True` (`--legacy-clause-gate`) only
# ADDS the old per-card requirement -- `ready` in `data/oracle_readiness.json`
# (spec 2026-10-04 §4.7) -- for transition comparison runs.

_REGENERATE_MECHANICS = "PYTHONPATH=code python -m tools.rules.mechanics_readiness"
_REGENERATE_ADMISSIBILITY = "PYTHONPATH=code python -m digimon_gym.agents.card_admissibility"
_REGENERATE_READINESS = "PYTHONPATH=code python -m tools.clause_coverage.readiness"
_NOT_ORACLE_READY = "not_oracle_ready"


class MechanicsReadinessMissingError(RuntimeError):
    """`data/mechanics_readiness.json` is missing; training refuses to guess."""


class CardAdmissibilityMissingError(RuntimeError):
    """`data/card_admissibility.json` is missing; training refuses to guess."""


class MechanicsNotReadyError(RuntimeError):
    """A blocking mechanics gap is open: no decklist is admitted."""


class OracleReadinessMissingError(RuntimeError):
    """`data/oracle_readiness.json` is missing under `--legacy-clause-gate`."""


class EmptyTrainingPoolError(RuntimeError):
    """No decklist survived the training gate."""


def _load_mechanics_readiness(path: Optional[str | Path] = None) -> Dict[str, Any]:
    """The committed mechanics artifact. No fallback: a missing file stops training."""
    path = Path(_MECHANICS_READINESS_PATH if path is None else path)
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        raise MechanicsReadinessMissingError(
            f"{path} is missing; copy it from the repo (data/mechanics_readiness.json), "
            f"or regenerate it with `{_REGENERATE_MECHANICS}`"
        ) from None


def require_mechanics_ready(readiness: Optional[Mapping[str, Any]] = None) -> None:
    """Layer 1: raise `MechanicsNotReadyError` unless the mechanics gate is open.

    `readiness` defaults to the committed artifact. The message names every
    blocking rule and gap id and the regenerate command.
    """
    readiness = _load_mechanics_readiness() if readiness is None else readiness
    if readiness.get("ready") is True:
        return
    blocking = list(readiness.get("blocking") or [])
    listed = "; ".join(
        f"{b.get('rule', '?')} ({b.get('gap_id') or 'no gap id'})" for b in blocking
    ) or "none listed (no proven obligation?)"
    counts = readiness.get("counts") or {}
    raise MechanicsNotReadyError(
        f"The mechanics training gate is closed: {len(blocking)} blocking mechanics gap(s): "
        f"{listed}. Counts: {json.dumps(counts, sort_keys=True)}. No decklist is admitted "
        "until they are fixed or reclassified in the obligation ledger; then regenerate "
        f"data/mechanics_readiness.json with `{_REGENERATE_MECHANICS}`."
    )


def _load_card_admissibility(path: Optional[str | Path] = None) -> CardAdmissibility:
    """The committed per-card artifact. No fallback: a missing file stops training."""
    path = Path(_CARD_ADMISSIBILITY_PATH if path is None else path)
    try:
        return CardAdmissibility.from_artifact(path)
    except FileNotFoundError:
        raise CardAdmissibilityMissingError(
            f"{path} is missing; copy it from the repo (data/card_admissibility.json), "
            "or regenerate it in a checkout that has qa/ and the engine binding with "
            f"`{_REGENERATE_ADMISSIBILITY}`"
        ) from None


def _load_oracle_ready_card_ids(path: Optional[str | Path] = None) -> Set[str]:
    """Card IDs whose every printed clause is adjudicated against the DCGO oracle.

    Read only under `legacy_clause_gate=True`. There is deliberately no
    fallback: a missing artifact stops a legacy-gated run.
    """
    path = Path(_ORACLE_READINESS_PATH if path is None else path)
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        raise OracleReadinessMissingError(
            f"{path} is missing; copy it from the repo (data/oracle_readiness.json), "
            "or regenerate it in a checkout that has qa/qa-reports/exam-verdicts with "
            f"`{_REGENERATE_READINESS}`"
        ) from None
    return {
        str(cid) for cid, entry in raw.get("cards", {}).items()
        if entry.get("status") == "ready"
    }


def _inadmissible_cards(card_ids: List[str], admissibility: Any) -> Dict[str, str]:
    """{card id: the artifact's reason} for each distinct non-admissible card, sorted."""
    return {
        cid: admissibility.reason(cid) or "not_admissible"
        for cid in sorted(set(card_ids))
        if not admissibility.admissible(cid)
    }


def _format_card_reasons(blocked: Mapping[str, str]) -> str:
    return ", ".join(f"{cid} ({reason})" for cid, reason in blocked.items())


def validate_admissible_deck(
    card_ids: List[str],
    admissibility: Optional[Any] = None,
    *,
    label: str = "deck",
) -> None:
    """Layer 2: raise `NotAdmissibleDeckError` if a deck holds a non-admissible card.

    For decks that reach training without passing through `MetaGauntlet.load`
    (pool snapshots, a job's `agent_deck: {source: deck_id}` library deck).
    `admissibility` (a `CardAdmissibility`, or anything with `admissible` and
    `reason`) defaults to the committed artifact. Layer 1 is checked separately,
    once per load: `require_mechanics_ready`.
    """
    admissibility = _load_card_admissibility() if admissibility is None else admissibility
    blocked = _inadmissible_cards(card_ids, admissibility)
    if blocked:
        raise NotAdmissibleDeckError(
            f"{label} contains cards that are not admissible: {_format_card_reasons(blocked)}"
        )


def validate_oracle_ready_deck(*args: Any, **kwargs: Any) -> None:
    """Deprecated alias of `validate_admissible_deck` (removed next release)."""
    warnings.warn(
        "validate_oracle_ready_deck is deprecated; use validate_admissible_deck "
        "(the training gate is mechanics + light per-card, spec 2026-10-15 §5b)",
        DeprecationWarning,
        stacklevel=2,
    )
    validate_admissible_deck(*args, **kwargs)


def _validate_oracle_ready_deck_legacy(
    card_ids: List[str], oracle_ready: Set[str], *, label: str
) -> None:
    not_ready = sorted({cid for cid in card_ids if cid not in oracle_ready})
    if not_ready:
        raise NotAdmissibleDeckError(
            f"{label} contains cards that are not oracle-ready (--legacy-clause-gate): "
            f"{', '.join(not_ready)}"
        )


def _card_name(card_db: Optional[Any], card_id: str) -> str:
    card = card_db.get_card(card_id) if card_db is not None else None
    return f" {card.card_name}" if card is not None and getattr(card, "card_name", "") else ""


def _describe_blockers(blockers: Counter, reasons: Mapping[str, str], top: int = 10) -> str:
    """The fail-fast message: card, name, decklists blocked and reason, worst first."""
    try:
        card_db: Optional[Any] = CardDatabase()
    except Exception:  # names are a courtesy; never mask the real error
        card_db = None
    worst = sorted(blockers.items(), key=lambda kv: (-kv[1], kv[0]))[:top]
    listed = ", ".join(
        f"{cid}{_card_name(card_db, cid)} ({n} decklist{'' if n == 1 else 's'}, {reasons[cid]})"
        for cid, n in worst
    )
    message = (
        "No training decklist is admissible. Cards blocking the most decklists: "
        f"{listed}. Fix the cards or their verdicts, then regenerate "
        f"data/card_admissibility.json with `{_REGENERATE_ADMISSIBILITY}` "
        f"(and data/mechanics_readiness.json with `{_REGENERATE_MECHANICS}`)."
    )
    if _NOT_ORACLE_READY in reasons.values():
        message += f" --legacy-clause-gate is on; next exams: `{_REGENERATE_READINESS} --plan`."
    return message


# Deck source priority for within-archetype selection (higher = preferred).
# DigimonMeta lists are highly-optimised top-cut builds.
_SOURCE_PREFERENCE = {"digimonmeta": 3, "egman": 2, "digimoncard_io": 1, "file": 0, "manual": 0, "test": 0}
SNAPSHOT_SCHEMA_VERSION = 1


class UnimplementedDeckError(ValueError):
    """Raised when a training deck contains cards outside the Rust registry."""


class NotAdmissibleDeckError(ValueError):
    """A deck outside the gauntlet (pool snapshot, library deck) holds a card the
    training gate does not admit."""


# Deprecated alias (one release): the gate is no longer per-clause oracle readiness.
NotOracleReadyDeckError = NotAdmissibleDeckError


def canonical_deck_counts(card_ids: List[str]) -> Dict[str, int]:
    """Return a stable card-count mapping for deck identity and snapshots."""
    return dict(sorted(Counter(card_ids).items()))


def stable_deck_id(card_ids: List[str]) -> str:
    """Content-address a deck by its canonical card counts."""
    payload = json.dumps(
        {"card_counts": canonical_deck_counts(card_ids)},
        sort_keys=True,
        separators=(",", ":"),
    )
    return "sha256:" + hashlib.sha256(payload.encode("utf-8")).hexdigest()


def load_playable_card_ids() -> Set[str]:
    """Card IDs a training game can be built with.

    A card must be registered in the Rust engine AND present in the card
    database `RustHeadlessGame` loads (`data/cards.json`). A YAML spec alone
    registers a card: P-241 had one but no cards.json entry, and building a
    game with it raised "Card P-241 not found in card database".
    """
    card_db = CardDatabase()
    return {cid for cid in load_implemented_card_ids() if card_db.get_card(cid) is not None}


def missing_unimplemented_card_ids(
    card_ids: List[str],
    implemented_card_ids: Set[str],
) -> List[str]:
    """Return sorted unique card IDs that are absent from the Rust registry."""
    return sorted({cid for cid in card_ids if cid not in implemented_card_ids})


def validate_implemented_deck(
    card_ids: List[str],
    implemented_card_ids: Set[str],
    *,
    label: str = "deck",
) -> None:
    """Fail fast if a deck contains unimplemented card IDs."""
    missing = missing_unimplemented_card_ids(card_ids, implemented_card_ids)
    if missing:
        raise UnimplementedDeckError(
            f"{label} contains unimplemented card IDs: {', '.join(missing)}"
        )


def _parse_library_decklist(dl: Dict[str, Any]) -> List[str]:
    """Parse a deck_library decklist record into flat card IDs."""
    tts_str = dl.get("decklist", "")
    if tts_str:
        return parse_tts(tts_str)
    return list(dl.get("card_ids", []))


def _snapshot_hash(payload: Dict[str, Any]) -> str:
    canonical = json.dumps(payload, sort_keys=True, separators=(",", ":"))
    return "sha256:" + hashlib.sha256(canonical.encode("utf-8")).hexdigest()


# ─── Data Classes ────────────────────────────────────────────────────

@dataclass
class DeckEntry:
    """A concrete deck that can be played in training."""
    deck_id: str
    archetype_name: str
    card_ids: List[str]  # Flat list ready for HeadlessGame
    source: str = ""
    threat_index: float = 0.0
    source_deck_id: str = ""
    source_url: str = ""

    @property
    def card_counts(self) -> Dict[str, int]:
        return canonical_deck_counts(self.card_ids)


@dataclass
class ArchetypeStats:
    """Archetype-level stats and associated decks."""
    archetype_name: str
    display_card_id: Optional[str] = None
    primary_color: Optional[str] = None
    # DigiLab-sourced stats (sole basis for Threat Index)
    digilab_meta_share: float = 0.0
    digilab_conversion_rate: float = 0.0
    digilab_win_rate: float = 0.0
    digilab_times_played: int = 0
    # Computed
    threat_index: float = 0.0
    sampling_probability: float = 0.0
    decks: List[DeckEntry] = field(default_factory=list)


@dataclass
class GeneralistDeckPool:
    """Implementation-safe deck pool for generalist pilot curricula."""

    archetypes: Dict[str, List[DeckEntry]]
    snapshot_hash: str = ""
    snapshot_path: str = ""

    @property
    def archetype_names(self) -> List[str]:
        return sorted(self.archetypes)

    @property
    def deck_count(self) -> int:
        return sum(len(decks) for decks in self.archetypes.values())

    @property
    def archetype_count(self) -> int:
        return len(self.archetypes)

    def sample_uniform_archetype(self, rng: np.random.Generator) -> DeckEntry:
        """Sample archetype uniformly, then deck uniformly within archetype."""
        names = self.archetype_names
        if not names:
            raise RuntimeError("GeneralistDeckPool has no eligible archetypes.")
        arch = str(rng.choice(names))
        decks = sorted(self.archetypes[arch], key=lambda d: d.deck_id)
        return decks[int(rng.integers(0, len(decks)))]

    def to_snapshot(self) -> Dict[str, Any]:
        records: List[Dict[str, Any]] = []
        for arch in self.archetype_names:
            for deck in sorted(self.archetypes[arch], key=lambda d: d.deck_id):
                records.append({
                    "deck_id": deck.deck_id,
                    "archetype": arch,
                    "card_counts": deck.card_counts,
                    "card_ids": list(deck.card_ids),
                    "source": deck.source,
                    "source_deck_id": deck.source_deck_id,
                    "source_url": deck.source_url,
                })
        content = {
            "schema_version": SNAPSHOT_SCHEMA_VERSION,
            "sampling_policy": "uniform_archetype_then_deck",
            "eligible_archetypes": self.archetype_names,
            "decks": records,
        }
        snapshot_hash = _snapshot_hash(content)
        return {
            **content,
            "snapshot_hash": snapshot_hash,
            "generated_at": datetime.now(timezone.utc).isoformat(),
        }

    def write_snapshot(self, path: str | Path) -> str:
        path = Path(path)
        path.parent.mkdir(parents=True, exist_ok=True)
        snapshot = self.to_snapshot()
        path.write_text(json.dumps(snapshot, indent=2, sort_keys=True), encoding="utf-8")
        self.snapshot_hash = str(snapshot["snapshot_hash"])
        self.snapshot_path = str(path)
        return self.snapshot_hash

    @classmethod
    def from_snapshot(
        cls,
        path: str | Path,
        *,
        implemented_card_ids: Optional[Set[str]] = None,
        admissibility: Optional[Any] = None,
        mechanics_readiness: Optional[Mapping[str, Any]] = None,
        legacy_clause_gate: bool = False,
    ) -> "GeneralistDeckPool":
        """Reload a written pool. Every deck must still pass the training gate
        (mechanics, then light per-card; plus oracle readiness under
        `legacy_clause_gate`): a snapshot taken under an older gate cannot
        smuggle non-admissible decks back in. `admissibility` and
        `mechanics_readiness` default to the committed artifacts."""
        require_mechanics_ready(mechanics_readiness)
        path = Path(path)
        raw = json.loads(path.read_text(encoding="utf-8"))
        if raw.get("schema_version") != SNAPSHOT_SCHEMA_VERSION:
            raise ValueError(
                f"Unsupported deck-pool snapshot schema_version={raw.get('schema_version')!r}"
            )
        content = {
            "schema_version": raw.get("schema_version"),
            "sampling_policy": raw.get("sampling_policy"),
            "eligible_archetypes": raw.get("eligible_archetypes", []),
            "decks": raw.get("decks", []),
        }
        expected_hash = _snapshot_hash(content)
        if raw.get("snapshot_hash") != expected_hash:
            raise ValueError("Deck-pool snapshot hash mismatch.")

        admissibility = (
            _load_card_admissibility() if admissibility is None else admissibility
        )
        oracle_ready = _load_oracle_ready_card_ids() if legacy_clause_gate else None
        archetypes: Dict[str, List[DeckEntry]] = {}
        for record in raw.get("decks", []):
            card_ids = list(record.get("card_ids", []))
            if not card_ids and record.get("card_counts"):
                for cid, count in sorted(record["card_counts"].items()):
                    card_ids.extend([cid] * int(count))
            if implemented_card_ids is not None:
                validate_implemented_deck(
                    card_ids,
                    implemented_card_ids,
                    label=f"snapshot deck {record.get('deck_id', '?')}",
                )
            label = f"snapshot deck {record.get('deck_id', '?')}"
            validate_admissible_deck(card_ids, admissibility, label=label)
            if oracle_ready is not None:
                _validate_oracle_ready_deck_legacy(card_ids, oracle_ready, label=label)
            deck_id = stable_deck_id(card_ids)
            if record.get("deck_id") != deck_id:
                raise ValueError(
                    f"Snapshot deck_id mismatch for {record.get('deck_id', '?')}"
                )
            arch = str(record["archetype"])
            archetypes.setdefault(arch, []).append(DeckEntry(
                deck_id=deck_id,
                archetype_name=arch,
                card_ids=card_ids,
                source=record.get("source", ""),
                source_deck_id=record.get("source_deck_id", ""),
                source_url=record.get("source_url", ""),
            ))
        pool = cls(archetypes=archetypes, snapshot_hash=expected_hash, snapshot_path=str(path))
        return pool

    @classmethod
    def from_archetype_stats(
        cls,
        archetypes: Dict[str, ArchetypeStats],
    ) -> "GeneralistDeckPool":
        return cls(
            archetypes={
                name: list(stats.decks)
                for name, stats in archetypes.items()
                if stats.decks
            }
        )


def load_generalist_deck_pool(
    path: str = DECK_LIBRARY_PATH,
    *,
    implemented_card_ids: Optional[Set[str]] = None,
    allowed_archetypes: Optional[Set[str]] = None,
    admissibility: Optional[Any] = None,
    mechanics_readiness: Optional[Mapping[str, Any]] = None,
    legacy_clause_gate: bool = False,
) -> GeneralistDeckPool:
    """Load the same gated eligible pool used by MetaGauntlet."""
    gauntlet = MetaGauntlet(
        sampling_mode="random",
        implemented_card_ids=implemented_card_ids,
        allowed_archetypes=allowed_archetypes,
        admissibility=admissibility,
        mechanics_readiness=mechanics_readiness,
    )
    gauntlet.load(path, legacy_clause_gate=legacy_clause_gate)
    return gauntlet.as_generalist_pool()


# ─── MetaGauntlet ───────────────────────────────────────────────────

class MetaGauntlet:
    """Meta-weighted opponent deck pool for RL training.

    Threat Index formula (DigiLab data only):
        if digilab_times_played >= confidence_min_appearances:
            TI = (digilab_meta_share * alpha) + (digilab_conversion_rate * beta)
        else:
            TI = digilab_meta_share * alpha   # insufficient data for conversion

    Sleeper rule: if digilab_conversion_rate > sleeper_threshold (default 50%)
    AND the archetype meets the confidence threshold, force minimum sleeper_floor
    (default 5%) sampling probability.

    Deck Pool Routing: when sampling within an archetype, decks from higher-
    quality sources (digimonmeta > egman > others) are preferred.

    sampling_mode:
      - "meta": normalize Threat Index by archetype, then split each archetype's
        probability evenly across its decklists.
      - "random": sample uniformly across all training-ready decklists.
    """

    def __init__(
        self,
        alpha: float = 1.0,
        beta: float = 2.0,
        sleeper_threshold: float = 0.50,
        sleeper_floor: float = 0.05,
        confidence_min_appearances: int = 5,
        seed: Optional[int] = None,
        sampling_mode: str = "meta",
        implemented_only: bool = True,
        implemented_card_ids: Optional[Set[str]] = None,
        allowed_archetypes: Optional[Set[str]] = None,
        admissibility: Optional[Any] = None,
        mechanics_readiness: Optional[Mapping[str, Any]] = None,
    ) -> None:
        if sampling_mode not in {"meta", "random"}:
            raise ValueError("sampling_mode must be 'meta' or 'random'")
        self.alpha = alpha
        self.beta = beta
        self.sleeper_threshold = sleeper_threshold
        self.sleeper_floor = sleeper_floor
        self.confidence_min_appearances = confidence_min_appearances
        self.sampling_mode = sampling_mode
        self.implemented_only = implemented_only
        self._implemented_card_ids = implemented_card_ids
        self._allowed_archetypes = allowed_archetypes
        # Injected gate inputs (tests); None = the committed data/ artifacts.
        self._admissibility = admissibility
        self._mechanics_readiness = mechanics_readiness

        self.archetypes: Dict[str, ArchetypeStats] = {}
        self._deck_pool: List[DeckEntry] = []
        self._weights: Optional[np.ndarray] = None
        self._rng = np.random.default_rng(seed)

    def load(self, path: str = DECK_LIBRARY_PATH, *, legacy_clause_gate: bool = False) -> None:
        """Load deck library JSON and compute threat indices + sampling weights.

        Statistical weights come ONLY from digilab_stats.  Scraper stats
        (meta_share, conversion_rate in the top-level ``stats`` block) are
        ignored for TI computation to avoid survivorship bias.

        The training gate applies to every library path: the mechanics layer
        first (`MechanicsNotReadyError` before any deck is examined), then each
        decklist is admitted only if every card is registered and admissible.
        `legacy_clause_gate=True` additionally requires every card to be
        oracle-ready; it never admits a deck the two layers reject.
        """
        require_mechanics_ready(self._mechanics_readiness)
        admissibility = (
            self._admissibility if self._admissibility is not None
            else _load_card_admissibility()
        )
        oracle_ready = _load_oracle_ready_card_ids() if legacy_clause_gate else None

        with open(path, "r", encoding="utf-8") as f:
            data = json.load(f)

        self.archetypes = {}
        self._deck_pool = []
        self._weights = None
        implemented = None
        if self.implemented_only:
            implemented = (
                self._implemented_card_ids
                if self._implemented_card_ids is not None
                else load_playable_card_ids()
            )
        blockers: Counter = Counter()
        blocker_reasons: Dict[str, str] = {}

        canonical_allowed: Optional[Set[str]] = None
        if self._allowed_archetypes is not None:
            canonical_allowed = {
                canonicalize_archetype(name) for name in self._allowed_archetypes
            }
            library_canonical = {
                canonicalize_archetype(raw_name)
                for raw_name in data.get("archetypes", {}).keys()
            }
            unrecognized = canonical_allowed - library_canonical
            for name in sorted(unrecognized):
                logger.warning(
                    "allowed_archetypes entry %r did not match any archetype in %s",
                    name, path,
                )

        # First pass: gather DigiLab total_appearances for meta_share denominator
        # Aggregate aliased entries under canonical names
        digilab_total_appearances = 0
        arch_digilab: Dict[str, dict] = {}

        for raw_name, arch_data in data.get("archetypes", {}).items():
            arch_name = canonicalize_archetype(raw_name)
            digilab = arch_data.get("digilab_stats")
            if digilab:
                tp = digilab.get("times_played", 0)
                digilab_total_appearances += tp
                if arch_name in arch_digilab:
                    # Merge stats from alias
                    existing = arch_digilab[arch_name]
                    old_tp = existing.get("times_played", 0)
                    new_tp = old_tp + tp
                    if new_tp > 0:
                        for key in ("conversion_rate", "win_rate", "top4_rate"):
                            existing[key] = (
                                existing.get(key, 0.0) * old_tp
                                + digilab.get(key, 0.0) * tp
                            ) / new_tp
                    existing["times_played"] = new_tp
                else:
                    arch_digilab[arch_name] = dict(digilab)

        # Group raw archetype data by canonical name for merging
        canonical_groups: Dict[str, List[tuple]] = {}
        for raw_name, arch_data in data.get("archetypes", {}).items():
            arch_name = canonicalize_archetype(raw_name)
            if arch_name not in canonical_groups:
                canonical_groups[arch_name] = []
            canonical_groups[arch_name].append((raw_name, arch_data))

        # Second pass: build archetype stats and deck pools. Each decklist is
        # admitted on its own: every card registered and admissible.
        rejected: Dict[str, Counter] = defaultdict(Counter)
        for arch_name, group in canonical_groups.items():
            if canonical_allowed is not None and arch_name not in canonical_allowed:
                logger.debug(
                    "Skipping %s: not in allowed_archetypes",
                    arch_name,
                )
                continue
            # Merge all entries in the group
            arch_data_merged = {}
            for _, ad in group:
                for key in ("display_card_id", "primary_color"):
                    if not arch_data_merged.get(key) and ad.get(key):
                        arch_data_merged[key] = ad[key]
                arch_data_merged.setdefault("decklists", []).extend(ad.get("decklists", []))
            arch_data = arch_data_merged
            decks: List[DeckEntry] = []
            for dl in arch_data.get("decklists", []):
                try:
                    card_ids = _parse_library_decklist(dl)
                except ValueError:
                    logger.warning("Invalid TTS decklist in %s, skipping", dl.get("deck_id", "?"))
                    continue
                if not card_ids:
                    continue
                if implemented is not None:
                    missing = missing_unimplemented_card_ids(card_ids, implemented)
                else:
                    missing = []
                if missing:
                    logger.debug(
                        "Skipping unimplemented decklist %s for %s: %s",
                        dl.get("deck_id", "?"),
                        arch_name,
                        ", ".join(missing),
                    )
                    rejected[arch_name]["unregistered"] += 1
                    continue
                blocked = _inadmissible_cards(card_ids, admissibility)
                if blocked:
                    logger.debug(
                        "Skipping not-admissible decklist %s for %s: %s",
                        dl.get("deck_id", "?"), arch_name, _format_card_reasons(blocked),
                    )
                    rejected[arch_name]["not_admissible"] += 1
                    blockers.update(blocked.keys())
                    blocker_reasons.update(blocked)
                    continue
                if oracle_ready is not None:
                    not_oracle = sorted({cid for cid in card_ids if cid not in oracle_ready})
                    if not_oracle:
                        logger.debug(
                            "Skipping not-oracle-ready decklist %s for %s "
                            "(--legacy-clause-gate): %s",
                            dl.get("deck_id", "?"), arch_name, ", ".join(not_oracle),
                        )
                        rejected[arch_name][_NOT_ORACLE_READY] += 1
                        blockers.update(not_oracle)
                        for cid in not_oracle:
                            blocker_reasons.setdefault(cid, _NOT_ORACLE_READY)
                        continue
                decks.append(DeckEntry(
                    deck_id=stable_deck_id(card_ids),
                    archetype_name=arch_name,
                    card_ids=card_ids,
                    source=dl.get("source", ""),
                    source_deck_id=dl.get("deck_id", ""),
                    source_url=dl.get("source_url", ""),
                ))

            if not decks:
                logger.debug("Archetype %s has no decklists, skipping", arch_name)
                continue

            # DigiLab-sourced stats (sole basis for TI)
            digilab = arch_digilab.get(arch_name, {})
            digilab_tp = digilab.get("times_played", 0)
            digilab_conv = digilab.get("conversion_rate", 0.0)
            digilab_win = digilab.get("win_rate", 0.0)
            digilab_ms = (
                digilab_tp / digilab_total_appearances
                if digilab_total_appearances > 0
                else 0.0
            )

            # Sort decks by source preference (best first) for routing
            decks.sort(
                key=lambda d: _SOURCE_PREFERENCE.get(d.source, 0),
                reverse=True,
            )

            stats = ArchetypeStats(
                archetype_name=arch_name,
                display_card_id=arch_data.get("display_card_id"),
                primary_color=arch_data.get("primary_color"),
                digilab_meta_share=digilab_ms,
                digilab_conversion_rate=digilab_conv,
                digilab_win_rate=digilab_win,
                digilab_times_played=digilab_tp,
                decks=decks,
            )
            self.archetypes[arch_name] = stats

        if canonical_allowed is not None:
            for name in sorted((canonical_allowed & set(canonical_groups)) - set(self.archetypes)):
                logger.info(
                    "allowed_archetypes entry %r excluded: no training-ready decklist "
                    "(%d with unregistered cards, %d with not-admissible cards, "
                    "%d not oracle-ready under --legacy-clause-gate)",
                    name,
                    rejected[name]["unregistered"],
                    rejected[name]["not_admissible"],
                    rejected[name][_NOT_ORACLE_READY],
                )

        if not self.archetypes and blockers:
            raise EmptyTrainingPoolError(_describe_blockers(blockers, blocker_reasons))

        self._compute_threat_indices()
        self._compute_sampling_weights()

        logger.info(
            "MetaGauntlet loaded: %d archetypes, %d total decks",
            len(self.archetypes), len(self._deck_pool),
        )

    def override_meta_shares(self, overrides: Dict[str, float]) -> None:
        """Replace digilab_meta_share with scoped values, recompute TI + weights.

        Archetypes not present in *overrides* get meta_share = 0.0 (they may
        still have non-zero TI from conversion rate if they meet the confidence
        threshold).
        """
        for name, stats in self.archetypes.items():
            stats.digilab_meta_share = overrides.get(name, 0.0)
        self._compute_threat_indices()
        self._compute_sampling_weights()

    def _compute_threat_indices(self) -> None:
        """Compute TI using DigiLab stats only.

        Confidence threshold: conversion_rate is only factored in when
        digilab_times_played >= confidence_min_appearances.  Otherwise
        TI is based on meta_share alone (avoids noisy small-sample conv rates).
        """
        for stats in self.archetypes.values():
            ti = stats.digilab_meta_share * self.alpha
            if stats.digilab_times_played >= self.confidence_min_appearances:
                ti += stats.digilab_conversion_rate * self.beta
            stats.threat_index = ti
            for deck in stats.decks:
                deck.threat_index = stats.threat_index

    def _compute_sampling_weights(self) -> None:
        """Build flat deck pool and compute per-deck sampling probabilities.

        Each deck within an archetype gets equal share of that archetype's weight.
        Sleeper rule: archetypes meeting the confidence threshold with
        conversion_rate > sleeper_threshold get minimum sleeper_floor.
        """
        self._deck_pool = []

        if self.sampling_mode == "random":
            for stats in self.archetypes.values():
                self._deck_pool.extend(stats.decks)
            n = len(self._deck_pool)
            if n == 0:
                self._weights = np.array([], dtype=np.float64)
                return
            self._weights = np.ones(n, dtype=np.float64) / n
            for stats in self.archetypes.values():
                stats.sampling_probability = len(stats.decks) / n
            return

        archetype_weights: Dict[str, float] = {}

        # Step 1: Raw TI weights per archetype
        for name, stats in self.archetypes.items():
            archetype_weights[name] = stats.threat_index

        # Step 2: Normalize to probabilities
        total_ti = sum(archetype_weights.values())
        if total_ti == 0:
            n = len(self.archetypes)
            for name in archetype_weights:
                archetype_weights[name] = 1.0 / n if n > 0 else 0.0
        else:
            for name in archetype_weights:
                archetype_weights[name] /= total_ti

        # Step 3: Sleeper rule (uses DigiLab conv rate + confidence gate)
        sleepers: List[str] = []
        redistribution_needed = 0.0

        for name, stats in self.archetypes.items():
            if (stats.digilab_times_played >= self.confidence_min_appearances
                    and stats.digilab_conversion_rate > self.sleeper_threshold
                    and archetype_weights[name] < self.sleeper_floor):
                deficit = self.sleeper_floor - archetype_weights[name]
                redistribution_needed += deficit
                archetype_weights[name] = self.sleeper_floor
                sleepers.append(name)

        if redistribution_needed > 0:
            non_sleeper_total = sum(
                w for n, w in archetype_weights.items() if n not in sleepers
            )
            if non_sleeper_total > 0:
                factor = (non_sleeper_total - redistribution_needed) / non_sleeper_total
                factor = max(factor, 0.01)
                for name in archetype_weights:
                    if name not in sleepers:
                        archetype_weights[name] *= factor

        # Step 4: Distribute archetype weight evenly across its decklists
        weights_list: List[float] = []
        for name, stats in self.archetypes.items():
            n_decks = len(stats.decks)
            per_deck_weight = archetype_weights[name] / n_decks
            stats.sampling_probability = archetype_weights[name]
            for deck in stats.decks:
                self._deck_pool.append(deck)
                weights_list.append(per_deck_weight)

        # Step 5: Final normalization
        total = sum(weights_list)
        if total > 0:
            self._weights = np.array(weights_list, dtype=np.float64)
            self._weights /= self._weights.sum()
        else:
            n = max(len(weights_list), 1)
            self._weights = np.ones(n, dtype=np.float64) / n

    def sample_opponent(self) -> DeckEntry:
        """Sample one opponent deck weighted by threat index.

        Returns:
            DeckEntry with card_ids ready for HeadlessGame and threat_index.

        Raises:
            RuntimeError: If no decks are loaded.
        """
        if not self._deck_pool or self._weights is None:
            raise RuntimeError(
                "MetaGauntlet has no decks loaded. "
                "Call load() first, or check that deck_library.json has decklists."
            )
        idx = self._rng.choice(len(self._deck_pool), p=self._weights)
        return self._deck_pool[idx]

    def sample_opponents(self, n: int) -> List[DeckEntry]:
        """Sample n opponent decks (with replacement)."""
        if not self._deck_pool or self._weights is None:
            raise RuntimeError("MetaGauntlet has no decks loaded.")
        indices = self._rng.choice(len(self._deck_pool), size=n, p=self._weights)
        return [self._deck_pool[i] for i in indices]

    def get_archetype_summary(self) -> List[Dict]:
        """Return summary for logging/debugging, sorted by TI descending."""
        return [
            {
                "archetype": s.archetype_name,
                "digilab_meta_share": round(s.digilab_meta_share, 4),
                "digilab_conversion_rate": round(s.digilab_conversion_rate, 4),
                "digilab_times_played": s.digilab_times_played,
                "threat_index": round(s.threat_index, 4),
                "sampling_prob": round(s.sampling_probability, 4),
                "n_decks": len(s.decks),
            }
            for s in sorted(
                self.archetypes.values(),
                key=lambda x: x.threat_index,
                reverse=True,
            )
        ]

    def as_generalist_pool(self) -> GeneralistDeckPool:
        """Return this loaded gauntlet as a generalist deck pool."""
        return GeneralistDeckPool.from_archetype_stats(self.archetypes)

    @property
    def deck_count(self) -> int:
        """Total number of playable decks in the pool."""
        return len(self._deck_pool)

    @property
    def archetype_count(self) -> int:
        """Number of archetypes with at least one deck."""
        return len(self.archetypes)


# ─── GauntletWrapper ────────────────────────────────────────────────

class GauntletWrapper(gymnasium.Wrapper):
    """Wraps an env to sample opponent decks from MetaGauntlet each episode.

    On reset(), samples a new opponent deck and injects it via options.
    On step(), adds bounty bonus to terminal rewards for beating strong opponents.

    Wrapper chain: DigimonEnv -> OpponentWrapper -> GauntletWrapper -> ActionMasker
    """

    def __init__(
        self,
        env: gymnasium.Env,
        gauntlet: MetaGauntlet,
        player_deck: List[str],
        bounty_threshold: float = 0.15,
        bounty_bonus: float = 0.5,
    ) -> None:
        super().__init__(env)
        self.gauntlet = gauntlet
        self.player_deck = player_deck
        self.bounty_threshold = bounty_threshold
        self.bounty_bonus = bounty_bonus
        self._current_opponent: Optional[DeckEntry] = None

    def reset(self, **kwargs):
        self._current_opponent = self.gauntlet.sample_opponent()

        options = kwargs.get("options") or {}
        if "deck1" not in options:
            options["deck1"] = self.player_deck
        options["deck2"] = self._current_opponent.card_ids
        kwargs["options"] = options

        obs, info = self.env.reset(**kwargs)

        info["opponent_archetype"] = self._current_opponent.archetype_name
        info["opponent_threat_index"] = self._current_opponent.threat_index

        return obs, info

    def step(self, action):
        obs, reward, terminated, truncated, info = self.env.step(action)

        # Bounty bonus on terminal win against strong opponent
        if terminated and float(reward) > 0:
            if (self._current_opponent is not None
                    and self._current_opponent.threat_index > self.bounty_threshold):
                reward = float(reward) + self.bounty_bonus
                info["bounty_applied"] = True

        return obs, reward, terminated, truncated, info

    @property
    def current_opponent(self) -> Optional[DeckEntry]:
        """The opponent deck for the current episode."""
        return self._current_opponent


class GeneralistDeckPoolWrapper(gymnasium.Wrapper):
    """Samples player decks from one or two `GeneralistDeckPool`s each episode.

    Symmetric mode (default): both decks sampled from `deck_pool`.

    Asymmetric mode: pass `opponent_pool` to sample P2's deck from a different
    pool than P1. Used for curricula where the agent should pilot a restricted
    archetype set but face the full opponent field — e.g.,
    `--archetypes "DNA Omnimon,BG Imperial"` for the agent paired with an
    unfiltered opponent pool.
    """

    def __init__(
        self,
        env: gymnasium.Env,
        deck_pool: GeneralistDeckPool,
        seed: Optional[int] = None,
        opponent_pool: Optional[GeneralistDeckPool] = None,
    ) -> None:
        super().__init__(env)
        self.deck_pool = deck_pool
        self.opponent_pool = opponent_pool if opponent_pool is not None else deck_pool
        self._rng = np.random.default_rng(seed)
        self._current_deck1: Optional[DeckEntry] = None
        self._current_deck2: Optional[DeckEntry] = None

    def reset(self, **kwargs):
        self._current_deck1 = self.deck_pool.sample_uniform_archetype(self._rng)
        self._current_deck2 = self.opponent_pool.sample_uniform_archetype(self._rng)

        options = kwargs.get("options") or {}
        options["deck1"] = self._current_deck1.card_ids
        options["deck2"] = self._current_deck2.card_ids
        kwargs["options"] = options

        obs, info = self.env.reset(**kwargs)
        info["deck1_archetype"] = self._current_deck1.archetype_name
        info["deck1_deck_id"] = self._current_deck1.deck_id
        info["opponent_archetype"] = self._current_deck2.archetype_name
        info["opponent_deck_id"] = self._current_deck2.deck_id
        return obs, info

    @property
    def current_deck1(self) -> Optional[DeckEntry]:
        return self._current_deck1

    @property
    def current_deck2(self) -> Optional[DeckEntry]:
        return self._current_deck2
