"""Behavioral tests for P-216 WaruMonzaemon (Digimon, Black, Lv.5, Cost 6, DP 6000).

Card text:
  Blocker.
  [On Play] You may play 1 [Dark Masters] trait Digimon card from your hand
  without paying the cost. The Digimon this effect played can't digivolve and
  is deleted at turn end.
  [On Deletion] You may play 1 face-up [Dark Masters] trait Digimon card from
  your security stack without paying the cost. At the end of your turn, delete
  the Digimon this effect played.
  Inherited: Blocker.

C# reference: P_216.cs
- Blocker: BlockerSelfStaticEffect (main + inherited)
- On Play: OnEnterFieldAnyone timing, select from hand, PlayPermanentCards
  with activateETB=true, CanNotDigivolveClass, EffectDuration.UntilEachTurnEnd delete
- On Deletion: OnDestroyedAnyone timing, select face-up from security (!IsFlipped),
  PlayPermanentCards, EffectDuration.UntilOwnerTurnEnd delete
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, GamePhase


# Minimal decks. BT15-031 is MetalSeadramon (Dark Masters trait, Lv.6, Digimon).
# ST1-03 is filler (Agumon, Digimon, no Dark Masters trait).
DECK = ["P-216"] * 4 + ["BT15-031"] * 4 + ["ST1-03"] * 42
DECK2 = ["ST1-03"] * 50


@pytest.mark.behavioral
class TestP216Blocker:
    """Tests for P-216 Blocker keyword (main effect)."""

    def test_blocker_effect_present(self):
        """P-216 should have a Blocker effect (non-inherited)."""
        from digimon_gym.engine.data.card_registry import CardRegistry
        from digimon_gym.engine.data.card_database import CardDatabase

        CardRegistry.ensure_initialized()
        db = CardDatabase()
        cs = db.create_card_source("P-216", None)
        effects = cs.effect_list(EffectTiming.NoTiming)

        blocker_effects = [
            e for e in effects
            if getattr(e, '_is_blocker', False)
            and not getattr(e, 'is_inherited_effect', False)
        ]
        assert len(blocker_effects) >= 1, (
            "P-216 should have a non-inherited Blocker effect"
        )


@pytest.mark.behavioral
class TestP216InheritedBlocker:
    """Tests for P-216 inherited Blocker."""

    def test_inherited_blocker_effect_present(self):
        """P-216 should have an inherited Blocker effect."""
        from digimon_gym.engine.data.card_registry import CardRegistry
        from digimon_gym.engine.data.card_database import CardDatabase

        CardRegistry.ensure_initialized()
        db = CardDatabase()
        cs = db.create_card_source("P-216", None)
        effects = cs.effect_list(EffectTiming.NoTiming)

        inherited_blocker = [
            e for e in effects
            if getattr(e, '_is_blocker', False)
            and getattr(e, 'is_inherited_effect', False)
        ]
        assert len(inherited_blocker) >= 1, (
            "P-216 should have an inherited Blocker effect"
        )


@pytest.mark.behavioral
class TestP216OnPlay:
    """Tests for P-216 [On Play] play Dark Masters from hand."""

    def test_on_play_plays_dark_masters_from_hand(self, debug_runner):
        """Playing P-216 should allow playing a Dark Masters Digimon from hand
        without paying the cost."""
        runner = debug_runner(
            deck1=DECK, deck2=DECK2,
            initial_memory=10,
        )
        game = runner.game
        p1 = game.player1

        # Set up: P-216 in hand, BT15-031 (MetalSeadramon, Dark Masters) in hand
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "P-216", "hand")
        runner.inject_card(1, "BT15-031", "hand")

        runner.set_phase("Main")
        hand_before = len(p1.hand_cards)
        field_before = len(p1.battle_area)

        # Play P-216 (WaruMonzaemon)
        action = runner.find_action("WaruMonzaemon")
        assert action is not None, "Should be able to play P-216 WaruMonzaemon"
        runner.execute(action)

        # Should have a selection phase for On Play effect (optional)
        # Auto-resolve should select the Dark Masters card or skip
        runner.auto_resolve()

        # P-216 should be on field
        p216_on_field = [
            p for p in p1.battle_area
            if p.top_card and p.top_card.c_entity_base
            and p.top_card.c_entity_base.card_id == "P-216"
        ]
        assert len(p216_on_field) >= 1, "P-216 should be on the field after playing"

        # BT15-031 should also be on field (played via On Play)
        dm_on_field = [
            p for p in p1.battle_area
            if p.top_card and p.top_card.c_entity_base
            and p.top_card.c_entity_base.card_id == "BT15-031"
        ]
        assert len(dm_on_field) >= 1, (
            "BT15-031 (MetalSeadramon) should be on field via P-216 On Play"
        )

    def test_on_play_does_not_play_non_dark_masters(self, debug_runner):
        """On Play should not offer non-Dark Masters Digimon for selection."""
        runner = debug_runner(
            deck1=DECK, deck2=DECK2,
            initial_memory=10,
        )
        game = runner.game
        p1 = game.player1

        # Only non-Dark Masters Digimon in hand
        runner.clear_zone(1, "hand")
        runner.inject_card(1, "P-216", "hand")
        runner.inject_card(1, "ST1-03", "hand")  # Agumon, no Dark Masters trait

        runner.set_phase("Main")
        field_before = len(p1.battle_area)

        action = runner.find_action("WaruMonzaemon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # Only P-216 should be on field, ST1-03 should NOT have been played
        st1_on_field = [
            p for p in p1.battle_area
            if p.top_card and p.top_card.c_entity_base
            and p.top_card.c_entity_base.card_id == "ST1-03"
        ]
        assert len(st1_on_field) == 0, (
            "Non-Dark Masters Digimon should not be playable via P-216 On Play"
        )

    def test_on_play_is_optional(self, debug_runner):
        """On Play effect should be optional (can skip even with valid targets)."""
        runner = debug_runner(
            deck1=DECK, deck2=DECK2,
            initial_memory=10,
        )
        game = runner.game
        p1 = game.player1

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "P-216", "hand")
        runner.inject_card(1, "BT15-031", "hand")

        runner.set_phase("Main")
        action = runner.find_action("WaruMonzaemon")
        assert action is not None
        runner.execute(action)

        # After playing P-216, should be in a selection phase
        # Check that a "skip" / "pass" action is available
        if game.current_phase not in (GamePhase.Main, GamePhase.End):
            skip = runner.find_action("skip") or runner.find_action("pass") or runner.find_action("no")
            assert skip is not None or 0 in runner.action_mask(), (
                "On Play effect should be optional (skip/pass action available)"
            )
            runner.auto_resolve()

    def test_on_play_applies_cannot_digivolve(self, debug_runner):
        """The Digimon played via On Play should have CANNOT_DIGIVOLVE modifier."""
        runner = debug_runner(
            deck1=DECK, deck2=DECK2,
            initial_memory=10,
        )
        game = runner.game
        p1 = game.player1

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "P-216", "hand")
        runner.inject_card(1, "BT15-031", "hand")

        runner.set_phase("Main")
        action = runner.find_action("WaruMonzaemon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # Find MetalSeadramon on field
        dm_on_field = [
            p for p in p1.battle_area
            if p.top_card and p.top_card.c_entity_base
            and p.top_card.c_entity_base.card_id == "BT15-031"
        ]
        assert len(dm_on_field) >= 1, "BT15-031 should be on field"

        perm = dm_on_field[0]
        # Check CANNOT_DIGIVOLVE modifier
        from digimon_gym.engine.interfaces.modifiers import ModifierType
        has_cannot_digi = game.modifiers.has_modifier(
            perm, ModifierType.CANNOT_DIGIVOLVE
        )
        assert has_cannot_digi, (
            "BT15-031 played via P-216 On Play should have CANNOT_DIGIVOLVE modifier"
        )

    def test_on_play_played_digimon_deleted_at_turn_end(self, debug_runner):
        """The Digimon played via On Play should be deleted at turn end."""
        runner = debug_runner(
            deck1=DECK, deck2=DECK2,
            initial_memory=10,
        )
        game = runner.game
        p1 = game.player1

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "P-216", "hand")
        runner.inject_card(1, "BT15-031", "hand")

        runner.set_phase("Main")
        action = runner.find_action("WaruMonzaemon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # MetalSeadramon should be on field
        dm_on_field = [
            p for p in p1.battle_area
            if p.top_card and p.top_card.c_entity_base
            and p.top_card.c_entity_base.card_id == "BT15-031"
        ]
        assert len(dm_on_field) >= 1, "BT15-031 should be on field before turn end"

        # End the turn — pass action to trigger end-of-turn
        pass_action = runner.find_action("pass")
        if pass_action is not None:
            runner.execute(pass_action)
            runner.auto_resolve()

        # After end of turn, MetalSeadramon should be deleted (in trash)
        dm_on_field_after = [
            p for p in p1.battle_area
            if p.top_card and p.top_card.c_entity_base
            and p.top_card.c_entity_base.card_id == "BT15-031"
        ]
        dm_in_trash = [
            c for c in p1.trash_cards
            if c.c_entity_base and c.c_entity_base.card_id == "BT15-031"
        ]
        assert len(dm_on_field_after) == 0, (
            "BT15-031 should be deleted from field at turn end"
        )
        assert len(dm_in_trash) >= 1, (
            "BT15-031 should be in trash after end-of-turn deletion"
        )

    def test_on_play_does_not_cost_memory(self, debug_runner):
        """Playing via On Play should be free (no memory cost)."""
        runner = debug_runner(
            deck1=DECK, deck2=DECK2,
            initial_memory=10,
        )
        game = runner.game
        p1 = game.player1

        runner.clear_zone(1, "hand")
        runner.inject_card(1, "P-216", "hand")
        runner.inject_card(1, "BT15-031", "hand")

        runner.set_phase("Main")
        # P-216 costs 6, so after playing P-216 memory should be 10-6=4
        # Playing BT15-031 via effect should NOT cost additional memory
        action = runner.find_action("WaruMonzaemon")
        assert action is not None
        runner.execute(action)
        runner.auto_resolve()

        # Memory should be 10 - 6 = 4 (only P-216's cost, not BT15-031's)
        assert game.memory == 4, (
            f"Memory should be 4 after playing P-216 (cost 6) from 10, "
            f"but got {game.memory}. BT15-031 should be free."
        )


@pytest.mark.behavioral
class TestP216OnDeletion:
    """Tests for P-216 [On Deletion] play Dark Masters from security."""

    def test_on_deletion_plays_face_up_dm_from_security(self, debug_runner):
        """Deleting P-216 should allow playing a face-up Dark Masters Digimon
        from security without paying the cost."""
        runner = debug_runner(
            deck1=DECK, deck2=DECK2,
            initial_memory=10,
        )
        game = runner.game
        p1 = game.player1

        # Place P-216 on field
        perm = runner.place_on_field(1, ["P-216"])

        # Add face-up Dark Masters card to security
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        dm_card = db.create_card_source("BT15-031", p1)
        p1.add_to_security_face_up(dm_card)

        security_before = len(p1.security_cards)
        field_before = len(p1.battle_area)

        # Delete P-216
        p1.delete_permanent(perm)
        runner.auto_resolve()

        # BT15-031 should now be on field (played from security)
        dm_on_field = [
            p for p in p1.battle_area
            if p.top_card and p.top_card.c_entity_base
            and p.top_card.c_entity_base.card_id == "BT15-031"
        ]
        assert len(dm_on_field) >= 1, (
            "BT15-031 should be on field after P-216 On Deletion (played from security)"
        )

        # Security should be reduced by 1 (the card was removed)
        assert len(p1.security_cards) == security_before - 1, (
            "Security stack should have 1 fewer card after playing from it"
        )

    def test_on_deletion_rejects_face_down_security(self, debug_runner):
        """On Deletion should NOT allow playing face-down security cards."""
        runner = debug_runner(
            deck1=DECK, deck2=DECK2,
            initial_memory=10,
        )
        game = runner.game
        p1 = game.player1

        # Place P-216 on field
        perm = runner.place_on_field(1, ["P-216"])

        # Add face-DOWN Dark Masters card to security (normal inject, not face-up)
        runner.inject_card(1, "BT15-031", "security_top")

        security_before = len(p1.security_cards)
        field_before = len(p1.battle_area)

        # Delete P-216
        p1.delete_permanent(perm)
        runner.auto_resolve()

        # BT15-031 should NOT be on field (it was face-down)
        dm_on_field = [
            p for p in p1.battle_area
            if p.top_card and p.top_card.c_entity_base
            and p.top_card.c_entity_base.card_id == "BT15-031"
        ]
        assert len(dm_on_field) == 0, (
            "Face-down Dark Masters card in security should NOT be playable "
            "via P-216 On Deletion"
        )

        # Security should still have the card
        assert len(p1.security_cards) == security_before, (
            "Security should be unchanged when no valid face-up target exists"
        )

    def test_on_deletion_eot_delete_at_owner_turn_end(self, debug_runner):
        """Digimon played via On Deletion should be deleted at the end of
        the owner's turn (not opponent's turn)."""
        runner = debug_runner(
            deck1=DECK, deck2=DECK2,
            initial_memory=10,
        )
        game = runner.game
        p1 = game.player1

        # Place P-216 on field
        perm = runner.place_on_field(1, ["P-216"])

        # Add face-up Dark Masters card to security
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        dm_card = db.create_card_source("BT15-031", p1)
        p1.add_to_security_face_up(dm_card)

        # Delete P-216 to trigger On Deletion
        p1.delete_permanent(perm)
        runner.auto_resolve()

        # Verify BT15-031 is on field
        dm_on_field = [
            p for p in p1.battle_area
            if p.top_card and p.top_card.c_entity_base
            and p.top_card.c_entity_base.card_id == "BT15-031"
        ]
        assert len(dm_on_field) >= 1, "BT15-031 should be on field"

        # End the turn (P1's turn)
        pass_action = runner.find_action("pass")
        if pass_action is not None:
            runner.execute(pass_action)
            runner.auto_resolve()

        # After P1's turn end, BT15-031 should be deleted
        dm_on_field_after = [
            p for p in p1.battle_area
            if p.top_card and p.top_card.c_entity_base
            and p.top_card.c_entity_base.card_id == "BT15-031"
        ]
        dm_in_trash = [
            c for c in p1.trash_cards
            if c.c_entity_base and c.c_entity_base.card_id == "BT15-031"
        ]
        assert len(dm_on_field_after) == 0, (
            "BT15-031 should be deleted from field at end of owner's turn"
        )
        assert len(dm_in_trash) >= 1, (
            "BT15-031 should be in trash after end-of-turn deletion"
        )

    def test_on_deletion_is_optional(self, debug_runner):
        """On Deletion effect should be optional."""
        runner = debug_runner(
            deck1=DECK, deck2=DECK2,
            initial_memory=10,
        )
        game = runner.game
        p1 = game.player1

        # Place P-216 on field
        perm = runner.place_on_field(1, ["P-216"])

        # Add face-up Dark Masters card to security
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        dm_card = db.create_card_source("BT15-031", p1)
        p1.add_to_security_face_up(dm_card)

        # Delete P-216 to trigger On Deletion
        p1.delete_permanent(perm)

        # After deletion, if in a selection phase, skip action should be available
        if game.current_phase not in (GamePhase.Main, GamePhase.End):
            skip = runner.find_action("skip") or runner.find_action("pass") or runner.find_action("no")
            assert skip is not None or 0 in runner.action_mask(), (
                "On Deletion effect should be optional (skip/pass action available)"
            )
        runner.auto_resolve()

    def test_on_deletion_does_not_play_non_digimon(self, debug_runner):
        """On Deletion should not play non-Digimon Dark Masters cards from security."""
        # Use a deck WITHOUT Dark Masters Digimon to avoid false matches in security
        no_dm_deck = ["P-216"] * 4 + ["ST1-03"] * 46
        runner = debug_runner(
            deck1=no_dm_deck, deck2=DECK2,
            initial_memory=10,
        )
        game = runner.game
        p1 = game.player1

        # Clear security to have a clean slate
        runner.clear_zone(1, "security")

        # Place P-216 on field
        perm = runner.place_on_field(1, ["P-216"])

        # Add face-up EX10-072 (Spiral Mountain, Option, Dark Masters trait)
        from digimon_gym.engine.data.card_database import CardDatabase
        db = CardDatabase()
        opt_card = db.create_card_source("EX10-072", p1)
        p1.add_to_security_face_up(opt_card)

        security_before = len(p1.security_cards)

        # Delete P-216
        p1.delete_permanent(perm)
        runner.auto_resolve()

        # EX10-072 is an Option, not a Digimon — should NOT be played
        # Security should be unchanged
        assert len(p1.security_cards) == security_before, (
            "Non-Digimon Dark Masters card should not be played from security"
        )
