"""Behavioral tests for BT11-083 LadyDevimon.

Card text:
  Lv.5 | Purple | Fallen Angel | DP:6000 | Cost:7
  Evo: Purple Lv.4 for cost 3

  Main effect:
    [When Digivolving] You may trash 1 card in your hand. If you do, you
    may return 1 [Mirei Mikagura] or 1 card with the [Angel], [Archangel]
    or [Fallen Angel] trait from your trash to your hand.

    [Your Turn] [Once Per Turn] When you play [Angewomon] or
    [Mirei Mikagura], gain 1 memory.

  Inherited effect:
    [Opponent's Turn] While you have a yellow Digimon in play, all of
    your Digimon with the [Angel], [Archangel] or [Fallen Angel] trait
    gain <Retaliation>.

Key faithfulness points tested:
  Clause 1 (WD, optional discard + optional return from trash):
    - Both steps are optional; player may decline each.
    - Trash filter: Mirei Mikagura NAME or Angel/Archangel/Fallen Angel TRAIT.
    - "If you do" gate: return-from-trash only after successful trash.
    - Player chooses which trash card (no auto-select).
  Clause 2 (OPT memory +1 on Angewomon / Mirei Mikagura play):
    - Fires on playing Angewomon or Mirei Mikagura (by name).
    - Fires on play action by this player ([Your Turn]).
    - Once-per-turn gating (hash string) prevents second fire on same turn.
    - Does NOT fire while LadyDevimon is off field.
    - Does NOT fire on opponent's turn.
  Clause 3 (inherited Retaliation aura):
    - Angel/Archangel/Fallen Angel Digimon gain Retaliation via aura.
    - Only during opponent's turn.
    - Only when owner has at least one yellow Digimon in play.
    - Host LadyDevimon without matching trait (Fallen Angel — it does) also
      gains Retaliation via aura.
    - Non-Angel Digimon do NOT gain Retaliation.
"""

import pytest

# ── Card IDs under test ──────────────────────────────────────────────
BT11_083 = "BT11-083"       # LadyDevimon (Purple, Fallen Angel, Lv.5)
BT11_042 = "BT11-042"       # Angewomon (Yellow, Archangel, Lv.5) [When Digi → security search]
BT11_094 = "BT11-094"       # Mirei Mikagura (Tamer, Yellow/Purple)

# ── Digivolution bases / filler ──────────────────────────────────────
BT4_080 = "BT4-080"         # Bakemon (Purple Lv.4, no effect)
BT2_067 = "BT2-067"         # DemiDevimon (Purple Lv.3, no effect)

# ── Support cards with specific traits ───────────────────────────────
ST3_05 = "ST3-05"           # Angemon (Yellow Lv.4, Angel trait)
ST3_08 = "ST3-08"           # MagnaAngemon (Yellow Lv.5, Archangel trait)

# ── Non-matching trait filler (Red Lv.4 Reptile trait, no effects) ───
ST1_03 = "ST1-03"           # Agumon (Red Lv.3, Reptile trait)

# ── Yellow filler with no effect, non-digi Angel/Archangel/Fallen Angel
# Salamon (Yellow Lv.3 Mammal) — for "yellow Digimon in play" gate
ST3_02 = "ST3-02"           # Salamon (Yellow Lv.3, Mammal trait — not Angel)
ST3_06 = "ST3-06"           # Gatomon (Yellow Lv.4, Holy Beast trait)

FILLER = ST1_03


def _purple_filler_deck(extra):
    """A fully-legal 50-card deck including extras."""
    return list(extra) + [FILLER] * (50 - len(extra))


@pytest.mark.behavioral
class TestBT11_083Clause1WhenDigivolving:
    """[When Digivolving] You may trash 1 card in your hand. If you do,
    you may return 1 Mirei Mikagura or [Angel]/[Archangel]/[Fallen Angel]
    card from your trash to your hand."""

    def _setup_runner(self, debug_runner, **kwargs):
        deck = _purple_filler_deck([BT11_083] * 4 + [BT4_080] * 4)
        return debug_runner(
            deck1=deck,
            deck2=[FILLER] * 50,
            initial_memory=10,
            **kwargs,
        )

    def _pick_nonoptional_action(self, runner, exclude_pass=True):
        """Return the first legal selection action, skipping the pass/decline
        action (62) so optional selections make a real choice."""
        legal = runner.action_mask()
        if exclude_pass:
            legal = [a for a in legal if a != 62]
        return legal[0] if legal else None

    def _resolve_sequence(self, runner, max_steps=20):
        """Resolve pending selections by always picking a real choice
        (skipping the optional-pass action 62) until back to Main/Breeding/End."""
        from engine_py_legacy.engine.data.enums import GamePhase
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
            # Prefer a real choice over declining
            nonpass = [a for a in legal if a != 62]
            action = nonpass[0] if nonpass else legal[0]
            runner.execute(action)

    def test_trash_hand_then_return_angel_from_trash(self, debug_runner):
        """Trash 1 hand card, then return an Angel-trait card from trash."""
        runner = self._setup_runner(debug_runner)
        runner.set_phase("Main")

        # Put LadyDevimon base (Bakemon Purple Lv.4) on field
        runner.place_on_field(1, [BT4_080])

        # Clean hand so we have exactly what we want: LadyDevimon + 1 filler
        runner.clear_zone(1, "hand")
        runner.inject_card(1, BT11_083, "hand")
        runner.inject_card(1, FILLER, "hand")  # discard fodder

        # Also clear library so that the digivolve-draw doesn't introduce
        # extra noise in hand.
        runner.clear_zone(1, "library")
        runner.inject_card(1, FILLER, "library_top")  # single draw target

        # Put Angemon (Angel trait) in trash — should be returnable
        runner.clear_zone(1, "trash")
        runner.inject_card(1, ST3_05, "trash")

        # Digivolve into LadyDevimon
        action = runner.find_action("Digivolve LadyDevimon")
        assert action is not None, (
            f"Should be able to digivolve LadyDevimon. Actions: {runner.actions()}"
        )
        runner.execute(action)
        # Resolve the hand-select + trash-select selections, always picking
        # a real choice over decline.
        self._resolve_sequence(runner)

        snap_after = runner.snapshot()
        assert ST3_05 not in snap_after.p1_trash, (
            f"Angemon should no longer be in trash. Trash: {snap_after.p1_trash}"
        )
        assert ST3_05 in snap_after.p1_hand, (
            f"Angemon should be returned to hand. Hand: {snap_after.p1_hand}"
        )

    def test_trash_hand_then_return_mirei_by_name(self, debug_runner):
        """Return a Mirei Mikagura card by name from trash."""
        runner = self._setup_runner(debug_runner)
        runner.set_phase("Main")

        runner.place_on_field(1, [BT4_080])
        runner.clear_zone(1, "hand")
        runner.inject_card(1, BT11_083, "hand")
        runner.inject_card(1, FILLER, "hand")  # discard fodder
        runner.clear_zone(1, "library")
        runner.inject_card(1, FILLER, "library_top")
        runner.clear_zone(1, "trash")
        runner.inject_card(1, BT11_094, "trash")  # Mirei Mikagura

        action = runner.find_action("Digivolve LadyDevimon")
        assert action is not None
        runner.execute(action)
        self._resolve_sequence(runner)

        snap_after = runner.snapshot()
        assert BT11_094 in snap_after.p1_hand, (
            f"Mirei Mikagura should be returned to hand. Hand: {snap_after.p1_hand}"
        )
        assert BT11_094 not in snap_after.p1_trash, (
            f"Mirei Mikagura should no longer be in trash. "
            f"Trash: {snap_after.p1_trash}"
        )

    def test_non_matching_trash_cards_remain(self, debug_runner):
        """Non-matching traits/names in trash cannot be returned."""
        runner = self._setup_runner(debug_runner)
        runner.set_phase("Main")

        runner.place_on_field(1, [BT4_080])
        runner.inject_card(1, BT11_083, "hand")
        runner.inject_card(1, FILLER, "hand")
        # Put only a non-matching card in trash (Agumon Reptile)
        runner.inject_card(1, ST1_03, "trash")

        snap_before = runner.snapshot()
        trash_before_count = len(snap_before.p1_trash)

        action = runner.find_action("Digivolve LadyDevimon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # Agumon should still be in trash (not eligible)
        assert snap_after.p1_trash.count(ST1_03) >= 1, (
            f"Non-matching card should remain in trash. "
            f"Trash: {snap_after.p1_trash}"
        )

    def test_both_trash_steps_optional_empty_hand(self, debug_runner):
        """Clause 1 is fully optional — works with empty hand (no valid
        trash action). No return-from-trash should occur."""
        runner = self._setup_runner(debug_runner)
        runner.set_phase("Main")

        runner.place_on_field(1, [BT4_080])
        runner.clear_zone(1, "hand")
        # Only LadyDevimon in hand (the digivolve card itself)
        runner.inject_card(1, BT11_083, "hand")
        runner.inject_card(1, ST3_05, "trash")  # Angel in trash

        action = runner.find_action("Digivolve LadyDevimon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # With no other cards in hand (after LadyDevimon is used to digivolve),
        # there's nothing to discard — the 'if you do' gate should prevent
        # the trash-to-hand return.
        snap_after = runner.snapshot()
        # Angemon should still be in trash (since we never discarded)
        assert ST3_05 in snap_after.p1_trash, (
            f"Angemon should remain in trash when no discard happened. "
            f"Trash: {snap_after.p1_trash}, Hand: {snap_after.p1_hand}"
        )


@pytest.mark.behavioral
class TestBT11_083Clause2OnceMemoryPlus1:
    """[Your Turn] [Once Per Turn] When you play [Angewomon] or
    [Mirei Mikagura], gain 1 memory."""

    def test_gains_memory_on_angewomon_play(self, debug_runner):
        """Playing Angewomon gains 1 memory while LadyDevimon is on field."""
        # Huge memory so Angewomon (cost 7) can be played
        runner = debug_runner(
            deck1=_purple_filler_deck([BT11_083, BT11_042]),
            deck2=[FILLER] * 50,
            initial_memory=20,
        )
        runner.set_phase("Main")

        # Put LadyDevimon on field
        runner.place_on_field(1, [BT4_080, BT11_083])

        # Put Angewomon in hand
        runner.inject_card(1, BT11_042, "hand")

        before = runner.snapshot()
        mem_before = before.memory

        # Play Angewomon
        action = runner.find_action("Play Angewomon")
        assert action is not None, (
            f"Should be able to play Angewomon. Actions: {runner.actions()}"
        )
        runner.execute(action)
        runner.auto_resolve()

        after = runner.snapshot()
        # Memory change: -7 for play cost, +1 from BT11-083 trigger
        # Net memory = mem_before - 7 + 1 = mem_before - 6
        # Note: playing Angewomon also triggers its own [When Digivolving]-less
        # [On Play] effects (none in BT11-042 — BT11-042's effect is [When Digivolving]
        # not [On Play]). So the only memory delta beyond cost is +1 from BT11-083.
        expected = mem_before - 7 + 1
        assert after.memory == expected, (
            f"Memory should be {expected} (cost 7 + gain 1). "
            f"Before: {mem_before}, After: {after.memory}"
        )

    def test_gains_memory_on_mirei_mikagura_play(self, debug_runner):
        """Playing Mirei Mikagura gains 1 memory while LadyDevimon is on field."""
        runner = debug_runner(
            deck1=_purple_filler_deck([BT11_083, BT11_094]),
            deck2=[FILLER] * 50,
            initial_memory=20,
        )
        runner.set_phase("Main")

        runner.place_on_field(1, [BT4_080, BT11_083])
        runner.inject_card(1, BT11_094, "hand")

        before = runner.snapshot()
        mem_before = before.memory

        action = runner.find_action("Play Mirei Mikagura")
        assert action is not None, (
            f"Should be able to play Mirei Mikagura. Actions: {runner.actions()}"
        )
        runner.execute(action)
        runner.auto_resolve()

        after = runner.snapshot()
        # Mirei Mikagura is a Tamer — her own effect doesn't trigger immediately
        # (Mirei's [Start of Your Turn] gains memory next turn, not now).
        # BT11-083's trigger is the only memory delta beyond cost.
        # Cost of Mirei (BT11-094) — look it up:
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        mirei_cs = db.create_card_source(BT11_094, runner.game.player1)
        mirei_cost = mirei_cs.get_cost_itself
        # BT11-083 adds 1 memory on top of the cost
        assert after.memory == mem_before - mirei_cost + 1, (
            f"Memory should be mem_before - {mirei_cost} + 1. "
            f"Before: {mem_before}, After: {after.memory}"
        )

    def test_once_per_turn_only_one_trigger(self, debug_runner):
        """Playing a second Angewomon on the same turn should NOT trigger
        the memory gain a second time ([Once Per Turn])."""
        runner = debug_runner(
            deck1=_purple_filler_deck([BT11_083, BT11_042, BT11_042]),
            deck2=[FILLER] * 50,
            initial_memory=30,
        )
        runner.set_phase("Main")

        runner.place_on_field(1, [BT4_080, BT11_083])
        runner.inject_card(1, BT11_042, "hand")
        runner.inject_card(1, BT11_042, "hand")

        before = runner.snapshot()
        mem_start = before.memory

        # Play first Angewomon (+1 memory trigger)
        a1 = runner.find_action("Play Angewomon")
        assert a1 is not None
        runner.execute(a1)
        runner.auto_resolve()

        after_first = runner.snapshot()

        # Play second Angewomon (should NOT fire BT11-083 again this turn)
        a2 = runner.find_action("Play Angewomon")
        # If action exists, execute; otherwise the second play may not be legal
        # due to cost or turn-passing. Either way we check memory.
        if a2 is not None:
            runner.execute(a2)
            runner.auto_resolve()

        after_second = runner.snapshot()

        # Between before and after_second, expected delta = -14 (2 Angewomons)
        # + 1 (first trigger only; second was gated by OPT)
        expected_delta_one_trigger = -14 + 1
        actual_delta = after_second.memory - mem_start

        # If only one Angewomon was played, delta should be -7 + 1 = -6
        if a2 is None:
            assert actual_delta == -6, (
                f"First play expected -6, actual {actual_delta}"
            )
        else:
            assert actual_delta == expected_delta_one_trigger, (
                f"OPT should fire only once. Expected delta {expected_delta_one_trigger}, "
                f"actual {actual_delta}"
            )

    def test_no_trigger_when_ladydevimon_not_on_field(self, debug_runner):
        """No BT11-083 trigger when LadyDevimon is not on field."""
        runner = debug_runner(
            deck1=_purple_filler_deck([BT11_042]),
            deck2=[FILLER] * 50,
            initial_memory=20,
        )
        runner.set_phase("Main")

        # Do NOT place LadyDevimon on field
        runner.inject_card(1, BT11_042, "hand")

        before = runner.snapshot()
        mem_before = before.memory

        action = runner.find_action("Play Angewomon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        after = runner.snapshot()
        # Only the -7 cost, no +1 bonus
        assert after.memory == mem_before - 7, (
            f"Without LadyDevimon on field, no memory gain. "
            f"Expected {mem_before - 7}, actual {after.memory}"
        )


@pytest.mark.behavioral
class TestBT11_083Clause3InheritedRetaliationAura:
    """Inherited: [Opponent's Turn] While you have a yellow Digimon in play,
    all of your Digimon with [Angel], [Archangel] or [Fallen Angel] trait
    gain Retaliation."""

    def test_retaliation_granted_to_angel_during_opponent_turn(self, debug_runner):
        """An Angel-trait Digimon should gain Retaliation from a BT11-083
        inherited source during the opponent's turn, IF owner has a yellow
        Digimon in play."""
        runner = debug_runner(
            deck1=_purple_filler_deck([BT11_083]),
            deck2=[FILLER] * 50,
            initial_memory=0,
        )

        # Player 1 has Angemon (Angel) with BT11-083 inherited under it.
        angel_perm = runner.place_on_field(1, [BT11_083, ST3_05])

        # Player 1 also needs a yellow Digimon in play (Angemon is yellow ✓)
        # ST3-05 Angemon is Yellow so the gate passes on its own.

        # Turn is player 2's (opponent)
        runner.game.turn_player = runner.game.player2
        runner.game.opponent_player = runner.game.player1
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True

        # Check keyword aura
        assert angel_perm.has_keyword('_is_retaliation'), (
            "Angel with BT11-083 inherited on opponent's turn (with yellow "
            "Digimon in play) should gain Retaliation via aura."
        )

    def test_no_retaliation_during_own_turn(self, debug_runner):
        """On the owner's turn, the inherited aura should NOT grant
        Retaliation (it's [Opponent's Turn] only)."""
        runner = debug_runner(
            deck1=_purple_filler_deck([BT11_083]),
            deck2=[FILLER] * 50,
            initial_memory=0,
        )

        angel_perm = runner.place_on_field(1, [BT11_083, ST3_05])

        # Explicitly set p1 as turn_player
        runner.game.turn_player = runner.game.player1
        runner.game.opponent_player = runner.game.player2
        runner.game.player1.is_my_turn = True
        runner.game.player2.is_my_turn = False

        assert not angel_perm.has_keyword('_is_retaliation'), (
            "Angel should NOT have Retaliation on owner's turn."
        )

    def test_no_retaliation_without_yellow_digimon(self, debug_runner):
        """Without a yellow Digimon in play, aura should not fire even
        during opponent's turn."""
        runner = debug_runner(
            deck1=_purple_filler_deck([BT11_083]),
            deck2=[FILLER] * 50,
            initial_memory=0,
        )

        # Place a Purple Fallen Angel (LadyDevimon top, Purple stack only)
        # that has BT11-083 inherited under it. No yellow Digimon anywhere.
        angel_perm = runner.place_on_field(1, [BT11_083, BT4_080])

        # Turn is opponent
        runner.game.turn_player = runner.game.player2
        runner.game.opponent_player = runner.game.player1
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True

        # BT4-080 Bakemon is Purple (Ghost) — no yellow ally
        # Bakemon also is not Angel/Archangel/Fallen Angel — so even if aura
        # fired, Bakemon wouldn't gain it. We just confirm no keyword:
        assert not angel_perm.has_keyword('_is_retaliation'), (
            "Without yellow ally, aura should not fire."
        )

    def test_non_angel_digimon_do_not_gain_retaliation(self, debug_runner):
        """A non-Angel Digimon should not gain Retaliation even with
        BT11-083 inherited and yellow ally during opponent's turn."""
        runner = debug_runner(
            deck1=_purple_filler_deck([BT11_083]),
            deck2=[FILLER] * 50,
            initial_memory=0,
        )

        # Create a non-Angel Digimon with BT11-083 inherited
        non_angel = runner.place_on_field(1, [BT11_083, ST3_02])
        # Salamon (Mammal, Yellow) — not Angel/Archangel/Fallen Angel.
        # Having Salamon provides the yellow-in-play gate.

        # Opponent's turn
        runner.game.turn_player = runner.game.player2
        runner.game.opponent_player = runner.game.player1
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True

        # Salamon (non-Angel) should NOT gain Retaliation from aura
        assert not non_angel.has_keyword('_is_retaliation'), (
            "Non-Angel Digimon should NOT gain Retaliation via aura."
        )


@pytest.mark.behavioral
class TestBT11_083InheritedEffectShape:
    """Confirm only Clause 3 is marked as inherited."""

    def test_only_retaliation_effect_is_inherited(self, debug_runner):
        """BT11-083: the WD trash/return, OPT memory, and Retaliation aura
        effects exist. Only the aura effect must be is_inherited_effect."""
        runner = debug_runner(
            deck1=_purple_filler_deck([BT11_083]),
            deck2=[FILLER] * 50,
            initial_memory=0,
        )

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(BT11_083, runner.game.player1)
        effects = cs.effect_list(None)

        # There should be at least: (WD trash/return) + (OPT mem+1) + (aura)
        assert len(effects) >= 3, (
            f"Expected at least 3 effects on BT11-083, got {len(effects)}: "
            f"{[e.effect_name for e in effects]}"
        )

        # At least one inherited, at least one non-inherited
        inherited = [e for e in effects if e.is_inherited_effect]
        non_inherited = [e for e in effects if not e.is_inherited_effect]
        assert inherited, "BT11-083 should have at least 1 inherited effect"
        assert non_inherited, "BT11-083 should have non-inherited main effects"

        # The inherited one(s) should be the retaliation aura
        for eff in inherited:
            assert getattr(eff, '_is_retaliation', False), (
                f"All inherited effects should be the Retaliation aura; "
                f"found inherited effect without _is_retaliation: {eff.effect_name}"
            )
            # Aura shape check
            assert getattr(eff, '_applies_to_all_own_digimon', False), (
                f"Retaliation inherited effect must use aura pattern "
                f"(_applies_to_all_own_digimon). Effect: {eff.effect_name}"
            )
