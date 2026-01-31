import numpy as np
import logging
import sys
import os
from typing import List, Tuple, Dict, Any, Optional

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
ACTION_ATTACK_END = 32

logger = logging.getLogger(__name__)

class DigimonEnv:
    def __init__(self, dll_path: str = "Digimon.Core.dll"):
        self.adapter = None
        self.observation_space_size = 234 # Matches C# GetBoardStateTensor size

        try:
            import clr
            # Try to load the assembly
            if os.path.exists(dll_path):
                sys.path.append(os.path.dirname(os.path.abspath(dll_path)))
                clr.AddReference(os.path.basename(dll_path).replace(".dll", ""))
            else:
                # Attempt to load from GAC or current path by name
                clr.AddReference("Digimon.Core")

            # Import the Adapter class
            # Note: This requires the C# class to be in the loaded assembly and namespace
            # If DigimonRLAdapter is in the global namespace:
            import DigimonRLAdapter
            self.adapter = DigimonRLAdapter.Instance

            if self.adapter is None:
                logger.error("DigimonRLAdapter.Instance is null. Ensure the game is running.")

        except ImportError:
            logger.error("pythonnet (clr) is not installed. Please install it to use the C# engine.")
        except Exception as e:
            logger.error(f"Failed to load C# assembly or adapter: {e}")

    def reset(self) -> np.ndarray:
        # For RL, reset usually starts a new game.
        if self.adapter:
            try:
                self.adapter.Reset()
            except Exception as e:
                logger.error(f"Error resetting game: {e}")
        return self.get_observation()

    def get_observation(self) -> np.ndarray:
        if self.adapter:
            try:
                # GetBoardStateTensor returns float[]
                state = self.adapter.GetBoardStateTensor()
                return np.array(state, dtype=np.float32)
            except Exception as e:
                logger.error(f"Error getting observation: {e}")

        # Fallback empty observation
        return np.zeros(self.observation_space_size, dtype=np.float32)

    def step(self, action: int) -> Tuple[np.ndarray, float, bool, Dict[str, Any]]:
        """
        Execute action and return (obs, reward, done, info).
        """
        reward = 0.0
        done = False
        info = {}

        if self.adapter:
            try:
                # ApplyAction returns float[2] -> [reward, done_flag]
                result = self.adapter.ApplyAction(int(action))
                reward = float(result[0])
                done = (result[1] > 0.5)
            except Exception as e:
                logger.error(f"Error applying action: {e}")
                # Penalize error?
                reward = -1.0
        else:
            logger.warning("Adapter not connected. Step ignored.")

        obs = self.get_observation()
        return obs, reward, done, info

    def get_action_mask(self) -> np.ndarray:
        """
        Optional: Implement if C# adapter supports getting valid moves.
        For now, returns all ones (or logic from Python side if we partially replicate state).
        """
        mask = np.ones(ACTION_SPACE_SIZE, dtype=bool)
        # TODO: Implement GetActionMask in C# adapter and call it here.
        return mask

# Legacy/Wrapper for code that expects GameState
class GameState(DigimonEnv):
    pass
