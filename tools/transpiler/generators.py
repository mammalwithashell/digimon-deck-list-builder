"""Python code generation from EffectBlock structures."""
from typing import List, Optional, Dict

from .models import EffectBlock
from .patterns import TIMING_TO_PROPERTY


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

    # Fix 13: Triggers checking event source (suspended permanent)
    if "EffectTiming.OnTappedAnyone" in eb.timing:
        # Check color conditions against suspended permanent
        if eb.color_checks:
            or_parts = " or ".join(f"CardColor.{c} in suspended.top_card.card_colors" for c in eb.color_checks)
            checks.append(f"{indent}suspended = context.get('suspended_permanent')")
            checks.append(f"{indent}if suspended:")
            checks.append(f"{indent}    if not ({or_parts}):")
            checks.append(f"{indent}        return False")

        # Check trait/name conditions against suspended permanent?
        # Typically "One of your [Trait] becomes suspended"
        if eb.trait_checks:
             # Need to distinguish if traits apply to self or the event source.
             # If "Anyone", usually implies context check.
             # Simplification: if inherited, maybe check self?
             # But "When one of your ... becomes suspended" means event source.
             # We'll apply traits to suspended permanent if present.
             or_parts = " or ".join(f"'{t}' in suspended.top_card.card_traits" for t in eb.trait_checks)
             checks.append(f"{indent}suspended = context.get('suspended_permanent')")
             checks.append(f"{indent}if suspended:")
             checks.append(f"{indent}    if not ({or_parts}):")
             checks.append(f"{indent}        return False")

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


def _build_target_filter_code(eb: EffectBlock, indent: str = "            ") -> List[str]:
    """Build filter condition lines for opponent permanent targeting.

    Includes DP/level checks and name/trait checks. Name/trait filters are
    always applied — effects that target permanents by name or trait (e.g.
    "Delete 1 Digimon with [Dragon] in its type") need these filters on the
    permanent selection, not just on card selection.
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
    # Use OR logic for combined name/trait checks
    if eb.name_checks or eb.trait_checks:
        parts.extend(_build_or_filter_perm(eb.name_checks, eb.trait_checks, indent))
    return parts


def _build_filter_from_dict(d: dict, indent: str = "            ") -> List[str]:
    """Build filter condition lines from a pass data dict.

    Used for multi-pass reveal where each pass has its own filter dict.
    """
    parts = []
    kind = d.get("kind")
    if kind:
        if kind == "Digimon_or_Tamer":
            parts.append(f"{indent}    if not (getattr(c, 'is_digimon', False) or getattr(c, 'is_tamer', False)):")
            parts.append(f"{indent}        return False")
        else:
            kind_lower = kind.lower()
            parts.append(f"{indent}    if not getattr(c, 'is_{kind_lower}', False):")
            parts.append(f"{indent}        return False")
    if d.get("exclude_digi_egg"):
        parts.append(f"{indent}    if getattr(c, 'is_digi_egg', False):")
        parts.append(f"{indent}        return False")
    if d.get("has_play_cost"):
        parts.append(f"{indent}    if not getattr(c, 'has_play_cost', False):")
        parts.append(f"{indent}        return False")
    if d.get("cost_max") is not None:
        parts.append(f"{indent}    if getattr(c, 'get_cost_itself', 0) > {d['cost_max']}:")
        parts.append(f"{indent}        return False")
    if d.get("cost_min") is not None:
        parts.append(f"{indent}    if getattr(c, 'get_cost_itself', 0) < {d['cost_min']}:")
        parts.append(f"{indent}        return False")
    if d.get("level_max") is not None:
        parts.append(f"{indent}    if getattr(c, 'level', None) is None or c.level > {d['level_max']}:")
        parts.append(f"{indent}        return False")
    if d.get("level_min") is not None:
        parts.append(f"{indent}    if getattr(c, 'level', None) is None or c.level < {d['level_min']}:")
        parts.append(f"{indent}        return False")
    colors = d.get("colors", [])
    if colors:
        color_checks = " or ".join(
            f"'{col}' in [col.name for col in getattr(c, 'card_colors', [])]"
            for col in colors)
        parts.append(f"{indent}    if not ({color_checks}):")
        parts.append(f"{indent}        return False")
    or_clauses = []
    names = d.get("names", [])
    traits = d.get("traits", [])
    if names:
        name_ors = " or ".join(f"'{n}' in _n" for n in names)
        or_clauses.append(f"any({name_ors} for _n in getattr(c, 'card_names', []))")
    if traits:
        trait_ors = " or ".join(f"'{t}' in _t" for t in traits)
        or_clauses.append(f"any({trait_ors} for _t in (getattr(c, 'card_traits', []) or []))")
    if or_clauses:
        combined = " or ".join(or_clauses)
        parts.append(f"{indent}    if not ({combined}):")
        parts.append(f"{indent}        return False")
    return parts


def _build_card_filter_code(eb: EffectBlock, indent: str = "            ") -> List[str]:
    """Build filter condition lines for card selection (hand/reveal/trash).

    Prefers card_filter_* fields (from CanSelectCardCondition extraction)
    over trait_checks/name_checks (from whole-block scanning).
    """
    parts = []

    has_card_filter = (eb.card_filter_traits or eb.card_filter_names or
                       eb.card_filter_cost_max is not None or
                       eb.card_filter_cost_min is not None or
                       eb.card_filter_level_max is not None or
                       eb.card_filter_level_min is not None or
                       eb.card_filter_colors or eb.card_filter_kind or
                       eb.card_filter_exclude_digi_egg or
                       eb.card_filter_has_play_cost)

    if has_card_filter:
        # Delegate to _build_filter_from_dict using the EB's merged fields
        d = {
            "kind": eb.card_filter_kind,
            "exclude_digi_egg": eb.card_filter_exclude_digi_egg,
            "has_play_cost": eb.card_filter_has_play_cost,
            "cost_max": eb.card_filter_cost_max,
            "cost_min": eb.card_filter_cost_min,
            "level_max": eb.card_filter_level_max,
            "level_min": eb.card_filter_level_min,
            "colors": eb.card_filter_colors,
            "names": eb.card_filter_names,
            "traits": eb.card_filter_traits,
        }
        parts = _build_filter_from_dict(d, indent)
    else:
        # Fallback: use whole-block trait_checks/name_checks (backward compat)
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

    # Note: name/trait filters are now always applied to both permanent and card
    # targeting. The previous perm_only logic was too aggressive in stripping
    # filters from permanent selection in multi-action blocks.

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
            _emit_action(eb, action, lines, indent)
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
            _emit_action(eb, action, lines, indent)

    # WI 6: Determine if callback has substantive content
    # Lines that don't count: preamble ctx.get() calls, pure comments, blank lines
    preamble_lines = {
        f"player = ctx.get('player')",
        f"perm = ctx.get('permanent')",
        f"game = ctx.get('game')",
    }

    def _is_substantive(l):
        stripped = l.strip()
        if not stripped:
            return False
        if stripped in preamble_lines:
            return False
        if stripped.startswith('#'):
            return False
        return True

    if not any(_is_substantive(l) for l in lines):
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


def _emit_action(eb: EffectBlock, action: str, lines: List[str], indent: str):
    """Emit code for a single action type."""
    if action == "delete":
        lines.append(f"{indent}if not (player and game):")
        lines.append(f"{indent}    return")
        target_filter = _build_target_filter_code(eb, indent)
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
    elif action == "delete_and_process":
        # Complex conditional deletion logic
        lines.append(f"{indent}# Complex conditional deletion (BT13-111 style)")
        lines.append(f"{indent}enemy = player.enemy if player else None")
        lines.append(f"{indent}if enemy:")
        lines.append(f"{indent}    # Attempt first deletion condition")
        lines.append(f"{indent}    def first_filter(p):")
        if eb.target_dp_limit:
            lines.append(f"{indent}        return p.dp is not None and p.dp <= {eb.target_dp_limit}")
        else:
            lines.append(f"{indent}        return False # Fallback if no condition extracted")
        lines.append(f"{indent}    ")
        lines.append(f"{indent}    def on_first_delete(target):")
        lines.append(f"{indent}        deleted = enemy.delete_permanent(target)")
        lines.append(f"{indent}        if not deleted:")
        lines.append(f"{indent}            # If failure (not deleted), trigger secondary logic")
        lines.append(f"{indent}            # TODO: Extract secondary logic from FailureProcess")
        lines.append(f"{indent}            pass")
        lines.append(f"{indent}    ")
        lines.append(f"{indent}    # Simplified implementation: Check if any match, then ask selection")
        lines.append(f"{indent}    game.effect_select_opponent_permanent(player, on_first_delete, filter_fn=first_filter, is_optional={eb.is_optional})")
        lines.append(f"{indent}pass  # descriptive-tagged: delete_and_process")
    elif action == "bounce":
        lines.append(f"{indent}if not (player and game):")
        lines.append(f"{indent}    return")
        target_filter = _build_target_filter_code(eb, indent)
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
        target_filter = _build_target_filter_code(eb, indent)
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
        if len(eb.card_filter_passes) > 1:
            # Multi-pass reveal: emit per-pass filter functions
            for pidx, pdata in enumerate(eb.card_filter_passes):
                lines.append(f"{indent}def reveal_filter_{pidx}(c):")
                pfilter = _build_filter_from_dict(pdata, indent)
                if pfilter:
                    lines.extend(pfilter)
                    lines.append(f"{indent}    return True")
                else:
                    lines.append(f"{indent}    return True")
            passes_items = ", ".join(
                f"(reveal_filter_{pidx}, '{pdata.get('placement', 'hand')}')"
                for pidx, pdata in enumerate(eb.card_filter_passes)
            )
            lines.append(f"{indent}game.effect_reveal_and_select_multi(")
            lines.append(f"{indent}    player, {count}, [{passes_items}],")
            lines.append(f"{indent}    remaining_placement='deck_bottom', is_optional=True)")
        else:
            # Single-pass reveal (original behavior)
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
        val = eb.cost_reduction_val
        if val:
            lines.append(f"{indent}# Cost reduction by {val} — handled via cost_reduction property")
        else:
            lines.append(f"{indent}# Cost reduction (variable amount) — handled via cost_reduction property")
        lines.append(f"{indent}pass  # descriptive-tagged: cost_reduction")
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
        target_filter = _build_target_filter_code(eb, indent)
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
    elif action.startswith("gain_keyword_"):
        # Keyword grant — collect ALL gain_keyword_* actions for this effect
        all_grant_keywords = [a[len("gain_keyword_"):] for a in eb.actions if a.startswith("gain_keyword_")]
        # Only emit once (for the first gain_keyword_* action encountered)
        first_grant_action = f"gain_keyword_{all_grant_keywords[0]}" if all_grant_keywords else action
        if action != first_grant_action:
            pass  # Skip — already emitted by the first gain_keyword_* action
        else:
            # Check if gain_keyword is the ONLY substantive action (targets another perm)
            other_actions = [a for a in eb.actions
                            if not a.startswith("gain_keyword_") and a not in (
                                "draw", "gain_memory", "change_dp", "recovery", "cost_reduction")]
            if not other_actions and eb.grant_is_self:
                # Self-grant path: no selection needed, grant to own permanent
                lines.append(f"{indent}if perm:")
                for kw in all_grant_keywords:
                    lines.append(f"{indent}    perm.grant_keyword('_is_{kw}')")
            elif not other_actions:
                # Target-grant path: grant keyword(s) to a selected permanent
                lines.append(f"{indent}if not (player and game):")
                lines.append(f"{indent}    return")

                if eb.grant_has_reference_selection:
                    # Two-step selection: choose reference perm first, then filter targets
                    lines.append(f"{indent}def on_select_reference(ref_perm):")
                    lines.append(f"{indent}    ref_digi_count = len(ref_perm.digivolution_cards)")
                    target_filter = _build_target_filter_code(eb, indent + "    ")
                    lines.append(f"{indent}    def target_filter(p):")
                    if target_filter:
                        lines.extend(target_filter)
                    if eb.grant_reference_filter == "digi_count_lte":
                        lines.append(f"{indent}        if len(p.digivolution_cards) > ref_digi_count:")
                        lines.append(f"{indent}            return False")
                    lines.append(f"{indent}        return p.is_digimon")
                    lines.append(f"{indent}    def on_grant(target_perm):")
                    for kw in all_grant_keywords:
                        lines.append(f"{indent}        target_perm.grant_keyword('_is_{kw}')")
                    select_method = "effect_select_opponent_permanent" if eb.grant_target_is_opponent else "effect_select_own_permanent"
                    lines.append(f"{indent}    game.{select_method}(")
                    lines.append(f"{indent}        player, on_grant, filter_fn=target_filter, is_optional={eb.is_optional})")
                    lines.append(f"{indent}game.effect_select_own_permanent(")
                    lines.append(f"{indent}    player, on_select_reference, filter_fn=lambda p: p.is_digimon, is_optional=False)")
                else:
                    # Standard single-step target selection
                    target_filter = _build_target_filter_code(eb, indent)
                    lines.append(f"{indent}def target_filter(p):")
                    if target_filter:
                        lines.extend(target_filter)
                        lines.append(f"{indent}    return p.is_digimon")
                    else:
                        lines.append(f"{indent}    return p.is_digimon")
                    lines.append(f"{indent}def on_grant(target_perm):")
                    for kw in all_grant_keywords:
                        lines.append(f"{indent}    target_perm.grant_keyword('_is_{kw}')")
                    select_method = "effect_select_opponent_permanent" if eb.grant_target_is_opponent else "effect_select_own_permanent"
                    lines.append(f"{indent}game.{select_method}(")
                    lines.append(f"{indent}    player, on_grant, filter_fn=target_filter, is_optional={eb.is_optional})")
            else:
                # Self-grant path: keyword applies to this permanent via callback
                lines.append(f"{indent}if perm:")
                for kw in all_grant_keywords:
                    lines.append(f"{indent}    perm.grant_keyword('_is_{kw}')")
    elif action == "mill":
        # P2: Mill — trash cards from top of deck
        count = eb.mill_count or 3
        if eb.mill_target == "enemy":
            lines.append(f"{indent}# Mill {count} cards from opponent's deck")
            lines.append(f"{indent}enemy = player.enemy if player else None")
            lines.append(f"{indent}if enemy and enemy.library_cards:")
            lines.append(f"{indent}    mill_count = min({count}, len(enemy.library_cards))")
            lines.append(f"{indent}    trashed = enemy.library_cards[:mill_count]")
            lines.append(f"{indent}    enemy.library_cards = enemy.library_cards[mill_count:]")
            lines.append(f"{indent}    enemy.trash_cards.extend(trashed)")
        else:
            lines.append(f"{indent}# Mill {count} cards from own deck")
            lines.append(f"{indent}if player and player.library_cards:")
            lines.append(f"{indent}    mill_count = min({count}, len(player.library_cards))")
            lines.append(f"{indent}    trashed = player.library_cards[:mill_count]")
            lines.append(f"{indent}    player.library_cards = player.library_cards[mill_count:]")
            lines.append(f"{indent}    player.trash_cards.extend(trashed)")
    elif action == "ignore_color_req":
        # P4: Descriptive tag
        lines.append(f"{indent}# Ignores color requirement for playing Options — not modeled in engine")
        lines.append(f"{indent}pass  # descriptive-tagged")
    elif action == "app_fusion_condition":
        lines.append(f"{indent}# App Fusion condition — not yet supported")
        lines.append(f"{indent}pass  # descriptive-tagged")
    elif action == "link_condition":
        lines.append(f"{indent}# Link condition setup — not yet supported")
        lines.append(f"{indent}pass  # descriptive-tagged")
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
    lines.append("from ....data.enums import CardColor")
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
