"""Behavioral tests for BT15-038 Angewomon (Lv.5 Yellow, Cost 4, Archangel/Vaccine).

Card text (per cards.json / C# source of truth):
[Hand] [Counter] <Blast Digivolve> (Your Digimon may digivolve into this card
    without paying the cost.)
[On Play] [When Digivolving] By trashing the top or bottom card of your security
    stack, 1 of your opponent's Digimon gets -6000 DP until the end of their turn.
[All Turns] [Once Per Turn] When a card is removed from your security stack, if
    you have 3 or fewer security cards, <Recovery +1 (Deck)>.

Inherited: Ace Overflow <-3>
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, GamePhase
from engine_py_legacy.engine.interfaces.modifiers import ModifierType


@pytest.mark.behavioral
class TestBT15038Angewomon:
    """Tests for BT15-038 Angewomon."""

    # ------------------------------------------------------------------
    # Clause 1: [Hand][Counter] <Blast Digivolve>
    # ------------------------------------------------------------------

    def test_has_blast_digivolve_keyword(self, debug_runner):
        """Should have Blast Digivolve keyword effect flagged as counter."""
        runner = debug_runner(initial_memory=5)
        runner.inject_card(1, "BT15-038", "hand")
        bt15_card = runner.game.player1.hand_cards[-1]

        effects = bt15_card.effect_list(None)
        blast = [e for e in effects if getattr(e, "_is_blast_digivolve", False)]
        assert len(blast) == 1, "Should have exactly 1 Blast Digivolve effect"
        assert getattr(blast[0], "is_counter_effect", False), (
            "Blast Digivolve should be a [Counter] effect"
        )

    # ------------------------------------------------------------------
    # Clause 2/3: [On Play] / [When Digivolving] DP debuff
    # ------------------------------------------------------------------

    def _get_on_play_dp_effect(self, perm):
        """Extract the OnPlay DP-debuff effect from the permanent."""
        effects = perm.top_card.effect_list(None)
        on_play = [
            e for e in effects
            if getattr(e, "is_on_play", False)
            and not getattr(e, "is_when_digivolving", False)
            and "6000" in (getattr(e, "effect_description", "") or "")
        ]
        return on_play[0] if on_play else None

    def _get_when_digivolving_dp_effect(self, perm):
        """Extract the WhenDigivolving DP-debuff effect from the permanent."""
        effects = perm.top_card.effect_list(None)
        wd = [
            e for e in effects
            if getattr(e, "is_when_digivolving", False)
            and not getattr(e, "is_on_play", False)
            and "6000" in (getattr(e, "effect_description", "") or "")
        ]
        return wd[0] if wd else None

    def test_on_play_and_wd_effects_exist(self, debug_runner):
        """Should have separate effects for [On Play] and [When Digivolving]."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-038"])

        on_play = self._get_on_play_dp_effect(perm)
        wd = self._get_when_digivolving_dp_effect(perm)
        assert on_play is not None, "Should have [On Play] DP debuff effect"
        assert wd is not None, "Should have [When Digivolving] DP debuff effect"
        assert on_play is not wd, "On Play and When Digivolving should be separate instances"

    def test_dp_effect_is_optional(self, debug_runner):
        """DP debuff is an optional 'By trashing' cost effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-038"])

        on_play = self._get_on_play_dp_effect(perm)
        assert on_play.is_optional, (
            "[On Play] DP debuff should be optional ('By trashing' cost)"
        )
        wd = self._get_when_digivolving_dp_effect(perm)
        assert wd.is_optional, (
            "[When Digivolving] DP debuff should be optional"
        )

    def test_dp_effect_requires_security(self, debug_runner):
        """Condition should fail with no security cards."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-038"])
        game = runner.game

        # Clear security
        game.player1.security_cards.clear()

        on_play = self._get_on_play_dp_effect(perm)
        result = on_play.can_use_condition({
            "player": game.player1, "permanent": perm, "game": game,
        })
        assert not result, "Should not be usable with 0 security cards"

    def test_dp_effect_passes_with_security_on_field(self, debug_runner):
        """Condition passes when card is on field and has security cards."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-038"])
        game = runner.game

        assert len(game.player1.security_cards) > 0
        on_play = self._get_on_play_dp_effect(perm)
        result = on_play.can_use_condition({
            "player": game.player1, "permanent": perm, "game": game,
        })
        assert result, "Should be usable when on field with security"

    def test_dp_effect_fails_when_not_on_field(self, debug_runner):
        """Condition should fail if card is not on the field."""
        runner = debug_runner(initial_memory=5)
        runner.inject_card(1, "BT15-038", "hand")
        bt15_card = runner.game.player1.hand_cards[-1]

        effects = bt15_card.effect_list(None)
        on_play = [
            e for e in effects
            if getattr(e, "is_on_play", False)
            and "6000" in (getattr(e, "effect_description", "") or "")
        ][0]

        # Not on field yet - condition should fail
        result = on_play.can_use_condition({
            "player": runner.game.player1, "game": runner.game,
        })
        assert not result, "Should not be usable when not on field"

    def test_dp_effect_trashes_security_and_applies_dp_debuff(self, debug_runner):
        """Process should trash top security and register CHANGE_DP -6000 on opp Digimon."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-038"])
        # Opponent Digimon
        opp_perm = runner.place_on_field(2, ["ST1-07"])  # Greymon Lv.4 4000 DP
        game = runner.game

        security_before = len(game.player1.security_cards)
        base_dp = opp_perm.dp

        on_play = self._get_on_play_dp_effect(perm)
        on_play.on_process_callback({
            "player": game.player1, "permanent": perm, "game": game,
        })

        # Auto-resolve: picks branch 0 (top security) then target 0 (opp digimon)
        runner.auto_resolve()

        assert len(game.player1.security_cards) == security_before - 1, (
            "Should trash 1 security card"
        )
        # DP should be reduced by 6000
        assert opp_perm.dp == max(0, base_dp - 6000), (
            f"Opponent Digimon DP should drop by 6000: expected "
            f"{max(0, base_dp - 6000)}, got {opp_perm.dp}"
        )

    def test_dp_effect_trashes_top_option(self, debug_runner):
        """Choosing branch 0 (top) should trash the top security card."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-038"])
        opp = runner.place_on_field(2, ["ST1-07"])
        game = runner.game

        if len(game.player1.security_cards) < 2:
            return
        top_card = game.player1.security_cards[0]

        on_play = self._get_on_play_dp_effect(perm)
        on_play.on_process_callback({
            "player": game.player1, "permanent": perm, "game": game,
        })
        runner.auto_resolve()

        assert top_card in game.player1.trash_cards, (
            "The top security card should end up in trash"
        )

    def test_dp_effect_expiry_is_end_of_opponent_turn(self, debug_runner):
        """The -6000 DP modifier should expire at end of opponent's turn."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-038"])
        opp = runner.place_on_field(2, ["ST1-07"])
        game = runner.game

        on_play = self._get_on_play_dp_effect(perm)
        on_play.on_process_callback({
            "player": game.player1, "permanent": perm, "game": game,
        })
        runner.auto_resolve()

        # Check modifier registry for CHANGE_DP entries on the opponent's perm
        mods = game.modifiers._modifiers.get(ModifierType.CHANGE_DP, [])
        matching = [m for m in mods if m.source_permanent is opp or
                    any(m.is_active(opp, {}) for _ in [None])]
        # At least one CHANGE_DP modifier targeting opp should exist with right expiry
        active = [m for m in mods if m.is_active(opp, {})]
        assert any(m.expiry == "end_of_opponent_turn" for m in active), (
            f"CHANGE_DP modifier should expire at end_of_opponent_turn; "
            f"got {[m.expiry for m in active]}"
        )

    # ------------------------------------------------------------------
    # Clause 4: [All Turns] [OPT] Recovery +1 when <=3 security
    # ------------------------------------------------------------------

    def _get_recovery_effect(self, perm):
        effects = perm.top_card.effect_list(None)
        matches = [
            e for e in effects
            if e.timing == EffectTiming.OnLoseSecurity
            and "Recovery" in (getattr(e, "effect_description", "") or "")
        ]
        return matches[0] if matches else None

    def test_recovery_effect_exists_and_opt(self, debug_runner):
        """Recovery effect should exist, fire on OnLoseSecurity, be OPT."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-038"])

        rec = self._get_recovery_effect(perm)
        assert rec is not None, "Should have OnLoseSecurity Recovery effect"
        assert rec.max_count_per_turn == 1, "Recovery effect should be OPT"
        assert getattr(rec, "hash_string", None) == "Recovery1_BT15_038", (
            "Recovery effect should have the correct hash_string"
        )

    def test_recovery_blocks_when_more_than_3_security(self, debug_runner):
        """Condition should fail when player has >3 security cards."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-038"])
        game = runner.game

        # Game starts with 5 security
        assert len(game.player1.security_cards) > 3
        rec = self._get_recovery_effect(perm)
        ctx = {
            "player": game.player1, "permanent": perm, "game": game,
            "event_player": game.player1,
        }
        assert not rec.can_use_condition(ctx), (
            "Recovery should not activate with >3 security cards"
        )

    def test_recovery_passes_when_3_or_fewer_security(self, debug_runner):
        """Condition should pass when player has <=3 security cards."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-038"])
        game = runner.game

        # Trim security to exactly 3
        while len(game.player1.security_cards) > 3:
            c = game.player1.security_cards.pop()
            game.player1.trash_cards.append(c)

        rec = self._get_recovery_effect(perm)
        ctx = {
            "player": game.player1, "permanent": perm, "game": game,
            "event_player": game.player1,
        }
        assert rec.can_use_condition(ctx), (
            "Recovery should activate with exactly 3 security cards"
        )

    def test_recovery_does_not_fire_when_opponent_loses_security(self, debug_runner):
        """The OPT effect triggers only when YOUR security is lost (C# playerCondition)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-038"])
        game = runner.game

        # Trim our security to 3
        while len(game.player1.security_cards) > 3:
            c = game.player1.security_cards.pop()
            game.player1.trash_cards.append(c)

        rec = self._get_recovery_effect(perm)
        # Simulate OnLoseSecurity fired by OPPONENT losing security
        ctx = {
            "player": game.player1, "permanent": perm, "game": game,
            "event_player": game.player2,
        }
        assert not rec.can_use_condition(ctx), (
            "Recovery must NOT trigger when the OPPONENT loses security"
        )

    def test_recovery_fails_when_not_on_field(self, debug_runner):
        """Condition should fail when card is not on the field."""
        runner = debug_runner(initial_memory=5)
        runner.inject_card(1, "BT15-038", "hand")
        bt15_card = runner.game.player1.hand_cards[-1]

        effects = bt15_card.effect_list(None)
        rec_list = [
            e for e in effects
            if e.timing == EffectTiming.OnLoseSecurity
            and "Recovery" in (getattr(e, "effect_description", "") or "")
        ]
        assert rec_list, "Recovery effect should exist in hand (but disabled)"
        rec = rec_list[0]

        # Trim security
        game = runner.game
        while len(game.player1.security_cards) > 3:
            c = game.player1.security_cards.pop()
            game.player1.trash_cards.append(c)

        ctx = {
            "player": game.player1, "permanent": None, "game": game,
            "event_player": game.player1,
        }
        assert not rec.can_use_condition(ctx), (
            "Recovery should not fire while card is in hand"
        )

    def test_recovery_process_adds_security_card(self, debug_runner):
        """Process: recovery(1) should move top deck card to security."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT15-038"])
        game = runner.game

        # Trim to 2 security, so <=3
        while len(game.player1.security_cards) > 2:
            c = game.player1.security_cards.pop()
            game.player1.trash_cards.append(c)

        security_before = len(game.player1.security_cards)
        library_before = len(game.player1.library_cards)

        rec = self._get_recovery_effect(perm)
        rec.on_process_callback({
            "player": game.player1, "permanent": perm, "game": game,
        })

        assert len(game.player1.security_cards) == security_before + 1, (
            "Recovery should add 1 card to security stack"
        )
        assert len(game.player1.library_cards) == library_before - 1, (
            "Recovery should remove 1 card from top of library"
        )

    # ------------------------------------------------------------------
    # Clause 5: Inherited Ace Overflow <-3>
    # ------------------------------------------------------------------

    def test_has_ace_overflow_inherited(self, debug_runner):
        """Should have an inherited Ace Overflow marker effect with value -3."""
        runner = debug_runner(initial_memory=5)
        runner.inject_card(1, "BT15-038", "hand")
        bt15_card = runner.game.player1.hand_cards[-1]

        effects = bt15_card.effect_list(None)
        ace_effects = [
            e for e in effects
            if getattr(e, "is_inherited_effect", False)
            and "Ace Overflow" in (getattr(e, "effect_description", "") or "")
        ]
        assert len(ace_effects) == 1, "Should have 1 inherited Ace Overflow effect"

    def test_card_is_flagged_as_ace(self, debug_runner):
        """The CardSource metadata should mark BT15-038 as an ACE card with cost 3."""
        runner = debug_runner(initial_memory=5)
        runner.inject_card(1, "BT15-038", "hand")
        bt15_card = runner.game.player1.hand_cards[-1]

        base = bt15_card.c_entity_base
        assert base is not None
        # Core engine applies ACE Overflow via c_entity_base.is_ace / ace_overflow_cost
        assert getattr(base, "is_ace", False), (
            "BT15-038 should be flagged as ACE in card metadata"
        )
        assert getattr(base, "ace_overflow_cost", 0) == 3, (
            "BT15-038 Ace Overflow cost should be 3"
        )
