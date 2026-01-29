using System.Collections.Generic;
using System.Threading.Tasks;
using UnityEngine;

[CreateAssetMenu(menuName = "Create/CEntity_Base")]
public class CEntity_Base : ScriptableObject
{
    public int CardIndex;
    public List<CardColor> cardColors;
    public int PlayCost;
    public List<EvoCost> EvoCosts;
    public int Level;
    public string CardName_JPN;
    public string CardName_ENG;
    public List<string> Form_JPN;
    public List<string> Form_ENG;
    public List<string> Attribute_JPN;
    public List<string> Attribute_ENG;
    public List<string> Type_JPN;
    public List<string> Type_ENG;
    public string CardSpriteName;
    public CardKind cardKind;
    [TextArea] public string EffectDiscription_JPN;
    [TextArea] public string EffectDiscription_ENG;
    [TextArea] public string InheritedEffectDiscription_JPN;
    [TextArea] public string InheritedEffectDiscription_ENG;
    [TextArea] public string SecurityEffectDiscription_JPN;
    [TextArea] public string SecurityEffectDiscription_ENG;
    public string CardEffectClassName;
    public int DP;
    public Rarity rarity;
    public int OverflowMemory;
    public int LinkDP;
    [TextArea] public string LinkEffect;
    [TextArea] public string LinkRequirement;
    public string CardID;
    public int MaxCountInDeck;

    // Stubs for properties and methods
    public bool HasInhetitedEffect => !string.IsNullOrEmpty(InheritedEffectDiscription_ENG);
    public bool HasSecutiryEffect => !string.IsNullOrEmpty(SecurityEffectDiscription_ENG);
    public bool IsACE => false; // Logic needed?
    public string SetID => ""; // Logic needed?
    public bool IsPermanent => cardKind == CardKind.Digimon || cardKind == CardKind.Tamer || cardKind == CardKind.DigiEgg;
    public bool HasLevel => Level > 0;
    public bool HasPlayCost => PlayCost >= 0; // Check logic
    public bool HasUseCost => PlayCost >= 0; // Check logic

    public Task LoadCardImage() { return Task.CompletedTask; }
    public Task<Sprite> GetCardSprite() { return Task.FromResult<Sprite>(null); }
}
