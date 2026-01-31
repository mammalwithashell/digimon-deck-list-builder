using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;

public class DigimonRLAdapter
{
    public static readonly DigimonRLAdapter Instance = new DigimonRLAdapter();

    private DigimonRLAdapter() { }

    /// <summary>
    /// Returns a flattened float array representing the board state.
    /// Layout:
    /// [0-3]: Global (Turn, Phase, Memory, PendingAction)
    /// [4-118]: Player 1 (Hand 50, Field 50, Security 5, Trash 10)
    /// [119-233]: Player 2 (Hand 50, Field 50, Security 5, Trash 10)
    /// Total Size: 234
    /// </summary>
    public float[] GetBoardStateTensor()
    {
        if (GManager.Instance == null) return new float[234];

        List<float> state = new List<float>();

        // 1. Global Info
        var g = GManager.Instance;
        var p1 = g.You;
        var p2 = g.Opponent;

        state.Add(g.TurnStateMachine.TurnCount);
        state.Add(0); // Phase (Placeholder)
        state.Add(p1.MemoryForPlayer);
        state.Add(0); // PendingAction (Placeholder)

        // 2. Player 1 State
        AppendPlayerState(state, p1);

        // 3. Player 2 State
        AppendPlayerState(state, p2);

        return state.ToArray();
    }

    private void AppendPlayerState(List<float> state, Player player)
    {
        // Hand (Max 10 cards, 5 features each = 50)
        for (int i = 0; i < 10; i++)
        {
            if (i < player.HandCards.Count)
            {
                var card = player.HandCards[i];
                state.Add(1); // Exists
                state.Add(card.Level);
                state.Add(card.BaseDP);
                state.Add(0); // Color (Placeholder)
                state.Add(0); // Type (Placeholder)
            }
            else
            {
                state.AddRange(new float[] { 0, 0, 0, 0, 0 });
            }
        }

        // Field (Max 10 perms, 5 features each = 50)
        var perms = player.GetFieldPermanents();
        for (int i = 0; i < 10; i++)
        {
            if (i < perms.Count)
            {
                var perm = perms[i];
                state.Add(1); // Exists
                state.Add(perm.Level);
                state.Add(perm.DP);
                state.Add(perm.IsSuspended ? 1 : 0);
                state.Add(0); // Type
            }
            else
            {
                state.AddRange(new float[] { 0, 0, 0, 0, 0 });
            }
        }

        // Security (Max 5 shown, 1 feature = 5)
        for (int i = 0; i < 5; i++)
        {
            state.Add(i < player.SecurityCards.Count ? 1 : 0);
        }

        // Trash (Max 10 shown, 1 feature = 10)
        for (int i = 0; i < 10; i++)
        {
            state.Add(i < player.TrashCards.Count ? 1 : 0);
        }
    }

    /// <summary>
    /// Executes the action and returns reward and done flag.
    /// Returns: [Reward, Done (0 or 1)]
    /// </summary>
    public float[] ApplyAction(int action)
    {
        if (GManager.Instance == null) return new float[] { 0, 0 };

        float reward = 0;
        bool done = false;

        try
        {
            var player = GManager.Instance.You;

            if (action >= 0 && action < 10)
            {
                // Play Card
                int handIndex = action;
                if (handIndex < player.HandCards.Count)
                {
                    Debug.Log($"[RL] Playing card at index {handIndex}");
                    // Implementation Note: Invoke PlayCardClass logic here.
                    // Example: PlayCardClass.GetPlayCardClassFromHashtable(...)
                }
            }
            else if (action >= 10 && action < 20)
            {
                // Attack
                int permIndex = action - 10;
                var perms = player.GetFieldPermanents();
                if (permIndex < perms.Count)
                {
                    Debug.Log($"[RL] Attacking with permanent {permIndex}");
                    // Implementation Note: Invoke Attack logic here.
                }
            }
            else if (action == 20)
            {
                // Pass
                Debug.Log("[RL] Passing turn");
                // Implementation Note: GManager.Instance.TurnStateMachine.PassTurn() or similar.
            }

            // Check Game End
            if (GManager.Instance.TurnStateMachine.endGame)
            {
                done = true;
                if (!player.IsLose) reward = 1.0f;
                else reward = -1.0f;
            }
        }
        catch (Exception e)
        {
            Debug.LogError($"[RL] Error applying action: {e.Message}");
        }

        return new float[] { reward, done ? 1.0f : 0.0f };
    }

    public void Reset()
    {
        if (GManager.Instance == null) return;
        Debug.Log("[RL] Reset requested.");
        // Implementation Note: Invoke Game Start / Reset Logic.
        // Example: GManager.Instance.TurnStateMachine.StartGame();
    }
}
