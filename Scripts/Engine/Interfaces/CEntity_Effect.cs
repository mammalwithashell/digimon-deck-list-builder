using System.Collections.Generic;

public abstract class CEntity_Effect : MonoBehaviourPunCallbacks
{
    public virtual List<ICardEffect> CardEffects(EffectTiming timing, CardSource cardSource)
    {
        return new List<ICardEffect>();
    }

    public List<ICardEffect> GetCardEffects(EffectTiming timing, CardSource cardSource)
    {
        return CardEffects(timing, cardSource);
    }

    public static bool isExistOnField(CardSource card)
    {
        return card != null && card.PermanentOfThisCard() != null;
    }
}
