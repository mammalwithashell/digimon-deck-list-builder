"""MetaGauntlet: meta-weighted opponent deck sampling for RL training.

Loads the deck library (produced by tools/meta_loader.py), computes a Threat
Index for each archetype, and samples opponent decks weighted by TI.

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
from dataclasses import dataclass, field
from typing import Dict, List, Optional

import numpy as np
import gymnasium

from digimon_gym.engine.data.deck_loader import parse_tts

logger = logging.getLogger(__name__)

from digimon_gym.data_paths import (
    ARCHETYPE_ALIASES as _ARCHETYPE_ALIASES_PATH,
    DECK_LIBRARY as _DECK_LIBRARY_PATH,
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


def canonicalize_archetype(name: str) -> str:
    """Resolve an archetype name to its canonical form via the alias index."""
    global _ALIAS_MAP
    if _ALIAS_MAP is None:
        _ALIAS_MAP = _load_alias_map()
    return _ALIAS_MAP.get(name.lower(), name)

# Deck source priority for within-archetype selection (higher = preferred).
# DigimonMeta lists are highly-optimised top-cut builds.
_SOURCE_PREFERENCE = {"digimonmeta": 3, "egman": 2, "digimoncard_io": 1, "file": 0, "manual": 0, "test": 0}


# ─── Data Classes ────────────────────────────────────────────────────

@dataclass
class DeckEntry:
    """A concrete deck that can be played in training."""
    deck_id: str
    archetype_name: str
    card_ids: List[str]  # Flat list ready for HeadlessGame
    source: str = ""
    threat_index: float = 0.0


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
    """

    def __init__(
        self,
        alpha: float = 1.0,
        beta: float = 2.0,
        sleeper_threshold: float = 0.50,
        sleeper_floor: float = 0.05,
        confidence_min_appearances: int = 5,
        seed: Optional[int] = None,
    ) -> None:
        self.alpha = alpha
        self.beta = beta
        self.sleeper_threshold = sleeper_threshold
        self.sleeper_floor = sleeper_floor
        self.confidence_min_appearances = confidence_min_appearances

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
                # Parse TTS format decklist (JSON array string from digimoncard.io export)
                tts_str = dl.get("decklist", "")
                if tts_str:
                    try:
                        card_ids = parse_tts(tts_str)
                    except ValueError:
                        logger.warning("Invalid TTS decklist in %s, skipping", dl.get("deck_id", "?"))
                        continue
                else:
                    # Backward compat: fall back to legacy card_ids field
                    card_ids = dl.get("card_ids", [])
                if not card_ids:
                    continue
                decks.append(DeckEntry(
                    deck_id=dl.get("deck_id", ""),
                    archetype_name=arch_name,
                    card_ids=card_ids,
                    source=dl.get("source", ""),
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
