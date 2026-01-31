from __future__ import annotations
from typing import TYPE_CHECKING, Optional
import random
from python_impl.engine.data.enums import GamePhase, PendingAction
from python_impl.engine.core.player import Player

if TYPE_CHECKING:
    pass

class Game:
    def __init__(self):
        self.player1: Player = Player()
        self.player2: Player = Player()
        self.player1.player_name = "Player 1"
        self.player2.player_name = "Player 2"
        self.player1.player_id = 1
        self.player2.player_id = 2

        self.turn_player: Player = self.player1
        self.opponent_player: Player = self.player2

        # Memory relative to turn player.
        # Positive means turn player has memory.
        # Negative means opponent has memory (and turn should end).
        self.memory: int = 0

        self.turn_count: int = 0
        self.current_phase: GamePhase = GamePhase.Start
        self.pending_action: PendingAction = PendingAction.NO_ACTION
        self.game_over: bool = False
        self.winner: Optional[Player] = None

    def start_game(self):
        # Determine first player (random for now)
        if random.choice([True, False]):
            self.turn_player = self.player1
            self.opponent_player = self.player2
        else:
            self.turn_player = self.player2
            self.opponent_player = self.player1

        print(f"Game Started. First player: {self.turn_player.player_name}")

        # These methods will be implemented in Player
        self.player1.setup_game()
        self.player2.setup_game()

        self.turn_count = 1
        self.current_phase = GamePhase.Start
        self.memory = 0

    def next_phase(self):
        if self.game_over:
            return

        print(f"Phase Transition: {self.current_phase.name} -> ", end="")

        if self.current_phase == GamePhase.Start:
            self.current_phase = GamePhase.Draw
            print(f"{self.current_phase.name}")
            self.phase_draw()
        elif self.current_phase == GamePhase.Draw:
            self.current_phase = GamePhase.Breeding
            print(f"{self.current_phase.name}")
            self.phase_breeding()
        elif self.current_phase == GamePhase.Breeding:
            self.current_phase = GamePhase.Main
            print(f"{self.current_phase.name}")
            self.phase_main()
        elif self.current_phase == GamePhase.Main:
            self.current_phase = GamePhase.End
            print(f"{self.current_phase.name}")
            self.phase_end()
        elif self.current_phase == GamePhase.End:
            self.switch_turn()
            print(f"{self.current_phase.name} (New Turn)")

    def phase_draw(self):
        # Player going first DOES NOT draw on their first turn (Turn 1).
        # We need to track absolute turns or just know if it's the very first turn.
        # self.turn_count starts at 1.
        if self.turn_count == 1:
            print("First turn: No draw.")
        else:
            self.turn_player.draw()

        self.next_phase()

    def phase_breeding(self):
        # In a real game, wait for user input.
        # For simplicity in this engine structure, we'll assume the player has done their breeding actions or chose to skip.
        # This function is just the entry to the phase.
        # In an interactive loop, we would yield or return here.
        # For this PoC, we will just proceed to Main Phase automatically,
        # assuming the 'Agent' or 'Loop' calls specific actions then calls next_phase.
        # But `next_phase` is recursive here.
        # To avoid infinite recursion in a synchronous script, we should probably STOP at Main Phase.
        pass

    def phase_main(self):
        # Main phase is where most actions happen.
        # We stop here and wait for actions.
        pass

    def phase_end(self):
        # Resolve end of turn effects.
        # Then switch turn.
        self.next_phase()

    def switch_turn(self):
        self.turn_player, self.opponent_player = self.opponent_player, self.turn_player
        self.turn_count += 1
        self.current_phase = GamePhase.Start

        # Memory handling
        # Memory is inverted because we switch perspective
        self.memory = -self.memory

        # If memory is still negative (meaning the previous player passed with memory on THEIR side?),
        # that shouldn't happen if we follow rules.
        # But standard rule: Start of turn, if memory <= 0, set to 3.
        # Wait, if I am now Turn Player, and previous player passed (memory = -3 for them),
        # Then for me, memory is 3.
        # So `self.memory` should be positive.

        if self.memory < 3: # If less than 3 (e.g. 1 or 2, or negative), set to 3?
            # No.
            # Rule: If memory is on your side, you keep it? No.
            # Rule: At start of turn, if memory is on Opponent's side (Negative for me), set to 3? No.
            # Rule: If the previous player ended their turn, the memory MUST be on my side (Positive for me).
            # If they passed, they gave me 3 memory.
            # So self.memory should be 3.
            pass

        # Specific logic: "If memory is at 0 or less on your side, it becomes 3."
        # Wait, if memory is on YOUR side, it's your turn.
        # If it's your turn start, memory MUST be on your side.
        pass

    def pass_turn(self):
        # Player chooses to pass.
        # Memory set to 3 on opponent side.
        # From current player perspective, memory = -3.
        if self.memory >= 0:
            self.memory = -3

        self.current_phase = GamePhase.End
        self.next_phase()
