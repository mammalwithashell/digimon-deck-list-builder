"""C# parsing and effect extraction functions."""
import os
import re
from typing import List, Optional, Tuple

from .models import EffectBlock
from .patterns import (
    RE_CLASS, RE_TIMING_BLOCK, RE_EFFECT_DESC, RE_SET_INHERITED,
    RE_HASH_STRING, RE_MAX_COUNT, RE_IS_OPTIONAL, RE_EFFECT_NAME,
    RE_DRAW, RE_ADD_MEMORY, RE_CHANGE_DP, RE_DELETE, RE_BOUNCE, RE_SUSPEND,
    RE_RECOVERY, RE_PLAY_CARD, RE_TRASH_HAND, RE_TRASH_DIGI,
    RE_ADD_TO_HAND, RE_ADD_SECURITY, RE_REVEAL, RE_DEGENERATION,
    RE_DIGIVOLVE, RE_COST_REDUCTION, RE_MIND_LINK,
    RE_TARGET_DP_LIMIT, RE_TARGET_DP_MIN, RE_TARGET_LEVEL_LIMIT,
    RE_TARGET_LEVEL_MIN,
    RE_REVEAL_COUNT, RE_PLAY_FROM_TRASH, RE_PLAY_FREE,
    RE_DIGI_COST_FIXED, RE_DIGI_IGNORE_REQS, RE_DEGEN_COUNT,
    RE_SELECT_PERM_MODE, RE_DESTROY_SECURITY, RE_REDUCE_SECURITY,
    RE_UNSUSPEND, RE_RESTRICT_ATTACK, RE_TARGET_LOCK, RE_FLIP_SECURITY,
    RE_RETURN_DECK_BOTTOM, RE_JOGRESS,
    RE_MILL, RE_IGNORE_COLOR, RE_APP_FUSION, RE_LINK_CONDITION,
    RE_ALSO_TREATED_AS, RE_CANT_PUT_FIELD,
    RE_GAIN_KEYWORD, GAIN_KEYWORD_MAP,
    RE_SHARED_COROUTINE_DELEGATE, RE_COROUTINE_DELEGATE,
    RE_PLAY_TOKEN, RE_SELECT_ATTACK, RE_CHANGE_SA_TARGET,
    RE_DISABLE_EFFECT, RE_HAND_BOUNCE_CLASS, RE_ADD_EFFECT_TO_PERM,
    RE_CHANGE_DP_COMMONS, RE_PUT_SECURITY_PERM,
    RE_CUSTOM_CALLBACK, RE_AFTER_CUSTOM_CALLBACK,
    RE_COND_ON_BATTLE, RE_COND_OWNER_TURN, RE_COND_ON_PLAY,
    RE_COND_ON_ATTACK, RE_COND_ON_DELETION, RE_COND_WHEN_DIGI,
    RE_COND_SEC_EFFECT, RE_COND_OPTION_MAIN,
    RE_COND_TRAIT, RE_COND_NAME, RE_COND_COLOR,
    RE_COND_HAS_TEXT, RE_COND_ROYAL_KNIGHT,
    RE_FACTORY_COND_DIGI_COUNT, RE_FACTORY_COND_SOURCE_NAME,
    RE_FACTORY_COND_SOURCE_TRAIT, RE_FACTORY_COND_PERM_NAME,
    RE_FACTORY_COND_PERM_TRAIT,
    RE_FACTORY_BLOCKER, RE_FACTORY_JAMMING, RE_FACTORY_RUSH,
    RE_FACTORY_REBOOT, RE_FACTORY_RAID, RE_FACTORY_ALLIANCE,
    RE_FACTORY_SEC_PLAY, RE_FACTORY_SA_PLUS, RE_FACTORY_DP,
    RE_FACTORY_DP_ALL, RE_FACTORY_DP_ALL_VALUE,
    RE_FACTORY_ARMOR_PURGE, RE_FACTORY_BLAST_DIGI,
    RE_FACTORY_SET_MEM_3, RE_FACTORY_GAIN_MEM,
    RE_FACTORY_PIERCING, RE_FACTORY_COLLISION, RE_FACTORY_BLITZ,
    RE_FACTORY_FORTITUDE, RE_FACTORY_EVADE, RE_FACTORY_BARRIER,
    RE_FACTORY_DECOY, RE_FACTORY_RETALIATION, RE_FACTORY_SAVE,
    RE_FACTORY_MATERIAL_SAVE, RE_FACTORY_OVERCLOCK, RE_FACTORY_VORTEX,
    RE_FACTORY_TRAINING, RE_FACTORY_PROGRESS,
    RE_FACTORY_DIGISORPTION, RE_FACTORY_DIGIBURST, RE_FACTORY_DELAY,
    RE_FACTORY_PARTITION, RE_FACTORY_DIGIXROS, RE_FACTORY_SCAPEGOAT,
    RE_FACTORY_DECODE, RE_FACTORY_ICECLAD, RE_FACTORY_FRAGMENT,
    RE_FACTORY_EXECUTE,
    RE_FACTORY_ADD_DIGI_REQ, RE_FACTORY_CHANGE_DIGI_COST,
    RE_FACTORY_CHANGE_DIGI_COST_VALUE,
    RE_FACTORY_DIGI_REQ_COST, RE_FACTORY_DIGI_REQ_NAME, RE_FACTORY_DIGI_REQ_TRAIT,
    RE_FACTORY_DP_VALUE, RE_FACTORY_SA_VALUE,
    # P7: Stub reduction patterns
    RE_CHANGE_COST_VALUE,
    RE_IDEGENERATION, RE_SWITCH_DEFENDER, RE_PLAY_PERMANENT_CARDS, RE_DIGIVOLVE_INTO,
    RE_ADD_SKILL_CLASS,
    RE_ADD_JOGRESS_LEVELS, RE_CHANGE_CARD_NAMES, RE_CAN_ATTACK_TARGET,
    RE_CAN_NOT_AFFECTED,
    # Complex Flow patterns
    RE_DELETE_AND_PROCESS,
    # Keyword grant targeting patterns
    RE_PERM_COND_OPPONENT_AREA, RE_GRANT_MAX_COUNT,
    RE_SELECTED_PERMANENT_REF, RE_DIGI_COUNT_COMPARE,
    # Card selection filter patterns (CanSelectCardCondition body)
    RE_CF_EQUALS_TRAITS, RE_CF_CONTAINS_TRAITS,
    RE_CF_EQUALS_NAME, RE_CF_CONTAINS_NAME,
    RE_CF_COST_MAX, RE_CF_COST_MIN,
    RE_CF_LEVEL_MAX, RE_CF_LEVEL_MIN, RE_CF_IS_LEVEL,
    RE_CF_COLOR,
    RE_CF_IS_DIGIMON, RE_CF_IS_TAMER, RE_CF_IS_OPTION,
    RE_CF_NOT_DIGI_EGG, RE_CF_HAS_PLAY_COST,
    RE_CF_HAS_TRAITS, HAS_TRAITS_MAP,
    RE_CS_LAMBDA, RE_REVEAL_PASS_ENTRY,
    # Hand-or-trash zone choice
    RE_PLAY_HAND_OR_TRASH,
)


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


def _extract_method_body(full_source: str, method_name: str) -> str:
    """Extract the body of a named method from the full C# source.

    Uses brace-depth matching (like extract_timing_blocks) to find the
    complete method body for SharedActivateCoroutine and similar methods.
    """
    # Match method signatures with optional access/static/virtual/async modifiers
    # and various return types (IEnumerator, void, bool, int, string, List<T>)
    pattern = re.compile(
        rf'(?:(?:static|virtual|override|async|private|public|protected|internal)\s+)*'
        rf'(?:IEnumerator|void|bool|int|string|List<[^>]*>)\s+'
        rf'{re.escape(method_name)}\s*\(')
    match = pattern.search(full_source)
    if not match:
        return ""
    start = match.end()
    # Find the opening brace of the method body
    depth = 0
    block_start = None
    for i in range(start, len(full_source)):
        if full_source[i] == '{':
            if block_start is None:
                block_start = i
            depth += 1
        elif full_source[i] == '}':
            depth -= 1
            if depth == 0 and block_start is not None:
                return full_source[block_start:i + 1]
    return ""


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


def extract_activate_effects(block: str, full_source: str = "") -> List[EffectBlock]:
    """Extract ActivateClass-based effects from a block.

    If full_source is provided and a block delegates to a SharedActivateCoroutine,
    the shared method body is extracted and scanned for actions.
    """
    effects = []

    # Split on ActivateClass instantiations
    activate_splits = re.split(r'(ActivateClass\s+\w+\s*=\s*new\s+ActivateClass\s*\(\s*\)\s*;)', block)

    # Also handle ChangeCostClass separately (it doesn't use ActivateClass pattern usually)
    # But it appears in timing blocks.
    if "ChangeCostClass" in block and "ActivateClass" not in block:
        # Treat as a special effect block if not mixed with ActivateClass
        eb = EffectBlock(raw_block=block)
        eb.effect_name = "Cost Reduction"
        _scan_actions(block, eb)
        # Extract conditions specifically for cost reduction
        if "cost_reduction" in eb.actions:
             # Try to find 'count()' method or logic for variable cost
             # Look for `int count()` or similar inside the block
             count_match = re.search(r'int\s+count\s*\(\s*\)\s*{([^}]+)}', block)
             if count_match:
                 # Extract logic from count() body? Too complex for regex.
                 # Just mark it as variable cost reduction.
                 pass
        effects.append(eb)
        return effects

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
            # Check for ChangeCostClass mixed in
            if 'ChangeCostClass' in full_block:
                 eb = EffectBlock(raw_block=full_block)
                 eb.effect_name = "Cost Reduction"
                 _scan_actions(full_block, eb)
                 if "cost_reduction" in eb.actions:
                     effects.append(eb)
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

        # Extract actions using shared scanner (single source of truth)
        _scan_actions(full_block, eb)

        # Fix 6: Detect trash-as-cost pattern (post-scan analysis)
        if ("trash_from_hand" in eb.actions and
                ("draw" in eb.actions or "gain_memory" in eb.actions)):
            desc_lower = eb.description.lower()
            if "by trashing" in desc_lower or "by discarding" in desc_lower:
                eb.is_trash_as_cost = True

        # P1: SharedActivateCoroutine resolution
        # If no actions were found and block delegates to a shared coroutine,
        # extract the shared body and re-run action detection on it.
        # _scan_actions() handles mill, descriptive tags, and all other patterns.
        if not eb.actions and full_source:
            shared_body = _resolve_shared_coroutine(full_block, full_source)
            if shared_body:
                _scan_actions(shared_body, eb)

        # Fix 10: Extract CanActivateCondition patterns
        _extract_activate_conditions(full_block, eb)

        # Extract card selection filter (CanSelectCardCondition lambda)
        _extract_card_filter_conditions(full_block, full_source, eb)

        effects.append(eb)

    return effects


def _resolve_shared_coroutine(block: str, full_source: str) -> str:
    """P1: Detect delegation to a shared coroutine and extract its body.

    Looks for patterns like:
      hash => SharedActivateCoroutine(hash, activateClass)
      hash => WDWASharedActivateCoroutine(hash, activateClass)

    Returns the shared method body or empty string if not found.
    """
    # Try the specific shared coroutine regex first
    m = RE_SHARED_COROUTINE_DELEGATE.search(block)
    if m:
        method_name = m.group(1) or m.group(2)
        if method_name:
            body = _extract_method_body(full_source, method_name)
            if body:
                return body

    # Fallback: try general coroutine delegation (hash => SomeCoroutine(hash, ...))
    m = RE_COROUTINE_DELEGATE.search(block)
    if m:
        method_name = m.group(1)
        # Only resolve if the method name contains "Coroutine" and isn't
        # a standard ActivateCoroutine (which is the timing block itself)
        if method_name and "Activate" not in method_name:
            body = _extract_method_body(full_source, method_name)
            if body:
                return body

    return ""


def _resolve_custom_callback(block: str, full_block: str) -> str:
    """P6: Extract the body of a Mode.Custom selectPermanentCoroutine callback.

    When SelectPermanentEffect.Mode.Custom is used, the actual effect logic
    lives in a nested local function (the callback). This function finds and
    returns that callback body so action detection can be re-run on it.
    """
    # Try selectPermanentCoroutine parameter first
    m = RE_CUSTOM_CALLBACK.search(block)
    if m:
        callback_name = m.group(1)
        if callback_name and callback_name != "null":
            body = _extract_method_body(full_block, callback_name)
            if body:
                return body

    # Also try afterSelectPermanentCoroutine
    m = RE_AFTER_CUSTOM_CALLBACK.search(block)
    if m:
        callback_name = m.group(1)
        if callback_name and callback_name != "null":
            body = _extract_method_body(full_block, callback_name)
            if body:
                return body

    return ""


def _scan_actions(block: str, eb: EffectBlock):
    """Scan a C# code block for action patterns and merge into an EffectBlock.

    This is the single source of truth for all regex-based action detection.
    Called by both extract_activate_effects() and _extract_actions_from_block().
    Uses 'not in eb.actions' guards to safely merge into pre-existing actions.
    """
    # ── Core value-extracting actions ──
    m = RE_DRAW.search(block)
    if m and "draw" not in eb.actions:
        eb.draw_count = int(m.group(1))
        eb.actions.append("draw")

    m = RE_ADD_MEMORY.search(block)
    if m and "gain_memory" not in eb.actions:
        eb.memory_gain = int(m.group(1))
        eb.actions.append("gain_memory")

    m = RE_CHANGE_DP.search(block)
    if m and "change_dp" not in eb.actions:
        eb.dp_change = int(m.group(1))
        eb.actions.append("change_dp")

    m = RE_RECOVERY.search(block)
    if m and "recovery" not in eb.actions:
        eb.recovery_count = int(m.group(1))
        eb.actions.append("recovery")

    # ── Simple boolean action patterns ──
    if RE_DELETE.search(block) and "delete" not in eb.actions:
        eb.actions.append("delete")
    # BT13-111 Logic: DeletePeremanentAndProcessAccordingToResult
    if RE_DELETE_AND_PROCESS.search(block) and "delete_and_process" not in eb.actions:
        eb.actions.append("delete_and_process")
        eb.descriptive_tag = "delete_and_process"

    if RE_BOUNCE.search(block) and "bounce" not in eb.actions:
        eb.actions.append("bounce")
    if RE_SUSPEND.search(block) and "suspend" not in eb.actions:
        eb.actions.append("suspend")
    if RE_PLAY_CARD.search(block) and "play_card" not in eb.actions:
        eb.actions.append("play_card")
    if RE_TRASH_HAND.search(block) and "trash_from_hand" not in eb.actions:
        eb.actions.append("trash_from_hand")
    if RE_TRASH_DIGI.search(block) and "trash_digivolution_cards" not in eb.actions:
        eb.actions.append("trash_digivolution_cards")
    if RE_ADD_TO_HAND.search(block) and "add_to_hand" not in eb.actions:
        eb.actions.append("add_to_hand")
    if RE_ADD_SECURITY.search(block) and "add_to_security" not in eb.actions:
        eb.actions.append("add_to_security")
    if RE_REVEAL.search(block) and "reveal_and_select" not in eb.actions:
        eb.actions.append("reveal_and_select")
    if RE_DEGENERATION.search(block) and "de_digivolve" not in eb.actions:
        eb.actions.append("de_digivolve")
    if RE_DIGIVOLVE.search(block) and "digivolve" not in eb.actions:
        eb.actions.append("digivolve")
    if RE_COST_REDUCTION.search(block) and "cost_reduction" not in eb.actions:
        eb.actions.append("cost_reduction")
        m2 = re.search(r'Cost\s*-=\s*(\d+)', block)
        if m2:
            eb.cost_reduction_val = int(m2.group(1))
        elif eb.cost_reduction_val is None:
            # WI 2: Also try extracting from targetCost/targetCount += N
            m3 = RE_CHANGE_COST_VALUE.search(block)
            if m3:
                eb.cost_reduction_val = int(m3.group(1))

        # Check for complex cost reduction logic (BT13-111 style)
        # Look for "int count()" definition which implies variable cost
        if "int count()" in block:
            # Variable cost logic detected
            pass

    if RE_MIND_LINK.search(block) and "mind_link" not in eb.actions:
        eb.actions.append("mind_link")

    # ── SelectPermanentEffect.Mode.* patterns ──
    for m_mode in RE_SELECT_PERM_MODE.finditer(block):
        eb._has_select_permanent = True  # Track that selection exists (for grant targeting)
        mode = m_mode.group(1)
        if mode == "Destroy" and "delete" not in eb.actions:
            eb.actions.append("delete")
        elif mode == "Tap" and "suspend" not in eb.actions:
            eb.actions.append("suspend")
        elif mode in ("Bounce", "PutLibraryBottom") and "bounce" not in eb.actions:
            eb.actions.append("bounce")
        elif mode == "UnTap" and "unsuspend" not in eb.actions:
            eb.actions.append("unsuspend")
        elif mode == "Custom":
            # P6: Mode.Custom — extract nested callback body and scan for actions
            callback_body = _resolve_custom_callback(block, block)
            if callback_body:
                _scan_actions(callback_body, eb)

    # ── IDestroySecurity ──
    m_ds = RE_DESTROY_SECURITY.search(block)
    if m_ds and "destroy_security" not in eb.actions:
        eb.actions.append("destroy_security")
        eb.destroy_security_count = int(m_ds.group(1))
    elif RE_REDUCE_SECURITY.search(block) and "destroy_security" not in eb.actions:
        eb.actions.append("destroy_security")

    # ── Unsuspend ──
    if RE_UNSUSPEND.search(block) and "unsuspend" not in eb.actions:
        eb.actions.append("unsuspend")

    # ── Attack restriction / target lock ──
    if RE_RESTRICT_ATTACK.search(block) and "restrict_attack" not in eb.actions:
        eb.actions.append("restrict_attack")
    if RE_TARGET_LOCK.search(block) and "target_lock" not in eb.actions:
        eb.actions.append("target_lock")

    # ── Security flip ──
    if RE_FLIP_SECURITY.search(block) and "flip_security" not in eb.actions:
        eb.actions.append("flip_security")

    # ── Return to deck bottom ──
    if RE_RETURN_DECK_BOTTOM.search(block) and "bounce" not in eb.actions and "return_to_deck" not in eb.actions:
        eb.actions.append("return_to_deck")

    # ── DNA/Jogress digivolution ──
    if RE_JOGRESS.search(block) and "jogress_condition" not in eb.actions:
        eb.actions.append("jogress_condition")

    # ── CardEffectCommons.Gain*() keyword grants ──
    for m_gain in RE_GAIN_KEYWORD.finditer(block):
        keyword_name = m_gain.group(1)
        mapped = GAIN_KEYWORD_MAP.get(keyword_name)
        if mapped and f"gain_keyword_{mapped}" not in eb.actions:
            eb.actions.append(f"gain_keyword_{mapped}")
            if not hasattr(eb, 'gained_keywords'):
                eb.gained_keywords = []
            eb.gained_keywords.append(mapped)

    # ── Keyword grant targeting context ──
    # Use _has_select_permanent flag (accumulated across all _scan_actions passes,
    # including shared coroutine resolution) to determine if grant targets a selection
    # vs self. Also search both block and raw_block for opponent/count patterns.
    has_gain_keywords = any(a.startswith("gain_keyword_") for a in eb.actions)
    if has_gain_keywords:
        # Combine raw_block and current block for pattern searches
        combined = (eb.raw_block or "") + "\n" + block
        # Detect self-grant: no SelectPermanentEffect found in any scan pass
        if not eb._has_select_permanent:
            eb.grant_is_self = True
        else:
            eb.grant_is_self = False
        # Detect if grant targets opponent's permanents
        if RE_PERM_COND_OPPONENT_AREA.search(combined):
            eb.grant_target_is_opponent = True
        # Detect multi-target count (Math.Min(N, ...))
        max_count_matches = list(RE_GRANT_MAX_COUNT.finditer(combined))
        if max_count_matches:
            last_count = int(max_count_matches[-1].group(1))
            if last_count > 1:
                eb.grant_max_targets = last_count
        # Detect two-step reference selection: requires BOTH selectedPermanent
        # assignment AND a filter using selectedPermanent's properties (e.g. digi count)
        if RE_SELECTED_PERMANENT_REF.search(combined) and RE_DIGI_COUNT_COMPARE.search(combined):
            eb.grant_has_reference_selection = True
            eb.grant_reference_filter = "digi_count_lte"

    # ── P5: Token play ──
    m_token = RE_PLAY_TOKEN.search(block)
    if m_token and "play_token" not in eb.actions:
        eb.actions.append("play_token")
        eb.token_name = m_token.group(1)
        eb.descriptive_tag = "play_token"

    # ── P5: SelectAttackEffect — forced attack ──
    if RE_SELECT_ATTACK.search(block) and "force_attack" not in eb.actions:
        eb.actions.append("force_attack")
        eb.descriptive_tag = "force_attack"

    # ── P5: ChangeDigimonSAttack — SA modifier to target ──
    m_sa = RE_CHANGE_SA_TARGET.search(block)
    if m_sa and "change_security_attack" not in eb.actions:
        eb.actions.append("change_security_attack")
        eb.descriptive_tag = "change_security_attack"

    # ── P5: DisableEffectClass — effect invalidation ──
    if RE_DISABLE_EFFECT.search(block) and "disable_effect" not in eb.actions:
        eb.actions.append("disable_effect")
        eb.descriptive_tag = "disable_effect"

    # ── P5: HandBounceClass — bounce via helper class ──
    if RE_HAND_BOUNCE_CLASS.search(block) and "bounce" not in eb.actions:
        eb.actions.append("bounce")

    # ── P5: ChangeDigimonDP — DP change via helper ──
    m_dp_commons = RE_CHANGE_DP_COMMONS.search(block)
    if m_dp_commons and "change_dp" not in eb.actions:
        eb.dp_change = int(m_dp_commons.group(1))
        eb.actions.append("change_dp")

    # ── P5: AddEffectToPermanent — grants temporary effects ──
    if RE_ADD_EFFECT_TO_PERM.search(block) and "add_temp_effect" not in eb.actions:
        eb.actions.append("add_temp_effect")
        eb.descriptive_tag = "add_temp_effect"

    # ── P5: IPutSecurityPermanent — place permanent into security ──
    if RE_PUT_SECURITY_PERM.search(block) and "put_to_security" not in eb.actions:
        eb.actions.append("put_to_security")
        eb.descriptive_tag = "put_to_security"

    # ── CanNotPutFieldClass — play restriction ──
    if RE_CANT_PUT_FIELD.search(block) and "play_restriction" not in eb.actions:
        eb.actions.append("play_restriction")
        eb.descriptive_tag = "play_restriction"

    # ── P4: Descriptive tagging for non-implementable effects ──
    if RE_IGNORE_COLOR.search(block) and "ignore_color_req" not in eb.actions:
        eb.descriptive_tag = "ignore_color_req"
        eb.actions.append("ignore_color_req")
    if RE_APP_FUSION.search(block) and "app_fusion_condition" not in eb.actions:
        eb.descriptive_tag = "app_fusion_condition"
        eb.actions.append("app_fusion_condition")
    if RE_LINK_CONDITION.search(block) and "link_condition" not in eb.actions:
        eb.descriptive_tag = "link_condition"
        eb.actions.append("link_condition")
    elif action == "also_treated_as":
        tag = getattr(eb, 'descriptive_tag', 'also_treated_as') or 'also_treated_as'
        if tag == "also_treated_as_level":
            lines.append(f"{indent}# Also treated as additional levels — metadata not modeled in engine")
        elif tag == "also_treated_as_name":
            lines.append(f"{indent}# Also treated as [Name] — name aliasing not modeled in engine")
        else:
            lines.append(f"{indent}# Also treated as [Name/Level] — metadata not modeled in engine")
        lines.append(f"{indent}pass  # descriptive-tagged: {tag}")
    elif action == "redirect_attack":
        lines.append(f"{indent}# Redirect attack target (SwitchDefender) — not yet in engine")
        lines.append(f"{indent}pass  # descriptive-tagged: redirect_attack")
    elif action == "effect_immunity":
        lines.append(f"{indent}# Grant effect immunity (CanNotAffectedClass) — not yet in engine")
        lines.append(f"{indent}pass  # descriptive-tagged: effect_immunity")
    elif action == "grant_skill":
        lines.append(f"{indent}# Grant keyword to other permanents (AddSkillClass) — not yet in engine")
        lines.append(f"{indent}pass  # descriptive-tagged: grant_skill")
    elif action == "attack_unsuspended":
        lines.append(f"{indent}# Can attack unsuspended Digimon (CanAttackTargetDefendingPermanentClass) — not yet in engine")
        lines.append(f"{indent}pass  # descriptive-tagged: attack_unsuspended")
    elif action == "play_restriction":
        lines.append(f"{indent}# Play restriction (CanNotPutFieldClass) — opponent play restrictions")
        lines.append(f"{indent}pass  # descriptive-tagged")

    # P5: New descriptive-tagged action types
    elif action == "play_token":
        token = eb.token_name or "Unknown"
        lines.append(f"{indent}# Play {token} Token — token play not yet supported in engine")
        lines.append(f"{indent}pass  # descriptive-tagged: play_token")
    elif action == "force_attack":
        lines.append(f"{indent}# Force attack — target Digimon may attack (requires engine SelectAttack)")
        lines.append(f"{indent}pass  # descriptive-tagged: force_attack")
    elif action == "change_security_attack":
        lines.append(f"{indent}# Grant Security Attack modifier to target permanent")
        lines.append(f"{indent}pass  # descriptive-tagged: change_security_attack")
    elif action == "disable_effect":
        lines.append(f"{indent}# Disable/invalidate effects on target — not yet in engine")
        lines.append(f"{indent}pass  # descriptive-tagged: disable_effect")
    elif action == "add_temp_effect":
        lines.append(f"{indent}# Grant temporary effect to target permanent")
        lines.append(f"{indent}pass  # descriptive-tagged: add_temp_effect")
    elif action == "put_to_security":
        lines.append(f"{indent}# Place a permanent into the security stack")
        lines.append(f"{indent}if not (player and game):")
        lines.append(f"{indent}    return")
        lines.append(f"{indent}def target_filter(p):")
        lines.append(f"{indent}    return p.is_digimon")
        lines.append(f"{indent}def on_put_security(target_perm):")
        lines.append(f"{indent}    if player:")
        lines.append(f"{indent}        player.put_permanent_to_security(target_perm)")
        lines.append(f"{indent}game.effect_select_own_permanent(")
        lines.append(f"{indent}    player, on_put_security, filter_fn=target_filter, is_optional={eb.is_optional})")


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

    # Keyword grants from CardEffectCommons.Gain*()
    gained = getattr(eb, 'gained_keywords', [])
    for kw in gained:
        flag_name = f"_is_{kw}"
        lines.append(f"        {var}.{flag_name} = True")

    # Condition — pass `effect` variable name for condition closures
    # We need the effect var name for accessing effect_source_permanent
    lines.append(f"")
    lines.append(f"        effect = {var}  # alias for condition closure")
    lines.append(f"        def condition{idx}(context: Dict[str, Any]) -> bool:")
    lines.append(generate_condition_code(eb, "            "))
    lines.append(f"")
    lines.append(f"        {var}.set_can_use_condition(condition{idx})")

    # Callback for actions
    # Phase 1: Skip callback entirely if jogress_condition is the only substantive action
    substantive_actions = [a for a in eb.actions if a not in (
        "jogress_condition", "draw", "gain_memory", "change_dp", "recovery")]
    has_non_jogress_actions = bool(substantive_actions) or eb.draw_count or eb.memory_gain or eb.dp_change or eb.recovery_count
    if eb.actions and has_non_jogress_actions:
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
