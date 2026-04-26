"""Structured text formatters for debug game output.

Produces plain-text output optimized for AI agents (Claude) to read.
Uses fixed-width markers and section headers. No JSON.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Dict, List, Optional

if TYPE_CHECKING:
    from engine_py_legacy.engine.game import Game
    from engine_py_legacy.engine.runners.debug_runner import (
        ActionResult, FieldSlot, StateSnapshot,
    )


def format_board(game: "Game", snap: "StateSnapshot") -> str:
    """One-screen board state overview."""
    lines = []

    # Header
    memory_dir = "→P1" if snap.memory >= 0 else "→P2"
    lines.append(
        f"══ BOARD ══  Turn {snap.turn} | Phase: {snap.phase} | "
        f"Memory: {snap.memory} ({memory_dir}) | Active: P{snap.active_player}"
    )

    if snap.is_game_over:
        winner = f"P{snap.winner}" if snap.winner else "Draw"
        lines.append(f"  ** GAME OVER — Winner: {winner} **")

    # Player 1
    lines.append(_format_player_zones(snap, 1))
    # Player 2
    lines.append(_format_player_zones(snap, 2))

    return "\n".join(lines)


def _format_player_zones(snap: "StateSnapshot", pid: int) -> str:
    """Format all zones for a single player."""
    lines = []
    prefix = f"P{pid}"

    hand = snap.p1_hand if pid == 1 else snap.p2_hand
    field_slots = snap.p1_field if pid == 1 else snap.p2_field
    security = snap.p1_security_count if pid == 1 else snap.p2_security_count
    trash = snap.p1_trash if pid == 1 else snap.p2_trash
    breeding = snap.p1_breeding if pid == 1 else snap.p2_breeding
    library = snap.p1_library_size if pid == 1 else snap.p2_library_size

    # Field
    lines.append(f"── {prefix} Field ({len(field_slots)}) ──")
    if field_slots:
        for slot in field_slots:
            lines.append(f"  {_format_field_slot(slot)}")
    else:
        lines.append("  (empty)")

    # Breeding
    if breeding:
        lines.append(f"── {prefix} Breeding ──  {breeding}")

    # Hand
    hand_str = " | ".join(hand) if hand else "(empty)"
    lines.append(f"── {prefix} Hand ({len(hand)}) ──  {hand_str}")

    # Summary line: security, trash, library
    trash_str = f"Trash: {len(trash)}" if trash else "Trash: 0"
    lines.append(f"   Security: {security} | {trash_str} | Library: {library}")

    return "\n".join(lines)


def _format_field_slot(slot: "FieldSlot") -> str:
    """Format a single permanent on the field."""
    parts = [f"[{slot.slot}]"]
    parts.append(f"{slot.card_name} ({slot.card_id})")

    if slot.level is not None:
        parts.append(f"Lv.{slot.level}")
    if slot.dp is not None:
        parts.append(f"DP:{slot.dp}")
    if slot.is_suspended:
        parts.append("[suspended]")
    if slot.is_tamer and not slot.is_digimon:
        parts.append("[tamer]")
    if len(slot.stack_ids) > 1:
        parts.append(f"stack:{len(slot.stack_ids)}")
    if slot.keywords:
        parts.append(f"keywords:[{','.join(slot.keywords)}]")

    return " ".join(parts)


def format_actions(actions: Dict[int, str]) -> str:
    """Format action list grouped by type."""
    if not actions:
        return "══ ACTIONS ══\n  (no legal actions)"

    # Group actions by category
    play = {}
    digivolve = {}
    attack = {}
    select = {}
    other = {}

    for aid, desc in sorted(actions.items()):
        desc_lower = desc.lower()
        if "play " in desc_lower or "use option" in desc_lower:
            play[aid] = desc
        elif "digivolve" in desc_lower or "dna digivolve" in desc_lower:
            digivolve[aid] = desc
        elif "attack" in desc_lower:
            attack[aid] = desc
        elif "select" in desc_lower or "choose" in desc_lower:
            select[aid] = desc
        else:
            other[aid] = desc

    lines = ["══ ACTIONS ══"]

    def _add_group(label: str, group: dict):
        if group:
            entries = " | ".join(f"{aid}→{desc}" for aid, desc in group.items())
            lines.append(f"  {label}: {entries}")

    _add_group("PLAY", play)
    _add_group("DIGI", digivolve)
    _add_group("ATTACK", attack)
    _add_group("SELECT", select)
    _add_group("OTHER", other)

    return "\n".join(lines)


def format_diff(before: "StateSnapshot", after: "StateSnapshot") -> str:
    """Show what changed between two snapshots."""
    changes = []

    # Memory
    if before.memory != after.memory:
        delta = after.memory - before.memory
        sign = "+" if delta > 0 else ""
        changes.append(f"Memory: {before.memory} → {after.memory} ({sign}{delta})")

    # Phase
    if before.phase != after.phase:
        changes.append(f"Phase: {before.phase} → {after.phase}")

    # Turn
    if before.turn != after.turn:
        changes.append(f"Turn: {before.turn} → {after.turn}")

    # Active player
    if before.active_player != after.active_player:
        changes.append(f"Active: P{before.active_player} → P{after.active_player}")

    # Per-player zone changes
    for pid in [1, 2]:
        prefix = f"P{pid}"

        # Hand
        hand_b = before.p1_hand if pid == 1 else before.p2_hand
        hand_a = after.p1_hand if pid == 1 else after.p2_hand
        if hand_b != hand_a:
            added = _list_diff_added(hand_b, hand_a)
            removed = _list_diff_removed(hand_b, hand_a)
            parts = []
            if added:
                parts.append(f"+[{','.join(added)}]")
            if removed:
                parts.append(f"-[{','.join(removed)}]")
            changes.append(f"{prefix} Hand: {' '.join(parts)} ({len(hand_b)}→{len(hand_a)})")

        # Field
        field_b = before.p1_field if pid == 1 else before.p2_field
        field_a = after.p1_field if pid == 1 else after.p2_field
        ids_b = {s.card_id for s in field_b}
        ids_a = {s.card_id for s in field_a}
        if ids_b != ids_a:
            entered = ids_a - ids_b
            left = ids_b - ids_a
            parts = []
            if entered:
                parts.append(f"+[{','.join(entered)}]")
            if left:
                parts.append(f"-[{','.join(left)}]")
            changes.append(f"{prefix} Field: {' '.join(parts)} ({len(field_b)}→{len(field_a)})")
        else:
            # Check for state changes (suspend, DP, etc.)
            for sb, sa in zip(field_b, field_a):
                if sb.is_suspended != sa.is_suspended:
                    state = "suspended" if sa.is_suspended else "unsuspended"
                    changes.append(f"{prefix} [{sb.slot}] {sb.card_id}: {state}")
                if sb.dp != sa.dp and sb.dp is not None and sa.dp is not None:
                    changes.append(f"{prefix} [{sb.slot}] {sb.card_id}: DP {sb.dp}→{sa.dp}")

        # Security
        sec_b = before.p1_security_count if pid == 1 else before.p2_security_count
        sec_a = after.p1_security_count if pid == 1 else after.p2_security_count
        if sec_b != sec_a:
            changes.append(f"{prefix} Security: {sec_b} → {sec_a}")

        # Trash
        trash_b = before.p1_trash if pid == 1 else before.p2_trash
        trash_a = after.p1_trash if pid == 1 else after.p2_trash
        if trash_b != trash_a:
            added = _list_diff_added(trash_b, trash_a)
            if added:
                changes.append(f"{prefix} Trash: +[{','.join(added)}] ({len(trash_b)}→{len(trash_a)})")
            elif len(trash_b) != len(trash_a):
                changes.append(f"{prefix} Trash: {len(trash_b)} → {len(trash_a)}")

        # Breeding
        breed_b = before.p1_breeding if pid == 1 else before.p2_breeding
        breed_a = after.p1_breeding if pid == 1 else after.p2_breeding
        if breed_b != breed_a:
            changes.append(f"{prefix} Breeding: {breed_b or 'empty'} → {breed_a or 'empty'}")

        # Library
        lib_b = before.p1_library_size if pid == 1 else before.p2_library_size
        lib_a = after.p1_library_size if pid == 1 else after.p2_library_size
        if lib_b != lib_a:
            changes.append(f"{prefix} Library: {lib_b} → {lib_a}")

    # Game over
    if not before.is_game_over and after.is_game_over:
        winner = f"P{after.winner}" if after.winner else "Draw"
        changes.append(f"** GAME OVER — Winner: {winner} **")

    if not changes:
        return "── DIFF ──  (no changes)"

    return "── DIFF ──\n" + "\n".join(f"  {c}" for c in changes)


def format_action_result(result: "ActionResult") -> str:
    """Format a complete action result: action + diff + key logs."""
    lines = [
        f"══ ACTION ══  [{result.action_id}] {result.action_description}",
        result.diff_summary,
    ]

    # Include key log lines (filter noise)
    key_logs = [
        log for log in result.logs
        if not log.startswith("[Phase]") and not log.startswith("[Turn]")
    ]
    if key_logs:
        lines.append("── LOGS ──")
        for log in key_logs[:15]:  # Cap at 15 lines
            lines.append(f"  {log}")
        if len(key_logs) > 15:
            lines.append(f"  ... ({len(key_logs) - 15} more)")

    return "\n".join(lines)


def _list_diff_added(before: List[str], after: List[str]) -> List[str]:
    """Find items in after that weren't in before (multiset diff)."""
    before_copy = list(before)
    added = []
    for item in after:
        if item in before_copy:
            before_copy.remove(item)
        else:
            added.append(item)
    return added


def _list_diff_removed(before: List[str], after: List[str]) -> List[str]:
    """Find items in before that aren't in after (multiset diff)."""
    after_copy = list(after)
    removed = []
    for item in before:
        if item in after_copy:
            after_copy.remove(item)
        else:
            removed.append(item)
    return removed
