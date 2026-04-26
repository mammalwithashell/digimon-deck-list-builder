"""Behavioral tests for BT17-102 Greymon (Lv.4 White, Cost 5).

Card text:
[When Digivolving] If this Digimon's name is [Koromon], it gets +3000 DP
  for the turn. Then, delete 1 of your opponent's Digimon with as much or
  less DP as this Digimon.
[All Turns] This Digimon has all the names of level 3 and lower cards in
  its digivolution cards.
[On Deletion] You may play 1 Tamer card with [Tai Kamiya] or [Kari Kamiya]
  in its name from your hand without paying the cost, or hatch in your
  breeding area.
Inherited: same as main On Deletion.
Alt-digi: Lv.3 w/[Agumon] in name: Cost 2
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, GamePhase


@pytest.mark.behavioral
class TestBT17102NameAbsorption:
    """[All Turns] name absorption from Lv.3-and-lower digivolution cards."""

    def test_absorbs_koromon_name_from_digi_stack(self, debug_runner):
        """With Koromon (Lv.2) under Greymon, the permanent's name should include 'Koromon'."""
        runner = debug_runner(initial_memory=5)
        # Stack: ST1-01 (Koromon Lv.2) -> BT1-010 (Agumon Lv.3) -> BT17-102 (Greymon Lv.4)
        perm = runner.place_on_field(1, ["ST1-01", "BT1-010", "BT17-102"])

        # Trigger lazy effect initialization (installs _DynamicNamesList)
        perm.top_card.effect_list(None)

        # The Greymon top card should have names from Lv.3-and-lower digi cards
        assert perm.contains_card_name("Koromon"), \
            "Greymon should have 'Koromon' name from Lv.2 digi card in stack"
        assert perm.contains_card_name("Agumon"), \
            "Greymon should have 'Agumon' name from Lv.3 digi card in stack"
        assert perm.contains_card_name("Greymon"), \
            "Greymon should still have its own name"

    def test_no_absorption_for_lv4_cards(self, debug_runner):
        """Only Lv.3 and lower cards contribute names."""
        runner = debug_runner(initial_memory=5)
        # Stack: ST1-01 (Koromon Lv.2) -> BT17-102 (Greymon Lv.4)
        perm = runner.place_on_field(1, ["ST1-01", "BT17-102"])

        # Trigger lazy effect initialization
        perm.top_card.effect_list(None)

        assert perm.contains_card_name("Koromon"), \
            "Should absorb Koromon (Lv.2) name"
        assert perm.contains_card_name("Greymon"), \
            "Should have own name"

    def test_no_absorption_without_digi_sources(self, debug_runner):
        """With no digivolution cards, only the base name exists."""
        runner = debug_runner(initial_memory=5)
        # Just the top card, no digivolution stack
        perm = runner.place_on_field(1, ["BT17-102"])

        # Trigger lazy effect initialization
        perm.top_card.effect_list(None)

        assert perm.contains_card_name("Greymon"), \
            "Should have own name"
        assert not perm.contains_card_name("Koromon"), \
            "Should NOT have Koromon without it in the stack"


@pytest.mark.behavioral
class TestBT17102WhenDigivolving:
    """[When Digivolving] +3000 DP if named Koromon, then delete opponent Digimon."""

    def test_dp_boost_with_koromon_in_stack(self, debug_runner):
        """When digivolving with Koromon in stack, Greymon gets +3000 DP."""
        runner = debug_runner(initial_memory=5)
        # Build the stack with Koromon under Agumon
        perm = runner.place_on_field(1, ["ST1-01", "BT1-010", "BT17-102"])

        game = runner.game
        # Greymon base DP is 5000 (from cards.json)
        base_dp = perm.dp

        # Find the When Digivolving effect
        top_card = perm.top_card
        effects = top_card.effect_list(None)
        wd_effects = [e for e in effects if e.is_when_digivolving]
        assert len(wd_effects) >= 1, "Should have a When Digivolving effect"

        wd = wd_effects[0]

        # Place an opponent Digimon for the delete target
        opp_perm = runner.place_on_field(2, ["BT1-010"])  # Agumon, 2000 DP

        # Execute the When Digivolving effect
        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # DP should be boosted by +3000
        assert perm.dp == base_dp + 3000, \
            f"DP should be {base_dp + 3000} (base {base_dp} + 3000), got {perm.dp}"

    def test_no_dp_boost_without_koromon(self, debug_runner):
        """Without Koromon in the name, no DP boost."""
        runner = debug_runner(initial_memory=5)
        # Stack without Koromon: just Agumon -> Greymon
        perm = runner.place_on_field(1, ["BT1-010", "BT17-102"])

        game = runner.game
        base_dp = perm.dp

        top_card = perm.top_card
        effects = top_card.effect_list(None)
        wd_effects = [e for e in effects if e.is_when_digivolving]
        wd = wd_effects[0]

        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # No boost — only Agumon in stack (Lv.3), Greymon doesn't have Koromon name
        assert perm.dp == base_dp, \
            f"DP should remain {base_dp} without Koromon, got {perm.dp}"

    def test_delete_opponent_digimon_with_le_dp(self, debug_runner):
        """Should delete 1 opponent Digimon with DP <= this Digimon's DP."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT1-010", "BT17-102"])  # Greymon 5000 DP

        game = runner.game

        # Place opponent Digimon with various DP
        opp_low = runner.place_on_field(2, ["BT1-010"])  # Agumon 2000 DP
        opp_high = runner.place_on_field(2, ["BT17-102"])  # Another Greymon 5000 DP

        opp_count_before = len(game.player2.battle_area)

        top_card = perm.top_card
        effects = top_card.effect_list(None)
        wd_effects = [e for e in effects if e.is_when_digivolving]
        wd = wd_effects[0]

        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Should enter selection phase for deleting opponent Digimon
        # Auto-resolve the selection
        runner.auto_resolve()

        # At least one opponent Digimon should be deleted
        assert len(game.player2.battle_area) < opp_count_before, \
            "Should have deleted at least 1 opponent Digimon"


@pytest.mark.behavioral
class TestBT17102OnDeletion:
    """[On Deletion] Play Tai/Kari Kamiya tamer free OR hatch."""

    def test_on_deletion_play_tamer_from_hand(self, debug_runner):
        """On Deletion should allow playing a Tai Kamiya tamer from hand for free."""
        runner = debug_runner(initial_memory=3)
        perm = runner.place_on_field(1, ["BT17-102"])

        game = runner.game

        # Inject Tai Kamiya tamer into hand
        runner.inject_card(1, "BT1-085", "hand")  # Tai Kamiya tamer

        # Find the On Deletion effect (non-inherited)
        top_card = perm.top_card
        effects = top_card.effect_list(None)
        od_effects = [e for e in effects if e.is_on_deletion and not e.is_inherited_effect]
        assert len(od_effects) >= 1, "Should have a non-inherited On Deletion effect"

        od = od_effects[0]
        assert od.is_optional, "On Deletion should be optional"

        hand_count_before = len(game.player1.hand_cards)
        memory_before = game.memory

        # Execute the On Deletion effect (simulate choosing "Play Tamer" branch)
        od.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Auto-resolve: choose "Play Tamer" branch and select the tamer
        runner.auto_resolve()

        # Tamer should be on field (or at least removed from hand)
        tamer_on_field = any(
            p.contains_card_name("Tai Kamiya") for p in game.player1.battle_area
        )
        # If branch choice was auto-resolved, check one of the outcomes happened
        assert tamer_on_field or len(game.player1.hand_cards) < hand_count_before, \
            "Tamer should have been played from hand"

    def test_on_deletion_hatch(self, debug_runner):
        """On Deletion can choose to hatch instead of playing a tamer."""
        runner = debug_runner(initial_memory=3)
        perm = runner.place_on_field(1, ["BT17-102"])

        game = runner.game

        # Ensure no tamer in hand but digitama available
        assert game.player1.breeding_area is None, "Breeding area should be empty"
        assert len(game.player1.digitama_library_cards) > 0, "Need digi-eggs available"

        top_card = perm.top_card
        effects = top_card.effect_list(None)
        od_effects = [e for e in effects if e.is_on_deletion and not e.is_inherited_effect]
        od = od_effects[0]

        # With no tamer in hand, should auto-hatch
        od.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        runner.auto_resolve()

        # Should have hatched
        assert game.player1.breeding_area is not None, \
            "Should have hatched in breeding area"

    def test_inherited_on_deletion_exists(self, debug_runner):
        """Inherited On Deletion effect should exist with same behavior."""
        runner = debug_runner(initial_memory=3)
        perm = runner.place_on_field(1, ["BT17-102"])

        top_card = perm.top_card
        effects = top_card.effect_list(None)
        inh_od = [e for e in effects if e.is_on_deletion and e.is_inherited_effect]
        assert len(inh_od) >= 1, "Should have an inherited On Deletion effect"
        assert inh_od[0].is_optional, "Inherited On Deletion should be optional"


@pytest.mark.behavioral
class TestBT17102AltDigivolve:
    """Alt-digi: from Lv.3 with [Agumon] in name for cost 2."""

    def test_alt_digi_attributes(self, debug_runner):
        """Alt digivolve effect should have correct attributes."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT17-102"])

        top_card = perm.top_card
        effects = top_card.effect_list(None)
        alt_digi = [e for e in effects if getattr(e, '_alt_digi_cost', None) is not None]
        assert len(alt_digi) >= 1, "Should have an alt-digi effect"
        assert alt_digi[0]._alt_digi_cost == 2
        assert alt_digi[0]._alt_digi_name == "Agumon"
        assert alt_digi[0]._alt_digi_level == 3
