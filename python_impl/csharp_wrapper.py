import sys
import os
from typing import List
import clr_loader
from pythonnet import set_runtime

# Configure .NET Runtime
try:
    # Attempt to load CoreCLR
    rt = clr_loader.get_coreclr()
    set_runtime(rt)
except Exception as e:
    print(f"Warning: Failed to auto-detect CoreCLR: {e}")

import clr

# Path to DLL
# Assuming running from repo root
DLL_PATH = os.path.abspath("Digimon.Core/bin/Debug/net8.0/Digimon.Core.dll")

if not os.path.exists(DLL_PATH):
    raise FileNotFoundError(f"DLL not found at {DLL_PATH}. Please build the C# project.")

sys.path.append(os.path.dirname(DLL_PATH))

clr.AddReference("Digimon.Core")

from Digimon.Core import HeadlessGame
from System.Collections.Generic import List as CsList
from System import String

class CSharpGameWrapper:
    def __init__(self, deck1: List[str], deck2: List[str]):
        cs_deck1 = CsList[String]()
        for card_id in deck1:
            cs_deck1.Add(card_id)

        cs_deck2 = CsList[String]()
        for card_id in deck2:
            cs_deck2.Add(card_id)

        self.headless_game = HeadlessGame(cs_deck1, cs_deck2)

    def run_until_conclusion(self) -> int:
        return self.headless_game.RunUntilConclusion()

    def get_state_json(self) -> str:
        return self.headless_game.GameInstance.ToJson()

    def step(self, action_id: int):
        self.headless_game.Step(action_id)
