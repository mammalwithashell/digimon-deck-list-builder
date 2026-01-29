using System;
using System.Collections.Generic;
using UnityEngine;

public class CardSource : MonoBehaviour
{
    private CEntity_Base _cEntity_Base;
    public PhotonView PhotonView { get; set; }
    public Player Owner { get; set; }
    public int CardIndex { get; set; }
    public CEntity_EffectController CEntity_EffectController { get; set; }
    public bool IsFlipped { get; set; }
    public int BaseDP { get; set; }
    public bool IsToken { get; set; }
    public bool WillBeRemoveSources { get; set; }
    public bool IsBeingRevealed { get; set; }
    public Permanent PermanentJustBeforeRemoveField { get; set; }

    public bool CanPlayFromHandDuringMainPhase => true; // Stub
    public bool CanNotPlayThisOption => false; // Stub
    public bool MatchColorRequirement => true; // Stub

    public List<CardColor> BaseCardColorsFromEntity => _cEntity_Base?.cardColors ?? new List<CardColor>();
    public List<CardColor> BaseCardColors => BaseCardColorsFromEntity; // Logic needed
    public List<CardColor> CardColors => BaseCardColors; // Logic needed

    public int Level => _cEntity_Base?.Level ?? 0;
    public string CardID => _cEntity_Base?.CardID ?? "";
    public List<string> CardNames => new List<string> { _cEntity_Base?.CardName_ENG };
    public List<string> CardTraits => new List<string>(); // Stub

    public bool IsDigimon => _cEntity_Base?.cardKind == CardKind.Digimon;
    public bool IsOption => _cEntity_Base?.cardKind == CardKind.Option;
    public bool IsTamer => _cEntity_Base?.cardKind == CardKind.Tamer;
    public bool IsDigiEgg => _cEntity_Base?.cardKind == CardKind.DigiEgg;
    public bool IsPermanent => IsDigimon || IsTamer || IsDigiEgg;

    public int GetCostItself => _cEntity_Base?.PlayCost ?? 0;
    public bool HasPlayCost => _cEntity_Base?.HasPlayCost ?? false;
    public bool HasUseCost => _cEntity_Base?.HasUseCost ?? false;

    public void Init() { }
    public void SetBaseData(CEntity_Base cEntity_Base, Player owner)
    {
        _cEntity_Base = cEntity_Base;
        Owner = owner;
    }

    public Permanent PermanentOfThisCard()
    {
        // Search logic or reference
        return null;
    }

    public List<ICardEffect> EffectList(EffectTiming timing)
    {
        return new List<ICardEffect>();
    }

    public int PayingCost(object root, List<Permanent> targetPermanents, bool checkAvailability = false, bool ignoreLevel = false, int FixedCost = -1)
    {
        return GetCostItself;
    }
}
