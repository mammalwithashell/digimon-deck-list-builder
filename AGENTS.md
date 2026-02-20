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
- State Space (S): A concatenation of three vectors:
    - Player_Deck: Vector of counts for all cards in the pool (size N).
    - Opponent_Deck: Vector of counts for the target meta deck.
    - Step_Counter: Integer representing the current iteration t.
- Action Space (A): Discrete actions representing a "Card Swap."
    - Action is a tuple (i,j) meaning "Remove card i from deck, Add card j from pool."
    - Constraint: Deck size must remain constant (D=50).
- Reward Function (R):
    - Instead of a simple sparse reward at the end, we use Cumulative Exponential Reward to amplify high win rates:
    - R=sum(exp(b * win_rate))
    - Where b=10 (Amplification Factor).
    - win_rate is determined by running a batch of simulated games (e.g., 100 matches) using the Pilot Agents.
Implementation Details
- Network: Multi-Layer Perceptron (MLP) with 1 hidden layer (1000 ReLU units).
- Exploration: epsilon-greedy strategy, annealing epsilon from 1.0 to 0.2 over training episodes.
- Library: PyTorch or Stable-Baselines3.
--------------------------------------------------------------------------------
2. The Pilot (Battle Agent)
Goal: Play Digimon TCG matches competently to provide a ground-truth "Win Rate" for the Architect.

A. Agent Types
The simulator supports swappable agent "brains" to trade off speed vs. skill.
1. Greedy Agent (Baseline)
    - Logic: Heuristic-based. Always plays the card with the highest PlayCost or highest DP reduction.
    - Speed: Extremely Fast (<1ms per move).
    - Use Case: Early training of the Architect; generating massive datasets.
2. MCTS Agent (Advanced)
    - Logic: Monte Carlo Tree Search. Simulates random playouts from the current state to find the most robust move.
    - Phases: Selection (UCB1) -> Expansion -> Simulation -> Backpropagation.
    - Speed: Slow (~1-5s per move depending on iteration count).
    - Use Case: Late-stage validation; testing against "Smart" opponents.
3. RL Pilot — MLP (MaskablePPO)
    - **Status:** Implemented.
    - Logic: Proximal Policy Optimization with action masking (MaskablePPO from sb3-contrib).
    - Policy: Standard feedforward MLP. Evaluates each turn independently with no memory of prior turns.
    - Implementation: `digimon_gym/agents/pilot_training.py`
    - Use Case: Fast, general-purpose pilot for deck evaluation matches.

    Architecture:
    ```
    Input: 981-float board tensor
      |
    FlattenExtractor (identity, 981-dim)
      |
    MLP Actor: Linear(981, 64) -> Tanh -> Linear(64, 2120) -> MaskedSoftmax
    MLP Critic: Linear(981, 64) -> Tanh -> Linear(64, 1)
    ```

    Training:
    ```bash
    python -m digimon_gym.agents.pilot_training --timesteps 500000
    python -m digimon_gym.agents.pilot_training --self-play --timesteps 1000000
    ```

4. RL Pilot — LSTM (MaskableRecurrentPPO)
    - **Status:** Implemented.
    - Logic: PPO with LSTM memory and action masking. Combines `RecurrentPPO` (LSTM hidden state threading) with `MaskablePPO` (action masking for legal moves).
    - Policy: LSTM recurrent policy. Carries hidden state across turns within a match, allowing the agent to remember previously revealed information (cards played, searched, trashed).
    - Implementation: `digimon_gym/agents/maskable_recurrent/` (custom implementation — no built-in SB3 class combines LSTM + action masking).
    - Use Case: Games with imperfect information where memory of past observations matters.

    Architecture:
    ```
    Input: 981-float board tensor
      |
    FlattenExtractor (identity, 981-dim)
      |
    LSTM (input=981, hidden=256, 1 layer) <- carries hidden state across turns
      |
    MLP Actor: Linear(256, 64) -> Tanh -> Linear(64, 2120) -> MaskedSoftmax
    MLP Critic: Linear(256, 64) -> Tanh -> Linear(64, 1)
    ```

    Key design decisions:
    - `lstm_hidden_size=256`: sufficient for card game state tracking
    - `n_lstm_layers=1`: card games don't benefit from deep LSTMs; 1 layer keeps training stable
    - `enable_critic_lstm=True`: critic needs its own LSTM to value partially-observable states
    - `net_arch=dict(pi=[64], vf=[64])`: light MLP heads after LSTM (LSTM does the heavy lifting)

    Implementation structure (`digimon_gym/agents/maskable_recurrent/`):
    - `buffers.py` — `MaskableRecurrentRolloutBuffer`: stores both LSTM hidden states and action masks with sequence-aware padding
    - `policies.py` — `MaskableRecurrentActorCriticPolicy`: LSTM forward pass + MaskableDistribution for legal action enforcement
    - `maskable_recurrent_ppo.py` — `MaskableRecurrentPPO`: rollout collection with mask + LSTM state gathering, training with masked log-prob computation

    LSTM state management during evaluation:
    ```python
    obs, info = env.reset()
    state = None                          # LSTM state: (h, c) numpy arrays
    episode_start = np.array([True])      # Zeros LSTM on first step

    while not done:
        action_masks = env.unwrapped.action_mask()
        action, state = model.predict(
            obs, state=state, episode_start=episode_start,
            deterministic=True, action_masks=action_masks,
        )
        obs, reward, terminated, truncated, info = env.step(int(action))
        episode_start = np.array([False])  # Preserve LSTM state
        done = terminated or truncated
    ```

    Training:
    ```bash
    python -m digimon_gym.agents.pilot_training --lstm --timesteps 500000
    python -m digimon_gym.agents.pilot_training --lstm --lstm-hidden-size 128 --self-play
    ```

B. State Representation (Gymnasium)
The game board is converted into a **981-float tensor** (Observation Space) for the Pilot.
See `TENSOR_SPEC.md` for the full layout.

- Global Info: [TurnCount, Phase, Memory, ...] (indices 0-9)
- Battle Area: 12 slots per player, 31 floats per slot (indices 10-753)
- Hand, Trash, Security: Lists of normalized card IDs (indices 754-903)
- Breeding Area: 1 slot per player (indices 904-965)
- Revealed Cards: List of card IDs (indices 966-975)
- Selection Context: (indices 976-980)

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

- Mask: A boolean array matching the size of the Action Space (2120).
- Instruction: The Agent must apply this mask to the logits before softmax selection.

D. Reward Shaping (Tactical Choices)
To teach tactics, we use Dense Rewards:
Rtotal = Rterminal + sum(Rtactical)

1. Terminal Reward: +1.0 (Win), -1.0 (Loss).
2. Tactical "Minties" (Intermediate Rewards):
    - Security Delta: (MySec - OppSec) * 0.01
    - Board Presence: (MyTotalDP - OppTotalDP) * 0.0001

E. WinRateCallback
The training pipeline includes a `WinRateCallback` that periodically evaluates the agent:
- Runs configurable number of evaluation episodes at set intervals
- Reports win rate, average episode length, and average reward
- Saves the best model based on win rate
- Supports both MLP and LSTM agents (threads LSTM state for recurrent models)
--------------------------------------------------------------------------------
3. MetaGauntlet — Opponent Sampling System
**Status:** Implemented (`digimon_gym/agents/gauntlet.py`)

The MetaGauntlet provides tournament meta-aware opponent deck sampling for RL training. Instead of training against a fixed opponent, the pilot agent faces a weighted distribution of meta-relevant archetypes.

A. Threat Index (TI)
Each archetype receives a Threat Index based on tournament data:
- **meta_share** — how frequently the archetype appears in tournaments (from DigiLab data)
- **conversion_rate** — how often the archetype converts to top cut placements
- Formula: `TI = alpha * meta_share + beta * conversion_rate` (when confidence threshold met)
- Below confidence threshold: `TI = meta_share` only (no conversion_rate inflation)

B. Survivorship Bias Fix
- Statistical weights (TI) are derived ONLY from DigiLab tournament log data (full field participation counts)
- Scraper-only sources (DigimonMeta, Egman Events) provide optimized decklists but NOT the statistical weights
- A confidence threshold (`confidence_min_appearances`) controls when conversion_rate factors into TI

C. Deck Pool Routing
When an archetype is sampled, the individual decklist is drawn preferentially:
1. DigimonMeta (highest priority — highly-optimized top-cut lists)
2. Egman Events (priority 2)
3. Other sources (priority 1)
4. Local file imports (priority 0)

D. GauntletWrapper
A Gymnasium wrapper that integrates MetaGauntlet into the training loop:
- On `reset()`: samples an opponent deck weighted by TI
- On terminal win: applies a bounty reward bonus proportional to the opponent's threat index
- Transparent to the agent — same observation/action interface

E. Deck Library Pipeline (`tools/meta_loader.py`)
Builds and maintains the deck library (`engine/data/deck_library.json`):
- Scrapes tournament decklists from DigimonMeta.com, Egman Events, DigimonCard.io
- Optionally enriches with DigiLab MotherDuck database for meta statistics
- Computes meta share and conversion rate from tournament placement data
- Deduplicates decklists and validates card IDs against the card registry
--------------------------------------------------------------------------------
4. Data Collection Pipeline
- Gauntlet: A collection of Meta Decks (scraped from Egman Events/DigimonMeta, weighted by Threat Index).
- Training Loop:
    1. The Architect generates a Deck Candidate.
    2. The Simulator spawns evaluation matches.
    3. Pilot A (Candidate Deck) fights Pilot B (Meta Deck sampled from Gauntlet).
    4. Win/Loss outcomes are returned to the Architect to update the Q-Network.
    5. MetaGauntlet bounty rewards amplify signal from wins against high-threat opponents.
--------------------------------------------------------------------------------
5. Instructions for AI Assistant
When implementing features, refer to this file for architectural decisions:
1. Strict Typing: Ensure all GameState objects can be serialized into Numpy arrays for the Agents.
2. Headless Priority: All game logic must run without UI dependencies. React visualizes the Log, not the real-time state.
3. Masking: Every step() function in the Python backend must return (observation, reward, done, info, action_mask).
4. LSTM Compatibility: Evaluation loops must thread LSTM state (state, episode_start) for recurrent agents.
