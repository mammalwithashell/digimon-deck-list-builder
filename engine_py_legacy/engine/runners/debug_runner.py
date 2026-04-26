"""DebugRunner: in-process debug game runner for AI-assisted QA testing.

Primary interface for programmatic game testing. No HTTP server required.
Provides state injection, structured action execution, state snapshots,
and human-readable output via the formatter module.

Usage:
    from digimon_gym.engine.runners.debug_runner import DebugRunner

    runner = DebugRunner(deck1_ids, deck2_ids)
    runner.place_on_field(2, ["ST1-03"])  # target on opponent field
    runner.set_memory(8)

    print(runner.board())
    action = runner.find_action("Play Medusamon")
    result = runner.execute(action)
    print(result.diff_summary)
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional

from digimon_gym.engine.game import Game
from digimon_gym.engine.loggers import EventLogger
from digimon_gym.engine.data.card_database import CardDatabase
from digimon_gym.engine.data.card_registry import CardRegistry
from digimon_gym.engine.data.enums import GamePhase
from digimon_gym.engine.core.permanent import Permanent
from digimon_gym.engine.core.player import Player
from digimon_gym.engine.debug.state_injection import (
    place_on_field as _place_on_field,
    place_in_breeding as _place_in_breeding,
    inject_card as _inject_card,
    clear_zone as _clear_zone,
    bulk_setup as _bulk_setup,
    BulkSetupConfig,
)


# ── Data classes ─────────────────────────────────────────────────────────


@dataclass
class FieldSlot:
    """Snapshot of a single permanent on the field."""
    slot: int
    card_id: str
    card_name: str
    level: Optional[int]
    dp: Optional[int]
    is_suspended: bool
    is_digimon: bool
    is_tamer: bool
    stack_ids: List[str]  # bottom-to-top card IDs
    keywords: List[str]


@dataclass
class StateSnapshot:
    """Lightweight, comparable game state snapshot."""
    memory: int
    turn: int
    phase: str
    active_player: int
    is_game_over: bool
    winner: Optional[int]
    p1_hand: List[str]
    p2_hand: List[str]
    p1_field: List[FieldSlot]
    p2_field: List[FieldSlot]
    p1_security_count: int
    p2_security_count: int
    p1_trash: List[str]
    p2_trash: List[str]
    p1_breeding: Optional[str]  # top card_id or None
    p2_breeding: Optional[str]
    p1_library_size: int
    p2_library_size: int


@dataclass
class ActionResult:
    """Structured result from executing an action."""
    action_id: int
    action_description: str
    before: StateSnapshot
    after: StateSnapshot
    logs: List[str]
    events: List[Dict[str, Any]]
    is_game_over: bool
    diff_summary: str


# ── DebugRunner ──────────────────────────────────────────────────────────


class DebugRunner:
    """In-process debug game runner. No HTTP server required.

    Creates a game with deterministic setup (skip_shuffle, auto_mulligan)
    and provides methods for state injection, action execution, and
    state inspection with human-readable output.
    """

    def __init__(
        self,
        deck1: List[str],
        deck2: List[str],
        skip_shuffle: bool = True,
        auto_mulligan: bool = True,
        first_player: int = 1,
        initial_memory: int = 0,
        starting_hand1: Optional[List[str]] = None,
        starting_hand2: Optional[List[str]] = None,
    ):
        CardRegistry.ensure_initialized()
        self._logger = EventLogger()
        self.game = Game(self._logger)

        # Monkey-patch shuffle if needed
        original_shuffle = Player.shuffle_for_game_start
        if skip_shuffle:
            Player.shuffle_for_game_start = lambda self: None

        try:
            self._setup_deck(self.game.player1, deck1)
            self._setup_deck(self.game.player2, deck2)
            self.game.start_game()
        finally:
            Player.shuffle_for_game_start = original_shuffle

        # Fix first player
        desired_first = self.game.player1 if first_player == 1 else self.game.player2
        if self.game.turn_player != desired_first:
            self.game.turn_player, self.game.opponent_player = (
                self.game.opponent_player, self.game.turn_player
            )
            self.game.turn_player.is_my_turn = True
            self.game.opponent_player.is_my_turn = False
            self.game._mulligan_order = [self.game.turn_player, self.game.opponent_player]
            self.game._mulligan_index = 0
            self.game.active_player = self.game._mulligan_order[0]

        # Set starting hands
        if starting_hand1:
            self._set_starting_hand(self.game.player1, starting_hand1)
        if starting_hand2:
            self._set_starting_hand(self.game.player2, starting_hand2)

        # Auto-mulligan
        if auto_mulligan:
            while self.game.current_phase == GamePhase.Mulligan:
                self.game.decode_action(0, self.game.current_player_id)

        # Set initial memory
        self.game.memory = initial_memory

        # Clear setup logs
        self._logger.clear()

    # ── Deck setup ───────────────────────────────────────────────────

    @staticmethod
    def _setup_deck(player: Player, card_ids: List[str]):
        db = CardDatabase()
        for card_id in card_ids:
            cs = db.create_card_source(card_id, player)
            if cs is None:
                continue
            if cs.is_digi_egg:
                player.digitama_library_cards.append(cs)
            else:
                player.library_cards.append(cs)

    @staticmethod
    def _set_starting_hand(player: Player, card_ids: List[str]):
        for card in reversed(player.hand_cards):
            player.library_cards.insert(0, card)
        player.hand_cards.clear()
        for wanted_id in card_ids:
            for i, card in enumerate(player.library_cards):
                if card.c_entity_base and card.c_entity_base.card_id == wanted_id:
                    player.hand_cards.append(player.library_cards.pop(i))
                    break

    # ── State injection (delegates to state_injection.py) ────────────

    def place_on_field(
        self, player_id: int, card_ids: List[str],
        is_suspended: bool = False, turn_played: int = -1,
    ) -> Permanent:
        return _place_on_field(self.game, player_id, card_ids, is_suspended, turn_played)

    def place_in_breeding(self, player_id: int, card_ids: List[str]) -> Permanent:
        return _place_in_breeding(self.game, player_id, card_ids)

    def inject_card(self, player_id: int, card_id: str, zone: str = "hand"):
        return _inject_card(self.game, player_id, card_id, zone)

    def set_memory(self, value: int):
        self.game.memory = value

    def set_phase(self, phase: str):
        self.game.current_phase = GamePhase[phase]

    def clear_zone(self, player_id: int, zone: str) -> int:
        return _clear_zone(self.game, player_id, zone)

    def bulk_setup(self, config: dict | BulkSetupConfig) -> dict:
        if isinstance(config, dict):
            config = BulkSetupConfig.from_dict(config)
        return _bulk_setup(self.game, config)

    # ── Action execution ─────────────────────────────────────────────

    def actions(self) -> Dict[int, str]:
        """Return {action_id: description} for all legal actions."""
        return self.game.describe_actions(self.game.current_player_id)

    def action_mask(self) -> List[int]:
        """Return list of legal action indices."""
        mask = self.game.get_action_mask(self.game.current_player_id)
        return [i for i, m in enumerate(mask) if m == 1.0]

    def execute(self, action_id: int) -> ActionResult:
        """Execute an action and return structured result with before/after diff."""
        before = self.snapshot()

        # Get description before executing
        descs = self.game.describe_actions(self.game.current_player_id)
        desc = descs.get(action_id, f"Action {action_id}")

        # Clear logs to capture only this action's output
        self._logger.clear()

        # Execute
        self.game.decode_action(action_id, self.game.current_player_id)

        after = self.snapshot()
        logs = self._logger.get_logs()
        events = self._logger.get_events()

        # Import formatter lazily to avoid circular imports
        from digimon_gym.engine.debug.formatter import format_diff
        diff_summary = format_diff(before, after)

        return ActionResult(
            action_id=action_id,
            action_description=desc,
            before=before,
            after=after,
            logs=logs,
            events=events,
            is_game_over=self.game.game_over,
            diff_summary=diff_summary,
        )

    def find_action(self, substring: str) -> Optional[int]:
        """Find first action whose description contains the substring (case-insensitive)."""
        sub_lower = substring.lower()
        for action_id, desc in self.actions().items():
            if sub_lower in desc.lower():
                return action_id
        return None

    def find_actions(self, substring: str) -> Dict[int, str]:
        """Find all actions matching a substring."""
        sub_lower = substring.lower()
        return {
            aid: desc for aid, desc in self.actions().items()
            if sub_lower in desc.lower()
        }

    def auto_resolve(self, max_steps: int = 20) -> List[ActionResult]:
        """Auto-resolve follow-up selection phases by picking the first legal action.

        Useful for resolving mandatory selections after playing a card.
        Stops when returning to Main phase, game is over, or max_steps reached.
        """
        results = []
        for _ in range(max_steps):
            if self.game.game_over:
                break
            # Stop if we're back in a "normal" phase (Main, Breeding, End)
            if self.game.current_phase in (GamePhase.Main, GamePhase.Breeding, GamePhase.End):
                break
            legal = self.action_mask()
            if not legal:
                break
            results.append(self.execute(legal[0]))
        return results

    # ── State inspection ─────────────────────────────────────────────

    def snapshot(self) -> StateSnapshot:
        """Capture current game state as a comparable snapshot."""
        game = self.game

        def _card_id(cs) -> str:
            return cs.c_entity_base.card_id if cs.c_entity_base else "unknown"

        def _card_name(cs) -> str:
            return cs.c_entity_base.card_name_eng if cs.c_entity_base else "unknown"

        def _field_slots(player: Player) -> List[FieldSlot]:
            slots = []
            for i, perm in enumerate(player.battle_area):
                top = perm.top_card
                keywords = []
                for kw in ["blocker", "rush", "piercing", "reboot", "security_attack_plus",
                           "jamming", "barrier", "armor_purge", "alliance", "vortex",
                           "overclock", "blast_digivolve", "material_save", "decoy",
                           "evade", "fortitude", "mind_link", "collision"]:
                    attr = f"_is_{kw}"
                    if hasattr(perm, 'has_keyword') and perm.has_keyword(attr):
                        keywords.append(kw)
                slots.append(FieldSlot(
                    slot=i,
                    card_id=_card_id(top) if top else "empty",
                    card_name=_card_name(top) if top else "empty",
                    level=perm.level,
                    dp=perm.dp,
                    is_suspended=perm.is_suspended,
                    is_digimon=perm.is_digimon,
                    is_tamer=perm.is_tamer,
                    stack_ids=[_card_id(cs) for cs in perm.card_sources],
                    keywords=keywords,
                ))
            return slots

        def _breeding_top(player: Player) -> Optional[str]:
            if player.breeding_area and player.breeding_area.top_card:
                return _card_id(player.breeding_area.top_card)
            return None

        return StateSnapshot(
            memory=game.memory,
            turn=game.turn_count,
            phase=game.current_phase.name,
            active_player=game.current_player_id,
            is_game_over=game.game_over,
            winner=game.winner.player_id if game.winner else None,
            p1_hand=[_card_id(c) for c in game.player1.hand_cards],
            p2_hand=[_card_id(c) for c in game.player2.hand_cards],
            p1_field=_field_slots(game.player1),
            p2_field=_field_slots(game.player2),
            p1_security_count=len(game.player1.security_cards),
            p2_security_count=len(game.player2.security_cards),
            p1_trash=[_card_id(c) for c in game.player1.trash_cards],
            p2_trash=[_card_id(c) for c in game.player2.trash_cards],
            p1_breeding=_breeding_top(game.player1),
            p2_breeding=_breeding_top(game.player2),
            p1_library_size=len(game.player1.library_cards),
            p2_library_size=len(game.player2.library_cards),
        )

    def board(self) -> str:
        """Return formatted board state string."""
        from digimon_gym.engine.debug.formatter import format_board
        return format_board(self.game, self.snapshot())

    def board_with_actions(self) -> str:
        """Return formatted board state + available actions."""
        from digimon_gym.engine.debug.formatter import format_board, format_actions
        board = format_board(self.game, self.snapshot())
        acts = format_actions(self.actions())
        return f"{board}\n{acts}"
