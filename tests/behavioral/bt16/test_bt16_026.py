"""Behavioral tests for BT16-026 Vikemon (Ace).

Card text (from cards.json / C# source BT16_026.cs):
  Lv.6 | Blue/Black | Beastkin | DP 12000 | Cost 7 | Attribute: Free
  [Digivolve] [Shakkoumon]/[Zudomon]: Cost 3
  [Hand] [Counter] <Blast Digivolve>
  [On Play] [When Digivolving] <De-Digivolve 2> 1 of your opponent's Digimon.
      Then, none of their Digimon with 1 or fewer digivolution cards can suspend
      until the end of their turn.
  [When Attacking] Delete 1 of your opponent's Digimon with 1 or fewer
      digivolution cards.
  Inherited: Ace Overflow <-4>

Key faithfulness points tested:
  1. Blast Digivolve flag
  2. Alt-digi from Shakkoumon for cost 3
  3. Alt-digi from Zudomon for cost 3 (separate effect)
  4. [On Play] De-Digivolve 2 targeting any opponent Digimon (mandatory, select 1)
  5. [On Play] Then: CANNOT_SUSPEND via register_modifier on opponent Digimon
     with <= 1 digivolution cards (i.e., len(card_sources) <= 2)
  6. [When Digivolving] Same as On Play
  7. [When Attacking] Delete 1 opponent Digimon with <= 1 digivolution cards
  8. Ace Overflow <-4> inherited effect
  9. CANNOT_SUSPEND uses register_modifier (not direct attribute mutation)
  10. Digivolution card count matches DCGO semantics (cards below top, not full stack)
"""

import pytest
from digimon_gym.engine.interfaces.modifiers import ModifierType
from digimon_gym.engine.data.enums import EffectTiming


# Card IDs used in tests
BT16_026 = "BT16-026"          # Card under test (Vikemon, Lv.6 Blue/Black)
AGUMON_ST1 = "ST1-03"          # Agumon (Lv.3, Red)
GREYMON_ST1 = "ST1-07"         # Greymon (Lv.4, Red)
METALGREYMON_ST1 = "ST1-09"    # MetalGreymon (Lv.5, Red)


# ── Clause 1: Blast Digivolve ──────────────────────────────────────────


@pytest.mark.behavioral
class TestBT16026BlastDigivolve:
    """Blast Digivolve flag should be set on an effect."""

    def test_has_blast_digivolve_flag(self, debug_runner):
        """Should have _is_blast_digivolve flag set on an effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [BT16_026])

        top = perm.top_card
        all_effects = top.effect_list(None)
        blast_effects = [
            e for e in all_effects if getattr(e, '_is_blast_digivolve', False)
        ]
        assert len(blast_effects) >= 1, "Should have at least 1 Blast Digivolve effect"


# ── Clauses 2-3: Alt-digi from Shakkoumon or Zudomon ───────────────────


@pytest.mark.behavioral
class TestBT16026AltDigi:
    """Alt-digi from [Shakkoumon] or [Zudomon] for cost 3."""

    def test_has_alt_digi_shakkoumon(self, debug_runner):
        """Should have alt-digi from Shakkoumon for cost 3."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [BT16_026])

        top = perm.top_card
        all_effects = top.effect_list(None)
        alt_digi = [
            e for e in all_effects
            if getattr(e, '_alt_digi_cost', None) is not None
            and getattr(e, '_alt_digi_name', '') == "Shakkoumon"
        ]
        assert len(alt_digi) >= 1, "Should have an alt-digi effect for Shakkoumon"
        assert alt_digi[0]._alt_digi_cost == 3, "Alt-digi cost from Shakkoumon should be 3"

    def test_has_alt_digi_zudomon(self, debug_runner):
        """Should have alt-digi from Zudomon for cost 3."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [BT16_026])

        top = perm.top_card
        all_effects = top.effect_list(None)
        alt_digi = [
            e for e in all_effects
            if getattr(e, '_alt_digi_cost', None) is not None
            and getattr(e, '_alt_digi_name', '') == "Zudomon"
        ]
        assert len(alt_digi) >= 1, "Should have an alt-digi effect for Zudomon"
        assert alt_digi[0]._alt_digi_cost == 3, "Alt-digi cost from Zudomon should be 3"


# ── Clauses 4-5: [On Play] De-Digivolve 2 + CANNOT_SUSPEND ────────────


@pytest.mark.behavioral
class TestBT16026OnPlay:
    """[On Play] De-Digivolve 2 one opponent Digimon, then can't suspend."""

    def test_on_play_effect_exists(self, debug_runner):
        """Should have an On Play effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [BT16_026])

        top = perm.top_card
        all_effects = top.effect_list(None)
        on_play_effects = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ]
        assert len(on_play_effects) >= 1, "Should have at least 1 On Play effect"

    def test_on_play_de_digivolves_opponent(self, debug_runner):
        """[On Play] De-Digivolve 2 should remove 2 cards from opponent's stack."""
        runner = debug_runner(initial_memory=10)

        # Place Vikemon on P1 field
        perm = runner.place_on_field(1, [BT16_026])

        # Place opponent Digimon with 3 cards in stack (Lv3 + Lv4 + Lv5)
        opp_perm = runner.place_on_field(2, [AGUMON_ST1, GREYMON_ST1, METALGREYMON_ST1])
        stack_before = len(opp_perm.card_sources)
        assert stack_before == 3, "Opponent should start with 3 cards in stack"

        game = runner.game
        top = perm.top_card
        on_play_effects = [
            e for e in top.effect_list(None)
            if getattr(e, 'is_on_play', False)
        ]
        assert len(on_play_effects) >= 1

        # Fire the On Play effect directly
        on_play_effects[0].on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        # De-Digivolve 2 removes 2 cards from top of stack (above base)
        stack_after = len(opp_perm.card_sources)
        assert stack_after == stack_before - 2, (
            f"De-Digivolve 2 should remove 2 cards from stack. "
            f"Before: {stack_before}, After: {stack_after}"
        )

    def test_on_play_de_digivolve_targets_any_opponent_digimon(self, debug_runner):
        """De-Digivolve 2 targets any opponent Digimon (no digi-card count filter)."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, [BT16_026])

        # Opponent with only 1 card in stack (can't de-digivolve below Lv.3 base)
        opp_single = runner.place_on_field(2, [AGUMON_ST1])

        game = runner.game
        top = perm.top_card
        on_play_effects = [
            e for e in top.effect_list(None)
            if getattr(e, 'is_on_play', False)
        ]

        # Should still fire and target the single-card Digimon
        on_play_effects[0].on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        # Single-card Digimon can't lose any cards (can't go below 1)
        assert len(opp_single.card_sources) == 1, (
            "Single-card Digimon should still have 1 card (can't de-digivolve below base)"
        )

    def test_on_play_cannot_suspend_uses_register_modifier(self, debug_runner):
        """The 'can't suspend' clause must use register_modifier with
        ModifierType.CANNOT_SUSPEND, not direct attribute mutation."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, [BT16_026])

        # Place opponent Digimon with <= 1 digivolution card (2 total cards: top + 1 under)
        opp_target = runner.place_on_field(2, [AGUMON_ST1, GREYMON_ST1])

        game = runner.game
        top = perm.top_card
        on_play_effects = [
            e for e in top.effect_list(None)
            if getattr(e, 'is_on_play', False)
        ]

        on_play_effects[0].on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        # After de-digivolve 2, the opponent's stack has 1 card (base only)
        # which means 0 digi-cards (below top), so <= 1 applies
        has_mod = game.modifiers.has_modifier(opp_target, ModifierType.CANNOT_SUSPEND)
        assert has_mod, (
            "Opponent Digimon with <= 1 digivolution card should have "
            "CANNOT_SUSPEND modifier registered via game.register_modifier"
        )

    def test_on_play_cannot_suspend_only_affects_few_digi_cards(self, debug_runner):
        """CANNOT_SUSPEND should only apply to opponent Digimon with <= 1
        digivolution cards (cards below top), not those with 2+."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, [BT16_026])

        # Place a small Digimon first so auto_resolve de-digivolves it
        opp_small = runner.place_on_field(2, [AGUMON_ST1])

        # Then place a big Digimon with 5-card stack (4 digi-cards).
        # Even if de-digivolve targets this instead, 5 - 2 = 3 cards (2 digi-cards > 1).
        opp_big = runner.place_on_field(2, [
            AGUMON_ST1, GREYMON_ST1, METALGREYMON_ST1, AGUMON_ST1, GREYMON_ST1
        ])  # 5-card stack, 4 digi-cards

        game = runner.game

        top = perm.top_card
        on_play_effects = [
            e for e in top.effect_list(None)
            if getattr(e, 'is_on_play', False)
        ]

        on_play_effects[0].on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        # opp_big should have >= 2 digi-cards even after possible de-digivolve
        # (worst case: 5 - 2 = 3 cards = 2 digi-cards), should NOT have modifier
        has_mod_big = game.modifiers.has_modifier(opp_big, ModifierType.CANNOT_SUSPEND)
        assert not has_mod_big, (
            f"Opponent Digimon with {len(opp_big.card_sources) - 1} digi-cards "
            f"(> 1) should NOT have CANNOT_SUSPEND"
        )

    def test_on_play_cannot_suspend_includes_stack_of_two(self, debug_runner):
        """A Digimon with exactly 1 digivolution card (2-card stack: top + 1 under)
        should receive CANNOT_SUSPEND. This tests the DCGO semantics where
        DigivolutionCards.Count counts cards BELOW top."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, [BT16_026])

        # Opponent with 2-card stack (1 digi-card under top)
        opp_two = runner.place_on_field(2, [AGUMON_ST1, GREYMON_ST1])

        # Also place a 1-card Digimon for de-digivolve target
        opp_one = runner.place_on_field(2, [METALGREYMON_ST1])

        game = runner.game
        top = perm.top_card
        on_play_effects = [
            e for e in top.effect_list(None)
            if getattr(e, 'is_on_play', False)
        ]

        on_play_effects[0].on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        # The 2-card stack has 1 digi-card (below top), which is <= 1
        has_mod = game.modifiers.has_modifier(opp_two, ModifierType.CANNOT_SUSPEND)
        assert has_mod, (
            "Opponent Digimon with 2-card stack (1 digi-card) should have "
            "CANNOT_SUSPEND. DCGO DigivolutionCards.Count <= 1 includes this case."
        )


# ── Clause 6: [When Digivolving] same as On Play ──────────────────────


@pytest.mark.behavioral
class TestBT16026WhenDigivolving:
    """[When Digivolving] Same effect as On Play."""

    def test_when_digivolving_effect_exists(self, debug_runner):
        """Should have a When Digivolving effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [BT16_026])

        top = perm.top_card
        all_effects = top.effect_list(None)
        wd_effects = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ]
        assert len(wd_effects) >= 1, "Should have at least 1 When Digivolving effect"

    def test_when_digivolving_de_digivolves_opponent(self, debug_runner):
        """[When Digivolving] Should De-Digivolve 2 an opponent Digimon."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, [GREYMON_ST1, BT16_026])

        opp_perm = runner.place_on_field(2, [AGUMON_ST1, GREYMON_ST1, METALGREYMON_ST1])
        stack_before = len(opp_perm.card_sources)

        game = runner.game
        top = perm.top_card
        wd_effects = [
            e for e in top.effect_list(None)
            if getattr(e, 'is_when_digivolving', False)
        ]

        wd_effects[0].on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        stack_after = len(opp_perm.card_sources)
        assert stack_after == stack_before - 2, (
            f"De-Digivolve 2 should remove 2 cards. Before: {stack_before}, After: {stack_after}"
        )

    def test_when_digivolving_cannot_suspend_applied(self, debug_runner):
        """[When Digivolving] Should apply CANNOT_SUSPEND to opponent Digimon
        with <= 1 digivolution cards."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, [GREYMON_ST1, BT16_026])

        # Opponent with 1-card stack (0 digi-cards) -- should get modifier
        opp_single = runner.place_on_field(2, [AGUMON_ST1])

        game = runner.game
        top = perm.top_card
        wd_effects = [
            e for e in top.effect_list(None)
            if getattr(e, 'is_when_digivolving', False)
        ]

        wd_effects[0].on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        has_mod = game.modifiers.has_modifier(opp_single, ModifierType.CANNOT_SUSPEND)
        assert has_mod, (
            "Opponent Digimon with 0 digi-cards should have CANNOT_SUSPEND after "
            "When Digivolving effect"
        )


# ── Clause 7: [When Attacking] Delete ──────────────────────────────────


@pytest.mark.behavioral
class TestBT16026WhenAttacking:
    """[When Attacking] Delete 1 opponent Digimon with <= 1 digivolution cards."""

    def test_when_attacking_effect_exists(self, debug_runner):
        """Should have a When Attacking effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [BT16_026])

        top = perm.top_card
        all_effects = top.effect_list(None)
        attack_effects = [
            e for e in all_effects
            if getattr(e, 'is_on_attack', False)
        ]
        assert len(attack_effects) >= 1, "Should have at least 1 When Attacking effect"

    def test_when_attacking_deletes_opponent_with_zero_digi_cards(self, debug_runner):
        """Should delete opponent Digimon with 0 digivolution cards (1-card stack)."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, [BT16_026])

        # Opponent with 1-card stack (0 digi-cards below top)
        opp_target = runner.place_on_field(2, [AGUMON_ST1])

        game = runner.game
        p2_field_before = len(game.player2.battle_area)

        top = perm.top_card
        attack_effects = [
            e for e in top.effect_list(None)
            if getattr(e, 'is_on_attack', False)
        ]

        attack_effects[0].on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        p2_field_after = len(game.player2.battle_area)
        assert p2_field_after == p2_field_before - 1, (
            f"Should delete 1 opponent Digimon with 0 digi-cards. "
            f"Field before: {p2_field_before}, after: {p2_field_after}"
        )

    def test_when_attacking_deletes_opponent_with_one_digi_card(self, debug_runner):
        """Should delete opponent Digimon with exactly 1 digivolution card
        (2-card stack: top + 1 under). DCGO DigivolutionCards.Count <= 1."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, [BT16_026])

        # Opponent with 2-card stack (1 digi-card below top)
        opp_target = runner.place_on_field(2, [AGUMON_ST1, GREYMON_ST1])

        game = runner.game
        p2_field_before = len(game.player2.battle_area)

        top = perm.top_card
        attack_effects = [
            e for e in top.effect_list(None)
            if getattr(e, 'is_on_attack', False)
        ]

        attack_effects[0].on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        p2_field_after = len(game.player2.battle_area)
        assert p2_field_after == p2_field_before - 1, (
            f"Should delete opponent Digimon with 1 digi-card (2-card stack). "
            f"DCGO DigivolutionCards.Count <= 1 includes this. "
            f"Field before: {p2_field_before}, after: {p2_field_after}"
        )

    def test_when_attacking_does_not_delete_with_two_digi_cards(self, debug_runner):
        """Should NOT delete opponent Digimon with 2+ digivolution cards
        (3+ card stack)."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, [BT16_026])

        # Opponent with 3-card stack (2 digi-cards below top)
        opp_big = runner.place_on_field(2, [AGUMON_ST1, GREYMON_ST1, METALGREYMON_ST1])

        game = runner.game
        p2_field_before = len(game.player2.battle_area)

        top = perm.top_card
        attack_effects = [
            e for e in top.effect_list(None)
            if getattr(e, 'is_on_attack', False)
        ]

        attack_effects[0].on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        p2_field_after = len(game.player2.battle_area)
        assert p2_field_after == p2_field_before, (
            f"Should NOT delete opponent Digimon with 2 digi-cards (3-card stack). "
            f"Field before: {p2_field_before}, after: {p2_field_after}"
        )

    def test_when_attacking_no_valid_targets_no_crash(self, debug_runner):
        """When Attacking should not crash when there are no valid targets."""
        runner = debug_runner(initial_memory=10)

        perm = runner.place_on_field(1, [BT16_026])

        # No opponent Digimon
        game = runner.game
        p2_field_before = len(game.player2.battle_area)

        top = perm.top_card
        attack_effects = [
            e for e in top.effect_list(None)
            if getattr(e, 'is_on_attack', False)
        ]

        # Should not raise
        attack_effects[0].on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        runner.auto_resolve()

        assert len(game.player2.battle_area) == p2_field_before


# ── Clause 8: Ace Overflow <-4> inherited ──────────────────────────────


@pytest.mark.behavioral
class TestBT16026AceOverflow:
    """Ace Overflow <-4> inherited effect should be present."""

    def test_ace_overflow_inherited_effect(self, debug_runner):
        """Should have Ace Overflow <-4> inherited effect."""
        from digimon_gym.engine.data.card_registry import CardRegistry
        CardRegistry.ensure_initialized()
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()

        runner = debug_runner(initial_memory=10)
        cs = db.create_card_source(BT16_026, runner.game.player1)
        assert cs is not None

        # Check entity_base for ACE properties
        assert cs.c_entity_base.is_ace, "BT16-026 should be detected as ACE"
        assert cs.c_entity_base.ace_overflow_cost == 4, (
            "Ace Overflow should be -4 (cost = 4)"
        )

        # Check inherited effect in script
        effects = cs.effect_list(None)
        ace_effects = [
            e for e in effects
            if e.is_inherited_effect and (
                'ace overflow' in (e.effect_name or '').lower()
                or 'ace overflow' in (e.effect_description or '').lower()
            )
        ]
        assert len(ace_effects) >= 1, (
            "Script should declare an inherited Ace Overflow effect"
        )
