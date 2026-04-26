"""Behavioral tests for BT22-015 Omnimon (Lv.7 Red/Yellow).

Card text (from cards.json):
<Blocker>
<Decode (Red/Black Lv.3)>
<Decode (Blue/Yellow Lv.3)>
[On Play] [When Attacking] Delete 1 of your opponent's Digimon with the lowest DP.
[When Digivolving] For every 2 same-level cards this Digimon's stack has,
    return 1 of your opponent's Digimon to the bottom of the deck.
    Then, this Digimon may attack.

Alt-digi: Lv.6 w/ [CS] trait: Cost 5
DNA: Lv.6 w/ [Greymon] + Lv.6 w/ [Garurumon], cost 0
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT22015Omnimon:
    """Tests for BT22-015 Omnimon."""

    # ── Alt-digi: Lv.6 w/ [CS] trait, cost 5 ──────────────────────────

    def test_alt_digi_attributes(self, debug_runner):
        """Alt-digi effect must require Lv.6 + CS trait for cost 5."""
        runner = debug_runner(initial_memory=10)
        # Inject BT22-015 into hand to read its effects
        runner.inject_card(1, "BT22-015", "hand")
        game = runner.game
        card = None
        for c in game.player1.hand_cards:
            if c.card_id == "BT22-015":
                card = c
                break
        assert card is not None

        effects = card.effect_list(None)
        alt_digi = [e for e in effects if getattr(e, '_alt_digi_cost', None) is not None]
        assert len(alt_digi) >= 1, "Should have at least 1 alt-digi effect"

        eff = alt_digi[0]
        assert eff._alt_digi_cost == 5, "Alt-digi cost should be 5"
        assert getattr(eff, '_alt_digi_level', None) == 6, "Alt-digi should require Lv.6"
        assert getattr(eff, '_alt_digi_trait', None) == "CS", \
            "Alt-digi should require [CS] trait"

    # ── Blocker ────────────────────────────────────────────────────────

    def test_blocker_keyword(self, debug_runner):
        """Omnimon should have the Blocker keyword."""
        runner = debug_runner(initial_memory=10)
        runner.inject_card(1, "BT22-015", "hand")
        game = runner.game
        card = [c for c in game.player1.hand_cards if c.card_id == "BT22-015"][0]
        effects = card.effect_list(None)
        blocker_effs = [e for e in effects if getattr(e, '_is_blocker', False)]
        assert len(blocker_effs) >= 1, "Should have at least 1 Blocker effect"

    # ── Two Decode effects ─────────────────────────────────────────────

    def test_two_decode_effects(self, debug_runner):
        """Should have exactly 2 Decode keyword effects."""
        runner = debug_runner(initial_memory=10)
        runner.inject_card(1, "BT22-015", "hand")
        game = runner.game
        card = [c for c in game.player1.hand_cards if c.card_id == "BT22-015"][0]
        effects = card.effect_list(None)
        decode_effs = [e for e in effects if getattr(e, '_is_decode', False)]
        assert len(decode_effs) == 2, (
            f"Should have exactly 2 Decode effects, got {len(decode_effs)}"
        )

    # ── [On Play] Delete 1 lowest DP ──────────────────────────────────

    def test_on_play_delete_lowest_dp(self, debug_runner):
        """[On Play] should delete 1 opponent Digimon with the lowest DP.

        Setup: opponent has 2 Digimon - one with 1000 DP, one with 5000 DP.
        After On Play triggers, the 1000 DP one should be deleted.
        """
        runner = debug_runner(initial_memory=15)

        # Place opponent Digimon with varying DP
        # BT1-011 Agumon Expert = 1000 DP Lv.3
        # AD1-001 Greymon = 5000 DP Lv.4
        runner.place_on_field(2, ["BT1-011"])
        runner.place_on_field(2, ["AD1-001"])

        snap_before = runner.snapshot()
        assert len(snap_before.p2_field) == 2, "Opponent should have 2 Digimon"

        # Place Omnimon on field (simulates On Play)
        perm = runner.place_on_field(1, ["BT22-015"])
        game = runner.game

        # Find and trigger the On Play effect
        card = perm.top_card
        effects = card.effect_list(None)
        on_play_effects = [
            e for e in effects
            if getattr(e, 'is_on_play', False)
            and e.timing == EffectTiming.OnEnterFieldAnyone
        ]
        assert len(on_play_effects) >= 1, "Should have On Play effect"

        op_eff = on_play_effects[0]
        assert op_eff.can_use_condition({}), "On Play condition should pass when on field"

        # Process the effect
        op_eff.on_process_callback({
            'player': game.player1,
            'game': game,
        })

        # Resolve selection (auto-pick the lowest DP one)
        runner.auto_resolve()

        snap_after = runner.snapshot()
        # One opponent Digimon should have been deleted
        assert len(snap_after.p2_field) == 1, (
            f"Opponent should have 1 Digimon remaining (lowest DP deleted), "
            f"got {len(snap_after.p2_field)}"
        )
        # The remaining one should be the higher-DP Greymon
        remaining = snap_after.p2_field[0]
        assert remaining.dp == 5000, (
            f"Remaining Digimon should be the 5000 DP one, got {remaining.dp}"
        )

    # ── [When Attacking] Delete 1 lowest DP ───────────────────────────

    def test_when_attacking_effect_exists(self, debug_runner):
        """Should have a When Attacking delete effect."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, ["BT22-015"])
        card = perm.top_card
        effects = card.effect_list(None)
        wa_effects = [
            e for e in effects
            if getattr(e, 'is_on_attack', False)
            and e.timing == EffectTiming.OnUseAttack
        ]
        assert len(wa_effects) >= 1, "Should have When Attacking effect"

    # ── [When Digivolving] Bottom-deck + may attack ───────────────────

    def test_when_digivolving_bottom_deck_count(self, debug_runner):
        """[When Digivolving] For every 2 same-level cards, bottom-deck 1.

        Setup: Stack has 2 Lv.3 cards and 2 Lv.6 cards (the top BT22-015 is Lv.7
        so doesn't pair). That's 2 pairs = bottom-deck 2 opponent Digimon.
        """
        runner = debug_runner(initial_memory=10)

        # Build a stack: 2 x Lv.3, 1 x Lv.4, 2 x Lv.6, then BT22-015 (Lv.7) on top
        # BT1-010 = Lv.3 Agumon, BT1-012 = Lv.3 Biyomon
        # AD1-001 = Lv.4 Greymon
        # BT22-013 = Lv.6 WarGreymon (CS), AD1-004 = Lv.6 WarGreymon
        perm = runner.place_on_field(1, [
            "BT1-010", "BT1-012",  # 2 x Lv.3
            "AD1-001",              # 1 x Lv.4 (no pair)
            "BT22-013", "AD1-004",  # 2 x Lv.6
            "BT22-015",             # Lv.7 (no pair)
        ])

        # Place 3 opponent Digimon
        runner.place_on_field(2, ["BT1-010"])
        runner.place_on_field(2, ["BT1-012"])
        runner.place_on_field(2, ["AD1-001"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        wd_effects = [
            e for e in effects
            if getattr(e, 'is_when_digivolving', False)
            and e.timing == EffectTiming.OnEnterFieldAnyone
        ]
        assert len(wd_effects) >= 1, "Should have When Digivolving effect"

        wd_eff = wd_effects[0]
        assert wd_eff.can_use_condition({}), "Condition should pass when on field"

        # Count same-level pairs
        level_counts = {}
        for cs in perm.card_sources:
            lvl = cs.level
            if lvl is not None:
                level_counts[lvl] = level_counts.get(lvl, 0) + 1
        expected_pairs = sum(count // 2 for count in level_counts.values())
        assert expected_pairs == 2, (
            f"Expected 2 same-level pairs (2 Lv.3, 2 Lv.6), got {expected_pairs}. "
            f"Level counts: {level_counts}"
        )

        # Process the effect
        wd_eff.on_process_callback({
            'player': game.player1,
            'game': game,
        })
        runner.auto_resolve()

        # 2 opponent Digimon should have been bottom-decked
        snap = runner.snapshot()
        assert len(snap.p2_field) == 1, (
            f"Opponent should have 1 Digimon remaining (2 bottom-decked), "
            f"got {len(snap.p2_field)}"
        )

    def test_when_digivolving_may_attack_modifier(self, debug_runner):
        """[When Digivolving] 'Then, this Digimon may attack' should register
        a MAY_ATTACK modifier on the permanent."""
        runner = debug_runner(initial_memory=10)

        # Simple stack: just 1 Lv.3 under Omnimon (no pairs, 0 bottom-decks)
        perm = runner.place_on_field(1, ["BT1-010", "BT22-015"])

        game = runner.game
        card = perm.top_card
        effects = card.effect_list(None)
        wd_effects = [
            e for e in effects
            if getattr(e, 'is_when_digivolving', False)
            and e.timing == EffectTiming.OnEnterFieldAnyone
        ]
        wd_eff = wd_effects[0]

        # Process the effect (0 bottom-decks but should still grant may-attack)
        wd_eff.on_process_callback({
            'player': game.player1,
            'game': game,
        })
        runner.auto_resolve()

        # Check MAY_ATTACK modifier is registered
        from engine_py_legacy.engine.interfaces.modifiers import ModifierType
        has_may_attack = game.modifiers.has_modifier(perm, ModifierType.MAY_ATTACK)
        assert has_may_attack, (
            "After When Digivolving resolves, Omnimon should have MAY_ATTACK modifier"
        )

    # ── No targets edge case ──────────────────────────────────────────

    def test_on_play_no_opponent_digimon(self, debug_runner):
        """[On Play] with no opponent Digimon should not crash."""
        runner = debug_runner(initial_memory=15)
        perm = runner.place_on_field(1, ["BT22-015"])
        game = runner.game

        card = perm.top_card
        effects = card.effect_list(None)
        on_play = [
            e for e in effects
            if getattr(e, 'is_on_play', False)
            and e.timing == EffectTiming.OnEnterFieldAnyone
        ][0]

        # Should not crash
        on_play.on_process_callback({
            'player': game.player1,
            'game': game,
        })
        runner.auto_resolve()

        snap = runner.snapshot()
        assert len(snap.p2_field) == 0, "No opponent Digimon to delete"
