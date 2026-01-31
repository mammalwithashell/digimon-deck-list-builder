from enum import Enum, auto

class CardColor(Enum):
    Red = 0
    Blue = 1
    Yellow = 2
    Green = 3
    White = 4
    Black = 5
    Purple = 6
    NoColor = 7 # Renamed from None to avoid conflict with Python's None

class CardKind(Enum):
    Digimon = 0
    Tamer = 1
    Option = 2
    DigiEgg = 3

class Rarity(Enum):
    C = 0
    U = 1
    R = 2
    SR = 3
    SEC = 4
    P = 5
    NoRarity = 6 # Renamed from None to avoid conflict

class GamePhase(Enum):
    Start = 0
    Draw = 1
    Breeding = 2
    Main = 3
    End = 4

class EffectTiming(Enum):
    NoTiming = 0 # Renamed from None
    OnUseOption = 1
    OnDeclaration = 2
    OnEnterFieldAnyone = 3
    OnGetDamage = 4
    OptionSkill = 5
    OnDestroyedAnyone = 6
    WhenDigisorption = 7
    WhenRemoveField = 8
    WhenPermanentWouldBeDeleted = 9
    WhenReturntoLibraryAnyone = 10
    WhenReturntoHandAnyone = 11
    WhenUntapAnyone = 12
    OnEndAttackPhase = 13
    OnEndTurn = 14
    OnStartTurn = 15
    OnEndMainPhase = 16
    OnDraw = 17
    OnAddHand = 18
    OnLoseSecurity = 19
    OnAddSecurity = 20
    OnUseDigiburst = 21
    OnDiscardHand = 22
    OnDiscardSecurity = 23
    OnDiscardLibrary = 24
    OnKnockOut = 25
    OnMove = 26
    OnEndCoinToss = 27
    OnUseAttack = 28
    OnTappedAnyone = 29
    OnUnTappedAnyone = 30
    OnAddDigivolutionCards = 31
    OnAllyAttack = 32
    OnCounterTiming = 33
    OnBlockAnyone = 34
    OnSecurityCheck = 35
    OnAttackTargetChanged = 36
    OnEndBlockDesignation = 37
    SecuritySkill = 38
    OnStartMainPhase = 39
    OnStartBattle = 40
    OnEndBattle = 41
    OnDetermineDoSecurityCheck = 42
    OnEndAttack = 43
    BeforePayCost = 44
    AfterPayCost = 45
    OnDigivolutionCardDiscarded = 46
    OnDigivolutionCardReturnToDeckBottom = 47
    OnReturnCardsToLibraryFromTrash = 48
    OnPermamemtReturnedToHand = 49
    OnReturnCardsToHandFromTrash = 50
    AfterEffectsActivate = 51
    WhenWouldDigivolutionCardDiscarded = 52
    WhenLinked = 53
    WhenTopCardTrashed = 54
    RulesTiming = 55
    OnRemovedField = 56
    WhenWouldDigivolve = 57
    WhenDigivolving = 58

class AttackResolution(Enum):
    Survivor = 0
    AttackerDeleted = 1
    BattleDraw = 2 # Both deleted
    GameEnd = 3 # Player lost
