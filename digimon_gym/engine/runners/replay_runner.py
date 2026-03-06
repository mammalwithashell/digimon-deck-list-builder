"""ReplayRunner: deterministic game replay from a recording.

Given a recording dict (initial_state + actions), reconstructs the game by
restoring post-shuffle zones and feeding back recorded action IDs sequentially.

Does NOT subclass BaseGameRunner because BaseGameRunner.__init__ calls
game.start_game() which shuffles decks and randomly picks first player.
ReplayRunner bypasses all randomness by directly placing cards into zones.
"""

from __future__ import annotations

from typing import Any, Dict, List, Optional

import numpy as np

from digimon_gym.engine.game import Game, ACTION_SPACE_SIZE, TENSOR_SIZE
from digimon_gym.engine.loggers import IGameLogger, SilentLogger
from digimon_gym.engine.data.card_database import CardDatabase
from digimon_gym.engine.data.card_registry import CardRegistry
from digimon_gym.engine.data.enums import GamePhase
from digimon_gym.engine.recording import ReplayStepResult


class ReplayRunner:
    """Deterministic game replay from a recording dict.

    Restores post-shuffle zone order from the recording's initial_state,
    then steps through recorded actions one at a time.

    Args:
        recording: Full recording dict with ``initial_state`` and ``actions`` keys.
        logger: Optional game logger (default SilentLogger).
        verify: If True, compares replayed state against recorded values at each step.
    """

    def __init__(
        self,
        recording: Dict[str, Any],
        logger: Optional[IGameLogger] = None,
        verify: bool = False,
    ) -> None:
        self._recording = recording
        self._verify = verify
        self._logger = logger or SilentLogger()
        self._actions: List[Dict[str, Any]] = recording.get("actions", [])

        initial = recording.get("initial_state")
        if not initial:
            raise ValueError("Recording must contain 'initial_state'")

        self._initial_state = initial
        self._current_step: int = 0  # 0 = initial, N = after N actions

        # Build the game from recording
        self._build_game()

    # ── Public interface ────────────────────────────────────────────────

    @property
    def current_step(self) -> int:
        """Current step number (0 = initial state, N = after N actions)."""
        return self._current_step

    @property
    def total_steps(self) -> int:
        """Total number of recorded actions."""
        return len(self._actions)

    @property
    def is_complete(self) -> bool:
        """True if all recorded actions have been replayed."""
        return self._current_step >= len(self._actions)

    @property
    def is_game_over(self) -> bool:
        return self.game.game_over

    @property
    def winner_id(self) -> Optional[int]:
        return self.game.winner.player_id if self.game.winner else None

    def step(self) -> ReplayStepResult:
        """Execute the next recorded action.

        Returns:
            ReplayStepResult with state snapshot after the action.

        Raises:
            IndexError: If no more actions to replay.
        """
        if self._current_step >= len(self._actions):
            raise IndexError(
                f"No more actions to replay (at step {self._current_step} "
                f"of {len(self._actions)})"
            )

        action_data = self._actions[self._current_step]
        action_id = action_data["action_id"]
        player_id = self.game.current_player_id

        # Capture pre-action state
        phase_before = self.game.current_phase.name
        memory_before = self.game.memory
        turn_before = self.game.turn_count

        # Execute the action
        self.game.decode_action(action_id, player_id)
        self._current_step += 1

        # Build result
        result = ReplayStepResult(
            step_number=self._current_step,
            action_id=action_id,
            player_id=player_id,
            phase_before=phase_before,
            phase_after=self.game.current_phase.name,
            memory_before=memory_before,
            memory_after=self.game.memory,
            turn_number=turn_before,
            is_game_over=self.game.game_over,
            winner_id=self.game.winner.player_id if self.game.winner else None,
            state=self.game.to_json(),
        )

        # Optional verification
        if self._verify:
            self._verify_step(result, action_data)

        return result

    def seek(self, target_step: int) -> ReplayStepResult:
        """Jump to a specific step.

        Forward: replays remaining actions to reach target.
        Backward: rebuilds from scratch and replays to target.

        Args:
            target_step: Target step number (1 to total_steps).

        Returns:
            ReplayStepResult for the action at target_step.

        Raises:
            ValueError: If target_step is out of range.
        """
        if target_step < 1 or target_step > len(self._actions):
            raise ValueError(
                f"target_step must be 1..{len(self._actions)}, got {target_step}"
            )

        if target_step <= self._current_step:
            # Backward seek: rebuild from scratch
            self._build_game()

        # Forward: replay to target
        result = None
        while self._current_step < target_step:
            result = self.step()

        return result

    def run_to_completion(self) -> ReplayStepResult:
        """Execute all remaining recorded actions.

        Returns:
            ReplayStepResult for the final action.

        Raises:
            IndexError: If there are no actions to replay.
        """
        if not self._actions:
            raise IndexError("No actions in recording")

        result = None
        while self._current_step < len(self._actions):
            result = self.step()
        return result

    def get_state(self) -> Dict[str, Any]:
        """Return current game state as dictionary."""
        return self.game.to_json()

    def get_board_tensor(self, player_id: int) -> np.ndarray:
        """Return board state tensor for the given player."""
        return self.game.get_board_state_tensor(player_id)

    def get_action_mask(self) -> np.ndarray:
        """Return action mask for the current player."""
        mask_list = self.game.get_action_mask(self.game.current_player_id)
        return np.array(mask_list, dtype=np.float32)

    # ── Internal: game construction ────────────────────────────────────

    def _build_game(self) -> None:
        """Create a Game and restore zones from the recording's initial_state.

        This replaces the normal start_game() flow:
        1. Create Game (no start_game)
        2. Place cards directly into zones from recorded order
        3. Set first player, turn_count, memory, is_my_turn
        4. Call phase_start() to advance to Breeding (matching normal flow)
        """
        CardRegistry.ensure_initialized()
        db = CardDatabase()

        self.game = Game(self._logger)
        self._current_step = 0

        initial = self._initial_state
        first_player_id = initial["first_player_id"]

        # Set first player (bypasses random.choice)
        if first_player_id == 1:
            self.game.turn_player = self.game.player1
            self.game.opponent_player = self.game.player2
        else:
            self.game.turn_player = self.game.player2
            self.game.opponent_player = self.game.player1

        # Restore zones for each player
        self._restore_player_zones(self.game.player1, initial["player1"], db)
        self._restore_player_zones(self.game.player2, initial["player2"], db)

        # Set initial game state (matches end of start_game() before phase_start)
        self.game.turn_count = 1
        self.game.current_phase = GamePhase.Start
        self.game.memory = 0
        self.game.turn_player.is_my_turn = True
        self.game.opponent_player.is_my_turn = False

        # phase_start() chains Start → Draw → Breeding (park)
        # This matches the normal start_game() flow exactly
        self.game.phase_start()

    @staticmethod
    def _restore_player_zones(player, player_data: Dict[str, Any], db: CardDatabase) -> None:
        """Place cards directly into a player's zones from recorded order.

        Bypasses Player.setup_game() which would shuffle and draw randomly.
        """
        # Library (remaining deck after security + hand were drawn)
        player.library_cards = []
        for card_id in player_data.get("library_order", []):
            cs = db.create_card_source(card_id, player)
            if cs:
                player.library_cards.append(cs)

        # Digitama library (egg deck)
        player.digitama_library_cards = []
        for card_id in player_data.get("digitama_library_order", []):
            cs = db.create_card_source(card_id, player)
            if cs:
                player.digitama_library_cards.append(cs)

        # Security stack
        player.security_cards = []
        for card_id in player_data.get("security_order", []):
            cs = db.create_card_source(card_id, player)
            if cs:
                player.security_cards.append(cs)

        # Initial hand
        player.hand_cards = []
        for card_id in player_data.get("initial_hand", []):
            cs = db.create_card_source(card_id, player)
            if cs:
                player.hand_cards.append(cs)

    # ── Internal: verification ────────────────────────────────────────

    def _verify_step(self, result: ReplayStepResult, recorded: Dict[str, Any]) -> None:
        """Compare replayed state against recorded values.

        Mismatches are collected in result.verification_errors (non-fatal).
        """
        errors: List[str] = []

        # Memory after action
        recorded_memory = recorded.get("memory_after")
        if recorded_memory is not None and result.memory_after != recorded_memory:
            errors.append(
                f"memory_after mismatch at step {result.step_number}: "
                f"replayed={result.memory_after}, recorded={recorded_memory}"
            )

        # Turn number
        recorded_turn = recorded.get("turn")
        if recorded_turn is not None and result.turn_number != recorded_turn:
            errors.append(
                f"turn mismatch at step {result.step_number}: "
                f"replayed={result.turn_number}, recorded={recorded_turn}"
            )

        # Phase before action
        recorded_phase = recorded.get("phase")
        if recorded_phase is not None and result.phase_before != recorded_phase:
            errors.append(
                f"phase_before mismatch at step {result.step_number}: "
                f"replayed={result.phase_before}, recorded={recorded_phase}"
            )

        # Game over flag
        recorded_game_over = recorded.get("is_game_over")
        if recorded_game_over is not None and result.is_game_over != recorded_game_over:
            errors.append(
                f"is_game_over mismatch at step {result.step_number}: "
                f"replayed={result.is_game_over}, recorded={recorded_game_over}"
            )

        # Winner
        recorded_winner = recorded.get("winner_id")
        if recorded_winner is not None and result.winner_id != recorded_winner:
            errors.append(
                f"winner_id mismatch at step {result.step_number}: "
                f"replayed={result.winner_id}, recorded={recorded_winner}"
            )

        result.verification_errors = errors
        result.verification_ok = len(errors) == 0
