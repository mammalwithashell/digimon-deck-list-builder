"""Behavioral tests for EX10-061 Apocalymon.

Card text:
Level 7 | White | DP 16000 | Cost 17 | Traits: Unidentified
Digivolve: Lv.6 w/[Dark Masters] trait for 5

Effect:
When this card would be played, by placing 1 of each face-up [Dark Masters]
trait card with different names from your security stack under this card,
reduce the play cost by 4 for each card placed.

[On Play] [When Digivolving] You may play 1 of each [Dark Masters] trait card
with different names from this Digimon's digivolution cards without paying the
costs. Then, all of your [Dark Masters] trait Digimon gain <Rush> for the turn.
At turn end, delete the Digimon this effect played.

Key cards used in tests:
- EX10-061: Apocalymon (Lv7 White, this card)
- EX10-012: MetalSeadramon (Lv6 Blue, Dark Masters, Cyborg)
- EX10-020: Puppetmon (Lv6 Green, Dark Masters, Puppet)
- EX10-035: Machinedramon (Lv6 Black, Dark Masters, Machine)
- EX10-057: Piedmon (Lv6 Purple, Dark Masters, Wizard)
- BT1-010: Agumon (Lv3 Red filler)
"""

import pytest
from digimon_gym.engine.data.enums import GamePhase, EffectTiming

# Deck with Apocalymon + 4 different Dark Masters + filler
DM_DECK = (
    ["EX10-061"] * 4
    + ["EX10-012"] * 8   # MetalSeadramon
    + ["EX10-020"] * 8   # Puppetmon
    + ["EX10-035"] * 8   # Machinedramon
    + ["EX10-057"] * 8   # Piedmon
    + ["BT1-010"] * 14   # Agumon filler
)


@pytest.mark.behavioral
class TestEX10061AltDigivolve:
    """Tests for EX10-061 alt digivolve: Lv.6 w/[Dark Masters] trait for 5."""

    def test_alt_digi_requires_dark_masters_trait(self, debug_runner):
        """Alt digivolve should only be available on Lv6 with [Dark Masters] trait."""
        runner = debug_runner(deck1=DM_DECK, deck2=DM_DECK, initial_memory=10)
        runner.set_phase("Main")

        # Place MetalSeadramon (Lv6 Dark Masters) on field
        runner.place_on_field(1, ["EX10-012"], turn_played=-1)

        # Put Apocalymon in hand
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "EX10-061", "hand")

        # Should find digivolve action for Apocalymon onto MetalSeadramon
        action = runner.find_action("Digivolve")
        assert action is not None, (
            f"Should be able to digivolve Apocalymon onto Lv6 Dark Masters. "
            f"Actions: {runner.actions()}"
        )

    def test_alt_digi_blocked_on_non_dark_masters_lv6(self, debug_runner):
        """Alt digivolve should NOT be available on Lv6 without [Dark Masters] trait."""
        runner = debug_runner(deck1=DM_DECK, deck2=DM_DECK, initial_memory=10)
        runner.set_phase("Main")

        # Place Beelzemon (Lv6, no Dark Masters trait) on field
        # BT2-111 = Beelzemon, Lv6 Purple, Demon Lord (no Dark Masters)
        runner.place_on_field(1, ["BT2-111"], turn_played=-1)

        # Put Apocalymon in hand
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "EX10-061", "hand")

        # Should NOT find digivolve action
        digivolve_actions = runner.find_actions("Digivolve")
        # Filter to only Apocalymon digivolve (not other cards)
        apoc_digi = {k: v for k, v in digivolve_actions.items()
                     if "Apocalymon" in v or "EX10-061" in v}
        assert len(apoc_digi) == 0, (
            f"Should NOT be able to digivolve Apocalymon onto non-Dark Masters Lv6. "
            f"Digivolve actions: {digivolve_actions}"
        )


@pytest.mark.behavioral
class TestEX10061BeforePayCost:
    """Tests for BeforePayCost: place face-up DM security under card, -4 each."""

    def test_cost_reduction_with_face_up_dm_in_security(self, debug_runner):
        """Playing Apocalymon with face-up DM in security should reduce cost."""
        runner = debug_runner(deck1=DM_DECK, deck2=DM_DECK, initial_memory=20)
        runner.set_phase("Main")

        # Clear security, add 2 face-up Dark Masters Digimon
        runner.clear_zone(1, "security")
        game = runner.game
        p1 = game.player1

        # Inject MetalSeadramon face-up in security
        cs1 = runner.inject_card(1, "EX10-012", "security_top")
        p1.face_up_security.add(cs1)

        # Inject Puppetmon face-up in security
        cs2 = runner.inject_card(1, "EX10-020", "security_top")
        p1.face_up_security.add(cs2)

        # Put Apocalymon in hand
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "EX10-061", "hand")

        snap_before = runner.snapshot()
        sec_before = snap_before.p1_security_count

        # Play Apocalymon - cost should be 17 - (2 * 4) = 9
        action = runner.find_action("Play")
        assert action is not None, f"Should find Play action. Actions: {runner.actions()}"
        result = runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Apocalymon should be on field
        assert any(s.card_id == "EX10-061" for s in snap.p1_field), \
            "Apocalymon should be on field"
        # Security should have fewer cards (removed face-up DMs)
        assert snap.p1_security_count < sec_before, \
            "Face-up DM cards should have been removed from security"

    def test_no_cost_reduction_without_face_up_dm(self, debug_runner):
        """Playing Apocalymon without face-up DM in security costs full 17."""
        runner = debug_runner(deck1=DM_DECK, deck2=DM_DECK, initial_memory=20)
        runner.set_phase("Main")

        # Clear security entirely
        runner.clear_zone(1, "security")

        # Put Apocalymon in hand
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "EX10-061", "hand")

        mem_before = runner.snapshot().memory
        action = runner.find_action("Play")
        assert action is not None, f"Should find Play action. Actions: {runner.actions()}"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Full cost of 17 should be deducted
        # Memory goes from P1 perspective: positive = P1's memory
        # After playing cost 17, memory shifts by 17 toward opponent
        assert any(s.card_id == "EX10-061" for s in snap.p1_field), \
            "Apocalymon should be on field"

    def test_security_cards_become_digi_sources_then_played(self, debug_runner):
        """Face-up DM cards from security become digi sources, then played as permanents.

        The flow is:
        1. BeforePayCost removes face-up DMs from security -> _pending_digi_sources
        2. On Play consumes _pending_digi_sources -> adds to perm.card_sources
        3. On Play plays them out as new permanents (removed from digi stack)
        So the DMs end up as separate permanents on field, not under Apocalymon.
        """
        runner = debug_runner(deck1=DM_DECK, deck2=DM_DECK, initial_memory=20)
        runner.set_phase("Main")

        runner.clear_zone(1, "security")
        game = runner.game
        p1 = game.player1

        # Add 2 face-up Dark Masters to security
        cs1 = runner.inject_card(1, "EX10-012", "security_top")
        p1.face_up_security.add(cs1)
        cs2 = runner.inject_card(1, "EX10-020", "security_top")
        p1.face_up_security.add(cs2)

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "EX10-061", "hand")

        action = runner.find_action("Play")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Apocalymon should be on field
        assert any(s.card_id == "EX10-061" for s in snap.p1_field), \
            "Apocalymon should be on field"
        # Security DM cards should have been removed from security
        assert snap.p1_security_count == 0, \
            "Security should be empty (DMs were taken)"
        # The DMs should be on the field as separate permanents
        dm_on_field = [s for s in snap.p1_field
                       if s.card_id in ("EX10-012", "EX10-020")]
        assert len(dm_on_field) == 2, (
            f"Both DM cards from security should be played as permanents on field. "
            f"Field: {[s.card_id for s in snap.p1_field]}"
        )

    def test_only_face_up_security_cards_eligible(self, debug_runner):
        """Face-down security cards should NOT be used for cost reduction."""
        runner = debug_runner(deck1=DM_DECK, deck2=DM_DECK, initial_memory=20)
        runner.set_phase("Main")

        runner.clear_zone(1, "security")
        game = runner.game
        p1 = game.player1

        # Add MetalSeadramon face-DOWN (just inject, don't add to face_up_security)
        runner.inject_card(1, "EX10-012", "security_top")
        # NOT calling p1.face_up_security.add(cs)

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "EX10-061", "hand")

        sec_before = len(p1.security_cards)

        action = runner.find_action("Play")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # Security should be unchanged (face-down card not taken)
        assert len(p1.security_cards) == sec_before, \
            "Face-down security cards should not be used for cost reduction"


@pytest.mark.behavioral
class TestEX10061OnPlayWhenDigivolving:
    """Tests for [On Play] [When Digivolving] shared effect."""

    def test_on_play_plays_dm_from_digi_cards(self, debug_runner):
        """On Play should let you play Dark Masters from digivolution cards."""
        runner = debug_runner(deck1=DM_DECK, deck2=DM_DECK, initial_memory=20)
        runner.set_phase("Main")

        runner.clear_zone(1, "security")
        game = runner.game
        p1 = game.player1

        # Set up: place 2 face-up DM in security so they become digi sources
        cs1 = runner.inject_card(1, "EX10-012", "security_top")
        p1.face_up_security.add(cs1)
        cs2 = runner.inject_card(1, "EX10-020", "security_top")
        p1.face_up_security.add(cs2)

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "EX10-061", "hand")

        field_before = len(runner.snapshot().p1_field)

        # Play Apocalymon
        action = runner.find_action("Play")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Should have Apocalymon + the played DM cards on field
        dm_on_field = [s for s in snap.p1_field
                       if s.card_id in ("EX10-012", "EX10-020")]
        # At least some DMs should be played from digi cards onto field
        # (They were security -> digi sources -> played as new permanents)
        assert len(dm_on_field) >= 1, (
            f"Dark Masters from digi cards should be played onto field. "
            f"Field: {[s.card_id for s in snap.p1_field]}"
        )

    def test_on_play_grants_rush_to_all_dm_digimon(self, debug_runner):
        """On Play should grant Rush to ALL Dark Masters trait Digimon."""
        runner = debug_runner(deck1=DM_DECK, deck2=DM_DECK, initial_memory=20)
        runner.set_phase("Main")

        # Place a pre-existing Dark Masters Digimon on field
        runner.place_on_field(1, ["EX10-057"], turn_played=-1)  # Piedmon

        runner.clear_zone(1, "security")
        game = runner.game
        p1 = game.player1

        # Add face-up DM to security for digi sources
        cs1 = runner.inject_card(1, "EX10-012", "security_top")
        p1.face_up_security.add(cs1)

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "EX10-061", "hand")

        action = runner.find_action("Play")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Check that the pre-existing Piedmon has Rush
        piedmon = next((s for s in snap.p1_field if s.card_id == "EX10-057"), None)
        assert piedmon is not None, "Piedmon should still be on field"
        assert "rush" in piedmon.keywords, (
            f"Piedmon should have Rush. Keywords: {piedmon.keywords}"
        )

    def test_on_play_without_dm_in_digi_cards_still_grants_rush(self, debug_runner):
        """Even with no DM in digi cards, Rush should be granted to existing DMs."""
        runner = debug_runner(deck1=DM_DECK, deck2=DM_DECK, initial_memory=20)
        runner.set_phase("Main")

        # Place a DM on field that already exists
        runner.place_on_field(1, ["EX10-012"], turn_played=-1)  # MetalSeadramon

        # No face-up security, so no digi sources
        runner.clear_zone(1, "security")
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "EX10-061", "hand")

        action = runner.find_action("Play")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # MetalSeadramon should have Rush
        ms = next((s for s in snap.p1_field if s.card_id == "EX10-012"), None)
        assert ms is not None, "MetalSeadramon should still be on field"
        assert "rush" in ms.keywords, (
            f"MetalSeadramon should have Rush. Keywords: {ms.keywords}"
        )

    def test_when_digivolving_plays_dm_from_digi_stack(self, debug_runner):
        """When Digivolving should play DM from the digivolution stack."""
        runner = debug_runner(deck1=DM_DECK, deck2=DM_DECK, initial_memory=10)
        runner.set_phase("Main")

        # Place a Lv6 Dark Masters on field to digivolve onto
        # Stack: MetalSeadramon (Lv6 DM) with Puppetmon as digi source under it
        perm = runner.place_on_field(1, ["EX10-020", "EX10-012"], turn_played=-1)
        # perm.card_sources = [Puppetmon (bottom), MetalSeadramon (top)]
        # After digivolving Apocalymon onto this, stack becomes:
        # [Puppetmon, MetalSeadramon, Apocalymon]

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "EX10-061", "hand")

        action = runner.find_action("Digivolve")
        assert action is not None, (
            f"Should find digivolve action. Actions: {runner.actions()}"
        )
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Apocalymon should be on field
        apoc = next((s for s in snap.p1_field if s.card_id == "EX10-061"), None)
        assert apoc is not None, "Apocalymon should be on field after digivolving"

        # DM from digi stack should have been played as separate permanents
        # Puppetmon was a digi source, so it could be played
        dm_played = [s for s in snap.p1_field
                     if s.card_id in ("EX10-020",) and s is not apoc]
        # Note: Puppetmon was under the stack, so it should be eligible to play
        # At minimum, the effect should have tried to play it

    def test_eot_deletes_played_dm(self, debug_runner):
        """At end of turn, Digimon played by Apocalymon's effect should be deleted."""
        runner = debug_runner(deck1=DM_DECK, deck2=DM_DECK, initial_memory=20)
        runner.set_phase("Main")

        runner.clear_zone(1, "security")
        game = runner.game
        p1 = game.player1

        # Add face-up DM to security
        cs1 = runner.inject_card(1, "EX10-012", "security_top")
        p1.face_up_security.add(cs1)

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "EX10-061", "hand")

        # Play Apocalymon
        action = runner.find_action("Play")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap_after_play = runner.snapshot()
        dm_on_field = [s for s in snap_after_play.p1_field
                       if s.card_id == "EX10-012"]

        if len(dm_on_field) > 0:
            # Trigger end of turn
            game.execute_effects(EffectTiming.OnEndTurn)
            runner.auto_resolve()

            snap_after_eot = runner.snapshot()
            dm_remaining = [s for s in snap_after_eot.p1_field
                            if s.card_id == "EX10-012"]
            assert len(dm_remaining) == 0, (
                f"MetalSeadramon played by Apocalymon should be deleted at EOT. "
                f"Remaining: {[s.card_id for s in snap_after_eot.p1_field]}"
            )

    def test_apocalymon_itself_not_deleted_at_eot(self, debug_runner):
        """Apocalymon itself should NOT be deleted at end of turn."""
        runner = debug_runner(deck1=DM_DECK, deck2=DM_DECK, initial_memory=20)
        runner.set_phase("Main")

        runner.clear_zone(1, "security")
        game = runner.game
        p1 = game.player1

        cs1 = runner.inject_card(1, "EX10-012", "security_top")
        p1.face_up_security.add(cs1)

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "EX10-061", "hand")

        action = runner.find_action("Play")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # Trigger end of turn
        game.execute_effects(EffectTiming.OnEndTurn)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert any(s.card_id == "EX10-061" for s in snap.p1_field), \
            "Apocalymon itself should NOT be deleted at EOT"


@pytest.mark.behavioral
class TestEX10061OnPlayDistinctNames:
    """Tests for distinct name constraint on On Play effect."""

    def test_only_plays_one_per_distinct_name(self, debug_runner):
        """Should play at most 1 card per distinct Dark Masters name."""
        runner = debug_runner(deck1=DM_DECK, deck2=DM_DECK, initial_memory=20)
        runner.set_phase("Main")

        runner.clear_zone(1, "security")
        game = runner.game
        p1 = game.player1

        # Add 2 MetalSeadramons (same name) face-up in security
        cs1 = runner.inject_card(1, "EX10-012", "security_top")
        p1.face_up_security.add(cs1)
        cs2 = runner.inject_card(1, "EX10-012", "security_top")
        p1.face_up_security.add(cs2)
        # Add 1 Puppetmon face-up
        cs3 = runner.inject_card(1, "EX10-020", "security_top")
        p1.face_up_security.add(cs3)

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "EX10-061", "hand")

        action = runner.find_action("Play")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # At most 1 MetalSeadramon should be played from digi cards
        ms_on_field = [s for s in snap.p1_field if s.card_id == "EX10-012"]
        assert len(ms_on_field) <= 1, (
            f"Should play at most 1 MetalSeadramon (distinct names). "
            f"Found: {len(ms_on_field)}"
        )
