import sys
import os
import numpy as np

# Ensure we can import from local
sys.path.append(os.getcwd())

from python_impl.digimon_gym import GameState, ACTION_HATCH, ACTION_PASS_TURN, ACTION_PLAY_CARD_START, ACTION_PLAY_CARD_END

def print_mask_summary(mask, label):
    print(f"\n--- Mask Summary: {label} ---")
    valid_indices = np.where(mask)[0]
    print(f"Total Valid Actions: {len(valid_indices)}")
    
    # Check specific ranges
    hatch = mask[ACTION_HATCH]
    pass_turn = mask[ACTION_PASS_TURN]
    play_cards = np.sum(mask[ACTION_PLAY_CARD_START:ACTION_PLAY_CARD_END+1])
    
    print(f"Hatch (60): {hatch}")
    print(f"Pass (62): {pass_turn}")
    print(f"Play Card Options (0-29): {play_cards}")
    
    if len(valid_indices) < 20:
        print(f"Valid Indices: {valid_indices}")

def main():
    print("Initializing GameState...")
    env = GameState()
    obs = env.reset()
    
    # 1. Initial State
    # Should be able to Hatch (if eggs exist) and Play Cards (if memory allows, usually always yes at start)
    mask_initial = env.get_action_mask()
    print_mask_summary(mask_initial, "Initial Turn 1")
    
    if not mask_initial[ACTION_HATCH]:
        print("ERROR: Hatch should be valid at start!")
    
    # 2. Perform Hatch
    print("\nAction: Hatching...")
    obs, reward, done, info = env.step(ACTION_HATCH)
    
    # 3. Post-Hatch State
    mask_post_hatch = env.get_action_mask()
    print_mask_summary(mask_post_hatch, "After Hatch")
    
    if mask_post_hatch[ACTION_HATCH]:
        print("ERROR: Hatch should be INVALID after hatching!")
        
    # Check if Move (61) is valid? 
    # Logic in Game.cs: mask[61] = 1.0f if BreedingArea[0].Level >= 3.
    # New egg is usually Level 2. So Move should be INVALID.
    if mask_post_hatch[61]:
         print("WARNING: Move is valid? Is the egg level 3?")
    else:
         print("Move (61) is correctly invalid (Egg is Level 2).")

    # 4. Pass Turn
    print("\nAction: Passing Turn...")
    env.step(ACTION_PASS_TURN)
    
    # Now it's Opponent turn? Or does Headless auto-play opponent?
    # HeadlessGame typically runs the opponent agent immediately if configured as Agent.
    # In wrapper, we initialized both as "agent".
    # HeadlessGame.Step calls ActionDecoder.
    # If it's Opponent Turn, Step might default to opponent logic or fail if we try to step for P1?
    # Wait, Gym `step` drives the "Agent" (Player 1). 
    # If we pass turn, `HeadlessGame` (if not managing P2) might verify that P1 cannot act.
    # But `HeadlessGame` logic:
    # `ExecuteAgentTurn` runs logic for Current Player.
    # If we manually `Step(action)`, we are forcing P1 action.
    # If we pass, `SwitchTurn` happens. `CurrentPlayer` becomes P2.
    # Implementation dependent: Does `Step` handle P2 automatically?
    # In `HeadlessGame.cs`: `Step(action)` calls `ActionDecoder`.
    # It acts for `GameInstance`. 
    # If P2 is "agent", does P2 have internal logic?
    # Currently `HeadlessGame` assumes *external* driver for the "Agent" (P1). P2 is usually AI.
    # But wrapper Init: `self.runner = HeadlessGame(cs_deck1, cs_deck2)`
    # This invokes default constructor which does NOT start a loop.
    # P2 AI logic is usually `RunAgentStep` or similar.
    # If we just Pass, P2 becomes active. 
    # If we try to get mask for P1, it might be all 0 or Invalid?
    
    mask_p2_turn = env.get_action_mask()
    print_mask_summary(mask_p2_turn, "After Pass (Player 2 Turn?)")
    
    # Check whose turn it is
    # wrapper.runner.GameInstance.CurrentPlayer.Id
    # (Getting this via observation tensor: index 151,152.. wait global data)
    # Tensor[154] is Memory relative to me.
    # Tensor[0] is Turn Count.
    
    print("\nVerification Complete.")

if __name__ == "__main__":
    main()
