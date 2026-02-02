import sys
import os
from typing import List
import clr_loader
from pythonnet import set_runtime
import numpy as np

# Configure .NET Runtime
try:
    # Attempt to load CoreCLR
    rt = clr_loader.get_coreclr()
    set_runtime(rt)
except Exception as e:
    print(f"Warning: Failed to auto-detect CoreCLR: {e}")

import clr

# Path to DLL
# Allow override via environment variable, default to Debug build
default_path = "Digimon.Core/bin/Debug/net8.0/Digimon.Core.dll"
DLL_PATH = os.environ.get("DIGIMON_CORE_DLL_PATH", os.path.abspath(default_path))

if not os.path.exists(DLL_PATH):
    raise FileNotFoundError(f"DLL not found at {DLL_PATH}. Please build the C# project or set DIGIMON_CORE_DLL_PATH.")

sys.path.append(os.path.dirname(DLL_PATH))

clr.AddReference("Digimon.Core")

from Digimon.Core import HeadlessGame, InteractiveGame, PlayerType, CardRegistry
from System.Collections.Generic import List as CsList
from System import String

class CSharpGameWrapper:
    def __init__(self, deck1: List[str], deck2: List[str], player1_type: str = "agent", player2_type: str = "agent"):
        cs_deck1 = CsList[String]()
        for card_id in deck1:
            cs_deck1.Add(card_id)

        cs_deck2 = CsList[String]()
        for card_id in deck2:
            cs_deck2.Add(card_id)

        # Initialize CardRegistry
        cards_json_path = os.path.join(os.path.dirname(__file__), "engine", "data", "cards.json")
        if os.path.exists(cards_json_path):
             CardRegistry.Initialize(cards_json_path)
        else:
             print(f"Warning: cards.json not found at {cards_json_path}. CardRegistry not initialized.")

        self.is_interactive = False
        p1_human = player1_type.lower() == "human"
        p2_human = player2_type.lower() == "human"
        
        if not p1_human and not p2_human:
             self.runner = HeadlessGame(cs_deck1, cs_deck2)
             self.is_interactive = False
        else:
             t1 = PlayerType.Human if p1_human else PlayerType.Agent
             t2 = PlayerType.Human if p2_human else PlayerType.Agent
             self.runner = InteractiveGame(cs_deck1, cs_deck2, t1, t2)
             self.is_interactive = True

    def run_until_conclusion(self) -> int:
        if self.is_interactive:
            # Interactive games usually require external input and don't run in a tight loop
            raise NotImplementedError("run_until_conclusion is not supported for Interactive games.")
        return self.runner.RunUntilConclusion(200)

    def get_state_json(self) -> str:
        return self.runner.GameInstance.ToJson()

    def step(self, action_id: int):
        if self.is_interactive:
            self.runner.Step(action_id)
        else:
            # For Headless, Step is now supported via stub
            self.runner.Step(action_id)

    def run_step(self) -> str:
        """Runs a single step of the game loop. Valid helpers for Interactive mode."""
        if self.is_interactive:
            return self.runner.RunStep()
        else:
            self.runner.RunAgentStep()
            return self.runner.GameInstance.ToJson()

    def get_board_tensor(self, player_id: int) -> np.ndarray:
        # Get float array from C#
        float_arr = self.runner.GameInstance.GetBoardStateTensor(player_id)
        # Convert System.Single[] to numpy array
        return np.array(float_arr, dtype=np.float32)

    def get_action_mask(self, player_id: int) -> np.ndarray:
        # Get float array (0.0/1.0) from C#
        float_arr = self.runner.GameInstance.GetActionMask(player_id)
        # Convert to numpy boolean array (or keep as float)
        # Using float for now as it's often used as mask multiplier
        # But Gym often prefers bool for explicit masking API.
        # Let's return float to be safe with C# return type.
        return np.array(float_arr, dtype=np.float32)
