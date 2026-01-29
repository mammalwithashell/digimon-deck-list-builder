using System;

[Serializable]
public class EvoCost : IEquatable<EvoCost>
{
    public CardColor CardColor;
    public int Level;
    public int MemoryCost;

    public bool Equals(EvoCost other)
    {
        if (other == null) return false;
        return CardColor == other.CardColor && Level == other.Level && MemoryCost == other.MemoryCost;
    }

    public override int GetHashCode()
    {
        return HashCode.Combine(CardColor, Level, MemoryCost);
    }
}
