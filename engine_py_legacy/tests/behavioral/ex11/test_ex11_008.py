"""Behavioral tests for EX11-008 Elizamon (Lv.3 Red Reptile/LIBERATOR).

Card text:
[When Moving] [On Play] 1 of your Digimon with the [Reptile] or [Dragonkin]
trait gains <Raid> and +3000 DP for the turn.

Inherited:
[Your Turn] [Once Per Turn] When your opponent's security stack is removed from,
gain 1 memory.

Critical bugs being validated:
1. OnMove must only fire when THIS Elizamon moves, not any permanent.
2. Inherited must only fire when OPPONENT's security is removed, not own.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


ELIZAMON_DECK = ["EX11-008"] * 50


@pytest.mark.behavioral
class TestEX11008WhenMoving:
    """[When Moving] — only fires when this Elizamon moves from breeding."""

    def test_when_moving_fires_on_own_move(self, debug_runner):
        """When Elizamon moves from breeding, it should grant Raid+3000 DP
        to a Reptile/Dragonkin Digimon."""
        runner = debug_runner(deck1=ELIZAMON_DECK, deck2=ELIZAMON_DECK, initial_memory=10)

        # Place Elizamon in breeding and a Reptile target on field
        runner.place_in_breeding(1, ["EX11-008"])
        target_perm = runner.place_on_field(1, ["EX11-008"])  # another Elizamon (Reptile)

        target_dp_before = target_perm.dp

        # Move from breeding
        runner.set_phase("Breeding")
        action = runner.find_action("Move")
        assert action is not None, "Should have Move action"
        runner.execute(action)
        runner.auto_resolve()

        # Target should have gained Raid and +3000 DP
        assert target_perm.has_keyword("_is_raid"), \
            "Target should have Raid after Elizamon moves"
        assert target_perm.dp == target_dp_before + 3000, \
            f"Target DP should be +3000: expected {target_dp_before + 3000}, got {target_perm.dp}"

    def test_when_moving_does_not_fire_for_other_permanent(self, debug_runner):
        """When a DIFFERENT Digimon moves from breeding, Elizamon's [When Moving]
        must NOT fire. This validates the moved_permanent == this card's permanent check."""
        runner = debug_runner(deck1=ELIZAMON_DECK, deck2=ELIZAMON_DECK, initial_memory=10)

        # Elizamon on field (it has [When Moving] effect)
        field_elizamon = runner.place_on_field(1, ["EX11-008"])
        # A different Digimon in breeding that will move
        runner.place_in_breeding(1, ["EX11-008"])
        # A third Reptile target on field to receive the grant
        target_perm = runner.place_on_field(1, ["EX11-008"])

        target_dp_before = target_perm.dp

        # Move the breeding Digimon (NOT the field Elizamon)
        runner.set_phase("Breeding")
        action = runner.find_action("Move")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # The breeding Digimon that moved should get the effect (its own [When Moving])
        # but the field Elizamon's [When Moving] should NOT have fired additionally.
        # With the bug, BOTH Elizamons' OnMove would fire, granting Raid/DP twice.
        # With the fix, only the MOVED Elizamon's OnMove fires once.
        # Check that total Raid grants match: only the moved one should trigger.
        # The moved permanent itself should be the one that fires (it's the Elizamon
        # that moved from breeding). The field Elizamon should NOT fire its OnMove.

        # Count how many permanents gained Raid (should be exactly 1 from the moved one)
        raid_count = sum(
            1 for p in runner.game.player1.battle_area
            if p.has_keyword("_is_raid")
        )
        # Only 1 permanent should have Raid (granted by the moved Elizamon's [When Moving])
        assert raid_count == 1, \
            f"Expected exactly 1 Raid grant (from moved Elizamon only), got {raid_count}"


@pytest.mark.behavioral
class TestEX11008OnPlay:
    """[On Play] — grants Raid + 3000 DP to a Reptile/Dragonkin Digimon."""

    def test_on_play_grants_raid_and_dp(self, debug_runner):
        """Playing Elizamon should grant Raid and +3000 DP to a Reptile/Dragonkin."""
        runner = debug_runner(deck1=ELIZAMON_DECK, deck2=ELIZAMON_DECK, initial_memory=10)

        # Put a Reptile target on field
        target_perm = runner.place_on_field(1, ["EX11-008"])
        target_dp_before = target_perm.dp

        # Play Elizamon from hand
        runner.inject_card(1, "EX11-008", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Elizamon")
        assert action is not None, "Should be able to play Elizamon"
        runner.execute(action)
        runner.auto_resolve()

        # Target should gain Raid and +3000 DP
        assert target_perm.has_keyword("_is_raid"), \
            "Target should have Raid after Elizamon on-play"
        assert target_perm.dp == target_dp_before + 3000, \
            f"Target DP should be +3000: expected {target_dp_before + 3000}, got {target_perm.dp}"

    def test_on_play_targets_reptile_only(self, debug_runner):
        """On Play should only target Digimon with Reptile or Dragonkin trait."""
        runner = debug_runner(deck1=ELIZAMON_DECK, deck2=ELIZAMON_DECK, initial_memory=10)

        # Place a non-Reptile/non-Dragonkin on field (use a generic test card)
        from tests.helpers.game_builder import make_card
        from digimon_gym.engine.core.permanent import Permanent
        from digimon_gym.engine.core.entity_base import CEntity_Base

        # Create a Digimon without Reptile/Dragonkin traits
        entity = CEntity_Base()
        entity.card_id = "TEST-001"
        entity.card_name_eng = "TestDigimon"
        entity.card_kind = 0  # Digimon
        entity.dp = 5000
        entity.level = 4
        entity.play_cost = 5
        entity.card_colors = [0]
        entity.type_eng = ["Machine"]  # Not Reptile or Dragonkin

        from digimon_gym.engine.core.card_source import CardSource
        cs = CardSource()
        cs.set_base_data(entity, runner.game.player1)
        non_reptile_perm = Permanent([cs])
        runner.game.player1.battle_area.append(non_reptile_perm)

        non_reptile_dp_before = non_reptile_perm.dp

        # Play Elizamon (it will auto-select itself as target since it's Reptile)
        runner.inject_card(1, "EX11-008", "hand")
        runner.set_phase("Main")

        action = runner.find_action("Play Elizamon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # Non-Reptile should NOT have gained anything
        assert not non_reptile_perm.has_keyword("_is_raid"), \
            "Non-Reptile Digimon should not gain Raid"
        assert non_reptile_perm.dp == non_reptile_dp_before, \
            "Non-Reptile Digimon DP should be unchanged"


@pytest.mark.behavioral
class TestEX11008InheritedLoseSecurity:
    """Inherited: [Your Turn] [Once Per Turn] When opponent's security is removed."""

    def test_inherited_fires_on_opponent_security_loss(self, debug_runner):
        """When opponent loses security during your turn, gain 1 memory."""
        runner = debug_runner(deck1=ELIZAMON_DECK, deck2=ELIZAMON_DECK, initial_memory=10)

        # Place a Digimon with Elizamon as inherited source (digivolve stack)
        # We need a Lv.4+ digimon with Elizamon underneath for inherited effect
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()

        # Create Elizamon card source for inheritance
        elizamon_cs = db.create_card_source("EX11-008", runner.game.player1)
        # Create a Lv.4 digimon on top
        # Use another Elizamon as simplification - place two in stack
        top_cs = db.create_card_source("EX11-008", runner.game.player1)

        from digimon_gym.engine.core.permanent import Permanent
        perm = Permanent([elizamon_cs, top_cs])  # bottom=elizamon, top=another
        runner.game.player1.battle_area.append(perm)

        # Ensure opponent has security
        assert len(runner.game.player2.security_cards) > 0, "Opponent needs security"

        memory_before = runner.game.memory

        # Simulate opponent losing security by directly calling the method
        # (during player 1's turn)
        runner.game.player1.is_my_turn = True
        runner.game.player2.is_my_turn = False
        runner.game.turn_player = runner.game.player1
        runner.game.opponent_player = runner.game.player2

        # Remove opponent security
        sec_card = runner.game.player2.security_cards[0]
        runner.game.player2.security_cards.remove(sec_card)
        runner.game.player2.trash_cards.append(sec_card)
        runner.game.player2._fire_timing(
            EffectTiming.OnLoseSecurity,
            {"lost_card": sec_card, "player": runner.game.player2}
        )

        memory_after = runner.game.memory

        # Should have gained 1 memory
        assert memory_after == memory_before + 1, \
            f"Should gain 1 memory when opponent loses security. Before={memory_before}, After={memory_after}"

    def test_inherited_does_not_fire_on_own_security_loss(self, debug_runner):
        """When YOUR OWN security is removed, inherited should NOT fire.
        This validates the opponent check (event_player must be enemy)."""
        runner = debug_runner(deck1=ELIZAMON_DECK, deck2=ELIZAMON_DECK, initial_memory=10)

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()

        # Create Elizamon inherited stack on player 1's field
        elizamon_cs = db.create_card_source("EX11-008", runner.game.player1)
        top_cs = db.create_card_source("EX11-008", runner.game.player1)

        from digimon_gym.engine.core.permanent import Permanent
        perm = Permanent([elizamon_cs, top_cs])
        runner.game.player1.battle_area.append(perm)

        # Ensure player 1 has security
        assert len(runner.game.player1.security_cards) > 0, "Player 1 needs security"

        memory_before = runner.game.memory

        # It's player 1's turn
        runner.game.player1.is_my_turn = True
        runner.game.player2.is_my_turn = False
        runner.game.turn_player = runner.game.player1
        runner.game.opponent_player = runner.game.player2

        # Remove OWN security (player 1 loses security)
        sec_card = runner.game.player1.security_cards[0]
        runner.game.player1.security_cards.remove(sec_card)
        runner.game.player1.trash_cards.append(sec_card)
        runner.game.player1._fire_timing(
            EffectTiming.OnLoseSecurity,
            {"lost_card": sec_card, "player": runner.game.player1}
        )

        memory_after = runner.game.memory

        # Should NOT gain memory (it was own security, not opponent's)
        assert memory_after == memory_before, \
            f"Should NOT gain memory when own security is lost. Before={memory_before}, After={memory_after}"

    def test_inherited_once_per_turn(self, debug_runner):
        """Inherited should only fire once per turn."""
        runner = debug_runner(deck1=ELIZAMON_DECK, deck2=ELIZAMON_DECK, initial_memory=10)

        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()

        elizamon_cs = db.create_card_source("EX11-008", runner.game.player1)
        top_cs = db.create_card_source("EX11-008", runner.game.player1)

        from digimon_gym.engine.core.permanent import Permanent
        perm = Permanent([elizamon_cs, top_cs])
        runner.game.player1.battle_area.append(perm)

        runner.game.player1.is_my_turn = True
        runner.game.player2.is_my_turn = False
        runner.game.turn_player = runner.game.player1
        runner.game.opponent_player = runner.game.player2

        memory_before = runner.game.memory

        # Remove opponent security twice
        for _ in range(2):
            if not runner.game.player2.security_cards:
                break
            sec_card = runner.game.player2.security_cards[0]
            runner.game.player2.security_cards.remove(sec_card)
            runner.game.player2.trash_cards.append(sec_card)
            runner.game.player2._fire_timing(
                EffectTiming.OnLoseSecurity,
                {"lost_card": sec_card, "player": runner.game.player2}
            )

        memory_after = runner.game.memory

        # Should have gained only 1 memory (once per turn)
        assert memory_after == memory_before + 1, \
            f"Should gain only 1 memory (OPT). Before={memory_before}, After={memory_after}"
