using System.Collections;
using UnityEngine;

public class TurnStateMachine : MonoBehaviourPunCallbacks
{
    public GameContext gameContext; // Need GameContext class?
    public bool IsSelecting;
    public bool isSync;
    public bool isExecuting;
    public int TurnCount { get; set; }
    public bool DoneStartGame { get; set; }
    public bool endGame { get; set; }

    public IEnumerator Init() { yield break; }
    public IEnumerator GameStateMachine() { yield break; }
    private IEnumerator StartGame() { yield break; }
    public void EndGame(Player Winner, bool Surrendered, string effectName = "")
    {
        endGame = true;
    }
}

public class GameContext
{
    public Player[] Players;
}
