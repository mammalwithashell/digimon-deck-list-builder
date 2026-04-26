"""Game recording: captures game state, actions, and optional tensors for replay and debugging.

Used by HeadlessGame for server-side recording and by InteractiveGame for
initial state capture (client-side recording metadata).
"""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime, timezone
from typing import TYPE_CHECKING, Any, Dict, List, Optional

if TYPE_CHECKING:
    from engine_py_legacy.engine.game import Game


@dataclass
class PlayerInitialState:
    """Post-shuffle zone order for one player, captured after start_game()."""

    player_id: int
    deck_list: List[str]  # Original deck list as submitted (pre-shuffle)
    library_order: List[str]  # Card IDs in library after shuffle (index 0 = top)
    digitama_library_order: List[str]  # Egg deck after shuffle
    security_order: List[str]  # Security stack (index 0 = bottom, last = top)
    initial_hand: List[str]  # Opening hand card IDs


@dataclass
class InitialState:
    """Complete initial game state after start_game()."""

    first_player_id: int
    player1: PlayerInitialState
    player2: PlayerInitialState
    timestamp: str = field(
        default_factory=lambda: datetime.now(timezone.utc).isoformat()
    )


@dataclass
class RecordedAction:
    """A single game action in the recording."""

    step_number: int
    player_id: int
    action_id: int
    phase: str  # GamePhase.name
    memory_before: int
    memory_after: int
    turn_number: int
    is_game_over: bool = False
    winner_id: Optional[int] = None


@dataclass
class TensorSnapshot:
    """Optional tensor + action mask capture at a given step."""

    step_number: int
    player_id: int
    tensor: List[float]  # board state tensor (TENSOR_SIZE floats)
    action_mask: List[int]  # ACTION_SPACE_SIZE-int action mask


@dataclass
class ReplayStepResult:
    """Result of executing one replay step."""

    step_number: int
    action_id: int
    player_id: int
    phase_before: str
    phase_after: str
    memory_before: int
    memory_after: int
    turn_number: int
    is_game_over: bool
    winner_id: Optional[int]
    state: Dict[str, Any]  # game.to_json() snapshot after action
    verification_ok: Optional[bool] = None
    verification_errors: List[str] = field(default_factory=list)


class GameRecorder:
    """Records game actions and optionally tensor data.

    Used directly by HeadlessGame for server-side recording.
    Used by BaseGameRunner/InteractiveGame to capture initial state for API enrichment.
    """

    def __init__(self, record_tensors: bool = False) -> None:
        self.record_tensors = record_tensors
        self.initial_state: Optional[InitialState] = None
        self.actions: List[RecordedAction] = []
        self.tensor_snapshots: List[TensorSnapshot] = []
        self._step_counter: int = 0

    def capture_initial_state(self, game: Game) -> InitialState:
        """Capture post-shuffle initial state from a Game object.

        Must be called AFTER game.start_game() has completed (shuffles, security, hand).
        """

        def _capture_player(player) -> PlayerInitialState:
            return PlayerInitialState(
                player_id=player.player_id,
                deck_list=[],  # Backfilled by runner with original IDs
                library_order=[c.card_id for c in player.library_cards],
                digitama_library_order=[
                    c.card_id for c in player.digitama_library_cards
                ],
                security_order=[c.card_id for c in player.security_cards],
                initial_hand=[c.card_id for c in player.hand_cards],
            )

        self.initial_state = InitialState(
            first_player_id=game.turn_player.player_id,
            player1=_capture_player(game.player1),
            player2=_capture_player(game.player2),
        )
        return self.initial_state

    def record_action(
        self, game: Game, action_id: int, player_id: int
    ) -> RecordedAction:
        """Record a single action. Call BEFORE game.decode_action().

        Returns the RecordedAction so finalize_action() can update it after execution.
        """
        self._step_counter += 1
        action = RecordedAction(
            step_number=self._step_counter,
            player_id=player_id,
            action_id=action_id,
            phase=game.current_phase.name,
            memory_before=game.memory,
            memory_after=game.memory,  # Updated by finalize_action()
            turn_number=game.turn_count,
        )
        self.actions.append(action)
        return action

    def finalize_action(self, game: Game, action: RecordedAction) -> None:
        """Update the action with post-execution state. Call AFTER game.decode_action()."""
        action.memory_after = game.memory
        action.is_game_over = game.game_over
        action.winner_id = game.winner.player_id if game.winner else None

    def record_tensor(self, game: Game, player_id: int) -> None:
        """Capture tensor + action mask for current state.

        Only captures if record_tensors=True. Call before the action
        to capture the decision-point state.
        """
        if not self.record_tensors:
            return
        tensor = game.get_board_state_tensor(player_id)
        mask = game.get_action_mask(player_id)
        snapshot = TensorSnapshot(
            step_number=self._step_counter + 1,  # Will match next action's step
            player_id=player_id,
            tensor=list(tensor),
            action_mask=list(mask),
        )
        self.tensor_snapshots.append(snapshot)

    def to_dict(self) -> Dict[str, Any]:
        """Serialize the full recording to a JSON-serializable dict."""
        initial = None
        if self.initial_state:
            initial = {
                "first_player_id": self.initial_state.first_player_id,
                "timestamp": self.initial_state.timestamp,
                "player1": {
                    "player_id": self.initial_state.player1.player_id,
                    "deck_list": self.initial_state.player1.deck_list,
                    "library_order": self.initial_state.player1.library_order,
                    "digitama_library_order": self.initial_state.player1.digitama_library_order,
                    "security_order": self.initial_state.player1.security_order,
                    "initial_hand": self.initial_state.player1.initial_hand,
                },
                "player2": {
                    "player_id": self.initial_state.player2.player_id,
                    "deck_list": self.initial_state.player2.deck_list,
                    "library_order": self.initial_state.player2.library_order,
                    "digitama_library_order": self.initial_state.player2.digitama_library_order,
                    "security_order": self.initial_state.player2.security_order,
                    "initial_hand": self.initial_state.player2.initial_hand,
                },
            }

        result: Dict[str, Any] = {
            "initial_state": initial,
            "actions": [
                {
                    "step": a.step_number,
                    "player_id": a.player_id,
                    "action_id": a.action_id,
                    "phase": a.phase,
                    "memory_before": a.memory_before,
                    "memory_after": a.memory_after,
                    "turn": a.turn_number,
                    "is_game_over": a.is_game_over,
                    "winner_id": a.winner_id,
                }
                for a in self.actions
            ],
            "total_actions": len(self.actions),
            "tensor_snapshots_count": len(self.tensor_snapshots),
        }

        if self.tensor_snapshots:
            result["tensor_snapshots"] = [
                {
                    "step": s.step_number,
                    "player_id": s.player_id,
                    "tensor": s.tensor,
                    "action_mask": s.action_mask,
                }
                for s in self.tensor_snapshots
            ]

        return result
