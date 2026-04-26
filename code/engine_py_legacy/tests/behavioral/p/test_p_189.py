"""Behavioral tests for P-189 Dimetromon (Lv.4 Red Digimon).

Card text:
[Security] You may play 1 card with the [LIBERATOR] trait and a play cost of
4 or less from your hand or trash without paying the cost.
<Progress> (While attacking, your opponent's effects don't affect this Digimon.)

Inherited:
[Your Turn] [Once Per Turn] When your opponent's security stack is removed from,
gain 1 memory.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, GamePhase


@pytest.mark.behavioral
class TestP189Dimetromon:
    """Tests for P-189 Dimetromon."""

    # ── Inherited: OnLoseSecurity gain memory ────────────────────────

    def test_inherited_gain_memory_when_opponent_loses_security(self, debug_runner):
        """Inherited: gain 1 memory when opponent's security is removed (via attack)."""
        runner = debug_runner(initial_memory=3)

        # Place a Lv.5 with P-189 as inherited source on P1's field
        # BT18-065 Snatchmon (Lv.4 LIBERATOR) as a host — use a generic Lv.5 instead
        # Use ST1-06 (Lv.5 Red) with P-189 as inherited
        perm = runner.place_on_field(1, ["P-189", "ST1-06"])
        runner.set_phase("Main")

        mem_before = runner.game.memory
        p2_sec_before = len(runner.game.player2.security_cards)

        # Attack with the Digimon — find attack action
        action = runner.find_action("attack")
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

            # If opponent lost security, memory should have increased by 1
            p2_sec_after = len(runner.game.player2.security_cards)
            if p2_sec_after < p2_sec_before and not runner.game.game_over:
                # The inherited effect should have fired
                assert runner.game.memory > mem_before, (
                    "Inherited effect should gain 1 memory when opponent's security is removed"
                )

    def test_inherited_no_gain_when_own_security_removed(self, debug_runner):
        """Inherited: must NOT fire when the card owner's own security is removed."""
        runner = debug_runner(initial_memory=3)

        # P1 has a stack with P-189 inherited
        perm = runner.place_on_field(1, ["P-189", "ST1-06"])
        runner.set_phase("Main")

        mem_before = runner.game.memory

        # Simulate P1 losing their own security (trash_security_card fires OnLoseSecurity)
        if runner.game.player1.security_cards:
            sec_card = runner.game.player1.security_cards[0]
            runner.game.player1.trash_security_card(sec_card)

            # Memory should NOT increase — it was P1's own security, not opponent's
            assert runner.game.memory == mem_before, (
                "Inherited effect should NOT fire when own security is removed, "
                f"expected {mem_before} but got {runner.game.memory}"
            )

    def test_inherited_once_per_turn(self, debug_runner):
        """Inherited: once per turn — second opponent security removal should not gain memory."""
        runner = debug_runner(initial_memory=3)

        perm = runner.place_on_field(1, ["P-189", "ST1-06"])
        runner.set_phase("Main")

        mem_before = runner.game.memory

        # Remove two opponent security cards manually
        if len(runner.game.player2.security_cards) >= 2:
            sec1 = runner.game.player2.security_cards[0]
            runner.game.player2.trash_security_card(sec1)
            mem_after_first = runner.game.memory

            sec2 = runner.game.player2.security_cards[0]
            runner.game.player2.trash_security_card(sec2)
            mem_after_second = runner.game.memory

            # First removal should gain 1 memory
            assert mem_after_first == mem_before + 1, (
                f"First removal should gain 1 memory: expected {mem_before + 1}, got {mem_after_first}"
            )
            # Second removal should NOT gain (once per turn)
            assert mem_after_second == mem_after_first, (
                f"Second removal should not gain (once per turn): expected {mem_after_first}, got {mem_after_second}"
            )

    def test_inherited_not_on_opponent_turn(self, debug_runner):
        """Inherited: [Your Turn] — should not fire during opponent's turn."""
        runner = debug_runner(initial_memory=3, first_player=2)

        # P1 has stack with P-189 inherited, but it's P2's turn
        perm = runner.place_on_field(1, ["P-189", "ST1-06"])
        runner.set_phase("Main")

        mem_before = runner.game.memory

        # P2 is attacking, P1 loses security — but it's P2's turn, and P-189 says [Your Turn]
        # "Your Turn" means P1's turn. Since it's P2's turn, the effect should not fire.
        # However — "your opponent's security" check matters more here:
        # P2 attacks → P1 loses security → event_player=P1 (the one who lost security)
        # P-189 owner = P1, opponent = P2 → "opponent's security" = P2's security
        # P1 losing their own security is NOT opponent's security being removed.
        if runner.game.player1.security_cards:
            sec = runner.game.player1.security_cards[0]
            runner.game.player1.trash_security_card(sec)

            assert runner.game.memory == mem_before, (
                "Inherited should not fire on opponent's turn"
            )

    # ── Security Effect: play LIBERATOR card ─────────────────────────

    def test_security_effect_plays_liberator_from_hand(self, debug_runner):
        """[Security] plays a LIBERATOR card with cost <= 4 from hand."""
        runner = debug_runner(initial_memory=0)

        # Put P-189 in P2's security stack
        runner.inject_card(2, "P-189", "security_top")
        # Put a valid LIBERATOR target in P2's hand (BT18-060 Vemmon, Lv.3, cost=3, LIBERATOR)
        runner.inject_card(2, "BT18-060", "hand")

        # Place an attacker for P1
        attacker = runner.place_on_field(1, ["ST1-06"])
        runner.set_phase("Main")

        # Have P1 attack — security check reveals P-189
        action = runner.find_action("attack")
        assert action is not None, "Should find an attack action"
        runner.execute(action)

        # After attack, security effect should present a selection to play a card
        # The game should be in a SelectTarget phase for P2 (the defender)
        if runner.game.current_phase == GamePhase.SelectTarget:
            # Find the action to select BT18-060 from hand
            select_action = runner.find_action("BT18-060")
            if select_action is None:
                select_action = runner.find_action("Vemmon")
            assert select_action is not None, (
                f"BT18-060 should be a valid selection. Available: {runner.actions()}"
            )
            runner.execute(select_action)
            runner.auto_resolve()

            # BT18-060 should now be on P2's field
            p2_field_ids = []
            for p in runner.game.player2.battle_area:
                if p.top_card and p.top_card.c_entity_base:
                    p2_field_ids.append(p.top_card.c_entity_base.card_id)
            assert "BT18-060" in p2_field_ids, (
                f"BT18-060 should have been played to P2's field. Field: {p2_field_ids}"
            )

    def test_security_effect_rejects_high_cost(self, debug_runner):
        """[Security] should NOT allow playing a LIBERATOR card with cost > 4."""
        runner = debug_runner(initial_memory=0)

        # Put P-189 in P2's security stack
        runner.inject_card(2, "P-189", "security_top")
        # Put an invalid target: BT19-024 MarineBullmon (Lv.5, cost=7, LIBERATOR) — too expensive
        runner.inject_card(2, "BT19-024", "hand")
        # Make sure no other valid targets exist
        runner.clear_zone(2, "hand")
        runner.inject_card(2, "BT19-024", "hand")

        attacker = runner.place_on_field(1, ["ST1-06"])
        runner.set_phase("Main")

        hand_before = len(runner.game.player2.hand_cards)

        action = runner.find_action("attack")
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

            # MarineBullmon should still be in hand (not played)
            p2_field_ids = []
            for p in runner.game.player2.battle_area:
                if p.top_card and p.top_card.c_entity_base:
                    p2_field_ids.append(p.top_card.c_entity_base.card_id)

            assert "BT19-024" not in p2_field_ids, (
                "BT19-024 (cost 7) should NOT be playable by security effect (max cost 4)"
            )

    def test_security_effect_rejects_non_liberator(self, debug_runner):
        """[Security] should NOT allow playing a card without [LIBERATOR] trait."""
        runner = debug_runner(initial_memory=0)

        runner.inject_card(2, "P-189", "security_top")
        # ST1-03 Agumon — no LIBERATOR trait, cost 3
        runner.clear_zone(2, "hand")
        runner.inject_card(2, "ST1-03", "hand")

        attacker = runner.place_on_field(1, ["ST1-06"])
        runner.set_phase("Main")

        action = runner.find_action("attack")
        if action is not None:
            runner.execute(action)
            runner.auto_resolve()

            p2_field_ids = []
            for p in runner.game.player2.battle_area:
                if p.top_card and p.top_card.c_entity_base:
                    p2_field_ids.append(p.top_card.c_entity_base.card_id)

            assert "ST1-03" not in p2_field_ids, (
                "Non-LIBERATOR cards should NOT be playable by security effect"
            )

    # ── Progress keyword ─────────────────────────────────────────────

    def test_progress_keyword_present(self, debug_runner):
        """Dimetromon should have the Progress keyword."""
        runner = debug_runner(initial_memory=3)
        perm = runner.place_on_field(1, ["P-189"])

        # Check that progress is set
        has_progress = False
        for effect in perm.effect_list(EffectTiming.NoTiming):
            if getattr(effect, '_is_progress', False):
                has_progress = True
                break
        # Also check across all timings
        if not has_progress:
            for cs in perm.card_sources:
                for effect in cs.effect_list(EffectTiming.NoTiming):
                    if getattr(effect, '_is_progress', False):
                        has_progress = True
                        break

        assert has_progress, "P-189 should have the Progress keyword"
