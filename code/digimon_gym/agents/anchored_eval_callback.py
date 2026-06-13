"""In-training anchored evaluation (`harden-training-pipeline` D2).

A dedicated SB3 callback that periodically scores the live policy against
FIXED anchors — greedy plus every layout-compatible champion from the
registry — seat-balanced, independent of the training opponent. This is the
trustworthy in-run progress signal the training-opponent eval cannot give
(CLAUDE.md rule 30): a collapsing run becomes visible within 1–2 panels
instead of after the run ends.

Separate from ``WinRateCallback`` on purpose: different opponents, different
cost profile, different cadence — and either can be disabled without the
other. Results land as ``pilot/anchored/*`` TensorBoard scalars plus one row
per panel in ``anchored_evals.jsonl`` (NEVER ``evals.jsonl`` — existing
readers assume one training-eval row shape there).

Failure containment: a crash inside any panel game (engine panic included)
marks that anchor failed in the sidecar row and the run continues — the
panel must never abort training.
"""
from __future__ import annotations

import json
import logging
import time
from pathlib import Path
from typing import Callable, List, Optional, Tuple

from stable_baselines3.common.callbacks import BaseCallback

from digimon_gym.digimon_gym import greedy_policy
from digimon_gym.agents.anchored_eval import MatchRecord, play_matchup
from digimon_gym.agents.champion_registry import ChampionRegistry

logger = logging.getLogger(__name__)

#: Default archetype fallback when a run has neither a generalist pool nor
#: explicit decks — the six starter decks.
DEFAULT_STARTER_ARCHETYPES = [
    "ST-1 Gaia Red", "ST-2 Cocytus Blue", "ST-3 Heaven's Yellow",
    "ST-4 Giga Green", "ST-5 Machine Black", "ST-6 Venomous Violet",
]

ANCHORED_SIDECAR_FILENAME = "anchored_evals.jsonl"


def _sanitize_tag(name: str) -> str:
    """TensorBoard-safe tag fragment for an anchor name."""
    return "".join(c if (c.isalnum() or c in "-_.") else "_" for c in name)


def build_anchored_deck_pool(
    generalist_deck_pool=None,
    deck1: Optional[List[str]] = None,
    deck2: Optional[List[str]] = None,
):
    """Resolve the deck pool the panel samples from.

    Fallback ladder (spec `in-training-anchored-eval`): the run's own
    generalist pool (which already reflects the frozen snapshot) → the
    run's explicit decks → the six starter archetypes.
    """
    if generalist_deck_pool is not None:
        return generalist_deck_pool

    from digimon_gym.agents.gauntlet import (
        DeckEntry,
        GeneralistDeckPool,
        load_generalist_deck_pool,
    )

    run_decks = [d for d in (deck1, deck2) if d]
    if run_decks:
        entries = [
            DeckEntry(f"run-deck-{i + 1}", "run-decks", list(deck))
            for i, deck in enumerate(run_decks)
        ]
        return GeneralistDeckPool(archetypes={"run-decks": entries})

    return load_generalist_deck_pool(
        allowed_archetypes=set(DEFAULT_STARTER_ARCHETYPES)
    )


class AnchoredEvalCallback(BaseCallback):
    """Periodic anchored panel against greedy + layout-compatible champions.

    Args:
        freq: env-steps between panels (caller should not attach the
            callback at all when the configured freq is 0).
        games: seat-balanced games per anchor per panel.
        registry_path: champion registry JSON.
        layout_hash: the run's tensor-layout hash (champion cohort filter).
        tensor_profile: observation profile for panel games.
        deck_pool_factory: zero-arg callable resolving the panel deck pool
            (lazy so a disabled/never-fired callback does no loading).
        sidecar_path: target ``anchored_evals.jsonl`` path (None disables
            the sidecar but scalars still emit).
        include_greedy: include the greedy anchor (default True).
        base_seed: seed root; per-panel seeds derive from it + panel index.
    """

    def __init__(
        self,
        freq: int,
        games: int,
        registry_path: str | Path,
        layout_hash: str,
        tensor_profile: str,
        deck_pool_factory: Callable[[], object],
        sidecar_path: Optional[Path] = None,
        include_greedy: bool = True,
        base_seed: int = 777,
        verbose: int = 0,
    ):
        super().__init__(verbose)
        self.freq = int(freq)
        self.games = int(games)
        self.registry_path = Path(registry_path)
        self.layout_hash = layout_hash
        self.tensor_profile = tensor_profile
        self._deck_pool_factory = deck_pool_factory
        self.sidecar_path = Path(sidecar_path) if sidecar_path else None
        self.include_greedy = include_greedy
        self.base_seed = int(base_seed)

        self._last_panel_step = 0
        self._panel_idx = 0
        self._deck_pool = None
        # [(anchor_name, opponent_fn)] — built lazily at the first panel;
        # champion weights load once and are reused across panels.
        self._anchor_fns: Optional[List[Tuple[str, Callable]]] = None
        self.excluded_champions: List[str] = []
        self.last_panel_results: dict = {}

    # ── anchor resolution ────────────────────────────────────────────

    def _on_training_start(self) -> None:
        # Log the anchor roster (and cohort exclusions) once at run start
        # without loading any champion weights yet.
        registry = ChampionRegistry.load(self.registry_path)
        compatible = registry.compatible(self.layout_hash)
        compatible_names = {c.name for c in compatible}
        self.excluded_champions = [
            c.name for c in registry.champions if c.name not in compatible_names
        ]
        if self.excluded_champions:
            logger.warning(
                "Anchored eval: excluding layout-incompatible champions %s "
                "(run layout %s).",
                self.excluded_champions,
                self.layout_hash,
            )
        anchor_names = (["greedy"] if self.include_greedy else []) + sorted(
            compatible_names
        )
        if self.verbose:
            print(
                f"  Anchored eval:  every {self.freq} steps, "
                f"{self.games} games/anchor vs {anchor_names or '(no anchors)'}"
            )

    def _build_anchor_fns(self) -> List[Tuple[str, Callable]]:
        from digimon_gym.agents.pilot_training import make_agent_opponent_fn

        anchors: List[Tuple[str, Callable]] = []
        if self.include_greedy:
            anchors.append(("greedy", greedy_policy))
        registry = ChampionRegistry.load(self.registry_path)
        for champ in registry.compatible(self.layout_hash):
            try:
                fn = make_agent_opponent_fn(
                    champ.weights_path, algorithm=champ.algorithm
                )
            except Exception:
                logger.exception(
                    "Anchored eval: failed to load champion %s; "
                    "anchor dropped for this run.",
                    champ.name,
                )
                continue
            anchors.append((champ.name, fn))
        return anchors

    def _pool(self):
        if self._deck_pool is None:
            self._deck_pool = self._deck_pool_factory()
        return self._deck_pool

    # ── panel execution ──────────────────────────────────────────────

    def _on_step(self) -> bool:
        if self.freq <= 0:
            return True
        if self.num_timesteps - self._last_panel_step < self.freq:
            return True
        self._last_panel_step = self.num_timesteps
        try:
            self._run_panel()
        except Exception:
            # Belt-and-suspenders: even a bug in panel bookkeeping must
            # never abort training.
            logger.exception("Anchored eval panel failed; training continues.")
        return True

    def _run_panel(self) -> None:
        t0 = time.time()
        if self._anchor_fns is None:
            self._anchor_fns = self._build_anchor_fns()
        panel_seed = self.base_seed + self._panel_idx * 100_003
        results: dict[str, MatchRecord] = {}
        failed: List[str] = []
        for name, fn in self._anchor_fns:
            try:
                results[name] = play_matchup(
                    self.model,
                    fn,
                    self._pool(),
                    self.games,
                    panel_seed,
                    self.tensor_profile,
                )
            except Exception:
                logger.exception(
                    "Anchored eval: anchor %r failed at step %d; marked "
                    "failed, panel continues.",
                    name,
                    self.num_timesteps,
                )
                failed.append(name)
        elapsed = time.time() - t0

        win_rates = [rec.win_rate for rec in results.values()]
        panel_mean = sum(win_rates) / len(win_rates) if win_rates else 0.0

        for name, rec in results.items():
            self.logger.record(
                f"pilot/anchored/{_sanitize_tag(name)}/win_rate", rec.win_rate
            )
        self.logger.record("pilot/anchored/panel_mean", panel_mean)

        row = {
            "step": int(self.num_timesteps),
            "wall_time": time.time(),
            "panel_idx": self._panel_idx,
            "games_per_anchor": self.games,
            "panel_seconds": round(elapsed, 3),
            "panel_mean_win_rate": panel_mean,
            "anchors": {
                name: {
                    "wins": rec.wins,
                    "losses": rec.losses,
                    "draws": rec.draws,
                    "win_rate": rec.win_rate,
                    "failed": False,
                }
                for name, rec in results.items()
            },
            "excluded_champions": self.excluded_champions,
        }
        for name in failed:
            row["anchors"][name] = {
                "wins": 0, "losses": 0, "draws": 0,
                "win_rate": None, "failed": True,
            }
        if self.sidecar_path is not None:
            self.sidecar_path.parent.mkdir(parents=True, exist_ok=True)
            with open(self.sidecar_path, "a", encoding="utf-8") as fh:
                fh.write(json.dumps(row, separators=(",", ":")) + "\n")

        self.last_panel_results = row
        self._panel_idx += 1
        if self.verbose:
            parts = ", ".join(
                f"{name}={100 * rec.win_rate:.0f}%" for name, rec in results.items()
            )
            print(
                f"  Anchored panel @ {self.num_timesteps}: {parts or 'no anchors'}"
                f"  ({elapsed:.0f}s)"
            )
