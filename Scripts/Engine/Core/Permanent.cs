using System;
using System.Collections;
using System.Collections.Generic;
using UnityEngine;

public class Permanent
{
    public List<CardSource> cardSources = new List<CardSource>();
    public List<CardSource> DigivolutionCards => cardSources; // Simplification
    public CardSource TopCard => cardSources.Count > 0 ? cardSources[cardSources.Count - 1] : null;
    public int Level => TopCard?.Level ?? 0;
    public bool IsSuspended;
    public bool IsToken => TopCard?.IsToken ?? false;
    public bool IsDigimon => TopCard?.IsDigimon ?? false;
    public bool IsTamer => TopCard?.IsTamer ?? false;
    public bool IsOption => TopCard?.IsOption ?? false;
    public int DP => TopCard?.BaseDP ?? 0; // Needs calculation logic

    public bool CanAttack(ICardEffect cardEffect, bool withoutTap = false, bool isVortex = false) => true; // Stub
    public bool CanBlock(Permanent AttackingPermanent) => false; // Stub

    public List<ICardEffect> EffectList(EffectTiming timing) => new List<ICardEffect>();

    public Permanent(List<CardSource> cardSources)
    {
        this.cardSources = cardSources;
    }

    public void AddCardSource(CardSource cardSource)
    {
        cardSources.Add(cardSource);
    }

    public IEnumerator DiscardEvoRoots(bool ignoreOverflow = false, bool putToTrash = true)
    {
        yield break;
    }
}
