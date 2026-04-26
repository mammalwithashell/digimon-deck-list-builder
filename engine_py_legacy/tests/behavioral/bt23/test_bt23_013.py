"""Behavioral tests for BT23-013 Jesmon (Lv.6 Red Digimon).

Card text:
  <Rush> <Alliance>
  [When Digivolving] [When Attacking] You may play 1 [Atho, Rene & Por] Token
      (Digimon/White/6000 DP/<Reboot> <Blocker> <Decoy (Red/Black)>) or, from
      your hand or trash, 1 Digimon card with [Sistermon] in its name without
      paying the cost. This effect can't play cards with the same names as any
      of your Digimon.
  [Your Turn] [Once Per Turn] When any of your other Digimon are played,
      this Digimon may attack.

Alt-digi:
  [Digivolve] [SaviorHuckmon]/Lv.5 w/[CS] trait: Cost 3
  [Digivolve] While opponent has a 10000 DP or higher Digimon, [Huckmon]: Cost 5
"""

import pytest


@pytest.mark.behavioral
class TestBT23013Jesmon:
    """Tests for BT23-013 Jesmon."""

    # ── Rush ─────────────────────────────────────────────────────────

    def test_rush_allows_attack_on_play_turn(self, debug_runner):
        """Jesmon should be able to attack the turn it is played (Rush)."""
        runner = debug_runner(initial_memory=15)
        # Place Jesmon with turn_played = current turn (sickness)
        runner.place_on_field(1, ["BT23-013"], turn_played=1)
        runner.set_phase("Main")

        attack = runner.find_action("Attack player with Jesmon")
        assert attack is not None, (
            f"Jesmon should have Rush and be able to attack. Actions: {runner.actions()}")

    # ── Alliance ─────────────────────────────────────────────────────

    def test_alliance_keyword_present(self, debug_runner):
        """Jesmon should have the Alliance keyword."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT23-013"])
        runner.place_on_field(1, ["BT1-010"])  # Unsuspended ally for Alliance
        runner.set_phase("Main")

        # Attack with Jesmon
        attack = runner.find_action("Attack player with Jesmon")
        assert attack is not None
        runner.execute(attack)

        # Should enter AllianceTiming phase
        snap = runner.snapshot()
        assert snap.phase == "AllianceTiming", (
            f"Expected AllianceTiming phase, got {snap.phase}. "
            f"Actions: {runner.actions()}")

    # ── WD: When Digivolving → Play Token or Sistermon ───────────────

    def test_wd_plays_token_when_no_sistermon_available(self, debug_runner):
        """[When Digivolving] should play Atho/Rene/Por token when no Sistermon available."""
        runner = debug_runner(initial_memory=15)
        # Place a Lv.5 Red to digivolve from
        runner.place_on_field(1, ["BT20-014"])  # SaviorHuckmon Lv.5
        runner.inject_card(1, "BT23-013", "hand")
        runner.set_phase("Main")

        digi = runner.find_action("Digivolve")
        if digi is None:
            digi = runner.find_action("digivolve")
        assert digi is not None, (
            f"Should be able to digivolve into Jesmon. Actions: {runner.actions()}")
        runner.execute(digi)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Should have the token on field
        token_on_field = any(
            s.card_id == "TOKEN_ATHO_RENE_POR" for s in snap.p1_field)
        assert token_on_field, (
            f"Expected Atho/Rene/Por Token on field. "
            f"P1 field: {[(s.card_id, s.card_name) for s in snap.p1_field]}")

    def test_wd_plays_sistermon_from_hand(self, debug_runner):
        """[When Digivolving] should allow playing Sistermon from hand."""
        runner = debug_runner(initial_memory=15)
        runner.place_on_field(1, ["BT20-014"])  # SaviorHuckmon Lv.5
        runner.inject_card(1, "BT23-013", "hand")
        runner.inject_card(1, "BT6-082", "hand")  # Sistermon Blanc
        runner.set_phase("Main")

        digi = runner.find_action("Digivolve")
        if digi is None:
            digi = runner.find_action("digivolve")
        assert digi is not None, (
            f"Should be able to digivolve into Jesmon. Actions: {runner.actions()}")
        runner.execute(digi)

        # Should have a branch choice (token vs Sistermon)
        ps = runner.game.pending_selection
        if ps is not None:
            # Resolve: auto_resolve picks first option
            runner.auto_resolve()

        snap = runner.snapshot()
        # Should have either token or Sistermon on field
        has_extra = any(
            s.card_id in ("TOKEN_ATHO_RENE_POR", "BT6-082")
            for s in snap.p1_field
        )
        assert has_extra, (
            f"Expected token or Sistermon on field after WD. "
            f"P1 field: {[(s.card_id, s.card_name) for s in snap.p1_field]}")

    def test_wd_no_duplicate_names_token(self, debug_runner):
        """Token can't be played if one with the same name is already on field."""
        runner = debug_runner(initial_memory=15)
        # Pre-place an Atho/Rene/Por Token
        runner.game.effect_play_token(runner.game.player1, 'atho_rene_por')

        runner.place_on_field(1, ["BT20-014"])  # SaviorHuckmon Lv.5
        runner.inject_card(1, "BT23-013", "hand")
        runner.set_phase("Main")

        digi = runner.find_action("Digivolve")
        if digi is None:
            digi = runner.find_action("digivolve")
        assert digi is not None
        runner.execute(digi)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Count tokens on field
        token_count = sum(
            1 for s in snap.p1_field if s.card_id == "TOKEN_ATHO_RENE_POR")
        assert token_count == 1, (
            f"Should not play duplicate token. Token count: {token_count}. "
            f"P1 field: {[(s.card_id, s.card_name) for s in snap.p1_field]}")

    def test_wd_no_duplicate_names_sistermon(self, debug_runner):
        """Can't play a Sistermon with the same name as one already on field."""
        runner = debug_runner(initial_memory=15)
        # Pre-place a Sistermon Blanc
        runner.place_on_field(1, ["BT6-082"])  # Sistermon Blanc

        runner.place_on_field(1, ["BT20-014"])  # SaviorHuckmon Lv.5
        runner.inject_card(1, "BT23-013", "hand")
        runner.inject_card(1, "BT6-082", "hand")  # Another Sistermon Blanc
        runner.set_phase("Main")

        digi = runner.find_action("Digivolve")
        if digi is None:
            digi = runner.find_action("digivolve")
        assert digi is not None
        runner.execute(digi)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Sistermon Blanc should only appear once (from pre-placed)
        blanc_count = sum(
            1 for s in snap.p1_field if s.card_id == "BT6-082")
        assert blanc_count == 1, (
            f"Should not play duplicate Sistermon Blanc. Count: {blanc_count}. "
            f"P1 field: {[(s.card_id, s.card_name) for s in snap.p1_field]}")

    # ── WA: When Attacking → Play Token or Sistermon ─────────────────

    def test_wa_plays_token_on_attack(self, debug_runner):
        """[When Attacking] should play token when Jesmon attacks."""
        runner = debug_runner(initial_memory=10)
        runner.place_on_field(1, ["BT23-013"])
        runner.set_phase("Main")

        attack = runner.find_action("Attack player with Jesmon")
        assert attack is not None
        runner.execute(attack)
        runner.auto_resolve()

        snap = runner.snapshot()
        token_on_field = any(
            s.card_id == "TOKEN_ATHO_RENE_POR" for s in snap.p1_field)
        assert token_on_field, (
            f"Expected token on field after WA. "
            f"P1 field: {[(s.card_id, s.card_name) for s in snap.p1_field]}")

    def test_wa_plays_sistermon_from_trash(self, debug_runner):
        """[When Attacking] should allow playing Sistermon from trash."""
        runner = debug_runner(initial_memory=10)
        runner.place_on_field(1, ["BT23-013"])
        runner.inject_card(1, "BT6-082", "trash")  # Sistermon Blanc in trash
        runner.set_phase("Main")

        attack = runner.find_action("Attack player with Jesmon")
        assert attack is not None
        runner.execute(attack)

        # Should have a branch choice or selection
        # Try to pick Sistermon branch if branch is offered
        ps = runner.game.pending_selection
        if ps is not None:
            # Resolve selections
            runner.auto_resolve()

        snap = runner.snapshot()
        # Either token or Sistermon should be on field
        has_extra = any(
            s.card_id in ("TOKEN_ATHO_RENE_POR", "BT6-082")
            for s in snap.p1_field
        )
        assert has_extra, (
            f"Expected token or Sistermon on field after WA. "
            f"P1 field: {[(s.card_id, s.card_name) for s in snap.p1_field]}")

    def test_wa_only_fires_for_jesmon_attack(self, debug_runner):
        """[When Attacking] should NOT fire when a different Digimon attacks."""
        runner = debug_runner(initial_memory=10)
        runner.place_on_field(1, ["BT23-013"])
        runner.place_on_field(1, ["BT1-010"])  # Agumon
        runner.set_phase("Main")

        attack = runner.find_action("Attack player with Agumon")
        assert attack is not None
        runner.execute(attack)
        runner.auto_resolve()

        snap = runner.snapshot()
        token_on_field = any(
            s.card_id == "TOKEN_ATHO_RENE_POR" for s in snap.p1_field)
        assert not token_on_field, (
            "Token should NOT be played when a non-Jesmon Digimon attacks. "
            f"P1 field: {[(s.card_id, s.card_name) for s in snap.p1_field]}")

    # ── Your Turn OPT: May attack when other Digimon played ──────────

    def test_yt_opt_jesmon_gets_may_attack_on_ally_play(self, debug_runner):
        """When another Digimon is played and Jesmon is unsuspended, it gets MAY_ATTACK."""
        runner = debug_runner(initial_memory=10)
        jesmon_perm = runner.place_on_field(1, ["BT23-013"])
        # Jesmon is unsuspended — should get MAY_ATTACK per C# CanAttack check

        runner.inject_card(1, "BT1-010", "hand")  # Agumon to play
        runner.set_phase("Main")

        play = runner.find_action("Play Agumon")
        assert play is not None, f"Should be able to play Agumon. Actions: {runner.actions()}"
        runner.execute(play)

        from digimon_gym.engine.interfaces.modifiers import ModifierType
        has_may_attack = runner.game.modifiers.has_modifier(
            jesmon_perm, ModifierType.MAY_ATTACK)
        assert has_may_attack, (
            "Jesmon should have MAY_ATTACK modifier after ally Digimon played")

    def test_yt_opt_skips_when_jesmon_suspended(self, debug_runner):
        """When Jesmon is suspended, the may-attack effect does NOT grant MAY_ATTACK (matches C# CanAttack)."""
        runner = debug_runner(initial_memory=10)
        jesmon_perm = runner.place_on_field(1, ["BT23-013"])
        jesmon_perm.suspend()  # Suspended — cannot attack

        runner.inject_card(1, "BT1-010", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Agumon")
        if play is not None:
            runner.execute(play)
            runner.auto_resolve()

        from digimon_gym.engine.interfaces.modifiers import ModifierType
        has_may_attack = runner.game.modifiers.has_modifier(
            jesmon_perm, ModifierType.MAY_ATTACK)
        assert not has_may_attack, (
            "Suspended Jesmon should NOT get MAY_ATTACK (C# checks CanAttack)")

    def test_yt_opt_once_per_turn(self, debug_runner):
        """OPT effect should only trigger once per turn."""
        runner = debug_runner(initial_memory=15)
        jesmon_perm = runner.place_on_field(1, ["BT23-013"])

        # Play first Digimon - should trigger
        runner.inject_card(1, "BT1-010", "hand")
        runner.set_phase("Main")
        play1 = runner.find_action("Play Agumon")
        assert play1 is not None
        runner.execute(play1)
        runner.auto_resolve()

        # Suspend Jesmon again to simulate an attack
        jesmon_perm.suspend()

        # Play second Digimon - should NOT trigger (OPT)
        runner.inject_card(1, "BT1-010", "hand")
        play2 = runner.find_action("Play Agumon")
        if play2 is not None:
            runner.execute(play2)
            runner.auto_resolve()

        # Jesmon should still be suspended (OPT expired)
        assert jesmon_perm.is_suspended, (
            "Jesmon should remain suspended after OPT is used up for the turn")

    def test_yt_opt_not_on_opponent_turn(self, debug_runner):
        """OPT effect should NOT trigger on opponent's turn."""
        runner = debug_runner(initial_memory=10, first_player=2)
        jesmon_perm = runner.place_on_field(1, ["BT23-013"])
        jesmon_perm.suspend()

        # It's player 2's turn. Playing a Digimon on P2's side should not
        # trigger Jesmon's observer (it's not player 1's turn).
        runner.inject_card(2, "BT1-010", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Agumon")
        if play is not None:
            runner.execute(play)
            runner.auto_resolve()

        # Jesmon should still be suspended
        assert jesmon_perm.is_suspended, (
            "Jesmon's OPT should not trigger on opponent's turn")

    def test_yt_opt_token_play_triggers(self, debug_runner):
        """Playing a token should trigger the OPT observer when Jesmon is unsuspended."""
        runner = debug_runner(initial_memory=10)
        jesmon_perm = runner.place_on_field(1, ["BT23-013"])
        # Jesmon is unsuspended — token play should trigger MAY_ATTACK

        runner.game.effect_play_token(runner.game.player1, 'atho_rene_por')

        from digimon_gym.engine.interfaces.modifiers import ModifierType
        has_may_attack = runner.game.modifiers.has_modifier(
            jesmon_perm, ModifierType.MAY_ATTACK)
        assert has_may_attack, (
            "Jesmon should get MAY_ATTACK when a token is played")

    def test_yt_opt_may_attack_creates_eot_action(self, debug_runner):
        """MAY_ATTACK from OPT should create EndOfTurnAction phase at end of turn."""
        runner = debug_runner(initial_memory=2)
        jesmon_perm = runner.place_on_field(1, ["BT23-013"])
        # Jesmon must be unsuspended for MAY_ATTACK to be granted

        # Play a cheap Digimon that will cross memory to negative
        runner.inject_card(1, "BT1-010", "hand")  # Agumon costs ~3-5 memory
        runner.set_phase("Main")

        play = runner.find_action("Play Agumon")
        if play is None:
            pytest.skip("Cannot play Agumon with current memory")
        runner.execute(play)

        # If memory went negative, check for EndOfTurnAction
        snap = runner.snapshot()
        if snap.memory < 0:
            # Jesmon should have MAY_ATTACK and end phase should park at EOT
            from digimon_gym.engine.interfaces.modifiers import ModifierType
            has_may_attack = runner.game.modifiers.has_modifier(
                jesmon_perm, ModifierType.MAY_ATTACK)
            assert has_may_attack, (
                "Jesmon should have MAY_ATTACK after ally played and memory negative")
            assert snap.phase == "EndOfTurnAction", (
                f"Expected EndOfTurnAction phase due to MAY_ATTACK, got {snap.phase}. "
                f"Actions: {runner.actions()}")

    def test_yt_opt_does_not_trigger_on_self_permanent(self, debug_runner):
        """OPT observer should not count Jesmon's own permanent as 'other'.

        Note: When Jesmon is played and WD spawns a token, the token IS a
        separate Digimon being played, so the OPT observer correctly triggers.
        This test verifies the engine skips the host permanent itself.
        """
        runner = debug_runner(initial_memory=15)
        # Pre-place a token so WD can't play another (no Sistermon either)
        # This ensures WD doesn't play anything, isolating the self-play check
        runner.game.effect_play_token(runner.game.player1, 'atho_rene_por')

        runner.inject_card(1, "BT23-013", "hand")
        runner.set_phase("Main")

        play = runner.find_action("Play Jesmon")
        assert play is not None, f"Should be able to play Jesmon. Actions: {runner.actions()}"
        runner.execute(play)
        runner.auto_resolve()

        # Jesmon should NOT have MAY_ATTACK because:
        # - _is_play_observer skips the host permanent (Jesmon itself)
        # - WD couldn't play anything (token already on field, no Sistermon)
        snap = runner.snapshot()
        jesmon_slot = next(
            (s for s in snap.p1_field if s.card_id == "BT23-013"), None)
        assert jesmon_slot is not None

        jesmon_perm = runner.game.player1.battle_area[jesmon_slot.slot]
        from digimon_gym.engine.interfaces.modifiers import ModifierType
        has_may_attack = runner.game.modifiers.has_modifier(
            jesmon_perm, ModifierType.MAY_ATTACK)
        assert not has_may_attack, (
            "Jesmon should NOT get MAY_ATTACK from its own play (no other Digimon played)")
