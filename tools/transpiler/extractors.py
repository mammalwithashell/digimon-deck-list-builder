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
    RE_FACTORY_BLAST_DNA_DIGI, RE_FACTORY_CANNOT_DESTROYED_SKILL,
    RE_FACTORY_CANNOT_RETURN_HAND, RE_FACTORY_CANNOT_RETURN_DECK,
    RE_FACTORY_CANNOT_BE_BLOCKED, RE_FACTORY_REBOOT_NON_SELF,
    RE_FACTORY_ADD_DIGI_REQ, RE_FACTORY_CHANGE_DIGI_COST,
    RE_FACTORY_CHANGE_DIGI_COST_VALUE,
    RE_FACTORY_DIGI_REQ_COST, RE_FACTORY_DIGI_REQ_NAME, RE_FACTORY_DIGI_REQ_TRAIT,
    RE_FACTORY_DIGI_REQ_HAS_TS, RE_FACTORY_DIGI_REQ_HAS_APPMON,
    RE_FACTORY_DP_VALUE, RE_FACTORY_SA_VALUE,
    # P7: Stub reduction patterns
    RE_CHANGE_COST_VALUE,
    RE_IDEGENERATION, RE_SWITCH_DEFENDER, RE_PLAY_PERMANENT_CARDS, RE_DIGIVOLVE_INTO,
    RE_ADD_SKILL_CLASS,
    RE_ADD_JOGRESS_LEVELS, RE_CHANGE_CARD_NAMES, RE_CAN_ATTACK_TARGET,
    RE_CAN_NOT_AFFECTED,
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
    # ActivateCoroutine lambda delegate (Fix 13)
    RE_ACTIVATE_COROUTINE_LAMBDA,
    # P8: Modifier interface patterns
    RE_CANNOT_BE_DESTROYED, RE_CANNOT_BE_SELECTED,
    RE_CANNOT_UNSUSPEND, RE_CANNOT_ATTACK, RE_CANNOT_BLOCK,
    RE_IMMUNE_FROM_DP_MINUS, RE_CANNOT_BE_RETURNED,
    RE_EFFECT_IMMUNITY, RE_DONT_BATTLE_SECURITY,
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
        # Rare factory patterns (audit gap fix)
        (RE_FACTORY_BLAST_DNA_DIGI, "blast_dna_digivolve", "Blast DNA Digivolve"),
        (RE_FACTORY_CANNOT_DESTROYED_SKILL, "cannot_destroyed_by_skill", "Cannot Be Destroyed by Effects"),
        (RE_FACTORY_CANNOT_RETURN_HAND, "cannot_return_to_hand", "Cannot Return to Hand"),
        (RE_FACTORY_CANNOT_RETURN_DECK, "cannot_return_to_deck", "Cannot Return to Deck"),
        (RE_FACTORY_CANNOT_BE_BLOCKED, "cannot_be_blocked", "Cannot Be Blocked"),
        (RE_FACTORY_REBOOT_NON_SELF, "reboot_non_self", "Reboot (grant to others)"),
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
                # Shorthand trait properties (HasTSTraits, HasAppmonTraits)
                if RE_FACTORY_DIGI_REQ_HAS_TS.search(block) and "TS" not in eb.trait_checks:
                    eb.trait_checks.append("TS")
                if RE_FACTORY_DIGI_REQ_HAS_APPMON.search(block) and "Appmon" not in eb.trait_checks:
                    eb.trait_checks.append("Appmon")
                # Extract required base level (IsLevel2, IsLevel3, etc.)
                m_level = RE_CF_IS_LEVEL.search(block)
                if m_level:
                    eb.digi_level = int(m_level.group(1))
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

    # Fix 13: Handle ActivateCoroutine referenced via hashtable lambda delegate.
    # Pattern: hashtable => ActivateCoroutine(hashtable, activateClass)
    # Distinct from a timing block's local ActivateCoroutine (which is passed directly,
    # not wrapped in a lambda). When wrapped in a lambda, the method is outer-scoped
    # and shared across multiple timing blocks.
    m = RE_ACTIVATE_COROUTINE_LAMBDA.search(block)
    if m:
        body = _extract_method_body(full_source, "ActivateCoroutine")
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
    if RE_ALSO_TREATED_AS.search(block) and "also_treated_as" not in eb.actions:
        eb.descriptive_tag = "also_treated_as"
        eb.actions.append("also_treated_as")

    # ── P7 WI 3: Helper classes inside Mode.Custom callbacks ──
    # IDegeneration — de-digivolve via helper class (catches inline use in callbacks)
    m_idegen = RE_IDEGENERATION.search(block)
    if m_idegen and "de_digivolve" not in eb.actions:
        eb.actions.append("de_digivolve")
        eb.degen_count = int(m_idegen.group(1))
    # SwitchDefender — redirect attack target
    if RE_SWITCH_DEFENDER.search(block) and "redirect_attack" not in eb.actions:
        eb.actions.append("redirect_attack")
        eb.descriptive_tag = "redirect_attack"
    # PlayPermanentCards — play card via helper
    if RE_PLAY_PERMANENT_CARDS.search(block) and "play_card" not in eb.actions:
        eb.actions.append("play_card")
    # DigivolveIntoHandOrTrashCard — digivolve from hand or trash
    if RE_DIGIVOLVE_INTO.search(block) and "digivolve" not in eb.actions:
        eb.actions.append("digivolve")
    # CanNotAffectedClass — effect immunity grant
    if RE_CAN_NOT_AFFECTED.search(block) and "effect_immunity" not in eb.actions:
        eb.actions.append("effect_immunity")
        eb.descriptive_tag = "effect_immunity"

    # ── P7 WI 4: AddSkillClass — grants keywords to other permanents ──
    if RE_ADD_SKILL_CLASS.search(block) and "grant_skill" not in eb.actions:
        eb.actions.append("grant_skill")
        eb.descriptive_tag = "grant_skill"

    # ── P7 WI 5: Metadata-only classes ──
    if RE_ADD_JOGRESS_LEVELS.search(block) and "also_treated_as" not in eb.actions:
        eb.actions.append("also_treated_as")
        eb.descriptive_tag = "also_treated_as_level"
    if RE_CHANGE_CARD_NAMES.search(block) and "also_treated_as" not in eb.actions:
        eb.actions.append("also_treated_as")
        eb.descriptive_tag = "also_treated_as_name"
    if RE_CAN_ATTACK_TARGET.search(block) and "attack_unsuspended" not in eb.actions:
        eb.actions.append("attack_unsuspended")
        eb.descriptive_tag = "attack_unsuspended"

    # ── P8: Modifier interface patterns (Phase 2 — DCGO engine alignment) ──
    # These detect C# modifier classes and emit actions that now have engine support
    # via the ModifierRegistry system added in Phase 1.
    if RE_CANNOT_BE_DESTROYED.search(block) and "grant_destruction_immunity" not in eb.actions:
        eb.actions.append("grant_destruction_immunity")
    if RE_CANNOT_BE_SELECTED.search(block) and "grant_targeting_immunity" not in eb.actions:
        eb.actions.append("grant_targeting_immunity")
    if RE_CANNOT_UNSUSPEND.search(block) and "grant_cannot_unsuspend" not in eb.actions:
        eb.actions.append("grant_cannot_unsuspend")
    if RE_CANNOT_ATTACK.search(block) and "grant_cannot_attack" not in eb.actions:
        eb.actions.append("grant_cannot_attack")
    if RE_CANNOT_BLOCK.search(block) and "grant_cannot_block" not in eb.actions:
        eb.actions.append("grant_cannot_block")
    if RE_IMMUNE_FROM_DP_MINUS.search(block) and "grant_dp_minus_immunity" not in eb.actions:
        eb.actions.append("grant_dp_minus_immunity")
    if RE_CANNOT_BE_RETURNED.search(block) and "grant_bounce_immunity" not in eb.actions:
        eb.actions.append("grant_bounce_immunity")
    if RE_EFFECT_IMMUNITY.search(block) and "effect_immunity" not in eb.actions:
        eb.actions.append("effect_immunity")
    if RE_DONT_BATTLE_SECURITY.search(block) and "grant_no_security_battle" not in eb.actions:
        eb.actions.append("grant_no_security_battle")

    # ── P2: Mill detection — IAddTrashCardsFromLibraryTop ──
    m_mill = RE_MILL.search(block)
    if m_mill and "mill" not in eb.actions:
        eb.actions.append("mill")
        count_str = m_mill.group(1)
        try:
            eb.mill_count = int(count_str)
        except ValueError:
            eb.mill_count = 3  # default fallback for variable counts
        if "Enemy" in m_mill.group(2):
            eb.mill_target = "enemy"
        else:
            eb.mill_target = "self"

    # ── Target extraction: DP/level limits ──
    if eb.target_dp_limit is None:
        m = RE_TARGET_DP_LIMIT.search(block)
        if m:
            eb.target_dp_limit = int(m.group(1))
    if eb.target_dp_min is None:
        m = RE_TARGET_DP_MIN.search(block)
        if m:
            eb.target_dp_min = int(m.group(1))
    if eb.target_level_limit is None:
        m = RE_TARGET_LEVEL_LIMIT.search(block)
        if m:
            eb.target_level_limit = int(m.group(1))
    if eb.target_level_min is None:
        m = RE_TARGET_LEVEL_MIN.search(block)
        if m:
            eb.target_level_min = int(m.group(1))

    # ── Reveal count ──
    if eb.reveal_count is None:
        m = RE_REVEAL_COUNT.search(block)
        if m:
            eb.reveal_count = int(m.group(1) or m.group(2))

    # ── Play from zone ──
    if RE_PLAY_HAND_OR_TRASH.search(block):
        eb.play_from_zone = 'hand_or_trash'
        # Remove false trash_from_hand (SelectHandEffect used for play, not trash)
        if "trash_from_hand" in eb.actions and "play_card" in eb.actions:
            eb.actions.remove("trash_from_hand")
    elif RE_PLAY_FROM_TRASH.search(block):
        eb.play_from_zone = 'trash'
    if RE_PLAY_FREE.search(block):
        eb.play_free = True

    # ── Digivolve details ──
    if eb.digi_cost_override is None:
        m = RE_DIGI_COST_FIXED.search(block)
        if m:
            eb.digi_cost_override = int(m.group(1))
    if RE_DIGI_IGNORE_REQS.search(block):
        eb.digi_ignore_reqs = True

    # ── De-digivolve count ──
    m = RE_DEGEN_COUNT.search(block)
    if m and eb.degen_count is None:
        eb.degen_count = int(m.group(1))


def _extract_actions_from_block(block: str, eb: EffectBlock):
    """Extract actions from a C# block into an existing EffectBlock.

    Thin wrapper around _scan_actions() for backward compatibility.
    Called from SharedActivateCoroutine resolution and Mode.Custom callback.
    """
    _scan_actions(block, eb)

    # De-digivolve count
    if eb.degen_count is None:
        m = RE_DEGEN_COUNT.search(block)
        if m:
            eb.degen_count = int(m.group(1))


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


def _merge_kinds(a: Optional[str], b: Optional[str]) -> Optional[str]:
    """Merge two card kind constraints with OR semantics.

    None means unconstrained. If either side is None, result is None.
    Otherwise, union the kinds.
    """
    if a is None or b is None:
        return None
    if a == b:
        return a
    kinds = set()
    for k in (a, b):
        if "_or_" in k:
            kinds.update(k.split("_or_"))
        else:
            kinds.add(k)
    return "_or_".join(sorted(kinds))


def _parse_filter_body_to_dict(body: str) -> dict:
    """Parse a condition function body into a dict of filter values.

    Returns dict with keys: traits, names, cost_max, cost_min, level_max,
    level_min, colors, kind, exclude_digi_egg, has_play_cost.
    """
    traits: List[str] = []
    for m in RE_CF_EQUALS_TRAITS.finditer(body):
        if m.group(1) not in traits:
            traits.append(m.group(1))
    for m in RE_CF_CONTAINS_TRAITS.finditer(body):
        if m.group(1) not in traits:
            traits.append(m.group(1))
    for m in RE_CF_HAS_TRAITS.finditer(body):
        prop_name = m.group(1)
        trait_str = HAS_TRAITS_MAP.get(prop_name)
        if trait_str and trait_str not in traits:
            traits.append(trait_str)

    names: List[str] = []
    for m in RE_CF_EQUALS_NAME.finditer(body):
        if m.group(1) not in names:
            names.append(m.group(1))
    for m in RE_CF_CONTAINS_NAME.finditer(body):
        if m.group(1) not in names:
            names.append(m.group(1))

    cost_max: Optional[int] = None
    m = RE_CF_COST_MAX.search(body)
    if m:
        cost_max = int(m.group(1))

    cost_min: Optional[int] = None
    m = RE_CF_COST_MIN.search(body)
    if m:
        cost_min = int(m.group(1))

    level_max: Optional[int] = None
    m = RE_CF_LEVEL_MAX.search(body)
    if m:
        level_max = int(m.group(1))

    level_min: Optional[int] = None
    m = RE_CF_LEVEL_MIN.search(body)
    if m:
        level_min = int(m.group(1))

    for m in RE_CF_IS_LEVEL.finditer(body):
        level_val = int(m.group(1))
        if level_max is None:
            level_max = level_val
        if level_min is None:
            level_min = level_val

    colors: List[str] = []
    for m in RE_CF_COLOR.finditer(body):
        if m.group(1) not in colors:
            colors.append(m.group(1))

    # Strip C# lambda expressions before kind-checking to avoid false positives
    kind_body = RE_CS_LAMBDA.sub("", body)
    kind: Optional[str] = None
    if RE_CF_IS_DIGIMON.search(kind_body):
        kind = "Digimon"
    if RE_CF_IS_TAMER.search(kind_body):
        if kind == "Digimon":
            kind = "Digimon_or_Tamer"
        elif kind is None:
            kind = "Tamer"
    if RE_CF_IS_OPTION.search(kind_body):
        if kind is None:
            kind = "Option"

    return {
        "traits": traits,
        "names": names,
        "cost_max": cost_max,
        "cost_min": cost_min,
        "level_max": level_max,
        "level_min": level_min,
        "colors": colors,
        "kind": kind,
        "exclude_digi_egg": bool(RE_CF_NOT_DIGI_EGG.search(body)),
        "has_play_cost": bool(RE_CF_HAS_PLAY_COST.search(body)),
    }


def _parse_filter_body(body: str, eb: EffectBlock, is_first: bool):
    """Parse a single condition function body and merge into eb.card_filter_* fields.

    For the first body, sets values directly. For subsequent bodies, applies
    OR-merge semantics (widen ranges, union sets).
    """
    d = _parse_filter_body_to_dict(body)

    local_traits = d["traits"]
    local_names = d["names"]
    local_cost_max = d["cost_max"]
    local_cost_min = d["cost_min"]
    local_level_max = d["level_max"]
    local_level_min = d["level_min"]
    local_colors = d["colors"]
    local_kind = d["kind"]
    local_exclude_digi_egg = d["exclude_digi_egg"]
    local_has_play_cost = d["has_play_cost"]

    # ── Merge into EffectBlock ──
    if is_first:
        eb.card_filter_traits = local_traits
        eb.card_filter_names = local_names
        eb.card_filter_cost_max = local_cost_max
        eb.card_filter_cost_min = local_cost_min
        eb.card_filter_level_max = local_level_max
        eb.card_filter_level_min = local_level_min
        eb.card_filter_colors = local_colors
        eb.card_filter_kind = local_kind
        eb.card_filter_exclude_digi_egg = local_exclude_digi_egg
        eb.card_filter_has_play_cost = local_has_play_cost
    else:
        # OR-merge: union sets
        for t in local_traits:
            if t not in eb.card_filter_traits:
                eb.card_filter_traits.append(t)
        for n in local_names:
            if n not in eb.card_filter_names:
                eb.card_filter_names.append(n)
        for c in local_colors:
            if c not in eb.card_filter_colors:
                eb.card_filter_colors.append(c)

        # OR-merge cost/level: if any body is unconstrained, result is unconstrained
        if local_cost_max is None:
            eb.card_filter_cost_max = None
        elif eb.card_filter_cost_max is not None:
            eb.card_filter_cost_max = max(eb.card_filter_cost_max, local_cost_max)

        if local_cost_min is None:
            eb.card_filter_cost_min = None
        elif eb.card_filter_cost_min is not None:
            eb.card_filter_cost_min = min(eb.card_filter_cost_min, local_cost_min)

        if local_level_max is None:
            eb.card_filter_level_max = None
        elif eb.card_filter_level_max is not None:
            eb.card_filter_level_max = max(eb.card_filter_level_max, local_level_max)

        if local_level_min is None:
            eb.card_filter_level_min = None
        elif eb.card_filter_level_min is not None:
            eb.card_filter_level_min = min(eb.card_filter_level_min, local_level_min)

        # OR-merge kind
        eb.card_filter_kind = _merge_kinds(eb.card_filter_kind, local_kind)

        # OR-merge booleans: if any body doesn't require it, don't require it
        if not local_exclude_digi_egg:
            eb.card_filter_exclude_digi_egg = False
        if not local_has_play_cost:
            eb.card_filter_has_play_cost = False


def _extract_card_filter_conditions(block: str, full_source: str, eb: EffectBlock):
    """Extract card selection filter from CanSelectCardCondition lambda body.

    Searches for CanSelectCardCondition/CardCondition/SelectDigimonCondition
    definitions in the block and full source. Also finds numbered variants
    (CanSelectCardCondition1, 2, ...) used in multi-pass reveal operations.
    Parses all found bodies with OR-merge semantics into eb.card_filter_* fields.
    """
    filter_actions = {"play_card", "reveal_and_select", "add_to_hand", "digivolve"}
    if not any(a in filter_actions for a in eb.actions):
        return

    # Priority order: specific names first, generic CardCondition last
    # (CardCondition is also used for IgnoreColorConditionClass and similar,
    #  so it can produce false positives if checked too early)
    base_names = ("CanSelectCardCondition", "CanSelectDNACardCondition",
                  "SelectDigimonCondition", "CardCondition")

    # Collect all condition bodies (base + numbered variants)
    all_bodies: List[str] = []
    for func_name in base_names:
        body = _extract_method_body(block, func_name)
        found_in_block = bool(body)
        if not body and full_source and full_source != block:
            body = _extract_method_body(full_source, func_name)
        if not body:
            continue

        # For generic "CardCondition", skip if the body is trivially
        # just an identity check (e.g., "cardSource == card") with no
        # card filter patterns.
        if func_name == "CardCondition":
            has_filter_pattern = any(p.search(body) for p in (
                RE_CF_IS_DIGIMON, RE_CF_IS_TAMER, RE_CF_IS_OPTION,
                RE_CF_COST_MAX, RE_CF_COST_MIN,
                RE_CF_LEVEL_MAX, RE_CF_LEVEL_MIN, RE_CF_IS_LEVEL,
                RE_CF_EQUALS_TRAITS, RE_CF_CONTAINS_TRAITS, RE_CF_HAS_TRAITS,
                RE_CF_EQUALS_NAME, RE_CF_CONTAINS_NAME,
                RE_CF_HAS_PLAY_COST, RE_CF_NOT_DIGI_EGG, RE_CF_COLOR))
            if not has_filter_pattern:
                continue

        all_bodies.append(body)
        # Search for numbered variants (CanSelectCardCondition1, 2, ...)
        # Only search in the same scope where the base name was found
        # to avoid picking up variants from other timing blocks.
        if func_name == "CanSelectCardCondition":
            search_scope = block if found_in_block else full_source
            for suffix in range(1, 10):
                variant_name = f"{func_name}{suffix}"
                variant_body = _extract_method_body(search_scope, variant_name)
                if variant_body:
                    all_bodies.append(variant_body)
                else:
                    break
        break  # Found a usable base name, don't try others

    if not all_bodies:
        return

    # Parse each body and merge results into card_filter_* (OR-merge)
    for i, body in enumerate(all_bodies):
        _parse_filter_body(body, eb, is_first=(i == 0))

    # For multi-pass reveals, also populate per-pass filter data
    if len(all_bodies) > 1 and "reveal_and_select" in eb.actions:
        # Extract per-pass modes from SimplifiedSelectCardConditionClass array
        search_src = block if "SimplifiedSelectCardConditionClass" in block else full_source
        pass_entries = RE_REVEAL_PASS_ENTRY.findall(search_src)
        mode_map = {name: mode for name, mode in pass_entries}

        for idx, body in enumerate(all_bodies):
            pass_data = _parse_filter_body_to_dict(body)
            # Determine the condition name for this body to look up its mode
            if idx == 0:
                cond_name = "CanSelectCardCondition"
            else:
                cond_name = f"CanSelectCardCondition{idx}"
            mode = mode_map.get(cond_name, "AddHand")
            # Map C# modes to engine placement strings
            if mode == "AddHand":
                pass_data["placement"] = "hand"
            elif mode == "Custom":
                pass_data["placement"] = "hand"  # Custom typically plays, default to hand
            elif mode == "Discard":
                pass_data["placement"] = "trash"
            else:
                pass_data["placement"] = "hand"
            eb.card_filter_passes.append(pass_data)


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

        # Then ActivateClass effects (pass full source for shared coroutine resolution)
        activate_effects = extract_activate_effects(block, full_source=source)
        for ae in activate_effects:
            ae.timing = timing
            all_effects.append(ae)

    # Also check for effects defined outside timing blocks (top-level)
    if not timing_blocks:
        factory_effects = extract_factory_effects(source)
        activate_effects = extract_activate_effects(source, full_source=source)
        for fe in factory_effects:
            all_effects.append(fe)
        for ae in activate_effects:
            all_effects.append(ae)

    return class_name, all_effects
