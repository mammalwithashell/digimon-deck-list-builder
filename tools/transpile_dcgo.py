#!/usr/bin/env python3
"""
Transpile DCGO C# card effect scripts into Python CardScript files.

Reads BT14 .cs files from the DCGO-Card-Scripts repo and generates
Python equivalents compatible with the digimon_gym engine.

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
RE_SET_INHERITED = re.compile(r'SetIsInheritedEffect\s*\(\s*(true|false)\s*\)')
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

# Factory method patterns
RE_FACTORY_BLOCKER = re.compile(r'BlockerSelfStaticEffect')
RE_FACTORY_JAMMING = re.compile(r'JammingSelfStaticEffect')
RE_FACTORY_RUSH = re.compile(r'RushSelfEffect')
RE_FACTORY_REBOOT = re.compile(r'RebootSelfStaticEffect')
RE_FACTORY_RAID = re.compile(r'RaidSelfEffect')
RE_FACTORY_ALLIANCE = re.compile(r'AllianceSelfEffect')
RE_FACTORY_SEC_PLAY = re.compile(r'PlaySelfTamerSecurityEffect|PlaySelfDigimonAfterBattleSecurityEffect')
RE_FACTORY_SA_PLUS = re.compile(r'ChangeSelfSAttackStaticEffect')
RE_FACTORY_DP = re.compile(r'ChangeSelfDPStaticEffect')
RE_FACTORY_ARMOR_PURGE = re.compile(r'ArmorPurgeEffect')
RE_FACTORY_BLAST_DIGI = re.compile(r'BlastDigivolveEffect')
RE_FACTORY_SET_MEM_3 = re.compile(r'SetMemoryTo3TamerEffect')
RE_FACTORY_GAIN_MEM = re.compile(r'Gain1MemoryTamerOpponentDigimonEffect')

# Condition patterns
RE_COND_ON_BATTLE = re.compile(r'IsExistOnBattleArea\s*\(\s*card\s*\)')
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
                eb.is_inherited = inh.group(1) == "true"
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
            eb.is_inherited = m.group(1) == "true"

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

        effects.append(eb)

    return effects


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
    # e.g., activateClass defined before timing check, added inside timing block
    # We handle this by also scanning the full source for factory patterns in timing==None
    if not timing_blocks:
        # No timing blocks found - check full source
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
            parts.append(action.replace("_", " ").title())
    return ", ".join(parts) if parts else "Effect"


def generate_callback_code(eb: EffectBlock, indent: str = "            ") -> str:
    """Generate the on_process_callback body with real engine calls."""
    lines = []
    lines.append(f"{indent}player = ctx.get('player')")
    lines.append(f"{indent}perm = ctx.get('permanent')")

    if eb.draw_count:
        lines.append(f"{indent}if player:")
        lines.append(f"{indent}    player.draw_cards({eb.draw_count})")
    if eb.memory_gain:
        lines.append(f"{indent}if player:")
        lines.append(f"{indent}    player.add_memory({eb.memory_gain})")
    if eb.dp_change:
        # DP change targets opponent's digimon by default
        if eb.dp_change < 0:
            lines.append(f"{indent}# DP change targets opponent digimon")
            lines.append(f"{indent}enemy = player.enemy if player else None")
            lines.append(f"{indent}if enemy and enemy.battle_area:")
            lines.append(f"{indent}    target = min(enemy.battle_area, key=lambda p: p.dp)")
            lines.append(f"{indent}    target.change_dp({eb.dp_change})")
        else:
            lines.append(f"{indent}if perm:")
            lines.append(f"{indent}    perm.change_dp({eb.dp_change})")
    if eb.recovery_count:
        lines.append(f"{indent}if player:")
        lines.append(f"{indent}    player.recovery({eb.recovery_count})")

    for action in eb.actions:
        if action in ("draw", "gain_memory", "change_dp", "recovery"):
            continue  # Already handled above
        elif action == "delete":
            lines.append(f"{indent}# Delete: target selection needed for full impl")
            lines.append(f"{indent}enemy = player.enemy if player else None")
            lines.append(f"{indent}if enemy and enemy.battle_area:")
            lines.append(f"{indent}    target = min(enemy.battle_area, key=lambda p: p.dp)")
            lines.append(f"{indent}    enemy.delete_permanent(target)")
        elif action == "bounce":
            lines.append(f"{indent}# Bounce: return opponent's digimon to hand")
            lines.append(f"{indent}enemy = player.enemy if player else None")
            lines.append(f"{indent}if enemy and enemy.battle_area:")
            lines.append(f"{indent}    target = enemy.battle_area[-1]")
            lines.append(f"{indent}    player.bounce_permanent_to_hand(target)")
        elif action == "suspend":
            lines.append(f"{indent}# Suspend opponent's digimon")
            lines.append(f"{indent}enemy = player.enemy if player else None")
            lines.append(f"{indent}if enemy and enemy.battle_area:")
            lines.append(f"{indent}    target = enemy.battle_area[-1]")
            lines.append(f"{indent}    target.suspend()")
        elif action == "trash_from_hand":
            lines.append(f"{indent}# Trash from hand (cost/effect)")
            lines.append(f"{indent}if player and player.hand_cards:")
            lines.append(f"{indent}    player.trash_from_hand([player.hand_cards[-1]])")
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
            lines.append(f"{indent}# Play a card (from hand/trash/reveal)")
            lines.append(f"{indent}pass  # TODO: target selection for play_card")
        elif action == "reveal_and_select":
            lines.append(f"{indent}# Reveal top cards and select")
            lines.append(f"{indent}pass  # TODO: reveal_and_select needs UI/agent choice")
        elif action == "de_digivolve":
            lines.append(f"{indent}# De-digivolve opponent's digimon")
            lines.append(f"{indent}enemy = player.enemy if player else None")
            lines.append(f"{indent}if enemy and enemy.battle_area:")
            lines.append(f"{indent}    target = enemy.battle_area[-1]")
            lines.append(f"{indent}    removed = target.de_digivolve(1)")
            lines.append(f"{indent}    enemy.trash_cards.extend(removed)")
        elif action == "digivolve":
            lines.append(f"{indent}pass  # TODO: digivolve effect needs card selection")
        elif action == "cost_reduction":
            if eb.cost_reduction_val:
                lines.append(f"{indent}# Cost reduction handled via cost_reduction property")
        elif action == "mind_link":
            lines.append(f"{indent}pass  # TODO: mind_link needs tamer/digimon selection")

    if not any(l.strip() != f"player = ctx.get('player')" and l.strip() != f"perm = ctx.get('permanent')" and l.strip() for l in lines):
        lines.append(f"{indent}pass")

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
        lines.append(f"        {var}._security_attack_modifier = 1")
    elif eb.factory_method == "dp_modifier":
        lines.append(f"        {var}.dp_modifier = 0  # TODO: extract DP value from C# source")
    elif eb.factory_method == "armor_purge":
        lines.append(f"        {var}._is_armor_purge = True")
    elif eb.factory_method == "blast_digivolve":
        lines.append(f"        {var}.is_counter_effect = True")
        lines.append(f"        {var}._is_blast_digivolve = True")
    elif eb.factory_method == "set_memory_3":
        lines.append(f"        # [Start of Your Turn] Set memory to 3 if <= 2")
    elif eb.factory_method == "gain_memory_tamer":
        lines.append(f"        # [Start of Main] Gain 1 memory if opponent has Digimon")

    lines.append(f"        def condition{idx}(context: Dict[str, Any]) -> bool:")
    lines.append(f"            return True")
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

    # Set timing property if applicable
    py_timing = TIMING_MAP.get(eb.timing, eb.timing)
    prop = TIMING_TO_PROPERTY.get(eb.timing)
    if prop:
        lines.append(f"        {var}.{prop} = True")

    if eb.timing == "EffectTiming.SecuritySkill":
        lines.append(f"        {var}.is_security_effect = True")

    # DP modifier
    if eb.dp_change and not any(a for a in eb.actions if a not in ("change_dp",)):
        lines.append(f"        {var}.dp_modifier = {eb.dp_change}")

    # Cost reduction
    if eb.cost_reduction_val and "cost_reduction" in eb.actions:
        lines.append(f"        {var}.cost_reduction = {eb.cost_reduction_val}")

    # Condition
    lines.append(f"")
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


def generate_python_script(class_name: str, card_id: str, effects: List[EffectBlock]) -> str:
    """Generate a complete Python CardScript file."""
    lines = []

    lines.append("from __future__ import annotations")
    lines.append("from typing import TYPE_CHECKING, List, Dict, Any")
    lines.append("from ....core.card_script import CardScript")
    lines.append("from ....interfaces.card_effect import ICardEffect")
    lines.append("")
    lines.append("if TYPE_CHECKING:")
    lines.append("    from ....core.card_source import CardSource")
    lines.append("")
    lines.append("")
    lines.append(f"class {class_name}(CardScript):")
    lines.append(f'    """Auto-transpiled from DCGO {class_name}.cs"""')
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

def main():
    dcgo_dir = sys.argv[1] if len(sys.argv) > 1 else "/tmp/dcgo-scripts/CardEffect/BT14"
    output_dir = sys.argv[2] if len(sys.argv) > 2 else os.path.join(
        os.path.dirname(__file__), "..", "digimon_gym", "engine", "data", "scripts", "bt14"
    )

    output_dir = os.path.abspath(output_dir)
    os.makedirs(output_dir, exist_ok=True)

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

        py_code = generate_python_script(class_name, card_id, effects)
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
        f.write("# BT14 Transpilation Report\n\n")
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


if __name__ == "__main__":
    main()
