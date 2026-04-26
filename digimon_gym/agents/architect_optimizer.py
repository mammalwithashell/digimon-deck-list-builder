"""MetaOptimizer — orchestrates architect agent training against a target meta.

Ties together the CandidatePool, DeckSimulator, DeckBuildingEnv, and
ArchitectDQN into a complete training and inference pipeline. Given a base
deck, a meta configuration (archetype shares), and a pilot policy, the
optimizer:

1. Builds a weighted opponent list from the deck library.
2. Trains a DQN agent to make card-swap decisions that maximise win rate.
3. Produces an optimized decklist with full provenance (swap history,
   win-rate delta, training metadata).

No DB imports. No auth. Pure engine + training.
"""

from __future__ import annotations

import json
import logging
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

import numpy as np

from digimon_gym.agents.architect_agent import ArchitectDQN
from digimon_gym.agents.architect_pool import CandidatePool, CardConstraint
from digimon_gym.agents.architect_simulator import DeckSimulator
from digimon_engine import load_implemented_card_ids

logger = logging.getLogger(__name__)

from digimon_gym.data_paths import DECK_LIBRARY as _DECK_LIBRARY_PATH

# Prefer decklists from sources with better data quality.
_SOURCE_PREFERENCE: Dict[str, int] = {
    "digilab": 3,
    "digimonmeta": 2,
    "egman": 1,
}


def _pick_best_decklist(
    decklists: List[dict],
    implemented: set,
) -> Optional[List[str]]:
    """Pick the best decklist that has 100% card coverage.

    Selects the decklist with the highest source preference whose cards
    are all present in the frozen manifest.  Returns ``None`` if no
    decklist qualifies.
    """
    candidates: List[Tuple[int, List[str]]] = []

    for entry in decklists:
        source = entry.get("source", "")
        pref = _SOURCE_PREFERENCE.get(source, 0)

        raw = entry.get("decklist", "[]")
        try:
            deck_ids: List[str] = json.loads(raw)
        except (json.JSONDecodeError, TypeError):
            continue

        if not deck_ids:
            continue

        # Check full coverage
        if all(cid in implemented for cid in deck_ids):
            candidates.append((pref, deck_ids))

    if not candidates:
        return None

    # Highest source preference wins; ties broken arbitrarily.
    candidates.sort(key=lambda x: x[0], reverse=True)
    return candidates[0][1]


class MetaOptimizer:
    """Orchestrates architect agent training against a target meta."""

    def __init__(
        self,
        archetype_name: str,
        base_deck: List[str],
        meta_config: dict,
        pilot_policy_path: str = "greedy",
        output_dir: str = "architect_runs",
        extra_cards: Optional[List[str]] = None,
        custom_pool: Optional[List[str]] = None,
        card_constraints: Optional[Dict[str, CardConstraint]] = None,
        use_restricted_list: bool = False,
    ):
        """
        Args:
            archetype_name: Target archetype name.
            base_deck: Starting deck (flat card ID list).
            meta_config: Dict with ``'archetypes'`` key mapping archetype
                names to stats with ``local_meta_share`` and/or
                ``global_meta_share``.
            pilot_policy_path: Path to pilot model or ``"greedy"``/``"random"``.
            output_dir: Directory to save training outputs.
            extra_cards: Additional card IDs for the candidate pool.
            custom_pool: Custom candidate pool (replaces archetype-derived).
            card_constraints: Per-card constraints (locks, min/max counts).
            use_restricted_list: If True, enforce the official ban/restricted list.
        """
        self.archetype_name = archetype_name
        self.base_deck = list(base_deck)
        self.meta_config = meta_config
        self.pilot_policy_path = pilot_policy_path
        self.output_dir = output_dir
        self.extra_cards = extra_cards
        self.custom_pool = custom_pool
        self.card_constraints = card_constraints
        self.use_restricted_list = use_restricted_list

        # Lazily initialised during train() / optimize_deck()
        self._pool: Optional[CandidatePool] = None
        self._agent: Optional[ArchitectDQN] = None
        self._simulator: Optional[DeckSimulator] = None
        self._opponents: Optional[List[Tuple[List[str], float]]] = None
        self._run_dir: Optional[Path] = None

        # Training metadata accumulated during train()
        self._metadata: Dict[str, Any] = {}

    # ------------------------------------------------------------------
    # Opponent construction
    # ------------------------------------------------------------------

    def build_opponents(self) -> List[Tuple[List[str], float]]:
        """Build opponent deck list with meta weights.

        Cross-references ``meta_config`` with ``deck_library.json``:

        1. For each archetype in meta_config (excluding self), find its
           decklists in the deck library.
        2. Pick the best available decklist by source preference
           (digilab > digimonmeta > egman).
        3. Filter to decks where all cards are simulatable (present in
           the frozen manifest).
        4. Return list of ``(deck_ids, meta_weight)`` tuples.
        """
        if not _DECK_LIBRARY_PATH.exists():
            logger.warning(
                "Deck library not found at %s; no opponents available.",
                _DECK_LIBRARY_PATH,
            )
            return []

        with open(_DECK_LIBRARY_PATH, "r", encoding="utf-8") as f:
            deck_library = json.load(f)

        implemented = load_implemented_card_ids()
        library_archetypes = deck_library.get("archetypes", {})

        opponents: List[Tuple[List[str], float]] = []
        for arch_name, arch_stats in self.meta_config.get("archetypes", {}).items():
            if arch_name == self.archetype_name:
                continue  # Skip self

            # Get meta weight (prefer local, fall back to global)
            weight = arch_stats.get("local_meta_share") or arch_stats.get(
                "global_meta_share", 0
            )
            if weight <= 0:
                continue

            # Find decklists in deck library
            lib_arch = library_archetypes.get(arch_name)
            if not lib_arch or not lib_arch.get("decklists"):
                logger.debug(
                    "Archetype %r not found in deck library; skipping.", arch_name
                )
                continue

            best_deck = _pick_best_decklist(lib_arch["decklists"], implemented)
            if best_deck is None:
                logger.debug(
                    "No fully-simulatable decklist for %r; skipping.", arch_name
                )
                continue

            opponents.append((best_deck, weight))

        logger.info(
            "Built %d opponents from meta config (%d archetypes skipped).",
            len(opponents),
            len(self.meta_config.get("archetypes", {})) - len(opponents) - 1,
        )
        return opponents

    # ------------------------------------------------------------------
    # Training
    # ------------------------------------------------------------------

    def train(
        self,
        episodes: int = 200,
        games_per_eval: int = 50,
        max_swaps: int = 10,
        n_workers: int = 1,
        reward_amplification: float = 10.0,
        checkpoint_freq: int = 50,
        checkpoint_games: int = 200,
        seed: Optional[int] = None,
    ) -> dict:
        """Run the full training loop.

        Args:
            episodes: Number of training episodes.
            games_per_eval: Games per matchup during in-episode evaluation.
            max_swaps: Maximum card swaps per episode.
            n_workers: Parallel workers for simulation.
            reward_amplification: Reward scaling factor.
            checkpoint_freq: Episodes between checkpoints.
            checkpoint_games: Games per matchup for checkpoint evaluation.
            seed: Random seed for reproducibility.

        Returns:
            Metadata dict with training results.
        """
        from digimon_gym.agents.architect_env import DeckBuildingEnv

        if seed is not None:
            np.random.seed(seed)

        # -- Setup --
        timestamp = datetime.now(timezone.utc).strftime("%Y%m%d_%H%M%S")
        self._run_dir = Path(self.output_dir) / f"{self.archetype_name}_{timestamp}"
        self._run_dir.mkdir(parents=True, exist_ok=True)

        # Build components
        self._pool = CandidatePool(
            self.base_deck,
            self.archetype_name,
            extra_cards=self.extra_cards,
            custom_pool=self.custom_pool,
            card_constraints=self.card_constraints,
            use_restricted_list=self.use_restricted_list,
        )

        self._opponents = self.build_opponents()
        if not self._opponents:
            raise RuntimeError(
                "No valid opponents could be built from meta config. "
                "Check that deck_library.json has simulatable decklists "
                "for the archetypes in meta_config."
            )

        self._simulator = DeckSimulator(
            pilot_policy_path=self.pilot_policy_path,
            opponent_policy="greedy",
            games_per_eval=games_per_eval,
            max_turns=200,
            n_workers=n_workers,
            cache_enabled=True,
        )

        env = DeckBuildingEnv(
            pool=self._pool,
            simulator=self._simulator,
            opponents=self._opponents,
            base_deck=self.base_deck,
            max_swaps=max_swaps,
            reward_amplification=reward_amplification,
        )

        state_dim = self._pool.n_candidates
        action_dim = self._pool.action_size

        self._agent = ArchitectDQN(
            state_dim=state_dim,
            action_dim=action_dim,
        )

        # -- Evaluate baseline --
        base_win_rate = self._simulator.evaluate_deck(
            self.base_deck, self._opponents
        )
        logger.info("Base deck win rate: %.3f", base_win_rate)

        # -- Training loop --
        best_win_rate = base_win_rate
        best_deck: Optional[List[str]] = None
        episode_log: List[Dict[str, Any]] = []

        for ep in range(1, episodes + 1):
            obs, info = env.reset(seed=seed + ep if seed is not None else None)
            mask = info["action_mask"]
            ep_reward = 0.0
            ep_loss_sum = 0.0
            ep_loss_count = 0

            for _t in range(max_swaps):
                action = self._agent.select_action(obs, mask)
                next_obs, reward, terminated, truncated, next_info = env.step(action)
                next_mask = next_info["action_mask"]
                done = terminated or truncated

                self._agent.store_transition(
                    obs, action, reward, next_obs, done, mask, next_mask
                )

                loss = self._agent.update()
                if loss is not None:
                    ep_loss_sum += loss
                    ep_loss_count += 1

                ep_reward += reward
                obs = next_obs
                mask = next_mask

                if done:
                    break

            self._agent.decay_epsilon()

            win_rate = env.last_win_rate
            n_swaps = len(env.swap_history)
            avg_loss = ep_loss_sum / ep_loss_count if ep_loss_count > 0 else 0.0

            episode_log.append({
                "episode": ep,
                "epsilon": self._agent.epsilon,
                "win_rate": win_rate,
                "reward": ep_reward,
                "loss": avg_loss,
                "n_swaps": n_swaps,
            })

            # Console logging every 10 episodes
            if ep % 10 == 0 or ep == 1:
                logger.info(
                    "[Episode %03d] \u03b5=%.3f  WR=%.3f  Reward=%.2f  "
                    "Loss=%.4f  Swaps=%d",
                    ep,
                    self._agent.epsilon,
                    win_rate,
                    ep_reward,
                    avg_loss,
                    n_swaps,
                )

            # Track best
            if win_rate > best_win_rate:
                best_win_rate = win_rate
                best_deck = list(env.current_deck_list)

            # Checkpoint
            if ep % checkpoint_freq == 0:
                self._checkpoint(ep, checkpoint_games, env)

        # -- Final save --
        final_win_rate = win_rate  # noqa: F841 — last episode's win rate

        self._metadata = {
            "archetype": self.archetype_name,
            "pilot_policy": self.pilot_policy_path,
            "episodes": episodes,
            "final_win_rate": float(final_win_rate),
            "base_win_rate": float(base_win_rate),
            "best_win_rate": float(best_win_rate),
            "training_params": {
                "games_per_eval": games_per_eval,
                "max_swaps": max_swaps,
                "n_workers": n_workers,
                "reward_amplification": reward_amplification,
                "checkpoint_freq": checkpoint_freq,
                "seed": seed,
                "state_dim": state_dim,
                "action_dim": action_dim,
            },
            "agent_config": self._agent.get_config(),
            "n_opponents": len(self._opponents),
            "episode_log": episode_log,
            "timestamp": datetime.now(timezone.utc).isoformat(),
        }

        if best_deck is not None:
            self._metadata["optimized_deck"] = best_deck

        self.save()

        logger.info(
            "Training complete. Base WR=%.3f  Best WR=%.3f  "
            "Artifacts saved to %s",
            base_win_rate,
            best_win_rate,
            self._run_dir,
        )

        return self._metadata

    # ------------------------------------------------------------------
    # Inference
    # ------------------------------------------------------------------

    def optimize_deck(
        self,
        base_deck: Optional[List[str]] = None,
        max_swaps: int = 10,
    ) -> dict:
        """Use trained agent to optimize a deck (inference mode).

        Args:
            base_deck: Starting deck. If ``None``, uses the training base deck.
            max_swaps: Maximum number of swaps.

        Returns:
            Dict with ``optimized_deck``, ``base_win_rate``,
            ``optimized_win_rate``, ``swaps``, and ``n_swaps``.
        """
        from digimon_gym.agents.architect_env import DeckBuildingEnv

        if self._agent is None:
            raise RuntimeError(
                "No trained agent available. Call train() first or "
                "load a saved run with MetaOptimizer.load()."
            )
        if self._pool is None:
            raise RuntimeError("CandidatePool not initialised.")
        if self._simulator is None or self._opponents is None:
            raise RuntimeError(
                "Simulator/opponents not initialised. Call train() "
                "first or load a saved run."
            )

        deck = list(base_deck) if base_deck is not None else list(self.base_deck)

        # Evaluate baseline
        base_wr = self._simulator.evaluate_deck(deck, self._opponents)

        env = DeckBuildingEnv(
            pool=self._pool,
            simulator=self._simulator,
            opponents=self._opponents,
            base_deck=deck,
            max_swaps=max_swaps,
            reward_amplification=1.0,  # irrelevant for inference
        )

        obs, info = env.reset()
        mask = info["action_mask"]

        for _t in range(max_swaps):
            action = self._agent.greedy_action(obs, mask)
            obs, _reward, terminated, truncated, info = env.step(action)
            mask = info["action_mask"]
            if terminated or truncated:
                break

        optimized_wr = env.last_win_rate

        return {
            "optimized_deck": list(env.current_deck_list),
            "base_win_rate": float(base_wr),
            "optimized_win_rate": float(optimized_wr),
            "swaps": list(env.swap_history),
            "n_swaps": len(env.swap_history),
        }

    # ------------------------------------------------------------------
    # Persistence
    # ------------------------------------------------------------------

    def save(self) -> None:
        """Save all training artifacts to output_dir."""
        if self._run_dir is None:
            timestamp = datetime.now(timezone.utc).strftime("%Y%m%d_%H%M%S")
            self._run_dir = Path(self.output_dir) / f"{self.archetype_name}_{timestamp}"
        self._run_dir.mkdir(parents=True, exist_ok=True)

        # Agent weights
        if self._agent is not None:
            self._agent.save(str(self._run_dir / "architect_dqn.pt"))

        # Candidate pool
        if self._pool is not None:
            with open(self._run_dir / "candidate_pool.json", "w", encoding="utf-8") as f:
                json.dump(self._pool.to_json(), f, indent=2)

        # Meta config
        with open(self._run_dir / "meta_config.json", "w", encoding="utf-8") as f:
            json.dump(self.meta_config, f, indent=2)

        # Training metadata
        if self._metadata:
            with open(self._run_dir / "metadata.json", "w", encoding="utf-8") as f:
                json.dump(self._metadata, f, indent=2)

        # Best optimized deck
        optimized = self._metadata.get("optimized_deck")
        if optimized:
            with open(self._run_dir / "optimized_deck.json", "w", encoding="utf-8") as f:
                json.dump(
                    {
                        "archetype": self.archetype_name,
                        "deck": optimized,
                        "base_win_rate": self._metadata.get("base_win_rate"),
                        "best_win_rate": self._metadata.get("best_win_rate"),
                    },
                    f,
                    indent=2,
                )

        logger.info("Saved artifacts to %s", self._run_dir)

    @classmethod
    def load(
        cls,
        run_dir: str,
        pilot_policy_path: str = "greedy",
    ) -> "MetaOptimizer":
        """Load a trained MetaOptimizer from a saved run directory.

        Args:
            run_dir: Path to the saved run directory.
            pilot_policy_path: Pilot policy to use for simulation.

        Returns:
            A ``MetaOptimizer`` with agent, pool, and simulator restored.
        """
        run_path = Path(run_dir)

        # Load metadata
        with open(run_path / "metadata.json", "r", encoding="utf-8") as f:
            metadata = json.load(f)

        # Load meta config
        with open(run_path / "meta_config.json", "r", encoding="utf-8") as f:
            meta_config = json.load(f)

        # Load candidate pool
        with open(run_path / "candidate_pool.json", "r", encoding="utf-8") as f:
            pool_data = json.load(f)
        pool = CandidatePool.from_json(pool_data)

        # Load agent
        agent = ArchitectDQN.load(str(run_path / "architect_dqn.pt"))

        # Reconstruct base deck from optimized_deck.json or metadata
        base_deck: List[str] = []
        opt_path = run_path / "optimized_deck.json"
        if opt_path.exists():
            with open(opt_path, "r", encoding="utf-8") as f:
                opt_data = json.load(f)
            # The optimized deck is stored; base deck is the training input.
            # We don't persist the base deck separately, so fall back to
            # the optimized deck as a starting point for further optimization.
            base_deck = opt_data.get("deck", [])
        elif "optimized_deck" in metadata:
            base_deck = metadata["optimized_deck"]

        archetype = metadata.get("archetype", pool_data.get("archetype_name", "unknown"))

        optimizer = cls(
            archetype_name=archetype,
            base_deck=base_deck,
            meta_config=meta_config,
            pilot_policy_path=pilot_policy_path,
            output_dir=str(run_path.parent),
        )

        optimizer._pool = pool
        optimizer._agent = agent
        optimizer._run_dir = run_path
        optimizer._metadata = metadata

        # Rebuild opponents and simulator for inference
        optimizer._opponents = optimizer.build_opponents()

        training_params = metadata.get("training_params", {})
        optimizer._simulator = DeckSimulator(
            pilot_policy_path=pilot_policy_path,
            opponent_policy="greedy",
            games_per_eval=training_params.get("games_per_eval", 50),
            max_turns=200,
            n_workers=training_params.get("n_workers", 1),
            cache_enabled=True,
        )

        logger.info(
            "Loaded MetaOptimizer from %s (archetype=%s, episodes=%d, "
            "best_wr=%.3f)",
            run_dir,
            archetype,
            metadata.get("episodes", 0),
            metadata.get("best_win_rate", 0),
        )

        return optimizer

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    def _checkpoint(
        self,
        episode: int,
        checkpoint_games: int,
        env: Any,
    ) -> None:
        """Save a checkpoint and run a full evaluation."""
        assert self._run_dir is not None
        assert self._agent is not None
        assert self._simulator is not None
        assert self._opponents is not None

        # Save agent checkpoint
        ckpt_path = self._run_dir / f"checkpoint_ep{episode:04d}.pt"
        self._agent.save(str(ckpt_path))

        # Run full evaluation with more games
        orig_games = self._simulator.games_per_eval
        self._simulator.games_per_eval = checkpoint_games
        self._simulator.clear_cache()

        wr = self._simulator.evaluate_deck(
            list(env.current_deck_list), self._opponents
        )

        self._simulator.games_per_eval = orig_games

        logger.info(
            "[Checkpoint %04d] Full eval WR=%.3f  (deck size=%d)",
            episode,
            wr,
            len(env.current_deck_list),
        )
