"""DigimonEnv: Gymnasium-compliant RL environment for the Digimon TCG.

Wraps HeadlessGame to provide a standard Gymnasium interface with:
- Proper observation_space and action_space definitions
- Gymnasium v1.0 API (5-tuple step, 2-tuple reset)
- Dense reward shaping (security delta, board presence)
- Action masking via info dict (compatible with SB3 MaskablePPO)
"""

import numpy as np
import logging
from typing import List, Tuple, Dict, Any, Optional
import gymnasium
from gymnasium import spaces
from digimon_gym.engine.runners.headless_game import HeadlessGame
from digimon_gym.engine.game import TENSOR_SIZE, ACTION_SPACE_SIZE
from digimon_gym.engine.data.enums import PendingAction

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
ACTION_SOURCE_END = 2119

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
        self.observation_space = spaces.Box(
            low=-10.0, high=10000.0,
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
        """Back-compat accessor for the underlying Game instance."""
        return self.runner.game if self.runner else None

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

        self.runner = HeadlessGame(deck1, deck2)
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
        terminated = self.runner.is_game_over
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
        game = self.runner.game

        # Terminal reward
        if terminated and game.game_over:
            if game.winner and game.winner.player_id == 1:
                return 1.0
            elif game.winner:
                return -1.0
            return 0.0

        # Dense shaping
        me = game.player1
        opp = game.player2
        reward = 0.0

        # Security delta
        sec_delta = len(me.security_cards) - len(opp.security_cards)
        reward += sec_delta * 0.01

        # Board presence (total DP — None for tamers/eggs, treat as 0)
        my_dp = sum((p.dp or 0) for p in me.battle_area)
        opp_dp = sum((p.dp or 0) for p in opp.battle_area)
        reward += (my_dp - opp_dp) * 0.0001

        return reward

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
    """Selects an action based on a simple heuristic.

    Works with both DigimonEnv and GameState.
    """
    if isinstance(env, DigimonEnv):
        mask = env.get_action_mask()
        game = env.game
    else:
        mask = env.get_action_mask()
        game = env.game

    valid_actions = np.where(mask)[0]

    if len(valid_actions) == 0:
        return ACTION_PASS_TURN

    player = game.turn_player

    # Helper to score a card
    def get_card_score(card):
        if card.is_digimon:
            return 10
        elif card.is_option:
            return 5
        return 1

    # 1. TRASH DECISION
    if game.pending_action == PendingAction.TRASH_CARD:
        best_action = -1
        min_score = 999

        for action in valid_actions:
            if ACTION_TRASH_CARD_START <= action <= ACTION_TRASH_CARD_END:
                idx = action - ACTION_TRASH_CARD_START
                if idx < len(player.hand_cards):
                    score = get_card_score(player.hand_cards[idx])
                    if score < min_score:
                        min_score = score
                        best_action = action
        return best_action if best_action != -1 else valid_actions[0]

    # 2. MAIN PHASE
    # Priority: Hatch > Play High Score > Attack > Pass

    # Hatch?
    if ACTION_HATCH in valid_actions:
        return ACTION_HATCH

    # Play Card?
    best_play_action = -1
    max_play_score = -1
    for action in valid_actions:
        if ACTION_PLAY_CARD_START <= action <= ACTION_PLAY_CARD_END:
            idx = action - ACTION_PLAY_CARD_START
            if idx < len(player.hand_cards):
                score = get_card_score(player.hand_cards[idx])
                if score > max_play_score:
                    max_play_score = score
                    best_play_action = action

    if best_play_action != -1:
        return best_play_action

    # Pass if nothing else
    if ACTION_PASS_TURN in valid_actions:
        return ACTION_PASS_TURN

    return valid_actions[0]
