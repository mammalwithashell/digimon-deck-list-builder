"""Behavioral tests for EX7-027 Chaperomon (Lv.5 Yellow Puppet/LIBERATOR).

Card text:
  <Overclock ([Puppet] Trait)>
  [When Digivolving] You may play 1 level 3 Digimon card with the [Puppet]
      trait from your hand without paying the cost.
  [Inherited][All Turns][Once Per Turn] When this Digimon would leave the
      battle area other than by your effects, by deleting 1 of your Tokens
      or other [Puppet] trait Digimon, prevent it from leaving.
"""

import pytest


@pytest.mark.behavioral
class TestEX7027Chaperomon:
    """Tests for EX7-027 Chaperomon."""

    # ── Effect 1: [When Digivolving] Play Lv3 Puppet from hand free ────

    def test_when_digivolving_plays_lv3_puppet_from_hand(self, debug_runner):
        """Digivolving into Chaperomon should allow playing a Lv3 Puppet from hand free."""
        # Use a minimal deck so the only Lv3 Puppet in hand is BT9-032
        minimal = ["EX7-027"] * 4 + ["BT13-039"] * 4 + ["BT9-032"] * 4 + ["BT2-055"] * 38
        runner = debug_runner(deck1=minimal, initial_memory=10)

        # Place a Yellow Lv4 Puppet on field (BT13-039 KnightChessmon)
        runner.place_on_field(1, ["BT13-039"])
        # Inject Lv3 Puppet into hand (BT9-032 ToyAgumon, Yellow Puppet)
        runner.inject_card(1, "BT9-032", "hand")

        runner.set_phase("Main")

        # Inject EX7-027 Chaperomon into hand and digivolve onto Yellow Lv4
        runner.inject_card(1, "EX7-027", "hand")
        digivolve = runner.find_action("Digivolve")
        assert digivolve is not None, "Should be able to digivolve into Chaperomon"
        runner.execute(digivolve)

        # Auto-resolve the When Digivolving trigger (play Lv3 Puppet)
        snap_before = runner.snapshot()
        field_count_before = len(snap_before.p1_field)
        results = runner.auto_resolve()

        snap = runner.snapshot()
        # A new Digimon should have been played from the WD trigger
        assert len(snap.p1_field) > field_count_before, (
            "A Lv3 Puppet should be played to the field via When Digivolving effect"
        )

    def test_when_digivolving_does_not_play_non_puppet(self, debug_runner):
        """Non-Puppet Digimon in hand should not be playable via this effect."""
        runner = debug_runner(initial_memory=10)

        runner.place_on_field(1, ["BT13-039"])  # Yellow Lv4 Puppet
        # Inject a non-Puppet Lv3 Digimon into hand
        runner.inject_card(1, "BT1-009", "hand")  # Agumon, no Puppet trait
        runner.inject_card(1, "EX7-027", "hand")

        runner.set_phase("Main")

        digivolve = runner.find_action("Digivolve")
        assert digivolve is not None
        runner.execute(digivolve)
        results = runner.auto_resolve()

        snap = runner.snapshot()
        agumon_on_field = any(
            s.card_id == "BT1-009" for s in snap.p1_field
        )
        assert not agumon_on_field, "Non-Puppet Digimon should NOT be played by this effect"

    # ── Effect 2: Inherited prevent-leaving ─────────────────────────────

    def test_inherited_prevents_leaving_by_deleting_puppet(self, debug_runner):
        """When the Digimon with Chaperomon inherited would be deleted by opponent's
        effect, deleting another Puppet should prevent it from leaving."""
        runner = debug_runner(initial_memory=10)

        # Stack: EX7-027 (bottom, inherited) + AD1-016 (top, Lv6)
        perm = runner.place_on_field(1, ["EX7-027", "AD1-016"])
        # Place another Puppet Digimon as substitute
        runner.place_on_field(1, ["BT6-084"])  # Sistermon Ciel (Lv4 Puppet)

        p1 = runner.game.player1

        # Delete by opponent effect
        result = p1.delete_permanent(perm, is_opponent_effect=True)

        # The effect should trigger and set _will_not_be_removed optimistically;
        # auto-resolve should select the substitute to delete
        runner.auto_resolve()

        snap = runner.snapshot()
        # The stack (with AD1-016 on top) should still be on field
        stack_on_field = any(
            s.card_id == "AD1-016" for s in snap.p1_field
        )
        assert stack_on_field, (
            "Digimon with Chaperomon inherited should still be on field after "
            "opponent tried to delete it and a Puppet substitute was available"
        )

    def test_inherited_does_not_prevent_leaving_by_own_effect(self, debug_runner):
        """When the Digimon would leave by YOUR OWN effects, the inherited
        should NOT trigger (card text: 'other than by your effects')."""
        runner = debug_runner(initial_memory=10)

        # Stack: EX7-027 (inherited) + AD1-016 (top)
        perm = runner.place_on_field(1, ["EX7-027", "AD1-016"])
        runner.place_on_field(1, ["BT6-084"])  # substitute available

        p1 = runner.game.player1

        # Delete by own effect (is_opponent_effect=False, is_own_effect=True)
        result = p1.delete_permanent(perm, is_opponent_effect=False)

        snap = runner.snapshot()
        # The stack should be deleted (own effect, not prevented)
        stack_on_field = any(
            s.card_id == "AD1-016" for s in snap.p1_field
        )
        assert not stack_on_field, (
            "Digimon should be deleted when removed by own effect"
        )

    def test_inherited_prevents_leaving_by_deleting_token(self, debug_runner):
        """A Token should also be a valid substitute for preventing leaving."""
        runner = debug_runner(initial_memory=10)

        # Stack: EX7-027 (inherited) + AD1-016 (top)
        perm = runner.place_on_field(1, ["EX7-027", "AD1-016"])

        # Create a token on our field
        p1 = runner.game.player1
        runner.game.effect_play_token(p1, "diaboromon")

        field_before = len(p1.battle_area)
        assert field_before == 2, "Should have stack + token"

        # Delete by opponent effect
        result = p1.delete_permanent(perm, is_opponent_effect=True)
        runner.auto_resolve()

        snap = runner.snapshot()
        # The stack should still be on field
        stack_on_field = any(
            s.card_id == "AD1-016" for s in snap.p1_field
        )
        assert stack_on_field, (
            "Digimon should still be on field after deleting a token as substitute"
        )

    def test_inherited_once_per_turn(self, debug_runner):
        """The inherited effect should only activate once per turn."""
        runner = debug_runner(initial_memory=10)

        # Stack: EX7-027 (inherited) + AD1-016 (top)
        perm = runner.place_on_field(1, ["EX7-027", "AD1-016"])
        runner.place_on_field(1, ["BT6-084"])  # substitute 1
        runner.place_on_field(1, ["BT2-055"])  # substitute 2 (Lv3 Puppet)

        p1 = runner.game.player1

        # First deletion attempt by opponent - should be prevented
        result = p1.delete_permanent(perm, is_opponent_effect=True)
        runner.auto_resolve()

        snap = runner.snapshot()
        stack_on_field = any(
            s.card_id == "AD1-016" for s in snap.p1_field
        )
        assert stack_on_field, "First deletion should be prevented"

        # Second deletion attempt by opponent - once per turn expired
        result = p1.delete_permanent(perm, is_opponent_effect=True)
        runner.auto_resolve()

        snap = runner.snapshot()
        stack_on_field = any(
            s.card_id == "AD1-016" for s in snap.p1_field
        )
        assert not stack_on_field, (
            "Second deletion should NOT be prevented (once per turn)"
        )

    def test_inherited_no_substitute_available(self, debug_runner):
        """If no Token or other Puppet is available, the effect should not trigger."""
        runner = debug_runner(initial_memory=10)

        # Stack: EX7-027 (inherited) + AD1-016 (top)
        perm = runner.place_on_field(1, ["EX7-027", "AD1-016"])
        # No other Puppet or Token on field

        p1 = runner.game.player1

        result = p1.delete_permanent(perm, is_opponent_effect=True)
        runner.auto_resolve()

        snap = runner.snapshot()
        stack_on_field = any(
            s.card_id == "AD1-016" for s in snap.p1_field
        )
        assert not stack_on_field, (
            "Digimon should be deleted when no substitute is available"
        )

    def test_inherited_effect_is_optional(self, debug_runner):
        """The effect is optional (is_optional=True on the ICardEffect).
        When the effect activates, the selection of substitute is mandatory.
        The optionality is at the effect level: the engine sorts optional effects
        after mandatory ones and the player can choose not to activate it.
        Once activated, the substitute selection is mandatory."""
        runner = debug_runner(initial_memory=10)

        # Stack: EX7-027 (inherited) + AD1-016 (top)
        perm = runner.place_on_field(1, ["EX7-027", "AD1-016"])
        runner.place_on_field(1, ["BT6-084"])  # substitute available

        p1 = runner.game.player1

        # Verify the inherited effect has is_optional=True
        from engine_py_legacy.engine.data.enums import EffectTiming
        effects = perm.effect_list(EffectTiming.WhenPermanentWouldBeDeleted)
        prevent_effects = [e for e in effects if "Prevent" in (e.effect_name or "")
                          or "prevent" in (e.effect_description or "").lower()]
        assert len(prevent_effects) >= 1, "Should have inherited prevent-leaving effect"
        assert prevent_effects[0].is_optional, "The prevent-leaving effect should be optional"
