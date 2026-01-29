using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;

// Interface representing the game state for MCTS
public interface IGameState
{
    // Returns a list of indices representing legal moves (e.g., card index in hand, target index on field)
    List<int> GetLegalMoves();

    // Applies the move to the state.
    void ApplyMove(int moveIndex);

    // Create a deep copy of the state.
    // CRITICAL: This requires the underlying engine to support serialization or deep copying.
    IGameState Clone();

    bool IsGameOver();

    // Returns 1 if the agent won, 0 if lost (or other metric)
    float GetResult(int agentPlayerId);

    int CurrentPlayerId { get; }
}

public class MCTSNode
{
    public IGameState State { get; private set; }
    public MCTSNode Parent { get; private set; }
    public List<MCTSNode> Children { get; private set; }
    public int Move { get; private set; } // The move that led to this state
    public int Visits { get; set; }
    public float Score { get; set; }
    public List<int> UntriedMoves { get; private set; }

    public MCTSNode(IGameState state, MCTSNode parent, int move)
    {
        State = state;
        Parent = parent;
        Move = move;
        Children = new List<MCTSNode>();
        UntriedMoves = state.GetLegalMoves();
        Visits = 0;
        Score = 0;
    }

    public bool IsFullyExpanded => UntriedMoves.Count == 0;
    public bool IsTerminal => State.IsGameOver();
}

public class MCTSAgent : MonoBehaviour
{
    [Header("MCTS Settings")]
    public int SimulationCount = 1000;
    public float UctConstant = 1.41f;
    public int MaxSimulationDepth = 50;

    private int agentPlayerId = 0; // Assuming 0 for now

    public void Initialize(GManager manager)
    {
        // Hook into the game manager.
        // For example, if GManager allows registering an "AI Decision Maker":
        // manager.AIDecisionMaker = this;

        // Or if we are using the Headless components, they will call methods on this Agent.
        Debug.Log("[MCTSAgent] Initialized.");
    }

    // The main entry point for making a decision
    public int GetBestMove(IGameState currentState)
    {
        MCTSNode root = new MCTSNode(currentState.Clone(), null, -1);

        for (int i = 0; i < SimulationCount; i++)
        {
            MCTSNode node = Select(root);

            // Expansion
            if (!node.IsTerminal && !node.IsFullyExpanded)
            {
                node = Expand(node);
            }

            // Simulation
            float result = Simulate(node.State);

            // Backpropagation
            Backpropagate(node, result);
        }

        // Select best child (e.g., most visited)
        if (root.Children.Count == 0) return -1; // No moves?

        MCTSNode bestChild = root.Children.OrderByDescending(c => c.Visits).First();
        return bestChild.Move;
    }

    // 1. Selection: Traverse tree using UCB1
    private MCTSNode Select(MCTSNode node)
    {
        while (!node.IsTerminal && node.IsFullyExpanded)
        {
            node = GetBestChild(node);
        }
        return node;
    }

    private MCTSNode GetBestChild(MCTSNode node)
    {
        // UCB1
        return node.Children.OrderByDescending(child =>
            (child.Score / child.Visits) +
            UctConstant * Mathf.Sqrt(Mathf.Log(node.Visits) / child.Visits)
        ).First();
    }

    // 2. Expansion: Add one child node
    private MCTSNode Expand(MCTSNode node)
    {
        int move = node.UntriedMoves[0];
        node.UntriedMoves.RemoveAt(0);

        IGameState newState = node.State.Clone();
        newState.ApplyMove(move);

        MCTSNode child = new MCTSNode(newState, node, move);
        node.Children.Add(child);
        return child;
    }

    // 3. Simulation: Random rollout
    private float Simulate(IGameState state)
    {
        IGameState simulationState = state.Clone();
        int depth = 0;

        while (!simulationState.IsGameOver() && depth < MaxSimulationDepth)
        {
            List<int> moves = simulationState.GetLegalMoves();
            if (moves.Count == 0) break;

            int randomMove = moves[UnityEngine.Random.Range(0, moves.Count)];
            simulationState.ApplyMove(randomMove);
            depth++;
        }

        return simulationState.GetResult(agentPlayerId);
    }

    // 4. Backpropagation: Update stats up the tree
    private void Backpropagate(MCTSNode node, float result)
    {
        while (node != null)
        {
            node.Visits++;
            // If the node's state player is the agent, add result.
            // Note: UCB logic often requires viewing score from the perspective of the *parent's* player.
            // For simplicity here, we assume Score accumulates wins for the agent.
            node.Score += result;
            node = node.Parent;
        }
    }
}

// ---------------------------------------------------------
// Integration Notes:
// ---------------------------------------------------------
// To hook this into DCGO AutoProcessing logic:
//
// 1. You must implement the 'IGameState' interface using the DCGO engine classes (GManager, Player, etc.).
//    This is the most challenging part as it requires Deep Copying the Unity state.
//    Recommendation: Create a lightweight 'Model' class that mirrors the game state and is purely C#.
//
// 2. Intercept 'Select*Effect' classes (SelectCardEffect.cs, SelectCost.cs, etc.).
//    In their 'Activate()' coroutines, instead of showing UI and waiting for clicks:
//
//    if (HeadlessGameManager.Instance != null) {
    //        IGameState currentState = new DCGOGameStateAdapter(GManager.Instance);
//        int bestMove = HeadlessGameManager.Instance.GetComponent<MCTSAgent>().GetBestMove(currentState);
//        // Apply bestMove to the selection result
//        yield break;
//    }
//
// 3. 'DCGOGameStateAdapter' would be a class that wraps the current GManager state and exposes it via IGameState.
