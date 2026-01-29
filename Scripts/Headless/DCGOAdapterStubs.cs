using System.Collections.Generic;

// This class is a placeholder showing how to adapt the DCGO GManager to the IGameState interface.
// Since GManager and CardSource are complex Unity objects, a full implementation requires
// a dedicated "Lightweight Game Model" (Pure C#) that can be cloned.

public class DCGOAdapterStub : IGameState
{
    // In a real implementation, this would hold a reference to a 'GameModel' object
    // which is a snapshot of GManager's state.

    public DCGOAdapterStub() { }

    public List<int> GetLegalMoves()
    {
        // 1. Check current phase/state in GManager (or internal model)
        // 2. Return list of indices (e.g., card index in hand to play, or target index)
        return new List<int>();
    }

    public void ApplyMove(int moveIndex)
    {
        // 1. Translate moveIndex to an action (e.g. PlayCard(hand[moveIndex]))
        // 2. Execute action on the internal model
    }

    public IGameState Clone()
    {
        // Return a new DCGOAdapterStub with a deep copy of the internal model
        // This is crucial for MCTS simulation speed and correctness.
        return new DCGOAdapterStub();
    }

    public bool IsGameOver()
    {
        // Check model's win condition
        return false;
    }

    public float GetResult(int agentPlayerId)
    {
        // Return 1.0 if agent won, 0.0 otherwise
        return 0.5f;
    }

    public int CurrentPlayerId => 0;
}
