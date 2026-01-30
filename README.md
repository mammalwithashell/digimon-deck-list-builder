# Digimon Deck List Optimizer

## Project Overview
This project is a **Digimon Trading Card Game (TCG) Deck List Optimizer**. Its primary goal is to discover optimal deck lists by running thousands of simultaneous, accelerated simulations.

The system utilizes an **Evolutionary Algorithm (Genetic Algorithm)** to iteratively improve deck construction. To evaluate a deck's strength, it plays matches against known "Meta Decks" using a **Monte Carlo Tree Search (MCTS) Agent** to make optimal gameplay decisions.

While the original logic references the open-source [DCGO Unity application](https://github.com/DCGO2/DCGO-Card-Scripts), this project implements the core engine in **Python** for performance and flexibility in headless simulations, with a **React** frontend for visual observation ("Headed" mode).

## Architecture

### 1. Python Game Engine (Core)
A full port of the Digimon TCG rules engine from C# to Python.
*   **Purpose:** Runs the actual game logic (rules, card effects, phases).
*   **Design:** Optimized for speed and headless execution. It mirrors the data structures of DCGO but resolves circular dependencies and logic for a Pythonic environment.
*   **Location:** `python_impl/`

### 2. Headless Simulation Layer
A simulation runner that executes matches without rendering graphics.
*   **Purpose:** Training and Optimization.
*   **Features:**
    *   Runs in "Batchmode" (pure CPU).
    *   Strips out UI/Audio delays.
    *   Accepts deck lists via command line or API.
    *   Outputs match results (Win/Loss, Turn Count) to JSON/StdOut.

### 3. Agent Layer (MCTS)
An AI agent responsible for playing the decks during simulation.
*   **Approach:** Monte Carlo Tree Search (MCTS).
*   **Steps:**
    *   **Selection:** Uses UCB1 to select promising moves.
    *   **Expansion:** Adds new game states to the tree.
    *   **Simulation:** Random rollouts to estimate win probability.
    *   **Backpropagation:** Updates nodes with simulation results.
*   **Role:** Provides a stronger, more consistent opponent than random/rule-based bots to ensure deck quality is evaluated fairly.

### 4. Optimization Layer (Genetic Algorithm)
The "Manager" that evolves the deck lists.
*   **Input:** Core Cards (Required) + Card Pool.
*   **Process:** Generates populations of decks, evaluates them via the Simulation Layer, and breeds/mutates the best performers.
*   **Output:** Optimized Deck List.

### 5. Frontend (React)
A web-based interface for "Headed" games.
*   **Purpose:** Debugging and Observation.
*   **Features:**
    *   Visualizes the game state (Board, Hand, Trash, Security).
    *   Connects to the Python Engine (likely via WebSocket/API).
    *   Allows human-vs-bot or bot-vs-bot observation.

## Tech Stack
*   **Engine & Optimization:** Python 3.x
*   **Frontend:** React, Node.js
*   **Reference Logic:** C# (DCGO)

## Roadmap

### Phase 1: Python Engine PoC
*   [x] Establish Core Data Structures (`Entity`, `CardSource`, `Player`).
*   [ ] Implement Game Loop (Phases, Turn Structure).
*   [ ] Port Card Effects and Interaction Logic.
*   [ ] Implement Headless CLI Runner.

### Phase 2: MCTS Agent Integration
*   [ ] Implement MCTS Node and Tree structure.
*   [ ] Integrate Agent with Game Controller.
*   [ ] Benchmark Agent performance.

### Phase 3: Deck Optimization Layer
*   [ ] Create Genetic Algorithm wrapper.
*   [ ] Define "Meta Decks" for evaluation.
*   [ ] Implement result parsing and population evolution.

### Phase 4: React Frontend Integration
*   [ ] Initialize React project.
*   [ ] Create Game Board UI.
*   [ ] Connect Frontend to Python Engine.

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
