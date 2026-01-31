RULES_CONTEXT.md
https://world.digimoncard.com/rule/pdf/general_rules.pdf?210521
https://digimoncardgame.fandom.com/wiki/Category:Rules
1. Core Mechanics & State Management
The Memory Gauge (Resource System)
Reference: Official Rulebook & Wiki
• Logic: Memory is a shared gauge ranging from -10 to 10.
• Costs: Paying a cost moves the counter toward the opponent's side.
• Turn End Condition: The turn attempts to end immediately when the memory counter crosses zero to the opponent's side (e.g., becomes 1 or greater for the opponent).
• The "End of Turn" Interrupt:
    1. If Memory > 0 for Opponent: Check for pending effects.
    2. Resolve all pending effects (including [End of Turn] triggers).
    3. Check Memory Again: If an effect gained memory and returned the gauge to 0 or the player's side, the turn continues.
    4. If Memory is still on opponent's side and stack is empty: Pass turn.
The Zones (State Representation)
Reference: Play Guide To be represented in GameState (NumPy Arrays):
1. Deck: Ordered list.
2. Hand: Private to player (Agent observes own, masks opponent's).
3. Trash: Public ordered list.
4. Security Stack: Face-down pile.
    ◦ Logic: Cards are removed from Top (Index 0) to Bottom.
    ◦ Zero Security Rule: If a player has 0 security cards and takes a successful direct attack, they lose.
5. Breeding Area:
    ◦ Constraint: Digimon here cannot activate effects, cannot be targeted, and cannot attack.
    ◦ Hatching: Can only hatch if the area is empty.
    ◦ Moving: Can move to Battle Area only if Level 3 or higher.
6. Battle Area: Where Digimon/Tamers reside.

--------------------------------------------------------------------------------
2. Phase Structure (Step Function Logic)
Reference: End of Turn Procedures Wiki The step() function must respect this sequence.
1. Unsuspend Phase:
    ◦ Action: All suspended (tapped) cards become unsuspended.
    ◦ Keyword Exception: Digimon with <Reboot> unsuspend during the opponent's unsuspend phase.
2. Draw Phase:
    ◦ Action: Player draws 1 card (First player skips this on Turn 1).
3. Breeding Phase:
    ◦ Exclusive Choice: The player may do ONE of the following:
        ▪ Hatch a Digitama (Egg).
        ▪ Move a Digimon from Breeding to Battle Area.
        ▪ Do nothing.
4. Main Phase:
    ◦ Free action loop (Play, Evolve, Attack).
    ◦ Ends when Memory crosses to opponent (and settles).

--------------------------------------------------------------------------------
3. Combat Logic & Resolution
Reference: Wiki Attack Resolution
Attack Flow
1. Declaration: Digimon suspends (taps) to attack. Target is Player or Suspended Digimon.
    ◦ Action Masking: Cannot attack if played this turn (Summoning Sickness), unless it has <Rush>.
2. Counter Timing: Opponent may use [Counter] effects or <Blocker>.
3. Resolution:
    ◦ Vs Digimon: Compare DP. Lower DP is deleted. Tie = Both deleted.
    ◦ Vs Security:
        1. Reveal top Security card.
        2. If Option: Activate [Security] Effect.
        3. If Digimon: Battle (Compare DP).
            • Note: Security Digimon are never treated as "Deleted" effects. They are just trashed after battle.
            • Note: Attacker does NOT die if it has <Jamming>.

--------------------------------------------------------------------------------
4. Keyword Implementation Guide
Reference: Wiki Keywords Jules must implement these specific flags in the Card logic.
Keyword
	
Logic Implementation
	
Interrupt: When Opponent declares attack, if this unit is Unsuspended, inject a decision node to Suspend and redirect target to self.
	
State Update: If Attacker wins battle and Defender is deleted, immediately queue a Security Check action.
	
Battle Logic: If Attacker loses battle against Security Digimon, do NOT mark Attacker for deletion.
<Recovery +X>
	
Action: Move top X cards from Deck to top of Security Stack.
<Security Attack +X>
	
Loop: Perform X additional checks. Critical: Checks happen one by one. If unit dies on check 1, checks 2-X are cancelled.
	
Action: Trash top X cards of target stack. Constraint: Cannot reduce below Level 3 (no trashing the base card if it's Lv3).
	
Trigger: If this unit dies in battle, mark the winner of that battle for deletion.
	
Interrupt: If would be deleted, Trash top card of stack instead. Unit survives (as the lower form).

--------------------------------------------------------------------------------
5. Edge Cases for Agent Training
1. Global Effects: Effects like "All Digimon get +1000 DP" must be recalculated dynamically on every step(), or cached and flagged dirty on board state changes.
2. Empty Deck Loss: If a player must draw but cannot, they lose immediately.
3. Simultaneous Effects:
    ◦ If multiple effects trigger at once (e.g., On Deletion), the Turn Player effects resolve first.
    ◦ Implementation: The Game class needs a ResolutionStack. When multiple triggers happen, push Turn Player effects, then Opponent effects. Pop from stack to resolve.
6. Setup & Game Flow Nuances
The Mulligan Rule (April 2023 Update)
Reference: News: Mulligan is effective To ensure the Agent doesn't learn to play with "brick" hands, implement the official Mulligan step:
1. Draw 5: Agent receives initial hand.
2. Decision Point: Agent evaluates hand quality.
3. Action: Can choose to return all 5 cards to deck, shuffle, and redraw 5 cards.
    ◦ Constraint: This can only be done once per game.
    ◦ Constraint: Must be done before setting up the Security Stack.
Security Digimon Properties (Critical Edge Case)
Reference: Wiki: Security Jules must distinguish between a "Digimon Card" and a "Digimon Instance."
• Not a Unit: A Digimon that appears from Security is NOT treated as a "Digimon" for card effects.
    ◦ Example: If a card says "Delete 1 Digimon", it cannot target a Security Digimon.
    ◦ Example: When a Security Digimon loses a battle, it does NOT trigger [On Deletion] effects. It is simply placed in the Trash.
    ◦ Battle Logic: Security Digimon do not take damage. They compare DP, resolve the battle, and vanish.
Option Card Color Requirements
Reference: Wiki: Security
• Hand/Main Phase: To play an Option card, you must have a Digimon or Tamer of that color in play (or in the breeding area).
• Security Stack: If an Option card triggers from Security, its Color Requirement is ignored. The effect activates regardless of board state.

--------------------------------------------------------------------------------
7. The "End of Turn" State Machine
Reference: Wiki: End of Turn Procedures The transition from Player A to Player B is not instant. It is a loop that the Agent can manipulate.
Logic Loop:
1. Trigger: Memory Gauge >0 on Opponent's side.
2. Check Pending: Are there unresolved effects? (e.g., [End of Turn] effects).
3. Resolve: Execute pending effects.
4. Re-Evaluate:
    ◦ If an effect gained memory and returned the gauge to ≤0 on the Player's side: The Turn Resumes. The player enters MainPhase again.
    ◦ If Memory is still >0 for Opponent: Proceed to pass turn.
Why this matters for RL: An advanced Agent (like Q-DeckRec) might learn to overspend memory to trigger an [End of Turn] effect that gains memory back, effectively extending their turn for "free" moves.

--------------------------------------------------------------------------------
8. Advanced Keywords & Interrupts
Reference: Wiki: Keywords Modern Digimon decks (Ace/Blast) rely on interaction during the opponent's turn.
The "Counter" Timing (Blast Digivolve)
• Trigger: Opponent declares an attack.
• Window: Before the attack hits, the non-turn player has a [Counter] window.
• Blast Digivolve: A Digimon in hand can digivolve onto a specific target for free (usually causing "Overflow" later). This changes the DP of the target mid-attack, potentially causing the attacker to suicide.
Raid (Switch Target)
• Logic: When attacking, this Digimon can switch the target to an unsuspended Digimon with the highest DP.
• Implementation: This overrides the standard "can only attack suspended Digimon" rule.
Partition (Recovery)
• Logic: When this Digimon is removed (Deleted/Bounced), it splits back into the Level X and Level Y cards that were in its sources.
• State Update: One object leaves the board; two objects enter the board.

--------------------------------------------------------------------------------
9. Data Structure Recommendations for Jules
To support the DeckGym/Q-DeckRec architecture efficiently:
1. Card ID Enum: Use the official Set ID (e.g., ST1-01) as the primary key.
2. Effect Queuing: Implement a Stack for effects.
    ◦ Rule: Turn player effects resolve first.
    ◦ Rule: If an effect triggers another effect (e.g., On Play triggers a Draw), the new effect goes to the top of the stack (LIFO).
3. Memory Cap: Hard clamp Memory between -10 and 10. Any gain beyond 10 is lost
