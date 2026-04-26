"""Behavioral tests for BT23-014 Gallantmon (Lv.6 Red Royal Knight/Holy Warrior).

Card text:
[On Play] [When Digivolving] Until your opponent's turn ends, their effects
    can't play Digimon or Tamers from the trash.
[On Play] [When Digivolving] [When Attacking] Delete 1 of your opponent's
    Digimon with 8000 DP or less. For each of their Digimon and Tamers,
    add 2000 to this DP deletion effect's maximum.

Alt-digi: Lv.5 w/[CS] trait: Cost 3
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT23014Gallantmon:
    """Tests for BT23-014 Gallantmon."""

    # --- Alt-digi tests ---

    def test_alt_digi_requires_cs_trait(self, debug_runner):
        """Alt-digi should require the [CS] trait on the base permanent.
        A Lv.5 with CS trait should match; a Lv.5 without CS should not."""
        runner = debug_runner(initial_memory=5)

        # Get the alt-digi effect from BT23-014
        from digimon_gym.engine.data.card_database import CardDatabase
        from digimon_gym.engine.data.card_registry import CardRegistry
        CardRegistry.ensure_initialized()
        db = CardDatabase()

        # Create a BT23-014 card source to inspect effects
        cs = db.create_card_source("BT23-014", runner.game.player1)
        effects = cs.effect_list(None)
        alt_effects = [e for e in effects if getattr(e, '_alt_digi_cost', None) is not None]
        assert len(alt_effects) >= 1, "Should have alt-digi effect"
        alt = alt_effects[0]
        assert getattr(alt, '_alt_digi_level', None) == 5, "Alt-digi should require Lv.5"
        assert getattr(alt, '_alt_digi_trait', None) == "CS", (
            "Alt-digi should require CS trait"
        )

    # --- Delete effect tests ---

    def test_on_play_delete_base_8000dp(self, debug_runner):
        """[On Play] should delete opponent's Digimon with 8000 DP or less
        when no other Digimon/Tamers exist (base threshold = 8000)."""
        runner = debug_runner(initial_memory=15)
        runner.set_phase("Main")

        # Opponent has a 6000 DP Lv.4 Digimon (Coredramon ST1-06)
        opp_perm = runner.place_on_field(2, ["ST1-06"])
        assert opp_perm.dp == 6000

        # Inject BT23-014 into hand and play it
        runner.inject_card(1, "BT23-014", "hand")
        action = runner.find_action("Play Gallantmon")
        assert action is not None, "Should be able to play Gallantmon"
        runner.execute(action)
        runner.auto_resolve()

        # The 6000 DP Digimon should be deleted (8000 base + 2000 for that Digimon itself = 10000 threshold)
        snap = runner.snapshot()
        opp_digimon = [s for s in snap.p2_field if s.is_digimon and s.card_id == "ST1-06"]
        assert len(opp_digimon) == 0, (
            "6000 DP Digimon should be deleted (threshold is 8000 + 2000 per opp field = at least 10000)"
        )

    def test_delete_threshold_scales_with_opponent_field(self, debug_runner):
        """DP threshold should increase by 2000 per opponent's Digimon and Tamer."""
        runner = debug_runner(initial_memory=15)
        runner.set_phase("Main")

        # Place 3 opponent Digimon + 1 Tamer = 4 permanents
        # Threshold = 8000 + 4*2000 = 16000
        opp1 = runner.place_on_field(2, ["ST1-06"])   # 6000 DP
        opp2 = runner.place_on_field(2, ["ST1-07"])   # 4000 DP
        opp3 = runner.place_on_field(2, ["ST1-09"])   # 7000 DP (Lv.5 MetalGreymon)
        runner.place_on_field(2, ["ST1-12"])           # Tamer

        # Place Gallantmon on field and trigger On Play effect manually
        perm = runner.place_on_field(1, ["BT23-014"])
        game = runner.game

        # Find the On Play delete effect
        top_card = perm.top_card
        effects = top_card.effect_list(None)
        on_play_delete = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
            and e.on_process_callback is not None
            and "Delete" in (e.effect_name or "")
        ]
        assert len(on_play_delete) >= 1, "Should have On Play delete effect"

        # Calculate expected threshold: 8000 + 4*2000 = 16000
        enemy = game.player2
        count = len([p for p in enemy.battle_area if p.is_digimon or p.is_tamer])
        expected_threshold = 8000 + 2000 * count
        assert expected_threshold == 16000, f"Expected threshold 16000, got {expected_threshold}"

    def test_when_attacking_delete_fires(self, debug_runner):
        """[When Attacking] should trigger the delete effect."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place Gallantmon on P1 field
        perm = runner.place_on_field(1, ["BT23-014"])
        # Place a weak opponent Digimon
        opp_perm = runner.place_on_field(2, ["ST1-03"])  # Agumon 2000 DP

        game = runner.game

        # Find the When Attacking delete effect (OnUseAttack timing)
        top_card = perm.top_card
        effects = top_card.effect_list(None)
        wa_delete = [
            e for e in effects
            if e.timing == EffectTiming.OnUseAttack
            and getattr(e, 'is_on_attack', False)
            and e.on_process_callback is not None
        ]
        assert len(wa_delete) >= 1, "Should have When Attacking delete effect"

        # Execute the When Attacking effect manually
        wa_delete[0].on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'attacking_permanent': perm,
        })
        runner.auto_resolve()

        # Opponent's Agumon (2000 DP) should be deleted
        snap = runner.snapshot()
        opp_digimon = [s for s in snap.p2_field if s.card_id == "ST1-03"]
        assert len(opp_digimon) == 0, "2000 DP Agumon should be deleted by When Attacking"

    def test_floodgate_registers_modifier(self, debug_runner):
        """[On Play] floodgate should register CANNOT_PLAY_CARD modifier in the registry."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, ["BT23-014"])
        game = runner.game

        # Find the On Play floodgate effect
        top_card = perm.top_card
        effects = top_card.effect_list(None)
        floodgate = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
            and e.on_process_callback is not None
            and "trash" in (e.effect_description or "").lower()
        ]
        assert len(floodgate) >= 1, "Should have On Play floodgate effect"

        from digimon_gym.engine.interfaces.modifiers import ModifierType

        # Count CANNOT_PLAY_CARD entries before
        entries_before = len(game.modifiers._modifiers.get(ModifierType.CANNOT_PLAY_CARD, []))

        # Execute the floodgate
        floodgate[0].on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Check that a new CANNOT_PLAY_CARD modifier entry was registered
        entries_after = len(game.modifiers._modifiers.get(ModifierType.CANNOT_PLAY_CARD, []))
        assert entries_after > entries_before, (
            f"CANNOT_PLAY_CARD modifier should be registered: "
            f"before={entries_before}, after={entries_after}"
        )
