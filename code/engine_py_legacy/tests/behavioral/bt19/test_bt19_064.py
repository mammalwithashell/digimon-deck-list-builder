"""Behavioral tests for BT19-064 Justimon: Blitz Arm (Ace).

Card text:
  Lv.6 | Black | Cost 6 | DP 11000 | Cyborg / Vaccine
  [Digivolve] Lv.5 Black for 3
  [Digivolve] [Justimon: Accel Arm]/[Justimon: Critical Arm]: Cost 1

  [Hand] [Counter] Blast Digivolve

  [On Play] [When Digivolving] This Digimon gains Blocker for the turn.
  Then, 1 of your opponent's Digimon gets -5000 DP for the turn.

  Inherited: Ace Overflow <-4>
"""

import pytest


# BT2-060 Megadramon: Lv.5 Black, 9000 DP (digivolve base for BT19-064)
# BT2-052 Hagurumon: Lv.3 Black, 3000 DP
# BT1-010 Agumon: Lv.3 Red, 2000 DP (generic filler)
# ST1-03 Biyomon: Lv.3 Red, 2000 DP (generic opponent Digimon)
DECK = ["BT19-064"] * 4 + ["BT2-060"] * 23 + ["BT2-052"] * 23


@pytest.mark.behavioral
class TestBT19064JustimonBlitzArm:
    """Tests for BT19-064 Justimon: Blitz Arm."""

    # ── [On Play] Blocker grant ─────────────────────────────────────

    def test_on_play_grants_blocker_for_the_turn(self, debug_runner):
        """[On Play] This Digimon gains Blocker for the turn."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        runner.set_phase("Main")

        # Put BT19-064 in hand
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT19-064", "hand")

        # Place an opponent Digimon for the -5000 DP target
        runner.place_on_field(2, ["ST1-03"], turn_played=-1)

        # Play BT19-064 from hand
        action = runner.find_action("Play")
        assert action is not None, f"Should find play action. Actions: {runner.actions()}"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        blitz = next((s for s in snap.p1_field if s.card_id == "BT19-064"), None)
        assert blitz is not None, "Justimon: Blitz Arm should be on field after play"
        assert "blocker" in blitz.keywords, (
            f"Justimon: Blitz Arm should have Blocker after [On Play]. "
            f"Keywords: {blitz.keywords}"
        )

    def test_on_play_blocker_uses_turn_duration(self, debug_runner):
        """Blocker from [On Play] should be 'for the turn' (expires at turn end)."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        runner.set_phase("Main")
        game = runner.game

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT19-064", "hand")
        runner.place_on_field(2, ["ST1-03"], turn_played=-1)

        action = runner.find_action("Play")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # Find the permanent on the field
        perm = None
        for p in game.player1.battle_area:
            if p.top_card and p.top_card.c_entity_base and p.top_card.c_entity_base.card_id == "BT19-064":
                perm = p
                break
        assert perm is not None, "Should find Blitz Arm permanent"

        # Blocker should be granted with turn-based duration, not permanent
        assert perm.has_keyword('_is_blocker'), "Should have blocker right after play"
        # The grant should expire (duration == turn_count, not -1 permanent)
        grant_duration = perm._granted_keywords.get('_is_blocker')
        assert grant_duration is not None, "Blocker should be in granted keywords"
        assert grant_duration != -1, (
            "Blocker should NOT be permanent (-1). Should be turn-based expiry."
        )

    # ── [On Play] -5000 DP to opponent Digimon ─────────────────────

    def test_on_play_minus_5000_dp_to_opponent_digimon(self, debug_runner):
        """[On Play] Then, 1 of opponent's Digimon gets -5000 DP for the turn."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        runner.set_phase("Main")

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT19-064", "hand")

        # Opponent has a Digimon with 9000 DP
        runner.place_on_field(2, ["BT2-060"], turn_played=-1)  # Megadramon 9000 DP

        snap_before = runner.snapshot()
        opp_digi = next(s for s in snap_before.p2_field if s.card_id == "BT2-060")
        assert opp_digi.dp == 9000, f"Megadramon should have 9000 DP, got {opp_digi.dp}"

        action = runner.find_action("Play")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        opp_digi_after = next(
            (s for s in snap.p2_field if s.card_id == "BT2-060"), None
        )
        assert opp_digi_after is not None, "Megadramon should still be on field"
        assert opp_digi_after.dp == 4000, (
            f"Megadramon should have 9000 - 5000 = 4000 DP after -5000 DP. "
            f"Got {opp_digi_after.dp}"
        )

    def test_on_play_minus_5000_dp_reduces_low_dp_to_zero(self, debug_runner):
        """If -5000 DP exceeds base DP, DP is clamped to 0 (rule_process destroys later)."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        runner.set_phase("Main")

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT19-064", "hand")

        # Opponent has a Digimon with 3000 DP (3000 - 5000 -> clamped to 0)
        runner.place_on_field(2, ["BT2-052"], turn_played=-1)  # Hagurumon 3000 DP

        action = runner.find_action("Play")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        opp_digi = next(
            (s for s in snap.p2_field if s.card_id == "BT2-052"), None
        )
        if opp_digi is not None:
            # DP should be 0 (3000 - 5000 clamped to 0)
            assert opp_digi.dp == 0, (
                f"Hagurumon should have 0 DP (3000 - 5000, clamped). Got {opp_digi.dp}"
            )
            # Rule process will destroy it; verify manually
            runner.game._rule_process()
            snap2 = runner.snapshot()
            assert not any(s.card_id == "BT2-052" for s in snap2.p2_field), (
                "Hagurumon at 0 DP should be destroyed by rule_process"
            )

    def test_on_play_no_opponent_digimon_skips_dp_reduction(self, debug_runner):
        """If opponent has no Digimon, -5000 DP step is skipped gracefully."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        runner.set_phase("Main")

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT19-064", "hand")

        # No opponent Digimon on field
        runner.clear_zone(2, "field")

        action = runner.find_action("Play")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        blitz = next((s for s in snap.p1_field if s.card_id == "BT19-064"), None)
        assert blitz is not None, "Blitz Arm should still be on field"
        assert "blocker" in blitz.keywords, "Should still have Blocker even with no targets"

    # ── [When Digivolving] Blocker grant ────────────────────────────

    def test_when_digivolving_grants_blocker(self, debug_runner):
        """[When Digivolving] This Digimon gains Blocker for the turn."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        runner.set_phase("Main")

        # Place Lv.5 Black on P1 field
        runner.place_on_field(1, ["BT2-060"], turn_played=-1)

        # Inject BT19-064 into hand for digivolution
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT19-064", "hand")

        # Place target for DP reduction
        runner.place_on_field(2, ["BT2-060"], turn_played=-1)

        action = runner.find_action("Digivolve")
        assert action is not None, f"Should find digivolve action. Actions: {runner.actions()}"
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        blitz = next((s for s in snap.p1_field if s.card_id == "BT19-064"), None)
        assert blitz is not None, "Justimon: Blitz Arm should be on field after digivolve"
        assert "blocker" in blitz.keywords, (
            f"Justimon: Blitz Arm should have Blocker after [When Digivolving]. "
            f"Keywords: {blitz.keywords}"
        )

    # ── [When Digivolving] -5000 DP to opponent Digimon ────────────

    def test_when_digivolving_minus_5000_dp(self, debug_runner):
        """[When Digivolving] Then, 1 of opponent's Digimon gets -5000 DP."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["BT2-060"], turn_played=-1)
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT19-064", "hand")

        # Opponent Digimon with 9000 DP
        runner.place_on_field(2, ["BT2-060"], turn_played=-1)

        action = runner.find_action("Digivolve")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        opp_digi = next(
            (s for s in snap.p2_field if s.card_id == "BT2-060"), None
        )
        assert opp_digi is not None, "Megadramon should still be on field"
        assert opp_digi.dp == 4000, (
            f"Megadramon should have 9000 - 5000 = 4000 DP. Got {opp_digi.dp}"
        )

    # ── Effect is NOT optional ──────────────────────────────────────

    def test_effect_is_mandatory(self, debug_runner):
        """The [On Play][When Digivolving] effect is mandatory (not optional)."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        runner.set_phase("Main")

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT19-064", "hand")
        runner.place_on_field(2, ["BT2-060"], turn_played=-1)

        action = runner.find_action("Play")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # Both parts should have fired: Blocker grant + DP reduction
        snap = runner.snapshot()
        blitz = next((s for s in snap.p1_field if s.card_id == "BT19-064"), None)
        assert blitz is not None
        assert "blocker" in blitz.keywords, "Blocker should be granted mandatorily"
        opp = next((s for s in snap.p2_field if s.card_id == "BT2-060"), None)
        assert opp is not None
        assert opp.dp == 4000, "DP reduction should happen mandatorily"

    # ── No spurious effects (trash option, effect immunity) ─────────

    def test_no_trash_option_unsuspend_effect(self, debug_runner):
        """BT19-064 should NOT have a 'trash option to unsuspend' effect."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        runner.set_phase("Main")

        runner.place_on_field(1, ["BT2-060"], turn_played=-1)
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT19-064", "hand")
        runner.place_on_field(2, ["ST1-03"], turn_played=-1)

        action = runner.find_action("Digivolve")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # Check that there's no "trash option" or "unsuspend" in the action list
        all_actions = runner.actions()
        for aid, desc in all_actions.items():
            assert "trash" not in desc.lower() or "option" not in desc.lower(), (
                f"Should NOT have a 'trash option' action. Found: {desc}"
            )

    def test_no_effect_immunity_modifier(self, debug_runner):
        """BT19-064 should NOT grant 'not affected by opponent effects' immunity."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)
        runner.set_phase("Main")
        game = runner.game

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "BT19-064", "hand")
        runner.place_on_field(2, ["ST1-03"], turn_played=-1)

        action = runner.find_action("Play")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # Find the permanent
        perm = None
        for p in game.player1.battle_area:
            if p.top_card and p.top_card.c_entity_base and p.top_card.c_entity_base.card_id == "BT19-064":
                perm = p
                break
        assert perm is not None

        # Should still be selectable by opponent effects (no immunity)
        from engine_py_legacy.engine.interfaces.modifiers import ModifierType
        can_select = game.modifiers.can_be_selected_by_effect(perm)
        assert can_select, (
            "Blitz Arm should NOT have effect immunity. "
            "The card text does not grant 'not affected by opponent effects'."
        )
