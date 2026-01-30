# Digimon Deck List Optimizer

## Project Overview
This project is a **Digimon Trading Card Game (TCG) Deck List Optimizer**. Its primary goal is to discover optimal deck lists by running thousands of simultaneous, accelerated simulations.

The system utilizes an **Evolutionary Algorithm (Genetic Algorithm)** to iteratively improve deck construction. To evaluate a deck's strength, it plays matches against known "Meta Decks" using a **Monte Carlo Tree Search (MCTS) Agent** or **Reinforcement Learning (RL) Agents** to make optimal gameplay decisions.

While the original logic references the open-source [DCGO Unity application](https://github.com/DCGO2/DCGO-Card-Scripts), this project implements the core engine in **Python** for performance and flexibility in headless simulations, with a **React** frontend for visual observation ("Headed" mode). This approach aligns with a **"Q-DeckRec"** style architecture, treating deck building as a Markov Decision Process (MDP) and leveraging a high-performance **"DeckGym"** simulator.

## Architecture

### 1. Python Game Engine (Core)
A full port of the Digimon TCG rules engine from C# to Python (`digimon_gym.py`).
*   **Purpose:** Runs the actual game logic (rules, card effects, phases).
*   **Design:** Optimized for speed, headless execution, and Machine Learning integration.
*   **State Representation:** Uses a `GameState` class with **numpy arrays** to efficiently represent the board (Security Stack, Battle Area, Hand, Trash) for ML input.
*   **Card Database:** A JSON-based structure for card effects (OnPlay, WhenDigivolving) that the engine parses dynamically, organizing scripts by set (similar to DCGO).
*   **Location:** `python_impl/`

### 2. Headless Simulation Layer
A simulation runner that executes matches without rendering graphics.
*   **Purpose:** Training and Optimization.
*   **Features:**
    *   Runs in "Batchmode" (pure CPU).
    *   Strips out UI/Audio delays.
    *   **Configurable Simulation Count:** Allows users to define exactly how many games are run per evaluation cycle.
    *   **Custom Matchups:** Users can configure specific "Meta Decks" (sourced from Egman Events/Digimon Meta) for the subject deck to play against.
    *   **FastAPI Endpoint:** Accepts deck strings, runs background simulations (e.g., 100 matches), and returns win rates and game logs.

### 3. Agent Layer (MCTS & RL)
AI agents responsible for playing the decks during simulation.
*   **Action Space:** A `step(action)` function that accepts integer-based actions (e.g., 0-100 representing card IDs/moves), applies them to the state, and returns `(next_state, reward, done)`.
*   **Approach A: Monte Carlo Tree Search (MCTS)**
    *   Standard phases: Selection (UCB1), Expansion, Simulation, Backpropagation.
    *   Provides a strong baseline opponent.
*   **Approach B: Reinforcement Learning (RL)**
    *   Agents trained to optimize decision-making over time (Q-Learning/Deep Q-Networks).
    *   Allows for "Agent vs. Agent" training scenarios.
*   **Role:** Provides a stronger, more consistent opponent than random/rule-based bots to ensure deck quality is evaluated fairly.

### 4. Optimization Layer (Genetic Algorithm)
The "Manager" that evolves the deck lists.
*   **Input Configuration:**
    *   **Core List:** A set of mandatory cards.
    *   **Starting Deck:** A full valid decklist (50 cards + 0-5 Digi-Eggs) to begin optimization from.
    *   **Side Deck:** A pool of allowed cards to swap in/out.
*   **Process:** Generates populations of decks, evaluates them via the Simulation Layer, and breeds/mutates the best performers.
*   **Output:** An optimized deck list, highlighting the alterations (additions/removals) made by the agents compared to the starting list.

### 5. Frontend (React)
A web-based interface for "Headed" games.
*   **Purpose:** Debugging, Observation, and Verification.
*   **Features:**
    *   Visualizes the game state (Board, Hand, Trash, Security).
    *   Connects to the Python Engine via **FastAPI**.
    *   **Agent vs. Agent Mode:** Observe two AI agents playing against each other in real-time.
    *   Allows human-vs-bot observation.

## Tech Stack
*   **Engine & Optimization:** Python 3.x, NumPy, FastAPI
*   **Frontend:** React, Node.js
*   **Reference Logic:** C# (DCGO)

## Roadmap

### Phase 1: Python Engine PoC
*   [x] Establish Core Data Structures (`Entity`, `CardSource`, `Player`).
*   [ ] Implement `GameState` class with NumPy array support.
*   [ ] Create JSON-based Card Database and dynamic parser.
*   [ ] Implement `step(action)` logic for ML integration.
*   [ ] Implement Game Loop (Phases, Turn Structure).

### Phase 2: Agent Integration (MCTS & RL)
*   [ ] Implement MCTS Node and Tree structure.
*   [ ] Integrate Agent with Game Controller using `step()` interface.
*   [ ] Investigate and prototype Reinforcement Learning agents (Q-DeckRec style).
*   [ ] Benchmark Agent performance.

### Phase 3: Deck Optimization Layer
*   [ ] Create Genetic Algorithm wrapper.
*   [ ] Implement logic to handle Core Lists, Full Decks, and Side Decks as inputs.
*   [ ] Define "Meta Decks" for evaluation (sourcing from Digimon Meta).
*   [ ] Implement result parsing and alteration tracking.

### Phase 4: React Frontend Integration
*   [ ] Initialize React project.
*   [ ] Create Game Board UI.
*   [ ] Build FastAPI endpoint for deck simulation.
*   [ ] Connect Frontend to Python Engine.
*   [ ] Implement "Agent vs. Agent" visual mode.

## Setup & Usage

### Prerequisites
*   Python 3.10+
*   Node.js & npm (for Frontend)

### Running Tests
```bash
python python_impl/test_structure.py
```

## References
*   **Card Logic:** [DCGO-Card-Scripts](https://github.com/DCGO2/DCGO-Card-Scripts) (C# Source)
*   **Manual Simulator UI:** [Digimon TCG Simulator](https://github.com/WE-Kaito/digimon-tcg-simulator) (UI Reference)
