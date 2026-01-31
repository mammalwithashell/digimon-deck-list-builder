AGENTS.md
Project Overview
This project utilizes two distinct types of AI agents to solve the Digimon TCG Deck Optimization problem:
1. The Architect (Deck Builder): An RL agent that optimizes deck lists using the Q-DeckRec algorithm.
2. The Pilot (Battle Agent): Agents that play the actual matches to generate win-rate data, ranging from Greedy Heuristics to MCTS and PPO.
--------------------------------------------------------------------------------
1. The Architect (Deck Builder Agent)
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
3. RL Pilot (PPO)
    ◦ Logic: Proximal Policy Optimization. Trains a neural network to mimic the MCTS agent but runs instantly.
    ◦ Use Case: The final production agent for high-speed optimization.
B. State Representation (Gymnasium)
The game board is converted into a numeric vector (Observation Space) for the Pilot:
• Global Info: [TurnCount, MyMemory, OpponentMemory, MySecurityCount, OppSecurityCount]
• Battle Area: One-Hot encoded vectors for every card on the board (ID, DP, DigivolutionSources).
• Hand: One-Hot encoded vector of cards in hand.
C. Action Masking (Critical)
To prevent illegal moves (hallucinations), the environment provides an action_mask:
• Mask: A boolean array matching the size of the Action Space.
• Logic:
    ◦ If Phase == Main: Actions 0-9 (Play Card) are True only if memory is sufficient.
    ◦ If Effect == TrashCard: Actions 10-19 (Trash Index) are True only for cards existing in hand.
    ◦ Instruction: The Agent must apply this mask to the logits before softmax selection.
D. Reward Shaping (Tactical Choices)
To teach tactics (e.g., "Trash the weak card, not the Boss Monster"), we use Dense Rewards:
Rtotal​=Rterminal​+∑Rtactical​
1. Terminal Reward: +1.0 (Win), -1.0 (Loss).
2. Tactical "Minties" (Intermediate Rewards):
    ◦ Security Delta: (MySec - OppSec) * 0.5
    ◦ Board Presence: (MyTotalDP - OppTotalDP) * 0.001
    ◦ Evolution Bonus: +0.2 for successfully Digivolving (encourages building stacks).
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
