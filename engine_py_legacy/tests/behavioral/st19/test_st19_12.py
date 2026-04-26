"""Behavioral tests for ST19-12 Cendrillmon (Lv.6 Yellow Puppet/LIBERATOR).

Card text:
  <Overclock ([Puppet] Trait)> (At the end of your turn, by deleting 1 of your
  Tokens or other [Puppet] trait Digimon, this Digimon attacks a player without
  suspending.)
  <Blocker>
  [When Digivolving] You may play 2 [Familiar] Tokens.
  (Digimon/Yellow/3000 DP/[On Deletion] 1 of your opponent's Digimon gets
  -3000 DP for the turn.)
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestST19_12Cendrillmon:
    """Tests for ST19-12 Cendrillmon."""

    # ── Keyword: Overclock ──────────────────────────────────────────

    def test_has_overclock(self, debug_runner):
        """ST19-12 should have an effect with _is_overclock = True."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["ST19-12"])

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        overclock_effects = [e for e in effects if getattr(e, '_is_overclock', False)]
        assert len(overclock_effects) >= 1, "Cendrillmon should have Overclock keyword"

    # ── Keyword: Blocker ────────────────────────────────────────────

    def test_has_blocker(self, debug_runner):
        """ST19-12 should have an effect with _is_blocker = True."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["ST19-12"])

        card = perm.card_sources[0]
        effects = card.effect_list(None)
        blocker_effects = [e for e in effects if getattr(e, '_is_blocker', False)]
        assert len(blocker_effects) >= 1, "Cendrillmon should have Blocker keyword"

    # ── When Digivolving: Play 2 Familiar Tokens ───────────────────

    def test_when_digivolving_effect_exists(self, debug_runner):
        """The When Digivolving effect should be present, optional, and have
        correct timing."""
        runner = debug_runner(initial_memory=5)
        # Place as a digivolution stack (Lv5 -> Lv6)
        perm = runner.place_on_field(1, ["ST19-09", "ST19-12"])

        card = perm.card_sources[-1]  # ST19-12 is top card
        effects = card.effect_list(None)
        wd_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ]
        assert len(wd_effects) == 1, "Should have exactly 1 When Digivolving effect"
        assert wd_effects[0].is_optional, "When Digivolving effect must be optional"

    def test_when_digivolving_plays_2_familiar_tokens(self, debug_runner):
        """Activating the When Digivolving effect should place 2 Familiar Tokens
        on the player's field."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["ST19-09", "ST19-12"])

        game = runner.game
        player = game.player1
        field_before = len(player.battle_area)

        card = perm.card_sources[-1]
        effects = card.effect_list(None)
        wd = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ][0]

        wd.on_process_callback({
            'player': player,
            'game': game,
            'permanent': perm,
        })

        # Should have 2 more permanents on field
        assert len(player.battle_area) == field_before + 2, \
            f"Should play 2 tokens; had {field_before}, now {len(player.battle_area)}"

        # Verify they are Familiar Tokens
        new_perms = player.battle_area[field_before:]
        for p in new_perms:
            assert p.is_token, "Played permanent should be a token"
            top = p.card_sources[0]
            assert top.c_entity_base.card_name_eng == "Familiar Token", \
                f"Token should be Familiar Token, got {top.c_entity_base.card_name_eng}"

    def test_when_digivolving_tokens_are_yellow(self, debug_runner):
        """Familiar Tokens should be Yellow Digimon with 3000 DP."""
        from engine_py_legacy.engine.data.enums import CardColor
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["ST19-09", "ST19-12"])

        game = runner.game
        player = game.player1
        field_before = len(player.battle_area)

        card = perm.card_sources[-1]
        effects = card.effect_list(None)
        wd = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ][0]

        wd.on_process_callback({
            'player': player,
            'game': game,
            'permanent': perm,
        })

        token_perm = player.battle_area[field_before]
        token_card = token_perm.card_sources[0]
        assert CardColor.Yellow in token_card.c_entity_base.card_colors, \
            "Familiar Token should be Yellow"
        assert token_card.c_entity_base.dp == 3000, \
            "Familiar Token should have 3000 DP"

    def test_when_digivolving_condition_requires_on_field(self, debug_runner):
        """The When Digivolving condition should fail if the card is not on field."""
        runner = debug_runner(initial_memory=5)

        # Inject into hand instead of field
        card_source = runner.inject_card(1, "ST19-12", zone="hand")

        effects = card_source.effect_list(None)
        wd_effects = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ]
        if wd_effects:
            assert not wd_effects[0].can_use_condition({}), \
                "Condition should fail when card is not on field"

    def test_when_digivolving_limited_by_field_slots(self, debug_runner):
        """If only 1 field slot is available, only 1 token should be played
        (effect_play_token handles the cap internally)."""
        from engine_py_legacy.engine.game.constants import FIELD_SLOTS
        runner = debug_runner(initial_memory=5)

        game = runner.game
        player = game.player1

        # Fill field to FIELD_SLOTS - 2 (cendrillmon + fillers)
        perm = runner.place_on_field(1, ["ST19-09", "ST19-12"])
        # Fill remaining slots minus 1 (so only 1 slot free after cendrillmon)
        slots_used = len(player.battle_area)
        slots_to_fill = FIELD_SLOTS - slots_used - 1  # leave exactly 1 free
        for i in range(slots_to_fill):
            runner.place_on_field(1, ["ST19-09"])

        assert len(player.battle_area) == FIELD_SLOTS - 1, \
            f"Should have {FIELD_SLOTS - 1} on field, got {len(player.battle_area)}"

        card = perm.card_sources[-1]
        effects = card.effect_list(None)
        wd = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ][0]

        wd.on_process_callback({
            'player': player,
            'game': game,
            'permanent': perm,
        })

        # Should have filled to FIELD_SLOTS (only 1 token fits)
        assert len(player.battle_area) == FIELD_SLOTS, \
            f"Should play only 1 token when 1 slot free; got {len(player.battle_area)}"

    def test_when_digivolving_condition_fails_field_full(self, debug_runner):
        """If the field is completely full, condition should fail."""
        from engine_py_legacy.engine.game.constants import FIELD_SLOTS
        runner = debug_runner(initial_memory=5)

        game = runner.game
        player = game.player1

        # Place cendrillmon
        perm = runner.place_on_field(1, ["ST19-09", "ST19-12"])

        # Fill remaining slots
        while len(player.battle_area) < FIELD_SLOTS:
            runner.place_on_field(1, ["ST19-09"])

        assert len(player.battle_area) == FIELD_SLOTS

        card = perm.card_sources[-1]
        effects = card.effect_list(None)
        wd = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ][0]

        assert not wd.can_use_condition({}), \
            "Condition should fail when field is full"
