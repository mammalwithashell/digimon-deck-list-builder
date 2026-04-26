"""Architect agent interpretability tooling.

Wraps a loaded MetaOptimizer and provides five analysis methods to
interrogate *why* the DQN agent made specific deck-building decisions.

No DB imports.  Pure engine + training.
"""

from __future__ import annotations

import argparse
import json
import logging
import sys
from collections import Counter
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

import numpy as np
import torch

from digimon_gym.agents.architect_optimizer import MetaOptimizer
from digimon_engine import CardDatabase

logger = logging.getLogger(__name__)


class ArchitectExplainer:
    """Interpretability layer over a trained architect agent.

    Loads a completed training run and exposes analyses ranging from
    instant Q-value inspection to full per-swap matchup attribution.
    """

    def __init__(
        self,
        run_dir: str,
        pilot: str = "greedy",
        games_per_eval: int = 50,
    ) -> None:
        self._optimizer = MetaOptimizer.load(run_dir, pilot_policy_path=pilot)
        self._run_dir = Path(run_dir)
        self._games_per_eval = games_per_eval
        self._db = CardDatabase()

        # Cache for matchup breakdown results (deck_hash -> list[dict])
        self._matchup_cache: Dict[str, List[dict]] = {}

        # Load metadata for swap history access
        with open(self._run_dir / "metadata.json", "r", encoding="utf-8") as f:
            self._metadata = json.load(f)

    # ------------------------------------------------------------------
    # Public analyses (cheapest → most expensive)
    # ------------------------------------------------------------------

    def q_value_landscape(
        self,
        deck: Optional[List[str]] = None,
        step: int = 0,
        top_k: int = 10,
    ) -> dict:
        """Show top-K swap actions by Q-value (instant — no simulation).

        Also includes top-K *masked* actions (high Q but currently
        invalid) to reveal constrained preferences.
        """
        pool = self._optimizer._pool
        agent = self._optimizer._agent
        assert pool is not None and agent is not None

        deck_ids = deck if deck is not None else self._optimized_deck()
        counts = pool.deck_list_to_counts(deck_ids)
        max_swaps = self._metadata.get("training_params", {}).get("max_swaps", 10)

        obs = self._build_obs(counts, step=step, max_swaps=max_swaps)
        state_tensor = torch.tensor(obs, dtype=torch.float32, device=agent._device).unsqueeze(0)

        with torch.no_grad():
            q_values = agent._online_net(state_tensor).squeeze(0).cpu().numpy()

        mask = pool.get_action_mask(counts)

        # Valid actions: sort by Q descending
        valid_indices = np.where(mask == 1)[0]
        valid_q = q_values[valid_indices]
        top_valid_idx = valid_indices[np.argsort(-valid_q)[:top_k]]

        top_valid = []
        for action_idx in top_valid_idx:
            swap = pool.decode_action(int(action_idx))
            entry = {
                "action": int(action_idx),
                "q_value": float(q_values[action_idx]),
            }
            if swap is None:
                entry["type"] = "no-op"
            else:
                remove_id, add_id = swap
                entry["type"] = "swap"
                entry["remove"] = self._card_info(remove_id)
                entry["add"] = self._card_info(add_id)
            top_valid.append(entry)

        # Masked (invalid) actions with highest Q: reveals constrained desires
        masked_indices = np.where(mask == 0)[0]
        if len(masked_indices) > 0:
            masked_q = q_values[masked_indices]
            top_masked_idx = masked_indices[np.argsort(-masked_q)[:top_k]]
            top_masked = []
            for action_idx in top_masked_idx:
                swap = pool.decode_action(int(action_idx))
                entry = {
                    "action": int(action_idx),
                    "q_value": float(q_values[action_idx]),
                }
                if swap is None:
                    entry["type"] = "no-op"
                else:
                    remove_id, add_id = swap
                    entry["type"] = "swap"
                    entry["remove"] = self._card_info(remove_id)
                    entry["add"] = self._card_info(add_id)
                top_masked.append(entry)
        else:
            top_masked = []

        return {
            "deck_size": sum(counts.values()),
            "n_valid_actions": int(mask.sum()),
            "n_total_actions": len(q_values),
            "step": step,
            "top_valid": top_valid,
            "top_masked": top_masked,
        }

    def matchup_breakdown(
        self,
        deck: Optional[List[str]] = None,
        games: Optional[int] = None,
    ) -> List[dict]:
        """Per-opponent win rates (one evaluation per opponent).

        Results are cached by deck hash for reuse by other analyses.
        """
        deck_ids = deck if deck is not None else self._optimized_deck()
        cache_key = self._deck_hash(deck_ids)

        if cache_key in self._matchup_cache:
            return self._matchup_cache[cache_key]

        simulator = self._optimizer._simulator
        opponents = self._optimizer._opponents
        assert simulator is not None and opponents is not None

        n_games = games if games is not None else self._games_per_eval
        opp_names = self._opponent_names()

        results = []
        for i, (opp_deck, meta_weight) in enumerate(opponents):
            # Evaluate against this single opponent
            orig_games = simulator.games_per_eval
            simulator.games_per_eval = n_games
            wr = simulator.evaluate_deck(deck_ids, [(opp_deck, 1.0)])
            simulator.games_per_eval = orig_games

            name = opp_names[i] if i < len(opp_names) else f"Opponent_{i}"
            results.append({
                "archetype": name,
                "meta_weight": float(meta_weight),
                "win_rate": float(wr),
                "games": n_games,
            })

        # Sort worst matchups first
        results.sort(key=lambda r: r["win_rate"])

        self._matchup_cache[cache_key] = results
        return results

    def meta_sensitivity(
        self,
        deck: Optional[List[str]] = None,
        weight_delta: float = 0.05,
        games: Optional[int] = None,
    ) -> List[dict]:
        """How overall WR shifts if an archetype's meta share changes.

        Pure arithmetic over matchup breakdown results — no new simulations
        (unless matchup_breakdown hasn't been run yet for this deck).
        """
        matchups = self.matchup_breakdown(deck, games=games)

        total_weight = sum(m["meta_weight"] for m in matchups)
        if total_weight <= 0:
            return []

        # Baseline weighted WR
        baseline_wr = sum(
            m["win_rate"] * m["meta_weight"] for m in matchups
        ) / total_weight

        results = []
        for target in matchups:
            # Shift this archetype's weight by +delta, renormalize
            shifted_total = total_weight + weight_delta
            shifted_wr = sum(
                m["win_rate"] * (
                    (m["meta_weight"] + weight_delta) if m["archetype"] == target["archetype"]
                    else m["meta_weight"]
                )
                for m in matchups
            ) / shifted_total

            sensitivity = (shifted_wr - baseline_wr) / weight_delta

            results.append({
                "archetype": target["archetype"],
                "meta_weight": target["meta_weight"],
                "win_rate": target["win_rate"],
                "sensitivity": float(sensitivity),
                "interpretation": (
                    "good" if sensitivity > 0 else "bad"
                ),
            })

        # Sort by absolute sensitivity descending
        results.sort(key=lambda r: abs(r["sensitivity"]), reverse=True)
        return results

    def card_importance(
        self,
        deck: Optional[List[str]] = None,
        games: Optional[int] = None,
    ) -> List[dict]:
        """Ablation: remove each unique card, fill with Q-network's best pick, measure WR drop."""
        pool = self._optimizer._pool
        agent = self._optimizer._agent
        simulator = self._optimizer._simulator
        opponents = self._optimizer._opponents
        assert pool is not None and agent is not None
        assert simulator is not None and opponents is not None

        deck_ids = deck if deck is not None else self._optimized_deck()
        counts = pool.deck_list_to_counts(deck_ids)
        n_games = games if games is not None else self._games_per_eval

        # Get baseline WR (may use cached matchup breakdown)
        matchups = self.matchup_breakdown(deck_ids, games=n_games)
        total_weight = sum(m["meta_weight"] for m in matchups)
        baseline_wr = (
            sum(m["win_rate"] * m["meta_weight"] for m in matchups) / total_weight
            if total_weight > 0 else 0.0
        )

        # Extract egg deck for reconstruction
        egg_ids = [cid for cid in deck_ids if cid not in counts]

        max_swaps = self._metadata.get("training_params", {}).get("max_swaps", 10)
        results = []

        for card_id, n_copies in sorted(counts.items()):
            # Remove all copies of this card
            ablated = dict(counts)
            del ablated[card_id]

            # Use Q-network to pick best replacements (one at a time)
            for _ in range(n_copies):
                obs = self._build_obs(ablated, step=0, max_swaps=max_swaps)
                state_tensor = torch.tensor(
                    obs, dtype=torch.float32, device=agent._device
                ).unsqueeze(0)

                # We need to find the best "add" action from the no-op-like perspective.
                # Build a mask where we can only add cards (not remove more).
                # The easiest approach: create a temporary counts dict with an
                # extra dummy slot, but that's fragile. Instead, just find the
                # highest-Q valid action that adds a card.
                mask = pool.get_action_mask(ablated)
                with torch.no_grad():
                    q_values = agent._online_net(state_tensor).squeeze(0).cpu().numpy()

                # Among valid actions, pick the one with highest Q
                valid_q = np.where(mask == 1, q_values, -np.inf)
                best_action = int(np.argmax(valid_q))
                swap = pool.decode_action(best_action)

                if swap is not None:
                    remove_card, add_card = swap
                    ablated = pool.apply_swap(ablated, remove_card, add_card)
                else:
                    # No-op was best; just add a random valid card to fill
                    break

            # Evaluate ablated deck
            ablated_deck = pool.counts_to_deck_list(ablated, egg_ids)

            orig_games = simulator.games_per_eval
            simulator.games_per_eval = n_games
            ablated_wr = simulator.evaluate_deck(ablated_deck, opponents)
            simulator.games_per_eval = orig_games

            wr_drop = baseline_wr - ablated_wr

            results.append({
                "card": self._card_info(card_id),
                "copies": n_copies,
                "baseline_wr": float(baseline_wr),
                "ablated_wr": float(ablated_wr),
                "wr_drop": float(wr_drop),
                "role": (
                    "load-bearing" if wr_drop > 0.03
                    else "flex" if wr_drop < 0.01
                    else "moderate"
                ),
            })

        # Sort by WR drop descending (most important first)
        results.sort(key=lambda r: r["wr_drop"], reverse=True)
        return results

    def swap_narrative(
        self,
        swaps: Optional[List[dict]] = None,
        games: Optional[int] = None,
    ) -> List[dict]:
        """Per-swap matchup attribution: which matchups did each swap target?

        Replays swaps from base deck, evaluating deck-before and deck-after
        against each opponent individually.
        """
        pool = self._optimizer._pool
        simulator = self._optimizer._simulator
        opponents = self._optimizer._opponents
        assert pool is not None and simulator is not None and opponents is not None

        if swaps is None:
            # Try to get from metadata (last optimize_deck or training best)
            swaps = self._metadata.get("best_episode_swaps", [])
            if not swaps:
                # Fall back to running optimize_deck to get swaps
                result = self._optimizer.optimize_deck()
                swaps = result.get("swaps", [])

        if not swaps:
            return []

        n_games = games if games is not None else self._games_per_eval
        opp_names = self._opponent_names()

        # Reconstruct base deck
        base_deck = list(self._optimizer.base_deck)
        base_counts = pool.deck_list_to_counts(base_deck)
        egg_ids = [cid for cid in base_deck if cid not in base_counts]

        narrative = []

        for i, swap in enumerate(swaps):
            removed = swap["removed"]
            added = swap["added"]

            # Build deck before and after this swap
            deck_before = pool.counts_to_deck_list(base_counts, egg_ids)

            after_counts = pool.apply_swap(dict(base_counts), removed, added)
            deck_after = pool.counts_to_deck_list(after_counts, egg_ids)

            # Evaluate both against each opponent
            matchup_deltas = []
            for j, (opp_deck, meta_weight) in enumerate(opponents):
                orig_games = simulator.games_per_eval
                simulator.games_per_eval = n_games

                wr_before = simulator.evaluate_deck(deck_before, [(opp_deck, 1.0)])
                wr_after = simulator.evaluate_deck(deck_after, [(opp_deck, 1.0)])

                simulator.games_per_eval = orig_games

                name = opp_names[j] if j < len(opp_names) else f"Opponent_{j}"
                matchup_deltas.append({
                    "archetype": name,
                    "meta_weight": float(meta_weight),
                    "wr_before": float(wr_before),
                    "wr_after": float(wr_after),
                    "wr_delta": float(wr_after - wr_before),
                })

            # Sort by absolute delta descending
            matchup_deltas.sort(key=lambda m: abs(m["wr_delta"]), reverse=True)

            narrative.append({
                "swap_index": i,
                "removed": self._card_info(removed),
                "added": self._card_info(added),
                "wr_before": swap.get("win_rate_before"),
                "wr_after": swap.get("win_rate_after"),
                "matchup_deltas": matchup_deltas,
            })

            # Advance base_counts for next swap
            base_counts = after_counts

        return narrative

    # ------------------------------------------------------------------
    # Helpers
    # ------------------------------------------------------------------

    def _card_info(self, card_id: str) -> dict:
        """Return card metadata dict for display."""
        card = self._db.get_card(card_id)
        if card is None:
            return {"card_id": card_id, "card_name": "Unknown"}

        return {
            "card_id": card_id,
            "card_name": card.card_name_eng or card_id,
            "card_kind": card.card_kind.name if card.card_kind else None,
            "level": card.level,
            "play_cost": card.play_cost,
            "dp": card.dp,
            "colors": [c.name for c in card.card_colors] if card.card_colors else [],
        }

    def _opponent_names(self) -> List[str]:
        """Map opponent indices to archetype names.

        Replicates the iteration order of ``build_opponents()`` over
        ``meta_config`` to produce a parallel list of names.
        """
        meta_config = self._optimizer.meta_config
        archetype_name = self._optimizer.archetype_name
        opponents = self._optimizer._opponents
        assert opponents is not None

        # Replicate build_opponents iteration to get names in order
        from digimon_engine import load_implemented_card_ids

        from digimon_gym.data_paths import DECK_LIBRARY as deck_library_path
        if deck_library_path.exists():
            with open(deck_library_path, "r", encoding="utf-8") as f:
                deck_library = json.load(f)
        else:
            deck_library = {}

        implemented = load_implemented_card_ids()
        library_archetypes = deck_library.get("archetypes", {})

        names: List[str] = []
        for arch_name, arch_stats in meta_config.get("archetypes", {}).items():
            if arch_name == archetype_name:
                continue

            weight = arch_stats.get("local_meta_share") or arch_stats.get(
                "global_meta_share", 0
            )
            if weight <= 0:
                continue

            lib_arch = library_archetypes.get(arch_name)
            if not lib_arch or not lib_arch.get("decklists"):
                continue

            # Check if there's a simulatable decklist (same logic as build_opponents)
            from digimon_gym.agents.architect_optimizer import _pick_best_decklist

            best_deck = _pick_best_decklist(lib_arch["decklists"], implemented)
            if best_deck is None:
                continue

            names.append(arch_name)

        return names

    def _build_obs(
        self,
        deck_counts: Dict[str, int],
        step: int = 0,
        max_swaps: int = 10,
    ) -> np.ndarray:
        """Build observation vector matching DeckBuildingEnv._build_obs."""
        pool = self._optimizer._pool
        opponents = self._optimizer._opponents
        assert pool is not None and opponents is not None

        deck_vec = pool.deck_to_vector(deck_counts)

        # Normalize meta weights (same as env)
        raw_weights = np.array(
            [w for _, w in opponents], dtype=np.float32
        )
        weight_sum = raw_weights.sum()
        if weight_sum > 0:
            meta_weights = raw_weights / weight_sum
        else:
            meta_weights = np.ones(len(opponents), dtype=np.float32) / max(len(opponents), 1)

        step_frac = np.array([step / max_swaps], dtype=np.float32)
        return np.concatenate([deck_vec, meta_weights, step_frac])

    def _optimized_deck(self) -> List[str]:
        """Load the optimized deck from run artifacts."""
        opt_path = self._run_dir / "optimized_deck.json"
        if opt_path.exists():
            with open(opt_path, "r", encoding="utf-8") as f:
                return json.load(f).get("deck", [])

        if "optimized_deck" in self._metadata:
            return self._metadata["optimized_deck"]

        return list(self._optimizer.base_deck)

    @staticmethod
    def _deck_hash(deck_ids: List[str]) -> str:
        """Stable hash for a deck (order-independent)."""
        import hashlib

        key = ",".join(sorted(deck_ids))
        return hashlib.sha256(key.encode()).hexdigest()[:16]


# ------------------------------------------------------------------
# CLI
# ------------------------------------------------------------------

def _format_card(info: dict) -> str:
    """One-line card description for CLI output."""
    name = info.get("card_name", info.get("card_id", "?"))
    card_id = info.get("card_id", "")
    level = info.get("level")
    cost = info.get("play_cost")

    parts = [f"{card_id} {name}"]
    if level is not None:
        parts.append(f"Lv.{level}")
    if cost is not None:
        parts.append(f"Cost:{cost}")
    return " | ".join(parts)


def _print_q_values(result: dict) -> None:
    print(f"\n=== Q-Value Landscape (step={result['step']}) ===")
    print(f"Valid actions: {result['n_valid_actions']} / {result['n_total_actions']}")

    print(f"\nTop {len(result['top_valid'])} valid actions:")
    for entry in result["top_valid"]:
        q = entry["q_value"]
        if entry["type"] == "no-op":
            print(f"  Q={q:+.4f}  [NO-OP] Stop swapping")
        else:
            rm = _format_card(entry["remove"])
            add = _format_card(entry["add"])
            print(f"  Q={q:+.4f}  -{rm}")
            print(f"            +{add}")

    if result["top_masked"]:
        print(f"\nTop {len(result['top_masked'])} masked (wanted but invalid):")
        for entry in result["top_masked"]:
            q = entry["q_value"]
            if entry["type"] == "no-op":
                print(f"  Q={q:+.4f}  [NO-OP]")
            else:
                rm = _format_card(entry["remove"])
                add = _format_card(entry["add"])
                print(f"  Q={q:+.4f}  -{rm}  +{add}")


def _print_matchups(results: List[dict]) -> None:
    print("\n=== Matchup Breakdown ===")
    print(f"{'Archetype':<25} {'Meta%':>6} {'WR':>6} {'Games':>5}")
    print("-" * 46)
    for m in results:
        print(
            f"{m['archetype']:<25} "
            f"{m['meta_weight']*100:5.1f}% "
            f"{m['win_rate']*100:5.1f}% "
            f"{m['games']:5d}"
        )

    total_w = sum(m["meta_weight"] for m in results)
    if total_w > 0:
        weighted_wr = sum(m["win_rate"] * m["meta_weight"] for m in results) / total_w
        print("-" * 46)
        print(f"{'Weighted WR':<25} {'':>6} {weighted_wr*100:5.1f}%")


def _print_sensitivity(results: List[dict]) -> None:
    print("\n=== Meta Sensitivity ===")
    print(f"{'Archetype':<25} {'WR':>6} {'Sensitivity':>11} {'Impact':>6}")
    print("-" * 52)
    for s in results:
        sign = "+" if s["sensitivity"] > 0 else ""
        print(
            f"{s['archetype']:<25} "
            f"{s['win_rate']*100:5.1f}% "
            f"{sign}{s['sensitivity']:10.4f} "
            f"{'good' if s['sensitivity'] > 0 else 'bad':>6}"
        )


def _print_importance(results: List[dict]) -> None:
    print("\n=== Card Importance (Ablation) ===")
    print(f"{'Card':<40} {'Copies':>6} {'WR Drop':>8} {'Role':>12}")
    print("-" * 70)
    for r in results:
        card = _format_card(r["card"])
        print(
            f"{card:<40} "
            f"{r['copies']:>5}x "
            f"{r['wr_drop']*100:+7.2f}% "
            f"{r['role']:>12}"
        )


def _print_swaps(results: List[dict]) -> None:
    print("\n=== Swap Narrative ===")
    for s in results:
        rm = _format_card(s["removed"])
        add = _format_card(s["added"])
        wr_b = s.get("wr_before")
        wr_a = s.get("wr_after")
        wr_str = ""
        if wr_b is not None and wr_a is not None:
            wr_str = f" (WR: {wr_b*100:.1f}% -> {wr_a*100:.1f}%)"

        print(f"\nSwap {s['swap_index']}: -{rm}  +{add}{wr_str}")
        print(f"  {'Archetype':<25} {'Before':>7} {'After':>7} {'Delta':>7}")
        print(f"  {'-'*48}")
        for md in s["matchup_deltas"][:5]:  # Show top 5 most affected
            print(
                f"  {md['archetype']:<25} "
                f"{md['wr_before']*100:6.1f}% "
                f"{md['wr_after']*100:6.1f}% "
                f"{md['wr_delta']*100:+6.1f}%"
            )
        if len(s["matchup_deltas"]) > 5:
            print(f"  ... and {len(s['matchup_deltas']) - 5} more matchups")


def main() -> None:
    parser = argparse.ArgumentParser(
        prog="architect_explain",
        description="Interpret architect agent decisions.",
    )
    parser.add_argument("run_dir", help="Path to training run directory")
    parser.add_argument(
        "analysis",
        choices=["q-values", "matchups", "sensitivity", "importance", "swaps", "summary"],
        help="Analysis to run",
    )
    parser.add_argument("--deck", help="Comma-separated card IDs (default: optimized deck)")
    parser.add_argument("--games", type=int, default=50, help="Games per matchup eval")
    parser.add_argument("--top-k", type=int, default=10, help="Top-K for Q-value analysis")
    parser.add_argument("--json", action="store_true", help="Output as JSON")
    parser.add_argument("--pilot", default="greedy", help="Pilot policy (default: greedy)")

    args = parser.parse_args()

    logging.basicConfig(
        level=logging.INFO,
        format="%(levelname)s %(name)s: %(message)s",
    )

    deck = args.deck.split(",") if args.deck else None

    explainer = ArchitectExplainer(
        run_dir=args.run_dir,
        pilot=args.pilot,
        games_per_eval=args.games,
    )

    if args.analysis == "q-values":
        result = explainer.q_value_landscape(deck=deck, top_k=args.top_k)
        if args.json:
            print(json.dumps(result, indent=2))
        else:
            _print_q_values(result)

    elif args.analysis == "matchups":
        result = explainer.matchup_breakdown(deck=deck, games=args.games)
        if args.json:
            print(json.dumps(result, indent=2))
        else:
            _print_matchups(result)

    elif args.analysis == "sensitivity":
        result = explainer.meta_sensitivity(deck=deck, games=args.games)
        if args.json:
            print(json.dumps(result, indent=2))
        else:
            _print_sensitivity(result)

    elif args.analysis == "importance":
        result = explainer.card_importance(deck=deck, games=args.games)
        if args.json:
            print(json.dumps(result, indent=2))
        else:
            _print_importance(result)

    elif args.analysis == "swaps":
        result = explainer.swap_narrative(games=args.games)
        if args.json:
            print(json.dumps(result, indent=2))
        else:
            _print_swaps(result)

    elif args.analysis == "summary":
        # Combined: q-values + matchups + sensitivity
        q_result = explainer.q_value_landscape(deck=deck, top_k=args.top_k)
        m_result = explainer.matchup_breakdown(deck=deck, games=args.games)
        s_result = explainer.meta_sensitivity(deck=deck, games=args.games)

        if args.json:
            print(json.dumps({
                "q_values": q_result,
                "matchups": m_result,
                "sensitivity": s_result,
            }, indent=2))
        else:
            _print_q_values(q_result)
            _print_matchups(m_result)
            _print_sensitivity(s_result)


if __name__ == "__main__":
    main()
