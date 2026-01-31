# Digimon TCG Deck List Builder & Optimizer

> **Status:** Pre-Alpha / Active Development
> **Stack:** Python (Gymnasium, PyTorch, FastAPI) + React

A high-performance, headless deck optimization engine for the **Digimon Trading Card Game**. Unlike standard simulators, this project treats deck building as a **Markov Decision Process (MDP)**, using Reinforcement Learning and Genetic Algorithms to discover optimal lists against a weighted "Meta Gauntlet."

## 🏗 Architecture

### 1. The Engine (`digimon_gym`)
A custom, lightweight Python implementation of the Digimon TCG rules, built to the **OpenAI Gym (Gymnasium)** standard.
*   **Headless by Design:** Runs purely on CPU with no graphical overhead, enabling thousands of simulations per minute (similar to **DeckGym**).
*   **Vectorized State:** The game board (Security, Hand, Battle Area) is serialized into **NumPy arrays** for direct consumption by Neural Networks.
*   **Action Masking:** Implements strict validity masking to prevent agents from attempting illegal moves (e.g., evolving a Tamer).

### 2. The Optimizer ("The Architect")
Based on the **Q-DeckRec** algorithm. Instead of randomly evolving decks, an RL agent learns a "Search Policy" to iteratively swap cards to maximize a **Cumulative Exponential Reward**.
*   **Input:** A core list of cards + A "Side Deck" pool.
*   **Objective:** Maximize win rate across a 4-Round "Locals" Tournament simulation.
*   **Reward Function:** $R = \sum \exp(10 \cdot \text{WinRate})$

### 3. The Pilot ("Battle Agent")
The AI responsible for piloting the decks during simulation. Unlike the Architect (which runs once per iteration), the Pilot runs thousands of times per iteration.
*  **Modular Brains**: The engine supports swapping the Pilot's logic to trade off speed vs. accuracy.
    * Greedy Heuristic: Fast execution (<1ms) for early-stage optimization.
    * MCTS (Procedural Personas): Slower, deliberate play using Monte Carlo Tree Search to simulate human archetypes (Aggro, Control, Combo).
    * RL Pilot (PPO): A trained neural network that approximates MCTS behavior for high-speed "Agent vs. Agent" training.
* Action Masking: Ensures the Pilot only attempts valid moves (e.g., preventing attacks with summoning sickness) to minimize training noise

### 4. The Gauntlet ("Locals Simulator")
We do not optimize against a single opponent. The engine simulates a "Locals" environment:
*   **Weighted Meta:** Opponents are sampled from a pool of Tier 1, Tier 2, and Rogue decks based on usage data (sourced from *Egman Events* & *Digimon Meta*).
*   **Fixed Swiss:** The candidate deck plays all 4 rounds regardless of record to generate granular performance data (Variance Reduction).
*   **Procedural Personas:** Opponent bots utilize **MCTS** with specific heuristic biases (Aggro, Control, Combo) to simulate human playstyles.

### 5. The Frontend (React)
A visualization tool for "Headed" gameplay and analysis.
*   **Log Replay:** Visualizes the raw logs from the Python engine to help humans understand *why* a deck is winning or losing.
*   **Human vs. Agent:** Allows a user to play against the MCTS/RL agents by sending actions via API.

---

## 🚀 Getting Started

### Prerequisites
*   Python 3.10+
*   Node.js 18+

### Installation

1.  **Clone the repository**
    ```bash
    git clone https://github.com/mammalwithashell/digimon-deck-list-builder.git
    cd digimon-deck-list-builder
    ```

2.  **Install Python Dependencies**
    ```bash
    pip install -r requirements.txt
    # Requires: gymnasium, numpy, torch, fastapi, uvicorn
    ```

3.  **Install Frontend Dependencies**
    ```bash
    cd frontend
    npm install
    ```

---

## 💻 Usage

### 1. Headless Optimization (CLI)
To run the optimizer on a deck list against the current meta gauntlet:

```bash
python main.py optimize --deck "path/to/my_deck.txt" --gauntlet "data/meta_bt14" --sims 1000
```
*   `--sims`: Number of simulations per iteration.
*   `--gauntlet`: Folder containing opponent deck lists.

### 2. Interactive Mode (React)
To launch the web interface for playing against agents or viewing optimization graphs:

```bash
# Terminal 1: Start Backend
uvicorn api.main:app --reload

# Terminal 2: Start Frontend
cd frontend
npm start
```

---

## 🧠 Agent Configuration

See [AGENTS.md](AGENTS.md) for detailed specifications on:
*   **State Space:** How cards are converted to One-Hot vectors.
*   **Action Mapping:** How Integer `42` maps to "Trash Card at Index 2".
*   **MCTS Heuristics:** The math behind the "Aggro" and "Control" personas.

---

## 🛠 Contributing

**Current Focus: The Gym Wrapper**
We are currently migrating logic from the legacy C# dump to the Python `digimon_gym` environment.

**Contribution Rules for AI/Jules:**
1.  **No Blocking Input:** Do not use `input()` or `await`. All logic must be contained within `env.step(action)`.
2.  **Strict Typing:** Observations must be returned as `np.array`.
3.  **Action Integers:** The engine must accept `int` actions. Use the `ActionMapper` class to translate integers to game logic.

---

## 📚 References & Credits
*   **Q-DeckRec:** Chen et al. (2018) - *A Fast Deck Recommendation System for CCGs*.
*   **DeckGym:** High-performance TCG simulation architecture.
*   **DCGO:** Original C# card logic reference.
*   **Digimon Meta / Egman Events:** Sources for tournament deck lists.
