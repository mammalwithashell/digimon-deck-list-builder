import numpy as np
import logging
from typing import List, Tuple, Dict, Any, Optional
from python_impl.engine.game import Game
from python_impl.engine.data.enums import GamePhase, PendingAction

# Action Space Constants
ACTION_SPACE_SIZE = 50
ACTION_PLAY_CARD_START = 0
ACTION_PLAY_CARD_END = 9
ACTION_TRASH_CARD_START = 10
ACTION_TRASH_CARD_END = 19
ACTION_HATCH = 20
ACTION_UNSUSPEND = 21
ACTION_PASS_TURN = 22
ACTION_ATTACK_START = 23
ACTION_ATTACK_END = 32 # Attack Security with Permanent 0-9
ACTION_MOVE = 33
# ... reserve others

logger = logging.getLogger(__name__)

class GameState:
    def __init__(self):
        self.game = Game()
        self.max_turns = 100
        self.done = False
        self.info = {}

    def reset(self, deck1: Optional[List[str]] = None, deck2: Optional[List[str]] = None) -> Dict[str, np.ndarray]:
        self.game = Game()

        from python_impl.engine.data.card_database import CardDatabase
        db = CardDatabase()

        # Populate logic
        def load_deck(player, deck_ids):
            if not deck_ids:
                # Load default random deck if none provided
                all_ids = list(db.get_all_cards().keys())
                if not all_ids:
                    return
                # Create a mixed deck
                deck_ids = (all_ids * 50)[:50]

            for cid in deck_ids:
                if not cid: continue
                cs = db.create_card_source(cid.strip(), player)
                if cs:
                    if cs.is_digi_egg:
                        player.digitama_library_cards.append(cs)
                    else:
                        player.library_cards.append(cs)

        load_deck(self.game.player1, deck1)
        load_deck(self.game.player2, deck2)

        self.game.start_game()
        self.done = False
        return self.get_observation()

    def get_observation(self) -> Dict[str, np.ndarray]:
        """
        Returns a dictionary of numpy arrays representing the board state.
        """
        p1 = self.game.player1
        p2 = self.game.player2

        # Helper to pad/trunc
        def pad(arr, size):
            res = np.zeros(size, dtype=np.int32)
            # For now, just putting 1s to indicate presence if real IDs aren't available
            # Or use hash of card name?
            # Ideally we use card IDs.
            for i, card in enumerate(arr[:size]):
                res[i] = 1 # Placeholder ID
            return res

        obs = {
            "p1_hand": pad(p1.hand_cards, 10),
            "p1_battle_area": pad(p1.battle_area, 10),
            "p1_security": pad(p1.security_cards, 5),
            "p1_trash": pad(p1.trash_cards, 30),

            "p2_hand": pad(p2.hand_cards, 10),
            "p2_battle_area": pad(p2.battle_area, 10),
            "p2_security": pad(p2.security_cards, 5),
            "p2_trash": pad(p2.trash_cards, 30),

            "global_info": np.array([
                self.game.turn_count,
                self.game.turn_player.player_id,
                self.game.memory,
                self.game.current_phase.value,
                self.game.pending_action.value
            ], dtype=np.int32)
        }
        return obs

    def get_action_mask(self) -> np.ndarray:
        mask = np.zeros(ACTION_SPACE_SIZE, dtype=bool)

        player = self.game.turn_player
        hand_size = len(player.hand_cards)

        # Scenario 1: Game is waiting for a TRASH decision (e.g. from an effect)
        if self.game.pending_action == PendingAction.TRASH_CARD:
            # If there are no cards to trash, allow PASS_TURN as a safe fallback
            # so policies don't get stuck repeatedly selecting invalid actions.
            if hand_size == 0:
                mask[ACTION_PASS_TURN] = True
                return mask

            # Enable Trash actions for existing cards
            # Clamp to max 10
            limit = min(hand_size, 10)
            for i in range(limit):
                mask[ACTION_TRASH_CARD_START + i] = True
            return mask

        # Scenario 2: Main Phase (Standard Turn Actions)
        if self.game.current_phase == GamePhase.Main:
            # Play Cards (0-9)
            limit = min(hand_size, 10)
            for i in range(limit):
                # In real game, check memory cost, prerequisites.
                # For now, allow playing any card in hand.
                mask[ACTION_PLAY_CARD_START + i] = True

            # Hatch (20)
            if player.breeding_area is None and len(player.digitama_library_cards) > 0:
                 mask[ACTION_HATCH] = True

            # Pass Turn (22)
            mask[ACTION_PASS_TURN] = True

            # Attack (23-32)
            # Check unsuspended digimon in battle area
            for i, perm in enumerate(player.battle_area):
                if i >= 10: break
                if not perm.is_suspended: # and can attack checks
                    mask[ACTION_ATTACK_START + i] = True

        # Scenario 3: Breeding Phase
        if self.game.current_phase == GamePhase.Breeding:
            # Hatch (20)
            if player.breeding_area is None and len(player.digitama_library_cards) > 0:
                 mask[ACTION_HATCH] = True

            # Move (33)
            if player.breeding_area is not None:
                 if player.breeding_area.level >= 3:
                     mask[ACTION_MOVE] = True

            # Pass Turn (22) - to skip breeding actions and go to Main
            mask[ACTION_PASS_TURN] = True

        return mask

    def step(self, action: int) -> Tuple[Dict[str, np.ndarray], float, bool, Dict[str, Any]]:
        """
        Execute action and return (obs, reward, done, info).
        """
        if self.done:
            return self.get_observation(), 0.0, True, self.info

        # Bounds check
        if action < 0 or action >= ACTION_SPACE_SIZE:
             return self.get_observation(), -1.0, self.done, {"error": "Action Out of Bounds"}

        player = self.game.turn_player
        mask = self.get_action_mask()

        if not mask[action]:
            # Invalid action!
            return self.get_observation(), -1.0, self.done, {"error": "Invalid Action"}

        # Execute Action
        self._apply_action(player, action)

        # Post-action game logic
        # Check win condition (e.g. deck out, security 0 + direct attack)
        # Assuming Game updates game_over and winner.

        # If pending action was resolved, clear it?
        # Note: _apply_action should handle clearing if it satisfied the request.

        # Advance phase if needed (e.g. if we just Passed Turn, or if Play Card effects are done)
        # Note: In a real engine, Play Card might trigger effects -> setting pending_action.
        # Here we assume synchronous simple execution.

        # If the turn was passed, the game.turn_player is now the OTHER player.

        reward = 0.0
        if self.game.game_over:
            self.done = True
            if self.game.winner == player:
                reward = 1.0
            else:
                reward = -1.0

        return self.get_observation(), reward, self.done, self.info

    def _apply_action(self, player: Any, action: int):
        # 1. TRASH CARD
        if ACTION_TRASH_CARD_START <= action <= ACTION_TRASH_CARD_END:
            idx = action - ACTION_TRASH_CARD_START
            if idx < len(player.hand_cards):
                card = player.hand_cards.pop(idx)
                player.trash_cards.append(card)
                logger.info(f"{player.player_name} trashed {card.card_names[0]}.")

            # If we were waiting for this, clear the flag
            if self.game.pending_action == PendingAction.TRASH_CARD:
                self.game.pending_action = PendingAction.NO_ACTION

        # 2. PLAY CARD
        elif ACTION_PLAY_CARD_START <= action <= ACTION_PLAY_CARD_END:
            idx = action - ACTION_PLAY_CARD_START
            if idx < len(player.hand_cards):
                card = player.hand_cards[idx] # don't pop here, play_card does it
                player.play_card(card)

                # Check for "On Play" effects?
                # For PoC, let's pretend every Option card requires a Trash.
                # This tests the PendingAction logic.
                if card.is_option:
                     self.game.pending_action = PendingAction.TRASH_CARD
                     logger.info("Effect triggered: Trash 1 card from hand.")

        # 3. HATCH
        elif action == ACTION_HATCH:
            player.hatch()

        # 3.5 MOVE
        elif action == ACTION_MOVE:
            player.move_from_breeding()

        # 4. PASS TURN
        elif action == ACTION_PASS_TURN:
            # Special case: If waiting for trash but hand is empty, we allow passing to escape loop
            if self.game.pending_action == PendingAction.TRASH_CARD:
                self.game.pending_action = PendingAction.NO_ACTION
            else:
                self.game.pass_turn()

        # 5. ATTACK (Placeholder)
        elif ACTION_ATTACK_START <= action <= ACTION_ATTACK_END:
            idx = action - ACTION_ATTACK_START
            if idx < len(player.battle_area):
                perm = player.battle_area[idx]
                # Perform attack logic...
                perm.is_suspended = True
                logger.info(f"{player.player_name}'s Digimon attacked!")

def greedy_policy(env: GameState) -> int:
    """
    Selects an action based on a simple heuristic.
    """
    mask = env.get_action_mask()
    valid_actions = np.where(mask)[0]

    if len(valid_actions) == 0:
        return ACTION_PASS_TURN

    player = env.game.turn_player

    # Helper to score a card
    def get_card_score(card):
        if card.is_digimon:
            # Higher level = better to keep usually? Or higher cost?
            # Prompt says: Level 6 = 10 pts.
            # Assuming card has level attribute. CEntity_Base has level?
            # card.c_entity_base.level ideally.
            return 10 # Placeholder
        elif card.is_option:
            return 5
        return 1

    # 1. TRASH DECISION
    if env.game.pending_action == PendingAction.TRASH_CARD:
        # Trash the one with LOWEST score.
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

    # Move?
    if ACTION_MOVE in valid_actions:
        return ACTION_MOVE

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
