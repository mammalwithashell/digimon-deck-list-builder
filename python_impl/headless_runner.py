import time
import random
import json
from typing import List, Dict, Any, Tuple
from python_impl.engine.game import Game
from python_impl.engine.data.card_database import CardDatabase
from python_impl.engine.data.enums import GamePhase

class HeadlessRunner:
    def __init__(self, deck1_str: str, deck2_str: str, num_simulations: int):
        self.deck1_str = deck1_str
        self.deck2_str = deck2_str
        self.num_simulations = num_simulations
        self.db = CardDatabase()

    def run(self) -> Dict[str, Any]:
        deck1_main, deck1_eggs = self._parse_deck(self.deck1_str)
        deck2_main, deck2_eggs = self._parse_deck(self.deck2_str)

        results = {
            "p1_wins": 0,
            "p2_wins": 0,
            "draws": 0,
            "games_played": 0,
            "logs": []
        }

        start_time = time.time()

        for i in range(self.num_simulations):
            game_result, log = self._run_single_game(i, deck1_main, deck1_eggs, deck2_main, deck2_eggs)
            results["games_played"] += 1
            if game_result == 1:
                results["p1_wins"] += 1
            elif game_result == 2:
                results["p2_wins"] += 1
            else:
                results["draws"] += 1

            # Keep logs for first 5 games
            if i < 5:
                results["logs"].append(log)

        end_time = time.time()
        results["duration"] = end_time - start_time
        if results["games_played"] > 0:
            results["p1_win_rate"] = results["p1_wins"] / results["games_played"]
            results["p2_win_rate"] = results["p2_wins"] / results["games_played"]
        else:
            results["p1_win_rate"] = 0
            results["p2_win_rate"] = 0

        return results

    def _parse_deck(self, deck_input: str) -> Tuple[List[str], List[str]]:
        # Handle JSON array format (e.g. ["Header", "ID1", "ID2"...])
        # or simple newline separated

        card_ids = []
        # Attempt to parse as JSON first
        try:
            # If it looks like a list
            if deck_input.strip().startswith("["):
                data = json.loads(deck_input)
                if isinstance(data, list):
                    card_ids = [str(x) for x in data if "Exported" not in str(x)]
            else:
                raise ValueError("Not JSON")
        except:
             # Fallback to newline split
             card_ids = deck_input.split('\n')

        main_deck = []
        egg_deck = []

        for cid in card_ids:
            cid = cid.strip()
            if not cid:
                continue

            # Check if egg
            card = self.db.get_card(cid)
            # enum value 3 is DigiEgg
            if card and card.card_kind.value == 3:
                 egg_deck.append(cid)
            else:
                 main_deck.append(cid)

        return main_deck, egg_deck

    def _run_single_game(self, sim_id: int, d1_main: List[str], d1_eggs: List[str], d2_main: List[str], d2_eggs: List[str]) -> Tuple[int, Dict[str, Any]]:
        game = Game()

        # Populate players
        self._populate_player(game.player1, d1_main, d1_eggs)
        self._populate_player(game.player2, d2_main, d2_eggs)

        game.start_game()

        # Limit turns to prevent infinite loops in headless mode
        MAX_TURNS = 100
        log = {"sim_id": sim_id, "turns": []}

        actions_this_turn = 0

        while not game.game_over and game.turn_count < MAX_TURNS:
            # Check phases and inject actions

            # Record log at start of phase
            log["turns"].append(f"Turn {game.turn_count} - {game.current_phase.name} - {game.turn_player.player_name} (Mem: {game.memory})")

            if game.current_phase == GamePhase.Main:
                if actions_this_turn < 3: # Limit 3 moves per turn to prevent dumping hand instantly
                     action_taken = self._inject_main_action(game)
                     if action_taken:
                         actions_this_turn += 1
                     else:
                         game.pass_turn()
                         actions_this_turn = 0
                else:
                    game.pass_turn()
                    actions_this_turn = 0

            elif game.current_phase == GamePhase.Breeding:
                 self._inject_breeding_action(game)
                 game.next_phase()

            else:
                # Start, Draw, End phases
                game.next_phase()

                # Reset actions count if new turn starts (handled by pass_turn logic usually but good to be safe)
                if game.current_phase == GamePhase.Start:
                    actions_this_turn = 0

        winner = 0
        if game.winner:
            winner = game.winner.player_id
        else:
            # Tie or max turns
            pass

        return winner, log

    def _populate_player(self, player, deck_ids, egg_ids):
        for cid in deck_ids:
            card = self.db.create_card_source(cid, player)
            if card:
                player.library_cards.append(card)

        for cid in egg_ids:
            card = self.db.create_card_source(cid, player)
            if card:
                player.digitama_library_cards.append(card)

    def _inject_breeding_action(self, game):
        # If empty, hatch. If occupied, promote.
        player = game.turn_player
        if not player.breeding_area:
            if player.digitama_library_cards:
                player.hatch()
        else:
            # Check if we can promote (logic: standard game rules usually require level 3 or turn wait,
            # but for this runner we just alternate Hatch -> Promote to keep game moving)
            player.promote()

    def _inject_main_action(self, game) -> bool:
        # Returns True if action taken (card played), False if pass.
        player = game.turn_player

        # Simple heuristic: Play first playable card
        if player.hand_cards:
            # 80% chance to play, 20% to pass
            if random.random() < 0.8:
                card = random.choice(player.hand_cards)
                player.play_card(card)
                return True
        return False
