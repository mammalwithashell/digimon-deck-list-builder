import sys
import os

# Add repo root to path
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..')))

from python_impl.engine.data.enums import CardColor, CardKind, GamePhase, EffectTiming
from python_impl.engine.core.entity_base import CEntity_Base
from python_impl.engine.data.evo_cost import EvoCost
from python_impl.engine.core.card_source import CardSource
from python_impl.engine.core.permanent import Permanent
from python_impl.engine.game import Game

def create_mock_card(name, kind=CardKind.Digimon, level=3, play_cost=3, dp=2000, evo_cost=0, color=CardColor.Red):
    entity = CEntity_Base()
    entity.card_name_eng = name
    entity.card_kind = kind
    entity.level = level
    entity.play_cost = play_cost
    entity.dp = dp
    entity.card_colors.append(color)

    if evo_cost > 0:
        cost = EvoCost(color, level - 1, evo_cost)
        entity.evo_costs.append(cost)

    card = CardSource()
    card.set_base_data(entity, None)
    return card

def test_game_loop():
    print("Testing Game Loop...")
    game = Game()

    # Mock Decks (Size 20 to avoid deck out)
    deck1 = [create_mock_card(f"Card1_{i}") for i in range(20)]
    eggs1 = [create_mock_card(f"Egg1_{i}", CardKind.DigiEgg, level=2, play_cost=0, dp=0) for i in range(5)]

    deck2 = [create_mock_card(f"Card2_{i}") for i in range(20)]
    eggs2 = [create_mock_card(f"Egg2_{i}", CardKind.DigiEgg, level=2, play_cost=0, dp=0) for i in range(5)]

    game.player1.library_cards = deck1
    game.player1.digitama_library_cards = eggs1
    game.player1.player_name = "Player 1"

    game.player2.library_cards = deck2
    game.player2.digitama_library_cards = eggs2
    game.player2.player_name = "Player 2"

    # Start Game
    print("--- Start Game ---")
    game.start_game()
    # Assume P1 went first.
    # start_game calls setup_game -> draw 5.
    # Then calls phase_start -> next_phase -> draw -> breeding -> main.
    # But wait, phase_main is a pass-through in my code currently?
    # No, phase_main prints waiting and returns.

    current_player = game.turn_player
    print(f"Turn Player: {current_player.player_name}")
    print(f"Phase: {game.current_phase}")
    print(f"Memory: {game.memory}")

    # Expect Main Phase (if Turn 1, Draw skipped?)
    # game.start_game calls phase_start.
    # phase_start -> next_phase (Start -> Draw)
    # phase_draw (if turn 1, skip draw) -> next_phase (Draw -> Breeding)
    # phase_breeding -> prints -> returns (state remains Breeding?)
    # Wait, my phase_breeding logic was:
    # print("Breeding Phase: Waiting for action (hatch/move/pass).")
    # pass
    # It does NOT call next_phase. So it STAYS in Breeding?
    # But next_phase called phase_breeding.
    # next_phase logic:
    # elif self.current_phase == GamePhase.Draw:
    #     self.current_phase = GamePhase.Breeding
    #     self.phase_breeding()
    # phase_breeding returns. next_phase continues? NO.
    # next_phase is one big IF/ELIF.
    # So if phase_breeding returns, next_phase finishes.
    # So YES, we are in Breeding Phase.

    assert game.current_phase == GamePhase.Breeding

    # Action: Hatch
    print("Action: Hatch")
    current_player.hatch()
    assert current_player.breeding_area is not None

    # Action: Pass Breeding (Manual transition to Main)
    print("Action: Proceed to Main")
    # Game doesn't have explicit "Go to Main" action in API yet, usually just next_phase logic.
    # I should expose next_phase or specific transitions?
    # I'll call game.next_phase() to simulate "Pass Breeding".
    game.next_phase()

    assert game.current_phase == GamePhase.Main

    # Action: Play Card (Agumon Cost 3)
    print("Action: Play Card")
    agumon = create_mock_card("Agumon", level=3, play_cost=3)
    current_player.hand_cards.append(agumon)
    card_index = len(current_player.hand_cards) - 1

    game.action_play_card(card_index)

    # Cost 3. Memory 3 -> 0. Still P1 turn.
    print(f"New Phase: {game.current_phase}")
    print(f"New Memory: {game.memory}")

    assert game.memory == 0
    assert game.turn_player == current_player

    # Action: Pass Turn
    print("Action: Pass Turn")
    game.action_pass_turn()

    print(f"New Turn Player: {game.turn_player.player_name}")
    assert game.turn_player != current_player
    assert game.memory == 3 # Reset to 3 for new player
    assert game.current_phase == GamePhase.Breeding

    print(f"--- {game.turn_player.player_name} Turn ---")
    p2 = game.turn_player
    p2.hatch()
    game.next_phase() # To Main

    # P2 Digivolves? Need Level 3.
    # Assume P2 played a card last turn? No, mock setup was fresh.
    # Let's give P2 a card on field.
    gabumon = create_mock_card("Gabumon", level=3, play_cost=3)
    p2.battle_area.append(Permanent([gabumon]))

    garurumon = create_mock_card("Garurumon", level=4, evo_cost=2)
    p2.hand_cards.append(garurumon)

    print("Action: Digivolve")
    game.action_digivolve(0, len(p2.hand_cards)-1)

    # Cost 2. Memory 3 -> 1. Still P2 turn.
    print(f"Memory: {game.memory}")
    assert game.memory == 1
    assert game.turn_player == p2

    # P2 Attacks
    print("Action: Attack")
    game.action_attack_player(0)
    # Garurumon suspends.
    assert p2.battle_area[0].is_suspended == True

    print("Test Complete.")

if __name__ == "__main__":
    test_game_loop()
