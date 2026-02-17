AGENTS.md
Refer to RULES_CONTEXT.md for rule implementation


Project Overview
This project utilizes two distinct types of AI agents to solve the Digimon TCG Deck Optimization problem:
1. The Architect (Deck Builder): An RL agent that optimizes deck lists using the Q-DeckRec algorithm.
2. The Pilot (Battle Agent): Agents that play the actual matches to generate win-rate data, ranging from Greedy Heuristics to MCTS and PPO.
--------------------------------------------------------------------------------
1. The Architect (Deck Builder Agent)
**Status:** Specced, not yet implemented.

Algorithm: Deep Q-Network (DQN) / Q-DeckRec Implementation. Goal: Maximize the cumulative exponential win rate of a deck against a specific meta-opponent.
Markov Decision Process (MDP) Definition
• State Space (S): A concatenation of three vectors:
    ◦ Player_Deck: Vector of counts for all cards in the pool (size N).
    ◦ Opponent_Deck: Vector of counts for the target meta deck.
    ◦ Step_Counter: Integer representing the current iteration t.
• Action Space (A): Discrete actions representing a "Card Swap."
    ◦ Action is a tuple (i,j) meaning "Remove card i from deck, Add card j from pool."
    ◦ Constraint: Deck size must remain constant (D=50).
• Reward Function (R):
    ◦ Instead of a simple sparse reward at the end, we use Cumulative Exponential Reward to amplify high win rates:
    ◦ R=∑exp(b⋅win_rate)
    ◦ Where b=10 (Amplification Factor).
    ◦ win_rate is determined by running a batch of simulated games (e.g., 100 matches) using the Pilot Agents.
Implementation Details
• Network: Multi-Layer Perceptron (MLP) with 1 hidden layer (1000 ReLU units).
• Exploration: ϵ-greedy strategy, annealing ϵ from 1.0 to 0.2 over training episodes.
• Library: PyTorch or Stable-Baselines3.
--------------------------------------------------------------------------------
2. The Pilot (Battle Agent)
Goal: Play Digimon TCG matches competently to provide a ground-truth "Win Rate" for the Architect.

A. Agent Types
The simulator supports swappable agent "brains" to trade off speed vs. skill.
1. Greedy Agent (Baseline)
    ◦ Logic: Heuristic-based. Always plays the card with the highest PlayCost or highest DP reduction.
    ◦ Speed: Extremely Fast (<1ms per move).
    ◦ Use Case: Early training of the Architect; generating massive datasets.
2. MCTS Agent (Advanced)
    ◦ Logic: Monte Carlo Tree Search. Simulates random playouts from the current state to find the most robust move.
    ◦ Phases: Selection (UCB1) -> Expansion -> Simulation -> Backpropagation.
    ◦ Speed: Slow (~1-5s per move depending on iteration count).
    ◦ Use Case: Late-stage validation; testing against "Smart" opponents.
3. RL Pilot (MaskablePPO)
    ◦ Logic: Proximal Policy Optimization with action masking (MaskablePPO from sb3-contrib).
    ◦ Implementation: `digimon_gym/agents/pilot_training.py`
    ◦ Use Case: The final production agent for high-speed optimization.

B. State Representation (Gymnasium)
The game board is converted into a **981-float tensor** (Observation Space) for the Pilot.
See `TENSOR_SPEC.md` for the full layout.

• Global Info: [TurnCount, Phase, Memory, ...] (indices 0-9)
• Battle Area: 12 slots per player, 31 floats per slot (indices 10-753)
• Hand, Trash, Security: Lists of normalized card IDs (indices 754-903)
• Breeding Area: 1 slot per player (indices 904-965)
• Revealed Cards: List of card IDs (indices 966-975)
• Selection Context: (indices 976-980)

C. Action Space & Masking
To prevent illegal moves (hallucinations), the environment provides an action_mask.
The action space consists of **2120 discrete actions**:

| Range | Action |
|-------|--------|
| 0-29 | Play card from hand (index) |
| 30-59 | Trash card from hand (index) |
| 60 | Hatch from egg deck |
| 61 | Move from breeding area |
| 62 | Pass turn / breeding pass / decline optional |
| 63-92 | DNA Digivolve (hand index) |
| 100-399 | Attack with permanent (slot x target) |
| 400-999 | Digivolve (hand x field) |
| 1000-1999 | Effect activation (source x effectIdx) |
| 2000-2119 | Source selection (field x sourceIdx) |

• Mask: A boolean array matching the size of the Action Space (2120).
• Instruction: The Agent must apply this mask to the logits before softmax selection.

D. Reward Shaping (Tactical Choices)
To teach tactics, we use Dense Rewards:
Rtotal = Rterminal + ∑Rtactical

1. Terminal Reward: +1.0 (Win), -1.0 (Loss).
2. Tactical "Minties" (Intermediate Rewards):
    ◦ Security Delta: (MySec - OppSec) * 0.01
    ◦ Board Presence: (MyTotalDP - OppTotalDP) * 0.0001
--------------------------------------------------------------------------------
3. Data Collection Pipeline
• Gauntlet: A collection of Meta Decks (scraped from Egman Events/DigimonMeta).
• Training Loop:
    1. The Architect generates a Deck Candidate.
    2. The Simulator spawns 100 threads.
    3. Pilot A (Candidate Deck) fights Pilot B (Random Meta Deck).
    4. Win/Loss outcomes are returned to the Architect to update the Q-Network.
--------------------------------------------------------------------------------
4. Instructions for AI Assistant (Jules)
When implementing features, refer to this file for architectural decisions:
1. Strict Typing: Ensure all GameState objects can be serialized into Numpy arrays for the Agents.
2. Headless Priority: All game logic must run without UI dependencies. React visualizes the Log, not the real-time state.
3. Masking: Every step() function in the Python backend must return (observation, reward, done, info, action_mask).
