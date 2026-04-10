"""Behavioral tests for EX7-060 Nidhoggmon.

Card text:
  [Trash] [Main] If you have 4 or fewer cards in your hand, you may play
      this card from your trash with the play cost reduced by 4.
  <Blocker>
  [On Deletion] You may play 1 level 5 or lower Digimon card with the
      [Dark Dragon] or [Evil Dragon] trait from your trash without paying
      the cost.
"""

import pytest

from digimon_gym.engine.data.card_registry import CardRegistry
from digimon_gym.engine.data.enums import EffectTiming, GamePhase

# Card IDs
NIDHOGGMON_ID = "EX7-060"       # Lv.6 Purple Dark Dragon, cost 11
DARK_DRAGON_LV4 = "BT2-013"    # Growlmon, Lv.4 Dark Dragon, cost 5
EVIL_DRAGON_LV4 = "BT3-081"    # Devidramon, Lv.4 Evil Dragon, cost 5
DARK_DRAGON_LV5 = "BT4-058"    # Orochimon, Lv.5 Dark Dragon, cost 8
DARK_DRAGON_LV6 = "BT4-062"    # Nidhoggmon, Lv.6 Dark Dragon (too high)
NON_DRAGON_LV4 = "AD1-001"     # Greymon, Lv.4, no Dark/Evil Dragon trait
FILLER_ID = "BT1-003"          # Cheap filler


def _build_deck(extras=None):
    """50-card deck: extras + filler."""
    cards = list(extras or [])
    while len(cards) < 50:
        cards.append(FILLER_ID)
    return cards


def _select_non_decline(runner, max_steps=20):
    """Resolve selection phases by picking non-decline actions.

    For optional effects, auto_resolve picks action 0 which is often
    'Decline / Pass'. This helper picks the first non-62 (non-decline)
    action when in a selection phase, allowing us to exercise the effect.
    """
    results = []
    for _ in range(max_steps):
        if runner.game.game_over:
            break
        if runner.game.current_phase in (GamePhase.Main, GamePhase.Breeding, GamePhase.End):
            break
        legal = runner.action_mask()
        if not legal:
            break
        # Pick non-decline if available, else pick first
        non_decline = [a for a in legal if a != 62]
        pick = non_decline[0] if non_decline else legal[0]
        results.append(runner.execute(pick))
    return results


# =====================================================================
# Trash Main effect tests
# =====================================================================

class TestEX7060TrashMain:
    """[Trash] [Main] If you have 4 or fewer cards in your hand, you may play
    this card from your trash with the play cost reduced by 4."""

    def test_trash_main_available_when_in_trash_hand_le_4(self, debug_runner):
        """Trash main action should appear when Nidhoggmon is in trash and hand <= 4."""
        deck1 = _build_deck([NIDHOGGMON_ID] * 4)
        deck2 = _build_deck()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=10)
        runner.set_phase("Main")

        # Clear hand to ensure <= 4
        runner.clear_zone(1, "hand")
        for _ in range(3):
            runner.inject_card(1, FILLER_ID, "hand")
        # Put Nidhoggmon in trash
        runner.inject_card(1, NIDHOGGMON_ID, "trash")

        assert len(runner.game.player1.hand_cards) <= 4

        actions = runner.find_actions("Nidhoggmon")
        assert len(actions) > 0, (
            f"Expected trash main action for Nidhoggmon, but found none. "
            f"Available actions: {runner.actions()}"
        )

    def test_trash_main_blocked_when_hand_gt_4(self, debug_runner):
        """Trash main action should NOT appear when hand > 4."""
        deck1 = _build_deck([NIDHOGGMON_ID] * 4)
        deck2 = _build_deck()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=10)
        runner.set_phase("Main")

        # Clear hand and put exactly 5 cards
        runner.clear_zone(1, "hand")
        for _ in range(5):
            runner.inject_card(1, FILLER_ID, "hand")
        runner.inject_card(1, NIDHOGGMON_ID, "trash")

        assert len(runner.game.player1.hand_cards) == 5

        actions = runner.find_actions("Nidhoggmon")
        assert len(actions) == 0, (
            f"Expected NO trash main action when hand > 4, but found: {actions}"
        )

    def test_trash_main_not_available_when_not_in_trash(self, debug_runner):
        """Trash main should not appear if Nidhoggmon is not in trash."""
        deck1 = _build_deck([NIDHOGGMON_ID] * 4)
        deck2 = _build_deck()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=10)
        runner.set_phase("Main")

        runner.clear_zone(1, "hand")
        # Nidhoggmon only in hand, not in trash
        runner.inject_card(1, NIDHOGGMON_ID, "hand")

        # Should not find a trash-main style action
        trash_actions = runner.find_actions("[Trash][Main]")
        nidhogg_trash = {k: v for k, v in trash_actions.items()
                         if "nidhoggmon" in v.lower()}
        assert len(nidhogg_trash) == 0, (
            f"Expected no trash-main action when not in trash, found: {nidhogg_trash}"
        )

    def test_trash_main_cost_reduction_applies(self, debug_runner):
        """When played from trash, play cost should be reduced by 4 (11 -> 7)."""
        deck1 = _build_deck([NIDHOGGMON_ID] * 4)
        deck2 = _build_deck()
        # Need at least 7 memory to afford cost 11 - 4 = 7
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=7)
        runner.set_phase("Main")

        runner.clear_zone(1, "hand")
        runner.inject_card(1, FILLER_ID, "hand")  # 1 card in hand
        runner.inject_card(1, NIDHOGGMON_ID, "trash")

        mem_before = runner.game.memory

        action = runner.find_action("Nidhoggmon")
        assert action is not None, "Should find trash main action for Nidhoggmon"
        runner.execute(action)
        runner.auto_resolve()

        # Nidhoggmon should be on field
        snap = runner.snapshot()
        assert any(s.card_id == NIDHOGGMON_ID for s in snap.p1_field), \
            "Nidhoggmon should be on field after trash main play"

        # Memory should decrease by 7 (cost 11 - 4 reduction)
        mem_after = runner.game.memory
        cost_paid = mem_before - mem_after
        assert cost_paid == 7, (
            f"Expected cost of 7 (11 - 4 reduction), but paid {cost_paid} "
            f"(memory went from {mem_before} to {mem_after})"
        )

    def test_trash_main_cost_reduction_no_hand_recheck(self, debug_runner):
        """Cost reduction should apply regardless of hand size changing during play.

        The C# only checks card_source == card for cost reduction condition,
        NOT re-checking the hand count. The hand count is only gated at the
        OnDeclaration condition (condition0).
        """
        deck1 = _build_deck([NIDHOGGMON_ID] * 4)
        deck2 = _build_deck()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=10)
        runner.set_phase("Main")

        # Start with exactly 4 cards in hand (meets condition)
        runner.clear_zone(1, "hand")
        for _ in range(4):
            runner.inject_card(1, FILLER_ID, "hand")
        runner.inject_card(1, NIDHOGGMON_ID, "trash")

        assert len(runner.game.player1.hand_cards) == 4

        action = runner.find_action("Nidhoggmon")
        assert action is not None, "Should find trash main action with 4 cards in hand"

        mem_before = runner.game.memory
        runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        assert any(s.card_id == NIDHOGGMON_ID for s in snap.p1_field), \
            "Nidhoggmon should be on field"

        # Cost should be 7 (11 - 4), not 11
        mem_after = runner.game.memory
        cost_paid = mem_before - mem_after
        assert cost_paid == 7, (
            f"Cost reduction should apply: expected 7 (11-4), paid {cost_paid}"
        )


# =====================================================================
# Blocker tests
# =====================================================================

class TestEX7060Blocker:
    """<Blocker>"""

    def test_has_blocker_keyword(self, debug_runner):
        """Nidhoggmon should have the Blocker keyword when on field."""
        deck1 = _build_deck([NIDHOGGMON_ID] * 4)
        deck2 = _build_deck()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=3)

        perm = runner.place_on_field(1, [NIDHOGGMON_ID])
        assert perm.has_keyword("_is_blocker"), (
            "Nidhoggmon should have Blocker keyword"
        )


# =====================================================================
# On Deletion tests
# =====================================================================

class TestEX7060OnDeletion:
    """[On Deletion] You may play 1 level 5 or lower Digimon card with the
    [Dark Dragon] or [Evil Dragon] trait from your trash without paying the cost."""

    def test_on_deletion_plays_dark_dragon_from_trash(self, debug_runner):
        """When deleted, should allow playing a Dark Dragon from trash."""
        deck1 = _build_deck([NIDHOGGMON_ID] * 4 + [DARK_DRAGON_LV4] * 4)
        deck2 = _build_deck()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=5)
        runner.set_phase("Main")

        nidhogg_perm = runner.place_on_field(1, [NIDHOGGMON_ID])
        runner.inject_card(1, DARK_DRAGON_LV4, "trash")

        # Delete Nidhoggmon to trigger On Deletion
        runner.game.player1.delete_permanent(nidhogg_perm)
        # Select play target (non-decline) to exercise the effect
        _select_non_decline(runner)

        snap = runner.snapshot()
        assert any(s.card_id == DARK_DRAGON_LV4 for s in snap.p1_field), (
            f"Dark Dragon Lv4 should be played from trash. Field: "
            f"{[s.card_id for s in snap.p1_field]}"
        )

    def test_on_deletion_plays_evil_dragon_from_trash(self, debug_runner):
        """When deleted, should allow playing an Evil Dragon from trash."""
        deck1 = _build_deck([NIDHOGGMON_ID] * 4 + [EVIL_DRAGON_LV4] * 4)
        deck2 = _build_deck()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=5)
        runner.set_phase("Main")

        nidhogg_perm = runner.place_on_field(1, [NIDHOGGMON_ID])
        runner.inject_card(1, EVIL_DRAGON_LV4, "trash")

        runner.game.player1.delete_permanent(nidhogg_perm)
        _select_non_decline(runner)

        snap = runner.snapshot()
        assert any(s.card_id == EVIL_DRAGON_LV4 for s in snap.p1_field), (
            f"Evil Dragon Lv4 should be played from trash. Field: "
            f"{[s.card_id for s in snap.p1_field]}"
        )

    def test_on_deletion_allows_level_5(self, debug_runner):
        """Level 5 Dark Dragon should be eligible (level 5 or lower)."""
        deck1 = _build_deck([NIDHOGGMON_ID] * 4 + [DARK_DRAGON_LV5] * 4)
        deck2 = _build_deck()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=5)
        runner.set_phase("Main")

        nidhogg_perm = runner.place_on_field(1, [NIDHOGGMON_ID])
        runner.inject_card(1, DARK_DRAGON_LV5, "trash")

        runner.game.player1.delete_permanent(nidhogg_perm)
        _select_non_decline(runner)

        snap = runner.snapshot()
        assert any(s.card_id == DARK_DRAGON_LV5 for s in snap.p1_field), (
            f"Lv5 Dark Dragon should be playable. Field: "
            f"{[s.card_id for s in snap.p1_field]}"
        )

    def test_on_deletion_rejects_level_6(self, debug_runner):
        """Level 6 Dark Dragon should NOT be eligible (must be level 5 or lower)."""
        deck1 = _build_deck([NIDHOGGMON_ID] * 4 + [DARK_DRAGON_LV6] * 4)
        deck2 = _build_deck()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=5)
        runner.set_phase("Main")

        nidhogg_perm = runner.place_on_field(1, [NIDHOGGMON_ID])
        runner.inject_card(1, DARK_DRAGON_LV6, "trash")

        runner.game.player1.delete_permanent(nidhogg_perm)
        # Even with _select_non_decline, there should be no valid target
        _select_non_decline(runner)

        snap = runner.snapshot()
        assert not any(s.card_id == DARK_DRAGON_LV6 for s in snap.p1_field), (
            f"Lv6 Dark Dragon should NOT be playable. Field: "
            f"{[s.card_id for s in snap.p1_field]}"
        )

    def test_on_deletion_rejects_non_dragon_trait(self, debug_runner):
        """Non-Dark Dragon/Evil Dragon Digimon should NOT be eligible."""
        deck1 = _build_deck([NIDHOGGMON_ID] * 4 + [NON_DRAGON_LV4] * 4)
        deck2 = _build_deck()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=5)
        runner.set_phase("Main")

        nidhogg_perm = runner.place_on_field(1, [NIDHOGGMON_ID])
        runner.inject_card(1, NON_DRAGON_LV4, "trash")

        runner.game.player1.delete_permanent(nidhogg_perm)
        _select_non_decline(runner)

        snap = runner.snapshot()
        assert not any(s.card_id == NON_DRAGON_LV4 for s in snap.p1_field), (
            f"Non-dragon Digimon should NOT be playable. Field: "
            f"{[s.card_id for s in snap.p1_field]}"
        )

    def test_on_deletion_is_optional(self, debug_runner):
        """On Deletion effect is optional ('you may') -- declining should work."""
        deck1 = _build_deck([NIDHOGGMON_ID] * 4 + [DARK_DRAGON_LV4] * 4)
        deck2 = _build_deck()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=5)
        runner.set_phase("Main")

        nidhogg_perm = runner.place_on_field(1, [NIDHOGGMON_ID])
        runner.inject_card(1, DARK_DRAGON_LV4, "trash")

        runner.game.player1.delete_permanent(nidhogg_perm)
        # Use auto_resolve which picks Decline/Pass (first legal action)
        runner.auto_resolve(max_steps=20)

        snap = runner.snapshot()
        # Decline means nothing played
        assert not any(s.card_id == DARK_DRAGON_LV4 for s in snap.p1_field), (
            f"Declining should result in nothing played. Field: "
            f"{[s.card_id for s in snap.p1_field]}"
        )

    def test_on_deletion_plays_for_free(self, debug_runner):
        """The played Digimon should not cost any memory."""
        deck1 = _build_deck([NIDHOGGMON_ID] * 4 + [DARK_DRAGON_LV4] * 4)
        deck2 = _build_deck()
        runner = debug_runner(deck1=deck1, deck2=deck2, initial_memory=3)
        runner.set_phase("Main")

        nidhogg_perm = runner.place_on_field(1, [NIDHOGGMON_ID])
        runner.inject_card(1, DARK_DRAGON_LV4, "trash")

        mem_before = runner.game.memory

        runner.game.player1.delete_permanent(nidhogg_perm)
        _select_non_decline(runner)

        mem_after = runner.game.memory
        assert mem_after == mem_before, (
            f"Playing from trash should be free, but memory changed from "
            f"{mem_before} to {mem_after}"
        )

    def test_on_deletion_trait_exact_match(self, debug_runner):
        """Trait matching should be exact, not substring (C# uses EqualsTraits)."""
        CardRegistry.ensure_initialized()
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()

        # Verify Dark Dragon Lv4 has exactly 'Dark Dragon' as a trait
        dark_dragon_cs = db.create_card_source(DARK_DRAGON_LV4)
        assert 'Dark Dragon' in dark_dragon_cs.card_traits, (
            f"Test card {DARK_DRAGON_LV4} should have 'Dark Dragon' trait, "
            f"got {dark_dragon_cs.card_traits}"
        )

        # Verify Evil Dragon Lv4 has exactly 'Evil Dragon' as a trait
        evil_dragon_cs = db.create_card_source(EVIL_DRAGON_LV4)
        assert 'Evil Dragon' in evil_dragon_cs.card_traits, (
            f"Test card {EVIL_DRAGON_LV4} should have 'Evil Dragon' trait, "
            f"got {evil_dragon_cs.card_traits}"
        )

        # Verify non-dragon card does NOT have Dark/Evil Dragon trait
        non_dragon_cs = db.create_card_source(NON_DRAGON_LV4)
        assert 'Dark Dragon' not in non_dragon_cs.card_traits, (
            f"Non-dragon card {NON_DRAGON_LV4} should not have 'Dark Dragon' trait"
        )
        assert 'Evil Dragon' not in non_dragon_cs.card_traits, (
            f"Non-dragon card {NON_DRAGON_LV4} should not have 'Evil Dragon' trait"
        )
