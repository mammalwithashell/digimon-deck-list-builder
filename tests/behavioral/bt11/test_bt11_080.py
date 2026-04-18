"""Behavioral tests for BT11-080 Devimon (Lv.4 Purple Fallen Angel, DP 5000, Cost 5).

Card text:
  [Your Turn] While you have a yellow Digimon or yellow Tamer in play,
  this Digimon gains <Rush> (This Digimon can attack the turn it comes into
  play.) and <Retaliation> (When this Digimon is deleted after losing a
  battle, delete the Digimon it was battling.)

Key faithfulness points tested:
  - Rush and Retaliation BOTH require "[Your Turn]" gate (owner's turn only).
  - Both keywords require a yellow Digimon OR yellow Tamer on the owner's
    battle area.
  - Effects fire from the top card (not inherited). The card has no
    inherited effect text.
  - Rush actually enables attacking the turn it was played (integration via
    has_keyword('_is_rush')).
  - Retaliation actually triggers when Devimon loses a battle, deleting the
    winning opponent Digimon.
  - Without any yellow permanent, neither keyword is granted.
  - During opponent's turn, neither keyword is active (Rush doesn't matter
    there but Retaliation is still gated by [Your Turn]).
  - Effects must be gone when Devimon leaves the field (condition uses
    permanent_of_this_card()).
"""

import pytest

from digimon_gym.engine.data.enums import EffectTiming

# ── Card IDs under test ─────────────────────────────────────────────────

BT11_080 = "BT11-080"       # Devimon — Lv.4 Purple, DP 5000, Cost 5

# Yellow digimon/tamer to satisfy condition
PATAMON_Y = "BT1-048"       # Patamon — Lv.3 Yellow Digimon, DP 2000
TAI_Y = "BT4-094"           # Tai Kamiya — Lv.3 Yellow Tamer

# Non-yellow fillers
AGUMON_RED = "ST1-03"       # Agumon — Lv.3 Red, DP 2000 (non-yellow filler)

# Battle target — higher DP than Devimon so Devimon LOSES and Retaliation
# fires when the opponent attacks Devimon.
COREDRAMON = "ST1-06"       # Coredramon — Lv.4 Red, DP 6000


FILLER_DECK = [AGUMON_RED] * 50


@pytest.mark.behavioral
class TestBT11080DevimonConditionalKeywords:
    """BT11-080 grants Rush + Retaliation only while a yellow Digimon/Tamer is in play, on owner's turn."""

    # ── Rush — conditional grant ────────────────────────────────────────

    def test_rush_active_when_yellow_digimon_in_play(self, debug_runner):
        """Rush must be active while a yellow Digimon is on the owner's field
        and it's the owner's turn."""
        runner = debug_runner(
            deck1=[BT11_080] * 4 + [PATAMON_Y] * 4 + [AGUMON_RED] * 42,
            deck2=FILLER_DECK,
            initial_memory=10,
        )
        runner.set_phase("Main")

        # Place a yellow Digimon first
        runner.place_on_field(1, [PATAMON_Y])
        # Place Devimon — simulate being played this turn (summoning sickness)
        perm = runner.place_on_field(1, [BT11_080], turn_played=runner.game.turn_count)

        assert runner.game.player1.is_my_turn, "Must be P1's turn"
        assert perm.has_keyword('_is_rush'), (
            "Devimon should have Rush while yellow Digimon in play on P1 turn"
        )

    def test_rush_active_when_yellow_tamer_in_play(self, debug_runner):
        """Rush must be active while a yellow Tamer is on the owner's field."""
        runner = debug_runner(
            deck1=[BT11_080] * 4 + [TAI_Y] * 4 + [AGUMON_RED] * 42,
            deck2=FILLER_DECK,
            initial_memory=10,
        )
        runner.set_phase("Main")

        # Place a yellow Tamer
        runner.place_on_field(1, [TAI_Y])
        perm = runner.place_on_field(1, [BT11_080], turn_played=runner.game.turn_count)

        assert perm.has_keyword('_is_rush'), (
            "Devimon should have Rush while yellow Tamer in play on P1 turn"
        )

    def test_rush_inactive_without_any_yellow_permanent(self, debug_runner):
        """Rush must NOT be active when no yellow Digimon/Tamer is in play."""
        runner = debug_runner(
            deck1=[BT11_080] * 4 + [AGUMON_RED] * 46,
            deck2=FILLER_DECK,
            initial_memory=10,
        )
        runner.set_phase("Main")

        # No yellow anywhere
        perm = runner.place_on_field(1, [BT11_080], turn_played=runner.game.turn_count)
        assert not perm.has_keyword('_is_rush'), (
            "Devimon should NOT have Rush with no yellow permanent in play"
        )

    def test_rush_inactive_with_only_non_yellow_digimon(self, debug_runner):
        """Having a red Digimon on the field must not satisfy the condition."""
        runner = debug_runner(
            deck1=[BT11_080] * 4 + [AGUMON_RED] * 46,
            deck2=FILLER_DECK,
            initial_memory=10,
        )
        runner.set_phase("Main")

        runner.place_on_field(1, [AGUMON_RED])  # Red Digimon
        perm = runner.place_on_field(1, [BT11_080], turn_played=runner.game.turn_count)

        assert not perm.has_keyword('_is_rush'), (
            "Red Digimon in play must NOT satisfy yellow requirement"
        )

    def test_rush_inactive_during_opponent_turn(self, debug_runner):
        """Rush must NOT be active during opponent's turn (gated by [Your Turn])."""
        runner = debug_runner(
            deck1=[BT11_080] * 4 + [PATAMON_Y] * 4 + [AGUMON_RED] * 42,
            deck2=FILLER_DECK,
            initial_memory=10,
        )
        runner.set_phase("Main")

        runner.place_on_field(1, [PATAMON_Y])
        perm = runner.place_on_field(1, [BT11_080], turn_played=runner.game.turn_count)

        # Flip to opponent's turn
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True

        assert not perm.has_keyword('_is_rush'), (
            "Rush must not be active during opponent's turn — [Your Turn] gate"
        )

    def test_rush_yellow_on_opponent_side_does_not_count(self, debug_runner):
        """A yellow Digimon on the OPPONENT's field must not satisfy the condition."""
        runner = debug_runner(
            deck1=[BT11_080] * 4 + [AGUMON_RED] * 46,
            deck2=[PATAMON_Y] * 4 + FILLER_DECK[:46],
            initial_memory=10,
        )
        runner.set_phase("Main")

        # Yellow Patamon on opponent's side
        runner.place_on_field(2, [PATAMON_Y])
        perm = runner.place_on_field(1, [BT11_080], turn_played=runner.game.turn_count)

        assert not perm.has_keyword('_is_rush'), (
            "Yellow permanent on opponent's side must NOT satisfy the condition"
        )

    # ── Retaliation — conditional grant ─────────────────────────────────

    def test_retaliation_active_when_yellow_digimon_in_play(self, debug_runner):
        """Retaliation must be active while a yellow Digimon is on the owner's field."""
        runner = debug_runner(
            deck1=[BT11_080] * 4 + [PATAMON_Y] * 4 + [AGUMON_RED] * 42,
            deck2=FILLER_DECK,
            initial_memory=10,
        )
        runner.set_phase("Main")

        runner.place_on_field(1, [PATAMON_Y])
        perm = runner.place_on_field(1, [BT11_080])

        assert perm.has_keyword('_is_retaliation'), (
            "Devimon should have Retaliation while yellow Digimon in play on P1 turn"
        )

    def test_retaliation_inactive_without_yellow(self, debug_runner):
        """Retaliation must NOT be active without any yellow in play."""
        runner = debug_runner(
            deck1=[BT11_080] * 4 + [AGUMON_RED] * 46,
            deck2=FILLER_DECK,
            initial_memory=10,
        )
        runner.set_phase("Main")

        perm = runner.place_on_field(1, [BT11_080])
        assert not perm.has_keyword('_is_retaliation'), (
            "Devimon should NOT have Retaliation without yellow in play"
        )

    def test_retaliation_inactive_during_opponent_turn(self, debug_runner):
        """[Your Turn] gate: Retaliation is not granted during opponent's turn."""
        runner = debug_runner(
            deck1=[BT11_080] * 4 + [PATAMON_Y] * 4 + [AGUMON_RED] * 42,
            deck2=FILLER_DECK,
            initial_memory=10,
        )
        runner.set_phase("Main")

        runner.place_on_field(1, [PATAMON_Y])
        perm = runner.place_on_field(1, [BT11_080])

        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True

        assert not perm.has_keyword('_is_retaliation'), (
            "Retaliation must not be active during opponent's turn"
        )

    # ── Structural checks ───────────────────────────────────────────────

    def test_card_has_exactly_one_rush_and_one_retaliation_effect(self, debug_runner):
        """Card should expose exactly one Rush effect and one Retaliation effect."""
        runner = debug_runner(
            deck1=[BT11_080] * 4 + [AGUMON_RED] * 46,
            deck2=FILLER_DECK,
            initial_memory=5,
        )
        perm = runner.place_on_field(1, [BT11_080])
        effects = perm.top_card.effect_list(None)

        rush_effects = [e for e in effects if getattr(e, '_is_rush', False)]
        retal_effects = [e for e in effects if getattr(e, '_is_retaliation', False)]

        assert len(rush_effects) == 1, f"Expected 1 Rush effect, got {len(rush_effects)}"
        assert len(retal_effects) == 1, f"Expected 1 Retaliation effect, got {len(retal_effects)}"

    def test_effects_are_not_inherited(self, debug_runner):
        """BT11-080 has no inherited effect text — Rush/Retaliation should fire
        from the top card, not as inherited effects."""
        runner = debug_runner(
            deck1=[BT11_080] * 4 + [AGUMON_RED] * 46,
            deck2=FILLER_DECK,
            initial_memory=5,
        )
        perm = runner.place_on_field(1, [BT11_080])
        effects = perm.top_card.effect_list(None)

        for e in effects:
            assert not getattr(e, 'is_inherited_effect', False), (
                f"BT11-080 has no inherited text — effect {e.effect_name} "
                "should not be flagged as inherited"
            )

    def test_condition_requires_field_presence(self, debug_runner):
        """Effects require the source card to be on the field (field presence check).
        If Devimon's permanent doesn't exist, condition should fail."""
        runner = debug_runner(
            deck1=[BT11_080] * 4 + [AGUMON_RED] * 46,
            deck2=FILLER_DECK,
            initial_memory=5,
        )

        # Create a card source but do not place on field
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(BT11_080, runner.game.player1)
        # Put a yellow permanent on field to satisfy the yellow condition
        runner.place_on_field(1, [PATAMON_Y])

        effects = cs.effect_list(None)
        rush = [e for e in effects if getattr(e, '_is_rush', False)][0]

        # Condition must fail because cs is not on battle area
        assert not rush.can_use_condition({}), (
            "Rush condition must fail when source card is not on field"
        )


@pytest.mark.behavioral
class TestBT11080RushIntegration:
    """Integration: Rush actually lets Devimon attack the turn it is played."""

    def test_rush_enables_attack_on_play_turn(self, debug_runner):
        """With Rush active (yellow Patamon in play), Devimon should be able
        to attack the turn it was played."""
        runner = debug_runner(
            deck1=[BT11_080] * 4 + [PATAMON_Y] * 4 + [AGUMON_RED] * 42,
            deck2=FILLER_DECK,
            initial_memory=10,
        )
        runner.set_phase("Main")

        # Yellow Patamon first
        runner.place_on_field(1, [PATAMON_Y])
        # Give opponent security so an attack can resolve
        for _ in range(3):
            runner.inject_card(2, AGUMON_RED, "security_top")

        # Place Devimon "this turn" (simulating just being played)
        devimon = runner.place_on_field(
            1, [BT11_080], turn_played=runner.game.turn_count
        )

        assert devimon.has_keyword('_is_rush'), "Devimon must have Rush"
        # can_attack should succeed because of Rush
        assert devimon.can_attack(), (
            "Devimon should be able to attack the turn it was played (Rush active)"
        )

    def test_rush_not_enabled_without_yellow(self, debug_runner):
        """Without a yellow Digimon/Tamer, Devimon cannot attack the turn
        it was played (no Rush, hit with summoning sickness)."""
        runner = debug_runner(
            deck1=[BT11_080] * 4 + [AGUMON_RED] * 46,
            deck2=FILLER_DECK,
            initial_memory=10,
        )
        runner.set_phase("Main")

        for _ in range(3):
            runner.inject_card(2, AGUMON_RED, "security_top")

        devimon = runner.place_on_field(
            1, [BT11_080], turn_played=runner.game.turn_count
        )

        assert not devimon.has_keyword('_is_rush'), (
            "Devimon should NOT have Rush without yellow in play"
        )
        assert not devimon.can_attack(), (
            "Devimon must have summoning sickness without Rush on the turn it was played"
        )


@pytest.mark.behavioral
class TestBT11080RetaliationIntegration:
    """Integration: Retaliation actually triggers on battle loss when condition is met."""

    def test_retaliation_kills_attacker_when_condition_met(self, debug_runner):
        """When Devimon loses a battle and Retaliation is active, the opposing
        Digimon that attacked/was attacked should also be deleted."""
        runner = debug_runner(
            deck1=[BT11_080] * 4 + [PATAMON_Y] * 4 + [AGUMON_RED] * 42,
            deck2=[COREDRAMON] * 4 + FILLER_DECK[:46],
            initial_memory=10,
        )
        runner.set_phase("Main")

        # P1: yellow Patamon to satisfy the condition, plus Devimon
        runner.place_on_field(1, [PATAMON_Y])
        devimon = runner.place_on_field(1, [BT11_080])

        # P2: Coredramon (Lv.4, 6000 DP > Devimon 5000 DP) — suspended so
        # Devimon can attack it.
        coredramon = runner.place_on_field(
            2, [COREDRAMON], is_suspended=True
        )

        # Sanity: Devimon has Retaliation
        assert devimon.has_keyword('_is_retaliation'), (
            "Devimon should have Retaliation (yellow Patamon in play, P1 turn)"
        )

        # Attack Coredramon — Devimon (5000) vs Coredramon (6000): Devimon
        # loses, Retaliation kills Coredramon. Use direct _resolve_battle
        # to skip counter/block timing for unit-test isolation.
        from digimon_gym.engine.game.constants import PendingAttack
        devimon.is_attacking = True
        runner.game.pending_attack = PendingAttack(
            attacker=devimon,
            original_target=coredramon,
            effective_target=coredramon,
        )
        runner.game._resolve_battle()

        # Devimon should be deleted (lost battle)
        assert devimon not in runner.game.player1.battle_area, (
            "Devimon should be deleted after losing the battle"
        )
        # Coredramon should ALSO be deleted from Retaliation
        assert coredramon not in runner.game.player2.battle_area, (
            "Coredramon should be deleted by Retaliation"
        )

    def test_no_retaliation_without_yellow(self, debug_runner):
        """Without a yellow permanent, Retaliation is not active — Devimon
        loses the battle alone."""
        runner = debug_runner(
            deck1=[BT11_080] * 4 + [AGUMON_RED] * 46,
            deck2=[COREDRAMON] * 4 + FILLER_DECK[:46],
            initial_memory=10,
        )
        runner.set_phase("Main")

        devimon = runner.place_on_field(1, [BT11_080])
        coredramon = runner.place_on_field(
            2, [COREDRAMON], is_suspended=True
        )

        assert not devimon.has_keyword('_is_retaliation'), (
            "No yellow in play — Devimon should not have Retaliation"
        )

        from digimon_gym.engine.game.constants import PendingAttack
        devimon.is_attacking = True
        runner.game.pending_attack = PendingAttack(
            attacker=devimon,
            original_target=coredramon,
            effective_target=coredramon,
        )
        runner.game._resolve_battle()

        # Devimon lost and was deleted
        assert devimon not in runner.game.player1.battle_area, (
            "Devimon should be deleted after losing"
        )
        # Coredramon should SURVIVE (no Retaliation)
        assert coredramon in runner.game.player2.battle_area, (
            "Without Retaliation, Coredramon should survive"
        )
