"""DigimonEnv: Gymnasium-compliant RL environment for the Digimon TCG.

Wraps HeadlessGame to provide a standard Gymnasium interface with:
- Proper observation_space and action_space definitions
- Gymnasium v1.0 API (5-tuple step, 2-tuple reset)
- Dense reward shaping (security delta, board presence)
- Action masking via info dict (compatible with SB3 MaskablePPO)
"""

import os
import random
import numpy as np
import logging
from typing import List, Tuple, Dict, Any, Optional
import gymnasium
from gymnasium import spaces
from engine_py_legacy.engine.runners.headless_game import HeadlessGame

try:
    from digimon_engine import RustHeadlessGame  # type: ignore
except ImportError:
    RustHeadlessGame = None


def _make_runner(deck1: List[str], deck2: List[str], seed: Optional[int] = None):
    """Build the game runner chosen by the `DIGIMON_BACKEND` env var.

    - `DIGIMON_BACKEND=rust` → `RustHeadlessGame` (requires the `digimon_engine`
      PyO3 wheel to be installed, e.g. via `maturin develop`).
    - anything else (including unset) → Python `HeadlessGame`.

    Both runners expose the same duck-typed surface: `step`, `get_action_mask`,
    `get_board_tensor`, and `is_game_over`. Rust runners also expose
    `get_rl_state()` for backend-neutral RL metadata.
    """
    backend = os.environ.get("DIGIMON_BACKEND", "py").lower()
    if backend == "rust":
        if RustHeadlessGame is None:
            raise RuntimeError(
                "DIGIMON_BACKEND=rust but digimon_engine wheel is not installed. "
                "Run `cd digimon-engine-py && maturin develop --release`."
            )
        return RustHeadlessGame(deck1, deck2, seed=seed)
    if seed is None:
        return HeadlessGame(deck1, deck2)
    rng_state = random.getstate()
    original_choice = random.choice

    def seeded_choice(seq):
        if list(seq) == [True, False]:
            return seed % 2 == 0
        return original_choice(seq)

    try:
        random.seed(seed)
        random.choice = seeded_choice
        return HeadlessGame(deck1, deck2)
    finally:
        random.choice = original_choice
        random.setstate(rng_state)
from engine_py_legacy.engine.game import (
    FIELD_SLOTS, TARGETS_PER_ATTACKER, FIELDS_PER_HAND,
    SECURITY_TARGET, BREEDING_SLOT,
)  # parity-doc: these geometry constants stay on Python until migrated
from engine_py_legacy.engine.data.enums import PendingAction  # parity-doc: Python-engine fallback path only
from digimon_engine import ACTION_SPACE_SIZE, GamePhase, TENSOR_SIZE

# Action Space Constants (re-exported for backward compatibility)
ACTION_PLAY_CARD_START = 0
ACTION_PLAY_CARD_END = 29
ACTION_TRASH_CARD_START = 30
ACTION_TRASH_CARD_END = 59
ACTION_HATCH = 60
ACTION_MOVE = 61
ACTION_PASS_TURN = 62
ACTION_ATTACK_START = 100
ACTION_ATTACK_END = 399
ACTION_DIGIVOLVE_START = 400
ACTION_DIGIVOLVE_END = 999
ACTION_EFFECT_START = 1000
ACTION_EFFECT_END = 1999
ACTION_SOURCE_START = 2000
ACTION_SOURCE_END = ACTION_SPACE_SIZE - 1  # 2167

logger = logging.getLogger(__name__)


class DigimonEnv(gymnasium.Env):
    """Gymnasium-compliant environment for Digimon TCG RL training.

    Observation: float32 tensor of shape (TENSOR_SIZE,) representing
                 the board state from Player 1's perspective.
    Action:      Discrete(ACTION_SPACE_SIZE) with action masking.
    Reward:      Dense shaping (security delta, board DP) + terminal ±1.0.

    Action masks are provided in the info dict under 'action_mask',
    compatible with SB3's MaskablePPO.
    """

    metadata = {"render_modes": ["ansi"]}

    def __init__(self, deck1: Optional[List[str]] = None,
                 deck2: Optional[List[str]] = None,
                 render_mode: Optional[str] = None,
                 max_turns: int = 100):
        super().__init__()

        # Observation and action spaces
        # Card ID slots can hold registry indices up to 20000;
        # scalar fields stay within normal game ranges.
        self.observation_space = spaces.Box(
            low=-10.0, high=20001.0,
            shape=(TENSOR_SIZE,), dtype=np.float32
        )
        self.action_space = spaces.Discrete(ACTION_SPACE_SIZE)

        # Deck configuration
        self._deck1 = deck1 if deck1 else ["ST1-01"] * 5 + ["ST1-03"] * 45
        self._deck2 = deck2 if deck2 else ["ST1-01"] * 5 + ["ST1-03"] * 45

        self.render_mode = render_mode
        self.max_turns = max_turns
        self.runner: Optional[HeadlessGame] = None
        self._step_count = 0

    @property
    def game(self):
        """Back-compat accessor for the legacy Python Game instance.

        Rust runners intentionally do not expose a Python Game object. Use
        current_player_id, is_game_over, winner_id, and _rl_state() for
        backend-neutral RL code.
        """
        return getattr(self.runner, "game", None) if self.runner else None

    def _rl_state(self) -> Dict[str, Any]:
        if self.runner is None:
            return {}
        get_state = getattr(self.runner, "get_rl_state", None)
        if get_state is not None:
            return dict(get_state())
        game = getattr(self.runner, "game", None)
        if game is None:
            return {}
        winner = getattr(game, "winner", None)
        winner_id = getattr(winner, "player_id", winner)
        return {
            "game_over": bool(getattr(game, "game_over", False)),
            "winner_id": winner_id,
            "current_player_id": int(getattr(game, "current_player_id", 1)),
            "phase": getattr(getattr(game, "current_phase", None), "name", None),
            "memory": getattr(game, "memory", 0),
            "p1_security": len(game.player1.security_cards),
            "p2_security": len(game.player2.security_cards),
            "p1_total_dp": sum((p.dp or 0) for p in game.player1.battle_area),
            "p2_total_dp": sum((p.dp or 0) for p in game.player2.battle_area),
        }

    @property
    def current_player_id(self) -> int:
        return int(self._rl_state().get("current_player_id", 1))

    @property
    def is_game_over(self) -> bool:
        if self.runner is None:
            return False
        return bool(self._rl_state().get("game_over", self.runner.is_game_over))

    @property
    def winner_id(self) -> Optional[int]:
        winner = self._rl_state().get("winner_id")
        return int(winner) if winner is not None else None

    def greedy_action(self) -> int:
        if self.runner is None:
            return ACTION_PASS_TURN
        choose = getattr(self.runner, "greedy_action", None)
        if choose is not None:
            return int(choose())
        return greedy_policy(self)

    def reset(self, seed: Optional[int] = None,
              options: Optional[Dict[str, Any]] = None
              ) -> Tuple[np.ndarray, Dict[str, Any]]:
        """Reset the environment and return initial observation.

        Args:
            seed: Optional random seed (passed to super for reproducibility).
            options: Optional dict. Supports 'deck1' and 'deck2' keys to
                     override decks for this episode.

        Returns:
            (observation, info) tuple per Gymnasium v1.0 API.
        """
        super().reset(seed=seed)

        # Allow per-episode deck overrides
        deck1 = self._deck1
        deck2 = self._deck2
        if options:
            deck1 = options.get("deck1", deck1)
            deck2 = options.get("deck2", deck2)

        self.runner = _make_runner(deck1, deck2, seed=seed)
        self._step_count = 0

        obs = self.runner.get_board_tensor(1)
        info = {"action_mask": self.action_mask()}
        return obs, info

    def step(self, action: int
             ) -> Tuple[np.ndarray, float, bool, bool, Dict[str, Any]]:
        """Execute one action and return the result.

        Args:
            action: Integer action from the action space.

        Returns:
            (observation, reward, terminated, truncated, info) — Gymnasium v1.0 API.
        """
        if self.runner is None:
            raise RuntimeError("Environment must be reset before stepping.")

        self.runner.step(action)
        self._step_count += 1

        obs = self.runner.get_board_tensor(1)
        terminated = self.is_game_over
        truncated = self._step_count >= self.max_turns * 10  # safety limit
        reward = self._compute_reward(terminated)

        if truncated and not terminated:
            # Force game conclusion on truncation
            terminated = True

        info = {"action_mask": self.action_mask()}
        return obs, reward, terminated, truncated, info

    def action_mask(self) -> np.ndarray:
        """Return boolean action mask for the current state.

        Returns:
            int8 array of shape (ACTION_SPACE_SIZE,) where 1 = valid action.
        """
        if self.runner:
            mask_float = self.runner.get_action_mask()
            return (mask_float > 0.5).astype(np.int8)
        return np.ones(ACTION_SPACE_SIZE, dtype=np.int8)

    def get_action_mask(self) -> np.ndarray:
        """Alias for action_mask() — backward compatibility with GameState."""
        if self.runner:
            mask_float = self.runner.get_action_mask()
            return mask_float > 0.5
        return np.ones(ACTION_SPACE_SIZE, dtype=bool)

    def _compute_reward(self, terminated: bool) -> float:
        """Compute reward with dense shaping and terminal bonuses.

        Terminal: +1.0 for win, -1.0 for loss, 0.0 for draw.
        Dense (per-step):
            - Security delta: (my_security - opp_security) × 0.01
            - Board presence: (my_total_DP - opp_total_DP) × 0.0001
        """
        state = self._rl_state()

        if terminated and bool(state.get("game_over", False)):
            winner_id = state.get("winner_id")
            if winner_id == 1:
                return 1.0
            if winner_id == 2:
                return -1.0
            return 0.0

        sec_delta = int(state.get("p1_security", 0)) - int(state.get("p2_security", 0))
        dp_delta = int(state.get("p1_total_dp", 0)) - int(state.get("p2_total_dp", 0))
        return float(sec_delta * 0.01 + dp_delta * 0.0001)

    def render(self) -> Optional[str]:
        """Render the current game state.

        Returns:
            String representation when render_mode='ansi', None otherwise.
        """
        if self.render_mode == "ansi" and self.runner:
            state = self.runner.game.to_json()
            lines = [
                f"Turn {state['TurnCount']} | Phase: {state['CurrentPhase']} | Memory: {state['MemoryGauge']}",
                f"P1: Hand={state['Player1']['HandCount']}, Security={state['Player1']['SecurityCount']}, "
                f"Board={state['Player1']['BattleAreaCount']}",
                f"P2: Hand={state['Player2']['HandCount']}, Security={state['Player2']['SecurityCount']}, "
                f"Board={state['Player2']['BattleAreaCount']}",
            ]
            if state["IsGameOver"]:
                lines.append(f"Game Over! Winner: Player {state['Winner']}")
            return "\n".join(lines)
        return None


# ─── Backward Compatibility ──────────────────────────────────────────

class GameState:
    """DEPRECATED: Use DigimonEnv instead.

    Thin wrapper providing the old 4-tuple step API for backward compatibility.
    New code should use DigimonEnv directly.
    """

    def __init__(self):
        self._env = DigimonEnv()
        self.done = False
        self.info: Dict[str, Any] = {}

    @property
    def runner(self):
        return self._env.runner

    @property
    def game(self):
        return self._env.game

    @property
    def max_turns(self):
        return self._env.max_turns

    @max_turns.setter
    def max_turns(self, value):
        self._env.max_turns = value

    def reset(self, deck1: Optional[List[str]] = None,
              deck2: Optional[List[str]] = None) -> Dict[str, np.ndarray]:
        options = {}
        if deck1:
            options["deck1"] = deck1
        if deck2:
            options["deck2"] = deck2
        obs, info = self._env.reset(options=options if options else None)
        self.done = False
        return {"tensor": obs}

    def get_observation(self) -> Dict[str, np.ndarray]:
        if self._env.runner:
            return {"tensor": self._env.runner.get_board_tensor(1)}
        return {"tensor": np.zeros(TENSOR_SIZE, dtype=np.float32)}

    def get_action_mask(self) -> np.ndarray:
        return self._env.get_action_mask()

    def step(self, action: int) -> Tuple[Dict[str, np.ndarray], float, bool, Dict[str, Any]]:
        if self.done:
            return self.get_observation(), 0.0, True, self.info

        obs, reward, terminated, truncated, info = self._env.step(action)
        self.done = terminated or truncated
        self.info = info
        return {"tensor": obs}, reward, self.done, self.info


# ─── Policies ─────────────────────────────────────────────────────────

def greedy_policy(env) -> int:
    """Heuristic opponent policy for training/simulation.

    Main phase priority:
      1) Digivolve while keeping turn.
      2) Attack.
      3) Play.
      4) Pass.

    Breeding priority:
      1) Hatch.
      2) Move.
      3) Pass.
    """
    if isinstance(env, DigimonEnv):
        mask = np.asarray(env.get_action_mask())
        rust_greedy = getattr(env.runner, "greedy_action", None) if env.runner else None
        if rust_greedy is not None:
            return int(rust_greedy())
        game = env.game
    else:
        mask = np.asarray(env.get_action_mask())
        game = env.game

    valid_actions = np.where(mask > 0)[0].astype(int)

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
