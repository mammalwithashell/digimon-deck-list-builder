"""Behavioral tests for BT9-082 Ordinemon (Lv.7 Purple/Yellow Digimon).

Card text:
  [When Digivolving] When DNA digivolving, delete 1 of your opponent's level
      6 or higher Digimon and all of their level 5 or lower Digimon. For each
      Digimon deleted by this effect, <Recovery +1 (Deck)> (Place the top
      card of your deck on top of your security stack.)
  [On Deletion] You may trash the top card of your security stack to play
      this card without paying its memory cost.

C# reference: DCGO BT9_082.cs
  - [When Digivolving] ActivateClass with:
      - CanUseCondition: CanTriggerWhenDigivolving + IsJogress (DNA only)
      - CanActivateCondition: permanent on battle area AND (Lv6+ target OR Lv5- target)
      - ActivateCoroutine: select 1 Lv6+ (maxCount=1, canNoSelect=false),
        concat all Lv5- (auto-delete), destroy all, recovery(deletedCount)
  - [On Deletion] ActivateClass with:
      - CanUseCondition: CanTriggerOnDeletion
      - CanActivateCondition: IsExistOnTrash(card) && SecurityCards.Count >= 1
      - Optional (setUpActivateClass is_optional=true)
      - Activate: IDestroySecurity(top 1) then PlayPermanentCards from Trash, payCost=false

Key faithfulness points tested:
  - C1 DNA gate: does NOT fire on normal digivolve (no is_dna_digivolve flag).
  - C1 Lv6+ choice: the mandatory Lv6+ deletion is exposed via selection phase.
  - C1 Lv5- mass auto-delete.
  - C1 Recovery +1 per deleted (count matches actual deletions).
  - C1 Recovery 0 when no opponent Digimon exist.
  - C2 On-deletion condition: requires card in trash AND security >= 1.
  - C2 Trashes top security, plays BT9-082 from trash without paying cost.
"""

from __future__ import annotations

import pytest

from engine_py_legacy.engine.data.enums import EffectTiming


# Card IDs used in tests
BT9_082 = "BT9-082"          # Ordinemon, Lv.7 Purple/Yellow, DP 15000
ST1_03 = "ST1-03"            # Agumon, Lv.3 Red, 2000 DP
ST1_07 = "ST1-07"            # Greymon, Lv.4 Red, 4000 DP
ST1_09 = "ST1-09"            # MetalGreymon, Lv.5 Red, 7000 DP
ST1_11 = "ST1-11"            # WarGreymon, Lv.6 Red, 12000 DP
ST1_10 = "ST1-10"            # Phoenixmon, Lv.6 Red, 12000 DP
ST1_12 = "ST1-12"            # Tamer card
FILLER = [ST1_03] * 50


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _get_when_digivolving_effect(perm):
    """Return BT9-082's [When Digivolving] mass delete effect."""
    top = perm.top_card
    effects = top.effect_list(None)
    wd = [
        e for e in effects
        if getattr(e, 'is_when_digivolving', False)
        and e.on_process_callback is not None
    ]
    return wd[0] if wd else None


def _get_on_deletion_effect_from_cardsource(card_source):
    """Return BT9-082's [On Deletion] self-play effect from a CardSource.

    Uses the CardSource directly because by the time the effect fires the
    permanent no longer exists on field.
    """
    effects = card_source.effect_list(None)
    od = [
        e for e in effects
        if getattr(e, 'is_on_deletion', False)
        and e.on_process_callback is not None
    ]
    return od[0] if od else None


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------

@pytest.mark.behavioral
class TestBT9082EffectStructure:
    """Script registers the expected effects."""

    def test_has_when_digivolving_effect(self, debug_runner):
        runner = debug_runner(
            deck1=[BT9_082] * 4 + FILLER,
            deck2=FILLER,
            initial_memory=5,
        )
        perm = runner.place_on_field(1, [BT9_082])
        assert _get_when_digivolving_effect(perm) is not None, (
            "BT9-082 must have a [When Digivolving] effect"
        )

    def test_has_on_deletion_effect(self, debug_runner):
        runner = debug_runner(
            deck1=[BT9_082] * 4 + FILLER,
            deck2=FILLER,
            initial_memory=5,
        )
        from engine_py_legacy.engine.data.card_database import CardDatabase
        from engine_py_legacy.engine.data.card_registry import CardRegistry
        CardRegistry.ensure_initialized()
        db = CardDatabase()
        cs = db.create_card_source(BT9_082, runner.game.player1)

        assert _get_on_deletion_effect_from_cardsource(cs) is not None, (
            "BT9-082 must have an [On Deletion] effect"
        )

    def test_on_deletion_is_optional(self, debug_runner):
        """The [On Deletion] 'you may' clause must be marked optional."""
        runner = debug_runner(deck1=[BT9_082] * 4 + FILLER, deck2=FILLER)
        from engine_py_legacy.engine.data.card_database import CardDatabase
        from engine_py_legacy.engine.data.card_registry import CardRegistry
        CardRegistry.ensure_initialized()
        db = CardDatabase()
        cs = db.create_card_source(BT9_082, runner.game.player1)

        eff = _get_on_deletion_effect_from_cardsource(cs)
        assert eff is not None
        assert eff.is_optional is True, (
            "[On Deletion] 'you may trash security' must be optional"
        )


@pytest.mark.behavioral
class TestBT9082DnaGate:
    """Clause 1: [When Digivolving] must only fire when DNA digivolving."""

    def test_condition_requires_is_dna_digivolve_flag(self, debug_runner):
        runner = debug_runner(
            deck1=[BT9_082] * 4 + FILLER,
            deck2=FILLER,
            initial_memory=5,
        )
        perm = runner.place_on_field(1, [BT9_082])
        eff = _get_when_digivolving_effect(perm)
        assert eff is not None

        # No DNA context — should NOT fire
        result = eff.can_use_condition({
            'game': runner.game,
            'player': runner.game.player1,
            'permanent': perm,
        })
        assert result is False, (
            "[When Digivolving] must fail without is_dna_digivolve flag"
        )

        # Explicit False — should NOT fire
        result = eff.can_use_condition({
            'game': runner.game,
            'player': runner.game.player1,
            'permanent': perm,
            'is_dna_digivolve': False,
        })
        assert result is False, (
            "[When Digivolving] must fail with is_dna_digivolve=False"
        )

        # DNA True — should fire
        result = eff.can_use_condition({
            'game': runner.game,
            'player': runner.game.player1,
            'permanent': perm,
            'is_dna_digivolve': True,
        })
        assert result is True, (
            "[When Digivolving] must activate with is_dna_digivolve=True"
        )


@pytest.mark.behavioral
class TestBT9082DnaMassDelete:
    """Clause 1: delete 1 Lv6+ Digimon + all Lv5- Digimon + recovery per deletion."""

    def _fire_wd(self, runner, perm):
        eff = _get_when_digivolving_effect(perm)
        assert eff is not None
        eff.on_process_callback({
            'game': runner.game,
            'player': runner.game.player1,
            'permanent': perm,
            'card': perm.top_card,
            'is_dna_digivolve': True,
            'digivolved_permanent': perm,
        })

    def test_deletes_all_lv5_or_lower_when_no_high(self, debug_runner):
        """With only Lv5- opponents, skip the Lv6+ selection and mass-delete."""
        runner = debug_runner(
            deck1=[BT9_082] * 4 + FILLER,
            deck2=[ST1_03] * 50,
            initial_memory=5,
        )

        ordinemon = runner.place_on_field(1, [BT9_082])
        # Three Lv5- opponent Digimon (3, 4, 5)
        runner.place_on_field(2, [ST1_03])  # Lv.3
        runner.place_on_field(2, [ST1_07])  # Lv.4
        runner.place_on_field(2, [ST1_09])  # Lv.5

        game = runner.game
        p2_count_before = sum(1 for p in game.player2.battle_area if p.is_digimon)
        assert p2_count_before == 3

        self._fire_wd(runner, ordinemon)
        runner.auto_resolve()

        p2_count_after = sum(1 for p in game.player2.battle_area if p.is_digimon)
        assert p2_count_after == 0, (
            f"All 3 Lv5- opponent Digimon should be deleted, {p2_count_after} remain"
        )

    def test_deletes_lv5_or_lower_and_high_target(self, debug_runner):
        """With both Lv6+ and Lv5- opponents, chosen Lv6+ + all Lv5- are deleted."""
        runner = debug_runner(
            deck1=[BT9_082] * 4 + FILLER,
            deck2=[ST1_03] * 50,
            initial_memory=5,
        )

        ordinemon = runner.place_on_field(1, [BT9_082])
        runner.place_on_field(2, [ST1_03])  # Lv.3
        runner.place_on_field(2, [ST1_09])  # Lv.5
        runner.place_on_field(2, [ST1_11])  # Lv.6 (Lv6+ target)

        game = runner.game

        self._fire_wd(runner, ordinemon)
        # Script must have entered selection phase for the mandatory Lv6+ target.
        from engine_py_legacy.engine.data.enums import GamePhase
        assert game.current_phase == GamePhase.SelectTarget, (
            f"Lv6+ deletion must be a player choice (SelectTarget phase); "
            f"got {game.current_phase}"
        )
        runner.auto_resolve()

        p2_digimon = [p for p in game.player2.battle_area if p.is_digimon]
        # All 3 should be gone: 1 Lv6+ picked + 2 Lv5- auto-deleted
        assert len(p2_digimon) == 0, (
            f"All 3 opponent Digimon should be deleted; {len(p2_digimon)} remain"
        )

    def test_preserves_extra_high_level_targets(self, debug_runner):
        """Only 1 Lv6+ is deleted; additional Lv6+ survive."""
        runner = debug_runner(
            deck1=[BT9_082] * 4 + FILLER,
            deck2=[ST1_03] * 50,
            initial_memory=5,
        )

        ordinemon = runner.place_on_field(1, [BT9_082])
        # Two Lv.6 opponent Digimon — only 1 gets deleted
        runner.place_on_field(2, [ST1_11])  # Lv.6 WarGreymon
        runner.place_on_field(2, [ST1_10])  # Lv.6 Phoenixmon

        game = runner.game

        self._fire_wd(runner, ordinemon)
        runner.auto_resolve()

        p2_digimon = [p for p in game.player2.battle_area if p.is_digimon]
        assert len(p2_digimon) == 1, (
            f"Only 1 Lv6+ should be deleted, leaving 1 survivor; got {len(p2_digimon)}"
        )

    def test_recovery_equals_deletion_count(self, debug_runner):
        """Recovery +1 per deleted Digimon — 3 deletions => +3 security from deck."""
        runner = debug_runner(
            deck1=[BT9_082] * 4 + FILLER,
            deck2=[ST1_03] * 50,
            initial_memory=5,
        )

        ordinemon = runner.place_on_field(1, [BT9_082])
        runner.place_on_field(2, [ST1_03])  # Lv.3
        runner.place_on_field(2, [ST1_09])  # Lv.5
        runner.place_on_field(2, [ST1_11])  # Lv.6 (chosen target)

        game = runner.game
        sec_before = len(game.player1.security_cards)
        deck_before = len(game.player1.library_cards)

        self._fire_wd(runner, ordinemon)
        runner.auto_resolve()

        sec_after = len(game.player1.security_cards)
        deck_after = len(game.player1.library_cards)

        assert sec_after - sec_before == 3, (
            f"Recovery +3 expected for 3 deletions (1 Lv6+ + 2 Lv5-); "
            f"sec went from {sec_before} to {sec_after}"
        )
        assert deck_before - deck_after == 3, (
            f"3 deck cards should have moved to security; deck went from "
            f"{deck_before} to {deck_after}"
        )

    def test_no_recovery_when_no_opponent_digimon(self, debug_runner):
        """Effect activates but deletes nothing => recovery 0."""
        runner = debug_runner(
            deck1=[BT9_082] * 4 + FILLER,
            deck2=FILLER,
            initial_memory=5,
        )

        ordinemon = runner.place_on_field(1, [BT9_082])

        game = runner.game
        sec_before = len(game.player1.security_cards)

        self._fire_wd(runner, ordinemon)
        runner.auto_resolve()

        sec_after = len(game.player1.security_cards)
        assert sec_after == sec_before, (
            f"No deletions => no recovery; sec changed from {sec_before} to {sec_after}"
        )

    def test_ignores_opponent_tamers(self, debug_runner):
        """Clause 1 only targets Digimon, never Tamers."""
        runner = debug_runner(
            deck1=[BT9_082] * 4 + FILLER,
            deck2=[ST1_12] * 50,
            initial_memory=5,
        )

        ordinemon = runner.place_on_field(1, [BT9_082])
        runner.place_on_field(2, [ST1_12])  # Tamer
        runner.place_on_field(2, [ST1_03])  # Lv.3 Digimon

        game = runner.game
        p2_before = len(game.player2.battle_area)
        assert p2_before == 2

        self._fire_wd(runner, ordinemon)
        runner.auto_resolve()

        # Tamer survives, Lv.3 Digimon deleted
        p2_after = [
            p for p in game.player2.battle_area
            if p.top_card and p.top_card.card_id == ST1_12
        ]
        assert len(p2_after) == 1, "Tamer should not be affected"
        remaining_digimon = [p for p in game.player2.battle_area if p.is_digimon]
        assert len(remaining_digimon) == 0, "All Lv5- Digimon should be deleted"

    def test_recovery_counts_only_actual_deletions(self, debug_runner):
        """Recovery must reflect successful deletions; protected Digimon must
        not inflate the recovery count.

        Grants CANNOT_BE_DESTROYED to one of two Lv5- opponents and expects
        recovery = 1 (only the unprotected one is deleted).
        """
        from engine_py_legacy.engine.interfaces.modifiers import ModifierType

        runner = debug_runner(
            deck1=[BT9_082] * 4 + FILLER,
            deck2=[ST1_03] * 50,
            initial_memory=5,
        )

        ordinemon = runner.place_on_field(1, [BT9_082])
        protected = runner.place_on_field(2, [ST1_03])  # Lv.3 — protected
        runner.place_on_field(2, [ST1_07])               # Lv.4 — unprotected

        game = runner.game
        # Target-specific protection: only `protected` is immune
        game.register_modifier(
            protected,
            ModifierType.CANNOT_BE_DESTROYED,
            condition=lambda perm, ctx, _p=protected: perm is _p,
            value_fn=lambda: True,
            expiry='permanent',
        )

        sec_before = len(game.player1.security_cards)

        self._fire_wd(runner, ordinemon)
        runner.auto_resolve()

        sec_after = len(game.player1.security_cards)
        # Only 1 real deletion — the unprotected Lv.4
        assert sec_after - sec_before == 1, (
            f"Recovery should match actual deletions (1); "
            f"sec went from {sec_before} to {sec_after}"
        )
        # Protected Digimon survives
        p2_digimon = [p for p in game.player2.battle_area if p.is_digimon]
        assert len(p2_digimon) == 1, (
            "Protected Lv.3 Digimon should have survived deletion"
        )


@pytest.mark.behavioral
class TestBT9082SelfDeletionTrigger:
    """Integration: verify [On Deletion] actually fires when the card is
    deleted via the normal engine delete path (execute_deletion_effects)."""

    def test_on_deletion_fires_from_battle_area_delete(self, debug_runner):
        """Deleting the Ordinemon permanent triggers the replay-from-trash effect."""
        runner = debug_runner(
            deck1=[BT9_082] * 4 + FILLER,
            deck2=FILLER,
            initial_memory=5,
        )

        ordinemon = runner.place_on_field(1, [BT9_082])
        top_cs = ordinemon.top_card

        game = runner.game
        sec_top_before = game.player1.security_cards[0]
        sec_count_before = len(game.player1.security_cards)

        # Trigger normal engine deletion
        game.player1.delete_permanent(ordinemon, is_opponent_effect=False)
        runner.auto_resolve()

        # Security top must have moved to trash
        assert sec_top_before in game.player1.trash_cards, (
            "Top security card should be trashed by [On Deletion] cost"
        )
        assert len(game.player1.security_cards) == sec_count_before - 1

        # BT9-082 must be back on field
        on_field = [
            p for p in game.player1.battle_area
            if p.top_card and p.top_card.card_id == BT9_082
        ]
        assert len(on_field) >= 1, (
            "BT9-082 should have replayed itself from trash after deletion"
        )
        # And no longer in trash
        assert top_cs not in game.player1.trash_cards, (
            "BT9-082 must be removed from trash after replaying"
        )


@pytest.mark.behavioral
class TestBT9082OnDeletionCondition:
    """Clause 2: [On Deletion] condition checks."""

    def test_condition_fails_when_no_security(self, debug_runner):
        """Effect cannot activate when player has 0 security."""
        runner = debug_runner(
            deck1=[BT9_082] * 4 + FILLER,
            deck2=FILLER,
            initial_memory=5,
        )
        from engine_py_legacy.engine.data.card_database import CardDatabase
        from engine_py_legacy.engine.data.card_registry import CardRegistry
        CardRegistry.ensure_initialized()
        db = CardDatabase()

        cs = db.create_card_source(BT9_082, runner.game.player1)
        # Simulate card in trash
        runner.game.player1.trash_cards.append(cs)
        # Empty security
        runner.clear_zone(1, "security")

        eff = _get_on_deletion_effect_from_cardsource(cs)
        assert eff is not None
        ctx = {
            'game': runner.game,
            'player': runner.game.player1,
            'card': cs,
            'permanent': None,
        }
        assert eff.can_use_condition(ctx) is False, (
            "Condition must fail with empty security stack"
        )

    def test_condition_fails_when_card_still_on_field(self, debug_runner):
        """If the card is still on the field, the deletion effect must not fire."""
        runner = debug_runner(
            deck1=[BT9_082] * 4 + FILLER,
            deck2=FILLER,
            initial_memory=5,
        )
        perm = runner.place_on_field(1, [BT9_082])

        eff = _get_on_deletion_effect_from_cardsource(perm.top_card)
        assert eff is not None

        ctx = {
            'game': runner.game,
            'player': runner.game.player1,
            'card': perm.top_card,
            'permanent': perm,
        }
        assert eff.can_use_condition(ctx) is False, (
            "[On Deletion] must not fire while the card is still on field"
        )

    def test_condition_passes_when_in_trash_with_security(self, debug_runner):
        runner = debug_runner(
            deck1=[BT9_082] * 4 + FILLER,
            deck2=FILLER,
            initial_memory=5,
        )
        from engine_py_legacy.engine.data.card_database import CardDatabase
        from engine_py_legacy.engine.data.card_registry import CardRegistry
        CardRegistry.ensure_initialized()
        db = CardDatabase()

        cs = db.create_card_source(BT9_082, runner.game.player1)
        runner.game.player1.trash_cards.append(cs)
        # Ensure security is non-empty (default opening has 5)
        assert len(runner.game.player1.security_cards) >= 1

        eff = _get_on_deletion_effect_from_cardsource(cs)
        assert eff is not None

        ctx = {
            'game': runner.game,
            'player': runner.game.player1,
            'card': cs,
            'permanent': None,
        }
        assert eff.can_use_condition(ctx) is True, (
            "Condition must pass when card is in trash and security >= 1"
        )


@pytest.mark.behavioral
class TestBT9082OnDeletionProcess:
    """Clause 2: execution of [On Deletion] trashes top security and plays from trash."""

    def test_trashes_top_security_and_plays_from_trash(self, debug_runner):
        """Processing the effect: trash top of security, put card on field from trash."""
        runner = debug_runner(
            deck1=[BT9_082] * 4 + FILLER,
            deck2=FILLER,
            initial_memory=5,
        )
        from engine_py_legacy.engine.data.card_database import CardDatabase
        from engine_py_legacy.engine.data.card_registry import CardRegistry
        CardRegistry.ensure_initialized()
        db = CardDatabase()

        # Put BT9-082 in trash as if it was just deleted
        cs = db.create_card_source(BT9_082, runner.game.player1)
        runner.game.player1.trash_cards.append(cs)

        game = runner.game
        sec_before = len(game.player1.security_cards)
        trash_before = len(game.player1.trash_cards)
        field_before = len(game.player1.battle_area)
        sec_top_before = game.player1.security_cards[0]

        eff = _get_on_deletion_effect_from_cardsource(cs)
        assert eff is not None

        eff.on_process_callback({
            'game': game,
            'player': game.player1,
            'card': cs,
            'permanent': None,
            'deleted_permanent': None,
        })
        runner.auto_resolve()

        # Security: top card removed & sent to trash
        assert len(game.player1.security_cards) == sec_before - 1, (
            "Top security card must be removed"
        )
        assert sec_top_before in game.player1.trash_cards, (
            "Trashed security card must be in trash"
        )

        # BT9-082 must have left trash
        assert cs not in game.player1.trash_cards, (
            "BT9-082 should be removed from trash after replaying"
        )

        # BT9-082 must now be a permanent on the field
        field_perms = [
            p for p in game.player1.battle_area
            if p.top_card and p.top_card.card_id == BT9_082
        ]
        assert len(field_perms) >= 1, (
            f"BT9-082 should be on field after [On Deletion]; "
            f"field has {len(game.player1.battle_area)} perms (was {field_before})"
        )

    def test_memory_not_spent_when_replaying(self, debug_runner):
        """Replaying from trash must not cost memory (play cost = 15, but free)."""
        runner = debug_runner(
            deck1=[BT9_082] * 4 + FILLER,
            deck2=FILLER,
            initial_memory=0,
        )
        from engine_py_legacy.engine.data.card_database import CardDatabase
        from engine_py_legacy.engine.data.card_registry import CardRegistry
        CardRegistry.ensure_initialized()
        db = CardDatabase()

        cs = db.create_card_source(BT9_082, runner.game.player1)
        runner.game.player1.trash_cards.append(cs)

        game = runner.game
        game.memory = 0  # zero memory before
        eff = _get_on_deletion_effect_from_cardsource(cs)
        eff.on_process_callback({
            'game': game,
            'player': game.player1,
            'card': cs,
            'permanent': None,
            'deleted_permanent': None,
        })
        runner.auto_resolve()

        # Play was free — memory should not have gone below 0 by 15
        assert game.memory >= -1, (
            f"Memory must not be spent replaying from trash; got {game.memory}"
        )
        # And the card should be on field
        assert any(
            p.top_card and p.top_card.card_id == BT9_082
            for p in game.player1.battle_area
        ), "BT9-082 should be on the field after free replay"
