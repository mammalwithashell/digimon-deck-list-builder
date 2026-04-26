"""Behavioral tests for BT15-037 Gatomon (Lv.4 Yellow Holy Beast).

Card text (source of truth):
    When an effect trashes this card from the security stack, you may play it
    without paying the cost.
    <Barrier> (When this Digimon would be deleted in battle, by trashing the top
    card of your security stack, prevent that deletion.)
    [All Turns] [Once Per Turn] When a card is removed from your security stack,
    gain 1 memory.

Inherited:
    <Barrier>
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT15037Gatomon:
    """Tests for BT15-037 Gatomon."""

    # ------------------------------------------------------------------
    # Effect inventory / structure
    # ------------------------------------------------------------------

    def test_script_loads(self, debug_runner):
        """Script imports cleanly and produces effects."""
        runner = debug_runner(initial_memory=5)
        runner.inject_card(1, "BT15-037", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)
        assert len(effects) >= 4, (
            f"Expected at least 4 effects "
            f"(Barrier, inherited Barrier, OnLoseSecurity +1 memory, "
            f"OnDiscardSecurity play self), got {len(effects)}"
        )

    def test_has_barrier_non_inherited(self, debug_runner):
        """Main-text Barrier: non-inherited _is_barrier effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-037"])
        top = perm.top_card
        effects = top.effect_list(None)
        barrier = [
            e for e in effects
            if getattr(e, "_is_barrier", False)
            and not getattr(e, "is_inherited_effect", False)
        ]
        assert len(barrier) >= 1, "Should have a non-inherited Barrier effect"

    def test_has_barrier_inherited(self, debug_runner):
        """Inherited Barrier: separate _is_barrier instance with is_inherited_effect=True."""
        runner = debug_runner(initial_memory=5)
        runner.inject_card(1, "BT15-037", "hand")
        card = runner.game.player1.hand_cards[-1]
        effects = card.effect_list(None)
        inherited_barrier = [
            e for e in effects
            if getattr(e, "_is_barrier", False)
            and getattr(e, "is_inherited_effect", False)
        ]
        assert len(inherited_barrier) >= 1, "Should have an inherited Barrier effect"

    def test_barrier_keyword_active_on_field(self, debug_runner):
        """Gatomon on field should register as having barrier via has_keyword."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-037"])
        assert perm.has_keyword("_is_barrier"), (
            "Gatomon on battle area should have barrier keyword via effect scan"
        )

    def test_inherited_barrier_active_under_digivolve(self, debug_runner):
        """BT15-037 underneath another Digimon should grant barrier via inherited effect."""
        runner = debug_runner(initial_memory=5)
        # Stack BT15-037 under any Lv.5 yellow digimon.
        # Using raw stack injection avoids needing a valid digivolve path.
        perm = runner.place_on_field(1, ["BT15-037", "ST1-03"])
        assert perm.has_keyword("_is_barrier"), (
            "Barrier should be active via inherited effect of BT15-037 under top"
        )

    # ------------------------------------------------------------------
    # Clause 3: [All Turns] [OPT] OnLoseSecurity +1 memory
    # ------------------------------------------------------------------

    def _get_lose_security_effect(self, card):
        effects = card.effect_list(None)
        found = [
            e for e in effects
            if e.timing == EffectTiming.OnLoseSecurity
            and not getattr(e, "is_inherited_effect", False)
        ]
        assert len(found) == 1, (
            f"Should have exactly 1 non-inherited OnLoseSecurity effect, got {len(found)}"
        )
        return found[0]

    def test_lose_security_is_once_per_turn(self, debug_runner):
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-037"])
        eff = self._get_lose_security_effect(perm.top_card)
        assert eff.max_count_per_turn == 1, "Should be Once Per Turn"

    def test_lose_security_not_inherited(self, debug_runner):
        """Main-text OPT memory trigger must be non-inherited."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-037"])
        eff = self._get_lose_security_effect(perm.top_card)
        assert not getattr(eff, "is_inherited_effect", False)

    def test_lose_security_requires_field_presence(self, debug_runner):
        """Condition must fail when Gatomon is not on the field (e.g. still in hand)."""
        runner = debug_runner(initial_memory=5)
        runner.inject_card(1, "BT15-037", "hand")
        card = runner.game.player1.hand_cards[-1]
        eff = self._get_lose_security_effect(card)
        ctx = {
            "game": runner.game,
            "player": runner.game.player1,
            "permanent": None,
            "card": card,
            "event_player": runner.game.player1,
            "lost_card": card,
        }
        assert eff.can_use_condition(ctx) is False, (
            "OPT +1 memory must not fire when Gatomon is in hand"
        )

    def test_lose_security_condition_requires_owner_event(self, debug_runner):
        """Owner gate: must fail when OPPONENT loses security."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-037"])
        eff = self._get_lose_security_effect(perm.top_card)
        game = runner.game

        # Event fired for OPPONENT losing security — should NOT trigger for player1's Gatomon.
        ctx_opp = {
            "game": game,
            "player": game.player1,  # zone owner / effect host owner
            "permanent": perm,
            "card": perm.top_card,
            "event_player": game.player2,  # the opponent lost security
            "lost_card": game.player2.security_cards[0] if game.player2.security_cards else None,
        }
        assert eff.can_use_condition(ctx_opp) is False, (
            "Must not fire when opponent loses security (owner gate)"
        )

    def test_lose_security_condition_passes_on_own_event(self, debug_runner):
        """Owner gate: should pass when OWNER's security is lost."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-037"])
        eff = self._get_lose_security_effect(perm.top_card)
        game = runner.game
        ctx_own = {
            "game": game,
            "player": game.player1,
            "permanent": perm,
            "card": perm.top_card,
            "event_player": game.player1,
            "lost_card": game.player1.security_cards[0] if game.player1.security_cards else None,
        }
        assert eff.can_use_condition(ctx_own) is True, (
            "Should fire when owner's security is lost"
        )

    def test_lose_security_process_gains_1_memory_on_own_turn(self, debug_runner):
        """Process callback should add 1 memory to the effect owner (positive for turn player)."""
        runner = debug_runner(initial_memory=3)
        perm = runner.place_on_field(1, ["BT15-037"])
        eff = self._get_lose_security_effect(perm.top_card)
        game = runner.game

        # Ensure player1 is the turn player
        assert game.turn_player is game.player1
        before = game.memory

        eff.on_process_callback({
            "game": game,
            "player": game.player1,
            "permanent": perm,
            "card": perm.top_card,
            "event_player": game.player1,
        })
        # Memory is positive for turn player — + 1
        assert game.memory == before + 1, (
            f"Memory should be {before + 1}, got {game.memory}"
        )

    def test_lose_security_process_gains_1_memory_on_opponent_turn(self, debug_runner):
        """When not turn player, add_memory decrements game.memory (i.e. shifts +1 toward self).

        This verifies we're calling player.add_memory(1) (engine helper), not raw
        game.memory += 1.
        """
        runner = debug_runner(initial_memory=3)
        perm = runner.place_on_field(1, ["BT15-037"])
        eff = self._get_lose_security_effect(perm.top_card)
        game = runner.game

        # Flip turn player so player1 (Gatomon owner) is NOT the turn player
        game.turn_player, game.opponent_player = game.player2, game.player1
        game.player2.is_my_turn = True
        game.player1.is_my_turn = False

        before = game.memory  # e.g. 3 from the perspective of player2 being turn player
        eff.on_process_callback({
            "game": game,
            "player": game.player1,
            "permanent": perm,
            "card": perm.top_card,
            "event_player": game.player1,
        })
        # Non-turn player add_memory(1) => game.memory -= 1
        assert game.memory == before - 1, (
            f"Expected memory {before - 1} (shift toward non-turn-player), got {game.memory}"
        )

    # ------------------------------------------------------------------
    # Clause 1: OnDiscardSecurity — play this card from trash without paying cost
    # ------------------------------------------------------------------

    def _get_discard_security_effect(self, card):
        effects = card.effect_list(None)
        found = [
            e for e in effects
            if e.timing == EffectTiming.OnDiscardSecurity
        ]
        assert len(found) == 1, (
            f"Should have exactly 1 OnDiscardSecurity effect, got {len(found)}"
        )
        return found[0]

    def test_discard_security_effect_exists_and_is_optional(self, debug_runner):
        runner = debug_runner(initial_memory=5)
        runner.inject_card(1, "BT15-037", "security_top")
        card = runner.game.player1.security_cards[0]
        eff = self._get_discard_security_effect(card)
        assert eff.is_optional is True, (
            "'you may play' => must be marked is_optional"
        )
        assert not getattr(eff, "is_inherited_effect", False), (
            "Play-from-trash trigger is main text, not inherited"
        )

    def test_discard_security_condition_fails_for_other_card(self, debug_runner):
        """Guard: when OnDiscardSecurity fires for a DIFFERENT card, condition must fail."""
        runner = debug_runner(initial_memory=5)
        # Put BT15-037 in trash (simulates it was already trashed earlier)
        runner.inject_card(1, "BT15-037", "trash")
        gato = runner.game.player1.trash_cards[-1]
        eff = self._get_discard_security_effect(gato)

        # Simulate OnDiscardSecurity fired for a different card (e.g. another security card)
        other_card = object()  # placeholder — just not `gato`
        ctx = {
            "game": runner.game,
            "player": runner.game.player1,
            "permanent": None,
            "card": gato,
            "discarded_card": other_card,
        }
        assert eff.can_use_condition(ctx) is False, (
            "Must not re-fire when a different card is trashed from security"
        )

    def test_discard_security_condition_passes_for_self(self, debug_runner):
        """When THIS card is the one being trashed, condition should pass."""
        runner = debug_runner(initial_memory=5)
        runner.inject_card(1, "BT15-037", "trash")
        gato = runner.game.player1.trash_cards[-1]
        eff = self._get_discard_security_effect(gato)
        ctx = {
            "game": runner.game,
            "player": runner.game.player1,
            "permanent": None,
            "card": gato,
            "discarded_card": gato,
        }
        assert eff.can_use_condition(ctx) is True, (
            "Must fire when THIS card is trashed from security"
        )

    def test_discard_security_process_plays_self_from_trash(self, debug_runner):
        """Process should play THIS specific card from trash for free (no cost paid)."""
        runner = debug_runner(initial_memory=5)
        runner.inject_card(1, "BT15-037", "trash")
        game = runner.game
        gato = game.player1.trash_cards[-1]
        eff = self._get_discard_security_effect(gato)

        memory_before = game.memory
        field_before = len(game.player1.battle_area)
        assert gato in game.player1.trash_cards

        eff.on_process_callback({
            "game": game,
            "player": game.player1,
            "permanent": None,
            "card": gato,
            "discarded_card": gato,
        })
        # Resolve any follow-up selections (the "may" decline branch or play branch)
        runner.auto_resolve()

        # BT15-037 should have left the trash and be on the battle area
        assert gato not in game.player1.trash_cards, (
            "Gatomon should be removed from trash after play"
        )
        assert len(game.player1.battle_area) == field_before + 1, (
            "One new permanent should be on the battle area"
        )
        # The top card of the new permanent should be gato
        new_perm = game.player1.battle_area[-1]
        assert new_perm.top_card is gato, (
            "The new permanent's top card should be the trashed Gatomon"
        )
        # Cost must not be paid (memory unchanged)
        assert game.memory == memory_before, (
            f"Memory should be unchanged (free play). Before={memory_before}, after={game.memory}"
        )

    def test_discard_security_process_does_not_play_from_hand(self, debug_runner):
        """Regression: BUG the original script plays from 'hand', but the card is in trash.

        A Gatomon sitting in hand must NOT be the one that gets played by this effect.
        Only the card that was trashed from security is eligible.
        """
        runner = debug_runner(initial_memory=5)
        # Put one Gatomon in trash (the trashed-from-security one)
        runner.inject_card(1, "BT15-037", "trash")
        # And another in hand (decoy — must not be played)
        runner.inject_card(1, "BT15-037", "hand")
        game = runner.game
        trashed_gato = game.player1.trash_cards[-1]
        hand_gato = game.player1.hand_cards[-1]
        eff = self._get_discard_security_effect(trashed_gato)

        hand_before = list(game.player1.hand_cards)
        eff.on_process_callback({
            "game": game,
            "player": game.player1,
            "permanent": None,
            "card": trashed_gato,
            "discarded_card": trashed_gato,
        })
        runner.auto_resolve()

        # The hand Gatomon must still be in hand
        assert hand_gato in game.player1.hand_cards, (
            "Hand Gatomon must NOT be played by this effect; only the trashed one"
        )
        # The trashed Gatomon should have been played
        assert trashed_gato not in game.player1.trash_cards
        assert any(
            p.top_card is trashed_gato for p in game.player1.battle_area
        ), "Trashed Gatomon should now be on battle area"
