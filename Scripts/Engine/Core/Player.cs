using System.Collections.Generic;
using UnityEngine;

public class Player : MonoBehaviour
{
    public int PlayerID { get; set; }
    public string PlayerName { get; set; }
    public bool IsMyTurn { get; set; }
    public int Memory { get; set; }
    public List<CardSource> HandCards { get; set; } = new List<CardSource>();
    public List<CardSource> LibraryCards { get; set; } = new List<CardSource>();
    public List<CardSource> SecurityCards { get; set; } = new List<CardSource>();
    public List<CardSource> TrashCards { get; set; } = new List<CardSource>();
    public List<CardSource> DigitamaLibraryCards { get; set; } = new List<CardSource>();

    public bool IsLose => false; // Stub

    public void Draw() { } // Stub
}
