#!/usr/bin/env python3
"""
Transpile DCGO C# card effect scripts into Python CardScript files.

Reads .cs files from the DCGO-Card-Scripts repo and generates
Python equivalents compatible with the digimon_gym engine.

Usage:
    python tools/transpile_dcgo.py <DCGO_DIR> <OUTPUT_DIR>
    python tools/transpile_dcgo.py /tmp/dcgo-scripts/CardEffect/BT14 digimon_gym/engine/data/scripts/bt14
    python tools/transpile_dcgo.py /tmp/dcgo-scripts/CardEffect/BT24 digimon_gym/engine/data/scripts/bt24

Strategy:
- Parse C# structurally using regex (not a full parser)
- Extract: timing blocks, effect metadata, conditions, actions
- Map DCGO patterns to Python ICardEffect properties
- Generate Python CardScript classes
"""

import os
import re
import sys
import json
from pathlib import Path
from dataclasses import dataclass, field
from typing import List, Optional, Dict, Tuple

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

# Regex patterns for C# parsing
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
RE_DRAW = re.compile(r'new DrawClass\s*\([^,]+,\s*(\d+)')
RE_ADD_MEMORY = re.compile(r'\.AddMemory\s*\(\s*(\d+)')
RE_CHANGE_DP = re.compile(r'ChangeDigimonDP\s*\([^,]*,\s*changeValue:\s*(-?\d+)')
RE_DELETE = re.compile(r'Mode\.Destroy|DestroyPermanentsClass')
RE_BOUNCE = re.compile(r'Mode\.Bounce')
RE_SUSPEND = re.compile(r'SuspendPermanentsClass|\.Tap\(\)')
RE_RECOVERY = re.compile(r'new IRecovery\s*\([^,]+,\s*(\d+)')
RE_PLAY_CARD = re.compile(r'PlayPermanentCards|PlayCardClass')
RE_TRASH_HAND = re.compile(r'Mode\.Discard|SelectHandEffect')
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

# DP value extraction from ChangeSelfDPStaticEffect
RE_FACTORY_DP_VALUE = re.compile(r'ChangeSelfDPStaticEffect\s*\(\s*(?:changeValue:\s*)?(-?\d+)')

# Security Attack modifier value extraction
RE_FACTORY_SA_VALUE = re.compile(r'ChangeSelfSAttackStaticEffect\s*\(\s*(?:changeValue:\s*)?(-?\d+)')

# Factory method patterns
RE_FACTORY_BLOCKER = re.compile(r'Blocker(?:Self)?StaticEffect')
RE_FACTORY_JAMMING = re.compile(r'Jamming(?:Self)?StaticEffect')
RE_FACTORY_RUSH = re.compile(r'Rush(?:Self)?Effect')
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
RE_FACTORY_COLLISION = re.compile(r'Collision(?:Self)?Effect')
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
RE_FACTORY_PROGRESS = re.compile(r'Progress(?:Self)?Effect')
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


@dataclass
class EffectBlock:
    """Represents one extracted effect from a timing block."""
    timing: str = ""
    effect_name: str = ""
    description: str = ""
    is_inherited: bool = False
    is_optional: bool = False
    max_count_per_turn: int = -1
    hash_string: str = ""
    is_factory: bool = False
    factory_method: str = ""
    actions: List[str] = field(default_factory=list)
    conditions: List[str] = field(default_factory=list)
    trait_checks: List[str] = field(default_factory=list)
    name_checks: List[str] = field(default_factory=list)
    color_checks: List[str] = field(default_factory=list)
    dp_change: Optional[int] = None
    draw_count: Optional[int] = None
    memory_gain: Optional[int] = None
    cost_reduction_val: Optional[int] = None
    recovery_count: Optional[int] = None
    raw_block: str = ""
    # Enhanced extraction fields for game helper method calls
    target_dp_limit: Optional[int] = None
    target_dp_min: Optional[int] = None
    target_level_limit: Optional[int] = None
    target_level_min: Optional[int] = None
    reveal_count: Optional[int] = None
    play_from_zone: Optional[str] = None
    play_free: bool = False
    digi_cost_override: Optional[int] = None
    digi_ignore_reqs: bool = False
    factory_dp_value: Optional[int] = None
    factory_sa_value: Optional[int] = None
    # Fix 4: De-digivolve count
    degen_count: Optional[int] = None
    # Fix 6: Trash-as-cost ordering
    is_trash_as_cost: bool = False
    # Fix 7: HasText checks (card text search, not name)
    has_text_checks: List[str] = field(default_factory=list)
    # Fix 1: Factory condition closure fields
    factory_cond_owner_turn: bool = False
    factory_cond_on_battle: bool = False
    factory_cond_digi_count: Optional[int] = None
    factory_cond_has_text: List[str] = field(default_factory=list)
    factory_cond_source_name: List[str] = field(default_factory=list)
    factory_cond_source_trait: List[str] = field(default_factory=list)
    factory_cond_perm_name: List[str] = field(default_factory=list)
    factory_cond_perm_trait: List[str] = field(default_factory=list)
    # Fix 5: Non-self DP (applies to all your Digimon)
    is_dp_all: bool = False
    # Fix 11: Destroy security count
    destroy_security_count: Optional[int] = None
    # Fix 10: CanActivateCondition fields for activate effects
    activate_cond_digi_count: Optional[int] = None
    activate_cond_source_name: List[str] = field(default_factory=list)
    activate_cond_source_trait: List[str] = field(default_factory=list)
    activate_cond_has_text: List[str] = field(default_factory=list)
    activate_cond_perm_name: List[str] = field(default_factory=list)


def extract_timing_blocks(source: str) -> List[Tuple[str, str]]:
    """Extract (timing, block_content) pairs from C# source."""
    blocks = []
    # Find each timing check and its associated block
    for match in RE_TIMING_BLOCK.finditer(source):
        timing = match.group(1)
        start = match.end()
        # Find the matching brace block
        depth = 0
        block_start = None
        for i in range(start, len(source)):
            if source[i] == '{':
                if block_start is None:
                    block_start = i
                depth += 1
            elif source[i] == '}':
                depth -= 1
                if depth == 0:
                    blocks.append((f"EffectTiming.{timing}" if not timing.startswith("EffectTiming.") else timing,
                                   source[block_start:i+1]))
                    break
    return blocks


def _extract_factory_conditions(block: str, eb: EffectBlock):
    """Fix 1: Extract condition closure body for factory effects.

    Scans block for CanActivateCondition/Condition closures and extracts
    checks like IsOwnerTurn, DigivolutionCards.Count, HasText, etc.
    """
    if RE_COND_OWNER_TURN.search(block):
        eb.factory_cond_owner_turn = True
    if RE_COND_ON_BATTLE.search(block):
        eb.factory_cond_on_battle = True

    # DigivolutionCards.Count >= N (plain count, no predicate)
    m = RE_FACTORY_COND_DIGI_COUNT.search(block)
    if m:
        eb.factory_cond_digi_count = int(m.group(1))

    # HasText("X") — checks card text field
    for m in RE_COND_HAS_TEXT.finditer(block):
        if m.group(1) not in eb.factory_cond_has_text:
            eb.factory_cond_has_text.append(m.group(1))

    # DigivolutionCards.Count(... EqualsCardName("X") ...) — source name check
    for m in RE_FACTORY_COND_SOURCE_NAME.finditer(block):
        if m.group(1) not in eb.factory_cond_source_name:
            eb.factory_cond_source_name.append(m.group(1))

    # DigivolutionCards.Count(... EqualsTraits("X") ...) — source trait check
    for m in RE_FACTORY_COND_SOURCE_TRAIT.finditer(block):
        if m.group(1) not in eb.factory_cond_source_trait:
            eb.factory_cond_source_trait.append(m.group(1))

    # TopCard.EqualsCardName("X") — permanent name
    for m in RE_FACTORY_COND_PERM_NAME.finditer(block):
        if m.group(1) not in eb.factory_cond_perm_name:
            eb.factory_cond_perm_name.append(m.group(1))

    # TopCard.EqualsTraits("X") — permanent trait
    for m in RE_FACTORY_COND_PERM_TRAIT.finditer(block):
        if m.group(1) not in eb.factory_cond_perm_trait:
            eb.factory_cond_perm_trait.append(m.group(1))

    # HasRoyalKnightTraits (Fix 8)
    if RE_COND_ROYAL_KNIGHT.search(block):
        if "Royal Knight" not in eb.factory_cond_perm_trait:
            eb.factory_cond_perm_trait.append("Royal Knight")


def extract_factory_effects(block: str) -> List[EffectBlock]:
    """Extract factory method calls from a block."""
    effects = []

    factories = [
        (RE_FACTORY_BLOCKER, "blocker", "Blocker"),
        (RE_FACTORY_JAMMING, "jamming", "Jamming"),
        (RE_FACTORY_RUSH, "rush", "Rush"),
        (RE_FACTORY_REBOOT, "reboot", "Reboot"),
        (RE_FACTORY_RAID, "raid", "Raid"),
        (RE_FACTORY_ALLIANCE, "alliance", "Alliance"),
        (RE_FACTORY_SEC_PLAY, "security_play", "Security: Play this card"),
        (RE_FACTORY_SA_PLUS, "security_attack_plus", "Security Attack +1"),
        (RE_FACTORY_DP, "dp_modifier", "DP modifier"),
        (RE_FACTORY_ARMOR_PURGE, "armor_purge", "Armor Purge"),
        (RE_FACTORY_BLAST_DIGI, "blast_digivolve", "Blast Digivolve"),
        (RE_FACTORY_SET_MEM_3, "set_memory_3", "Set memory to 3"),
        (RE_FACTORY_GAIN_MEM, "gain_memory_tamer", "Gain 1 memory (Tamer)"),
        (RE_FACTORY_ADD_DIGI_REQ, "alt_digivolve_req", "Alternate digivolution requirement"),
        (RE_FACTORY_CHANGE_DIGI_COST, "change_digi_cost", "Change digivolution cost"),
        # Fix 11: Missing factory keywords
        (RE_FACTORY_PIERCING, "piercing", "Piercing"),
        (RE_FACTORY_COLLISION, "collision", "Collision"),
        (RE_FACTORY_BLITZ, "blitz", "Blitz"),
        (RE_FACTORY_FORTITUDE, "fortitude", "Fortitude"),
        (RE_FACTORY_EVADE, "evade", "Evade"),
        (RE_FACTORY_BARRIER, "barrier", "Barrier"),
        (RE_FACTORY_DECOY, "decoy", "Decoy"),
        (RE_FACTORY_RETALIATION, "retaliation", "Retaliation"),
        (RE_FACTORY_SAVE, "save", "Save"),
        (RE_FACTORY_MATERIAL_SAVE, "material_save", "Material Save"),
        (RE_FACTORY_OVERCLOCK, "overclock", "Overclock"),
        (RE_FACTORY_VORTEX, "vortex", "Vortex"),
        (RE_FACTORY_TRAINING, "training", "Training"),
        (RE_FACTORY_PROGRESS, "progress", "Progress"),
        # Fix 12: Additional keywords from rules evaluation
        (RE_FACTORY_DIGISORPTION, "digisorption", "Digisorption"),
        (RE_FACTORY_DIGIBURST, "digiburst", "Digi-Burst"),
        (RE_FACTORY_DELAY, "delay", "Delay"),
        (RE_FACTORY_PARTITION, "partition", "Partition"),
        (RE_FACTORY_DIGIXROS, "digixros", "DigiXros"),
        (RE_FACTORY_SCAPEGOAT, "scapegoat", "Scapegoat"),
        (RE_FACTORY_DECODE, "decode", "Decode"),
        (RE_FACTORY_ICECLAD, "iceclad", "Iceclad"),
        (RE_FACTORY_FRAGMENT, "fragment", "Fragment"),
        (RE_FACTORY_EXECUTE, "execute", "Execute"),
    ]

    for regex, method, desc in factories:
        if regex.search(block):
            eb = EffectBlock(
                is_factory=True,
                factory_method=method,
                description=desc,
            )
            # Check if inherited
            inh = RE_SET_INHERITED.search(block)
            if inh:
                eb.is_inherited = (inh.group(1) or inh.group(2)) == "true"

            # Fix 1: Extract factory condition closures
            _extract_factory_conditions(block, eb)

            # Extract DP value for dp_modifier factory
            if method == "dp_modifier":
                m_val = RE_FACTORY_DP_VALUE.search(block)
                if m_val:
                    eb.factory_dp_value = int(m_val.group(1))
            # Extract SA value for security_attack_plus factory
            if method == "security_attack_plus":
                m_val = RE_FACTORY_SA_VALUE.search(block)
                if m_val:
                    eb.factory_sa_value = int(m_val.group(1))
            # Extract digivolve requirement details
            if method == "alt_digivolve_req":
                m_cost = RE_FACTORY_DIGI_REQ_COST.search(block)
                if m_cost:
                    eb.digi_cost_override = int(m_cost.group(1))
                m_name = RE_FACTORY_DIGI_REQ_NAME.search(block)
                if m_name:
                    eb.name_checks.append(m_name.group(1))
                m_trait = RE_FACTORY_DIGI_REQ_TRAIT.search(block)
                if m_trait:
                    eb.trait_checks.append(m_trait.group(1))
            # Extract digivolve cost change value
            if method == "change_digi_cost":
                m_val = RE_FACTORY_CHANGE_DIGI_COST_VALUE.search(block)
                if m_val:
                    eb.cost_reduction_val = int(m_val.group(1))
                # Extract trait/name conditions for what this cost change applies to
                for m_t in RE_COND_TRAIT.finditer(block):
                    eb.trait_checks.append(m_t.group(1))
                for m_n in RE_COND_NAME.finditer(block):
                    eb.name_checks.append(m_n.group(1))
                eb.trait_checks = list(dict.fromkeys(eb.trait_checks))
                eb.name_checks = list(dict.fromkeys(eb.name_checks))
            effects.append(eb)

    # Fix 5: Handle ChangeDPStaticEffect (non-self, applies to all your Digimon)
    if RE_FACTORY_DP_ALL.search(block) and not RE_FACTORY_DP.search(block):
        eb = EffectBlock(
            is_factory=True,
            factory_method="dp_modifier_all",
            description="All your Digimon DP modifier",
            is_dp_all=True,
        )
        inh = RE_SET_INHERITED.search(block)
        if inh:
            eb.is_inherited = (inh.group(1) or inh.group(2)) == "true"
        m_val = RE_FACTORY_DP_ALL_VALUE.search(block)
        if m_val:
            eb.factory_dp_value = int(m_val.group(1))
        _extract_factory_conditions(block, eb)
        effects.append(eb)

    return effects


def extract_activate_effects(block: str) -> List[EffectBlock]:
    """Extract ActivateClass-based effects from a block."""
    effects = []

    # Split on ActivateClass instantiations
    activate_splits = re.split(r'(ActivateClass\s+\w+\s*=\s*new\s+ActivateClass\s*\(\s*\)\s*;)', block)

    i = 0
    while i < len(activate_splits):
        segment = activate_splits[i]
        if 'new ActivateClass' in segment and i + 1 < len(activate_splits):
            # Combine the instantiation line with the following block
            full_block = segment + activate_splits[i + 1]
            i += 2
        else:
            full_block = segment
            i += 1

        if 'ActivateClass' not in full_block and 'SetUpICardEffect' not in full_block:
            continue

        eb = EffectBlock(raw_block=full_block)

        # Extract metadata
        m = RE_EFFECT_NAME.search(full_block)
        if m:
            eb.effect_name = m.group(1)

        descs = RE_EFFECT_DESC.findall(full_block)
        if descs:
            eb.description = descs[0]

        m = RE_SET_INHERITED.search(full_block)
        if m:
            eb.is_inherited = (m.group(1) or m.group(2)) == "true"

        m = RE_HASH_STRING.search(full_block)
        if m:
            eb.hash_string = m.group(1)

        m = RE_MAX_COUNT.search(full_block)
        if m:
            eb.max_count_per_turn = int(m.group(1))

        m = RE_IS_OPTIONAL.search(full_block)
        if m:
            eb.is_optional = m.group(1) == "true"

        # Extract conditions
        if RE_COND_ON_BATTLE.search(full_block):
            eb.conditions.append("on_battle_area")
        if RE_COND_OWNER_TURN.search(full_block):
            eb.conditions.append("your_turn")
        if RE_COND_ON_PLAY.search(full_block):
            eb.conditions.append("trigger_on_play")
        if RE_COND_ON_ATTACK.search(full_block):
            eb.conditions.append("trigger_on_attack")
        if RE_COND_ON_DELETION.search(full_block):
            eb.conditions.append("trigger_on_deletion")
        if RE_COND_WHEN_DIGI.search(full_block):
            eb.conditions.append("trigger_when_digivolving")
        if RE_COND_SEC_EFFECT.search(full_block):
            eb.conditions.append("trigger_security")
        if RE_COND_OPTION_MAIN.search(full_block):
            eb.conditions.append("trigger_option_main")

        for m in RE_COND_TRAIT.finditer(full_block):
            eb.trait_checks.append(m.group(1))
        for m in RE_COND_NAME.finditer(full_block):
            eb.name_checks.append(m.group(1))
        for m in RE_COND_COLOR.finditer(full_block):
            eb.color_checks.append(m.group(1))

        # Fix 7: HasText checks
        for m in RE_COND_HAS_TEXT.finditer(full_block):
            if m.group(1) not in eb.has_text_checks:
                eb.has_text_checks.append(m.group(1))

        # Fix 8: HasRoyalKnightTraits
        if RE_COND_ROYAL_KNIGHT.search(full_block):
            if "Royal Knight" not in eb.trait_checks:
                eb.trait_checks.append("Royal Knight")

        # Deduplicate
        eb.trait_checks = list(dict.fromkeys(eb.trait_checks))
        eb.name_checks = list(dict.fromkeys(eb.name_checks))
        eb.color_checks = list(dict.fromkeys(eb.color_checks))

        # Extract actions
        m = RE_DRAW.search(full_block)
        if m:
            eb.draw_count = int(m.group(1))
            eb.actions.append("draw")

        m = RE_ADD_MEMORY.search(full_block)
        if m:
            eb.memory_gain = int(m.group(1))
            eb.actions.append("gain_memory")

        m = RE_CHANGE_DP.search(full_block)
        if m:
            eb.dp_change = int(m.group(1))
            eb.actions.append("change_dp")

        m = RE_RECOVERY.search(full_block)
        if m:
            eb.recovery_count = int(m.group(1))
            eb.actions.append("recovery")

        if RE_DELETE.search(full_block):
            eb.actions.append("delete")
        if RE_BOUNCE.search(full_block):
            eb.actions.append("bounce")
        if RE_SUSPEND.search(full_block):
            eb.actions.append("suspend")
        if RE_PLAY_CARD.search(full_block):
            eb.actions.append("play_card")
        if RE_TRASH_HAND.search(full_block):
            eb.actions.append("trash_from_hand")
        if RE_TRASH_DIGI.search(full_block):
            eb.actions.append("trash_digivolution_cards")
        if RE_ADD_TO_HAND.search(full_block):
            eb.actions.append("add_to_hand")
        if RE_ADD_SECURITY.search(full_block):
            eb.actions.append("add_to_security")
        if RE_REVEAL.search(full_block):
            eb.actions.append("reveal_and_select")
        if RE_DEGENERATION.search(full_block):
            eb.actions.append("de_digivolve")
        if RE_DIGIVOLVE.search(full_block):
            eb.actions.append("digivolve")
        if RE_COST_REDUCTION.search(full_block):
            eb.actions.append("cost_reduction")
            m2 = re.search(r'Cost\s*-=\s*(\d+)', full_block)
            if m2:
                eb.cost_reduction_val = int(m2.group(1))
        if RE_MIND_LINK.search(full_block):
            eb.actions.append("mind_link")

        # Fix 11: Additional action patterns from ActivateCoroutine bodies
        # SelectPermanentEffect.Mode.* implies specific actions
        for m_mode in RE_SELECT_PERM_MODE.finditer(full_block):
            mode = m_mode.group(1)
            if mode == "Destroy" and "delete" not in eb.actions:
                eb.actions.append("delete")
            elif mode == "Tap" and "suspend" not in eb.actions:
                eb.actions.append("suspend")
            elif mode in ("Bounce", "PutLibraryBottom") and "bounce" not in eb.actions:
                eb.actions.append("bounce")
            elif mode == "UnTap" and "unsuspend" not in eb.actions:
                eb.actions.append("unsuspend")

        # IDestroySecurity — trash opponent security
        m_ds = RE_DESTROY_SECURITY.search(full_block)
        if m_ds:
            eb.actions.append("destroy_security")
            eb.destroy_security_count = int(m_ds.group(1))
        elif RE_REDUCE_SECURITY.search(full_block):
            eb.actions.append("destroy_security")

        # IUnsuspendPermanents
        if RE_UNSUSPEND.search(full_block) and "unsuspend" not in eb.actions:
            eb.actions.append("unsuspend")

        # Attack restriction effects
        if RE_RESTRICT_ATTACK.search(full_block):
            eb.actions.append("restrict_attack")
        if RE_TARGET_LOCK.search(full_block):
            eb.actions.append("target_lock")

        # Security flip
        if RE_FLIP_SECURITY.search(full_block):
            eb.actions.append("flip_security")

        # Return to deck bottom
        if RE_RETURN_DECK_BOTTOM.search(full_block) and "bounce" not in eb.actions:
            eb.actions.append("return_to_deck")

        # DNA/Jogress digivolution
        if RE_JOGRESS.search(full_block):
            eb.actions.append("jogress_condition")

        # ── Enhanced extraction for game helper method calls ──

        # Target DP/level limits (for delete, bounce, suspend filters)
        m = RE_TARGET_DP_LIMIT.search(full_block)
        if m:
            eb.target_dp_limit = int(m.group(1))
        m = RE_TARGET_DP_MIN.search(full_block)
        if m:
            eb.target_dp_min = int(m.group(1))
        m = RE_TARGET_LEVEL_LIMIT.search(full_block)
        if m:
            eb.target_level_limit = int(m.group(1))
        m = RE_TARGET_LEVEL_MIN.search(full_block)
        if m:
            eb.target_level_min = int(m.group(1))

        # Reveal count
        m = RE_REVEAL_COUNT.search(full_block)
        if m:
            eb.reveal_count = int(m.group(1) or m.group(2))

        # Play from zone
        if RE_PLAY_FROM_TRASH.search(full_block):
            eb.play_from_zone = 'trash'
        if RE_PLAY_FREE.search(full_block):
            eb.play_free = True

        # Digivolve details
        m = RE_DIGI_COST_FIXED.search(full_block)
        if m:
            eb.digi_cost_override = int(m.group(1))
        if RE_DIGI_IGNORE_REQS.search(full_block):
            eb.digi_ignore_reqs = True

        # Fix 4: De-digivolve count
        m = RE_DEGEN_COUNT.search(full_block)
        if m:
            eb.degen_count = int(m.group(1))

        # Fix 6: Detect trash-as-cost pattern
        if ("trash_from_hand" in eb.actions and
                ("draw" in eb.actions or "gain_memory" in eb.actions)):
            desc_lower = eb.description.lower()
            if "by trashing" in desc_lower or "by discarding" in desc_lower:
                eb.is_trash_as_cost = True

        # Fix 10: Extract CanActivateCondition patterns
        _extract_activate_conditions(full_block, eb)

        effects.append(eb)

    return effects


def _extract_activate_conditions(block: str, eb: EffectBlock):
    """Fix 10: Extract conditions from CanActivateCondition closures."""
    # DigivolutionCards.Count >= N
    m = RE_FACTORY_COND_DIGI_COUNT.search(block)
    if m:
        eb.activate_cond_digi_count = int(m.group(1))

    # DigivolutionCards.Count(... EqualsCardName("X") ...)
    for m in RE_FACTORY_COND_SOURCE_NAME.finditer(block):
        if m.group(1) not in eb.activate_cond_source_name:
            eb.activate_cond_source_name.append(m.group(1))

    # DigivolutionCards.Count(... EqualsTraits("X") ...)
    for m in RE_FACTORY_COND_SOURCE_TRAIT.finditer(block):
        if m.group(1) not in eb.activate_cond_source_trait:
            eb.activate_cond_source_trait.append(m.group(1))

    # HasText("X")
    for m in RE_COND_HAS_TEXT.finditer(block):
        if m.group(1) not in eb.activate_cond_has_text:
            eb.activate_cond_has_text.append(m.group(1))

    # TopCard.EqualsCardName("X")
    for m in RE_FACTORY_COND_PERM_NAME.finditer(block):
        if m.group(1) not in eb.activate_cond_perm_name:
            eb.activate_cond_perm_name.append(m.group(1))


def parse_cs_file(filepath: str) -> Tuple[str, List[EffectBlock]]:
    """Parse a C# card effect file. Returns (class_name, effects)."""
    with open(filepath, 'r', encoding='utf-8', errors='replace') as f:
        source = f.read()

    m = RE_CLASS.search(source)
    class_name = m.group(1) if m else os.path.basename(filepath).replace('.cs', '')

    all_effects = []

    # Extract timing blocks
    timing_blocks = extract_timing_blocks(source)

    for timing, block in timing_blocks:
        # Check for factory effects first
        factory_effects = extract_factory_effects(block)
        for fe in factory_effects:
            fe.timing = timing
            all_effects.append(fe)

        # Then ActivateClass effects
        activate_effects = extract_activate_effects(block)
        for ae in activate_effects:
            ae.timing = timing
            all_effects.append(ae)

    # Also check for effects defined outside timing blocks (top-level)
    if not timing_blocks:
        factory_effects = extract_factory_effects(source)
        activate_effects = extract_activate_effects(source)
        for fe in factory_effects:
            all_effects.append(fe)
        for ae in activate_effects:
            all_effects.append(ae)

    return class_name, all_effects


# ─── Python Code Generation ─────────────────────────────────────────

def generate_condition_code(eb: EffectBlock, indent: str = "            ") -> str:
    """Generate Python condition function code from extracted conditions."""
    checks = []

    if "on_battle_area" in eb.conditions:
        checks.append(f"{indent}if card and card.permanent_of_this_card() is None:")
        checks.append(f"{indent}    return False")

    if "your_turn" in eb.conditions:
        checks.append(f"{indent}if not (card and card.owner and card.owner.is_my_turn):")
        checks.append(f"{indent}    return False")

    if "trigger_on_play" in eb.conditions:
        checks.append(f"{indent}# Triggered on play — validated by engine timing")

    if "trigger_on_attack" in eb.conditions:
        checks.append(f"{indent}# Triggered on attack — validated by engine timing")

    if "trigger_on_deletion" in eb.conditions:
        checks.append(f"{indent}# Triggered on deletion — validated by engine timing")

    if "trigger_when_digivolving" in eb.conditions:
        checks.append(f"{indent}# Triggered when digivolving — validated by engine timing")

    if "trigger_security" in eb.conditions:
        checks.append(f"{indent}# Security effect — validated by engine timing")

    if "trigger_option_main" in eb.conditions:
        checks.append(f"{indent}# Option main effect — validated by engine timing")

    # Fix 10: Activate condition checks from CanActivateCondition
    if eb.activate_cond_has_text:
        or_parts = " or ".join(f"'{t}' in text" for t in eb.activate_cond_has_text)
        checks.append(f"{indent}permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None")
        checks.append(f"{indent}if permanent and permanent.top_card:")
        checks.append(f"{indent}    text = permanent.top_card.card_text")
        checks.append(f"{indent}    if not ({or_parts}):")
        checks.append(f"{indent}        return False")
        checks.append(f"{indent}else:")
        checks.append(f"{indent}    return False")

    if eb.activate_cond_digi_count is not None:
        checks.append(f"{indent}permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None")
        checks.append(f"{indent}if not (permanent and len(permanent.digivolution_cards) >= {eb.activate_cond_digi_count}):")
        checks.append(f"{indent}    return False")

    if eb.activate_cond_source_name:
        or_parts = " or ".join(
            f"src.contains_card_name('{n}')" for n in eb.activate_cond_source_name)
        checks.append(f"{indent}permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None")
        checks.append(f"{indent}if permanent:")
        checks.append(f"{indent}    if not any({or_parts} for src in permanent.digivolution_cards):")
        checks.append(f"{indent}        return False")
        checks.append(f"{indent}else:")
        checks.append(f"{indent}    return False")

    if eb.activate_cond_perm_name:
        or_parts = " or ".join(
            f"permanent.contains_card_name('{n}')" for n in eb.activate_cond_perm_name)
        checks.append(f"{indent}permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None")
        checks.append(f"{indent}if not (permanent and ({or_parts})):")
        checks.append(f"{indent}    return False")

    if not checks:
        return f"{indent}return True"

    checks.append(f"{indent}return True")
    return "\n".join(checks)


def generate_action_comment(eb: EffectBlock) -> str:
    """Generate a comment describing what the effect does."""
    parts = []
    if eb.draw_count:
        parts.append(f"Draw {eb.draw_count}")
    if eb.memory_gain:
        parts.append(f"Gain {eb.memory_gain} memory")
    if eb.dp_change:
        parts.append(f"DP {eb.dp_change:+d}")
    if eb.recovery_count:
        parts.append(f"Recovery +{eb.recovery_count}")
    if eb.cost_reduction_val:
        parts.append(f"Cost -{eb.cost_reduction_val}")
    for action in eb.actions:
        if action not in ("draw", "gain_memory", "change_dp", "recovery", "cost_reduction"):
            name = action.replace("_", " ").title()
            parts.append(name)
    return ", ".join(parts) if parts else "Effect"


def _build_target_filter_code(eb: EffectBlock, indent: str = "            ",
                               perm_only: bool = False) -> List[str]:
    """Build filter condition lines for opponent permanent targeting.

    Fix 3: When perm_only=True, only include DP/level checks, not name/trait
    checks (which belong to card selection filters in multi-action blocks).
    """
    parts = []
    if eb.target_dp_limit is not None:
        parts.append(f"{indent}    if p.dp is None or p.dp > {eb.target_dp_limit}:")
        parts.append(f"{indent}        return False")
    if eb.target_dp_min is not None:
        parts.append(f"{indent}    if p.dp is None or p.dp < {eb.target_dp_min}:")
        parts.append(f"{indent}        return False")
    if eb.target_level_limit is not None:
        parts.append(f"{indent}    if p.level is None or p.level > {eb.target_level_limit}:")
        parts.append(f"{indent}        return False")
    if eb.target_level_min is not None:
        parts.append(f"{indent}    if p.level is None or p.level < {eb.target_level_min}:")
        parts.append(f"{indent}        return False")
    if not perm_only:
        # Fix 2: Use OR logic for combined name/trait checks
        if eb.name_checks or eb.trait_checks:
            parts.extend(_build_or_filter_perm(eb.name_checks, eb.trait_checks, indent))
    return parts


def _build_card_filter_code(eb: EffectBlock, indent: str = "            ") -> List[str]:
    """Build filter condition lines for card selection (hand/reveal/trash).

    Fix 2: Uses OR logic for name/trait checks instead of AND.
    """
    parts = []
    if eb.name_checks or eb.trait_checks:
        parts.extend(_build_or_filter_card(eb.name_checks, eb.trait_checks, indent))
    if eb.target_level_limit is not None:
        parts.append(f"{indent}    if getattr(c, 'level', None) is None or c.level > {eb.target_level_limit}:")
        parts.append(f"{indent}        return False")
    if eb.target_level_min is not None:
        parts.append(f"{indent}    if getattr(c, 'level', None) is None or c.level < {eb.target_level_min}:")
        parts.append(f"{indent}        return False")
    return parts


def _build_or_filter_card(name_checks: List[str], trait_checks: List[str],
                           indent: str) -> List[str]:
    """Fix 2: Build OR-combined name/trait filter for card selection."""
    parts = []
    or_clauses = []
    if name_checks:
        name_ors = " or ".join(f"'{n}' in _n" for n in name_checks)
        or_clauses.append(f"any({name_ors} for _n in getattr(c, 'card_names', []))")
    if trait_checks:
        trait_ors = " or ".join(f"'{t}' in _t" for t in trait_checks)
        or_clauses.append(f"any({trait_ors} for _t in (getattr(c, 'card_traits', []) or []))")
    if or_clauses:
        combined = " or ".join(or_clauses)
        parts.append(f"{indent}    if not ({combined}):")
        parts.append(f"{indent}        return False")
    return parts


def _build_or_filter_perm(name_checks: List[str], trait_checks: List[str],
                           indent: str) -> List[str]:
    """Fix 2: Build OR-combined name/trait filter for permanent targeting."""
    parts = []
    or_clauses = []
    for name in name_checks:
        or_clauses.append(f"p.contains_card_name('{name}')")
    for trait in trait_checks:
        or_clauses.append(f"any('{trait}' in t for t in (getattr(p.top_card, 'card_traits', []) or []))")
    if or_clauses:
        combined = " or ".join(or_clauses)
        parts.append(f"{indent}    if not ({combined}):")
        parts.append(f"{indent}        return False")
    return parts


def generate_callback_code(eb: EffectBlock, indent: str = "            ") -> str:
    """Generate the on_process_callback body with real engine calls."""
    lines = []
    lines.append(f"{indent}player = ctx.get('player')")
    lines.append(f"{indent}perm = ctx.get('permanent')")
    lines.append(f"{indent}game = ctx.get('game')")

    # Fix 3: Determine if this is a multi-action block where delete and card
    # selection actions coexist — need to separate their filters.
    has_perm_action = any(a in eb.actions for a in ("delete", "bounce", "suspend"))
    has_card_action = any(a in eb.actions for a in (
        "play_card", "digivolve", "reveal_and_select", "trash_from_hand"))
    use_perm_only = has_perm_action and has_card_action

    # Fix 6: Handle trash-as-cost ordering.
    # When trash is a cost (e.g. "By trashing X, draw Y"), emit trash first.
    if eb.is_trash_as_cost:
        # Emit trash_from_hand first
        _emit_trash_from_hand(eb, lines, indent)
        # Then draw/memory as reward
        if eb.draw_count:
            lines.append(f"{indent}if player:")
            lines.append(f"{indent}    player.draw_cards({eb.draw_count})")
        if eb.memory_gain:
            lines.append(f"{indent}if player:")
            lines.append(f"{indent}    player.add_memory({eb.memory_gain})")
        # Handle remaining actions (skip draw, gain_memory, trash_from_hand)
        for action in eb.actions:
            if action in ("draw", "gain_memory", "trash_from_hand"):
                continue
            _emit_action(eb, action, lines, indent, use_perm_only)
    else:
        # Normal ordering: draw/memory/dp/recovery first, then other actions
        if eb.draw_count:
            lines.append(f"{indent}if player:")
            lines.append(f"{indent}    player.draw_cards({eb.draw_count})")
        if eb.memory_gain:
            lines.append(f"{indent}if player:")
            lines.append(f"{indent}    player.add_memory({eb.memory_gain})")
        if eb.dp_change:
            if eb.dp_change < 0:
                lines.append(f"{indent}# DP change targets opponent digimon")
                lines.append(f"{indent}enemy = player.enemy if player else None")
                lines.append(f"{indent}if enemy and enemy.battle_area:")
                lines.append(f"{indent}    dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]")
                lines.append(f"{indent}    if dp_targets:")
                lines.append(f"{indent}        target = min(dp_targets, key=lambda p: p.dp)")
                lines.append(f"{indent}        target.change_dp({eb.dp_change})")
            else:
                lines.append(f"{indent}if perm:")
                lines.append(f"{indent}    perm.change_dp({eb.dp_change})")
        if eb.recovery_count:
            lines.append(f"{indent}if player:")
            lines.append(f"{indent}    player.recovery({eb.recovery_count})")

        for action in eb.actions:
            if action in ("draw", "gain_memory", "change_dp", "recovery"):
                continue
            _emit_action(eb, action, lines, indent, use_perm_only)

    preamble_lines = {
        f"player = ctx.get('player')",
        f"perm = ctx.get('permanent')",
        f"game = ctx.get('game')",
    }
    if not any(l.strip() not in preamble_lines and l.strip() for l in lines):
        lines.append(f"{indent}pass")

    return "\n".join(lines)


def _emit_trash_from_hand(eb: EffectBlock, lines: List[str], indent: str):
    """Emit trash_from_hand action code."""
    lines.append(f"{indent}if not (player and game):")
    lines.append(f"{indent}    return")
    card_filter = _build_card_filter_code(eb, indent)
    lines.append(f"{indent}def hand_filter(c):")
    if card_filter:
        lines.extend(card_filter)
        lines.append(f"{indent}    return True")
    else:
        lines.append(f"{indent}    return True")
    lines.append(f"{indent}def on_trashed(selected):")
    lines.append(f"{indent}    if selected in player.hand_cards:")
    lines.append(f"{indent}        player.hand_cards.remove(selected)")
    lines.append(f"{indent}        player.trash_cards.append(selected)")
    lines.append(f"{indent}game.effect_select_hand_card(")
    lines.append(f"{indent}    player, hand_filter, on_trashed, is_optional={eb.is_optional})")


def _emit_action(eb: EffectBlock, action: str, lines: List[str], indent: str,
                  use_perm_only: bool = False):
    """Emit code for a single action type."""
    if action == "delete":
        lines.append(f"{indent}if not (player and game):")
        lines.append(f"{indent}    return")
        # Fix 3: When multi-action, only use DP/level filters for delete
        target_filter = _build_target_filter_code(eb, indent, perm_only=use_perm_only)
        lines.append(f"{indent}def target_filter(p):")
        if target_filter:
            lines.extend(target_filter)
            lines.append(f"{indent}    return p.is_digimon")
        else:
            lines.append(f"{indent}    return p.is_digimon")
        lines.append(f"{indent}def on_delete(target_perm):")
        lines.append(f"{indent}    enemy = player.enemy if player else None")
        lines.append(f"{indent}    if enemy:")
        lines.append(f"{indent}        enemy.delete_permanent(target_perm)")
        lines.append(f"{indent}game.effect_select_opponent_permanent(")
        lines.append(f"{indent}    player, on_delete, filter_fn=target_filter, is_optional={eb.is_optional})")
    elif action == "bounce":
        lines.append(f"{indent}if not (player and game):")
        lines.append(f"{indent}    return")
        target_filter = _build_target_filter_code(eb, indent, perm_only=use_perm_only)
        lines.append(f"{indent}def target_filter(p):")
        if target_filter:
            lines.extend(target_filter)
            lines.append(f"{indent}    return True")
        else:
            lines.append(f"{indent}    return True")
        lines.append(f"{indent}def on_bounce(target_perm):")
        lines.append(f"{indent}    enemy = player.enemy if player else None")
        lines.append(f"{indent}    if enemy:")
        lines.append(f"{indent}        enemy.bounce_permanent_to_hand(target_perm)")
        lines.append(f"{indent}game.effect_select_opponent_permanent(")
        lines.append(f"{indent}    player, on_bounce, filter_fn=target_filter, is_optional={eb.is_optional})")
    elif action == "suspend":
        lines.append(f"{indent}if not (player and game):")
        lines.append(f"{indent}    return")
        target_filter = _build_target_filter_code(eb, indent, perm_only=use_perm_only)
        lines.append(f"{indent}def target_filter(p):")
        if target_filter:
            lines.extend(target_filter)
            lines.append(f"{indent}    return True")
        else:
            lines.append(f"{indent}    return True")
        lines.append(f"{indent}def on_suspend(target_perm):")
        lines.append(f"{indent}    target_perm.suspend()")
        lines.append(f"{indent}game.effect_select_opponent_permanent(")
        lines.append(f"{indent}    player, on_suspend, filter_fn=target_filter, is_optional={eb.is_optional})")
    elif action == "trash_from_hand":
        _emit_trash_from_hand(eb, lines, indent)
    elif action == "trash_digivolution_cards":
        lines.append(f"{indent}# Trash digivolution cards from this permanent")
        lines.append(f"{indent}if perm and not perm.has_no_digivolution_cards:")
        lines.append(f"{indent}    trashed = perm.trash_digivolution_cards(1)")
        lines.append(f"{indent}    if player:")
        lines.append(f"{indent}        player.trash_cards.extend(trashed)")
    elif action == "add_to_hand":
        lines.append(f"{indent}# Add card to hand (from trash/reveal)")
        lines.append(f"{indent}if player and player.trash_cards:")
        lines.append(f"{indent}    card_to_add = player.trash_cards.pop()")
        lines.append(f"{indent}    player.hand_cards.append(card_to_add)")
    elif action == "add_to_security":
        lines.append(f"{indent}# Add top card of deck to security")
        lines.append(f"{indent}if player:")
        lines.append(f"{indent}    player.recovery(1)")
    elif action == "play_card":
        lines.append(f"{indent}if not (player and game):")
        lines.append(f"{indent}    return")
        zone = eb.play_from_zone or 'hand'
        card_filter = _build_card_filter_code(eb, indent)
        lines.append(f"{indent}def play_filter(c):")
        if card_filter:
            lines.extend(card_filter)
            lines.append(f"{indent}    return True")
        else:
            lines.append(f"{indent}    return True")
        lines.append(f"{indent}game.effect_play_from_zone(")
        lines.append(f"{indent}    player, '{zone}', play_filter, free=True, is_optional=True)")
    elif action == "reveal_and_select":
        count = eb.reveal_count or 4
        lines.append(f"{indent}if not (player and game):")
        lines.append(f"{indent}    return")
        card_filter = _build_card_filter_code(eb, indent)
        lines.append(f"{indent}def reveal_filter(c):")
        if card_filter:
            lines.extend(card_filter)
            lines.append(f"{indent}    return True")
        else:
            lines.append(f"{indent}    return True")
        lines.append(f"{indent}def on_revealed(selected, remaining):")
        lines.append(f"{indent}    player.hand_cards.append(selected)")
        lines.append(f"{indent}    for c in remaining:")
        lines.append(f"{indent}        player.library_cards.append(c)")
        lines.append(f"{indent}game.effect_reveal_and_select(")
        lines.append(f"{indent}    player, {count}, reveal_filter, on_revealed, is_optional=True)")
    elif action == "de_digivolve":
        # Fix 4: Use extracted degen count instead of hardcoded 1
        count = eb.degen_count if eb.degen_count is not None else 1
        lines.append(f"{indent}if not (player and game):")
        lines.append(f"{indent}    return")
        lines.append(f"{indent}def on_de_digivolve(target_perm):")
        lines.append(f"{indent}    removed = target_perm.de_digivolve({count})")
        lines.append(f"{indent}    enemy = player.enemy if player else None")
        lines.append(f"{indent}    if enemy:")
        lines.append(f"{indent}        enemy.trash_cards.extend(removed)")
        lines.append(f"{indent}game.effect_select_opponent_permanent(")
        lines.append(f"{indent}    player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional={eb.is_optional})")
    elif action == "digivolve":
        lines.append(f"{indent}if not (player and perm and game):")
        lines.append(f"{indent}    return")
        card_filter = _build_card_filter_code(eb, indent)
        lines.append(f"{indent}def digi_filter(c):")
        if card_filter:
            lines.extend(card_filter)
            lines.append(f"{indent}    return True")
        else:
            lines.append(f"{indent}    return True")
        kwargs = []
        if eb.digi_cost_override is not None:
            kwargs.append(f"cost_override={eb.digi_cost_override}")
        if eb.digi_ignore_reqs:
            kwargs.append("ignore_requirements=True")
        kwargs.append("is_optional=True")
        kwargs_str = ", ".join(kwargs)
        lines.append(f"{indent}game.effect_digivolve_from_hand(")
        lines.append(f"{indent}    player, perm, digi_filter, {kwargs_str})")
    elif action == "cost_reduction":
        if eb.cost_reduction_val:
            lines.append(f"{indent}# Cost reduction handled via cost_reduction property")
    elif action == "mind_link":
        lines.append(f"{indent}if not (player and game):")
        lines.append(f"{indent}    return")
        lines.append(f"{indent}game.effect_link_to_permanent(player, card, is_optional=True)")
    # Fix 11: New action types
    elif action == "unsuspend":
        lines.append(f"{indent}if not (player and game):")
        lines.append(f"{indent}    return")
        lines.append(f"{indent}def target_filter(p):")
        lines.append(f"{indent}    return True")
        lines.append(f"{indent}def on_unsuspend(target_perm):")
        lines.append(f"{indent}    target_perm.unsuspend()")
        lines.append(f"{indent}game.effect_select_own_permanent(")
        lines.append(f"{indent}    player, on_unsuspend, filter_fn=target_filter, is_optional={eb.is_optional})")
    elif action == "destroy_security":
        count = eb.destroy_security_count or 1
        lines.append(f"{indent}# Trash opponent's top security card(s)")
        lines.append(f"{indent}enemy = player.enemy if player else None")
        lines.append(f"{indent}if enemy:")
        lines.append(f"{indent}    for _ in range({count}):")
        lines.append(f"{indent}        if enemy.security_cards:")
        lines.append(f"{indent}            trashed = enemy.security_cards.pop()")
        lines.append(f"{indent}            enemy.trash_cards.append(trashed)")
    elif action == "restrict_attack":
        lines.append(f"{indent}# Attack restriction — select opponent permanent to restrict")
        lines.append(f"{indent}if not (player and game):")
        lines.append(f"{indent}    return")
        lines.append(f"{indent}def target_filter(p):")
        lines.append(f"{indent}    return p.is_digimon")
        lines.append(f"{indent}def on_restrict(target_perm):")
        lines.append(f"{indent}    target_perm.suspend()  # Approximate as suspend")
        lines.append(f"{indent}game.effect_select_opponent_permanent(")
        lines.append(f"{indent}    player, on_restrict, filter_fn=target_filter, is_optional={eb.is_optional})")
    elif action == "target_lock":
        lines.append(f"{indent}# Target lock — this Digimon's attack target can't be switched")
        lines.append(f"{indent}pass  # Handled by engine attack target resolution")
    elif action == "flip_security":
        lines.append(f"{indent}# Flip opponent's top face-down security card face up")
        lines.append(f"{indent}enemy = player.enemy if player else None")
        lines.append(f"{indent}if enemy and enemy.security_cards:")
        lines.append(f"{indent}    pass  # Security flip — engine handles face-up/face-down state")
    elif action == "return_to_deck":
        lines.append(f"{indent}if not (player and game):")
        lines.append(f"{indent}    return")
        target_filter = _build_target_filter_code(eb, indent, perm_only=use_perm_only)
        lines.append(f"{indent}def target_filter(p):")
        if target_filter:
            lines.extend(target_filter)
            lines.append(f"{indent}    return True")
        else:
            lines.append(f"{indent}    return True")
        lines.append(f"{indent}def on_return(target_perm):")
        lines.append(f"{indent}    enemy = player.enemy if player else None")
        lines.append(f"{indent}    if enemy:")
        lines.append(f"{indent}        enemy.return_permanent_to_deck_bottom(target_perm)")
        lines.append(f"{indent}game.effect_select_opponent_permanent(")
        lines.append(f"{indent}    player, on_return, filter_fn=target_filter, is_optional={eb.is_optional})")
    elif action == "jogress_condition":
        lines.append(f"{indent}# DNA/Jogress digivolution condition — handled by engine")
        lines.append(f"{indent}pass")


def _generate_factory_condition_code(eb: EffectBlock, idx: int, indent: str = "        ") -> str:
    """Fix 1: Generate condition code for factory effects using extracted closure data."""
    checks = []
    inner = indent + "    "

    if eb.factory_cond_owner_turn:
        checks.append(f"{inner}if not (card and card.owner and card.owner.is_my_turn):")
        checks.append(f"{inner}    return False")

    if eb.factory_cond_on_battle:
        checks.append(f"{inner}if card and card.permanent_of_this_card() is None:")
        checks.append(f"{inner}    return False")

    if eb.factory_cond_digi_count is not None:
        checks.append(f"{inner}permanent = card.permanent_of_this_card() if card else None")
        checks.append(f"{inner}if not (permanent and len(permanent.digivolution_cards) >= {eb.factory_cond_digi_count}):")
        checks.append(f"{inner}    return False")

    if eb.factory_cond_has_text:
        or_parts = " or ".join(f"'{t}' in text" for t in eb.factory_cond_has_text)
        checks.append(f"{inner}permanent = card.permanent_of_this_card() if card else None")
        checks.append(f"{inner}if permanent and permanent.top_card:")
        checks.append(f"{inner}    text = permanent.top_card.card_text")
        checks.append(f"{inner}    if not ({or_parts}):")
        checks.append(f"{inner}        return False")
        checks.append(f"{inner}else:")
        checks.append(f"{inner}    return False")

    if eb.factory_cond_source_name:
        or_parts = " or ".join(
            f"src.contains_card_name('{n}')" for n in eb.factory_cond_source_name)
        checks.append(f"{inner}permanent = card.permanent_of_this_card() if card else None")
        checks.append(f"{inner}if not (permanent and any({or_parts} for src in permanent.digivolution_cards)):")
        checks.append(f"{inner}    return False")

    if eb.factory_cond_source_trait:
        or_parts = " or ".join(
            f"any('{t}' in tr for tr in (getattr(src, 'card_traits', []) or []))"
            for t in eb.factory_cond_source_trait)
        checks.append(f"{inner}permanent = card.permanent_of_this_card() if card else None")
        checks.append(f"{inner}if not (permanent and any({or_parts} for src in permanent.digivolution_cards)):")
        checks.append(f"{inner}    return False")

    if eb.factory_cond_perm_name:
        or_parts = " or ".join(
            f"permanent.contains_card_name('{n}')" for n in eb.factory_cond_perm_name)
        checks.append(f"{inner}permanent = card.permanent_of_this_card() if card else None")
        checks.append(f"{inner}if not (permanent and ({or_parts})):")
        checks.append(f"{inner}    return False")

    if eb.factory_cond_perm_trait:
        or_parts = " or ".join(
            f"any('{t}' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or []))"
            for t in eb.factory_cond_perm_trait)
        checks.append(f"{inner}permanent = card.permanent_of_this_card() if card else None")
        checks.append(f"{inner}if not (permanent and permanent.top_card and ({or_parts})):")
        checks.append(f"{inner}    return False")

    lines = [f"{indent}def condition{idx}(context: Dict[str, Any]) -> bool:"]
    if checks:
        lines.extend(checks)
        lines.append(f"{inner}return True")
    else:
        lines.append(f"{inner}return True")
    return "\n".join(lines)


def generate_factory_effect(eb: EffectBlock, card_id: str, idx: int) -> str:
    """Generate Python code for a factory-based effect."""
    lines = []
    var = f"effect{idx}"
    lines.append(f"        # Factory effect: {eb.factory_method}")
    lines.append(f"        # {eb.description}")
    lines.append(f"        {var} = ICardEffect()")
    lines.append(f'        {var}.set_effect_name("{card_id} {eb.description}")')
    lines.append(f'        {var}.set_effect_description("{eb.description}")')

    if eb.is_inherited:
        lines.append(f"        {var}.is_inherited_effect = True")

    if eb.factory_method == "blocker":
        lines.append(f"        {var}._is_blocker = True")
    elif eb.factory_method == "jamming":
        lines.append(f"        {var}._is_jamming = True")
    elif eb.factory_method == "rush":
        lines.append(f"        {var}._is_rush = True")
    elif eb.factory_method == "reboot":
        lines.append(f"        {var}._is_reboot = True")
    elif eb.factory_method == "raid":
        lines.append(f"        {var}._is_raid = True")
    elif eb.factory_method == "alliance":
        lines.append(f"        {var}._is_alliance = True")
    elif eb.factory_method == "security_play":
        lines.append(f"        {var}.is_security_effect = True")
    elif eb.factory_method == "security_attack_plus":
        sa_val = eb.factory_sa_value if eb.factory_sa_value is not None else 1
        lines.append(f"        {var}._security_attack_modifier = {sa_val}")
    elif eb.factory_method == "dp_modifier":
        dp_val = eb.factory_dp_value if eb.factory_dp_value is not None else 0
        lines.append(f"        {var}.dp_modifier = {dp_val}")
    elif eb.factory_method == "dp_modifier_all":
        # Fix 5: Non-self DP modifier (all your Digimon)
        dp_val = eb.factory_dp_value if eb.factory_dp_value is not None else 0
        lines.append(f"        {var}.dp_modifier = {dp_val}")
        lines.append(f"        {var}._applies_to_all_own_digimon = True")
    elif eb.factory_method == "armor_purge":
        lines.append(f"        {var}._is_armor_purge = True")
    elif eb.factory_method == "blast_digivolve":
        lines.append(f"        {var}.is_counter_effect = True")
        lines.append(f"        {var}._is_blast_digivolve = True")
    # Fix 11: New factory keywords
    elif eb.factory_method == "piercing":
        lines.append(f"        {var}._is_piercing = True")
    elif eb.factory_method == "collision":
        lines.append(f"        {var}._is_collision = True")
    elif eb.factory_method == "blitz":
        lines.append(f"        {var}._is_blitz = True")
    elif eb.factory_method == "fortitude":
        lines.append(f"        {var}._is_fortitude = True")
    elif eb.factory_method == "evade":
        lines.append(f"        {var}._is_evade = True")
    elif eb.factory_method == "barrier":
        lines.append(f"        {var}._is_barrier = True")
    elif eb.factory_method == "decoy":
        lines.append(f"        {var}._is_decoy = True")
    elif eb.factory_method == "retaliation":
        lines.append(f"        {var}._is_retaliation = True")
    elif eb.factory_method == "save":
        lines.append(f"        {var}._is_save = True")
    elif eb.factory_method == "material_save":
        lines.append(f"        {var}._is_material_save = True")
    elif eb.factory_method == "overclock":
        lines.append(f"        {var}._is_overclock = True")
    elif eb.factory_method == "vortex":
        lines.append(f"        {var}._is_vortex = True")
    elif eb.factory_method == "training":
        lines.append(f"        {var}._is_training = True")
    elif eb.factory_method == "progress":
        lines.append(f"        {var}._is_progress = True")
    # Fix 12: New keywords from rules evaluation
    elif eb.factory_method == "digisorption":
        lines.append(f"        {var}._is_digisorption = True")
    elif eb.factory_method == "digiburst":
        lines.append(f"        {var}._is_digiburst = True")
    elif eb.factory_method == "delay":
        lines.append(f"        {var}._is_delay = True")
    elif eb.factory_method == "partition":
        lines.append(f"        {var}._is_partition = True")
    elif eb.factory_method == "digixros":
        lines.append(f"        {var}._is_digixros = True")
    elif eb.factory_method == "scapegoat":
        lines.append(f"        {var}._is_scapegoat = True")
    elif eb.factory_method == "decode":
        lines.append(f"        {var}._is_decode = True")
    elif eb.factory_method == "iceclad":
        lines.append(f"        {var}._is_iceclad = True")
    elif eb.factory_method == "fragment":
        lines.append(f"        {var}._is_fragment = True")
    elif eb.factory_method == "execute":
        lines.append(f"        {var}._is_execute = True")
    elif eb.factory_method == "set_memory_3":
        lines.append(f"        # [Start of Your Turn] Set memory to 3 if <= 2")
    elif eb.factory_method == "gain_memory_tamer":
        lines.append(f"        # [Start of Main] Gain 1 memory if opponent has Digimon")
    elif eb.factory_method == "alt_digivolve_req":
        cost = eb.digi_cost_override if eb.digi_cost_override is not None else 0
        names = eb.name_checks
        traits = eb.trait_checks
        desc_parts = []
        if names:
            desc_parts.append(f"from [{names[0]}]")
        if traits:
            desc_parts.append(f"with [{traits[0]}] trait")
        desc_str = " ".join(desc_parts) if desc_parts else "alternate source"
        lines.append(f"        # Alternate digivolution: {desc_str} for cost {cost}")
        lines.append(f"        {var}._alt_digi_cost = {cost}")
        if names:
            lines.append(f"        {var}._alt_digi_name = \"{names[0]}\"")
        if traits:
            lines.append(f"        {var}._alt_digi_trait = \"{traits[0]}\"")
    elif eb.factory_method == "change_digi_cost":
        cost_val = eb.cost_reduction_val if eb.cost_reduction_val is not None else -1
        traits = eb.trait_checks
        names = eb.name_checks
        desc_parts = []
        if traits:
            desc_parts.append(f"[{'/'.join(traits)}] trait")
        if names:
            desc_parts.append(f"[{'/'.join(names)}] name")
        desc_str = " ".join(desc_parts) if desc_parts else "matching"
        lines.append(f"        # Reduce digivolution cost by {abs(cost_val)} for {desc_str}")
        lines.append(f"        {var}.cost_reduction = {abs(cost_val)}")

    # Fix 1: Generate real condition code from extracted closure data
    lines.append(f"")
    lines.append(_generate_factory_condition_code(eb, idx, "        "))
    lines.append(f"        {var}.set_can_use_condition(condition{idx})")
    lines.append(f"        effects.append({var})")
    return "\n".join(lines)


def generate_activate_effect(eb: EffectBlock, card_id: str, idx: int) -> str:
    """Generate Python code for an ActivateClass-based effect."""
    lines = []
    var = f"effect{idx}"
    action_desc = generate_action_comment(eb)
    desc = eb.description or action_desc

    lines.append(f"        # Timing: {eb.timing}")
    lines.append(f"        # {desc}")
    lines.append(f"        {var} = ICardEffect()")
    lines.append(f'        {var}.set_effect_name("{card_id} {eb.effect_name or action_desc}")')
    lines.append(f'        {var}.set_effect_description("{desc}")')

    if eb.is_inherited:
        lines.append(f"        {var}.is_inherited_effect = True")

    if eb.is_optional:
        lines.append(f"        {var}.is_optional = True")

    if eb.max_count_per_turn > 0:
        lines.append(f"        {var}.set_max_count_per_turn({eb.max_count_per_turn})")

    if eb.hash_string:
        lines.append(f'        {var}.set_hash_string("{eb.hash_string}")')

    # Fix 9: Separate is_on_play vs is_when_digivolving
    prop = TIMING_TO_PROPERTY.get(eb.timing)
    if prop:
        if prop == "is_on_play" and "trigger_when_digivolving" in eb.conditions:
            # This is a When Digivolving effect, not On Play
            lines.append(f"        {var}.is_when_digivolving = True")
        else:
            lines.append(f"        {var}.{prop} = True")

    if eb.timing == "EffectTiming.SecuritySkill":
        lines.append(f"        {var}.is_security_effect = True")

    # DP modifier
    if eb.dp_change and not any(a for a in eb.actions if a not in ("change_dp",)):
        lines.append(f"        {var}.dp_modifier = {eb.dp_change}")

    # Cost reduction
    if eb.cost_reduction_val and "cost_reduction" in eb.actions:
        lines.append(f"        {var}.cost_reduction = {eb.cost_reduction_val}")

    # Condition — pass `effect` variable name for condition closures
    # We need the effect var name for accessing effect_source_permanent
    lines.append(f"")
    lines.append(f"        effect = {var}  # alias for condition closure")
    lines.append(f"        def condition{idx}(context: Dict[str, Any]) -> bool:")
    lines.append(generate_condition_code(eb, "            "))
    lines.append(f"")
    lines.append(f"        {var}.set_can_use_condition(condition{idx})")

    # Callback for actions
    if eb.actions:
        lines.append(f"")
        lines.append(f"        def process{idx}(ctx: Dict[str, Any]):")
        lines.append(f"            \"\"\"Action: {action_desc}\"\"\"")
        lines.append(generate_callback_code(eb, "            "))
        lines.append(f"")
        lines.append(f"        {var}.set_on_process_callback(process{idx})")

    lines.append(f"        effects.append({var})")
    return "\n".join(lines)


def generate_python_script(class_name: str, card_id: str, effects: List[EffectBlock],
                           card_db: Optional[Dict[str, dict]] = None) -> str:
    """Generate a complete Python CardScript file."""
    lines = []

    # Look up card metadata from cards.json
    card_meta = (card_db or {}).get(card_id, {})
    card_name = card_meta.get("card_name_eng", "")
    card_level = card_meta.get("level", 0)

    lines.append("from __future__ import annotations")
    lines.append("from typing import TYPE_CHECKING, List, Dict, Any")
    lines.append("from ....core.card_script import CardScript")
    lines.append("from ....interfaces.card_effect import ICardEffect")
    lines.append("")
    lines.append("if TYPE_CHECKING:")
    lines.append("    from ....core.card_source import CardSource")
    lines.append("")
    lines.append("")
    # Include card name and level as comment/docstring
    doc_parts = [f"{card_id} {card_name}" if card_name else card_id]
    if card_level:
        doc_parts.append(f"Lv.{card_level}")
    lines.append(f"class {class_name}(CardScript):")
    lines.append(f'    """{" | ".join(doc_parts)}"""')
    lines.append("")
    lines.append(f"    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:")
    lines.append(f"        effects = []")

    if not effects:
        lines.append(f"        # No effects found in DCGO source")
        lines.append(f"        return effects")
        return "\n".join(lines) + "\n"

    for idx, eb in enumerate(effects):
        lines.append("")
        if eb.is_factory:
            lines.append(generate_factory_effect(eb, card_id, idx))
        else:
            lines.append(generate_activate_effect(eb, card_id, idx))

    lines.append("")
    lines.append("        return effects")
    return "\n".join(lines) + "\n"


# ─── Main ────────────────────────────────────────────────────────────

def _build_validation_patterns():
    """Build decision patterns for cross-validation against digimoncard.io effect text."""
    def _ab(inner):
        return rf"(?:<{inner}>|＜{inner}＞)"

    patterns = []
    def _p(name, check_fn, pattern):
        patterns.append((name, check_fn, re.compile(pattern, re.IGNORECASE)))
    # Map: (pattern_name, function_to_check_if_script_handles_it, regex)
    _p("reveal_top", lambda s: "effect_reveal_and_select" in s,
       r"reveal\s+(?:the\s+)?top\s+\d+\s+cards?")
    _p("play", lambda s: "effect_play_from_zone" in s,
       r"play\s+(?:1|up\s+to\s+\d+|it\b)")
    _p("digivolve_into", lambda s: "effect_digivolve_from_hand" in s,
       r"(?:digivolve\s+into|may\s+digivolve|can\s+digivolve)")
    _p("delete_opponent", lambda s: "delete_permanent" in s or "effect_select_opponent_permanent" in s,
       r"delete\s+\d+\s+of\s+your\s+opponent")
    _p("de_digivolve", lambda s: "de_digivolve" in s,
       _ab(r"De-Digivolve\s*\d*"))
    _p("draw_keyword", lambda s: "draw_cards" in s,
       _ab(r"Draw\s*\d*"))
    _p("recovery", lambda s: "recovery" in s,
       _ab(r"Recovery\s*\+?\d*"))
    _p("memory_gain", lambda s: "add_memory" in s,
       r"gain\s+\d+\s+memory")
    _p("mind_link", lambda s: "effect_link_to_permanent" in s,
       _ab(r"Mind\s*Link"))
    _p("blocker", lambda s: "_is_blocker" in s,
       _ab(r"Blocker"))
    _p("piercing", lambda s: "_is_piercing" in s,
       _ab(r"Piercing"))
    _p("rush", lambda s: "_is_rush" in s,
       _ab(r"Rush"))
    _p("reboot", lambda s: "_is_reboot" in s,
       _ab(r"Reboot"))
    _p("raid", lambda s: "_is_raid" in s,
       _ab(r"Raid"))
    _p("dp_modification", lambda s: "change_dp" in s or "dp_modifier" in s,
       r"(?:get|gain)s?\s+[+\-]\d+\s*DP")
    _p("suspend_target", lambda s: "suspend" in s,
       r"suspend\s+\d+")
    _p("security_attack_mod", lambda s: "_security_attack_modifier" in s,
       _ab(r"Security\s+Attack\s+[+\-]\d+"))
    return patterns


def main():
    if len(sys.argv) < 2:
        print("Usage: python transpile_dcgo.py <DCGO_DIR> [OUTPUT_DIR] [--validate]")
        print("  e.g. python transpile_dcgo.py /tmp/dcgo-scripts/CardEffect/BT24")
        sys.exit(1)

    validate = "--validate" in sys.argv
    args = [a for a in sys.argv[1:] if not a.startswith("--")]

    dcgo_dir = args[0]
    # Infer set_id from directory name (e.g. /tmp/.../BT24 -> bt24)
    set_id = os.path.basename(dcgo_dir.rstrip("/")).lower()
    output_dir = args[1] if len(args) > 1 else os.path.join(
        os.path.dirname(__file__), "..", "digimon_gym", "engine", "data", "scripts", set_id
    )

    output_dir = os.path.abspath(output_dir)
    os.makedirs(output_dir, exist_ok=True)

    # Load cards.json for card metadata (name, level, kind)
    cards_json_path = os.path.join(
        os.path.dirname(__file__), "..", "digimon_gym", "engine", "data", "cards.json"
    )
    card_db: Dict[str, dict] = {}
    all_card_names: set = set()
    all_traits: set = set()
    if os.path.exists(cards_json_path):
        with open(cards_json_path, encoding="utf-8") as f:
            for c in json.load(f):
                card_db[c["card_id"]] = c
                if c.get("card_name_eng"):
                    all_card_names.add(c["card_name_eng"])
                for t in (c.get("type_eng") or []):
                    if t:
                        all_traits.add(t)

    if all_card_names:
        print(f"Card database: {len(card_db)} cards, {len(all_card_names)} unique names, {len(all_traits)} unique traits")

    # Build validation patterns if --validate
    validation_patterns = _build_validation_patterns() if validate else []

    # Find all .cs files
    cs_files = []
    for root, dirs, files in os.walk(dcgo_dir):
        for f in files:
            if f.endswith('.cs') and not f.endswith('.meta'):
                cs_files.append(os.path.join(root, f))

    cs_files.sort()
    print(f"Found {len(cs_files)} C# files in {dcgo_dir}")

    stats = {"total": 0, "with_effects": 0, "factory": 0, "activate": 0, "total_effects": 0}
    report = []

    for cs_path in cs_files:
        class_name, effects = parse_cs_file(cs_path)
        card_id = class_name.replace("_", "-")  # BT14_001 -> BT14-001
        module_name = class_name.lower()  # bt14_001

        stats["total"] += 1
        stats["total_effects"] += len(effects)

        if effects:
            stats["with_effects"] += 1
            for e in effects:
                if e.is_factory:
                    stats["factory"] += 1
                else:
                    stats["activate"] += 1

        py_code = generate_python_script(class_name, card_id, effects, card_db)
        out_path = os.path.join(output_dir, f"{module_name}.py")
        with open(out_path, "w", encoding="utf-8") as f:
            f.write(py_code)

        effect_summary = []
        for e in effects:
            if e.is_factory:
                effect_summary.append(f"  [factory] {e.factory_method}")
            else:
                actions = ", ".join(e.actions) if e.actions else "no-action"
                inherited = " (inherited)" if e.is_inherited else ""
                once = " (1/turn)" if e.max_count_per_turn > 0 else ""
                effect_summary.append(f"  [{e.timing}] {actions}{inherited}{once}")

        report.append(f"{class_name}: {len(effects)} effects")
        for s in effect_summary:
            report.append(s)

    # Write __init__.py
    init_path = os.path.join(output_dir, "__init__.py")
    with open(init_path, "w", encoding="utf-8") as f:
        f.write("")

    print(f"\nTranspilation complete:")
    print(f"  Scripts processed: {stats['total']}")
    print(f"  Scripts with effects: {stats['with_effects']}")
    print(f"  Total effects extracted: {stats['total_effects']}")
    print(f"    Factory effects: {stats['factory']}")
    print(f"    Activate effects: {stats['activate']}")
    print(f"  Output directory: {output_dir}")

    # Write report
    report_path = os.path.join(output_dir, "TRANSPILE_REPORT.md")
    with open(report_path, "w", encoding="utf-8") as f:
        f.write(f"# {set_id.upper()} Transpilation Report\n\n")
        f.write(f"Generated from DCGO C# card scripts.\n\n")
        f.write(f"- Total scripts: {stats['total']}\n")
        f.write(f"- Scripts with effects: {stats['with_effects']}\n")
        f.write(f"- Total effects: {stats['total_effects']}\n")
        f.write(f"- Factory effects: {stats['factory']}\n")
        f.write(f"- Activate effects: {stats['activate']}\n\n")
        f.write("## Per-Card Breakdown\n\n```\n")
        f.write("\n".join(report))
        f.write("\n```\n")

    print(f"  Report: {report_path}")

    # ── Cross-Validation against digimoncard.io effect text ──
    if validate and card_db:
        print("\n--- Cross-Validation ---")
        validation_issues = []
        validated_count = 0
        matched_count = 0

        for cs_path in cs_files:
            class_name, _ = parse_cs_file(cs_path)
            card_id = class_name.replace("_", "-")
            module_name = class_name.lower()
            card_meta = card_db.get(card_id, {})

            if not card_meta:
                continue

            # Get effect text from digimoncard.io data
            effect_text = (card_meta.get("effect_description_eng", "") or "") + " " + \
                          (card_meta.get("inherited_effect_description_eng", "") or "") + " " + \
                          (card_meta.get("security_effect_description_eng", "") or "")

            if not effect_text.strip():
                continue

            # Read the generated Python script
            py_path = os.path.join(output_dir, f"{module_name}.py")
            if not os.path.exists(py_path):
                continue

            with open(py_path, "r", encoding="utf-8") as f:
                py_code = f.read()

            validated_count += 1

            # Check each validation pattern
            for pname, check_fn, pregex in validation_patterns:
                if pregex.search(effect_text):
                    matched_count += 1
                    if not check_fn(py_code):
                        validation_issues.append(
                            f"{card_id}: API text has '{pname}' but script missing corresponding implementation"
                        )

        print(f"  Cards validated: {validated_count}")
        print(f"  Pattern matches checked: {matched_count}")
        print(f"  Mismatches found: {len(validation_issues)}")

        if validation_issues:
            # Append to report
            with open(report_path, "a", encoding="utf-8") as f:
                f.write("\n\n## Cross-Validation Issues\n\n")
                f.write(f"Checked {validated_count} cards against digimoncard.io effect text.\n\n")
                f.write("```\n")
                for issue in sorted(validation_issues):
                    f.write(f"{issue}\n")
                f.write("```\n")
            print(f"  Issues appended to {report_path}")
            for issue in validation_issues[:10]:
                print(f"    {issue}")
            if len(validation_issues) > 10:
                print(f"    ... and {len(validation_issues) - 10} more")
        else:
            with open(report_path, "a", encoding="utf-8") as f:
                f.write("\n\n## Cross-Validation\n\n")
                f.write(f"All {validated_count} cards passed cross-validation against digimoncard.io effect text.\n")
            print("  All checks passed!")

    print(f"\n  Report: {report_path}")


if __name__ == "__main__":
    main()
