using System;
using System.Collections;
using System.Collections.Generic;
using UnityEngine;

public class GManager : MonoBehaviourPun
{
    public static GManager Instance { get; private set; }

    public Player You { get; set; }
    public Player Opponent { get; set; }
    public TurnStateMachine TurnStateMachine { get; set; }
    public bool IsAI { get; set; }
    public int CardIndex { get; set; }

    // Stubs for fields
    public bool IsAuto { get; set; }

    private void Awake()
    {
        Instance = this;
        // turnStateMachine = GetComponent<TurnStateMachine>(); // Usually attached
    }

    public void OnDestroy()
    {
        if (Instance == this) Instance = null;
    }

    // Stubs for RPCs and Methods
    public void DrawCardRPC() { }
    public void TrashCardRPC() { }
    public void TopDeckCardRPC() { }
    public void PlaceInSecurityRPC(bool keyInput) { }
    public void AlterMemoryRPC(int value) { }

    public IEnumerator DrawCard(Player _player) { yield break; }
}
