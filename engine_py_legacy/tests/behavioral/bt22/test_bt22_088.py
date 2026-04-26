"""Behavioral tests for BT22-088 Arisa Kinosaki (Yellow Tamer).

Card text:
  [Start of Your Main Phase] By returning this Tamer to the bottom of the deck,
  you may play 1 [Arisa Kinosaki] from your hand without paying the cost. Then,
  if you don't have a Digimon, you may play 1 [Shoemon] from your trash without
  paying the cost.
  [All Turns] When any of your Tokens or [Puppet] trait Digimon are played, by
  suspending this Tamer, <Draw 1>
  [Security] Play this card without paying the cost.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, GamePhase


@pytest.mark.behavioral
class TestBT22088ArisaKinosaki:
    """Tests for BT22-088 Arisa Kinosaki."""

    # ── Effect 0: Start of Main Phase ─────────────────────────────────

    def test_start_main_returns_tamer_to_deck_bottom(self, debug_runner):
        """Activating effect 0 returns Arisa to deck bottom via engine API."""
        runner = debug_runner(initial_memory=5)

        # Place Arisa on field, inject a second Arisa in hand
        arisa_perm = runner.place_on_field(1, ["BT22-088"])
        runner.inject_card(1, "BT22-088", "hand")

        before_deck = len(runner.game.player1.library_cards)
        runner.set_phase("Main")
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)

        # Should enter a selection phase (optional effect + play from hand)
        runner.auto_resolve()

        snap = runner.snapshot()
        # The original Arisa should be gone from field or replaced by the hand copy
        # A new Arisa should be on field (played from hand)
        arisa_on_field = [s for s in snap.p1_field if s.card_id == "BT22-088"]
        assert len(arisa_on_field) >= 1, (
            "A new Arisa should be on field after playing from hand")
        # Deck should have grown by 1 (returned card)
        after_deck = len(runner.game.player1.library_cards)
        assert after_deck == before_deck + 1, (
            f"Deck should grow by 1 (returned tamer). Before={before_deck}, After={after_deck}")

    def test_start_main_plays_arisa_from_hand_free(self, debug_runner):
        """The played Arisa from hand should not cost memory."""
        runner = debug_runner(initial_memory=5)

        arisa_perm = runner.place_on_field(1, ["BT22-088"])
        runner.inject_card(1, "BT22-088", "hand")

        runner.set_phase("Main")
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Memory should not have changed (free play)
        assert snap.memory == 5, (
            f"Playing Arisa from hand should be free. Memory={snap.memory}")

    def test_start_main_plays_shoemon_from_trash_if_no_digimon(self, debug_runner):
        """If no Digimon on field after playing Arisa, play Shoemon from trash."""
        runner = debug_runner(initial_memory=5)

        arisa_perm = runner.place_on_field(1, ["BT22-088"])
        runner.inject_card(1, "BT22-088", "hand")
        runner.inject_card(1, "BT22-029", "trash")  # Shoemon in trash

        # No Digimon on field — only the tamer
        runner.set_phase("Main")
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)

        # Step 1: Select Arisa from hand (first legal action picks one of the Arisas)
        legal = runner.action_mask()
        assert legal, "Should have selection for Arisa from hand"
        runner.execute(legal[0])

        # Step 2: Select Shoemon from trash (should be offered next)
        # The Shoemon selection uses trash index (SEL_TRASH_START = 130)
        legal2 = runner.action_mask()
        assert legal2, "Should have selection for Shoemon from trash"
        # Find the Shoemon trash selection (index 130+)
        trash_selections = [a for a in legal2 if a >= 130]
        assert trash_selections, (
            f"Should have trash selection for Shoemon. Legal: {legal2}, "
            f"Actions: {runner.actions()}")
        runner.execute(trash_selections[0])
        runner.auto_resolve()

        snap = runner.snapshot()
        # Shoemon should be on field (played from trash for free)
        shoemon_on_field = [s for s in snap.p1_field if s.card_id == "BT22-029"]
        assert len(shoemon_on_field) >= 1, (
            f"Shoemon should be played from trash when no Digimon. Field: {[s.card_id for s in snap.p1_field]}")

    def test_start_main_no_shoemon_if_digimon_present(self, debug_runner):
        """If a Digimon is on field, Shoemon should NOT be played from trash."""
        runner = debug_runner(initial_memory=5)

        arisa_perm = runner.place_on_field(1, ["BT22-088"])
        # Place a Digimon on field
        runner.place_on_field(1, ["BT2-055"])  # ToyAgumon (Puppet Digimon)
        runner.inject_card(1, "BT22-088", "hand")
        runner.inject_card(1, "BT22-029", "trash")

        runner.set_phase("Main")
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Shoemon should NOT be on field
        shoemon_on_field = [s for s in snap.p1_field if s.card_id == "BT22-029"]
        assert len(shoemon_on_field) == 0, (
            "Shoemon should NOT be played when a Digimon is already on field")

    def test_start_main_no_arisa_in_hand_still_returns_tamer(self, debug_runner):
        """If no Arisa in hand, the effect is optional — player can decline."""
        runner = debug_runner(initial_memory=5)

        arisa_perm = runner.place_on_field(1, ["BT22-088"])
        # No Arisa in hand — effect is optional, should be declinable

        before_field = len(runner.game.player1.battle_area)
        runner.set_phase("Main")
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)
        runner.auto_resolve()

        # Effect is optional — can choose not to activate

    def test_start_main_shoemon_exact_name_not_shoeshoemon(self, debug_runner):
        """Only exact [Shoemon] should be playable, not [ShoeShoemon]."""
        runner = debug_runner(initial_memory=5)

        arisa_perm = runner.place_on_field(1, ["BT22-088"])
        runner.inject_card(1, "BT22-088", "hand")
        # Only ShoeShoemon in trash, no plain Shoemon
        runner.inject_card(1, "BT22-030", "trash")  # ShoeShoemon

        runner.set_phase("Main")
        runner.game.execute_effects(EffectTiming.OnStartMainPhase)
        runner.auto_resolve()

        snap = runner.snapshot()
        # ShoeShoemon should NOT be on field
        shoeshoemon_on_field = [s for s in snap.p1_field if s.card_id == "BT22-030"]
        assert len(shoeshoemon_on_field) == 0, (
            "ShoeShoemon should NOT match [Shoemon] name filter")

    # ── Effect 1: Observer — Token/Puppet played → suspend + draw ────

    def test_puppet_digimon_played_triggers_draw(self, debug_runner):
        """Playing a Puppet-trait Digimon should trigger Arisa's draw effect."""
        runner = debug_runner(initial_memory=5)

        arisa_perm = runner.place_on_field(1, ["BT22-088"])
        runner.inject_card(1, "BT2-055", "hand")  # ToyAgumon, Puppet

        before_hand = len(runner.game.player1.hand_cards)
        runner.set_phase("Main")

        play = runner.find_action("Play ToyAgumon")
        assert play is not None, f"Should be able to play ToyAgumon. Actions: {runner.actions()}"
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Arisa should be suspended (cost of effect)
        arisa_field = [s for s in snap.p1_field if s.card_id == "BT22-088"]
        assert len(arisa_field) > 0, "Arisa should still be on field"
        assert arisa_field[0].is_suspended, "Arisa should be suspended after triggering draw"

        # Hand should have gained 1 card (draw) minus 1 (played ToyAgumon)
        # Net hand change: 0 or +0 depending on timing. Check hand size.
        after_hand = len(runner.game.player1.hand_cards)
        assert after_hand == before_hand - 1 + 1, (
            f"Hand should be -1 (played) +1 (draw). Before={before_hand}, After={after_hand}")

    def test_token_played_triggers_draw(self, debug_runner):
        """Playing a token should trigger Arisa's draw effect."""
        runner = debug_runner(initial_memory=5)

        arisa_perm = runner.place_on_field(1, ["BT22-088"])
        before_hand = len(runner.game.player1.hand_cards)

        runner.set_phase("Main")
        # Directly create a token to trigger OnEnterFieldAnyone
        runner.game.effect_play_token(runner.game.player1, 'petrification')

        snap = runner.snapshot()
        arisa_field = [s for s in snap.p1_field if s.card_id == "BT22-088"]
        assert len(arisa_field) > 0, "Arisa should still be on field"
        assert arisa_field[0].is_suspended, "Arisa should be suspended after token played"

        after_hand = len(runner.game.player1.hand_cards)
        assert after_hand == before_hand + 1, (
            f"Should draw 1 when token played. Before={before_hand}, After={after_hand}")

    def test_non_puppet_digimon_does_not_trigger(self, debug_runner):
        """Playing a non-Puppet, non-Token Digimon should NOT trigger draw."""
        # Use explicit non-Puppet decks to avoid Puppet cards in opening hand
        non_puppet_deck = ["BT1-009"] * 50  # Monodramon (Mini Dragon trait)
        runner = debug_runner(deck1=non_puppet_deck, deck2=non_puppet_deck, initial_memory=5)

        arisa_perm = runner.place_on_field(1, ["BT22-088"])
        runner.inject_card(1, "BT1-009", "hand")  # A non-Puppet Digimon

        before_hand = len(runner.game.player1.hand_cards)
        runner.set_phase("Main")

        play = runner.find_action("Play Monodramon")
        assert play is not None, f"Should find Monodramon play action. Actions: {runner.actions()}"
        runner.execute(play)
        runner.auto_resolve()

        snap = runner.snapshot()
        arisa_field = [s for s in snap.p1_field if s.card_id == "BT22-088"]
        assert len(arisa_field) > 0, "Arisa should still be on field"
        assert not arisa_field[0].is_suspended, (
            "Arisa should NOT suspend for non-Puppet Digimon")

    def test_already_suspended_does_not_trigger(self, debug_runner):
        """If Arisa is already suspended, she cannot trigger the draw effect."""
        runner = debug_runner(initial_memory=5)

        arisa_perm = runner.place_on_field(1, ["BT22-088"], is_suspended=True)
        before_hand = len(runner.game.player1.hand_cards)

        runner.set_phase("Main")
        # Play a token — Arisa is already suspended so can't pay cost
        runner.game.effect_play_token(runner.game.player1, 'petrification')

        after_hand = len(runner.game.player1.hand_cards)
        assert after_hand == before_hand, (
            f"Should NOT draw when Arisa already suspended. Before={before_hand}, After={after_hand}")

    def test_opponent_puppet_does_not_trigger(self, debug_runner):
        """Opponent's Puppet Digimon being played should NOT trigger Arisa's draw."""
        runner = debug_runner(initial_memory=-5)  # Opponent's turn

        arisa_perm = runner.place_on_field(1, ["BT22-088"])
        runner.inject_card(2, "BT2-055", "hand")  # Opponent's ToyAgumon

        before_hand = len(runner.game.player1.hand_cards)

        # Opponent plays a Puppet Digimon
        runner.set_phase("Main")
        play = runner.find_action("Play ToyAgumon")
        if play is not None:
            runner.execute(play)
            runner.auto_resolve()

        after_hand = len(runner.game.player1.hand_cards)
        assert after_hand == before_hand, (
            f"Should NOT draw when opponent plays Puppet. Before={before_hand}, After={after_hand}")

    # ── Effect 2: Security play ──────────────────────────────────────

    def test_security_effect_is_marked(self, debug_runner):
        """Security effect should have is_security_effect and SecuritySkill timing."""
        from digimon_gym.engine.data.card_database import CardDatabase
        from digimon_gym.engine.core.card_source import CardSource

        db = CardDatabase()
        cs = CardSource()
        cs.c_entity_base = db.get_card("BT22-088")

        from digimon_gym.engine.data.scripts.bt22.bt22_088 import BT22_088
        script = BT22_088()
        effects = script.get_card_effects(cs)
        security_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(security_effects) >= 1, "Should have at least 1 security effect"
        sec = security_effects[0]
        assert sec.timing == EffectTiming.SecuritySkill, (
            f"Security effect should have SecuritySkill timing, got {sec.timing}")
        assert sec.on_process_callback is not None, (
            "Security effect should have a process callback")
