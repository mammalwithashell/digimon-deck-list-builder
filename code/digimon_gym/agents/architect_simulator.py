"""DeckSimulator: batch win-rate computation for deck evaluation.

Runs simulated games between decks using HeadlessGame and returns
win rates. Serves as the black-box evaluator for the deck-building
architect agent.

No DB imports. No auth. Pure engine + inference.

LEGACY-COUPLED MODULE: this drives the sunset Python engine
(``engine_py_legacy``) for the architect's win-rate simulation and is NOT
part of the legacy-free training surface. It is never imported by the
training entrypoint chain and is not shipped in ``Dockerfile.training``.
The ``engine_py_legacy`` import is lazy (see ``_load_headless_game`` and
``_headless_greedy_action``) so importing this module does not require the
legacy package to be present. See change make-training-build-legacy-free.
"""

from __future__ import annotations

import hashlib
import logging
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path
from typing import Dict, List, Optional, Protocol, Tuple, TYPE_CHECKING

import numpy as np

if TYPE_CHECKING:  # annotations only — never imported at runtime
    from engine_py_legacy.engine.runners.headless_game import HeadlessGame

logger = logging.getLogger(__name__)


def _load_headless_game():
    """Lazily import the legacy Python ``HeadlessGame`` runner.

    Keeping the import lazy means importing this module does not require
    ``engine_py_legacy`` to be installed; the legacy dependency is only
    materialized when the architect actually runs a simulation. See the
    module docstring and change make-training-build-legacy-free.
    """
    from engine_py_legacy.engine.runners.headless_game import HeadlessGame

    return HeadlessGame


# ---------------------------------------------------------------------------
# Policy protocol and implementations
# ---------------------------------------------------------------------------

class PilotPolicy(Protocol):
    """Uniform interface for any policy used with the simulator."""

    def predict(self, obs: np.ndarray, mask: np.ndarray) -> int: ...

    def reset(self) -> None: ...


def _headless_greedy_action(runner) -> int:
    """Python-engine greedy heuristic for the architect's ``HeadlessGame`` runner.

    This is the legacy heuristic formerly embedded in
    ``digimon_gym.digimon_gym.greedy_policy``. It operates on the Python ``Game``
    object exposed by ``HeadlessGame.game`` and is intentionally co-located with
    this module's (quarantined) Python-engine usage. The training env's
    ``greedy_policy`` now delegates to the Rust runner's native ``greedy_action``;
    this copy preserves the architect's opponent behavior unchanged. NOT part of
    the legacy-free training surface (see change make-training-build-legacy-free).

    Main phase priority: digivolve-while-keeping-turn, attack, play, pass.
    Breeding priority: hatch, move, pass.
    """
    from digimon_engine import GamePhase
    from engine_py_legacy.engine.game import (
        TARGETS_PER_ATTACKER,
        FIELDS_PER_HAND,
        SECURITY_TARGET,
        BREEDING_SLOT,
    )
    from engine_py_legacy.engine.data.enums import PendingAction
    from digimon_gym.digimon_gym import (
        ACTION_PASS_TURN,
        ACTION_HATCH,
        ACTION_MOVE,
        ACTION_PLAY_CARD_START,
        ACTION_PLAY_CARD_END,
        ACTION_TRASH_CARD_START,
        ACTION_TRASH_CARD_END,
        ACTION_DIGIVOLVE_START,
        ACTION_DIGIVOLVE_END,
        ACTION_ATTACK_START,
        ACTION_ATTACK_END,
    )

    mask = np.asarray(runner.get_action_mask())
    valid_actions = np.where(mask > 0)[0].astype(int)

    game = getattr(runner, "game", None)
    if game is None or len(valid_actions) == 0:
        return ACTION_PASS_TURN

    player = game.turn_player
    opponent = game.opponent_player

    def _first_non_pass() -> int:
        for action in valid_actions:
            if action != ACTION_PASS_TURN:
                return int(action)
        return ACTION_PASS_TURN

    if game.current_phase == GamePhase.Mulligan:
        acting = game.player1 if game.current_player_id == game.player1.player_id else game.player2
        has_level3 = any(
            card.is_digimon and (card.level or 0) == 3
            for card in acting.hand_cards
        )
        valid_set = set(int(a) for a in valid_actions)
        if not has_level3 and 1 in valid_set:
            return 1
        if 0 in valid_set:
            return 0
        return int(valid_actions[0])

    def _relative_memory() -> int:
        # Convert gauge to active-player-relative memory.
        return int(game.memory) if game.turn_player is game.player1 else int(-game.memory)

    def _estimate_digivolve_cost(card, base_perm) -> int:
        if not card.c_entity_base or not card.c_entity_base.evo_costs:
            return int(card.get_cost_itself)
        if not base_perm.top_card:
            return int(card.get_cost_itself)

        base_level = base_perm.level
        base_colors = set(base_perm.top_card.card_colors)
        matching_costs = []
        for evo_cost in card.c_entity_base.evo_costs:
            if evo_cost.level == base_level and evo_cost.card_color in base_colors:
                matching_costs.append(int(evo_cost.memory_cost))
        if not matching_costs:
            return int(card.get_cost_itself)
        return min(matching_costs)

    def _best_keep_turn_digivolve() -> Optional[int]:
        rel_memory = _relative_memory()
        keep_turn = []

        for action in valid_actions:
            if not (ACTION_DIGIVOLVE_START <= action <= ACTION_DIGIVOLVE_END):
                continue
            offset = int(action - ACTION_DIGIVOLVE_START)
            hand_idx = offset // FIELDS_PER_HAND
            field_idx = offset % FIELDS_PER_HAND

            if hand_idx >= len(player.hand_cards):
                continue
            card = player.hand_cards[hand_idx]

            if field_idx < len(player.battle_area):
                base_perm = player.battle_area[field_idx]
            elif field_idx == BREEDING_SLOT and player.breeding_area is not None:
                base_perm = player.breeding_area
            else:
                continue

            cost = _estimate_digivolve_cost(card, base_perm)
            if rel_memory - cost < 0:
                continue

            # Deterministic tie-breaks: level, DP, then lower cost.
            score = (
                int(card.level or 0),
                int(card.base_dp or 0),
                -cost,
                -hand_idx,
                -field_idx,
            )
            keep_turn.append((score, int(action)))

        if not keep_turn:
            return None
        keep_turn.sort(reverse=True)
        return keep_turn[0][1]

    def _best_attack() -> Optional[int]:
        attacks = []
        for action in valid_actions:
            if not (ACTION_ATTACK_START <= action <= ACTION_ATTACK_END):
                continue
            offset = int(action - ACTION_ATTACK_START)
            attacker_idx = offset // TARGETS_PER_ATTACKER
            target_idx = offset % TARGETS_PER_ATTACKER

            if attacker_idx >= len(player.battle_area):
                continue
            attacker = player.battle_area[attacker_idx]
            attacker_dp = int(attacker.dp or 0)

            if target_idx == SECURITY_TARGET:
                is_lethal = len(opponent.security_cards) == 0
                priority = 3 if is_lethal else 1
                score = (priority, attacker_dp, -attacker_idx)
                attacks.append((score, int(action)))
                continue

            if target_idx >= len(opponent.battle_area):
                continue
            target = opponent.battle_area[target_idx]
            target_dp = int(target.dp or 0)
            favorable = attacker_dp > target_dp
            priority = 2 if favorable else 0
            score = (priority, attacker_dp - target_dp, -attacker_idx, -target_idx)
            attacks.append((score, int(action)))

        if not attacks:
            return None
        attacks.sort(reverse=True)
        return attacks[0][1]

    def _best_play() -> Optional[int]:
        plays = []
        for action in valid_actions:
            if not (ACTION_PLAY_CARD_START <= action <= ACTION_PLAY_CARD_END):
                continue
            hand_idx = int(action - ACTION_PLAY_CARD_START)
            if hand_idx >= len(player.hand_cards):
                continue
            card = player.hand_cards[hand_idx]
            kind_score = 2 if card.is_digimon else (1 if card.is_option else 0)
            score = (int(card.get_cost_itself), kind_score, -hand_idx)
            plays.append((score, int(action)))
        if not plays:
            return None
        plays.sort(reverse=True)
        return plays[0][1]

    # Selection-time hand trashing: dump lowest-value card first.
    if game.pending_action == PendingAction.TRASH_CARD:
        trash_choices = []
        for action in valid_actions:
            if not (ACTION_TRASH_CARD_START <= action <= ACTION_TRASH_CARD_END):
                continue
            hand_idx = int(action - ACTION_TRASH_CARD_START)
            if hand_idx >= len(player.hand_cards):
                continue
            card = player.hand_cards[hand_idx]
            kind_score = 2 if card.is_digimon else (1 if card.is_option else 0)
            score = (kind_score, int(card.get_cost_itself), hand_idx)
            trash_choices.append((score, int(action)))
        if trash_choices:
            trash_choices.sort()
            return trash_choices[0][1]
        return _first_non_pass()

    if game.current_phase == GamePhase.Breeding:
        if ACTION_HATCH in valid_actions:
            return ACTION_HATCH
        if ACTION_MOVE in valid_actions:
            return ACTION_MOVE
        if ACTION_PASS_TURN in valid_actions:
            return ACTION_PASS_TURN
        return _first_non_pass()

    if game.current_phase == GamePhase.Main:
        best_keep_turn_digivolve = _best_keep_turn_digivolve()
        if best_keep_turn_digivolve is not None:
            return best_keep_turn_digivolve

        best_attack = _best_attack()
        if best_attack is not None:
            return best_attack

        best_play = _best_play()
        if best_play is not None:
            return best_play

        if ACTION_PASS_TURN in valid_actions:
            return ACTION_PASS_TURN
        return _first_non_pass()

    if ACTION_PASS_TURN in valid_actions:
        return ACTION_PASS_TURN
    return _first_non_pass()


class _HeadlessGreedyPolicy:
    """Greedy policy that operates on a ``HeadlessGame`` runner reference.

    Delegates to ``_headless_greedy_action`` (the relocated Python-engine
    heuristic) which inspects game state (phases, hand cards, board, etc.).

    Call ``bind_runner()`` before each game loop.
    """

    def __init__(self):
        self._runner: Optional[HeadlessGame] = None

    def bind_runner(self, runner: HeadlessGame) -> None:
        self._runner = runner

    def predict(self, obs: np.ndarray, mask: np.ndarray) -> int:
        if self._runner is None:
            valid = np.where(mask > 0.5)[0]
            return int(valid[-1]) if len(valid) > 0 else 62

        return _headless_greedy_action(self._runner)

    def reset(self) -> None:
        self._runner = None


class _RandomPolicy:
    """Uniformly random valid action."""

    def __init__(self, seed: Optional[int] = None):
        self._rng = np.random.default_rng(seed)

    def predict(self, obs: np.ndarray, mask: np.ndarray) -> int:
        valid = np.where(mask > 0.5)[0]
        if len(valid) == 0:
            return 62  # ACTION_PASS_TURN
        return int(self._rng.choice(valid))

    def reset(self) -> None:
        pass


class _OnnxPolicy:
    """Wrapper around OnnxMlpPolicy / OnnxLstmPolicy."""

    def __init__(self, path: str):
        from digimon_gym.inference.onnx_policy import load_onnx_policy
        self._policy = load_onnx_policy(path, model_type="auto")
        self._is_lstm = hasattr(self._policy, "h")

    def predict(self, obs: np.ndarray, mask: np.ndarray) -> int:
        return self._policy.predict(obs, mask)

    def reset(self) -> None:
        if self._is_lstm:
            self._policy.reset()


class _SB3Policy:
    """Wrapper around SB3 MaskablePPO / MaskableRecurrentPPO."""

    def __init__(self, path: str):
        self._path = path
        self._model = None
        self._is_lstm = False
        self._state = None
        self._episode_start = True
        self._load()

    def _load(self):
        try:
            from sb3_contrib import MaskablePPO
            self._model = MaskablePPO.load(self._path)
            self._is_lstm = False
        except Exception:
            from digimon_gym.agents.maskable_recurrent import MaskableRecurrentPPO
            self._model = MaskableRecurrentPPO.load(self._path)
            self._is_lstm = True
            self._state = None
            self._episode_start = True

    def predict(self, obs: np.ndarray, mask: np.ndarray) -> int:
        if self._is_lstm:
            action, self._state = self._model.predict(
                obs, state=self._state, action_masks=mask,
                deterministic=True, episode_start=self._episode_start,
            )
            self._episode_start = False
        else:
            action, _ = self._model.predict(obs, action_masks=mask, deterministic=True)
        return int(action)

    def reset(self) -> None:
        if self._is_lstm:
            self._state = None
            self._episode_start = True


# ---------------------------------------------------------------------------
# Policy factory
# ---------------------------------------------------------------------------

def load_pilot_policy(path_or_name: str) -> PilotPolicy:
    """Factory that returns a uniform PilotPolicy for any pilot type.

    Args:
        path_or_name: One of:
            - ``"greedy"``: heuristic greedy policy (uses full game-state logic)
            - ``"random"``: uniformly random valid action
            - Path ending in ``.onnx``: ONNX model (MLP or LSTM auto-detected)
            - Path ending in ``.zip``: SB3 model (MaskablePPO or recurrent)

    Returns:
        An object satisfying PilotPolicy with ``.predict(obs, mask)``
        and ``.reset()``.
    """
    name = path_or_name.strip().lower()

    if name == "greedy":
        return _HeadlessGreedyPolicy()
    if name == "random":
        return _RandomPolicy()

    p = Path(path_or_name)
    if not p.exists():
        raise FileNotFoundError(f"Policy file not found: {path_or_name}")

    if p.suffix == ".onnx":
        return _OnnxPolicy(str(p))
    if p.suffix == ".zip":
        return _SB3Policy(str(p))

    raise ValueError(
        f"Unsupported policy path: {path_or_name}. "
        "Expected 'greedy', 'random', or a path to .onnx / .zip file."
    )


# ---------------------------------------------------------------------------
# Standalone matchup function (module-level for multiprocessing)
# ---------------------------------------------------------------------------

def _run_matchup(
    deck1: List[str],
    deck2: List[str],
    n_games: int,
    max_turns: int,
    pilot_path: str,
    opponent_path: str,
) -> Tuple[int, int, int]:
    """Run *n_games* between deck1 and deck2.

    Returns ``(wins, losses, draws)`` from deck1's perspective.
    Module-level so ProcessPoolExecutor workers can invoke it. Each worker
    loads its own policy instances to avoid sharing state across processes.
    """
    pilot = load_pilot_policy(pilot_path)
    opponent = load_pilot_policy(opponent_path)

    wins = losses = draws = 0

    for i in range(n_games):
        # Alternate sides: even games deck1=P1, odd games deck1=P2
        if i % 2 == 0:
            d1, d2 = deck1, deck2
            pilot_player, opp_player = 1, 2
        else:
            d1, d2 = deck2, deck1
            pilot_player, opp_player = 2, 1

        pilot.reset()
        opponent.reset()

        try:
            runner = _load_headless_game()(d1, d2)

            # Bind runner for greedy policies that need game state access
            if isinstance(pilot, _HeadlessGreedyPolicy):
                pilot.bind_runner(runner)
            if isinstance(opponent, _HeadlessGreedyPolicy):
                opponent.bind_runner(runner)

            steps = 0
            while not runner.is_game_over and steps < max_turns:
                current_pid = runner.game.current_player_id
                obs = runner.get_board_tensor(current_pid)
                mask = runner.get_action_mask()

                if current_pid == pilot_player:
                    action = pilot.predict(obs, mask)
                else:
                    action = opponent.predict(obs, mask)

                runner.step(action)
                steps += 1

            if not runner.is_game_over:
                draws += 1
            elif runner.winner_id == pilot_player:
                wins += 1
            elif runner.winner_id == opp_player:
                losses += 1
            else:
                draws += 1
        except Exception:
            draws += 1

    return wins, losses, draws


# ---------------------------------------------------------------------------
# DeckSimulator
# ---------------------------------------------------------------------------

class DeckSimulator:
    """Batch win-rate evaluator for deck optimization.

    Runs simulated games between a candidate deck and a set of opponent decks,
    returning a weighted win rate. Supports caching and optional multiprocessing.

    Args:
        pilot_policy_path: Path to pilot model or ``"greedy"`` / ``"random"``.
        opponent_policy: Policy for the opponent side (``"greedy"`` or ``"random"``).
        games_per_eval: Number of games to simulate per matchup evaluation.
        max_turns: Maximum action steps per game before forced draw.
        n_workers: Parallel worker processes (1 = single-process, avoids
            serialization overhead).
        cache_enabled: Whether to cache matchup results by deck hash.
    """

    def __init__(
        self,
        pilot_policy_path: str = "greedy",
        opponent_policy: str = "greedy",
        games_per_eval: int = 50,
        max_turns: int = 200,
        n_workers: int = 1,
        cache_enabled: bool = True,
    ):
        self.pilot_policy_path = pilot_policy_path
        self.opponent_policy_path = opponent_policy
        self.games_per_eval = games_per_eval
        self.max_turns = max_turns
        self.n_workers = max(1, n_workers)
        self.cache_enabled = cache_enabled

        # Cache: (deck_hash, opponent_hash) -> (wins, losses, draws)
        self._cache: Dict[Tuple[str, str], Tuple[int, int, int]] = {}

        # For single-process mode, load policies once and reuse them
        self._pilot: Optional[PilotPolicy] = None
        self._opponent: Optional[PilotPolicy] = None
        if self.n_workers == 1:
            self._pilot = load_pilot_policy(pilot_policy_path)
            self._opponent = load_pilot_policy(opponent_policy)

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def evaluate_deck(
        self,
        deck: List[str],
        opponents: List[Tuple[List[str], float]],
    ) -> float:
        """Compute weighted win rate of *deck* against opponent decks.

        Args:
            deck: Flat card-ID list for the deck under evaluation.
            opponents: List of ``(opponent_deck_ids, meta_weight)`` tuples.
                       Weights are normalised internally; they need not sum to 1.

        Returns:
            Weighted win rate in ``[0.0, 1.0]``.
        """
        if not opponents:
            raise ValueError("opponents list must not be empty")

        total_weight = sum(w for _, w in opponents)
        if total_weight <= 0:
            raise ValueError("Total opponent weight must be positive")

        # Distribute games_per_eval proportional to weight (min 2 per opponent
        # so each matchup gets at least one game per side).
        raw_alloc = [
            max(2, round(self.games_per_eval * (w / total_weight)))
            for _, w in opponents
        ]

        # Collect matchup results
        if self.n_workers > 1:
            matchup_results = self._evaluate_parallel(deck, opponents, raw_alloc)
        else:
            matchup_results = self._evaluate_sequential(deck, opponents, raw_alloc)

        # Weighted win rate
        weighted_wr = 0.0
        norm = sum(w for _, w in opponents)
        for (wins, losses, draws), (_, weight) in matchup_results:
            total = wins + losses + draws
            if total == 0:
                continue
            wr = (wins + 0.5 * draws) / total
            weighted_wr += wr * (weight / norm)

        return weighted_wr

    def clear_cache(self) -> None:
        """Clear the matchup result cache."""
        self._cache.clear()

    # ------------------------------------------------------------------
    # Internal — sequential (single process)
    # ------------------------------------------------------------------

    def _evaluate_sequential(
        self,
        deck: List[str],
        opponents: List[Tuple[List[str], float]],
        alloc: List[int],
    ) -> List[Tuple[Tuple[int, int, int], Tuple[List[str], float]]]:
        results = []
        for (opp_deck, weight), n_games in zip(opponents, alloc):
            result = self._simulate_matchup(deck, opp_deck, n_games)
            results.append((result, (opp_deck, weight)))
        return results

    def _simulate_matchup(
        self,
        deck1: List[str],
        deck2: List[str],
        n_games: int,
    ) -> Tuple[int, int, int]:
        """Run n_games between deck1 (pilot) and deck2 (opponent).

        Returns ``(wins, losses, draws)`` from deck1's perspective.
        Uses the pre-loaded policies (single-process mode).
        Results are cached by deck hash if caching is enabled.
        """
        key = (self._deck_hash(deck1), self._deck_hash(deck2))

        if self.cache_enabled and key in self._cache:
            return self._cache[key]

        pilot = self._pilot
        opponent = self._opponent

        wins = losses = draws = 0

        for i in range(n_games):
            # Alternate sides to remove first-player bias
            if i % 2 == 0:
                d1, d2 = deck1, deck2
                pilot_player, opp_player = 1, 2
            else:
                d1, d2 = deck2, deck1
                pilot_player, opp_player = 2, 1

            pilot.reset()
            opponent.reset()

            try:
                runner = _load_headless_game()(d1, d2)

                # Bind runner for greedy policies
                if isinstance(pilot, _HeadlessGreedyPolicy):
                    pilot.bind_runner(runner)
                if isinstance(opponent, _HeadlessGreedyPolicy):
                    opponent.bind_runner(runner)

                steps = 0
                while not runner.is_game_over and steps < self.max_turns:
                    current_pid = runner.game.current_player_id
                    obs = runner.get_board_tensor(current_pid)
                    mask = runner.get_action_mask()

                    if current_pid == pilot_player:
                        action = pilot.predict(obs, mask)
                    else:
                        action = opponent.predict(obs, mask)

                    runner.step(action)
                    steps += 1

                if not runner.is_game_over:
                    draws += 1
                elif runner.winner_id == pilot_player:
                    wins += 1
                elif runner.winner_id == opp_player:
                    losses += 1
                else:
                    draws += 1
            except Exception:
                # Card script bugs (infinite recursion, missing attributes, etc.)
                # count as draws to avoid poisoning win-rate estimates
                logger.debug("Game crashed, counting as draw", exc_info=True)
                draws += 1

        result = (wins, losses, draws)

        if self.cache_enabled:
            self._cache[key] = result

        return result

    # ------------------------------------------------------------------
    # Internal — parallel (multiprocessing)
    # ------------------------------------------------------------------

    def _evaluate_parallel(
        self,
        deck: List[str],
        opponents: List[Tuple[List[str], float]],
        alloc: List[int],
    ) -> List[Tuple[Tuple[int, int, int], Tuple[List[str], float]]]:
        """Distribute matchups across worker processes."""
        futures = []
        results: List[Optional[Tuple[Tuple[int, int, int], Tuple[List[str], float]]]] = (
            [None] * len(opponents)
        )

        with ProcessPoolExecutor(max_workers=self.n_workers) as pool:
            for idx, ((opp_deck, weight), n_games) in enumerate(
                zip(opponents, alloc)
            ):
                # Check cache first
                if self.cache_enabled:
                    key = (self._deck_hash(deck), self._deck_hash(opp_deck))
                    if key in self._cache:
                        results[idx] = (self._cache[key], (opp_deck, weight))
                        continue

                fut = pool.submit(
                    _run_matchup,
                    deck, opp_deck, n_games, self.max_turns,
                    self.pilot_policy_path, self.opponent_policy_path,
                )
                futures.append((idx, opp_deck, weight, fut))

            for idx, opp_deck, weight, fut in futures:
                result = fut.result()
                results[idx] = (result, (opp_deck, weight))

                if self.cache_enabled:
                    key = (self._deck_hash(deck), self._deck_hash(opp_deck))
                    self._cache[key] = result

        return results

    # ------------------------------------------------------------------
    # Utilities
    # ------------------------------------------------------------------

    @staticmethod
    def _deck_hash(deck: List[str]) -> str:
        """Deterministic hash of a deck (order-independent)."""
        canonical = "\n".join(sorted(deck))
        return hashlib.sha256(canonical.encode("utf-8")).hexdigest()[:16]
