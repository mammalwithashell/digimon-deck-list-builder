"""Behavioral tests for BT22-032 ShoeShoemon (Lv.4 Yellow Puppet/LIBERATOR).

Card text:
Effect: [On Deletion] You may play 1 level 3 Digimon card with the [Puppet]
    trait from your hand without paying the cost.
Inherited Effect: [When Attacking] [Once Per Turn] 1 of your opponent's Digimon
    gets -2000 DP for the turn.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming

# Minimal deck filler (50 cards) that avoids Puppet/BT22 conflicts
_FILLER = ["ST1-05"] * 50


@pytest.mark.behavioral
class TestBT22032OnDeletion:
    """Tests for BT22-032 [On Deletion] effect."""

    def test_on_deletion_plays_lv3_puppet_from_hand(self, debug_runner):
        """On Deletion should allow playing a Lv.3 Puppet Digimon from hand free."""
        runner = debug_runner(deck1=_FILLER, deck2=_FILLER, initial_memory=0)
        game = runner.game

        # Place ShoeShoemon on field
        perm = runner.place_on_field(1, ["BT22-032"])

        # Put a Lv.3 Puppet in hand: BT22-029 Shoemon (Lv.3 Puppet/LIBERATOR)
        runner.inject_card(1, "BT22-029", zone="hand")

        # Find the on-deletion effect
        top_card = perm.top_card
        effects = top_card.effect_list(None)
        deletion_effects = [e for e in effects if e.is_on_deletion]
        assert len(deletion_effects) == 1, "Should have exactly 1 on-deletion effect"

        del_eff = deletion_effects[0]
        assert del_eff.is_optional, "On Deletion effect must be optional (You may)"

        # Simulate deletion: process the effect
        del_eff.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Auto-resolve any selection phases (picking the card from hand)
        runner.auto_resolve(max_steps=10)

        # The Lv.3 Puppet should now be on the field (played from hand free)
        puppet_on_field = any(
            p.top_card and p.top_card.c_entity_base
            and p.top_card.c_entity_base.card_id == "BT22-029"
            for p in game.player1.battle_area
        )
        assert puppet_on_field, "Lv.3 Puppet should be played onto the field from hand"

    def test_on_deletion_no_puppet_in_hand_noop(self, debug_runner):
        """If no Lv.3 Puppet in hand, the effect should resolve without error."""
        runner = debug_runner(deck1=_FILLER, deck2=_FILLER, initial_memory=0)
        game = runner.game

        perm = runner.place_on_field(1, ["BT22-032"])

        # Put a non-Puppet Lv.3 in hand (BT22-008 Agumon - Reptile)
        runner.inject_card(1, "BT22-008", zone="hand")

        hand_before = len(game.player1.hand_cards)

        top_card = perm.top_card
        effects = top_card.effect_list(None)
        del_eff = [e for e in effects if e.is_on_deletion][0]

        del_eff.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve(max_steps=10)

        # Hand should be unchanged (no valid target)
        assert len(game.player1.hand_cards) == hand_before, \
            "No Lv.3 Puppet in hand, so nothing should be played"

    def test_on_deletion_ignores_lv4_puppet(self, debug_runner):
        """Should not play a Lv.4 Puppet (must be level 3)."""
        runner = debug_runner(deck1=_FILLER, deck2=_FILLER, initial_memory=0)
        game = runner.game

        perm = runner.place_on_field(1, ["BT22-032"])

        # Inject another BT22-032 (Lv.4 Puppet) into hand - should NOT be playable
        runner.inject_card(1, "BT22-032", zone="hand")

        hand_before = len(game.player1.hand_cards)

        top_card = perm.top_card
        effects = top_card.effect_list(None)
        del_eff = [e for e in effects if e.is_on_deletion][0]

        del_eff.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve(max_steps=10)

        assert len(game.player1.hand_cards) == hand_before, \
            "Lv.4 Puppet should not be valid (must be level 3)"

    def test_on_deletion_free_play_no_memory_cost(self, debug_runner):
        """Playing via On Deletion should not cost memory."""
        runner = debug_runner(deck1=_FILLER, deck2=_FILLER, initial_memory=3)
        game = runner.game

        perm = runner.place_on_field(1, ["BT22-032"])
        runner.inject_card(1, "BT22-029", zone="hand")

        memory_before = game.memory

        top_card = perm.top_card
        effects = top_card.effect_list(None)
        del_eff = [e for e in effects if e.is_on_deletion][0]

        del_eff.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve(max_steps=10)

        assert game.memory == memory_before, \
            "Free play should not change memory"


@pytest.mark.behavioral
class TestBT22032InheritedWhenAttacking:
    """Tests for BT22-032 inherited [When Attacking] [Once Per Turn] -2000 DP."""

    def test_inherited_wa_reduces_opponent_dp(self, debug_runner):
        """Inherited WA should give -2000 DP to an opponent's Digimon."""
        runner = debug_runner(deck1=_FILLER, deck2=_FILLER, initial_memory=5)
        game = runner.game

        # Place a Digimon with BT22-032 as digivolution source
        # Stack: BT22-029 (Lv.3) -> BT22-032 (Lv.4) on top
        attacker = runner.place_on_field(1, ["BT22-029", "BT22-032"])

        # Place opponent Digimon (ST1-05 Birdramon, 5000 DP)
        target = runner.place_on_field(2, ["ST1-05"])
        dp_before = target.dp

        # The inherited effect is on the BT22-032 card source (index 0 = bottom)
        # card_sources are bottom-to-top: [BT22-029, BT22-032]
        # BT22-032 is at index 1 (top card)
        bt22_032_card = None
        for cs in attacker.card_sources:
            if cs.c_entity_base and cs.c_entity_base.card_id == "BT22-032":
                bt22_032_card = cs
                break
        assert bt22_032_card is not None

        effects = bt22_032_card.effect_list(None)
        wa_effects = [e for e in effects if e.is_on_attack and e.is_inherited_effect]
        assert len(wa_effects) == 1, "BT22-032 should have 1 inherited WA effect"

        wa_eff = wa_effects[0]

        # Process the effect
        wa_eff.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': attacker,
        })
        runner.auto_resolve(max_steps=10)

        # Target should have -2000 DP
        assert target.dp == dp_before - 2000, \
            f"Target DP should be {dp_before - 2000}, got {target.dp}"

    def test_inherited_wa_once_per_turn(self, debug_runner):
        """OPT: the inherited WA should only fire once per turn."""
        runner = debug_runner(deck1=_FILLER, deck2=_FILLER, initial_memory=5)
        game = runner.game

        attacker = runner.place_on_field(1, ["BT22-029", "BT22-032"])

        bt22_032_card = None
        for cs in attacker.card_sources:
            if cs.c_entity_base and cs.c_entity_base.card_id == "BT22-032":
                bt22_032_card = cs
                break

        effects = bt22_032_card.effect_list(None)
        wa_eff = [e for e in effects if e.is_on_attack and e.is_inherited_effect][0]

        assert wa_eff.max_count_per_turn == 1, "Should be Once Per Turn"
        assert wa_eff.hash_string == "BT22_032_WA", "Hash string for OPT tracking"

    def test_inherited_wa_effect_flags(self, debug_runner):
        """Inherited WA should have correct flags for engine dispatch."""
        runner = debug_runner(deck1=_FILLER, deck2=_FILLER, initial_memory=5)
        game = runner.game

        attacker = runner.place_on_field(1, ["BT22-029", "BT22-032"])

        bt22_032_card = None
        for cs in attacker.card_sources:
            if cs.c_entity_base and cs.c_entity_base.card_id == "BT22-032":
                bt22_032_card = cs
                break

        effects = bt22_032_card.effect_list(None)
        wa_eff = [e for e in effects if e.is_on_attack and e.is_inherited_effect][0]

        assert wa_eff.is_on_attack, "Must have is_on_attack flag"
        assert wa_eff.is_inherited_effect, "Must be marked as inherited"

    def test_inherited_wa_no_opponent_digimon_noop(self, debug_runner):
        """If opponent has no Digimon, effect resolves without error."""
        runner = debug_runner(deck1=_FILLER, deck2=_FILLER, initial_memory=5)
        game = runner.game

        attacker = runner.place_on_field(1, ["BT22-029", "BT22-032"])
        # No opponent Digimon

        bt22_032_card = None
        for cs in attacker.card_sources:
            if cs.c_entity_base and cs.c_entity_base.card_id == "BT22-032":
                bt22_032_card = cs
                break

        effects = bt22_032_card.effect_list(None)
        wa_eff = [e for e in effects if e.is_on_attack and e.is_inherited_effect][0]

        # Should not raise
        wa_eff.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': attacker,
        })
        runner.auto_resolve(max_steps=10)
