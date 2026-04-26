"""Core state manipulation functions for debug games.

Shared by both the HTTP debug endpoints and the in-process DebugRunner.
All card creation goes through CardDatabase.create_card_source() so real
card scripts are attached and effects fire correctly.
"""

from __future__ import annotations

from dataclasses import dataclass, field as dataclass_field
from typing import TYPE_CHECKING, List, Optional

from engine_py_legacy.engine.data.card_database import CardDatabase
from engine_py_legacy.engine.data.card_registry import CardRegistry
from engine_py_legacy.engine.core.permanent import Permanent

if TYPE_CHECKING:
    from engine_py_legacy.engine.game import Game
    from engine_py_legacy.engine.core.card_source import CardSource
    from engine_py_legacy.engine.core.player import Player


def _get_player(game: Game, player_id: int) -> Player:
    if player_id == 1:
        return game.player1
    elif player_id == 2:
        return game.player2
    else:
        raise ValueError(f"player_id must be 1 or 2, got {player_id}")


def _create_card(card_id: str, player: Player) -> CardSource:
    """Create a CardSource with real scripts via CardDatabase."""
    CardRegistry.ensure_initialized()
    db = CardDatabase()
    cs = db.create_card_source(card_id, player)
    if cs is None:
        raise ValueError(f"Card ID '{card_id}' not found in card database")
    return cs


def place_on_field(
    game: Game,
    player_id: int,
    card_ids: List[str],
    is_suspended: bool = False,
    turn_played: int = -1,
) -> Permanent:
    """Place a digivolution stack on a player's battle area.

    Args:
        game: The Game instance.
        player_id: 1 or 2.
        card_ids: Card IDs bottom-to-top (e.g. ["egg", "rookie", "champion"]).
        is_suspended: Whether the permanent starts suspended.
        turn_played: Turn this permanent entered the field.
            -1 means no summoning sickness.

    Returns:
        The created Permanent.
    """
    player = _get_player(game, player_id)
    card_sources = [_create_card(cid, player) for cid in card_ids]
    perm = Permanent(card_sources)
    perm.is_suspended = is_suspended
    perm.turn_played = turn_played
    perm._owner_game = game
    player.battle_area.append(perm)
    return perm


def place_in_breeding(
    game: Game,
    player_id: int,
    card_ids: List[str],
) -> Permanent:
    """Place a digivolution stack in a player's breeding area.

    Args:
        game: The Game instance.
        player_id: 1 or 2.
        card_ids: Card IDs bottom-to-top.

    Returns:
        The created Permanent.
    """
    player = _get_player(game, player_id)
    card_sources = [_create_card(cid, player) for cid in card_ids]
    perm = Permanent(card_sources)
    perm._owner_game = game
    player.breeding_area = perm
    return perm


def inject_card(
    game: Game,
    player_id: int,
    card_id: str,
    zone: str = "hand",
) -> CardSource:
    """Inject a single card into a player's zone.

    Args:
        game: The Game instance.
        player_id: 1 or 2.
        card_id: Card ID to inject.
        zone: One of "hand", "library_top", "library_bottom", "security_top",
              "security_bottom", "trash".

    Returns:
        The created CardSource.
    """
    player = _get_player(game, player_id)
    cs = _create_card(card_id, player)

    if zone == "hand":
        player.hand_cards.append(cs)
    elif zone == "library_top":
        player.library_cards.insert(0, cs)
    elif zone == "library_bottom":
        player.library_cards.append(cs)
    elif zone == "security_top":
        player.security_cards.insert(0, cs)
    elif zone == "security_bottom":
        player.security_cards.append(cs)
    elif zone == "trash":
        player.trash_cards.append(cs)
    else:
        raise ValueError(
            f"Invalid zone '{zone}'. Must be one of: hand, library_top, "
            f"library_bottom, security_top, security_bottom, trash"
        )
    return cs


def clear_zone(game: Game, player_id: int, zone: str) -> int:
    """Remove all cards from a player's zone.

    Args:
        game: The Game instance.
        player_id: 1 or 2.
        zone: One of "hand", "field", "security", "trash", "breeding", "library".

    Returns:
        Number of items cleared.
    """
    player = _get_player(game, player_id)

    if zone == "hand":
        count = len(player.hand_cards)
        player.hand_cards.clear()
    elif zone == "field":
        count = len(player.battle_area)
        player.battle_area.clear()
    elif zone == "security":
        count = len(player.security_cards)
        player.security_cards.clear()
    elif zone == "trash":
        count = len(player.trash_cards)
        player.trash_cards.clear()
    elif zone == "breeding":
        count = 1 if player.breeding_area is not None else 0
        player.breeding_area = None
    elif zone == "library":
        count = len(player.library_cards)
        player.library_cards.clear()
    else:
        raise ValueError(
            f"Invalid zone '{zone}'. Must be one of: hand, field, security, "
            f"trash, breeding, library"
        )
    return count


@dataclass
class FieldEntry:
    """A digivolution stack to place on the field."""
    card_ids: List[str]
    is_suspended: bool = False
    turn_played: int = -1  # -1 = no summoning sickness


@dataclass
class ZoneSetup:
    """Per-player zone setup configuration."""
    hand: List[str] = dataclass_field(default_factory=list)
    library_top: List[str] = dataclass_field(default_factory=list)
    security: List[str] = dataclass_field(default_factory=list)
    trash: List[str] = dataclass_field(default_factory=list)
    field: List[FieldEntry] = dataclass_field(default_factory=list)
    breeding: List[str] = dataclass_field(default_factory=list)


@dataclass
class BulkSetupConfig:
    """Configuration for bulk state manipulation."""
    player1: ZoneSetup = dataclass_field(default_factory=ZoneSetup)
    player2: ZoneSetup = dataclass_field(default_factory=ZoneSetup)
    memory: Optional[int] = None
    phase: Optional[str] = None

    @classmethod
    def from_dict(cls, data: dict) -> BulkSetupConfig:
        """Create from a plain dict (e.g. from YAML or JSON)."""
        def parse_zone(d: dict) -> ZoneSetup:
            field_entries = []
            for f in d.get("field", []):
                if isinstance(f, dict):
                    field_entries.append(FieldEntry(
                        card_ids=f.get("card_ids", []),
                        is_suspended=f.get("is_suspended", False),
                        turn_played=f.get("turn_played", -1),
                    ))
                elif isinstance(f, list):
                    field_entries.append(FieldEntry(card_ids=f))
            return ZoneSetup(
                hand=d.get("hand", []),
                library_top=d.get("library_top", []),
                security=d.get("security", []),
                trash=d.get("trash", []),
                field=field_entries,
                breeding=d.get("breeding", []),
            )

        return cls(
            player1=parse_zone(data.get("player1", {})),
            player2=parse_zone(data.get("player2", {})),
            memory=data.get("memory"),
            phase=data.get("phase"),
        )


def bulk_setup(game: Game, config: BulkSetupConfig) -> dict:
    """Apply bulk state manipulation to a game.

    Injects cards into all specified zones for both players,
    optionally sets memory and phase.

    Returns:
        Summary dict with counts of items placed per zone.
    """
    from engine_py_legacy.engine.data.enums import GamePhase

    summary = {"player1": {}, "player2": {}}

    for pid, zone_setup, key in [
        (1, config.player1, "player1"),
        (2, config.player2, "player2"),
    ]:
        # Hand
        for card_id in zone_setup.hand:
            inject_card(game, pid, card_id, "hand")
        if zone_setup.hand:
            summary[key]["hand"] = len(zone_setup.hand)

        # Library top (insert in reverse so first listed is on top)
        for card_id in reversed(zone_setup.library_top):
            inject_card(game, pid, card_id, "library_top")
        if zone_setup.library_top:
            summary[key]["library_top"] = len(zone_setup.library_top)

        # Security
        for card_id in zone_setup.security:
            inject_card(game, pid, card_id, "security_top")
        if zone_setup.security:
            summary[key]["security"] = len(zone_setup.security)

        # Trash
        for card_id in zone_setup.trash:
            inject_card(game, pid, card_id, "trash")
        if zone_setup.trash:
            summary[key]["trash"] = len(zone_setup.trash)

        # Field (digivolution stacks)
        for entry in zone_setup.field:
            place_on_field(game, pid, entry.card_ids, entry.is_suspended, entry.turn_played)
        if zone_setup.field:
            summary[key]["field"] = len(zone_setup.field)

        # Breeding
        if zone_setup.breeding:
            place_in_breeding(game, pid, zone_setup.breeding)
            summary[key]["breeding"] = 1

    # Memory
    if config.memory is not None:
        game.memory = config.memory
        summary["memory"] = config.memory

    # Phase
    if config.phase is not None:
        try:
            game.current_phase = GamePhase[config.phase]
            summary["phase"] = config.phase
        except KeyError:
            valid = [p.name for p in GamePhase]
            raise ValueError(f"Invalid phase '{config.phase}'. Valid: {valid}")

    return summary
