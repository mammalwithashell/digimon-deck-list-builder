import os
import sys

# Add repo root to path
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from python_impl.engine.game import Game
from python_impl.engine.data.card_database import CardDatabase

def run_sim():
    # Load Cards
    # Database singleton automatically loads cards.json from its own directory
    db = CardDatabase()

    print(f"Database loaded with {len(db.cards)} cards.")

    # Create Game
    game = Game()

    # Setup Decks
    # Player 1 Deck: 4 Agumon, 4 Greymon
    # Player 1 Eggs: 4 Koromon

    deck_list = ["ST1-03"] * 25 + ["ST1-07"] * 25
    egg_list = ["ST1-01"] * 5

    # Helper to populate player
    def populate_player(player, deck_ids, egg_ids):
        for cid in deck_ids:
            card = db.create_card_source(cid, player)
            if card:
                player.library_cards.append(card)

        for cid in egg_ids:
            card = db.create_card_source(cid, player)
            if card:
                player.digitama_library_cards.append(card)

    populate_player(game.player1, deck_list, egg_list)
    populate_player(game.player2, deck_list, egg_list)

    print(f"Player 1 Deck Size: {len(game.player1.library_cards)}")
    print(f"Player 1 Egg Deck Size: {len(game.player1.digitama_library_cards)}")

    # Start Game
    game.start_game()

    # Game performs setup: Draws 5 security, Draws 5 hand.
    # Check Hand
    p1 = game.player1
    p2 = game.player2
    print(f"P1 Hand: {[c.card_names[0] for c in p1.hand_cards]}")
    print(f"P1 Security: {len(p1.security_cards)}")

    # Current Phase should be Start -> Draw (if not turn 1) -> Breeding -> Main
    # Since start_game just sets state, we need to call next_phase to start the flow.
    # But start_game set phase to Start.

    # Let's manually step
    print("\n--- Starting Turn Flow ---")
    game.next_phase() # Start -> Draw (Executes Draw) -> Breeding (Stop)

    print(f"Current Phase: {game.current_phase.name}")
    print(f"Turn Player: {game.turn_player.player_name}")

    # Breeding Phase
    if game.current_phase.name == "Breeding":
        # Do hatch
        game.turn_player.hatch()
        # Pass phase
        game.next_phase() # Breeding -> Main (Stop)

    print(f"Current Phase: {game.current_phase.name}")

    # Main Phase
    if game.current_phase.name == "Main":
        # Play a card
        # Find Agumon
        agumon = next((c for c in game.turn_player.hand_cards if c.card_names[0] == "Agumon"), None)
        if agumon:
            game.turn_player.play_card(agumon)

            # Check field
            print(f"Field: {[p.top_card.card_names[0] for p in game.turn_player.battle_area]}")

        # Pass turn
        game.pass_turn()

    print(f"Current Phase: {game.current_phase.name}")
    print(f"Turn Count: {game.turn_count}")
    print(f"Turn Player: {game.turn_player.player_name}")
    print(f"Memory: {game.memory}")

if __name__ == "__main__":
    run_sim()
