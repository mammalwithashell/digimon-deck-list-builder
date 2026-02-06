from csharp_wrapper import CSharpGameWrapper
import time

def test_headless_silent():
    print("\n--- Test 1: Headless Silent (Performance) ---")
    deck = ["ST1-03"] * 50
    start_time = time.time()
    
    game = CSharpGameWrapper(deck, deck, verbose_logging=False)
    
    logs = game.get_log()
    print(f"Logs captured: {len(logs)}")
    
    if len(logs) == 0:
        print("PASS: No logs captured in Silent Mode.")
    else:
        print(f"FAIL: {len(logs)} logs captured in Silent Mode!")
        for l in logs[:5]: print(f" - {l}")

def test_headless_verbose():
    print("\n--- Test 2: Headless Verbose (Debugging) ---")
    deck = ["ST1-03"] * 50
    
    game = CSharpGameWrapper(deck, deck, verbose_logging=True)
    
    logs = game.get_log()
    print(f"Logs captured: {len(logs)}")
    
    if len(logs) > 0:
        print("PASS: Logs captured in Verbose Mode.")
        print("First 3 logs:")
        for l in logs[:3]:
            print(f" - {l}")
    else:
        print("FAIL: No logs captured in Verbose Mode!")

def main():
    try:
        test_headless_silent()
        test_headless_verbose()
    except Exception as e:
        print(f"FATAL ERROR: {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    main()
