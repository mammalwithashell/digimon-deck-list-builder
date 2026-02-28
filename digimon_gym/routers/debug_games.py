"""Debug game endpoints for deterministic testing. Only available when DEBUG_MODE=1."""

from __future__ import annotations

import random
from uuid import uuid4

from fastapi import APIRouter, HTTPException

from digimon_gym.engine.data.card_database import CardDatabase
from digimon_gym.engine.data.enums import PlayerType, GamePhase
from digimon_gym.engine.runners.interactive_game import InteractiveGame
from digimon_gym.engine.core.player import Player
from digimon_gym.routers.schemas import (
    CreateDebugGameRequest,
    SetMemoryRequest,
    InjectCardRequest,
)
from digimon_gym.routers.state import active_games

router = APIRouter(prefix="/debug", tags=["debug"])


class DebugGameRunner(InteractiveGame):
    def __init__(self, config: CreateDebugGameRequest):
        p1_type = PlayerType.Human if config.player1_type.lower() == "human" else PlayerType.Agent
        p2_type = PlayerType.Human if config.player2_type.lower() == "human" else PlayerType.Agent

        # Monkey-patch shuffle if skip_shuffle
        original_shuffle = Player.shuffle_for_game_start
        if config.skip_shuffle:
            Player.shuffle_for_game_start = lambda self: None

        try:
            super().__init__(
                config.deck1, config.deck2,
                p1_type, p2_type,
                player1_policy=config.player1_policy,
                player2_policy=config.player2_policy,
                agent_action_delay_ms=config.agent_action_delay_ms,
            )
        finally:
            Player.shuffle_for_game_start = original_shuffle

        # Fix first player if needed
        game = self.game
        desired_first = game.player1 if config.first_player == 1 else game.player2
        if game.turn_player != desired_first:
            game.turn_player, game.opponent_player = game.opponent_player, game.turn_player
            game.turn_player.is_my_turn = True
            game.opponent_player.is_my_turn = False
            # Fix mulligan order to match
            game._mulligan_order = [game.turn_player, game.opponent_player]
            game._mulligan_index = 0
            game.active_player = game._mulligan_order[0]

        # Set starting hands if specified
        if config.starting_hand1:
            self._set_starting_hand(game.player1, config.starting_hand1)
        if config.starting_hand2:
            self._set_starting_hand(game.player2, config.starting_hand2)

        # Auto-process mulligan
        if config.auto_mulligan == "keep":
            while game.current_phase == GamePhase.Mulligan:
                game.decode_action(0, game.current_player_id)  # 0 = keep hand

        # Set initial memory
        game.memory = config.initial_memory

    @staticmethod
    def _set_starting_hand(player: Player, card_ids: list[str]):
        """Replace player's hand with specific cards from their library."""
        # Return current hand cards to top of library
        for card in reversed(player.hand_cards):
            player.library_cards.insert(0, card)
        player.hand_cards.clear()

        # Pull requested cards from library into hand
        for wanted_id in card_ids:
            for i, card in enumerate(player.library_cards):
                if card.c_entity_base and card.c_entity_base.card_id == wanted_id:
                    player.hand_cards.append(player.library_cards.pop(i))
                    break


@router.post("/games")
def create_debug_game(request: CreateDebugGameRequest):
    if not request.deck1 or not request.deck2:
        raise HTTPException(status_code=400, detail="Both decks must be provided")

    game_id = str(uuid4())
    runner = DebugGameRunner(request)
    active_games[game_id] = runner
    runner.clear_log()

    p1_type = PlayerType.Human if request.player1_type.lower() == "human" else PlayerType.Agent
    p2_type = PlayerType.Human if request.player2_type.lower() == "human" else PlayerType.Agent

    state = runner.game.to_ui_json()
    mask = runner.get_action_mask().tolist()
    player_labels = {
        1: "You" if p1_type == PlayerType.Human else "Agent",
        2: "You" if p2_type == PlayerType.Human else "Agent",
    }

    return {
        "game_id": game_id,
        "state": state,
        "action_mask": mask,
        "action_descriptions": runner.game.describe_actions(runner.game.current_player_id),
        "player_labels": player_labels,
        "recording_metadata": runner.get_initial_state_dict(),
    }


@router.get("/games/{game_id}/internal-state")
def debug_internal_state(game_id: str):
    runner = active_games.get(game_id)
    if not runner:
        raise HTTPException(status_code=404, detail="Game not found")

    game = runner.game
    p1 = game.player1
    p2 = game.player2

    def _card_id(card_source):
        return card_source.c_entity_base.card_id if card_source.c_entity_base else "unknown"

    def _field_effects(player):
        results = []
        for i, perm in enumerate(player.battle_area):
            effect_names = []
            top = perm.card_sources[0] if perm.card_sources else None
            if top:
                script = CardDatabase().get_script(_card_id(top))
                if script:
                    try:
                        effects = script.get_card_effects(top)
                        effect_names = [e.effect_name for e in effects if hasattr(e, 'effect_name')]
                    except Exception:
                        pass
            results.append({
                "slot": i,
                "card_id": _card_id(perm.card_sources[0]) if perm.card_sources else None,
                "card_name": perm.card_sources[0].card_names[0] if perm.card_sources else None,
                "source_count": len(perm.card_sources),
                "effect_names": effect_names,
            })
        return results

    return {
        "library_top_5_p1": [_card_id(c) for c in p1.library_cards[:5]],
        "library_top_5_p2": [_card_id(c) for c in p2.library_cards[:5]],
        "library_size_p1": len(p1.library_cards),
        "library_size_p2": len(p2.library_cards),
        "hand_p1": [_card_id(c) for c in p1.hand_cards],
        "hand_p2": [_card_id(c) for c in p2.hand_cards],
        "security_cards_p1": [_card_id(c) for c in p1.security_cards],
        "security_cards_p2": [_card_id(c) for c in p2.security_cards],
        "trash_p1": [_card_id(c) for c in p1.trash_cards],
        "trash_p2": [_card_id(c) for c in p2.trash_cards],
        "memory": game.memory,
        "turn_count": game.turn_count,
        "phase": game.current_phase.name,
        "turn_player_id": game.turn_player.player_id,
        "active_player_id": game.active_player.player_id if game.active_player else game.turn_player.player_id,
        "is_game_over": game.game_over,
        "effects_on_field_p1": _field_effects(p1),
        "effects_on_field_p2": _field_effects(p2),
    }


@router.post("/games/{game_id}/set-memory")
def debug_set_memory(game_id: str, request: SetMemoryRequest):
    runner = active_games.get(game_id)
    if not runner:
        raise HTTPException(status_code=404, detail="Game not found")
    runner.game.memory = request.memory
    return {"memory": runner.game.memory}


@router.post("/games/{game_id}/inject-card")
def debug_inject_card(game_id: str, request: InjectCardRequest):
    runner = active_games.get(game_id)
    if not runner:
        raise HTTPException(status_code=404, detail="Game not found")

    player = runner.game.player1 if request.player_id == 1 else runner.game.player2
    db = CardDatabase()
    card_source = db.create_card_source(request.card_id, player)
    if card_source is None:
        raise HTTPException(status_code=400, detail=f"Card {request.card_id} not found")

    if request.zone == "hand":
        player.hand_cards.append(card_source)
    elif request.zone == "library_top":
        player.library_cards.insert(0, card_source)
    elif request.zone == "security_top":
        player.security_cards.insert(0, card_source)
    else:
        raise HTTPException(status_code=400, detail=f"Invalid zone: {request.zone}")

    return {"status": "injected", "card_id": request.card_id, "zone": request.zone}
