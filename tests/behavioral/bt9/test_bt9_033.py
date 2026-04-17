"""Behavioral tests for BT9-033 Pillomon (Digimon, Lv.3, Yellow, DP 2000, Cost 3).

Card text:
[All Turns] Players can't play Digimon by effects.

This is a continuous lock effect implemented via CANNOT_PLAY_BY_EFFECT modifier.
It blocks effect-based Digimon plays (e.g., play from trash via card effects)
while allowing normal hand plays from the Main phase.
Identical mechanic to BT9-047 Pomumon.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming
from digimon_gym.engine.interfaces.modifiers import ModifierType


FILLER_DECK = ["BT9-033"] * 50


@pytest.mark.behavioral
class TestBT9033Pillomon:
    """Tests for BT9-033 Pillomon."""

    # ── Modifier registration ───────────────────────────────────────

    def test_registers_cannot_play_by_effect_modifier(self, debug_runner):
        """Pillomon on field should register CANNOT_PLAY_BY_EFFECT modifier."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        perm = runner.place_on_field(1, ["BT9-033"])
        game = runner.game

        # Fire the On Play process callback to register the modifier
        top_card = perm.top_card
        effects = top_card.effect_list(None)
        on_play_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
            and e.on_process_callback is not None
        ]
        assert len(on_play_effects) >= 1, (
            "Pillomon should have an OnEnterFieldAnyone/On Play effect "
            "that registers the CANNOT_PLAY_BY_EFFECT modifier"
        )

        # Execute to register the modifier
        on_play_effects[0].on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Check modifier is registered
        has_modifier = False
        if hasattr(game, 'modifiers'):
            entries = game.modifiers._modifiers.get(ModifierType.CANNOT_PLAY_BY_EFFECT, [])
            has_modifier = len(entries) > 0
        assert has_modifier, (
            "CANNOT_PLAY_BY_EFFECT modifier should be registered after Pillomon enters field"
        )

    def test_blocks_effect_based_digimon_play(self, debug_runner):
        """With Pillomon on field, effect-based Digimon plays should be blocked."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        perm = runner.place_on_field(1, ["BT9-033"])
        game = runner.game

        # Fire the On Play effect to register modifier
        top_card = perm.top_card
        effects = top_card.effect_list(None)
        on_play_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
            and e.on_process_callback is not None
        ]
        on_play_effects[0].on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Now check _is_effect_play_blocked for a Digimon card
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        digimon_cs = db.create_card_source("ST1-03", game.player1)  # Agumon

        blocked = game._is_effect_play_blocked(digimon_cs)
        assert blocked, (
            "Effect-based Digimon play should be blocked with Pillomon on field"
        )

    def test_does_not_block_non_digimon_effect_play(self, debug_runner):
        """CANNOT_PLAY_BY_EFFECT should only block Digimon, not Options/Tamers."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        perm = runner.place_on_field(1, ["BT9-033"])
        game = runner.game

        # Fire the On Play effect
        top_card = perm.top_card
        effects = top_card.effect_list(None)
        on_play_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
            and e.on_process_callback is not None
        ]
        on_play_effects[0].on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Check that a Tamer play is NOT blocked
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        tamer_cs = db.create_card_source("ST1-12", game.player1)  # Tamer

        blocked = game._is_effect_play_blocked(tamer_cs)
        assert not blocked, (
            "Effect-based Tamer play should NOT be blocked by Pillomon"
        )

    def test_continuous_effect_requires_field_presence(self, debug_runner):
        """The effect condition should require Pillomon to be on the field."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)

        from digimon_gym.engine.data.card_database import CardDatabase
        from digimon_gym.engine.data.card_registry import CardRegistry
        CardRegistry.ensure_initialized()
        db = CardDatabase()

        cs = db.create_card_source("BT9-033", runner.game.player1)
        effects = cs.effect_list(None)

        # Find the main continuous effect
        on_play_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ]
        assert len(on_play_effects) >= 1

        # When not on field, condition should be False
        result = on_play_effects[0].can_use_condition({})
        assert result is False, (
            "Continuous effect condition should be False when Pillomon is not on field"
        )

    # ── Scope checks ─────────────────────────────────────────────────

    def _fire_on_play(self, game, perm):
        """Helper to fire the OnEnterField On Play effect callback."""
        top_card = perm.top_card
        effects = top_card.effect_list(None)
        on_play_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
            and e.on_process_callback is not None
        ]
        assert len(on_play_effects) >= 1
        on_play_effects[0].on_process_callback({
            'player': perm.owner,
            'game': game,
            'permanent': perm,
        })

    def test_blocks_digi_egg_effect_play(self, debug_runner):
        """Per DCGO C#, CardCondition is `IsDigimon || IsDigiEgg`, so Digi-Egg
        effect-plays should also be blocked."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, ["BT9-033"])
        game = runner.game
        self._fire_on_play(game, perm)

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        # BT1-001 Yokomon is a Digi-Egg
        egg_cs = db.create_card_source("BT1-001", game.player1)
        assert egg_cs.is_digi_egg, (
            "Test fixture precondition: BT1-001 Yokomon should be a Digi-Egg"
        )
        blocked = game._is_effect_play_blocked(egg_cs)
        assert blocked, (
            "Effect-based Digi-Egg play should be blocked with Pillomon on field "
            "(C# CardCondition: IsDigimon || IsDigiEgg)"
        )

    def test_does_not_block_option_effect_play(self, debug_runner):
        """Options must remain effect-playable (e.g. Omni Sword from trash)."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, ["BT9-033"])
        game = runner.game
        self._fire_on_play(game, perm)

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        option_cs = db.create_card_source("ST1-13", game.player1)  # Mini Flame
        assert option_cs.is_option, (
            "Test fixture precondition: ST1-13 should be an Option"
        )
        blocked = game._is_effect_play_blocked(option_cs)
        assert not blocked, (
            "Effect-based Option play should NOT be blocked by Pillomon"
        )

    def test_normal_hand_play_not_blocked(self, debug_runner):
        """Normal hand plays during Main phase must still work.

        CANNOT_PLAY_BY_EFFECT is ONLY checked inside effect_play_from_zone
        (see game/effects.py); the hand-play path uses
        `_is_play_blocked_by_modifier` which only checks CANNOT_PLAY_CARD.
        Pillomon must NOT register that more-restrictive modifier type.
        """
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, ["BT9-033"])
        game = runner.game
        self._fire_on_play(game, perm)

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        digimon_cs = db.create_card_source("ST1-03", game.player1)  # Agumon

        # The normal hand-play check must NOT be blocked.
        assert not game._is_play_blocked_by_modifier(digimon_cs), (
            "Normal hand plays must NOT be blocked: Pillomon only restricts "
            "effect-based plays (CANNOT_PLAY_BY_EFFECT), not CANNOT_PLAY_CARD"
        )

        # And no CANNOT_PLAY_CARD modifier should have been registered at all.
        cannot_play_entries = game.modifiers._modifiers.get(
            ModifierType.CANNOT_PLAY_CARD, []
        )
        assert len(cannot_play_entries) == 0, (
            "Pillomon should not register CANNOT_PLAY_CARD (that would also "
            "block normal hand plays)"
        )

    def test_blocks_effect_plays_for_both_players(self, debug_runner):
        """`[All Turns] Players can't play Digimon by effects` affects BOTH
        players — modifier is centrally registered so opponent Digimon effect
        plays are blocked too."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        # Put Pillomon on player 1's field.
        perm = runner.place_on_field(1, ["BT9-033"])
        game = runner.game
        self._fire_on_play(game, perm)

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        # Create a Digimon CardSource owned by player 2 — opponent of Pillomon.
        opp_digimon_cs = db.create_card_source("ST1-03", game.player2)

        blocked = game._is_effect_play_blocked(opp_digimon_cs)
        assert blocked, (
            "Opponent's effect-based Digimon plays should also be blocked "
            "([All Turns] affects both players)"
        )

    def test_condition_true_when_on_field(self, debug_runner):
        """Sanity: the effect's can_use_condition returns True while Pillomon
        is actually on the field."""
        runner = debug_runner(deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=5)
        perm = runner.place_on_field(1, ["BT9-033"])
        top_card = perm.top_card
        effects = top_card.effect_list(None)

        on_play_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ]
        assert len(on_play_effects) >= 1
        assert on_play_effects[0].can_use_condition({}) is True, (
            "Condition should be True while Pillomon is on the field"
        )
