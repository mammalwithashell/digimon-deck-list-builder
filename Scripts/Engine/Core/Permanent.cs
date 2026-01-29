using System;
using System.Collections;
using System.Collections.Generic;
using UnityEngine;

public class Permanent
{
    public List<CardSource> CardSources { get; set; } = new List<CardSource>();
    public List<CardSource> DigivolutionCards => CardSources; // Simplification
    public CardSource TopCard => CardSources.Count > 0 ? CardSources[CardSources.Count - 1] : null;
    public int Level => TopCard?.Level ?? 0;
    public bool IsSuspended { get; set; }
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
        this.CardSources = cardSources;
    }

    public void AddCardSource(CardSource cardSource)
    {
        CardSources.Add(cardSource);
    }

    public IEnumerator DiscardEvoRoots(bool ignoreOverflow = false, bool putToTrash = true)
    {
        yield break;
    }
}
