"""Architect co-training — iterative architect/pilot loops and multi-archetype gauntlet.

Two modes:
1. **CoTrainer**: alternates architect deck-optimization and pilot training cycles
   for a single archetype, converging toward a jointly-optimal (deck, policy) pair.
2. **ArchitectGauntlet**: runs multiple archetypes through co-training rounds with
   round-robin evaluation, producing ETWR-ranked meta standings.

Usage:
    # Single archetype co-training
    python -m digimon_gym.agents.architect_cotraining \
        --archetype "Medusamon" \
        --meta local_meta_dfw.json \
        --initial-pilot models/mlp_agent.onnx \
        --cycles 5 \
        --architect-episodes 100 \
        --pilot-timesteps 200000

    # Multi-archetype gauntlet
    python -m digimon_gym.agents.architect_cotraining \
        --gauntlet \
        --archetypes "Medusamon,Rocks,Zephagamon,TS Jupitermon,Millenniummon" \
        --meta local_meta_dfw.json \
        --initial-pilot models/mlp_agent.onnx \
        --rounds 3

No DB imports. No auth. Pure engine + training.
"""

from __future__ import annotations

import argparse
import json
import logging
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Deck resolution (duplicated from architect_training to avoid circular imports)
# ---------------------------------------------------------------------------

_DECK_LIBRARY_PATH = (
    Path(__file__).resolve().parent.parent
    / "engine" / "data" / "deck_library.json"
)

_SOURCE_PREFERENCE = {
    "digilab": 3,
    "digimonmeta": 2,
    "egman": 1,
}


def _resolve_base_deck(archetype_name: str) -> List[str]:
    """Find the best decklist for the given archetype from deck_library.

    Prefers decklists from higher-quality sources (digilab > digimonmeta > egman).
    """
    if not _DECK_LIBRARY_PATH.exists():
        raise FileNotFoundError(
            f"deck_library.json not found at {_DECK_LIBRARY_PATH}"
        )

    with open(_DECK_LIBRARY_PATH, "r", encoding="utf-8") as f:
        library = json.load(f)

    arch = library.get("archetypes", {}).get(archetype_name)
    if not arch or not arch.get("decklists"):
        raise ValueError(f"No decklists found for archetype: {archetype_name}")

    decklists = sorted(
        arch["decklists"],
        key=lambda d: _SOURCE_PREFERENCE.get(d.get("source", ""), 0),
        reverse=True,
    )

    for entry in decklists:
        raw = entry.get("decklist")
        if not raw:
            continue
        deck = json.loads(raw) if isinstance(raw, str) else raw
        if deck:
            return deck

    raise ValueError(f"No valid decklists for archetype: {archetype_name}")


def _build_default_meta_config() -> dict:
    """Build a meta config from deck_library.json global stats."""
    if not _DECK_LIBRARY_PATH.exists():
        raise FileNotFoundError(
            f"deck_library.json not found at {_DECK_LIBRARY_PATH}"
        )

    with open(_DECK_LIBRARY_PATH, "r", encoding="utf-8") as f:
        library = json.load(f)

    archetypes = {}
    for arch_name, arch_data in library.get("archetypes", {}).items():
        stats = arch_data.get("stats", {})
        meta_share = stats.get("meta_share", 0)
        if meta_share > 0.005:
            archetypes[arch_name] = {
                "global_meta_share": meta_share,
                "global_conversion_rate": stats.get("conversion_rate", 0),
            }

    return {"archetypes": archetypes}


# ---------------------------------------------------------------------------
# CoTrainer
# ---------------------------------------------------------------------------

class CoTrainer:
    """Iterative architect/pilot co-training loop for a single archetype.

    Each cycle:
    1. **Architect phase** — trains the deck-building DQN with the current pilot
       policy to produce an optimized decklist.
    2. **Pilot phase** — retrains the pilot PPO on the optimized deck to produce
       a new policy checkpoint.
    3. **ONNX export** — converts the pilot .zip to .onnx for the next architect
       cycle (the architect simulator needs an ONNX or named policy).

    Early-stops when the win-rate improvement falls below *convergence_threshold*
    for two consecutive cycles.
    """

    def __init__(
        self,
        archetype_name: str,
        base_deck: List[str],
        meta_config: dict,
        initial_pilot_path: str = "greedy",
        output_dir: str = "cotraining_runs",
        extra_cards: Optional[List[str]] = None,
    ):
        """
        Args:
            archetype_name: Target archetype name.
            base_deck: Starting deck (flat card ID list).
            meta_config: Dict with ``'archetypes'`` key mapping archetype names
                to stats with ``local_meta_share`` and/or ``global_meta_share``.
            initial_pilot_path: Pilot model path or ``"greedy"``/``"random"``.
            output_dir: Root directory for co-training artifacts.
            extra_cards: Additional card IDs for the architect candidate pool.
        """
        self.archetype_name = archetype_name
        self.base_deck = list(base_deck)
        self.meta_config = meta_config
        self.initial_pilot_path = initial_pilot_path
        self.output_dir = output_dir
        self.extra_cards = extra_cards

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def run(
        self,
        cycles: int = 5,
        architect_episodes: int = 100,
        architect_games_per_eval: int = 50,
        architect_max_swaps: int = 10,
        pilot_timesteps: int = 200_000,
        n_workers: int = 1,
        convergence_threshold: float = 0.01,
        seed: Optional[int] = None,
    ) -> dict:
        """Run co-training cycles.

        Args:
            cycles: Maximum number of architect/pilot cycles.
            architect_episodes: DQN training episodes per architect phase.
            architect_games_per_eval: Games per matchup during architect evaluation.
            architect_max_swaps: Maximum card swaps per architect episode.
            pilot_timesteps: PPO training timesteps per pilot phase.
            n_workers: Parallel simulation workers for architect evaluation.
            convergence_threshold: Stop early if win-rate improvement is below
                this value for two consecutive cycles.
            seed: Random seed for reproducibility.

        Returns:
            Dict with ``archetype``, ``cycles_completed``,
            ``convergence_history``, ``final_deck``, ``final_win_rate``,
            and ``output_dir``.
        """
        timestamp = datetime.now(timezone.utc).strftime("%Y%m%d_%H%M%S")
        run_dir = Path(self.output_dir) / self.archetype_name / timestamp
        run_dir.mkdir(parents=True, exist_ok=True)

        current_deck = list(self.base_deck)
        current_pilot = self.initial_pilot_path
        convergence_history: List[Dict[str, Any]] = []
        stagnant_count = 0
        prev_win_rate = 0.0

        logger.info(
            "Starting co-training for %s (%d cycles, pilot=%s)",
            self.archetype_name,
            cycles,
            current_pilot,
        )

        for cycle in range(cycles):
            cycle_dir = run_dir / f"cycle_{cycle}"
            cycle_dir.mkdir(parents=True, exist_ok=True)

            logger.info(
                "=== Cycle %d/%d  pilot=%s  deck_size=%d ===",
                cycle + 1, cycles, current_pilot, len(current_deck),
            )

            # --- Architect phase ---
            optimized_deck, win_rate = self._run_architect_cycle(
                cycle=cycle,
                pilot_path=current_pilot,
                current_deck=current_deck,
                cycle_dir=cycle_dir,
                episodes=architect_episodes,
                games_per_eval=architect_games_per_eval,
                max_swaps=architect_max_swaps,
                n_workers=n_workers,
                seed=(seed + cycle * 1000) if seed is not None else None,
            )

            improvement = win_rate - prev_win_rate if cycle > 0 else win_rate

            cycle_record = {
                "cycle": cycle,
                "win_rate": float(win_rate),
                "improvement": float(improvement),
                "pilot_path": current_pilot,
                "deck_size": len(optimized_deck),
            }

            logger.info(
                "Cycle %d architect: WR=%.3f  improvement=%.4f",
                cycle, win_rate, improvement,
            )

            # --- Pilot phase ---
            pilot_zip = self._run_pilot_cycle(
                cycle=cycle,
                deck=optimized_deck,
                cycle_dir=cycle_dir,
                timesteps=pilot_timesteps,
                seed=(seed + cycle * 1000 + 500) if seed is not None else None,
            )

            # --- ONNX export ---
            if pilot_zip is not None:
                onnx_path = str(cycle_dir / "pilot" / "pilot.onnx")
                exported = self._export_onnx(pilot_zip, onnx_path)
                if exported is not None:
                    current_pilot = exported
                    cycle_record["pilot_model"] = exported
                    logger.info("Cycle %d pilot exported: %s", cycle, exported)
                else:
                    logger.warning(
                        "Cycle %d ONNX export failed; keeping previous pilot.",
                        cycle,
                    )
            else:
                logger.warning(
                    "Cycle %d pilot training skipped; keeping previous pilot.",
                    cycle,
                )

            current_deck = optimized_deck
            prev_win_rate = win_rate
            convergence_history.append(cycle_record)

            # Save per-cycle deck
            deck_path = cycle_dir / "optimized_deck.json"
            with open(deck_path, "w", encoding="utf-8") as f:
                json.dump({
                    "archetype": self.archetype_name,
                    "cycle": cycle,
                    "deck": optimized_deck,
                    "win_rate": win_rate,
                }, f, indent=2)

            # --- Convergence check ---
            if cycle > 0 and abs(improvement) < convergence_threshold:
                stagnant_count += 1
                logger.info(
                    "Cycle %d below convergence threshold (%.4f < %.4f); "
                    "stagnant count=%d/2",
                    cycle, abs(improvement), convergence_threshold, stagnant_count,
                )
                if stagnant_count >= 2:
                    logger.info(
                        "Converged after %d cycles (2 consecutive sub-threshold).",
                        cycle + 1,
                    )
                    break
            else:
                stagnant_count = 0

        # --- Final summary ---
        result = {
            "archetype": self.archetype_name,
            "cycles_completed": len(convergence_history),
            "convergence_history": convergence_history,
            "final_deck": current_deck,
            "final_win_rate": float(prev_win_rate),
            "output_dir": str(run_dir),
        }

        summary_path = run_dir / "convergence.json"
        with open(summary_path, "w", encoding="utf-8") as f:
            json.dump(result, f, indent=2)

        logger.info(
            "Co-training complete for %s: %d cycles, final WR=%.3f, "
            "artifacts at %s",
            self.archetype_name,
            len(convergence_history),
            prev_win_rate,
            run_dir,
        )

        return result

    # ------------------------------------------------------------------
    # Internal phases
    # ------------------------------------------------------------------

    def _run_architect_cycle(
        self,
        cycle: int,
        pilot_path: str,
        current_deck: List[str],
        cycle_dir: Path,
        episodes: int,
        games_per_eval: int,
        max_swaps: int,
        n_workers: int,
        seed: Optional[int],
    ) -> Tuple[List[str], float]:
        """Train architect to optimize the deck, return (optimized_deck, win_rate)."""
        from digimon_gym.agents.architect_optimizer import MetaOptimizer

        architect_dir = cycle_dir / "architect"

        optimizer = MetaOptimizer(
            archetype_name=self.archetype_name,
            base_deck=current_deck,
            meta_config=self.meta_config,
            pilot_policy_path=pilot_path,
            output_dir=str(architect_dir),
            extra_cards=self.extra_cards,
        )

        metadata = optimizer.train(
            episodes=episodes,
            games_per_eval=games_per_eval,
            max_swaps=max_swaps,
            n_workers=n_workers,
            seed=seed,
        )

        optimized_deck = metadata.get("optimized_deck", current_deck)
        best_wr = metadata.get("best_win_rate", 0.0)

        return optimized_deck, best_wr

    def _run_pilot_cycle(
        self,
        cycle: int,
        deck: List[str],
        cycle_dir: Path,
        timesteps: int,
        seed: Optional[int],
    ) -> Optional[str]:
        """Train pilot on the given deck via subprocess.

        Returns:
            Path to the saved pilot model (.zip), or ``None`` if training
            failed or was skipped.

        Note:
            pilot_training.py accepts ``--deck1`` for a TTS/text format deck
            file. For co-training we need to pass a JSON card-ID list. This
            currently writes a JSON file and passes it via ``--deck1``.

            TODO: pilot_training.py's ``--deck1`` flag uses
            ``deck_loader.parse_deck()`` which expects TTS text format, not
            raw JSON card-ID lists. To fully integrate co-training, add a
            ``--deck-json`` flag to pilot_training.py that accepts a path to a
            JSON file containing a flat list of card IDs. Until then, the pilot
            phase logs a warning and returns None, keeping the previous pilot.
        """
        pilot_dir = cycle_dir / "pilot"
        pilot_dir.mkdir(parents=True, exist_ok=True)

        # Write deck as JSON for the pilot to pick up
        deck_json_path = pilot_dir / "deck.json"
        with open(deck_json_path, "w", encoding="utf-8") as f:
            json.dump(deck, f)

        # TODO: pilot_training.py does not yet accept --deck-json.
        # When that flag is added, uncomment the subprocess call below and
        # remove the early return.
        logger.warning(
            "Cycle %d pilot training: pilot_training.py does not yet support "
            "--deck-json. Skipping pilot retraining. Add --deck-json to "
            "pilot_training.py to enable full co-training.",
            cycle,
        )
        return None

        # --- Subprocess invocation (activate when --deck-json is supported) ---
        # save_dir = str(pilot_dir)
        # cmd = [
        #     sys.executable, "-m", "digimon_gym.agents.pilot_training",
        #     "--timesteps", str(timesteps),
        #     "--deck-json", str(deck_json_path),
        #     "--save-dir", save_dir,
        #     "--opponent", "greedy",
        #     "--eval-freq", str(max(timesteps // 10, 1000)),
        #     "--eval-episodes", "20",
        # ]
        # if seed is not None:
        #     # pilot_training does not have --seed yet; add if needed
        #     pass
        #
        # logger.info("Cycle %d pilot training: %s", cycle, " ".join(cmd))
        #
        # try:
        #     subprocess.run(cmd, check=True, timeout=3600)
        # except subprocess.CalledProcessError as exc:
        #     logger.error("Pilot training failed (exit %d)", exc.returncode)
        #     return None
        # except subprocess.TimeoutExpired:
        #     logger.error("Pilot training timed out (1h limit)")
        #     return None
        #
        # # Find the latest .zip model in the save dir
        # zips = sorted(Path(save_dir).glob("*.zip"), key=lambda p: p.stat().st_mtime)
        # if not zips:
        #     logger.error("No .zip model found after pilot training in %s", save_dir)
        #     return None
        #
        # return str(zips[-1])

    def _export_onnx(self, zip_path: str, onnx_path: str) -> Optional[str]:
        """Export an SB3 .zip model to ONNX format.

        Uses ``scripts/export_onnx.py`` via subprocess.

        Returns:
            The *onnx_path* on success, or ``None`` on failure.
        """
        export_script = (
            Path(__file__).resolve().parent.parent.parent / "scripts" / "export_onnx.py"
        )

        if not export_script.exists():
            logger.warning(
                "ONNX export script not found at %s; skipping export.",
                export_script,
            )
            return None

        # Auto-detect model type from filename
        model_type = "lstm" if "lstm" in Path(zip_path).stem.lower() else "mlp"

        cmd = [
            sys.executable, str(export_script),
            "--type", model_type,
            "--input", zip_path,
            "--output", onnx_path,
        ]

        logger.info("Exporting ONNX: %s", " ".join(cmd))

        try:
            subprocess.run(cmd, check=True, timeout=300)
        except subprocess.CalledProcessError as exc:
            logger.error("ONNX export failed (exit %d)", exc.returncode)
            return None
        except subprocess.TimeoutExpired:
            logger.error("ONNX export timed out (5m limit)")
            return None
        except FileNotFoundError:
            logger.error("Python executable not found for ONNX export")
            return None

        if Path(onnx_path).exists():
            return onnx_path

        logger.error("ONNX file not found after export: %s", onnx_path)
        return None


# ---------------------------------------------------------------------------
# ArchitectGauntlet
# ---------------------------------------------------------------------------

class ArchitectGauntlet:
    """Multi-archetype architect competition with round-robin evaluation.

    Each round:
    1. Every archetype runs an architect cycle to optimize its deck.
    2. Every archetype retrains its pilot on the optimized deck.
    3. Round-robin: all decks play all other decks, producing a matchup matrix.
    4. ETWR (Expected Tournament Win Rate) is computed per archetype using
       meta shares as weights. Meta shares are updated for the next round
       based on ETWR rankings.
    """

    def __init__(
        self,
        archetype_configs: List[Dict[str, Any]],
        meta_config: dict,
        initial_pilot_path: str = "greedy",
        output_dir: str = "gauntlet_runs",
    ):
        """
        Args:
            archetype_configs: List of dicts, each with at least ``'name'``
                (archetype name). May also include ``'base_deck'`` (card ID
                list), ``'extra_cards'``, etc.  If ``'base_deck'`` is omitted,
                it is resolved from deck_library.json.
            meta_config: Dict with ``'archetypes'`` key for meta shares.
            initial_pilot_path: Starting pilot model for all archetypes.
            output_dir: Root directory for gauntlet artifacts.
        """
        self.archetype_configs = archetype_configs
        self.meta_config = meta_config
        self.initial_pilot_path = initial_pilot_path
        self.output_dir = output_dir

        # Resolve base decks
        self._archetypes: Dict[str, Dict[str, Any]] = {}
        for cfg in archetype_configs:
            name = cfg["name"]
            base_deck = cfg.get("base_deck")
            if base_deck is None:
                base_deck = _resolve_base_deck(name)
            self._archetypes[name] = {
                "base_deck": list(base_deck),
                "current_deck": list(base_deck),
                "pilot_path": initial_pilot_path,
                "extra_cards": cfg.get("extra_cards"),
            }

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def run(
        self,
        rounds: int = 3,
        architect_episodes: int = 100,
        architect_games_per_eval: int = 50,
        architect_max_swaps: int = 10,
        pilot_timesteps: int = 200_000,
        games_per_matchup: int = 100,
        n_workers: int = 1,
        seed: Optional[int] = None,
    ) -> dict:
        """Run gauntlet rounds.

        Args:
            rounds: Number of gauntlet rounds.
            architect_episodes: DQN episodes per architect phase per archetype.
            architect_games_per_eval: Games per architect evaluation matchup.
            architect_max_swaps: Max card swaps per architect episode.
            pilot_timesteps: PPO timesteps per pilot retraining.
            games_per_matchup: Games per matchup in round-robin evaluation.
            n_workers: Parallel simulation workers.
            seed: Random seed for reproducibility.

        Returns:
            Dict with ``rounds_completed``, ``final_meta`` (archetype to ETWR),
            ``final_decks`` (archetype to card ID list), ``round_results``
            (per-round summaries), and ``output_dir``.
        """
        timestamp = datetime.now(timezone.utc).strftime("%Y%m%d_%H%M%S")
        run_dir = Path(self.output_dir) / timestamp
        run_dir.mkdir(parents=True, exist_ok=True)

        arch_names = list(self._archetypes.keys())

        # Initial meta shares — uniform if not in meta_config
        meta_shares: Dict[str, float] = {}
        for name in arch_names:
            arch_stats = self.meta_config.get("archetypes", {}).get(name, {})
            share = (
                arch_stats.get("local_meta_share")
                or arch_stats.get("global_meta_share")
                or 1.0 / len(arch_names)
            )
            meta_shares[name] = share

        round_results: List[Dict[str, Any]] = []

        logger.info(
            "Starting architect gauntlet: %d archetypes, %d rounds",
            len(arch_names), rounds,
        )

        for rnd in range(rounds):
            round_dir = run_dir / f"round_{rnd}"
            round_dir.mkdir(parents=True, exist_ok=True)

            logger.info(
                "=== Round %d/%d  archetypes=%s ===",
                rnd + 1, rounds, ", ".join(arch_names),
            )

            # --- Phase 1: architect training for each archetype ---
            for name in arch_names:
                arch_data = self._archetypes[name]
                arch_dir = round_dir / name

                cotrainer = CoTrainer(
                    archetype_name=name,
                    base_deck=arch_data["current_deck"],
                    meta_config=self.meta_config,
                    initial_pilot_path=arch_data["pilot_path"],
                    output_dir=str(arch_dir),
                    extra_cards=arch_data.get("extra_cards"),
                )

                # Run a single co-training cycle (architect + pilot)
                ct_result = cotrainer.run(
                    cycles=1,
                    architect_episodes=architect_episodes,
                    architect_games_per_eval=architect_games_per_eval,
                    architect_max_swaps=architect_max_swaps,
                    pilot_timesteps=pilot_timesteps,
                    n_workers=n_workers,
                    seed=(seed + rnd * 10000 + hash(name) % 1000)
                    if seed is not None else None,
                )

                arch_data["current_deck"] = ct_result["final_deck"]
                # Update pilot if co-training produced one
                history = ct_result.get("convergence_history", [])
                if history and history[-1].get("pilot_model"):
                    arch_data["pilot_path"] = history[-1]["pilot_model"]

                logger.info(
                    "Round %d %s: architect WR=%.3f",
                    rnd, name, ct_result["final_win_rate"],
                )

            # --- Phase 2: round-robin evaluation ---
            current_decks = {
                name: arch_data["current_deck"]
                for name, arch_data in self._archetypes.items()
            }
            # Use the initial pilot for round-robin (consistent baseline)
            rr_result = self._round_robin(
                decks=current_decks,
                games_per_matchup=games_per_matchup,
                pilot_path=self.initial_pilot_path,
                n_workers=n_workers,
            )

            # --- Phase 3: compute ETWR and update meta shares ---
            etwr = self._compute_etwr(rr_result["matchup_matrix"], meta_shares)

            # Update meta shares proportional to ETWR for next round
            total_etwr = sum(etwr.values())
            if total_etwr > 0:
                meta_shares = {
                    name: ew / total_etwr for name, ew in etwr.items()
                }

            round_record = {
                "round": rnd,
                "matchup_matrix": rr_result["matchup_matrix"],
                "etwr": etwr,
                "meta_shares": dict(meta_shares),
            }
            round_results.append(round_record)

            # Persist round results
            with open(round_dir / "round_robin.json", "w", encoding="utf-8") as f:
                json.dump(round_record, f, indent=2)

            logger.info(
                "Round %d ETWR: %s",
                rnd,
                ", ".join(f"{n}={e:.3f}" for n, e in sorted(
                    etwr.items(), key=lambda x: x[1], reverse=True
                )),
            )

        # --- Final summary ---
        final_etwr = round_results[-1]["etwr"] if round_results else {}
        final_decks = {
            name: arch_data["current_deck"]
            for name, arch_data in self._archetypes.items()
        }

        result = {
            "rounds_completed": len(round_results),
            "final_meta": final_etwr,
            "final_decks": final_decks,
            "round_results": round_results,
            "output_dir": str(run_dir),
        }

        with open(run_dir / "final_meta.json", "w", encoding="utf-8") as f:
            json.dump(result, f, indent=2)

        logger.info(
            "Gauntlet complete: %d rounds, artifacts at %s",
            len(round_results), run_dir,
        )

        return result

    # ------------------------------------------------------------------
    # Round-robin evaluation
    # ------------------------------------------------------------------

    def _round_robin(
        self,
        decks: Dict[str, List[str]],
        games_per_matchup: int,
        pilot_path: str,
        n_workers: int = 1,
    ) -> dict:
        """All-vs-all evaluation.

        Returns dict with ``matchup_matrix`` mapping
        ``{attacker_name: {defender_name: win_rate}}``.
        """
        from digimon_gym.agents.architect_simulator import DeckSimulator

        names = list(decks.keys())
        matrix: Dict[str, Dict[str, float]] = {n: {} for n in names}

        sim = DeckSimulator(
            pilot_policy_path=pilot_path,
            opponent_policy="greedy",
            games_per_eval=games_per_matchup,
            max_turns=200,
            n_workers=n_workers,
            cache_enabled=False,  # Different decks each round
        )

        for i, a_name in enumerate(names):
            for j, b_name in enumerate(names):
                if i == j:
                    matrix[a_name][b_name] = 0.5  # Mirror match
                    continue

                # evaluate_deck expects opponents as [(deck, weight)] list
                wr = sim.evaluate_deck(
                    decks[a_name],
                    [(decks[b_name], 1.0)],
                )
                matrix[a_name][b_name] = wr

                logger.debug(
                    "  %s vs %s: WR=%.3f", a_name, b_name, wr,
                )

        return {"matchup_matrix": matrix}

    @staticmethod
    def _compute_etwr(
        matchup_matrix: Dict[str, Dict[str, float]],
        meta_shares: Dict[str, float],
    ) -> Dict[str, float]:
        """Compute Expected Tournament Win Rate per archetype.

        ETWR(A) = sum(wr(A, X) * share(X) for X != A) / sum(share(X) for X != A)

        This gives the expected win rate of archetype A across the meta,
        weighted by each opponent's meta prevalence.
        """
        etwr: Dict[str, float] = {}

        for a_name, row in matchup_matrix.items():
            weighted_sum = 0.0
            weight_sum = 0.0
            for b_name, wr in row.items():
                if b_name == a_name:
                    continue
                share = meta_shares.get(b_name, 0.0)
                weighted_sum += wr * share
                weight_sum += share

            etwr[a_name] = weighted_sum / weight_sum if weight_sum > 0 else 0.5

        return etwr


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(
        description="Architect co-training and multi-archetype gauntlet.",
    )

    mode_group = parser.add_mutually_exclusive_group(required=True)
    mode_group.add_argument(
        "--archetype", type=str, default=None,
        help="Single archetype co-training mode.",
    )
    mode_group.add_argument(
        "--gauntlet", action="store_true", default=False,
        help="Multi-archetype gauntlet mode.",
    )

    parser.add_argument(
        "--archetypes", type=str, default=None,
        help="Comma-separated archetype names for gauntlet mode.",
    )
    parser.add_argument(
        "--meta", type=str, default=None,
        help="Path to meta config JSON file. Defaults to deck_library.json stats.",
    )
    parser.add_argument(
        "--initial-pilot", type=str, default="greedy",
        help="Initial pilot policy path or 'greedy'/'random' (default: greedy).",
    )
    parser.add_argument(
        "--output-dir", type=str, default=None,
        help="Output directory (default: cotraining_runs or gauntlet_runs).",
    )

    # Co-training params
    parser.add_argument(
        "--cycles", type=int, default=5,
        help="Number of architect/pilot cycles (co-training mode, default: 5).",
    )
    parser.add_argument(
        "--convergence-threshold", type=float, default=0.01,
        help="Early-stop threshold for win-rate improvement (default: 0.01).",
    )

    # Gauntlet params
    parser.add_argument(
        "--rounds", type=int, default=3,
        help="Number of gauntlet rounds (gauntlet mode, default: 3).",
    )
    parser.add_argument(
        "--games-per-matchup", type=int, default=100,
        help="Games per matchup in round-robin evaluation (default: 100).",
    )

    # Shared params
    parser.add_argument(
        "--architect-episodes", type=int, default=100,
        help="Architect DQN training episodes per cycle (default: 100).",
    )
    parser.add_argument(
        "--architect-games-per-eval", type=int, default=50,
        help="Games per architect evaluation matchup (default: 50).",
    )
    parser.add_argument(
        "--architect-max-swaps", type=int, default=10,
        help="Max card swaps per architect episode (default: 10).",
    )
    parser.add_argument(
        "--pilot-timesteps", type=int, default=200_000,
        help="Pilot PPO training timesteps per cycle (default: 200000).",
    )
    parser.add_argument(
        "--workers", type=int, default=1,
        help="Parallel simulation workers (default: 1).",
    )
    parser.add_argument(
        "--seed", type=int, default=None,
        help="Random seed for reproducibility.",
    )
    parser.add_argument(
        "--extra-cards", type=str, default=None,
        help="Comma-separated extra card IDs for architect pool.",
    )

    args = parser.parse_args()

    # Configure logging
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s [%(name)s] %(levelname)s: %(message)s",
    )

    # Load meta config
    if args.meta:
        meta_path = Path(args.meta)
        if not meta_path.exists():
            print(f"ERROR: Meta config not found: {meta_path}", file=sys.stderr)
            sys.exit(1)
        with open(meta_path, "r", encoding="utf-8") as f:
            meta_config = json.load(f)
        print(f"Meta config: {meta_path}")
    else:
        meta_config = _build_default_meta_config()
        n_arch = len(meta_config.get("archetypes", {}))
        print(f"Meta config: built from deck_library.json ({n_arch} archetypes)")

    extra_cards = None
    if args.extra_cards:
        extra_cards = [c.strip() for c in args.extra_cards.split(",") if c.strip()]

    # --- Co-training mode ---
    if args.archetype:
        base_deck = _resolve_base_deck(args.archetype)
        output_dir = args.output_dir or "cotraining_runs"

        print(f"\nCo-training: {args.archetype}")
        print(f"  Base deck:  {len(base_deck)} cards")
        print(f"  Pilot:      {args.initial_pilot}")
        print(f"  Cycles:     {args.cycles}")
        print(f"  Architect:  {args.architect_episodes} episodes, "
              f"{args.architect_max_swaps} max swaps")
        print(f"  Pilot:      {args.pilot_timesteps} timesteps")
        print(f"  Workers:    {args.workers}")
        if args.seed is not None:
            print(f"  Seed:       {args.seed}")
        print()

        cotrainer = CoTrainer(
            archetype_name=args.archetype,
            base_deck=base_deck,
            meta_config=meta_config,
            initial_pilot_path=args.initial_pilot,
            output_dir=output_dir,
            extra_cards=extra_cards,
        )

        result = cotrainer.run(
            cycles=args.cycles,
            architect_episodes=args.architect_episodes,
            architect_games_per_eval=args.architect_games_per_eval,
            architect_max_swaps=args.architect_max_swaps,
            pilot_timesteps=args.pilot_timesteps,
            n_workers=args.workers,
            convergence_threshold=args.convergence_threshold,
            seed=args.seed,
        )

        print(f"\nDone. {result['cycles_completed']} cycles completed.")
        print(f"Final win rate: {result['final_win_rate']:.3f}")
        print(f"Artifacts: {result['output_dir']}")

    # --- Gauntlet mode ---
    elif args.gauntlet:
        if not args.archetypes:
            print("ERROR: --archetypes required in gauntlet mode.", file=sys.stderr)
            sys.exit(1)

        arch_names = [n.strip() for n in args.archetypes.split(",") if n.strip()]
        if len(arch_names) < 2:
            print("ERROR: gauntlet requires at least 2 archetypes.", file=sys.stderr)
            sys.exit(1)

        output_dir = args.output_dir or "gauntlet_runs"

        print(f"\nArchitect Gauntlet: {len(arch_names)} archetypes")
        for name in arch_names:
            print(f"  - {name}")
        print(f"  Pilot:            {args.initial_pilot}")
        print(f"  Rounds:           {args.rounds}")
        print(f"  Architect:        {args.architect_episodes} episodes")
        print(f"  Pilot:            {args.pilot_timesteps} timesteps")
        print(f"  Games/matchup:    {args.games_per_matchup}")
        print(f"  Workers:          {args.workers}")
        if args.seed is not None:
            print(f"  Seed:             {args.seed}")
        print()

        archetype_configs = [
            {"name": name, "extra_cards": extra_cards}
            for name in arch_names
        ]

        gauntlet = ArchitectGauntlet(
            archetype_configs=archetype_configs,
            meta_config=meta_config,
            initial_pilot_path=args.initial_pilot,
            output_dir=output_dir,
        )

        result = gauntlet.run(
            rounds=args.rounds,
            architect_episodes=args.architect_episodes,
            architect_games_per_eval=args.architect_games_per_eval,
            architect_max_swaps=args.architect_max_swaps,
            pilot_timesteps=args.pilot_timesteps,
            games_per_matchup=args.games_per_matchup,
            n_workers=args.workers,
            seed=args.seed,
        )

        print(f"\nDone. {result['rounds_completed']} rounds completed.")
        print("\nFinal ETWR rankings:")
        ranked = sorted(
            result["final_meta"].items(),
            key=lambda x: x[1],
            reverse=True,
        )
        for rank, (name, etwr) in enumerate(ranked, 1):
            print(f"  {rank}. {name}: ETWR={etwr:.3f}")
        print(f"\nArtifacts: {result['output_dir']}")


if __name__ == "__main__":
    main()
