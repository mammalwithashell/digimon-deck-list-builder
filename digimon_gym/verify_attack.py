from csharp_wrapper import CSharpGameWrapper
import time

def test_attack_mechanics():
    print("\n--- Test: Attack Mechanics ---")
    deck = ["ST1-03"] * 50 # Agumon (2000 DP)
    
    # Init game with Verbose Logging
    game = CSharpGameWrapper(deck, deck, verbose_logging=True)
    
    # 1. Setup Board State (Cheat via internal C# access not easily possible from Python without exposing more)
    # Instead, we will rely on the fact that the deck is Agumon (2000 DP).
    # We need to simulate:
    # - P1 draws and plays Agumon.
    # - P2 draws and plays Agumon.
    # - P1 Attacks P2's Agumon (Tie -> Both Delete).
    
    print("Step 1: Setup - Both players play Agumon")
    
    # helper to print phase
    def print_phase(g):
        import json
        state = json.loads(g.get_state_json())
        print(f"Current Phase: {state['CurrentPhase']} | P{state['CurrentPlayer']} Turn")

    # P1 Turn 1
    print_phase(game)
    # If Breeding, Pass (62) to go to Main
    game.step(62) # Breeding Pass
    
    print_phase(game)
    # Now in Main?
    # Play Agumon (Cost 3) -> Memory -3 (P2 Turn)
    print("Action: P1 Play Card 0")
    game.step(0) 
    
    logs = game.get_log()
    for l in logs: print(f"LOG: {l}")

    # If memory passed, it's P2 turn.
    print_phase(game)
    # P2 Breeding Pass
    game.step(62)
    print_phase(game)
    
    # P2 Play Agumon
    print("Action: P2 Play Card 0")
    game.step(0) 
    logs = game.get_log()
    for l in logs: print(f"LOG: {l}")
    
    # Check Phase - Should still be P2 Main if Memory 0
    print_phase(game)
    print("Action: P2 Pass Turn (Force)")
    game.step(62) 

    # Back to P1.
    print_phase(game)
    print("Step 2: Passing turns to clear Summoning Sickness")
    
    # P1 Breeding Pass & Main Pass
    game.step(62)
    game.step(62)
    
    # P2 Breeding Pass & Main Pass
    game.step(62)
    game.step(62)
    
    print("Step 3: P1 Attacks P2 Security")
    print_phase(game)
    # P1 Breeding Pass
    game.step(62)
    print_phase(game) # Should be P1 Main
    
    # Action: Attack Security.
    # Slot 0 Attack Security (Target 12) -> 112.
    print("Action: P1 Attack Security")
    
    # DEBUG: Check Battle Area
    import json
    st = json.loads(game.get_state_json())
    p1_ba = st['Player1']['BattleAreaCount']
    print(f"DEBUG: P1 Battle Area Count: {p1_ba}")
    
    game.step(112)
    logs = game.get_log()
    for l in logs: print(f"LOG: {l}")
    
    print("Step 4: Digimon vs Digimon (P2 attacks P1)")
    # P1 Pass Turn
    print("Action: P1 Pass Turn")
    game.step(62)
    
    # P2 Breeding Pass
    game.step(62)
    print_phase(game) # P2 Main
    
    # P2 Agumon (Slot 0) attacks P1 Agumon (Slot 0).
    # P1 Agumon is Suspended from previous attack (IsSuspended=True).
    # Target 0: 100 + (0*15) + 0 = 100.
    print("Action: P2 Attack P1 Agumon")
    game.step(100)
    
    logs = game.get_log()
    for l in logs: print(f"LOG: {l}") 
    
    # Expected: 2000 vs 2000 -> Tie -> Both Deleted.

if __name__ == "__main__":
    test_attack_mechanics()
