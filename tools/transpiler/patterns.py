"""Regex patterns, timing maps, and keyword maps for C# → Python transpilation."""
import re

# ─── Pattern Recognition ────────────────────────────────────────────

# Map DCGO EffectTiming to our Python EffectTiming enum values
TIMING_MAP = {
    "EffectTiming.None": "EffectTiming.NoTiming",
    "EffectTiming.OnUseOption": "EffectTiming.OnUseOption",
    "EffectTiming.OnDeclaration": "EffectTiming.OnDeclaration",
    "EffectTiming.OnEnterFieldAnyone": "EffectTiming.OnEnterFieldAnyone",
    "EffectTiming.OnGetDamage": "EffectTiming.OnGetDamage",
    "EffectTiming.OptionSkill": "EffectTiming.OptionSkill",
    "EffectTiming.OnDestroyedAnyone": "EffectTiming.OnDestroyedAnyone",
    "EffectTiming.WhenDigisorption": "EffectTiming.WhenDigisorption",
    "EffectTiming.WhenRemoveField": "EffectTiming.WhenRemoveField",
    "EffectTiming.WhenPermanentWouldBeDeleted": "EffectTiming.WhenPermanentWouldBeDeleted",
    "EffectTiming.WhenReturntoLibraryAnyone": "EffectTiming.WhenReturntoLibraryAnyone",
    "EffectTiming.WhenReturntoHandAnyone": "EffectTiming.WhenReturntoHandAnyone",
    "EffectTiming.WhenUntapAnyone": "EffectTiming.WhenUntapAnyone",
    "EffectTiming.OnEndAttackPhase": "EffectTiming.OnEndAttackPhase",
    "EffectTiming.OnEndTurn": "EffectTiming.OnEndTurn",
    "EffectTiming.OnStartTurn": "EffectTiming.OnStartTurn",
    "EffectTiming.OnEndMainPhase": "EffectTiming.OnEndMainPhase",
    "EffectTiming.OnDraw": "EffectTiming.OnDraw",
    "EffectTiming.OnAddHand": "EffectTiming.OnAddHand",
    "EffectTiming.OnLoseSecurity": "EffectTiming.OnLoseSecurity",
    "EffectTiming.OnAddSecurity": "EffectTiming.OnAddSecurity",
    "EffectTiming.OnUseDigiburst": "EffectTiming.OnUseDigiburst",
    "EffectTiming.OnDiscardHand": "EffectTiming.OnDiscardHand",
    "EffectTiming.OnDiscardSecurity": "EffectTiming.OnDiscardSecurity",
    "EffectTiming.OnDiscardLibrary": "EffectTiming.OnDiscardLibrary",
    "EffectTiming.OnKnockOut": "EffectTiming.OnKnockOut",
    "EffectTiming.OnMove": "EffectTiming.OnMove",
    "EffectTiming.OnUseAttack": "EffectTiming.OnUseAttack",
    "EffectTiming.OnTappedAnyone": "EffectTiming.OnTappedAnyone",
    "EffectTiming.OnUnTappedAnyone": "EffectTiming.OnUnTappedAnyone",
    "EffectTiming.OnAddDigivolutionCards": "EffectTiming.OnAddDigivolutionCards",
    "EffectTiming.OnAllyAttack": "EffectTiming.OnAllyAttack",
    "EffectTiming.OnCounterTiming": "EffectTiming.OnCounterTiming",
    "EffectTiming.OnBlockAnyone": "EffectTiming.OnBlockAnyone",
    "EffectTiming.OnSecurityCheck": "EffectTiming.OnSecurityCheck",
    "EffectTiming.OnAttackTargetChanged": "EffectTiming.OnAttackTargetChanged",
    "EffectTiming.OnEndBlockDesignation": "EffectTiming.OnEndBlockDesignation",
    "EffectTiming.SecuritySkill": "EffectTiming.SecuritySkill",
    "EffectTiming.OnStartMainPhase": "EffectTiming.OnStartMainPhase",
    "EffectTiming.OnStartBattle": "EffectTiming.OnStartBattle",
    "EffectTiming.OnEndBattle": "EffectTiming.OnEndBattle",
    "EffectTiming.OnDetermineDoSecurityCheck": "EffectTiming.OnDetermineDoSecurityCheck",
    "EffectTiming.OnEndAttack": "EffectTiming.OnEndAttack",
    "EffectTiming.BeforePayCost": "EffectTiming.BeforePayCost",
    "EffectTiming.AfterPayCost": "EffectTiming.AfterPayCost",
    "EffectTiming.OnDigivolutionCardDiscarded": "EffectTiming.OnDigivolutionCardDiscarded",
    "EffectTiming.OnDigivolutionCardReturnToDeckBottom": "EffectTiming.OnDigivolutionCardReturnToDeckBottom",
    "EffectTiming.OnReturnCardsToLibraryFromTrash": "EffectTiming.OnReturnCardsToLibraryFromTrash",
    "EffectTiming.OnPermamemtReturnedToHand": "EffectTiming.OnPermamemtReturnedToHand",
    "EffectTiming.OnReturnCardsToHandFromTrash": "EffectTiming.OnReturnCardsToHandFromTrash",
    "EffectTiming.AfterEffectsActivate": "EffectTiming.AfterEffectsActivate",
    "EffectTiming.WhenWouldDigivolutionCardDiscarded": "EffectTiming.WhenWouldDigivolutionCardDiscarded",
    "EffectTiming.WhenLinked": "EffectTiming.WhenLinked",
    "EffectTiming.WhenTopCardTrashed": "EffectTiming.WhenTopCardTrashed",
    "EffectTiming.RulesTiming": "EffectTiming.RulesTiming",
    "EffectTiming.OnRemovedField": "EffectTiming.OnRemovedField",
    "EffectTiming.WhenWouldDigivolve": "EffectTiming.WhenWouldDigivolve",
    "EffectTiming.WhenDigivolving": "EffectTiming.WhenDigivolving",
}

# Map timing to ICardEffect boolean properties where applicable
TIMING_TO_PROPERTY = {
    "EffectTiming.OnEnterFieldAnyone": "is_on_play",  # OnPlay/WhenDigivolving
    "EffectTiming.OnAllyAttack": "is_on_attack",
    "EffectTiming.OnDestroyedAnyone": "is_on_deletion",
    "EffectTiming.SecuritySkill": "is_security_effect",
}

# ─── C# Parsing Regex Patterns ─────────────────────────────────────

RE_CLASS = re.compile(r'public class (\w+)\s*:\s*CEntity_Effect')
RE_TIMING_BLOCK = re.compile(r'if\s*\(\s*timing\s*==\s*(EffectTiming\.\w+)\s*\)')
RE_EFFECT_DESC = re.compile(r'return\s+"([^"]+)";\s*$', re.MULTILINE)
RE_SET_INHERITED = re.compile(
    r'SetIsInheritedEffect\s*\(\s*(true|false)\s*\)'
    r'|isInheritedEffect\s*:\s*(true|false)')

RE_HASH_STRING = re.compile(r'SetHashString\s*\(\s*"([^"]+)"\s*\)')
RE_MAX_COUNT = re.compile(r'SetUpActivateClass\s*\([^,]+,\s*[^,]+,\s*(\-?\d+)')
RE_IS_OPTIONAL = re.compile(r'SetUpActivateClass\s*\([^,]+,\s*[^,]+,\s*\-?\d+\s*,\s*(true|false)')
RE_EFFECT_NAME = re.compile(r'SetUpICardEffect\s*\(\s*"([^"]+)"')

# Action patterns in ActivateCoroutine
RE_DRAW = re.compile(r'new DrawClass\s*\([^)]*?(?:drawCount:\s*)?(\d+)', re.DOTALL)
RE_ADD_MEMORY = re.compile(r'\.AddMemory\s*\(\s*(\d+)')
RE_CHANGE_DP = re.compile(r'ChangeDigimonDP\s*\([^,]*,\s*changeValue:\s*(-?\d+)')
RE_DELETE = re.compile(r'Mode\.Destroy|DestroyPermanentsClass')
RE_BOUNCE = re.compile(r'Mode\.Bounce')
RE_SUSPEND = re.compile(r'SuspendPermanentsClass|\.Tap\(\)')
RE_RECOVERY = re.compile(r'new IRecovery\s*\([^,]+,\s*(\d+)')
RE_PLAY_CARD = re.compile(r'PlayPermanentCards|PlayCardClass')
RE_TRASH_HAND = re.compile(r'Mode\.Discard')
RE_TRASH_DIGI = re.compile(r'TrashDigivolutionCards|SelectTrashDigivolutionCards')
RE_ADD_TO_HAND = re.compile(r'Mode\.AddHand|AddHandCards|AddThisCardToHand')
RE_ADD_SECURITY = re.compile(r'AddSecurityCard')
RE_REVEAL = re.compile(r'SimplifiedRevealDeckTopCardsAndSelect|RevealDeckTopCardsAndProcessForAll')
RE_DEGENERATION = re.compile(r'new IDegeneration')
RE_DIGIVOLVE = re.compile(r'DigivolveIntoHandOrTrashCard|AddSelfDigivolutionRequirement')
RE_COST_REDUCTION = re.compile(r'ChangeCostClass|ChangeDigivolutionCostStaticEffect|Cost\s*-=\s*(\d+)')
RE_MIND_LINK = re.compile(r'MindLinkClass')

# Target condition patterns for opponent/own permanent selection
RE_TARGET_DP_LIMIT = re.compile(
    r'\.DP\s*<=?\s*(?:card\.Owner\.MaxDP_DeleteEffect\s*\(\s*)?(\d+)')
RE_TARGET_DP_MIN = re.compile(r'\.DP\s*>=?\s*(\d+)')
RE_TARGET_LEVEL_LIMIT = re.compile(r'\.Level\s*<=?\s*(\d+)')
RE_TARGET_LEVEL_MIN = re.compile(r'\.Level\s*>=?\s*(\d+)')
RE_TARGET_IS_SUSPENDED = re.compile(r'\.IsTapped|\.IsSuspended')

# Reveal count extraction
RE_REVEAL_COUNT = re.compile(
    r'SimplifiedRevealDeckTopCardsAndSelect\s*\(\s*(?:revealCount:\s*)?(\d+)'
    r'|RevealDeckTopCardsAndProcessForAll\s*\([^,]*,\s*(\d+)')

# Play from zone detection
RE_PLAY_FROM_TRASH = re.compile(r'TrashCards|PlayFromTrash|trashCards')
RE_PLAY_FREE = re.compile(r'ignoreCost\s*[:=]\s*true|noCost|withoutPayingCost', re.IGNORECASE)
# Hand-or-trash zone choice pattern (play from either zone)
RE_PLAY_HAND_OR_TRASH = re.compile(
    r'HasMatchConditionOwnersHand.*HasMatchConditionOwnersCardInTrash'
    r'|HasMatchConditionOwnersCardInTrash.*HasMatchConditionOwnersHand',
    re.DOTALL)

# Digivolve details extraction
RE_DIGI_COST_FIXED = re.compile(r'digivolutionCost\s*[:=]\s*(\d+)')
RE_DIGI_IGNORE_REQS = re.compile(r'ignoreDigivolutionRequirement\s*[:=]\s*true')

# Multi-choice / branch detection
RE_MULTI_CHOICE = re.compile(r'EffectChooseClass|ChooseEffect|MultiEffectClass')

# De-digivolve count extraction (Fix 4)
RE_DEGEN_COUNT = re.compile(r'new IDegeneration\s*\([^,]+,\s*(\d+)')

# Fix 11: Additional action patterns from ActivateCoroutine bodies
# SelectPermanentEffect Mode patterns — implicit actions from SetUp() mode parameter
RE_SELECT_PERM_MODE = re.compile(
    r'mode:\s*SelectPermanentEffect\.Mode\.(\w+)')
# IDestroySecurity — trash opponent security cards
RE_DESTROY_SECURITY = re.compile(
    r'new IDestroySecurity\s*\([^)]*destroySecurityCount:\s*(\d+)')
# IReduceSecurity — reduce/remove security
RE_REDUCE_SECURITY = re.compile(r'new IReduceSecurity\s*\(')
# IUnsuspendPermanents — unsuspend digimon
RE_UNSUSPEND = re.compile(r'IUnsuspendPermanents|\.UnTap\(\)')
# GainCanNotAttackPlayerEffect — attack restriction
RE_RESTRICT_ATTACK = re.compile(r'GainCanNotAttackPlayerEffect')
# CanNotSwitchAttackTargetClass — target lock
RE_TARGET_LOCK = re.compile(r'CanNotSwitchAttackTargetClass')
# SetFace — flip security face up
RE_FLIP_SECURITY = re.compile(r'\.SetFace\(\)')
# CardObjectController actions
RE_MOVE_PERMANENT = re.compile(r'CardObjectController\.MovePermanent')
# Return to deck bottom
RE_RETURN_DECK_BOTTOM = re.compile(r'AddLibraryBottomCards|ReturnDeckBottom|PutLibraryBottom')
# Jogress/DNA digivolution condition
RE_JOGRESS = re.compile(r'AddJogressConditionClass|BlastDNADigivolveEffect')
# P2: Mill — IAddTrashCardsFromLibraryTop(count, owner)
RE_MILL = re.compile(
    r'IAddTrashCardsFromLibraryTop\s*\(\s*(?:\w+:\s*)?(\d+|\w+)\s*,\s*(?:\w+:\s*)?(card\.Owner\.Enemy|card\.Owner)')
# P4: Descriptive tagging for non-implementable effects
RE_IGNORE_COLOR = re.compile(r'IgnoreColorConditionClass')
RE_APP_FUSION = re.compile(r'AddAppFusionCondition')
RE_LINK_CONDITION = re.compile(r'AddSelfLinkCondition')
RE_ALSO_TREATED_AS = re.compile(r'AlsoTreatedAs|Also.*?TreatedAs')
RE_CANT_PUT_FIELD = re.compile(r'CanNotPutFieldClass')

# CardEffectCommons.Gain*() keyword-granting methods in ActivateCoroutine bodies
RE_GAIN_KEYWORD = re.compile(r'CardEffectCommons\.Gain(\w+)\s*\(')
GAIN_KEYWORD_MAP = {
    'Jamming': 'jamming',
    'Blocker': 'blocker',
    'Piercing': 'piercing',
    'Rush': 'rush',
    'Reboot': 'reboot',
    'Retaliation': 'retaliation',
    'Raid': 'raid',
    'SecurityAttackPlus1': 'security_attack_plus',
    'Barrier': 'barrier',
    'Evade': 'evade',
    'ArmorPurge': 'armor_purge',
    'Alliance': 'alliance',
    'CanNotUnsuspend': 'cannot_unsuspend',
    'CanNotAttack': 'cannot_attack',
    'CanNotAttackPlayer': 'cannot_attack_player',
    'CanNotBlock': 'cannot_block',
    'SecurityAttackMinus1': 'security_attack_minus',
    'CanNotReturnToHand': 'cannot_return_to_hand',
    'CanNotReturnToDeck': 'cannot_return_to_deck',
    'CanNotBeDeletedByBattle': 'cannot_be_deleted_by_battle',
    'CanNotSuspendPlayerEffect': 'cannot_suspend_player',
    'CanNotUnsuspendPlayerEffect': 'cannot_unsuspend_player',
    'CantUnsuspendNextActivePhase': 'cannot_unsuspend',
    'CantUnsuspendUntilOpponentTurnEnd': 'cannot_unsuspend',
    'CanNotSuspend': 'cannot_suspend',
    'ImmuneFromDPMinus': 'immune_dp_minus',
    'Pierce': 'piercing',
    'CanNotBeBlocked': 'cannot_be_blocked',
    'CanNotAttackPlayerEffect': 'cannot_attack_player',
}

# DP value extraction from ChangeSelfDPStaticEffect
RE_FACTORY_DP_VALUE = re.compile(r'ChangeSelfDPStaticEffect\s*\(\s*(?:changeValue:\s*)?(-?\d+)')

# Security Attack modifier value extraction
RE_FACTORY_SA_VALUE = re.compile(r'ChangeSelfSAttackStaticEffect\s*\(\s*(?:changeValue:\s*)?(-?\d+)')

# Factory method patterns
RE_FACTORY_BLOCKER = re.compile(r'Blocker(?:Self)?StaticEffect')
RE_FACTORY_JAMMING = re.compile(r'Jamming(?:Self)?StaticEffect')
RE_FACTORY_RUSH = re.compile(r'Rush(?:Self)?(?:Static)?Effect')
RE_FACTORY_REBOOT = re.compile(r'Reboot(?:Self)?StaticEffect')
RE_FACTORY_RAID = re.compile(r'Raid(?:Self)?Effect')
RE_FACTORY_ALLIANCE = re.compile(r'Alliance(?:Self)?Effect')
RE_FACTORY_SEC_PLAY = re.compile(r'PlaySelfTamerSecurityEffect|PlaySelfDigimonAfterBattleSecurityEffect')
RE_FACTORY_SA_PLUS = re.compile(r'ChangeSelfSAttackStaticEffect')
RE_FACTORY_DP = re.compile(r'ChangeSelfDPStaticEffect')
RE_FACTORY_DP_ALL = re.compile(r'CardEffectFactory\.ChangeDPStaticEffect\b')  # Fix 5: non-self DP
RE_FACTORY_DP_ALL_VALUE = re.compile(r'ChangeDPStaticEffect\s*\([^)]*changeValue:\s*(-?\d+)')
RE_FACTORY_ARMOR_PURGE = re.compile(r'ArmorPurgeEffect')
RE_FACTORY_BLAST_DIGI = re.compile(r'BlastDigivolveEffect')
RE_FACTORY_SET_MEM_3 = re.compile(r'SetMemoryTo3TamerEffect')
RE_FACTORY_GAIN_MEM = re.compile(r'Gain1MemoryTamerOpponentDigimonEffect')
# Fix 11: Missing factory keywords
RE_FACTORY_PIERCING = re.compile(r'Piercing(?:Self)?StaticEffect')
# Matches CollisionSelfEffect/CollisionEffect but for CollisionSelfStaticEffect only
# when the card argument is 'card' (not 'cardSource') to avoid false positives in
# grant_skill coroutines that call CollisionSelfStaticEffect(false, cardSource, ...).
RE_FACTORY_COLLISION = re.compile(
    r'Collision(?:Self)?Effect'
    r'|CollisionSelfStaticEffect\s*\([^)]*\bcard\b[^S]'
)
RE_FACTORY_BLITZ = re.compile(r'Blitz(?:Self)?Effect')
RE_FACTORY_FORTITUDE = re.compile(r'Fortitude(?:Self)?StaticEffect')
RE_FACTORY_EVADE = re.compile(r'Evade(?:Self)?Effect')
RE_FACTORY_BARRIER = re.compile(r'Barrier(?:Self)?Effect')
RE_FACTORY_DECOY = re.compile(r'Decoy(?:Self)?Effect')
RE_FACTORY_RETALIATION = re.compile(r'Retaliation(?:Self)?Effect')
RE_FACTORY_SAVE = re.compile(r'Save(?:Self)?Effect')
RE_FACTORY_MATERIAL_SAVE = re.compile(r'MaterialSave(?:Self)?Effect')
RE_FACTORY_OVERCLOCK = re.compile(r'Overclock(?:Self)?Effect')
RE_FACTORY_VORTEX = re.compile(r'Vortex(?:Self)?Effect')
RE_FACTORY_TRAINING = re.compile(r'Training(?:Self)?Effect')
RE_FACTORY_PROGRESS = re.compile(r'Progress(?:Self)?(?:Static)?Effect')
# Fix 12: Additional missing keywords found via rules evaluation
RE_FACTORY_DIGISORPTION = re.compile(r'Digisorption(?:Self)?Effect')
RE_FACTORY_DIGIBURST = re.compile(r'DigiBurst(?:Self)?Effect|DigiBurstEffect')
RE_FACTORY_DELAY = re.compile(r'Delay(?:Self)?Effect')
RE_FACTORY_PARTITION = re.compile(r'Partition(?:Self)?Effect')
RE_FACTORY_DIGIXROS = re.compile(r'DigiXros(?:Self)?Effect|DigiCrossEffect')
RE_FACTORY_SCAPEGOAT = re.compile(r'Scapegoat(?:Self)?Effect')
RE_FACTORY_DECODE = re.compile(r'Decode(?:Self)?Effect')
RE_FACTORY_ICECLAD = re.compile(r'(?:Iceclad|IceClad)(?:Self)?Effect')
RE_FACTORY_FRAGMENT = re.compile(r'Fragment(?:Self)?Effect')
RE_FACTORY_EXECUTE = re.compile(r'Execute(?:Self)?Effect')
RE_FACTORY_ADD_DIGI_REQ = re.compile(r'AddSelfDigivolutionRequirementStaticEffect')
RE_FACTORY_CHANGE_DIGI_COST = re.compile(r'ChangeDigivolutionCostStaticEffect')
RE_FACTORY_CHANGE_DIGI_COST_VALUE = re.compile(
    r'ChangeDigivolutionCostStaticEffect\s*\(\s*(?:changeValue:\s*)?(-?\d+)')
RE_FACTORY_DIGI_REQ_COST = re.compile(
    r'AddSelfDigivolutionRequirementStaticEffect\s*\([^)]*digivolutionCost:\s*(\d+)')
RE_FACTORY_DIGI_REQ_NAME = re.compile(
    r'EqualsCardName\s*\(\s*"([^"]+)"\s*\)')
RE_FACTORY_DIGI_REQ_TRAIT = re.compile(
    r'EqualsTraits\s*\(\s*"([^"]+)"\s*\)')
RE_FACTORY_DIGI_REQ_HAS_TS = re.compile(r'\.HasTSTraits')
RE_FACTORY_DIGI_REQ_HAS_APPMON = re.compile(r'\.HasAppmonTraits')

# Condition patterns
RE_COND_ON_BATTLE = re.compile(r'IsExistOnBattleArea\w*\s*\(\s*card\s*\)')
RE_COND_OWNER_TURN = re.compile(r'IsOwnerTurn\s*\(\s*card\s*\)')
RE_COND_ON_PLAY = re.compile(r'CanTriggerOnPlay\s*\(')
RE_COND_ON_ATTACK = re.compile(r'CanTriggerOnAttack\s*\(')
RE_COND_ON_DELETION = re.compile(r'CanTriggerOnDeletion\s*\(')
RE_COND_WHEN_DIGI = re.compile(r'CanTriggerWhenDigivolving\s*\(')
RE_COND_SEC_EFFECT = re.compile(r'CanTriggerSecurityEffect\s*\(')
RE_COND_OPTION_MAIN = re.compile(r'CanTriggerOptionMainEffect\s*\(')
RE_COND_TRAIT = re.compile(r'CardTraits\.Contains\s*\(\s*"([^"]+)"\s*\)')
RE_COND_NAME = re.compile(r'ContainsCardName\s*\(\s*"([^"]+)"\s*\)')
RE_COND_COLOR = re.compile(r'CardColors\.Contains\s*\(\s*CardColor\.(\w+)\s*\)')

# Fix 7: HasText pattern (checks card full text, not just name)
RE_COND_HAS_TEXT = re.compile(r'HasText\s*\(\s*"([^"]+)"\s*\)')
# Fix 8: HasRoyalKnightTraits convenience property
RE_COND_ROYAL_KNIGHT = re.compile(r'HasRoyalKnightTraits')

# Fix 1: Factory condition closure patterns
RE_FACTORY_COND_DIGI_COUNT = re.compile(r'DigivolutionCards\.Count\s*>=?\s*(\d+)')
RE_FACTORY_COND_SOURCE_NAME = re.compile(
    r'DigivolutionCards\.Count\s*\([^)]*EqualsCardName\s*\(\s*"([^"]+)"')
RE_FACTORY_COND_SOURCE_TRAIT = re.compile(
    r'DigivolutionCards\.Count\s*\([^)]*EqualsTraits\s*\(\s*"([^"]+)"')
RE_FACTORY_COND_PERM_NAME = re.compile(r'TopCard\.EqualsCardName\s*\(\s*"([^"]+)"\s*\)')
RE_FACTORY_COND_PERM_TRAIT = re.compile(r'TopCard\.EqualsTraits\s*\(\s*"([^"]+)"\s*\)')
# Fix 5: permanentCondition for ChangeDPStaticEffect (non-self)
RE_PERM_COND_OWNER_AREA = re.compile(r'IsPermanentExistsOnOwnerBattleAreaDigimon')
# Keyword grant targeting patterns
RE_PERM_COND_OPPONENT_AREA = re.compile(r'IsPermanentExistsOnOpponentBattleAreaDigimon')
RE_GRANT_MAX_COUNT = re.compile(r'maxCount\s*=\s*Math\.Min\(\s*(\d+)')
RE_SELECTED_PERMANENT_REF = re.compile(r'selectedPermanent\s*=\s*permanent')
RE_DIGI_COUNT_COMPARE = re.compile(r'DigivolutionCards\.Count\s*<=\s*selectedPermanent\.DigivolutionCards\.Count')

# Regex to detect delegation to a shared coroutine from a timing block
RE_SHARED_COROUTINE_DELEGATE = re.compile(
    r'(?:hash\s*=>|=>\s*)?\s*(\w*[Ss]hared\w*Coroutine\w*)\s*\('
    r'|yield\s+return\s+.*?Start[Cc]oroutine\s*\(\s*(\w*[Ss]hared\w*Coroutine\w*)\s*\(')
# Also catch general coroutine delegation like: hash => SomeNameCoroutine(hash, activateClass)
RE_COROUTINE_DELEGATE = re.compile(
    r'(?:hash|hashtable|_hashtable|_ht|ht)\s*=>\s*(\w+Coroutine)\s*\(')
# Catch ActivateCoroutine lambda delegate (outer-scoped shared: hashtable => ActivateCoroutine(...))
RE_ACTIVATE_COROUTINE_LAMBDA = re.compile(
    r'(?:hash|hashtable|_hashtable|_ht|ht)\s*=>\s*(ActivateCoroutine)\s*\(')

# ─── P5: New action patterns for stub reduction ──────────────────────

# Token play via CardEffectCommons helper methods
RE_PLAY_TOKEN = re.compile(r'CardEffectCommons\.Play(\w+)Token\s*\(')
# SelectAttackEffect — forced attack after selection
RE_SELECT_ATTACK = re.compile(r'SelectAttackEffect')
# CardEffectCommons.ChangeDigimonSAttack — SA modifier grant to target
RE_CHANGE_SA_TARGET = re.compile(
    r'CardEffectCommons\.ChangeDigimonSAttack\s*\([^)]*changeValue:\s*(-?\d+)')
# DisableEffectClass — effect invalidation
RE_DISABLE_EFFECT = re.compile(r'DisableEffectClass')
# HandBounceClass (note C# typo "Claass" in some files)
RE_HAND_BOUNCE_CLASS = re.compile(r'HandBounceCla+ss')
# AddEffectToPermanent — grants temporary effects to targets
RE_ADD_EFFECT_TO_PERM = re.compile(r'CardEffectCommons\.AddEffectToPermanent')
# CardEffectCommons.ChangeDigimonDP — DP change via helper (not caught by RE_CHANGE_DP)
RE_CHANGE_DP_COMMONS = re.compile(
    r'CardEffectCommons\.ChangeDigimonDP\s*\([^)]*changeValue:\s*(-?\d+)')
# IPutSecurityPermanent — place permanent into security
RE_PUT_SECURITY_PERM = re.compile(r'IPutSecurityPermanent|PutSecurityPermanent')

# ─── P6: Mode.Custom nested callback resolution ─────────────────────

# selectPermanentCoroutine parameter in Mode.Custom SetUp calls
RE_CUSTOM_CALLBACK = re.compile(
    r'selectPermanentCoroutine:\s*(\w+)')
# afterSelectPermanentCoroutine parameter
RE_AFTER_CUSTOM_CALLBACK = re.compile(
    r'afterSelectPermanentCoroutine:\s*(\w+)')

# ─── P7: Stub reduction — new helper class patterns ─────────────────

# WI 2: ChangeCostClass cost value extraction (targetCost/targetCount += N)
RE_CHANGE_COST_VALUE = re.compile(r'(?:targetCost|targetCount|Cost)\s*[-+]=\s*(\d+)')

# WI 3: Helper classes inside Mode.Custom callbacks
# IDegeneration — de-digivolve via helper class (also catches inline use)
RE_IDEGENERATION = re.compile(r'new\s+IDegeneration\s*\(\s*\w+\s*,\s*(\d+)')
# SwitchDefender — redirect attack target
RE_SWITCH_DEFENDER = re.compile(r'SwitchDefender\s*\(')
# PlayPermanentCards — play card via helper
RE_PLAY_PERMANENT_CARDS = re.compile(r'CardEffectCommons\.PlayPermanentCards\s*\(')
# DigivolveIntoHandOrTrashCard — digivolve from hand or trash
RE_DIGIVOLVE_INTO = re.compile(r'DigivolveIntoHandOrTrashCard\s*\(')

# WI 4: AddSkillClass — grants keywords to other permanents
RE_ADD_SKILL_CLASS = re.compile(r'AddSkillClass|SetUpAddSkillClass')

# WI 5: Metadata-only classes
RE_ADD_JOGRESS_LEVELS = re.compile(r'AddJogressLevelsClass|SetUpAddJogressLevelsClass')
RE_CHANGE_CARD_NAMES = re.compile(r'ChangeCardNamesClass|SetUpChangeCardNamesClass')
RE_CAN_ATTACK_TARGET = re.compile(r'CanAttackTargetDefendingPermanentClass')
RE_CAN_NOT_AFFECTED = re.compile(r'CanNotAffectedClass')

# ─── P8: Modifier interface patterns (Phase 2 — DCGO engine alignment) ──

# Continuous modifier classes from DCGO's CardEffectInterfaces.cs
# These map to our engine's ModifierRegistry system
RE_CANNOT_BE_DESTROYED = re.compile(
    r'CanNotBeDestroyedClass|ICanNotBeDestroyed|CanNotBeDestroyedByBattleClass')
RE_CANNOT_BE_SELECTED = re.compile(
    r'CanNotBeSelectedByEffectClass|ICanNotBeSelectedByEffect|IcecladEffect')
RE_CHANGE_PLAY_COST = re.compile(
    r'ChangeCostClass|ChangePlayCostStaticEffect\s*\([^)]*changeValue:\s*(-?\d+)')
RE_CHANGE_DIGI_COST_MODIFIER = re.compile(
    r'ChangeDigivolutionCostClass\s*\([^)]*changeValue:\s*(-?\d+)')
RE_CANNOT_UNSUSPEND = re.compile(
    r'CanNotUnsuspendClass|GainCanNotUnsuspend')
RE_CANNOT_ATTACK = re.compile(
    r'CanNotAttackClass|GainCanNotAttack(?:Player)?Effect')
RE_CANNOT_BLOCK = re.compile(
    r'CanNotBlockClass|GainCanNotBlock')
RE_GRANT_SECURITY_ATTACK_MOD = re.compile(
    r'ChangeSAttackClass|ChangeDigimonSAttack')
RE_IMMUNE_FROM_DP_MINUS = re.compile(
    r'ImmuneFromDPMinusClass|ImmuneFromDPMinus(?:Static)?Effect')
RE_CANNOT_BE_RETURNED = re.compile(
    r'CanNotReturnToHandClass|CanNotReturnToDeckClass|GainCanNotReturnToHand|GainCanNotReturnToDeck')
RE_DONT_BATTLE_SECURITY = re.compile(
    r'DontBattleSecurityDigimonClass|IDontBattleSecurityDigimon')

# Protection/immunity grant: "this Digimon isn't affected by opponent effects"
RE_EFFECT_IMMUNITY = re.compile(
    r'CanNotBeAffectedByOpponentEffect|CanNotAffectedClass|TopCard\.CanNotBeAffected')
# Retaliation target selection (select an opposing Digimon to delete)
RE_RETALIATION_TARGET = re.compile(
    r'RetaliationProcess|RetaliationCoroutine')
# Redirect attack to self (Decoy-like)
RE_REDIRECT_ATTACK_TO_SELF = re.compile(
    r'RedirectAttackToSelf|SwitchDefenderToSelf')
# End of attack modifier
RE_END_OF_ATTACK_MODIFIER = re.compile(
    r'EndOfAttackModifier|IsEndAttack\s*=\s*true')


# ─── Card selection filter patterns (CanSelectCardCondition body) ─────

RE_CF_EQUALS_TRAITS = re.compile(r'\.EqualsTraits\s*\(\s*"([^"]+)"\s*\)')
RE_CF_CONTAINS_TRAITS = re.compile(r'\.ContainsTraits\s*\(\s*"([^"]+)"\s*\)')
RE_CF_EQUALS_NAME = re.compile(r'\.EqualsCardName\s*\(\s*"([^"]+)"\s*\)')
RE_CF_CONTAINS_NAME = re.compile(r'\.ContainsCardName\s*\(\s*"([^"]+)"\s*\)')
RE_CF_COST_MAX = re.compile(r'(?:\.GetCostItself|\.BasePlayCostFromEntity)\s*<=?\s*(\d+)')
RE_CF_COST_MIN = re.compile(r'(?:\.GetCostItself|\.BasePlayCostFromEntity)\s*>=?\s*(\d+)')
RE_CF_LEVEL_MAX = re.compile(r'\.Level\s*<=?\s*(\d+)')
RE_CF_LEVEL_MIN = re.compile(r'\.Level\s*>=?\s*(\d+)')
RE_CF_IS_LEVEL = re.compile(r'\.IsLevel(\d+)')
RE_CF_COLOR = re.compile(r'CardColors\.Contains\s*\(\s*CardColor\.(\w+)\s*\)')
RE_CF_IS_DIGIMON = re.compile(r'\.\s*IsDigimon(?:\s|[&|;)\r\n])')
RE_CF_IS_TAMER = re.compile(r'\.\s*IsTamer(?:\s|[&|;)\r\n])')
RE_CF_IS_OPTION = re.compile(r'\.\s*IsOption(?:\s|[&|;)\r\n])')
# Pattern to strip C# lambda expressions (e.g., "x => x.IsDigimon") before
# kind-checking, to avoid false positives from nested Filter/Where lambdas
RE_CS_LAMBDA = re.compile(r'\w+\s*=>\s*\w+\.\w+')
RE_CF_NOT_DIGI_EGG = re.compile(r'!\s*\w+\.IsDigiEgg')
RE_CF_HAS_PLAY_COST = re.compile(r'\.HasPlayCost')
RE_CF_HAS_TRAITS = re.compile(r'\.\s*Has(\w+)Traits(?:\s|[&|;)\r\n])')
# Multi-pass reveal: extract (conditionName, mode) from SimplifiedSelectCardConditionClass entries
RE_REVEAL_PASS_ENTRY = re.compile(
    r'SimplifiedSelectCardConditionClass\s*\('
    r'[^)]*canTargetCondition\s*:\s*(\w+)'
    r'[^)]*mode\s*:\s*SelectCardEffect\.Mode\.(\w+)',
    re.DOTALL
)

# Maps C# Has*Traits property names to the trait strings they check.
HAS_TRAITS_MAP = {
    "CS": "CS",
    "TS": "TS",
    "Appmon": "Appmon",
    "Seekers": "SEEKERS",
    "RoyalKnight": "Royal Knight",
    "RoyalBase": "Royal Base",
    "SoC": "SoC",
    "Undead": "Undead",
    "Hudie": "Hudie",
    "Eater": "Eater",
    "SeaBeast": "Sea Beast",
    "Plant": "Plant",
    "Beast": "Beast",
    "Dragon": "Dragon",
    "Fairy": "Fairy",
    "Aqua": "Aqua",
    "DigiPolice": "DigiPolice",
    "Liberator": "LIBERATOR",
    "BanchoGang": "Bancho Gang",
    "Xros": "Xros Heart",
    "BlueFlare": "Blue Flare",
    "Twilight": "Twilight",
    "BaggaMilitia": "Bagra Army",
}
