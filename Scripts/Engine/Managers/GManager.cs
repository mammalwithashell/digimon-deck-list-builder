using System;
using System.Collections;
using System.Collections.Generic;
using UnityEngine;

public class GManager : MonoBehaviourPun
{
    public static GManager instance;
    public Player You;
    public Player Opponent;
    public TurnStateMachine turnStateMachine;
    public bool IsAI;
    public int CardIndex;

    // Stubs for fields
    public bool isAuto;

    private void Awake()
    {
        instance = this;
        // turnStateMachine = GetComponent<TurnStateMachine>(); // Usually attached
    }

    public void OnDestroy()
    {
        if (instance == this) instance = null;
    }

    // Stubs for RPCs and Methods
    public void DrawCardRPC() { }
    public void TrashCardRPC() { }
    public void TopDeckCardRPC() { }
    public void PlaceInSecurityRPC(bool keyInput) { }
    public void AlterMemoryRPC(int value) { }

    public IEnumerator DrawCard(Player _player) { yield break; }
}
