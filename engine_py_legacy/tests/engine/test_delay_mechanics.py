"""Tests for <Delay> keyword and security effect callback fix.

Covers:
- Security effect callback receives proper context dict
- Delay action mask: exposed only when card has been in battle area > 1 turn
- Delay summoning sickness: not available on the same turn placed
- Delay activation: card trashed from battle area, delayed effect fires
- Delay effect receives proper context dict
"""

import pytest

from digimon_gym.engine.game import Game, ACTION_SPACE_SIZE
from digimon_gym.engine.data.enums import (
    GamePhase, CardKind, CardColor, EffectTiming, AttackResolution,
)
from digimon_gym.engine.core.player import Player
from digimon_gym.engine.core.permanent import Permanent
from digimon_gym.engine.core.card_source import CardSource
from digimon_gym.engine.core.entity_base import CEntity_Base
from digimon_gym.engine.interfaces.card_effect import ICardEffect


# ─── Mock Helpers ─────────────────────────────────────────────────────

class MockCardSourceWithEffects(CardSource):
    """CardSource that returns custom effects instead of querying CardDatabase."""
    def __init__(self):
        super().__init__()
        self._mock_effects = []

    def effect_list(self, timing):
        return self._mock_effects


def make_card(card_id="TEST-001", name="TestDigimon", kind=CardKind.Digimon,
              dp=5000, level=4, play_cost=5, colors=None, owner=None):
    """Create a CardSource with given attributes."""
    entity = CEntity_Base()
    entity.card_id = card_id
    entity.card_name_eng = name
    entity.card_kind = kind
    entity.dp = dp
    entity.level = level
    entity.play_cost = play_cost
    entity.card_colors = colors or [CardColor.Red]
    cs = CardSource()
    cs.set_base_data(entity, owner)
    return cs


def make_option_with_delay(card_id="OPT-001", name="DelayOption", delay_action=None,
                           colors=None, owner=None):
    """Create a MockCardSourceWithEffects that has Option + Delay pattern.

    Produces 3 effects:
    - effect0: OptionSkill main effect (no-op)
    - effect1: Delay marker (_is_delay = True)
    - effect2: Delayed callback (the actual effect)
    """
    entity = CEntity_Base()
    entity.card_id = card_id
    entity.card_name_eng = name
    entity.card_kind = CardKind.Option
    entity.dp = None
    entity.level = None
    entity.play_cost = 3
    entity.card_colors = colors or [CardColor.Red]
    cs = MockCardSourceWithEffects()
    cs.set_base_data(entity, owner)

    # Effect 0: Option main effect (no-op)
    effect0 = ICardEffect()
    effect0.set_effect_name(f"{card_id} Main")
    effect0.set_effect_description("Option main effect")

    # Effect 1: Delay marker
    effect1 = ICardEffect()
    effect1.set_effect_name(f"{card_id} Delay")
    effect1.set_effect_description("Delay")
    effect1._is_delay = True

    # Effect 2: Delayed callback
    effect2 = ICardEffect()
    effect2.set_effect_name(f"{card_id} Delayed Effect")
    effect2.set_effect_description("Delayed effect callback")

    def default_delay_action(ctx):
        """Default delay action: gain 2 memory."""
        player = ctx.get('player')
        if player:
            player.add_memory(2)

    effect2.set_on_process_callback(delay_action or default_delay_action)
    effect2.set_can_use_condition(lambda ctx: True)

    cs._mock_effects = [effect0, effect1, effect2]
    return cs


def make_security_card(card_id="SEC-001", name="SecurityOption", security_action=None,
                       colors=None, owner=None):
    """Create a MockCardSourceWithEffects with a security effect."""
    entity = CEntity_Base()
    entity.card_id = card_id
    entity.card_name_eng = name
    entity.card_kind = CardKind.Option
    entity.dp = None
    entity.level = None
    entity.play_cost = 3
    entity.card_colors = colors or [CardColor.Red]
    cs = MockCardSourceWithEffects()
    cs.set_base_data(entity, owner)

    # Security effect
    effect = ICardEffect()
    effect.set_effect_name(f"{card_id} Security")
    effect.set_effect_description("[Security] effect")
    effect.is_security_effect = True

    def default_security_action(ctx):
        """Default security action: draw 1 card."""
        player = ctx.get('player')
        game = ctx.get('game')
        if player and player.library_cards:
            drawn = player.library_cards.pop(0)
            player.hand_cards.append(drawn)

    effect.set_on_process_callback(security_action or default_security_action)
    cs._mock_effects = [effect]
    return cs


def setup_delay_game():
    """Create a game with an Option card with Delay in the battle area.

    Returns (game, delay_permanent, option_card_source).
    """
    game = Game()
    game.current_phase = GamePhase.Main
    game.memory = 5
    game.turn_count = 3  # Turn 3
    game.turn_player = game.player1
    game.opponent_player = game.player2
    game.player1.is_my_turn = True
    game.player1.game = game
    game.player2.game = game
    game.player1.enemy = game.player2
    game.player2.enemy = game.player1

    # Library and security
    for _ in range(10):
        game.player1.library_cards.append(make_card(name="P1Lib", owner=game.player1))
        game.player2.library_cards.append(make_card(name="P2Lib", owner=game.player2))
    for _ in range(5):
        game.player1.security_cards.append(
            make_card(name="P1Sec", kind=CardKind.Tamer, dp=None, level=None, owner=game.player1))
        game.player2.security_cards.append(
            make_card(name="P2Sec", kind=CardKind.Tamer, dp=None, level=None, owner=game.player2))

    # Option card with Delay in battle area
    option_cs = make_option_with_delay(owner=game.player1)
    delay_perm = Permanent([option_cs])
    delay_perm.turn_played = 1  # Placed on turn 1 — Delay eligible (turn 3 now)
    delay_perm._owner_game = game
    game.player1.battle_area.append(delay_perm)

    return game, delay_perm, option_cs



# ─── Security Effect Callback Tests ──────────────────────────────────

class TestSecurityEffectCallback:
    """Tests that security effect callbacks receive proper context dict."""

    def test_security_callback_receives_context(self):
        """Security effect callback must receive a context dict with game, player, card keys."""
        captured_ctx = {}

        def capture_ctx(ctx):
            captured_ctx.update(ctx)

        game = Game()
        game.current_phase = GamePhase.Main
        game.memory = 5
        game.turn_count = 2
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.game = game
        game.player2.game = game

        # Give player2 a security card with a capturing callback
        sec_card = make_security_card(security_action=capture_ctx, owner=game.player2)
        game.player2.security_cards.insert(0, sec_card)

        # Give player2 library cards (for draw)
        for _ in range(5):
            game.player2.library_cards.append(make_card(name="P2Lib", owner=game.player2))

        # Create attacker without Progress (so security effects fire)
        attacker_entity = CEntity_Base()
        attacker_entity.card_id = "ATK-001"
        attacker_entity.card_name_eng = "Attacker"
        attacker_entity.card_kind = CardKind.Digimon
        attacker_entity.dp = 7000
        attacker_entity.level = 4
        attacker_entity.card_colors = [CardColor.Red]
        attacker_cs = CardSource()
        attacker_cs.set_base_data(attacker_entity, game.player1)
        attacker = Permanent([attacker_cs])
        attacker.turn_played = 1

        # Trigger security attack
        result = game.player2.security_attack(attacker)

        # Verify context was passed
        assert 'game' in captured_ctx, "Security callback must receive 'game' in context"
        assert 'player' in captured_ctx, "Security callback must receive 'player' in context"
        assert 'card' in captured_ctx, "Security callback must receive 'card' in context"
        assert captured_ctx['game'] is game
        assert captured_ctx['player'] is game.player2
        assert captured_ctx['card'] is sec_card
        assert captured_ctx['turn_player'] is game.player1
        assert captured_ctx['opponent_player'] is game.player2
        assert captured_ctx['permanent'] is None  # Security card not on a permanent

    def test_security_callback_can_use_game_methods(self):
        """Security callback with proper context can call game engine methods."""
        drew_card = [False]

        def draw_one(ctx):
            player = ctx.get('player')
            if player and player.library_cards:
                card = player.library_cards.pop(0)
                player.hand_cards.append(card)
                drew_card[0] = True

        game = Game()
        game.current_phase = GamePhase.Main
        game.memory = 5
        game.turn_count = 2
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.game = game
        game.player2.game = game

        sec_card = make_security_card(security_action=draw_one, owner=game.player2)
        game.player2.security_cards.insert(0, sec_card)
        for _ in range(5):
            game.player2.library_cards.append(make_card(name="P2Lib", owner=game.player2))

        initial_hand = len(game.player2.hand_cards)
        initial_lib = len(game.player2.library_cards)

        attacker_cs = CardSource()
        attacker_entity = CEntity_Base()
        attacker_entity.card_id = "ATK-001"
        attacker_entity.card_name_eng = "Attacker"
        attacker_entity.card_kind = CardKind.Digimon
        attacker_entity.dp = 7000
        attacker_entity.level = 4
        attacker_entity.card_colors = [CardColor.Red]
        attacker_cs.set_base_data(attacker_entity, game.player1)
        attacker = Permanent([attacker_cs])

        game.player2.security_attack(attacker)

        assert drew_card[0], "Security callback should have drawn a card"
        assert len(game.player2.hand_cards) == initial_hand + 1
        assert len(game.player2.library_cards) == initial_lib - 1


# ─── Delay Keyword Tests ─────────────────────────────────────────────

class TestDelayActionMask:
    """Tests for Delay keyword action masking."""

    def test_delay_exposed_in_action_mask(self):
        """Delay action should be available for Option card placed in battle area on a previous turn."""
        game, delay_perm, _ = setup_delay_game()
        mask = game.get_action_mask(game.player1.player_id)

        # Delay is at effectIdx=1, slot 0 → action 1000 + 0*10 + 1 = 1001
        assert mask[1001] == 1.0, "Delay action should be available"

    def test_delay_not_available_same_turn(self):
        """Delay cannot be activated on the same turn the card was placed."""
        game, delay_perm, _ = setup_delay_game()
        delay_perm.turn_played = game.turn_count  # Placed this turn

        mask = game.get_action_mask(game.player1.player_id)
        assert mask[1001] == 0.0, "Delay should not be available on the same turn placed"

    def test_delay_not_available_for_digimon(self):
        """Delay flag on a Digimon card should still show up (engine scans all permanents)."""
        game = Game()
        game.current_phase = GamePhase.Main
        game.memory = 5
        game.turn_count = 3
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.game = game
        game.player2.game = game

        for _ in range(5):
            game.player1.library_cards.append(make_card(owner=game.player1))
            game.player2.library_cards.append(make_card(owner=game.player2))

        # Digimon with _is_delay (unusual but should still work)
        entity = CEntity_Base()
        entity.card_id = "DIGI-001"
        entity.card_name_eng = "DelayDigimon"
        entity.card_kind = CardKind.Digimon
        entity.dp = 5000
        entity.level = 4
        entity.card_colors = [CardColor.Red]
        cs = MockCardSourceWithEffects()
        cs.set_base_data(entity, game.player1)

        delay_effect = ICardEffect()
        delay_effect._is_delay = True
        callback_effect = ICardEffect()
        callback_effect.set_on_process_callback(lambda ctx: None)
        callback_effect.set_can_use_condition(lambda ctx: True)
        cs._mock_effects = [delay_effect, callback_effect]

        perm = Permanent([cs])
        perm.turn_played = 1
        game.player1.battle_area.append(perm)

        mask = game.get_action_mask(game.player1.player_id)
        assert mask[1001] == 1.0, "Delay on any permanent type should be available"

    def test_delay_multiple_slots(self):
        """Delay actions should map to correct slot indices."""
        game, delay_perm, _ = setup_delay_game()

        # Add a second delay card in slot 1
        option_cs2 = make_option_with_delay(card_id="OPT-002", name="DelayOption2",
                                             owner=game.player1)
        delay_perm2 = Permanent([option_cs2])
        delay_perm2.turn_played = 2  # Placed on turn 2 — eligible (turn 3)
        game.player1.battle_area.append(delay_perm2)

        mask = game.get_action_mask(game.player1.player_id)
        # Slot 0 → 1001, Slot 1 → 1011
        assert mask[1001] == 1.0, "Delay slot 0 should be available"
        assert mask[1011] == 1.0, "Delay slot 1 should be available"

    def test_no_delay_when_no_delay_effect(self):
        """Permanents without _is_delay should not have Delay actions exposed."""
        game = Game()
        game.current_phase = GamePhase.Main
        game.memory = 5
        game.turn_count = 3
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.game = game
        game.player2.game = game

        for _ in range(5):
            game.player1.library_cards.append(make_card(owner=game.player1))
            game.player2.library_cards.append(make_card(owner=game.player2))

        # Regular option card without Delay
        entity = CEntity_Base()
        entity.card_id = "OPT-NODLY"
        entity.card_name_eng = "NoDelayOption"
        entity.card_kind = CardKind.Option
        entity.dp = None
        entity.level = None
        entity.play_cost = 3
        entity.card_colors = [CardColor.Red]
        cs = MockCardSourceWithEffects()
        cs.set_base_data(entity, game.player1)
        cs._mock_effects = []

        perm = Permanent([cs])
        perm.turn_played = 1
        game.player1.battle_area.append(perm)

        mask = game.get_action_mask(game.player1.player_id)
        assert mask[1001] == 0.0, "No Delay action should be available without _is_delay effect"


class TestDelayExecution:
    """Tests for Delay keyword activation and execution."""

    def test_delay_trashes_card(self):
        """Activating Delay should trash the Option card from the battle area."""
        game, delay_perm, option_cs = setup_delay_game()
        initial_battle = len(game.player1.battle_area)
        initial_trash = len(game.player1.trash_cards)

        game.decode_action(1001, game.player1.player_id)  # Delay action slot 0

        assert len(game.player1.battle_area) == initial_battle - 1, \
            "Option card should be removed from battle area"
        assert len(game.player1.trash_cards) == initial_trash + 1, \
            "Option card should be moved to trash"
        assert option_cs in game.player1.trash_cards, \
            "The actual card source should be in trash"

    def test_delay_fires_callback(self):
        """Activating Delay should execute the delayed effect callback."""
        callback_fired = [False]
        captured_ctx = {}

        def delay_callback(ctx):
            callback_fired[0] = True
            captured_ctx.update(ctx)

        game, delay_perm, _ = setup_delay_game()
        # Replace the delayed callback
        effects = delay_perm.card_sources[0].effect_list(EffectTiming.NoTiming)
        effects[2].set_on_process_callback(delay_callback)

        game.decode_action(1001, game.player1.player_id)

        assert callback_fired[0], "Delayed effect callback should have fired"
        assert captured_ctx['game'] is game
        assert captured_ctx['player'] is game.player1
        assert captured_ctx['permanent'] is delay_perm
        assert captured_ctx['turn_player'] is game.player1

    def test_delay_memory_gain(self):
        """Default delay action (gain 2 memory) should work correctly."""
        game, delay_perm, _ = setup_delay_game()
        initial_memory = game.memory  # Game.memory is the shared memory gauge

        game.decode_action(1001, game.player1.player_id)

        # Default delay action: gain 2 memory (adjusts Game.memory)
        assert game.memory == initial_memory + 2, \
            "Game memory should have increased by 2 from Delay activation"

    def test_delay_records_activation(self):
        """Delay activation should record the effect activation count."""
        game, delay_perm, _ = setup_delay_game()
        effects = delay_perm.card_sources[0].effect_list(EffectTiming.NoTiming)
        delay_callback_effect = effects[2]

        assert delay_callback_effect._activate_count == 0
        game.decode_action(1001, game.player1.player_id)
        assert delay_callback_effect._activate_count == 1

    def test_delay_condition_checked(self):
        """Delay should check can_use_condition before firing."""
        callback_fired = [False]

        def delay_callback(ctx):
            callback_fired[0] = True

        game, delay_perm, _ = setup_delay_game()
        effects = delay_perm.card_sources[0].effect_list(EffectTiming.NoTiming)
        effects[2].set_on_process_callback(delay_callback)
        effects[2].set_can_use_condition(lambda ctx: False)  # Block activation

        game.decode_action(1001, game.player1.player_id)

        # Card should still be trashed (Delay cost is paid regardless)
        assert delay_perm not in game.player1.battle_area
        # But callback should NOT fire
        assert not callback_fired[0], "Delayed callback should not fire when condition is False"


class TestDelayHelpers:
    """Tests for _has_delay_effect and _get_delay_callback helpers."""

    def test_has_delay_effect_true(self):
        """_has_delay_effect returns True for permanent with _is_delay effect."""
        game, delay_perm, _ = setup_delay_game()
        assert game._has_delay_effect(delay_perm) is True

    def test_has_delay_effect_false(self):
        """_has_delay_effect returns False for permanent without _is_delay."""
        game = Game()
        cs = MockCardSourceWithEffects()
        entity = CEntity_Base()
        entity.card_id = "NONE-001"
        entity.card_name_eng = "NoDelay"
        entity.card_kind = CardKind.Option
        entity.card_colors = [CardColor.Red]
        cs.set_base_data(entity, None)
        cs._mock_effects = [ICardEffect()]  # No _is_delay
        perm = Permanent([cs])
        assert game._has_delay_effect(perm) is False

    def test_get_delay_callback_returns_correct_effect(self):
        """_get_delay_callback returns the effect after the _is_delay marker."""
        game, delay_perm, _ = setup_delay_game()
        callback = game._get_delay_callback(delay_perm)
        assert callback is not None
        assert callback.effect_name == "OPT-001 Delayed Effect"

    def test_get_delay_callback_no_delay_returns_none(self):
        """_get_delay_callback returns None if no _is_delay marker found."""
        game = Game()
        cs = MockCardSourceWithEffects()
        entity = CEntity_Base()
        entity.card_id = "NONE-001"
        entity.card_name_eng = "NoDelay"
        entity.card_kind = CardKind.Option
        entity.card_colors = [CardColor.Red]
        cs.set_base_data(entity, None)
        cs._mock_effects = [ICardEffect()]
        perm = Permanent([cs])
        assert game._get_delay_callback(perm) is None

    def test_get_delay_callback_delay_at_end_returns_none(self):
        """If _is_delay is the last effect (no following callback), return None."""
        game = Game()
        cs = MockCardSourceWithEffects()
        entity = CEntity_Base()
        entity.card_id = "EDGE-001"
        entity.card_name_eng = "EdgeCase"
        entity.card_kind = CardKind.Option
        entity.card_colors = [CardColor.Red]
        cs.set_base_data(entity, None)
        delay_marker = ICardEffect()
        delay_marker._is_delay = True
        cs._mock_effects = [delay_marker]  # No effect after marker
        perm = Permanent([cs])
        assert game._get_delay_callback(perm) is None


# ─── Enriched to_json() Tests ────────────────────────────────────────

class TestEnrichedToJson:
    """Tests for enriched BattleArea/BreedingArea serialization in to_json()."""

    def test_battle_area_includes_card_kind(self):
        """to_json() BattleArea entries should include CardKind."""
        game, delay_perm, _ = setup_delay_game()
        state = game.to_json()
        ba = state["Player1"]["BattleArea"]
        assert len(ba) == 1
        assert ba[0]["CardKind"] == "Option"

    def test_battle_area_includes_turn_played(self):
        """to_json() BattleArea entries should include TurnPlayed."""
        game, delay_perm, _ = setup_delay_game()
        state = game.to_json()
        ba = state["Player1"]["BattleArea"]
        assert ba[0]["TurnPlayed"] == 1  # Set in setup_delay_game

    def test_battle_area_includes_keywords(self):
        """to_json() BattleArea entries should include Keywords list."""
        game, delay_perm, _ = setup_delay_game()
        state = game.to_json()
        ba = state["Player1"]["BattleArea"]
        assert "Keywords" in ba[0]
        assert isinstance(ba[0]["Keywords"], list)
        assert "Delay" in ba[0]["Keywords"]

    def test_battle_area_includes_activatable_effects(self):
        """to_json() BattleArea entries should include ActivatableEffects."""
        game, delay_perm, _ = setup_delay_game()
        state = game.to_json()
        ba = state["Player1"]["BattleArea"]
        effects = ba[0]["ActivatableEffects"]
        assert isinstance(effects, list)
        assert len(effects) >= 1
        delay_effects = [e for e in effects if e["name"] == "Delay"]
        assert len(delay_effects) == 1
        assert delay_effects[0]["effectIdx"] == 1
        assert delay_effects[0]["actionId"] == 1001

    def test_activatable_effects_delay_not_same_turn(self):
        """ActivatableEffects should not include Delay on same turn placed."""
        game, delay_perm, _ = setup_delay_game()
        delay_perm.turn_played = game.turn_count  # Same turn
        state = game.to_json()
        ba = state["Player1"]["BattleArea"]
        effects = ba[0]["ActivatableEffects"]
        delay_effects = [e for e in effects if e["name"] == "Delay"]
        assert len(delay_effects) == 0, "Delay should not appear on same turn"

    def test_digimon_card_kind(self):
        """Digimon permanents should have CardKind='Digimon'."""
        game = Game()
        game.current_phase = GamePhase.Main
        game.memory = 5
        game.turn_count = 2
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.game = game
        game.player2.game = game

        digimon_cs = make_card(name="Greymon", kind=CardKind.Digimon, owner=game.player1)
        perm = Permanent([digimon_cs])
        perm.turn_played = 1
        game.player1.battle_area.append(perm)

        state = game.to_json()
        ba = state["Player1"]["BattleArea"]
        assert ba[0]["CardKind"] == "Digimon"
        assert ba[0]["TopCardName"] == "Greymon"

    def test_breeding_area_includes_keywords(self):
        """to_json() BreedingArea should include Keywords when a permanent is there."""
        game = Game()
        game.current_phase = GamePhase.Main
        game.memory = 5
        game.turn_count = 2
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.game = game
        game.player2.game = game

        digi_cs = make_card(name="Koromon", kind=CardKind.DigiEgg,
                            level=2, owner=game.player1)
        perm = Permanent([digi_cs])
        perm.turn_played = 1
        game.player1.breeding_area = perm

        state = game.to_json()
        ba = state["Player1"]["BreedingArea"]
        assert ba is not None
        assert "Keywords" in ba
        assert isinstance(ba["Keywords"], list)
        assert "TopCardName" in ba
        assert ba["TurnPlayed"] == 1

    def test_breeding_area_none_when_empty(self):
        """to_json() BreedingArea should be None when empty."""
        game, _, _ = setup_delay_game()
        state = game.to_json()
        assert state["Player1"]["BreedingArea"] is None


# ─── describe_actions() Tests ────────────────────────────────────────

class TestDescribeActions:
    """Tests for Game.describe_actions() human-readable action labels."""

    def test_describe_pass_action(self):
        """describe_actions should include Pass for action 62 in Main phase."""
        game, _, _ = setup_delay_game()
        descriptions = game.describe_actions(game.player1.player_id)
        assert 62 in descriptions
        assert "Pass" in descriptions[62]

    def test_describe_delay_action(self):
        """describe_actions should label Delay activation with card name."""
        game, _, _ = setup_delay_game()
        descriptions = game.describe_actions(game.player1.player_id)
        assert 1001 in descriptions
        assert "Delay" in descriptions[1001]
        assert "DelayOption" in descriptions[1001]

    def test_describe_play_card(self):
        """describe_actions should label play-from-hand with card name."""
        game = Game()
        game.current_phase = GamePhase.Main
        game.memory = 5
        game.turn_count = 2
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.game = game
        game.player2.game = game

        for _ in range(5):
            game.player1.library_cards.append(make_card(owner=game.player1))
            game.player2.library_cards.append(make_card(owner=game.player2))
            game.player1.security_cards.append(
                make_card(kind=CardKind.Tamer, dp=None, level=None, owner=game.player1))
            game.player2.security_cards.append(
                make_card(kind=CardKind.Tamer, dp=None, level=None, owner=game.player2))

        # Add a playable card to hand
        hand_card = make_card(card_id="BT1-001", name="Agumon",
                              kind=CardKind.Digimon, play_cost=3, owner=game.player1)
        game.player1.hand_cards.append(hand_card)

        descriptions = game.describe_actions(game.player1.player_id)
        # Action 0 = play first hand card
        if 0 in descriptions:
            assert "Agumon" in descriptions[0]
            assert "Play" in descriptions[0]

    def test_describe_only_valid_actions(self):
        """describe_actions should only include actions where mask is 1.0."""
        game, _, _ = setup_delay_game()
        descriptions = game.describe_actions(game.player1.player_id)
        mask = game.get_action_mask(game.player1.player_id)

        for action_id in descriptions:
            assert mask[action_id] == 1.0, \
                f"Action {action_id} ({descriptions[action_id]}) should have mask=1.0"

    def test_describe_training_action(self):
        """describe_actions should label Training activation."""
        game = Game()
        game.current_phase = GamePhase.Main
        game.memory = 5
        game.turn_count = 3
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.game = game
        game.player2.game = game

        for _ in range(5):
            game.player1.library_cards.append(make_card(owner=game.player1))
            game.player2.library_cards.append(make_card(owner=game.player2))

        # Digimon with Training keyword
        entity = CEntity_Base()
        entity.card_id = "TR-001"
        entity.card_name_eng = "TrainingMon"
        entity.card_kind = CardKind.Digimon
        entity.dp = 5000
        entity.level = 4
        entity.card_colors = [CardColor.Red]
        cs = MockCardSourceWithEffects()
        cs.set_base_data(entity, game.player1)
        training_effect = ICardEffect()
        training_effect._is_training = True
        cs._mock_effects = [training_effect]

        perm = Permanent([cs])
        perm.turn_played = 1
        game.player1.battle_area.append(perm)

        descriptions = game.describe_actions(game.player1.player_id)
        assert 1000 in descriptions
        assert "Training" in descriptions[1000]
        assert "TrainingMon" in descriptions[1000]
