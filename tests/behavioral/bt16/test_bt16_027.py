"""Behavioral tests for BT16-027 Imperialdramon: Fighter Mode (ACE)
Lv.6 Blue/Green | DP 13000 | Cost 8 (ACE play cost)

Card text (from cards.json):
[Hand] [Counter] <Blast Digivolve>
[On Play] [When Digivolving] Return 1 of your opponent's Digimon with as many
    or fewer digivolution cards as this Digimon to the bottom of the deck.
[End of Attack] [Once Per Turn] Unsuspend this Digimon. Then, if
    [Imperialdramon: Dragon Mode] is in this Digimon's digivolution cards,
    return 1 of your opponent's suspended Digimon to the bottom of the deck.
Inherited: Ace Overflow <-4>

Alt-Digi: From [Imperialdramon: Dragon Mode] for cost 2

Key behaviors verified:
1. Blast Digivolve flag is set
2. Alt-digi from Imperialdramon: Dragon Mode for cost 2
3. On Play: bounce opponent Digimon with <= digi-card count
4. When Digivolving: same bounce logic as On Play
5. End of Attack [OPT]: unsuspend, then conditional bounce of suspended opponent
6. Conditional bounce requires Dragon Mode in digivolution cards
7. Ace Overflow <-4> inherited effect present
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


# Card IDs used in tests:
# BT16-027 = Imperialdramon: Fighter Mode (Lv.6, DP 13000, ACE)
# BT16-028 = Imperialdramon: Dragon Mode (Lv.5)
# ST1-03 = Agumon (Lv.3, DP 2000)
# ST1-07 = Greymon (Lv.4, DP 5000)
# BT12-030 = Imperialdramon: Dragon Mode (Lv.6)


@pytest.mark.behavioral
class TestBT16027ImperialdramonFighterMode:
    """Tests for BT16-027 Imperialdramon: Fighter Mode (ACE)."""

    # ── Clause 1: Blast Digivolve flag ──────────────────────────────

    def test_blast_digivolve_flag(self, debug_runner):
        """Should have _is_blast_digivolve flag set on an effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-027"])

        top = perm.top_card
        all_effects = top.effect_list(None)
        blast_effects = [
            e for e in all_effects if getattr(e, '_is_blast_digivolve', False)
        ]
        assert len(blast_effects) >= 1, "Should have at least 1 Blast Digivolve effect"

    # ── Clause 2: Alt-digi from Dragon Mode for cost 2 ─────────────

    def test_alt_digi_from_dragon_mode(self, debug_runner):
        """Should have alt-digi from Imperialdramon: Dragon Mode for cost 2."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-027"])

        top = perm.top_card
        all_effects = top.effect_list(None)
        alt_digi = [
            e for e in all_effects if getattr(e, '_alt_digi_cost', None) is not None
        ]
        assert len(alt_digi) >= 1, "Should have an alt-digi effect"
        assert alt_digi[0]._alt_digi_cost == 2, "Alt-digi cost should be 2"
        assert alt_digi[0]._alt_digi_name == "Imperialdramon: Dragon Mode", (
            "Alt-digi name should be Imperialdramon: Dragon Mode"
        )

    # ── Clause 3: [On Play] bounce by digi-card count ──────────────

    def test_on_play_effect_exists(self, debug_runner):
        """Should have an On Play effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-027"])

        top = perm.top_card
        all_effects = top.effect_list(None)
        on_play_effects = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
        ]
        assert len(on_play_effects) >= 1, "Should have at least 1 On Play effect"

    def test_on_play_bounces_opponent_with_fewer_digi_cards(self, debug_runner):
        """[On Play] Should return opponent's Digimon with <= digi-card count
        to the bottom of the deck."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place Fighter Mode with 2 digi cards (stack of 3 = 2 under top)
        perm = runner.place_on_field(1, ["ST1-03", "ST1-07", "BT16-027"])

        # Opponent has a Digimon with 1 digi card (stack of 2 = 1 under top)
        opp_perm = runner.place_on_field(2, ["ST1-03", "ST1-07"])

        snap_before = runner.snapshot()
        p2_field_before = len(snap_before.p2_field)
        p2_lib_before = snap_before.p2_library_size

        # Fire On Play
        runner.game.execute_effects(EffectTiming.OnEnterFieldAnyone, {
            "permanent": perm,
            "played_card": perm.top_card,
            "player": runner.game.player1,
        })
        runner.auto_resolve()

        snap_after = runner.snapshot()

        # Opponent's Digimon should be returned to deck bottom
        assert len(snap_after.p2_field) == p2_field_before - 1, (
            "Opponent's Digimon with fewer digi-cards should be returned to deck bottom"
        )

    def test_on_play_does_not_bounce_opponent_with_more_digi_cards(self, debug_runner):
        """[On Play] Should NOT return opponent's Digimon with more digi-cards."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Place Fighter Mode with 0 digi cards (just 1 card = 0 under top)
        perm = runner.place_on_field(1, ["BT16-027"])

        # Opponent has a Digimon with 1 digi card (stack of 2)
        opp_perm = runner.place_on_field(2, ["ST1-03", "ST1-07"])

        snap_before = runner.snapshot()
        p2_field_before = len(snap_before.p2_field)

        # Fire On Play
        runner.game.execute_effects(EffectTiming.OnEnterFieldAnyone, {
            "permanent": perm,
            "played_card": perm.top_card,
            "player": runner.game.player1,
        })
        runner.auto_resolve()

        snap_after = runner.snapshot()

        # Opponent's Digimon should NOT be bounced (has more digi-cards)
        assert len(snap_after.p2_field) == p2_field_before, (
            "Opponent's Digimon with MORE digi-cards should NOT be returned"
        )

    # ── Clause 4: [When Digivolving] same bounce logic ─────────────

    def test_when_digivolving_effect_exists(self, debug_runner):
        """Should have a When Digivolving effect."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["BT16-027"])

        top = perm.top_card
        all_effects = top.effect_list(None)
        digivolve_effects = [
            e for e in all_effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_when_digivolving', False)
        ]
        assert len(digivolve_effects) >= 1, "Should have at least 1 When Digivolving effect"

    def test_when_digivolving_bounces_opponent(self, debug_runner):
        """[When Digivolving] Should bounce opponent's Digimon same as On Play."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Stack simulating digivolving: Dragon Mode -> Fighter Mode
        perm = runner.place_on_field(1, ["BT16-028", "BT16-027"])

        # Opponent has a Digimon with 0 digi cards (just 1 card)
        runner.place_on_field(2, ["ST1-03"])

        snap_before = runner.snapshot()
        p2_field_before = len(snap_before.p2_field)

        # Fire When Digivolving (WhenDigivolving timing, NOT OnEnterFieldAnyone)
        runner.game.execute_effects(EffectTiming.WhenDigivolving, {
            "permanent": perm,
            "digivolved_permanent": perm,
            "player": runner.game.player1,
        })
        runner.auto_resolve()

        snap_after = runner.snapshot()

        assert len(snap_after.p2_field) == p2_field_before - 1, (
            "[When Digivolving] should bounce opponent's Digimon"
        )

    # ── Clause 5: [End of Attack] Unsuspend (OPT) ──────────────────

    def test_end_of_attack_unsuspends(self, debug_runner):
        """[End of Attack] Should unsuspend this Digimon."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["BT16-027"])
        perm.suspend()
        assert perm.is_suspended, "Should be suspended before End of Attack"

        # Fire End of Attack
        runner.game.execute_effects(EffectTiming.OnEndAttack, {
            "attacker": perm,
            "permanent": perm,
            "player": runner.game.player1,
        })
        runner.auto_resolve()

        assert not perm.is_suspended, "Should be unsuspended after End of Attack"

    def test_end_of_attack_is_once_per_turn(self, debug_runner):
        """[End of Attack] [Once Per Turn] Should not fire twice in same turn."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["BT16-027"])

        # First attack: should unsuspend
        perm.suspend()
        runner.game.execute_effects(EffectTiming.OnEndAttack, {
            "attacker": perm,
            "permanent": perm,
            "player": runner.game.player1,
        })
        runner.auto_resolve()
        assert not perm.is_suspended, "Should unsuspend on first attack"

        # Second attack: should NOT unsuspend (OPT used)
        perm.suspend()
        runner.game.execute_effects(EffectTiming.OnEndAttack, {
            "attacker": perm,
            "permanent": perm,
            "player": runner.game.player1,
        })
        runner.auto_resolve()
        assert perm.is_suspended, (
            "Should remain suspended on second attack (OPT already used)"
        )

    # ── Clause 6: End of Attack conditional bounce ──────────────────

    def test_end_of_attack_with_dragon_mode_bounces_suspended(self, debug_runner):
        """[End of Attack] With Dragon Mode in digi-stack, should return 1
        opponent's suspended Digimon to deck bottom."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Stack: Dragon Mode under Fighter Mode
        perm = runner.place_on_field(1, ["BT16-028", "BT16-027"])
        perm.suspend()

        # Opponent has a suspended Digimon
        opp_perm = runner.place_on_field(2, ["ST1-03"], is_suspended=True)

        snap_before = runner.snapshot()
        p2_field_before = len(snap_before.p2_field)

        # Fire End of Attack
        runner.game.execute_effects(EffectTiming.OnEndAttack, {
            "attacker": perm,
            "permanent": perm,
            "player": runner.game.player1,
        })
        runner.auto_resolve()

        snap_after = runner.snapshot()

        # Should unsuspend AND bounce the suspended opponent
        assert not perm.is_suspended, "Should unsuspend"
        assert len(snap_after.p2_field) == p2_field_before - 1, (
            "Should return opponent's suspended Digimon to deck bottom"
        )

    def test_end_of_attack_without_dragon_mode_no_bounce(self, debug_runner):
        """[End of Attack] Without Dragon Mode in digi-stack, should NOT bounce."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # No Dragon Mode under Fighter Mode
        perm = runner.place_on_field(1, ["BT16-027"])
        perm.suspend()

        # Opponent has a suspended Digimon
        runner.place_on_field(2, ["ST1-03"], is_suspended=True)

        snap_before = runner.snapshot()
        p2_field_before = len(snap_before.p2_field)

        # Fire End of Attack
        runner.game.execute_effects(EffectTiming.OnEndAttack, {
            "attacker": perm,
            "permanent": perm,
            "player": runner.game.player1,
        })
        runner.auto_resolve()

        snap_after = runner.snapshot()

        # Should unsuspend but NOT bounce (no Dragon Mode)
        assert not perm.is_suspended, "Should still unsuspend"
        assert len(snap_after.p2_field) == p2_field_before, (
            "Should NOT bounce without Dragon Mode in digi-stack"
        )

    def test_end_of_attack_only_bounces_suspended_opponent(self, debug_runner):
        """[End of Attack] The conditional bounce should only target SUSPENDED
        opponent Digimon, not unsuspended ones."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        # Stack with Dragon Mode
        perm = runner.place_on_field(1, ["BT16-028", "BT16-027"])
        perm.suspend()

        # Opponent has an UNSUSPENDED Digimon only
        runner.place_on_field(2, ["ST1-03"], is_suspended=False)

        snap_before = runner.snapshot()
        p2_field_before = len(snap_before.p2_field)

        # Fire End of Attack
        runner.game.execute_effects(EffectTiming.OnEndAttack, {
            "attacker": perm,
            "permanent": perm,
            "player": runner.game.player1,
        })
        runner.auto_resolve()

        snap_after = runner.snapshot()

        # Should unsuspend but not bounce (opponent not suspended)
        assert not perm.is_suspended, "Should unsuspend"
        assert len(snap_after.p2_field) == p2_field_before, (
            "Should NOT bounce unsuspended opponent Digimon"
        )

    # ── Clause 7: Ace Overflow <-4> inherited effect ────────────────

    def test_ace_overflow_inherited_effect(self, debug_runner):
        """Should have Ace Overflow <-4> inherited effect."""
        from digimon_gym.engine.data.card_registry import CardRegistry
        CardRegistry.ensure_initialized()
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()

        runner = debug_runner(initial_memory=10)
        cs = db.create_card_source("BT16-027", runner.game.player1)
        assert cs is not None

        # Check entity_base for ACE properties
        assert cs.c_entity_base.is_ace, "BT16-027 should be detected as ACE"
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
