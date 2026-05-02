"""Behavioral tests for LM-021 Agumon - Bond of Bravery (Lv.7 Red, DP 14000, Cost 8).

Card text:
  Ace: Digivolve from [Agumon] for 3 cost when security <= 2
  [Hand] [Counter] <Blast Digivolve>
  [On Play] [When Digivolving] Delete any number of your opponent's Digimon
  whose total DP adds up to equal or less than this Digimon's DP.
  [When Attacking] [Once Per Turn] If you have a Tamer, trash the top card
  of your opponent's security stack.
  Inherited: Ace Overflow <-5>

C# reference:
  - Alt digi: from Agumon, cost 3, condition: owner security <= 2
  - Blast Digivolve
  - On Play / When Digivolving: delete opponent Digimon up to this DP (NOT optional)
  - When Attacking: once per turn, if owner has Tamer, trash opponent top security
  - Uses IDestroySecurity (top = index 0 in engine convention)
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestLM021AgumonBondOfBravery:
    """Tests for LM-021 Agumon - Bond of Bravery."""

    def test_has_blast_digivolve(self, debug_runner):
        """Should have Blast Digivolve keyword."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["LM-021"])

        card = perm.top_card
        effects = card.effect_list(None)
        blast = [e for e in effects if getattr(e, '_is_blast_digivolve', False)]
        assert len(blast) == 1, "Should have Blast Digivolve"
        assert blast[0].is_counter_effect, "Blast Digivolve should be a counter effect"

    def test_has_alt_digi_from_agumon(self, debug_runner):
        """Should have alt digi from Agumon for cost 3 with security <= 2 condition."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["LM-021"])

        card = perm.top_card
        effects = card.effect_list(EffectTiming.NoTiming)
        alt_digi = [e for e in effects if getattr(e, '_alt_digi_cost', None) is not None]
        assert len(alt_digi) >= 1, "Should have alt digi effect for Ace"

        found = any(
            getattr(e, '_alt_digi_name', None) == "Agumon"
            and getattr(e, '_alt_digi_cost', None) == 3
            for e in alt_digi
        )
        assert found, "Should have alt digi: from Agumon for cost 3"

    def test_has_on_play_effect(self, debug_runner):
        """Should have [On Play] delete effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["LM-021"])

        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [e for e in effects
                   if e.timing == EffectTiming.OnEnterFieldAnyone and e.is_on_play]
        assert len(on_play) == 1, "Should have On Play effect"

    def test_has_when_digivolving_effect(self, debug_runner):
        """Should have [When Digivolving] delete effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["LM-021"])

        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects
                     if e.timing == EffectTiming.OnEnterFieldAnyone and e.is_when_digivolving]
        assert len(when_digi) == 1, "Should have When Digivolving effect"

    def test_when_attacking_requires_tamer(self, debug_runner):
        """[When Attacking] should only activate when owner has a Tamer."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["LM-021"])
        game = runner.game

        card = perm.top_card
        effects = card.effect_list(None)
        atk_effects = [e for e in effects if e.timing == EffectTiming.OnUseAttack]
        assert len(atk_effects) == 1

        eff = atk_effects[0]
        # No tamer on field -> should fail
        assert not eff.can_use_condition({}), \
            "Should fail without a Tamer on field"

        # Place a tamer
        runner.place_on_field(1, ["ST1-12"])
        assert eff.can_use_condition({}), \
            "Should pass with a Tamer on field"

    def test_when_attacking_once_per_turn(self, debug_runner):
        """[When Attacking] should be once per turn."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["LM-021"])

        card = perm.top_card
        effects = card.effect_list(None)
        atk_effects = [e for e in effects if e.timing == EffectTiming.OnUseAttack]
        assert atk_effects[0].max_count_per_turn == 1

    def test_when_attacking_trashes_top_security(self, debug_runner):
        """[When Attacking] should trash TOP security (index 0), not bottom."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["LM-021"])
        runner.place_on_field(1, ["ST1-12"])  # Tamer required
        game = runner.game

        # Give opponent 3 security cards (security_top inserts at index 0)
        # Insert in reverse order so ST1-03 ends up at top
        runner.inject_card(2, "ST1-02", "security_top")
        runner.inject_card(2, "ST1-07", "security_top")
        runner.inject_card(2, "ST1-03", "security_top")

        top_sec_id = game.player2.security_cards[0].c_entity_base.card_id
        sec_count_before = len(game.player2.security_cards)
        trash_before = len(game.player2.trash_cards)

        card = perm.top_card
        effects = card.effect_list(None)
        atk_eff = [e for e in effects if e.timing == EffectTiming.OnUseAttack][0]

        atk_eff.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert len(game.player2.security_cards) == sec_count_before - 1, \
            "Should remove 1 security card"
        assert len(game.player2.trash_cards) == trash_before + 1, \
            "Should add 1 card to opponent's trash"
        # The trashed card should be the TOP card (index 0)
        trashed = game.player2.trash_cards[-1]
        assert trashed.c_entity_base.card_id == top_sec_id, \
            f"Should trash TOP security ({top_sec_id}), not bottom"

    def test_has_ace_overflow_inherited(self, debug_runner):
        """Should have Ace Overflow <-5> as inherited effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["LM-021"])

        card = perm.top_card
        effects = card.effect_list(None)
        ace = [e for e in effects
               if e.is_inherited_effect and "ace" in e.effect_name.lower()]
        assert len(ace) == 1, "Should have Ace Overflow inherited effect"
