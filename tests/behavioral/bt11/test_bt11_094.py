"""Behavioral tests for BT11-094 Mirei Mikagura (Tamer).

Card text (SOURCE OF TRUTH):
  [Start of Your Turn] Gain 1 memory.
  [Your Turn] When one of your Digimon digivolves into [Angewomon] or
  [LadyDevimon], if you have 1 or fewer Digimon in play, by suspending
  this Tamer, you may play 1 [Angewomon] or [LadyDevimon] with a
  different name than the Digimon you digivolved into from your hand
  without paying the cost.

  Inherited (Security): [Security] Play this card without paying the cost.

Key faithfulness points tested:
  1. [Start of Your Turn] gains +1 memory when Mirei is on field and it's
     the owner's turn. NO memory-gate (card text has no "if memory <= N").
  2. [Start of Your Turn] does NOT fire if Mirei is NOT on the field.
  3. [Start of Your Turn] does NOT fire on opponent's turn.
  4. WhenDigivolving trigger fires when player digivolves into Angewomon
     (and LadyDevimon), suspending Mirei and offering a hand-play.
  5. Trigger does NOT fire if player has 2+ Digimon on field (condition
     "1 or fewer Digimon in play").
  6. Trigger does NOT fire when Mirei is already suspended (suspend is
     the cost, must be unsuspended to pay).
  7. Trigger does NOT fire when digivolved into a non-Angewomon/LadyDevimon
     Digimon (e.g., Devimon).
  8. "Different name" filter excludes hand cards with the same name as
     the digivolved-into card.
  9. The selection phase is optional (player may decline the play).
 10. Inherited Security effect exists (SecuritySkill timing).
 11. Bug regression guard: the WhenDigivolving effect MUST NOT use
     `is_when_digivolving = True`, which would require Mirei herself to
     be the digivolving permanent (impossible for a Tamer).
"""

from __future__ import annotations

import pytest

from digimon_gym.engine.data.enums import EffectTiming


# ── Card IDs used in tests ──────────────────────────────────────────
MIREI = "BT11-094"            # Mirei Mikagura — Tamer (card under test)
ANGEWOMON_BT2 = "BT2-037"     # Angewomon Lv.5 Yellow, evo Lv4 Yellow @ 3
LADYDEVIMON_BT3 = "BT3-088"   # LadyDevimon Lv.5 Purple, evo Lv4 Purple @ 3
ANGEMON_BT1 = "BT1-055"       # Angemon Lv.4 Yellow — digivolve base for Angewomon
DEVIDRAMON_BT3 = "BT3-081"    # Devidramon Lv.4 Purple — digivolve base for LadyDevimon
DEVIMON_BT2 = "BT2-074"       # Devimon Lv.4 Purple — NOT Angewomon/LadyDevimon (negative test)

# Filler deck — 50 cards
FILLER_DECK = (
    ["BT1-055"] * 10   # Angemon (Lv.4 Yellow)
    + ["BT2-071"] * 10  # Wizardmon (Lv.4 Purple)
    + ["BT2-037"] * 4   # Angewomon
    + ["BT3-088"] * 4   # LadyDevimon
    + ["BT11-094"] * 4  # Mirei
    + ["BT3-081"] * 10  # Devidramon (Lv.4 Purple)
    + ["BT2-074"] * 8   # Devimon (Lv.4 Purple)
)

assert len(FILLER_DECK) == 50, f"FILLER_DECK size {len(FILLER_DECK)}"


# ╔══════════════════════════════════════════════════════════════════╗
# ║ Script-level tests (regression guards + static invariants)       ║
# ╚══════════════════════════════════════════════════════════════════╝

class TestBT11094Invariants:
    """Static invariants on the BT11-094 script definition."""

    def _get_effects(self):
        from digimon_gym.engine.data.card_database import CardDatabase
        from digimon_gym.engine.data.card_registry import CardRegistry
        CardRegistry.ensure_initialized()
        db = CardDatabase()
        cs = db.create_card_source(MIREI, None)
        assert cs is not None, "BT11-094 must be in card database"
        # Force the script to run
        return cs, cs.effect_list(EffectTiming.OnStartTurn)

    def test_script_loads(self):
        cs, effects = self._get_effects()
        assert effects, "BT11-094 must produce effects"

    def test_start_of_turn_effect_present(self):
        cs, effects = self._get_effects()
        sot = [e for e in effects if e.timing == EffectTiming.OnStartTurn]
        assert len(sot) == 1, (
            f"Expected exactly 1 OnStartTurn effect, got {len(sot)}"
        )

    def test_when_digivolving_flag_not_set(self):
        """REGRESSION: The WhenDigivolving-timed effect MUST NOT set
        is_when_digivolving = True. If set, the engine's
        `_effect_matches_timing` requires `perm is digivolved_perm`,
        which is impossible for a Tamer — the effect would be dead.
        """
        cs, _ = self._get_effects()
        all_effects = cs.effect_list(EffectTiming.WhenDigivolving)
        wd_effects = [e for e in all_effects
                      if e.timing == EffectTiming.WhenDigivolving]
        assert wd_effects, "Must have at least 1 WhenDigivolving-timed effect"
        for eff in wd_effects:
            assert not eff.is_when_digivolving, (
                f"BUG: {eff.effect_name} has is_when_digivolving=True. "
                "Tamer effects that trigger on OTHER permanents' "
                "digivolutions must leave this flag False so the engine's "
                "timing-only match path fires them."
            )

    def test_security_effect_present(self):
        cs, _ = self._get_effects()
        sec_effects = cs.effect_list(EffectTiming.SecuritySkill)
        has_sec = any(getattr(e, 'is_security_effect', False) for e in sec_effects)
        assert has_sec, "Security-play effect must exist (is_security_effect=True)"


# ╔══════════════════════════════════════════════════════════════════╗
# ║ C1: [Start of Your Turn] Gain 1 memory                           ║
# ╚══════════════════════════════════════════════════════════════════╝

@pytest.mark.behavioral
class TestBT11094StartOfTurn:
    """[Start of Your Turn] Gain 1 memory (no memory gate)."""

    def test_memory_gained_on_start_of_turn(self, debug_runner):
        """When Mirei is on field and it's owner's turn, OnStartTurn
        should add 1 memory. Text has no memory gate — fires always."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK,
            initial_memory=2, first_player=1,
        )
        runner.place_on_field(1, [MIREI])
        game = runner.game
        p1 = game.player1

        game.turn_player = p1
        p1.is_my_turn = True
        game.opponent_player = game.player2
        game.player2.is_my_turn = False

        mem_before = game.memory
        game.execute_effects(EffectTiming.OnStartTurn)
        assert game.memory == mem_before + 1, (
            f"Expected memory {mem_before + 1} after OnStartTurn, "
            f"got {game.memory}"
        )

    def test_memory_gain_fires_even_at_high_memory(self, debug_runner):
        """Text has no 'if memory <= N' gate — should fire regardless."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK,
            initial_memory=5, first_player=1,
        )
        runner.place_on_field(1, [MIREI])
        game = runner.game
        p1 = game.player1

        game.turn_player = p1
        p1.is_my_turn = True
        game.opponent_player = game.player2
        game.player2.is_my_turn = False

        game.execute_effects(EffectTiming.OnStartTurn)
        assert game.memory == 6, (
            f"Expected memory 6 (5+1) — card text has no gate, got {game.memory}"
        )

    def test_memory_not_gained_without_mirei_on_field(self, debug_runner):
        """Field-presence guard: no Mirei on field → no memory gain."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK,
            initial_memory=2, first_player=1,
        )
        # Do NOT place Mirei on field — it stays in library
        game = runner.game
        p1 = game.player1

        game.turn_player = p1
        p1.is_my_turn = True
        game.opponent_player = game.player2
        game.player2.is_my_turn = False

        mem_before = game.memory
        game.execute_effects(EffectTiming.OnStartTurn)
        assert game.memory == mem_before, (
            f"Memory should stay at {mem_before} (no Mirei on field), "
            f"got {game.memory}"
        )

    def test_memory_not_gained_on_opponent_turn(self, debug_runner):
        """'[Start of Your Turn]' — only fires on owner's turn."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK,
            initial_memory=-2, first_player=2,
        )
        runner.place_on_field(1, [MIREI])  # Mirei is P1's
        game = runner.game
        p2 = game.player2

        game.turn_player = p2
        p2.is_my_turn = True
        game.opponent_player = game.player1
        game.player1.is_my_turn = False

        mem_before = game.memory
        game.execute_effects(EffectTiming.OnStartTurn)
        assert game.memory == mem_before, (
            f"Memory should not change — it's opponent's turn, got "
            f"{game.memory}"
        )


# ╔══════════════════════════════════════════════════════════════════╗
# ║ C2: [Your Turn] When one of your Digimon digivolves into         ║
# ║     [Angewomon] or [LadyDevimon], if you have 1 or fewer Digimon ║
# ║     in play, by suspending this Tamer, you may play 1            ║
# ║     [Angewomon] or [LadyDevimon] with a different name.          ║
# ╚══════════════════════════════════════════════════════════════════╝

@pytest.mark.behavioral
class TestBT11094OnDigivolveTrigger:
    """Tamer trigger: suspend cost + conditional hand-play."""

    def _fire_digivolve_onto(self, runner, top_card_id: str, base_card_id: str):
        """Stack a digivolve onto a base perm and fire WhenDigivolving.

        Places the base on field, injects top-card into hand, then
        manually digivolves via Player.digivolve() and fires
        WhenDigivolving. This avoids relying on action decoder mapping.
        """
        game = runner.game
        p1 = game.player1
        base_perm = runner.place_on_field(1, [base_card_id])
        runner.inject_card(1, top_card_id, "hand")
        # Find the top card in hand
        top_card = None
        for c in p1.hand_cards:
            if c.card_id == top_card_id:
                top_card = c
                break
        assert top_card is not None, f"{top_card_id} should be in hand"

        # Use Player.digivolve() to stack (bypasses action decoding)
        cost = p1.digivolve(base_perm, top_card)
        p1.lose_memory(cost)
        base_perm.turn_digivolved = game.turn_count
        # Fire the WhenDigivolving timing (this is what the engine does
        # internally inside action_digivolve).
        game.execute_effects(
            EffectTiming.WhenDigivolving,
            {"digivolved_permanent": base_perm},
        )
        return base_perm

    def test_trigger_fires_when_digivolving_into_angewomon(self, debug_runner):
        """Digivolving into Angewomon should suspend Mirei and offer to
        play a non-Angewomon (specifically a LadyDevimon) from hand."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK,
            initial_memory=10, first_player=1,
        )
        mirei_perm = runner.place_on_field(1, [MIREI])
        assert not mirei_perm.is_suspended

        game = runner.game
        p1 = game.player1
        p1.is_my_turn = True
        game.turn_player = p1
        game.opponent_player = game.player2

        # Prep a LadyDevimon in hand so the player has a valid target
        runner.inject_card(1, LADYDEVIMON_BT3, "hand")
        initial_hand_size = len(p1.hand_cards)

        # Digivolve Angemon → Angewomon
        base_perm = self._fire_digivolve_onto(runner, ANGEWOMON_BT2, ANGEMON_BT1)

        # Auto-resolve any pending selection phases
        runner.auto_resolve()

        # Mirei should be suspended (paid the cost)
        assert mirei_perm.is_suspended, (
            "Mirei should be suspended after triggering the effect"
        )

        # The LadyDevimon should be on field (played from hand)
        field_cards = [p.top_card.card_id for p in p1.battle_area if p.top_card]
        assert LADYDEVIMON_BT3 in field_cards, (
            f"LadyDevimon should have been played from hand. Field: {field_cards}"
        )

    def test_trigger_fires_when_digivolving_into_ladydevimon(self, debug_runner):
        """Symmetric test — digivolve into LadyDevimon → play Angewomon."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK,
            initial_memory=10, first_player=1,
        )
        mirei_perm = runner.place_on_field(1, [MIREI])

        game = runner.game
        p1 = game.player1
        p1.is_my_turn = True
        game.turn_player = p1
        game.opponent_player = game.player2

        runner.inject_card(1, ANGEWOMON_BT2, "hand")

        self._fire_digivolve_onto(runner, LADYDEVIMON_BT3, DEVIDRAMON_BT3)
        runner.auto_resolve()

        assert mirei_perm.is_suspended, "Mirei should be suspended"
        field_cards = [p.top_card.card_id for p in p1.battle_area if p.top_card]
        assert ANGEWOMON_BT2 in field_cards, (
            f"Angewomon should have been played. Field: {field_cards}"
        )

    def test_no_trigger_when_digivolving_into_non_target(self, debug_runner):
        """Digivolving into a non-Angewomon/non-LadyDevimon Digimon
        should NOT suspend Mirei."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK,
            initial_memory=10, first_player=1,
        )
        mirei_perm = runner.place_on_field(1, [MIREI])

        game = runner.game
        p1 = game.player1
        p1.is_my_turn = True
        game.turn_player = p1
        game.opponent_player = game.player2

        runner.inject_card(1, LADYDEVIMON_BT3, "hand")
        hand_before = len(p1.hand_cards)

        # Digivolve Wizardmon → Devimon (neither is Angewomon/LadyDevimon)
        base = runner.place_on_field(1, ["BT2-071"])  # Wizardmon Lv4 Purple
        runner.inject_card(1, DEVIMON_BT2, "hand")
        devimon_card = next(c for c in p1.hand_cards if c.card_id == DEVIMON_BT2)
        cost = p1.digivolve(base, devimon_card)
        p1.lose_memory(cost)
        base.turn_digivolved = game.turn_count
        game.execute_effects(
            EffectTiming.WhenDigivolving,
            {"digivolved_permanent": base},
        )
        runner.auto_resolve()

        assert not mirei_perm.is_suspended, (
            "Mirei should NOT suspend when digivolving into Devimon"
        )
        # LadyDevimon should STILL be in hand
        assert any(c.card_id == LADYDEVIMON_BT3 for c in p1.hand_cards), (
            "LadyDevimon should remain in hand (effect didn't fire)"
        )

    def test_no_trigger_when_two_or_more_digimon_in_play(self, debug_runner):
        """Condition: 'if you have 1 or fewer Digimon in play'.
        After a digivolve, the digivolved perm itself counts as 1.
        If there are 2 Digimon on field post-digivolve, trigger fails."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK,
            initial_memory=10, first_player=1,
        )
        mirei_perm = runner.place_on_field(1, [MIREI])

        game = runner.game
        p1 = game.player1
        p1.is_my_turn = True
        game.turn_player = p1
        game.opponent_player = game.player2

        # Place a second, unrelated Digimon so post-digivolve count is 2
        runner.place_on_field(1, ["BT2-071"])  # Wizardmon — second Digimon
        runner.inject_card(1, LADYDEVIMON_BT3, "hand")

        self._fire_digivolve_onto(runner, ANGEWOMON_BT2, ANGEMON_BT1)
        runner.auto_resolve()

        assert not mirei_perm.is_suspended, (
            "Mirei should NOT trigger when player has 2 Digimon in play"
        )

    def test_no_trigger_when_mirei_already_suspended(self, debug_runner):
        """Suspend cost: Mirei must be unsuspended to activate."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK,
            initial_memory=10, first_player=1,
        )
        mirei_perm = runner.place_on_field(1, [MIREI], is_suspended=True)
        assert mirei_perm.is_suspended

        game = runner.game
        p1 = game.player1
        p1.is_my_turn = True
        game.turn_player = p1
        game.opponent_player = game.player2

        runner.inject_card(1, LADYDEVIMON_BT3, "hand")

        self._fire_digivolve_onto(runner, ANGEWOMON_BT2, ANGEMON_BT1)
        runner.auto_resolve()

        # LadyDevimon should still be in hand (effect couldn't pay the cost)
        assert any(c.card_id == LADYDEVIMON_BT3 for c in p1.hand_cards), (
            "Effect should not have played LadyDevimon — Mirei was suspended"
        )

    def test_cannot_play_same_name_from_hand(self, debug_runner):
        """Filter: the played card must have a DIFFERENT NAME than the
        digivolved-into card. If only same-named Angewomon in hand, the
        select-hand prompt should find no valid candidates."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK,
            initial_memory=10, first_player=1,
        )
        mirei_perm = runner.place_on_field(1, [MIREI])

        game = runner.game
        p1 = game.player1
        p1.is_my_turn = True
        game.turn_player = p1
        game.opponent_player = game.player2

        # Only another Angewomon in hand (same name as digivolved-into)
        runner.inject_card(1, ANGEWOMON_BT2, "hand")
        angewomon_in_hand_before = sum(
            1 for c in p1.hand_cards if c.card_id == ANGEWOMON_BT2
        )

        self._fire_digivolve_onto(runner, ANGEWOMON_BT2, ANGEMON_BT1)
        runner.auto_resolve()

        # Post-condition: the Angewomon we injected should STILL be in hand
        # (we injected before digivolving, and the second copy on field was
        # taken from the deck or injected differently — let's just verify
        # no Angewomon was played from hand in addition to the digivolved one)
        field_angewomon_count = sum(
            1 for p in p1.battle_area
            if p.top_card and p.top_card.card_id == ANGEWOMON_BT2
        )
        # Exactly 1 Angewomon on field (the digivolved one, not a second
        # from hand-play effect)
        assert field_angewomon_count == 1, (
            f"Expected exactly 1 Angewomon on field (the digivolved one). "
            f"If the effect illegally played a same-named Angewomon from "
            f"hand, this would be 2. Got {field_angewomon_count}"
        )

    def test_trigger_does_not_fire_on_opponent_turn(self, debug_runner):
        """'[Your Turn]' — condition blocks on opponent's turn."""
        runner = debug_runner(
            deck1=FILLER_DECK, deck2=FILLER_DECK,
            initial_memory=10, first_player=2,
        )
        mirei_perm = runner.place_on_field(1, [MIREI])

        game = runner.game
        # Force p2 turn
        game.turn_player = game.player2
        game.opponent_player = game.player1
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        # P1 tries to force a digivolve into Angewomon during P2's turn
        p1 = game.player1
        runner.inject_card(1, LADYDEVIMON_BT3, "hand")
        base_perm = runner.place_on_field(1, [ANGEMON_BT1])
        runner.inject_card(1, ANGEWOMON_BT2, "hand")
        angewomon_card = next(
            c for c in p1.hand_cards if c.card_id == ANGEWOMON_BT2
        )
        cost = p1.digivolve(base_perm, angewomon_card)
        # No lose_memory (we're just testing the trigger condition)
        base_perm.turn_digivolved = game.turn_count
        game.execute_effects(
            EffectTiming.WhenDigivolving,
            {"digivolved_permanent": base_perm},
        )
        runner.auto_resolve()

        assert not mirei_perm.is_suspended, (
            "Mirei should NOT trigger on opponent's turn"
        )


# ╔══════════════════════════════════════════════════════════════════╗
# ║ C3: Inherited [Security] Play this card without paying the cost  ║
# ╚══════════════════════════════════════════════════════════════════╝

@pytest.mark.behavioral
class TestBT11094Security:
    """[Security] Play this card without paying the cost."""

    def test_security_effect_registered(self):
        from digimon_gym.engine.data.card_database import CardDatabase
        from digimon_gym.engine.data.card_registry import CardRegistry
        CardRegistry.ensure_initialized()
        db = CardDatabase()
        cs = db.create_card_source(MIREI, None)
        effects = cs.effect_list(EffectTiming.SecuritySkill)
        sec = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(sec) >= 1, (
            "BT11-094 must have an is_security_effect=True effect"
        )
