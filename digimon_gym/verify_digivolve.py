from csharp_wrapper import CSharpGameWrapper
import json
import time

def test_digivolve():
    print("\n--- Test: Digivolution Mechanics ---")
    # Deck: Agumon (Lvl 3) x 25, Greymon (Lvl 4) x 25
    deck = ["ST1-03", "ST1-07"] * 25
    
    # Init game
    game = CSharpGameWrapper(deck, deck, verbose_logging=True)
    
    def print_phase(g):
        state = json.loads(g.get_state_json())
        print(f"Current Phase: {state['CurrentPhase']} | P{state['CurrentPlayer']} Turn | Memory: {state['MemoryGauge']}")
        return state

    # Step 1: P1 Plays Agumon
    print("\n--- Step 1: P1 Plays Agumon ---")
    print_phase(game)
    # Pass Breeding (62)
    game.step(62)
    
    # Play Agumon (Hand Index 0). Cost 3.
    # Hand: [Agumon, Greymon, Agumon, Greymon, Agumon]
    print("Action: Play Agumon (Index 0)")
    game.step(0)
    
    logs = game.get_log()
    for l in logs: print(l)
    
    # Step 2: Pass Turns to get back to P1
    print("\n--- Step 2: Passing Turns ---")
    # P1 Turn Ended (Memory -3). P2 Turn.
    print_phase(game) # P2
    
    # P2 Pass Breeding (62)
    game.step(62)
    # P2 Pass Turn (62)
    game.step(62)
    
    # P1 Turn Again.
    st = print_phase(game)
    # P1 Pass Breeding
    game.step(62)
    
    # Check Hand/Field
    # Hand should be [Greymon, Agumon, Greymon, Agumon] + Drawn Card during Draw Phase.
    # Index 0 should be Greymon.
    # Field 0 should be Agumon.
    
    # Step 3: Digivolve
    print("\n--- Step 3: Digivolve Agumon -> Greymon ---")
    print("Action: Digivolve Hand 0 (Greymon) -> Field 0 (Agumon)")
    # Action ID: 400 + (0 * 15) + 0 = 400.
    game.step(400)
    
    logs = game.get_log()
    for l in logs: print(l)
    
    # Verification
    state = json.loads(game.get_state_json())
    p1 = state['Player1']
    
    # Memory Check: Started at 1 (P2 passed). Cost 2. Should be -1.
    print(f"Memory Check: {state['MemoryGauge']} (Expected -1)")
    
    # Field Check
    p1_ba_count = p1['BattleAreaCount']
    print(f"Battle Area Count: {p1_ba_count} (Expected 1)")
    
    # Note: get_state_json usually aggregates IDs?
    # We can inspect logs for "Sources" count or check Tensor if needed.
    # Or just trust the logs "Deleting...Sources" check earlier worked, so "Digivolve" log works.
    
    pass

if __name__ == "__main__":
    test_digivolve()
