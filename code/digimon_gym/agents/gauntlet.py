"""MetaGauntlet: meta-weighted opponent deck sampling for RL training.

Loads the deck library (produced by tools/meta_loader.py), filters it to
QA-clean DSL archetypes whose cards are all registered in the Rust engine,
computes a Threat Index for each archetype, and samples opponent decks weighted
by TI or uniformly.

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
from collections import Counter
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional, Set

import numpy as np
import gymnasium

from digimon_engine import load_implemented_card_ids, parse_tts

logger = logging.getLogger(__name__)

from data_paths import (
    ARCHETYPE_ALIASES as _ARCHETYPE_ALIASES_PATH,
    DECK_LIBRARY as _DECK_LIBRARY_PATH,
)

DECK_LIBRARY_PATH = str(_DECK_LIBRARY_PATH)
ARCHETYPE_ALIASES_PATH = str(_ARCHETYPE_ALIASES_PATH)
_QA_DSL_STATUS_PATH = (
    _DECK_LIBRARY_PATH.parent.parent
    / "qa"
    / "qa-reports"
    / "validated_cards_dsl.json"
)


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
    has "ST-3 Heaven's Yellow" (straight U+0027) while the DSL ledger has
    "ST-3 Heaven’s Yellow" (curly U+2019). Without folding, the two never
    match and ST-3 is silently dropped from the training pool. Also lets a user
    type a normal apostrophe in `--archetypes`.
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


# DSL ledger verdicts that count as "ready to train". A freshly IMPLEMENTED card
# and an AUDITED-OK card (an existing YAML spec audited faithful against printed
# text + DCGO) are both fully playable. Other verdicts (PARTIAL, AUDITED-DRIFT,
# AUDITED-MISSING-TESTS, BLOCKED) are NOT ready and must still exclude their
# archetype — `all(...)` below enforces that one non-ready card drops the deck.
# Gate 2 (`missing_unimplemented_card_ids` against the live Rust registry) still
# independently verifies every card actually exists in the engine, so widening
# this verdict set cannot admit a card the engine can't play.
_TRAINING_READY_DSL_STATUSES = frozenset({"IMPLEMENTED", "AUDITED-OK"})


def _load_fully_implemented_archetypes(path: str = str(_QA_DSL_STATUS_PATH)) -> Optional[Set[str]]:
    """Return archetypes whose DSL ledger entries are all training-ready.

    "Training-ready" = every card's verdict is in `_TRAINING_READY_DSL_STATUSES`
    (IMPLEMENTED or AUDITED-OK). Missing status files return None so non-standard
    test/library callers can still load synthetic data. An existing file with no
    entry for an archetype means that archetype is not considered ready.
    """
    try:
        with open(path, "r", encoding="utf-8") as f:
            raw = json.load(f)
    except FileNotFoundError:
        logger.warning("DSL validation ledger not found: %s", path)
        return None

    by_archetype: Dict[str, List[str]] = {}
    for entry in raw.get("cards", {}).values():
        archetype = entry.get("archetype")
        status = entry.get("status")
        if not archetype or not status:
            continue
        # Canonicalize so ledger entries that use an alias (e.g. "Red Hybrid")
        # match library entries under the canonical name ("Red Hybrid
        # (AncientGreymon)"). Without this, a ledger / library naming mismatch
        # silently drops every decklist for the archetype from the pool.
        canonical = canonicalize_archetype(str(archetype))
        by_archetype.setdefault(canonical, []).append(str(status))

    return {
        archetype
        for archetype, statuses in by_archetype.items()
        if statuses and all(status in _TRAINING_READY_DSL_STATUSES for status in statuses)
    }

# Deck source priority for within-archetype selection (higher = preferred).
# DigimonMeta lists are highly-optimised top-cut builds.
_SOURCE_PREFERENCE = {"digimonmeta": 3, "egman": 2, "digimoncard_io": 1, "file": 0, "manual": 0, "test": 0}
SNAPSHOT_SCHEMA_VERSION = 1


class UnimplementedDeckError(ValueError):
    """Raised when a training deck contains cards outside the Rust registry."""


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
    ) -> "GeneralistDeckPool":
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
    fully_implemented_archetypes: Optional[Set[str]] = None,
    allowed_archetypes: Optional[Set[str]] = None,
) -> GeneralistDeckPool:
    """Load the same implementation-safe eligible pool used by MetaGauntlet."""
    gauntlet = MetaGauntlet(
        sampling_mode="random",
        implemented_card_ids=implemented_card_ids,
        fully_implemented_archetypes=fully_implemented_archetypes,
        allowed_archetypes=allowed_archetypes,
    )
    gauntlet.load(path)
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
      - "random": sample uniformly across all fully implemented decklists.
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
        fully_implemented_archetypes: Optional[Set[str]] = None,
        allowed_archetypes: Optional[Set[str]] = None,
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
        self._fully_implemented_archetypes = fully_implemented_archetypes
        self._allowed_archetypes = allowed_archetypes

        self.archetypes: Dict[str, ArchetypeStats] = {}
        self._deck_pool: List[DeckEntry] = []
        self._weights: Optional[np.ndarray] = None
        self._rng = np.random.default_rng(seed)

    def load(self, path: str = DECK_LIBRARY_PATH) -> None:
        """Load deck library JSON and compute threat indices + sampling weights.

        Statistical weights come ONLY from digilab_stats.  Scraper stats
        (meta_share, conversion_rate in the top-level ``stats`` block) are
        ignored for TI computation to avoid survivorship bias.
        """
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
                else load_implemented_card_ids()
            )
        fully_implemented_archetypes = self._fully_implemented_archetypes
        if fully_implemented_archetypes is None and os.path.abspath(path) == os.path.abspath(DECK_LIBRARY_PATH):
            fully_implemented_archetypes = _load_fully_implemented_archetypes()

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
            if fully_implemented_archetypes is not None:
                dropped_by_dsl = (
                    canonical_allowed & library_canonical
                ) - fully_implemented_archetypes
                for name in sorted(dropped_by_dsl):
                    logger.info(
                        "allowed_archetypes entry %r excluded: not in DSL-implemented set",
                        name,
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

        # Second pass: build archetype stats and deck pools
        for arch_name, group in canonical_groups.items():
            if (
                fully_implemented_archetypes is not None
                and arch_name not in fully_implemented_archetypes
            ):
                logger.debug("Skipping non-ready DSL archetype %s", arch_name)
                continue
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
