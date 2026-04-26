"""Behavioral tests for EX10-031 DarkKnightmon (Lv.5 Black/Purple Digimon).

Card text:
  [On Play] [When Digivolving] Until your opponent's turn ends, their
  <De-Digivolve> effects don't affect 1 of your Digimon, and it gets +3000 DP.

  [All Turns] [Once Per Turn] When this Digimon would leave the battle area,
  you may play 1 play cost 4 or lower card from its digivolution cards
  without paying the cost.

  Inherited: [Opponent's Turn] [Once Per Turn] When one of your opponent's
  Digimon attacks, you may change the attack target to this Digimon.

Alt-digi: From a Lv.4 Digimon with [Knightmon] in its card text, cost 3.

Key cards used:
  EX10-031: DarkKnightmon (Lv.5, Black/Purple, cost 7, 7000 DP)
  BT7-058:  SkullKnightmon (Lv.4, Black, cost 4) — alt-digi source & play-from-digi
  BT2-052:  Hagurumon (Lv.3, Black, cost 2) — cheap Black Digimon
  ST1-03:   Agumon (Lv.3, Red, cost 3) — non-Knightmon Lv.3 for negative alt-digi
  BT1-009:  Greymon (Lv.4, Red, cost 4) — non-Knightmon Lv.4
  BT2-058:  Guardromon (Lv.5, Black, cost 5) — Lv.5, not eligible for alt-digi level
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming, GamePhase
from engine_py_legacy.engine.interfaces.modifiers import ModifierType


# ── Helpers ────────────────────────────────────────────────────────────────


def _resolve_accepting(runner, max_steps=20):
    """Auto-resolve selection phases by always picking a non-decline action."""
    for _ in range(max_steps):
        if runner.game.game_over:
            break
        if runner.game.current_phase in (
            GamePhase.Main, GamePhase.Breeding, GamePhase.End
        ):
            break
        legal = runner.action_mask()
        if not legal:
            break
        non_decline = [a for a in legal if a != 62]
        if non_decline:
            runner.execute(non_decline[0])
        else:
            runner.execute(legal[0])


def _resolve_declining(runner, max_steps=10):
    """Auto-resolve selection phases by declining (action 62) when possible."""
    for _ in range(max_steps):
        if runner.game.game_over:
            break
        if runner.game.current_phase in (
            GamePhase.Main, GamePhase.Breeding, GamePhase.End
        ):
            break
        legal = runner.action_mask()
        if not legal:
            break
        if 62 in legal:
            runner.execute(62)
        else:
            runner.execute(legal[0])


def _get_perm_by_top(perm_list, card_id: str):
    """Return first permanent whose top card has the given card_id, or None."""
    for perm in perm_list:
        top = perm.top_card
        if top and top.c_entity_base and top.c_entity_base.card_id == card_id:
            return perm
    return None


# ── Alt-Digi requirement tests ─────────────────────────────────────────────


@pytest.mark.behavioral
class TestEX10031AltDigi:
    """Alt-digi: From Lv.4 Digimon with [Knightmon] in card text, cost 3."""

    def test_alt_digi_level_is_4(self, debug_runner):
        """EX10-031 declares _alt_digi_cost=3, _alt_digi_level=4."""
        runner = debug_runner()
        runner.inject_card(1, "EX10-031", "hand")
        # Find the alt-digi effect on the card in hand
        hand = runner.game.player1.hand_cards
        ex10_031_card = None
        for c in hand:
            if c.c_entity_base and c.c_entity_base.card_id == "EX10-031":
                ex10_031_card = c
                break
        assert ex10_031_card is not None
        effects = ex10_031_card.effect_list(EffectTiming.NoTiming)
        alt_effects = [e for e in effects
                       if getattr(e, '_alt_digi_cost', None) is not None]
        assert len(alt_effects) >= 1, "Should have at least 1 alt-digi effect"
        alt = alt_effects[0]
        assert alt._alt_digi_cost == 3, f"Cost should be 3, got {alt._alt_digi_cost}"
        assert getattr(alt, '_alt_digi_level', None) == 4, (
            "Level should be 4 (C# checks targetPermanent.TopCard.Level == 4)"
        )

    def test_alt_digi_requires_knightmon_text(self, debug_runner):
        """EX10-031 alt-digi requires the base to have 'Knightmon' in card text."""
        runner = debug_runner()
        runner.inject_card(1, "EX10-031", "hand")
        hand = runner.game.player1.hand_cards
        ex10_031_card = None
        for c in hand:
            if c.c_entity_base and c.c_entity_base.card_id == "EX10-031":
                ex10_031_card = c
                break
        effects = ex10_031_card.effect_list(EffectTiming.NoTiming)
        alt = [e for e in effects
               if getattr(e, '_alt_digi_cost', None) is not None][0]
        has_text = getattr(alt, '_alt_digi_has_text', None)
        cond_fn = getattr(alt, '_alt_digi_condition_fn', None)
        assert has_text is not None or cond_fn is not None, (
            "Alt-digi must enforce 'Knightmon' text via _alt_digi_has_text "
            "or _alt_digi_condition_fn (C# uses HasText('Knightmon'))"
        )

    def test_alt_digi_accepts_skullknightmon_lv4(self, debug_runner):
        """Alt-digi validator should accept Lv.4 Knightmon source."""
        from engine_py_legacy.engine.validation.digivolve_validator import (
            _check_alt_digivolve
        )
        runner = debug_runner()
        # Place SkullKnightmon Lv.4 (BT7-058) on field
        knight_perm = runner.place_on_field(1, ["BT7-058"])
        # Inject EX10-031 into hand
        runner.inject_card(1, "EX10-031", "hand")
        hand = runner.game.player1.hand_cards
        evo_card = next(
            c for c in hand if c.c_entity_base
            and c.c_entity_base.card_id == "EX10-031"
        )
        assert _check_alt_digivolve(evo_card, knight_perm), (
            "Alt-digi should accept SkullKnightmon Lv.4 (contains 'Knightmon' in name)"
        )

    def test_alt_digi_rejects_non_knightmon(self, debug_runner):
        """Alt-digi should reject a Lv.4 Digimon without 'Knightmon' in text."""
        from engine_py_legacy.engine.validation.digivolve_validator import (
            _check_alt_digivolve
        )
        runner = debug_runner()
        # Place a Lv.4 Digimon without Knightmon text (Greymon)
        greymon_perm = runner.place_on_field(1, ["BT1-009"])
        runner.inject_card(1, "EX10-031", "hand")
        hand = runner.game.player1.hand_cards
        evo_card = next(
            c for c in hand if c.c_entity_base
            and c.c_entity_base.card_id == "EX10-031"
        )
        assert not _check_alt_digivolve(evo_card, greymon_perm), (
            "Alt-digi should reject non-Knightmon Lv.4 Digimon"
        )

    def test_alt_digi_rejects_wrong_level(self, debug_runner):
        """Alt-digi should reject a Lv.5 Digimon even if it contains 'Knightmon'."""
        from engine_py_legacy.engine.validation.digivolve_validator import (
            _check_alt_digivolve
        )
        runner = debug_runner()
        # Place a Lv.5 Knightmon (BT18-069)
        knight5_perm = runner.place_on_field(1, ["BT18-069"])
        runner.inject_card(1, "EX10-031", "hand")
        hand = runner.game.player1.hand_cards
        evo_card = next(
            c for c in hand if c.c_entity_base
            and c.c_entity_base.card_id == "EX10-031"
        )
        assert not _check_alt_digivolve(evo_card, knight5_perm), (
            "Alt-digi should reject Lv.5 Knightmon (requires Lv.4)"
        )


# ── C1/C2 [On Play]/[When Digivolving] De-Digi immunity + +3000 DP ─────────


@pytest.mark.behavioral
class TestEX10031ImmunityAndDP:
    """Test [On Play] / [When Digivolving] grant DP boost and immunity."""

    def test_on_play_grants_3000_dp(self, debug_runner):
        """On Play, selecting a Digimon should grant +3000 DP."""
        runner = debug_runner(initial_memory=10)
        # Pre-place a target Digimon (Hagurumon) and another so we have choices
        target_perm = runner.place_on_field(1, ["BT2-052"])
        base_dp = target_perm.dp
        # Inject EX10-031 into hand with enough memory to play
        runner.inject_card(1, "EX10-031", "hand")
        runner.set_phase("Main")
        # Raise memory so we can afford cost 7
        runner.set_memory(10)

        play_action = runner.find_action("Play DarkKnightmon")
        assert play_action is not None, (
            f"Should be able to play EX10-031. Actions: {runner.actions()}"
        )
        runner.execute(play_action)

        # Resolve selection (pick the first valid non-decline target)
        _resolve_accepting(runner)

        # Target should have +3000 DP
        assert target_perm.dp >= base_dp + 3000, (
            f"Target should have +3000 DP (base {base_dp}, "
            f"now {target_perm.dp})"
        )

    def test_on_play_grants_de_digivolve_immunity(self, debug_runner):
        """The chosen Digimon should be immune to De-Digivolve."""
        runner = debug_runner(initial_memory=10)
        # Build a Digimon with a digi source so we can attempt de-digivolve
        # Stack: BT2-052 (Hagurumon, bottom) -> BT7-058 (SkullKnightmon, top)
        target_perm = runner.place_on_field(1, ["BT2-052", "BT7-058"])
        stack_before = len(target_perm.card_sources)
        assert stack_before == 2

        # Play EX10-031 (On Play triggers selection)
        runner.inject_card(1, "EX10-031", "hand")
        runner.set_phase("Main")
        runner.set_memory(10)
        play_action = runner.find_action("Play DarkKnightmon")
        assert play_action is not None
        runner.execute(play_action)

        # Resolve selection to pick the SkullKnightmon (target_perm)
        # The first non-decline target should be one of the two Digimon
        _resolve_accepting(runner)

        # Now: if we try de_digivolve on the target perm, if it was the one
        # selected, it should be immune. To guarantee selection was on the
        # target, verify via modifier registry state and attempted removal.
        game = runner.game
        # Try de-digivolving BOTH perms; at least one (the chosen) must be immune
        immune_count = 0
        for perm in game.player1.battle_area:
            if perm.is_digimon and len(perm.card_sources) >= 2:
                sources_before = len(perm.card_sources)
                removed = perm.de_digivolve(1)
                if not removed and sources_before >= 2:
                    # immune: return the removed cards if any
                    immune_count += 1
                else:
                    # Put back if removed (restore state)
                    perm.card_sources.extend(removed)
        assert immune_count >= 1, (
            "At least one Digimon should be immune to de-digivolve after "
            "EX10-031 On Play"
        )

    def test_dp_buff_expires_at_end_of_opponent_turn(self, debug_runner):
        """DP buff from On Play expires at the end of opponent's next turn."""
        runner = debug_runner(initial_memory=10)
        target_perm = runner.place_on_field(1, ["BT2-052"])
        base_dp = target_perm.dp

        runner.inject_card(1, "EX10-031", "hand")
        runner.set_phase("Main")
        runner.set_memory(10)
        play_action = runner.find_action("Play DarkKnightmon")
        assert play_action is not None
        runner.execute(play_action)
        _resolve_accepting(runner)

        # DP should be boosted now
        assert target_perm.dp >= base_dp + 3000

        # End our turn, then have the opponent end their turn; buff should expire
        game = runner.game
        # Clear registered modifiers' end_of_opponent_turn by advancing turns
        # End our turn -> opponent turn, then opponent ends turn -> our turn:
        # At the start of our turn, `clear_opponent_turn_expiry(turn_player)`
        # clears modifiers we granted at the start of the next opponent turn end.
        # Simulate by calling `phase_start` of opponent then of us.
        # Easiest path: directly invoke the registry clear.
        game.modifiers.clear_opponent_turn_expiry(game.player1)
        # After clearing, DP should be back to base
        assert target_perm.dp == base_dp, (
            f"DP should return to base after end of opponent turn. "
            f"Base: {base_dp}, now: {target_perm.dp}"
        )

    def test_on_play_is_optional_selection_target_required(self, debug_runner):
        """On Play requires selecting a target; with no own Digimon, nothing happens."""
        runner = debug_runner(initial_memory=10)
        # No pre-placed Digimon for player 1
        runner.inject_card(1, "EX10-031", "hand")
        runner.set_phase("Main")
        runner.set_memory(10)
        play_action = runner.find_action("Play DarkKnightmon")
        assert play_action is not None
        runner.execute(play_action)

        # Resolve anything pending (should be a no-op since no targets apart
        # from the card itself; the card IS a Digimon so it may self-target)
        _resolve_accepting(runner)
        # Sanity: game not broken, EX10-031 is on field
        assert any(
            p.top_card
            and p.top_card.c_entity_base
            and p.top_card.c_entity_base.card_id == "EX10-031"
            for p in runner.game.player1.battle_area
        )


# ── C3 [All Turns][OPT] Play from digivolution cards on leave ─────────────


@pytest.mark.behavioral
class TestEX10031PlayFromDigiOnLeave:
    """Test [All Turns][Once Per Turn] play cost 4 or lower card from digi
    cards when this Digimon would leave the battle area."""

    def test_effect_fires_when_this_digimon_deleted(self, debug_runner):
        """Deleting EX10-031 should trigger the optional play-from-digi effect."""
        runner = debug_runner(initial_memory=10)
        # Stack: BT7-058 (SkullKnightmon, cost 4) -> EX10-031 (top)
        perm = runner.place_on_field(1, ["BT7-058", "EX10-031"])
        # Trigger deletion via direct player.delete_permanent
        player = runner.game.player1
        # After deletion-would-be-deleted effect fires, we should land in a
        # selection phase (optional play); if not declined, the player plays
        # SkullKnightmon from digi cards.
        player.delete_permanent(perm)

        # Resolve all pending selections by accepting
        _resolve_accepting(runner)

        # Check that SkullKnightmon was played onto the field
        skull_on_field = any(
            p.top_card and p.top_card.c_entity_base
            and p.top_card.c_entity_base.card_id == "BT7-058"
            for p in player.battle_area
        )
        assert skull_on_field, (
            "SkullKnightmon should have been played from EX10-031's digi cards "
            f"when it was deleted. Field: "
            f"{[p.top_card.c_entity_base.card_id if p.top_card else '?' for p in player.battle_area]}"
        )

    def test_effect_is_optional_can_decline(self, debug_runner):
        """The play-from-digi effect should be declinable (action 62 legal)."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT7-058", "EX10-031"])
        player = runner.game.player1
        player.delete_permanent(perm)

        # Parked for optional play selection — decline should be legal
        legal = runner.action_mask()
        assert legal, "Should have pending selection after deletion-trigger"
        # If 62 (decline) is available, this effect is optional as required
        assert 62 in legal, (
            f"Effect should be optional (decline=62 legal). Legal: {legal}"
        )

    def test_cannot_play_cost_above_4(self, debug_runner):
        """Only cost <=4 Digimon from digi cards are eligible to play."""
        runner = debug_runner(initial_memory=10)
        # Stack a cost-5 Lv.5 card under EX10-031 (BT2-058 Guardromon Lv.5 cost 5)
        perm = runner.place_on_field(1, ["BT2-058", "EX10-031"])
        player = runner.game.player1
        trash_before = len(player.trash_cards)
        player.delete_permanent(perm)
        # Resolve any pending selections (attempt accept)
        _resolve_accepting(runner)

        # Guardromon (BT2-058) should NOT be on the field as a new permanent
        # (it should have ended up in the trash along with EX10-031's sources)
        guard_on_field = any(
            p.top_card and p.top_card.c_entity_base
            and p.top_card.c_entity_base.card_id == "BT2-058"
            for p in player.battle_area
        )
        assert not guard_on_field, (
            "Cost 5 Guardromon should NOT be playable from EX10-031's digi cards"
        )

    def test_does_not_fire_when_other_permanent_deleted(self, debug_runner):
        """Effect should only fire for THIS Digimon's deletion, not others."""
        runner = debug_runner(initial_memory=10)
        ex10_perm = runner.place_on_field(1, ["BT7-058", "EX10-031"])
        other_perm = runner.place_on_field(1, ["BT2-052"])
        player = runner.game.player1
        # Delete the OTHER digimon
        player.delete_permanent(other_perm)
        # If a selection is pending, it must not be related to EX10-031
        legal = runner.action_mask()
        # Since EX10-031 is still alive, we should be back in Main and no
        # pending selection that plays SkullKnightmon
        _resolve_accepting(runner)
        # BT7-058 should NOT have been played to the field; it should still
        # be in EX10-031's digi cards
        skull_on_field = any(
            p.top_card and p.top_card.c_entity_base
            and p.top_card.c_entity_base.card_id == "BT7-058"
            for p in player.battle_area
        )
        assert not skull_on_field, (
            "Effect should NOT fire when another Digimon is deleted"
        )
        # EX10-031 should still be on field with SkullKnightmon under it
        assert ex10_perm in player.battle_area
        assert len(ex10_perm.card_sources) == 2


# ── C4 Inherited [Opponent's Turn] redirect attack to this Digimon ─────────


@pytest.mark.behavioral
class TestEX10031InheritedRedirect:
    """Inherited: When opp Digimon attacks, may change target to this Digimon."""

    def test_inherited_effect_redirects_attack_to_host(self, debug_runner):
        """When EX10-031 is a digi source under another Digimon, and an opp
        Digimon attacks a player/other target, the host permanent may
        redirect the attack to itself."""
        runner = debug_runner(initial_memory=10)
        # P1 stack: EX10-031 (under) -> BT18-069 Knightmon Lv.5 (host top)
        host_perm = runner.place_on_field(1, ["EX10-031", "BT18-069"])
        # P1 second weaker Digimon that would otherwise be the attack target
        # (not strictly needed — attack can be against player as well)
        # P2: a Lv.4 attacker
        attacker = runner.place_on_field(2, ["BT7-058"])
        # Make P2 the turn player so they can attack
        runner.game.turn_player = runner.game.player2
        runner.game.opponent_player = runner.game.player1
        runner.game.current_phase = GamePhase.Main
        # Attacker must be unsuspended
        attacker.is_suspended = False

        # Enumerate the effects: verify an inherited _is_when_attacked_observer
        # effect exists on the host perm via the EX10-031 source card.
        ex10_src = None
        for cs in host_perm.card_sources:
            if cs.c_entity_base and cs.c_entity_base.card_id == "EX10-031":
                ex10_src = cs
                break
        assert ex10_src is not None
        effects = ex10_src.effect_list(EffectTiming.NoTiming)
        observer_effects = [
            e for e in effects if getattr(e, '_is_when_attacked_observer', False)
        ]
        assert len(observer_effects) >= 1, (
            "Inherited effect must set _is_when_attacked_observer=True for "
            "redirect to fire in combat.py:_fire_opponent_attack_observers. "
            f"Got effects: {[(e.effect_name, getattr(e, '_is_when_attacked_observer', False)) for e in effects]}"
        )

    def test_inherited_effect_only_active_on_opponent_turn(self, debug_runner):
        """Inherited redirect should be gated to opponent's turn."""
        runner = debug_runner(initial_memory=10)
        host_perm = runner.place_on_field(1, ["EX10-031", "BT18-069"])
        # Default: player 1 is turn player — it's our turn, effect should be
        # GATED OFF (condition returns False).
        runner.game.turn_player = runner.game.player1
        runner.game.opponent_player = runner.game.player2
        runner.game.player1.is_my_turn = True
        runner.game.player2.is_my_turn = False

        # Find the inherited observer effect
        ex10_src = None
        for cs in host_perm.card_sources:
            if cs.c_entity_base and cs.c_entity_base.card_id == "EX10-031":
                ex10_src = cs
                break
        effects = ex10_src.effect_list(EffectTiming.NoTiming)
        obs = [e for e in effects
               if getattr(e, '_is_when_attacked_observer', False)]
        assert obs, "Must have inherited observer effect"
        eff = obs[0]
        eff.set_effect_source_permanent(host_perm)
        # On our turn (player1 is turn_player), condition should return False
        ctx = {
            'game': runner.game,
            'player': runner.game.player1,
            'permanent': host_perm,
            'card': ex10_src,
            'attacker': None,
        }
        assert eff.can_use_condition(ctx) is False, (
            "Inherited effect should be gated OFF on our own turn"
        )

    def test_inherited_effect_has_once_per_turn_limit(self, debug_runner):
        """Inherited redirect should be tagged as [Once Per Turn]."""
        runner = debug_runner()
        host_perm = runner.place_on_field(1, ["EX10-031", "BT18-069"])
        ex10_src = None
        for cs in host_perm.card_sources:
            if cs.c_entity_base and cs.c_entity_base.card_id == "EX10-031":
                ex10_src = cs
                break
        effects = ex10_src.effect_list(EffectTiming.NoTiming)
        obs = [e for e in effects
               if getattr(e, '_is_when_attacked_observer', False)]
        assert obs
        eff = obs[0]
        assert eff.max_count_per_turn == 1, (
            f"Inherited redirect should be once per turn, "
            f"got {eff.max_count_per_turn}"
        )

    def test_inherited_effect_is_marked_inherited(self, debug_runner):
        """Effect should be flagged is_inherited_effect=True."""
        runner = debug_runner()
        host_perm = runner.place_on_field(1, ["EX10-031", "BT18-069"])
        ex10_src = None
        for cs in host_perm.card_sources:
            if cs.c_entity_base and cs.c_entity_base.card_id == "EX10-031":
                ex10_src = cs
                break
        effects = ex10_src.effect_list(EffectTiming.NoTiming)
        obs = [e for e in effects
               if getattr(e, '_is_when_attacked_observer', False)]
        assert obs
        assert obs[0].is_inherited_effect, (
            "Redirect effect must be flagged as inherited"
        )

    def test_inherited_effect_redirect_process_switches_target(self, debug_runner):
        """Executing the redirect process should update pending_attack.effective_target."""
        runner = debug_runner(initial_memory=10)
        host_perm = runner.place_on_field(1, ["EX10-031", "BT18-069"])
        attacker = runner.place_on_field(2, ["BT7-058"])
        runner.game.turn_player = runner.game.player2
        runner.game.opponent_player = runner.game.player1
        # Update per-player turn flags (they're stored attributes)
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True
        attacker.is_suspended = False
        # Manually set up a pending attack (player target)
        from engine_py_legacy.engine.game.constants import PendingAttack
        runner.game.pending_attack = PendingAttack(
            attacker=attacker,
            original_target=runner.game.player1,
            effective_target=runner.game.player1,
            without_suspend=False,
            is_vortex=False,
            return_phase=None,
        )

        ex10_src = None
        for cs in host_perm.card_sources:
            if cs.c_entity_base and cs.c_entity_base.card_id == "EX10-031":
                ex10_src = cs
                break
        effects = ex10_src.effect_list(EffectTiming.NoTiming)
        obs = [e for e in effects
               if getattr(e, '_is_when_attacked_observer', False)][0]
        obs.set_effect_source_permanent(host_perm)
        # Condition check: should be True since it's opp turn
        ctx = {
            'game': runner.game,
            'player': runner.game.player1,
            'permanent': host_perm,
            'card': ex10_src,
            'attacker': attacker,
        }
        assert obs.can_use_condition(ctx) is True

        # Fire the process
        assert obs.on_process_callback is not None
        obs.on_process_callback(ctx)

        # pending_attack.effective_target should now be host_perm
        assert runner.game.pending_attack.effective_target is host_perm, (
            f"Attack should be redirected to host perm (BT18-069). "
            f"Current effective_target: {runner.game.pending_attack.effective_target}"
        )
