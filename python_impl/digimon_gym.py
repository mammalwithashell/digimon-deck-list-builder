import numpy as np
import logging
from typing import List, Tuple, Dict, Any, Optional
from python_impl.csharp_wrapper import CSharpGameWrapper
# Action Space Constants
# Action Space Constants
ACTION_SPACE_SIZE = 2120
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

class GameState:
    def __init__(self):
        self.game_wrapper = None
        self.max_turns = 100
        self.done = False
        self.info = {}

    def reset(self, deck1: Optional[List[str]] = None, deck2: Optional[List[str]] = None) -> Dict[str, np.ndarray]:
        # Default decks if None
        d1 = deck1 if deck1 else ["ST1-01"] * 50
        d2 = deck2 if deck2 else ["ST1-01"] * 50
        
        self.game_wrapper = CSharpGameWrapper(d1, d2, "agent", "agent")
        self.done = False
        return self.get_observation()

    def get_observation(self) -> Dict[str, np.ndarray]:
        # Use player 1 perspective (Id=1)
        obs_tensor = self.game_wrapper.get_board_tensor(1)
        
        # Return as a dictionary for compatibility if needed, using "tensor" key
        # Or if the Gym space is defined as Dict, we map the single tensor to a key.
        # But wait, looking at the previous get_observation, it returned a Dict with individual keys.
        # The PROMPT requirement said "The AI 'sees' the board via GetBoardStateTensor".
        # So we should probably expose the raw tensor.
        return {"tensor": obs_tensor}

    def get_action_mask(self) -> np.ndarray:
        if self.game_wrapper:
            # Get mask for Player 1 (Agent)
            mask_float = self.game_wrapper.get_action_mask(1)
            # Convert to bool
            return mask_float > 0.5
        
        # Fallback
        return np.ones(ACTION_SPACE_SIZE, dtype=bool)

    def step(self, action: int) -> Tuple[Dict[str, np.ndarray], float, bool, Dict[str, Any]]:
        if self.done:
            return self.get_observation(), 0.0, True, self.info

        self.game_wrapper.step(action)
        
        # We need to query C# to see if game is over. 
        # C# Wrapper doesn't expose IsGameOver directly yet without JSON.
        # But we can infer from tensor or add a helper.
        # Let's rely on JSON state for metadata or just run for now.
        
        # Actually, get_board_tensor doesn't give Winner. 
        # But let's check global data in tensor[3] (WinnerID)? 
        # Wait, GetBoardStateTensor: [151] TurnCount, [152] Phase, [153] Memory.
        # Winner is not in the tensor explicitly unless we check logic.
        
        # Let's parse JSON state for Done check for now, as it's safe.
        state_json = self.game_wrapper.get_state_json()
        import json
        state = json.loads(state_json)
        
        self.done = state["IsGameOver"]
        reward = 0.0
        if self.done:
            winner_id = state["Winner"]
            # If Winner == 1, Reward 1. If 2, Reward -1.
            if winner_id == 1:
                reward = 1.0
            elif winner_id == 2:
                reward = -1.0
        
        return self.get_observation(), reward, self.done, self.info

    def _apply_action(self, player: Any, action: int):
       pass # Legacy

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
