"""Behavioral tests for BT23-007 Musclemon (Lv.3 Red, Cost 3, DP 2000).

Card text:
Effect: [Security] At the end of the battle, play this card without paying the cost.
Inherited: Link Requirements [Link] [Appmon] trait: Cost 1
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT23007SecurityPlay:
    """[Security] At the end of the battle, play this card without paying the cost."""

    def _get_security_effect(self, card):
        """Find the SecuritySkill effect from BT23-007."""
        effects = card.effect_list(None)
        sec_effs = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(sec_effs) == 1, (
            f"BT23-007 should have exactly 1 SecuritySkill effect, found {len(sec_effs)}"
        )
        return sec_effs[0]

    def test_security_effect_has_correct_timing(self, debug_runner):
        """Security effect must use EffectTiming.SecuritySkill."""
        runner = debug_runner(initial_memory=0)
        card = runner.inject_card(1, "BT23-007", "hand")
        effects = card.effect_list(None)
        sec_effs = [e for e in effects if e.timing == EffectTiming.SecuritySkill]
        assert len(sec_effs) == 1, (
            "Must have exactly 1 SecuritySkill-timed effect"
        )

    def test_security_effect_is_security_effect_flag(self, debug_runner):
        """Security effect must have is_security_effect = True."""
        runner = debug_runner(initial_memory=0)
        card = runner.inject_card(1, "BT23-007", "hand")
        eff = self._get_security_effect(card)
        assert eff.is_security_effect is True, "Must be marked as security effect"

    def test_security_effect_has_process_callback(self, debug_runner):
        """Security effect must have an on_process_callback to play the card."""
        runner = debug_runner(initial_memory=0)
        card = runner.inject_card(1, "BT23-007", "hand")
        eff = self._get_security_effect(card)
        assert eff.on_process_callback is not None, (
            "Security effect must have a process callback to play the card"
        )

    def test_security_play_puts_card_on_field(self, debug_runner):
        """Invoking the security process callback should play the card onto the field."""
        runner = debug_runner(initial_memory=0)
        game = runner.game
        player = game.player1

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT23-007", player)
        player.security_cards.append(cs)

        eff = self._get_security_effect(cs)
        eff.on_process_callback({
            'player': player,
            'game': game,
        })

        # Card should now be on the field
        field_ids = [
            p.top_card.c_entity_base.card_id
            for p in player.battle_area
            if p.top_card and p.top_card.c_entity_base
        ]
        assert "BT23-007" in field_ids, (
            f"BT23-007 should be on the field after security play, field={field_ids}"
        )

    def test_security_play_does_not_cost_memory(self, debug_runner):
        """Security play should not pay any memory cost."""
        runner = debug_runner(initial_memory=3)
        game = runner.game
        player = game.player1

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source("BT23-007", player)
        player.security_cards.append(cs)

        memory_before = game.memory
        eff = self._get_security_effect(cs)
        eff.on_process_callback({
            'player': player,
            'game': game,
        })

        assert game.memory == memory_before, (
            f"Memory should not change during security play, "
            f"was {memory_before}, now {game.memory}"
        )


@pytest.mark.behavioral
class TestBT23007LinkRequirement:
    """Inherited: Link Requirements [Link] [Appmon] trait: Cost 1."""

    def _get_link_effect(self, card):
        """Find the inherited link requirement effect from BT23-007."""
        effects = card.effect_list(None)
        link_effs = [
            e for e in effects
            if e.is_inherited_effect and getattr(e, '_is_link', False)
        ]
        assert len(link_effs) == 1, (
            f"BT23-007 should have exactly 1 inherited link effect, found {len(link_effs)}"
        )
        return link_effs[0]

    def test_has_inherited_link_effect(self, debug_runner):
        """Card should have an inherited effect with _is_link = True."""
        runner = debug_runner(initial_memory=0)
        card = runner.inject_card(1, "BT23-007", "hand")
        effects = card.effect_list(None)
        link_effs = [
            e for e in effects
            if e.is_inherited_effect and getattr(e, '_is_link', False)
        ]
        assert len(link_effs) == 1, (
            "Must have exactly 1 inherited link effect"
        )

    def test_link_cost_is_1(self, debug_runner):
        """Link cost should be 1."""
        runner = debug_runner(initial_memory=0)
        card = runner.inject_card(1, "BT23-007", "hand")
        eff = self._get_link_effect(card)
        assert getattr(eff, '_link_cost', None) == 1, (
            f"Link cost should be 1, got {getattr(eff, '_link_cost', None)}"
        )

    def test_link_trait_is_appmon(self, debug_runner):
        """Link trait requirement should be 'Appmon'."""
        runner = debug_runner(initial_memory=0)
        card = runner.inject_card(1, "BT23-007", "hand")
        eff = self._get_link_effect(card)
        assert getattr(eff, '_link_trait', None) == "Appmon", (
            f"Link trait should be 'Appmon', got {getattr(eff, '_link_trait', None)}"
        )

    def test_link_effect_is_inherited(self, debug_runner):
        """Link effect must be marked as inherited."""
        runner = debug_runner(initial_memory=0)
        card = runner.inject_card(1, "BT23-007", "hand")
        eff = self._get_link_effect(card)
        assert eff.is_inherited_effect is True, "Link effect must be inherited"


@pytest.mark.behavioral
class TestBT23007AltDigivolve:
    """Alternate digivolution: Lv.2 with [Appmon] trait for cost 0."""

    def _get_alt_digi_effect(self, card):
        """Find the alt digivolve effect from BT23-007."""
        effects = card.effect_list(None)
        alt_effs = [
            e for e in effects
            if getattr(e, '_alt_digi_cost', None) is not None
        ]
        assert len(alt_effs) == 1, (
            f"BT23-007 should have exactly 1 alt digi effect, found {len(alt_effs)}"
        )
        return alt_effs[0]

    def test_alt_digi_cost_is_0(self, debug_runner):
        """Alt digivolution cost should be 0."""
        runner = debug_runner(initial_memory=0)
        card = runner.inject_card(1, "BT23-007", "hand")
        eff = self._get_alt_digi_effect(card)
        assert eff._alt_digi_cost == 0

    def test_alt_digi_level_is_2(self, debug_runner):
        """Alt digivolution source level should be 2."""
        runner = debug_runner(initial_memory=0)
        card = runner.inject_card(1, "BT23-007", "hand")
        eff = self._get_alt_digi_effect(card)
        assert eff._alt_digi_level == 2

    def test_alt_digi_trait_is_appmon(self, debug_runner):
        """Alt digivolution trait requirement should be 'Appmon'."""
        runner = debug_runner(initial_memory=0)
        card = runner.inject_card(1, "BT23-007", "hand")
        eff = self._get_alt_digi_effect(card)
        assert eff._alt_digi_trait == "Appmon"
