"""Behavioral tests for BT24-011 Cyclonemon (Lv.4 Red).

Card text:
  Effect: <Rush> <Raid>
  Inherited Effect: <Raid>

Alt digivolution (from xros_req / C# reference):
  [Digivolve] Lv.3 w/[TS] trait: Cost 2
"""

import pytest


@pytest.mark.behavioral
class TestBT24011Cyclonemon:
    """Tests for BT24-011 Cyclonemon keywords and alt-digi."""

    # ── Rush keyword ───────────────────────────────────────────────

    def test_rush_keyword_present(self, debug_runner):
        """Cyclonemon should have the Rush keyword on the field."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT24-011"], turn_played=-1)
        assert perm.has_keyword('_is_rush'), (
            "Cyclonemon should have Rush keyword"
        )

    def test_rush_allows_same_turn_attack(self, debug_runner):
        """Cyclonemon placed this turn should be able to attack (Rush bypasses
        summoning sickness)."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        current_turn = runner.snapshot().turn
        runner.place_on_field(1, ["BT24-011"], turn_played=current_turn)

        action = runner.find_action("Attack player with Cyclonemon")
        assert action is not None, (
            "Cyclonemon with Rush should attack the turn it enters play"
        )

    # ── Raid keyword (own effect) ──────────────────────────────────

    def test_raid_keyword_present(self, debug_runner):
        """Cyclonemon should have the Raid keyword on the field."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT24-011"], turn_played=-1)
        assert perm.has_keyword('_is_raid'), (
            "Cyclonemon should have Raid keyword"
        )

    # ── Inherited Raid ─────────────────────────────────────────────

    def test_inherited_raid_when_in_sources(self, debug_runner):
        """When Cyclonemon is a digivolution source, the stack should inherit
        Raid."""
        runner = debug_runner(initial_memory=5)
        # BT24-016 Lamiamon is a red Lv.5 (can hold Cyclonemon as source)
        perm = runner.place_on_field(1, ["BT24-011", "BT24-016"], turn_played=-1)
        assert perm.has_keyword('_is_raid'), (
            "A Digimon with Cyclonemon in its sources should have inherited Raid"
        )

    def test_rush_not_inherited(self, debug_runner):
        """Rush is NOT an inherited effect — a stack with Cyclonemon as a
        source should NOT have Rush from this card's inherited effects."""
        runner = debug_runner(initial_memory=5)
        # BT24-016 Lamiamon Lv.5 does not have Rush on its own
        perm = runner.place_on_field(1, ["BT24-011", "BT24-016"], turn_played=-1)
        # Check if rush is present — it should NOT be inherited from Cyclonemon.
        # (The top card BT24-024 might have its own keywords; we check specifically
        # that Cyclonemon's Rush doesn't leak as inherited.)
        effects = perm.card_sources[0].effect_list(None)  # bottom card = Cyclonemon
        rush_inherited = [
            e for e in effects
            if getattr(e, '_is_rush', False) and e.is_inherited_effect
        ]
        assert len(rush_inherited) == 0, (
            "Cyclonemon's Rush should NOT be an inherited effect"
        )

    # ── Alt digivolution (Lv.3 with [TS] trait for cost 2) ────────

    def test_alt_digi_attributes_on_script(self, debug_runner):
        """The script should have alt_digi fields: level=3, trait=TS, cost=2."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT24-011"], turn_played=-1)
        top_card = perm.top_card
        effects = top_card.effect_list(None)

        alt_digi_effects = [
            e for e in effects
            if getattr(e, '_alt_digi_cost', None) is not None
        ]
        assert len(alt_digi_effects) == 1, (
            "Cyclonemon should have exactly 1 alt-digi effect"
        )
        eff = alt_digi_effects[0]
        assert eff._alt_digi_cost == 2, f"Alt-digi cost should be 2, got {eff._alt_digi_cost}"
        assert eff._alt_digi_level == 3, f"Alt-digi level should be 3, got {eff._alt_digi_level}"
        assert eff._alt_digi_trait == "TS", f"Alt-digi trait should be 'TS', got {eff._alt_digi_trait}"

    def test_alt_digi_allows_digivolve_from_ts_lv3(self, debug_runner):
        """Cyclonemon should be able to digivolve onto a Lv.3 Digimon with [TS]
        trait for cost 2 via alt-digi."""
        # Need BT24-011 in the deck so it ends up in hand
        deck = ["BT24-011"] * 4 + ["BT24-009"] * 46
        runner = debug_runner(
            deck1=deck,
            initial_memory=5,
            starting_hand1=["BT24-011"],
        )
        runner.set_phase("Main")

        # BT24-009 Shamanmon is Lv.3 with TS trait (red/purple)
        runner.place_on_field(1, ["BT24-009"], turn_played=-1)

        digivolve_actions = runner.find_actions("Digivolve")
        cyclonemon_digi = {
            aid: desc for aid, desc in digivolve_actions.items()
            if "Cyclonemon" in desc
        }
        assert len(cyclonemon_digi) > 0, (
            f"Should have a Digivolve action for Cyclonemon onto TS Lv.3. "
            f"Available actions: {runner.actions()}"
        )

    def test_alt_digi_blocked_for_non_ts_lv3(self, debug_runner):
        """Cyclonemon should NOT be able to digivolve onto a Lv.3 without [TS]
        trait via alt-digi."""
        runner = debug_runner(initial_memory=5)

        # BT1-009 Monodramon is Lv.3 Red but has NO TS trait
        runner.place_on_field(1, ["BT1-009"], turn_played=-1)

        from digimon_gym.engine.validation.digivolve_validator import _check_alt_digivolve
        from digimon_gym.engine.data.card_database import CardDatabase

        db = CardDatabase()
        cyclonemon_cs = db.create_card_source("BT24-011", runner.game.player1)
        monodramon_perm = runner.game.player1.battle_area[0]

        result = _check_alt_digivolve(cyclonemon_cs, monodramon_perm)
        assert result is False, (
            "Alt-digi should NOT match a Lv.3 without TS trait"
        )
