using System.Collections.Generic;
using UnityEngine;

public class Player : MonoBehaviour
{
    public int PlayerID;
    public string PlayerName;
    public bool isMyTurn;
    public int Memory;
    public List<CardSource> HandCards = new List<CardSource>();
    public List<CardSource> LibraryCards = new List<CardSource>();
    public List<CardSource> SecurityCards = new List<CardSource>();
    public List<CardSource> TrashCards = new List<CardSource>();
    public List<CardSource> DigitamaLibraryCards = new List<CardSource>();

    // Field Permanents? Usually GManager or Player manages this.
    // In GManager dump, it seems GManager holds references to Players.
    // Assuming Field logic is managed elsewhere or stubbed here.

    public bool IsLose => false; // Stub

    public void Draw() { } // Stub
}
