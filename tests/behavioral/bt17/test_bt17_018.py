"""Behavioral tests for BT17-018 Gallantmon: Crimson Mode (Lv.7 Red).

Card text:
[Hand] [Counter] <Blast Digivolve>
[On Play] [When Digivolving] Choose any number of your opponent's Digimon
    whose total DP adds up to 15000 or less and delete them.
[When Attacking] [Once Per Turn] For every 10 cards in both player's trash,
    trash 1 card from the top of your opponent's security stack.
Inherited: Ace Overflow <-5>

Alt-digi: Lv.6 w/[Gallantmon] in name: Cost 4
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestBT17018GallantmonCrimsonMode:
    """Tests for BT17-018 Gallantmon: Crimson Mode."""

    # --- Alt-digi tests ---

    def test_alt_digi_requires_gallantmon_name(self, debug_runner):
        """Alt-digi should use _alt_digi_name = 'Gallantmon' (not _alt_digi_name_filter)."""
        runner = debug_runner(initial_memory=5)

        from digimon_gym.engine.data.card_database import CardDatabase
        from digimon_gym.engine.data.card_registry import CardRegistry
        CardRegistry.ensure_initialized()
        db = CardDatabase()

        cs = db.create_card_source("BT17-018", runner.game.player1)
        effects = cs.effect_list(None)
        alt_effects = [e for e in effects if getattr(e, '_alt_digi_cost', None) is not None]
        assert len(alt_effects) >= 1, "Should have alt-digi effect"
        alt = alt_effects[0]
        assert getattr(alt, '_alt_digi_level', None) == 6, "Alt-digi should require Lv.6"
        assert getattr(alt, '_alt_digi_name', None) == "Gallantmon", (
            "Alt-digi should use _alt_digi_name='Gallantmon', "
            f"got _alt_digi_name={getattr(alt, '_alt_digi_name', None)}, "
            f"_alt_digi_name_filter={getattr(alt, '_alt_digi_name_filter', None)}"
        )
        assert getattr(alt, '_alt_digi_cost', None) == 4, "Alt-digi cost should be 4"

    def test_alt_digi_matches_gallantmon_lv6(self, debug_runner):
        """A Lv.6 Gallantmon on field should be a valid alt-digi base."""
        runner = debug_runner(initial_memory=5)

        from digimon_gym.engine.validation.digivolve_validator import _check_alt_digivolve
        from digimon_gym.engine.data.card_database import CardDatabase
        from digimon_gym.engine.data.card_registry import CardRegistry
        CardRegistry.ensure_initialized()
        db = CardDatabase()

        # Place a Lv.6 Gallantmon (ST7-09) on field
        perm = runner.place_on_field(1, ["ST1-03", "ST1-06", "ST7-09"])

        # Create the BT17-018 card source
        cs = db.create_card_source("BT17-018", runner.game.player1)

        # Check alt-digi
        result = _check_alt_digivolve(cs, perm)
        assert result, "BT17-018 should be able to alt-digivolve on top of Lv.6 Gallantmon"

    # --- Delete totaling 15000 DP tests ---

    def test_on_play_delete_single_target(self, debug_runner):
        """[On Play] should allow deleting a single opponent Digimon <= 15000 DP."""
        runner = debug_runner(initial_memory=20)
        runner.set_phase("Main")

        # Opponent has a 7000 DP Digimon
        opp_perm = runner.place_on_field(2, ["ST1-09"])  # MetalGreymon 7000 DP
        assert opp_perm.dp == 7000

        perm = runner.place_on_field(1, ["BT17-018"])
        game = runner.game

        # Find On Play delete effect
        top_card = perm.top_card
        effects = top_card.effect_list(None)
        on_play_del = [
            e for e in effects
            if e.timing == EffectTiming.OnEnterFieldAnyone
            and getattr(e, 'is_on_play', False)
            and e.on_process_callback is not None
        ]
        assert len(on_play_del) >= 1, "Should have On Play delete effect"

        # Execute the effect
        on_play_del[0].on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        # Auto-resolve the selection (pick the target, then decline more)
        runner.auto_resolve()

        # Opponent's MetalGreymon should be deleted
        snap = runner.snapshot()
        opp_digimon = [s for s in snap.p2_field if s.card_id == "ST1-09"]
        assert len(opp_digimon) == 0, "7000 DP MetalGreymon should be deleted (within 15000 budget)"

    def test_when_attacking_trash_security(self, debug_runner):
        """[When Attacking] should trash opponent security: 1 per 10 cards in both trashes."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["BT17-018"])
        game = runner.game

        # Add cards to both trash piles to get 20+ total = 2 security trashed
        for _ in range(12):
            runner.inject_card(1, "ST1-03", "trash")
        for _ in range(8):
            runner.inject_card(2, "ST1-03", "trash")
        # Total: 20 cards in trash = 2 security should be trashed

        sec_before = len(game.player2.security_cards)

        # Find the When Attacking trash security effect
        top_card = perm.top_card
        effects = top_card.effect_list(None)
        wa_trash = [
            e for e in effects
            if e.timing == EffectTiming.OnUseAttack
            and getattr(e, 'is_on_attack', False)
            and e.on_process_callback is not None
        ]
        assert len(wa_trash) >= 1, "Should have When Attacking trash security effect"

        wa_trash[0].on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'attacking_permanent': perm,
        })

        sec_after = len(game.player2.security_cards)
        expected_trashed = 20 // 10  # 2
        assert sec_before - sec_after == expected_trashed, (
            f"Should trash {expected_trashed} security cards, "
            f"but went from {sec_before} to {sec_after}"
        )

    def test_when_attacking_trash_security_rounds_down(self, debug_runner):
        """Trash count should round down (15 cards = 1 security trashed)."""
        runner = debug_runner(initial_memory=10)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["BT17-018"])
        game = runner.game

        for _ in range(15):
            runner.inject_card(1, "ST1-03", "trash")
        # Total: 15 cards in both trashes = 1 security

        sec_before = len(game.player2.security_cards)

        top_card = perm.top_card
        effects = top_card.effect_list(None)
        wa_trash = [
            e for e in effects
            if e.timing == EffectTiming.OnUseAttack
            and getattr(e, 'is_on_attack', False)
            and e.on_process_callback is not None
        ]
        wa_trash[0].on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'attacking_permanent': perm,
        })

        sec_after = len(game.player2.security_cards)
        assert sec_before - sec_after == 1, (
            f"15 cards in trash -> 1 security trashed, "
            f"but went from {sec_before} to {sec_after}"
        )

    def test_blast_digivolve_flag(self, debug_runner):
        """Blast Digivolve flag should be present."""
        runner = debug_runner(initial_memory=5)

        from digimon_gym.engine.data.card_database import CardDatabase
        from digimon_gym.engine.data.card_registry import CardRegistry
        CardRegistry.ensure_initialized()
        db = CardDatabase()

        cs = db.create_card_source("BT17-018", runner.game.player1)
        effects = cs.effect_list(None)
        blast_effects = [e for e in effects if getattr(e, '_is_blast_digivolve', False)]
        assert len(blast_effects) >= 1, "Should have Blast Digivolve effect"
