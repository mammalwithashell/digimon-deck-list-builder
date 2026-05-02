"""Behavioral tests for EX11-061 Mirai Kinosaki (Tamer).

Card text:
[Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
[Your Turn] When any of your Digimon digivolve into a [Puppet] trait Digimon,
    by suspending this Tamer, you may play 1 level 3 [Puppet] trait Digimon card
    from your hand without paying the cost. At turn end, delete the Digimon this
    effect played.
Security Effect [Security] Play this card without paying the cost.
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestEX11061StartOfMainPhase:
    """Tests for [Start of Your Main Phase] memory gain effect."""

    def test_gain_memory_when_opponent_has_digimon(self, debug_runner):
        """Should gain 1 memory at start of main phase if opponent has a Digimon."""
        runner = debug_runner(initial_memory=3)

        # Place tamer on P1's field
        runner.place_on_field(1, ["EX11-061"])
        # Place a Digimon on opponent's field
        runner.place_on_field(2, ["EX11-019"])  # Shoemon (Lv3 Puppet)

        game = runner.game
        tamer_card = None
        for perm in game.player1.battle_area:
            if perm.top_card and perm.top_card.card_id == "EX11-061":
                tamer_card = perm.top_card
                break
        assert tamer_card is not None

        # Find the OnStartMainPhase effect
        effects = tamer_card.effect_list(None)
        main_phase_effects = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase]
        assert len(main_phase_effects) == 1

        effect = main_phase_effects[0]
        # Condition: on field, owner's turn
        assert effect.can_use_condition({})

        memory_before = game.memory
        effect.on_process_callback({
            'player': game.player1,
            'game': game,
        })
        assert game.memory == memory_before + 1, "Should gain 1 memory"

    def test_no_memory_when_opponent_has_no_digimon(self, debug_runner):
        """Should NOT gain memory if opponent has no Digimon on field."""
        runner = debug_runner(initial_memory=3)

        runner.place_on_field(1, ["EX11-061"])
        # No Digimon on opponent's field

        game = runner.game
        tamer_card = None
        for perm in game.player1.battle_area:
            if perm.top_card and perm.top_card.card_id == "EX11-061":
                tamer_card = perm.top_card
                break
        assert tamer_card is not None

        effects = tamer_card.effect_list(None)
        main_phase_effects = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase]
        effect = main_phase_effects[0]

        memory_before = game.memory
        effect.on_process_callback({
            'player': game.player1,
            'game': game,
        })
        assert game.memory == memory_before, "Should NOT gain memory without opponent Digimon"

    def test_no_memory_when_opponent_has_only_tamer(self, debug_runner):
        """Should NOT gain memory if opponent only has a Tamer (not a Digimon)."""
        runner = debug_runner(initial_memory=3)

        runner.place_on_field(1, ["EX11-061"])
        # Place a tamer (not Digimon) on opponent's field
        runner.place_on_field(2, ["EX11-061"])  # Another tamer

        game = runner.game
        tamer_card = None
        for perm in game.player1.battle_area:
            if perm.top_card and perm.top_card.card_id == "EX11-061":
                tamer_card = perm.top_card
                break

        effects = tamer_card.effect_list(None)
        main_phase_effects = [e for e in effects if e.timing == EffectTiming.OnStartMainPhase]
        effect = main_phase_effects[0]

        memory_before = game.memory
        effect.on_process_callback({
            'player': game.player1,
            'game': game,
        })
        assert game.memory == memory_before, "Tamers are not Digimon, no memory gain"


@pytest.mark.behavioral
class TestEX11061DigivolveObserver:
    """Tests for [Your Turn] digivolve observer effect."""

    def _get_observer_effect(self, game, tamer_card):
        """Find the digivolve observer effect on the tamer."""
        effects = tamer_card.effect_list(None)
        observers = [e for e in effects if getattr(e, '_is_digivolve_observer', False)]
        assert len(observers) == 1, "Should have exactly 1 digivolve observer effect"
        return observers[0]

    def test_observer_has_correct_flags(self, debug_runner):
        """Observer effect must have _is_digivolve_observer=True and is_optional=True."""
        runner = debug_runner(initial_memory=5)
        runner.place_on_field(1, ["EX11-061"])

        game = runner.game
        tamer_card = None
        for perm in game.player1.battle_area:
            if perm.top_card and perm.top_card.card_id == "EX11-061":
                tamer_card = perm.top_card
                break

        effect = self._get_observer_effect(game, tamer_card)
        assert effect._is_digivolve_observer is True
        assert effect.is_optional is True
        # Should NOT have is_when_digivolving (that's for the digimon itself)
        assert not getattr(effect, 'is_when_digivolving', False), \
            "Observer should not have is_when_digivolving flag"

    def test_condition_passes_for_puppet_digivolve(self, debug_runner):
        """Condition should pass when a friendly Digimon digivolves into Puppet."""
        runner = debug_runner(initial_memory=5)
        tamer_perm = runner.place_on_field(1, ["EX11-061"])
        # Place a Puppet Lv4 on P1's field (represents a digivolved Puppet)
        puppet_perm = runner.place_on_field(1, ["EX11-019", "EX11-021"])  # Shoemon -> Kokeshimon (Puppet Lv4)

        game = runner.game
        tamer_card = tamer_perm.top_card

        effect = self._get_observer_effect(game, tamer_card)
        context = {
            'game': game,
            'player': game.player1,
            'permanent': tamer_perm,
            'digivolved_permanent': puppet_perm,
        }
        assert effect.can_use_condition(context) is True

    def test_condition_fails_for_non_puppet_digivolve(self, debug_runner):
        """Condition should fail when a friendly Digimon digivolves into non-Puppet."""
        runner = debug_runner(initial_memory=5)
        tamer_perm = runner.place_on_field(1, ["EX11-061"])
        # Place a non-Puppet Digimon
        non_puppet_perm = runner.place_on_field(1, ["ST1-03"])  # Agumon (non-Puppet)

        game = runner.game
        tamer_card = tamer_perm.top_card

        effect = self._get_observer_effect(game, tamer_card)
        context = {
            'game': game,
            'player': game.player1,
            'permanent': tamer_perm,
            'digivolved_permanent': non_puppet_perm,
        }
        assert effect.can_use_condition(context) is False, \
            "Should not trigger for non-Puppet digivolve"

    def test_condition_fails_for_opponent_puppet_digivolve(self, debug_runner):
        """Condition should fail when an opponent's Digimon digivolves into Puppet."""
        runner = debug_runner(initial_memory=5)
        tamer_perm = runner.place_on_field(1, ["EX11-061"])
        # Place Puppet on opponent's field
        opp_puppet_perm = runner.place_on_field(2, ["EX11-019", "EX11-021"])

        game = runner.game
        tamer_card = tamer_perm.top_card

        effect = self._get_observer_effect(game, tamer_card)
        context = {
            'game': game,
            'player': game.player1,
            'permanent': tamer_perm,
            'digivolved_permanent': opp_puppet_perm,
        }
        assert effect.can_use_condition(context) is False, \
            "Should not trigger for opponent's digivolve"

    def test_condition_fails_when_tamer_already_suspended(self, debug_runner):
        """Condition should fail when tamer is already suspended (cost cannot be paid)."""
        runner = debug_runner(initial_memory=5)
        tamer_perm = runner.place_on_field(1, ["EX11-061"], is_suspended=True)
        puppet_perm = runner.place_on_field(1, ["EX11-019", "EX11-021"])

        game = runner.game
        tamer_card = tamer_perm.top_card

        effect = self._get_observer_effect(game, tamer_card)
        context = {
            'game': game,
            'player': game.player1,
            'permanent': tamer_perm,
            'digivolved_permanent': puppet_perm,
        }
        assert effect.can_use_condition(context) is False, \
            "Should not activate when tamer is already suspended"

    def test_process_suspends_tamer(self, debug_runner):
        """Process callback should suspend the tamer as cost."""
        runner = debug_runner(initial_memory=5)
        tamer_perm = runner.place_on_field(1, ["EX11-061"])
        puppet_perm = runner.place_on_field(1, ["EX11-019", "EX11-021"])
        # Put a Lv3 Puppet in hand for the play
        runner.inject_card(1, "EX11-019", "hand")

        game = runner.game
        tamer_card = tamer_perm.top_card

        effect = self._get_observer_effect(game, tamer_card)
        assert not tamer_perm.is_suspended, "Tamer starts unsuspended"

        context = {
            'game': game,
            'player': game.player1,
            'permanent': tamer_perm,
            'digivolved_permanent': puppet_perm,
        }
        effect.on_process_callback(context)
        # Resolve any selection phases
        runner.auto_resolve()

        assert tamer_perm.is_suspended, "Tamer should be suspended after activation"

    def test_process_plays_lv3_puppet_from_hand(self, debug_runner):
        """Should allow playing a Lv3 Puppet from hand without paying cost."""
        runner = debug_runner(initial_memory=5)
        tamer_perm = runner.place_on_field(1, ["EX11-061"])
        puppet_perm = runner.place_on_field(1, ["EX11-019", "EX11-021"])

        game = runner.game
        # Inject a Lv3 Puppet into hand
        runner.inject_card(1, "EX11-019", "hand")
        hand_puppet = [c for c in game.player1.hand_cards if c.card_id == "EX11-019"]
        assert len(hand_puppet) > 0, "Should have Lv3 Puppet in hand"

        field_count_before = len(game.player1.battle_area)
        memory_before = game.memory

        tamer_card = tamer_perm.top_card
        effect = self._get_observer_effect(game, tamer_card)

        context = {
            'game': game,
            'player': game.player1,
            'permanent': tamer_perm,
            'digivolved_permanent': puppet_perm,
        }
        effect.on_process_callback(context)
        # Auto-resolve the selection (pick the Puppet from hand)
        runner.auto_resolve()

        # A new permanent should be on field
        assert len(game.player1.battle_area) == field_count_before + 1, \
            "Should have 1 more permanent on field"
        # Memory should not change (free play)
        assert game.memory == memory_before, "Play should be free (no memory cost)"

    def test_eot_deletion_of_played_digimon(self, debug_runner):
        """The Digimon played by this effect should be deleted at end of turn."""
        runner = debug_runner(initial_memory=5)
        tamer_perm = runner.place_on_field(1, ["EX11-061"])
        puppet_perm = runner.place_on_field(1, ["EX11-019", "EX11-021"])

        game = runner.game
        runner.inject_card(1, "EX11-019", "hand")

        tamer_card = tamer_perm.top_card
        effect = self._get_observer_effect(game, tamer_card)

        context = {
            'game': game,
            'player': game.player1,
            'permanent': tamer_perm,
            'digivolved_permanent': puppet_perm,
        }
        effect.on_process_callback(context)
        runner.auto_resolve()

        # Find the newly played permanent (not the tamer, not the digivolved Puppet)
        played_perms = [
            p for p in game.player1.battle_area
            if p is not tamer_perm and p is not puppet_perm
        ]
        assert len(played_perms) == 1, "Should have exactly 1 newly played permanent"
        played_perm = played_perms[0]

        # Check the played permanent has an OnEndTurn deletion effect
        has_eot_delete = False
        if played_perm.top_card:
            for eff in played_perm.top_card.effect_list(None):
                if eff.timing == EffectTiming.OnEndTurn:
                    has_eot_delete = True
                    break
        assert has_eot_delete, "Played Digimon should have an end-of-turn deletion effect"

        # Fire the end-of-turn deletion
        for eff in played_perm.top_card.effect_list(None):
            if eff.timing == EffectTiming.OnEndTurn:
                ctx = {'player': game.player1, 'game': game}
                if eff.can_use_condition is None or eff.can_use_condition(ctx):
                    eff.on_process_callback(ctx)
                break

        assert played_perm not in game.player1.battle_area, \
            "Played Digimon should be deleted at end of turn"

    def test_no_play_without_lv3_puppet_in_hand(self, debug_runner):
        """If no Lv3 Puppet is in hand, the effect just suspends (no play happens)."""
        runner = debug_runner(initial_memory=5)
        tamer_perm = runner.place_on_field(1, ["EX11-061"])
        puppet_perm = runner.place_on_field(1, ["EX11-019", "EX11-021"])
        # Clear hand of any Puppet cards
        game = runner.game
        game.player1.hand_cards = [
            c for c in game.player1.hand_cards
            if not any('Puppet' in t for t in (getattr(c, 'card_traits', []) or []))
        ]

        field_count_before = len(game.player1.battle_area)

        tamer_card = tamer_perm.top_card
        effect = self._get_observer_effect(game, tamer_card)

        context = {
            'game': game,
            'player': game.player1,
            'permanent': tamer_perm,
            'digivolved_permanent': puppet_perm,
        }
        effect.on_process_callback(context)
        runner.auto_resolve()

        # Tamer should still be suspended (cost was paid)
        assert tamer_perm.is_suspended, "Tamer should be suspended even if no play"
        # No new permanent
        assert len(game.player1.battle_area) == field_count_before, \
            "Should not have any new permanent (no valid target in hand)"


@pytest.mark.behavioral
class TestEX11061SecurityEffect:
    """Tests for [Security] play this card without paying the cost."""

    def test_security_effect_exists(self, debug_runner):
        """Should have a security effect that always passes condition."""
        runner = debug_runner(initial_memory=3)
        tamer_perm = runner.place_on_field(1, ["EX11-061"])

        game = runner.game
        tamer_card = tamer_perm.top_card

        effects = tamer_card.effect_list(None)
        security_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(security_effects) == 1, "Should have exactly 1 security effect"
        assert security_effects[0].can_use_condition({}) is True
