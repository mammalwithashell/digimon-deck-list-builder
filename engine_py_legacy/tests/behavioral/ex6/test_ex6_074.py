"""Behavioral tests for EX6-074 Mirei Mikagura (Tamer, Purple/Yellow, Cost 4).

Card text:
  [Your Turn] When one of your Digimon with the [Holy Beast], [Archangel] or
      [Fallen Angel] trait is played, by suspending this Tamer, gain 1 memory.
      Then, 1 of your Digimon may digivolve into [Angewomon] or [LadyDevimon]
      in your trash with the cost reduced by 1.
  [End of Your Turn] [Once Per Turn] 2 of your Digimon may DNA digivolve into a
      Digimon card in your hand.
  [Security] Play this card without paying the cost.

Clauses tested:
  C1 (OnPlayed trait trigger):
     - Effect exists at OnEnterFieldAnyone timing, is_optional
     - Condition: [Your Turn] only, tamer on field, tamer unsuspended
     - Condition: played_card has Holy Beast / Archangel / Fallen Angel trait
     - Condition: event_player is tamer owner (own Digimon only)
     - Process: suspends tamer (cost), gains 1 memory
     - Process: lets PLAYER select which Digimon to digivolve
     - Process: calls effect_digivolve_from_hand with include_trash=True,
       cost_reduction=1
     - Filter: only Angewomon / LadyDevimon cards valid
  C2 (EoT DNA digivolve):
     - Effect exists at OnEndTurn, is_optional, OPT (max_count_per_turn=1)
     - Has hash_string for OPT persistence
     - Condition: tamer on field, owner's turn
     - Process: calls effect_dna_digivolve_from_hand
  C3 (Security):
     - Security effect with is_security_effect = True
     - Playing from security plays the tamer for free

Key faithfulness points:
  - Tamer must be UNSUSPENDED to activate (cost is suspend)
  - Player picks BOTH the digivolve target AND the trash card (no auto-select)
  - include_trash=True is required (card says "in your trash")
  - cost_reduction=1 on digivolve
  - Trait filter matches Holy Beast / Archangel / Fallen Angel
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


# Card IDs used in tests
EX6_074 = "EX6-074"              # Card under test (Mirei Mikagura)
GATOMON_HOLY_BEAST = "BT2-036"   # Gatomon Lv.4 Yellow, Holy Beast trait (cost 5)
MAGNAANGEMON_ARCHANGEL = "BT14-037"  # MagnaAngemon Lv.5 Yellow, Archangel trait
DEVIMON_FALLEN_ANGEL = "BT2-074" # Devimon Lv.4 Purple, Fallen Angel trait (cost 6)
AGUMON_GENERIC = "ST1-03"        # Agumon Lv.3 Red (no matching traits)
ANGEWOMON = "BT2-037"            # Angewomon Lv.5 Yellow (digivolves from Yellow Lv.4)
LADYDEVIMON = "BT3-088"          # LadyDevimon Lv.5 Purple (digivolves from Purple Lv.4)
PAILDRAMON_RP = "BT20-016"       # Paildramon DNA: Red Lv.4 + Purple Lv.4
PURPLE_LV4 = "BT2-074"           # Devimon Lv.4 Purple (can be DNA material)
RED_LV4 = "BT1-015"              # Greymon Lv.4 Red (can be DNA material)


def _find_onplay_effect(card):
    """Return the OnEnterFieldAnyone trait-triggered effect (Clause 1)."""
    effects = card.effect_list(None)
    matches = [
        e for e in effects
        if e.timing == EffectTiming.OnEnterFieldAnyone
        and not getattr(e, 'is_security_effect', False)
        and not getattr(e, 'is_inherited_effect', False)
    ]
    return matches[0] if matches else None


def _find_eot_effect(card):
    """Return the OnEndTurn DNA digivolve effect (Clause 2)."""
    effects = card.effect_list(None)
    matches = [
        e for e in effects
        if e.timing == EffectTiming.OnEndTurn
        and not getattr(e, 'is_security_effect', False)
    ]
    return matches[0] if matches else None


def _find_security_effect(card):
    """Return the security effect (Clause 3)."""
    effects = card.effect_list(None)
    matches = [e for e in effects if getattr(e, 'is_security_effect', False)]
    return matches[0] if matches else None


# ======================================================================
# Clause 1: [Your Turn] OnPlayed trait trigger
# ======================================================================


@pytest.mark.behavioral
class TestEX6074Clause1Existence:
    """Effect definition and metadata for Clause 1."""

    def test_onplay_effect_exists(self, debug_runner):
        """Should have an OnEnterFieldAnyone effect for the trait trigger."""
        runner = debug_runner(initial_memory=5)
        tamer = runner.place_on_field(1, [EX6_074])
        effect = _find_onplay_effect(tamer.top_card)
        assert effect is not None, (
            f"EX6-074 should have an OnEnterFieldAnyone effect (Clause 1). "
            f"Effects: {[(getattr(e, '_effect_name', '?'), e.timing) for e in tamer.top_card.effect_list(None)]}"
        )

    def test_onplay_effect_is_optional(self, debug_runner):
        """'may digivolve' + 'by suspending this Tamer' --> optional."""
        runner = debug_runner(initial_memory=5)
        tamer = runner.place_on_field(1, [EX6_074])
        effect = _find_onplay_effect(tamer.top_card)
        assert effect is not None
        assert effect.is_optional, (
            "Effect must be optional (card text has 'may' and cost payment)"
        )


@pytest.mark.behavioral
class TestEX6074Clause1Condition:
    """Condition function checks for Clause 1."""

    def _build_ctx(self, game, tamer, played_card, event_player):
        """Build a context dict matching what the engine passes."""
        return {
            'game': game,
            'player': game.player1,
            'permanent': tamer,
            'card': tamer.top_card,
            'turn_player': game.turn_player,
            'opponent_player': game.opponent_player,
            'event_player': event_player,
            'played_card': played_card,
        }

    def test_holy_beast_trait_passes_condition(self, debug_runner):
        """A played Digimon with Holy Beast trait triggers the condition."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        tamer = runner.place_on_field(1, [EX6_074])
        effect = _find_onplay_effect(tamer.top_card)

        # Simulate playing a Holy Beast Digimon (Gatomon)
        from engine_py_legacy.engine.data.card_database import CardDatabase
        gatomon = CardDatabase().create_card_source(GATOMON_HOLY_BEAST, game.player1)
        ctx = self._build_ctx(game, tamer, gatomon, game.player1)

        assert effect.can_use_condition(ctx), (
            "Holy Beast trait should satisfy the trigger condition"
        )

    def test_archangel_trait_passes_condition(self, debug_runner):
        """A played Digimon with Archangel trait triggers the condition."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        tamer = runner.place_on_field(1, [EX6_074])
        effect = _find_onplay_effect(tamer.top_card)

        from engine_py_legacy.engine.data.card_database import CardDatabase
        magnaangemon = CardDatabase().create_card_source(MAGNAANGEMON_ARCHANGEL, game.player1)
        ctx = self._build_ctx(game, tamer, magnaangemon, game.player1)

        assert effect.can_use_condition(ctx), (
            "Archangel trait should satisfy the trigger condition"
        )

    def test_fallen_angel_trait_passes_condition(self, debug_runner):
        """A played Digimon with Fallen Angel trait triggers the condition."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        tamer = runner.place_on_field(1, [EX6_074])
        effect = _find_onplay_effect(tamer.top_card)

        from engine_py_legacy.engine.data.card_database import CardDatabase
        devimon = CardDatabase().create_card_source(DEVIMON_FALLEN_ANGEL, game.player1)
        ctx = self._build_ctx(game, tamer, devimon, game.player1)

        assert effect.can_use_condition(ctx), (
            "Fallen Angel trait should satisfy the trigger condition"
        )

    def test_non_matching_trait_fails_condition(self, debug_runner):
        """A played Digimon without any of the traits should not trigger."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        tamer = runner.place_on_field(1, [EX6_074])
        effect = _find_onplay_effect(tamer.top_card)

        from engine_py_legacy.engine.data.card_database import CardDatabase
        agumon = CardDatabase().create_card_source(AGUMON_GENERIC, game.player1)
        ctx = self._build_ctx(game, tamer, agumon, game.player1)

        assert not effect.can_use_condition(ctx), (
            "Agumon (no Holy Beast/Archangel/Fallen Angel trait) should NOT trigger"
        )

    def test_opponent_played_digimon_fails_condition(self, debug_runner):
        """If the played Digimon belongs to the opponent, condition fails."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        tamer = runner.place_on_field(1, [EX6_074])
        effect = _find_onplay_effect(tamer.top_card)

        from engine_py_legacy.engine.data.card_database import CardDatabase
        gatomon = CardDatabase().create_card_source(GATOMON_HOLY_BEAST, game.player2)
        # event_player is the opponent, not the tamer owner
        ctx = self._build_ctx(game, tamer, gatomon, game.player2)

        assert not effect.can_use_condition(ctx), (
            "Opponent playing a Holy Beast Digimon should NOT trigger Mirei"
        )

    def test_opponent_turn_fails_condition(self, debug_runner):
        """[Your Turn] --> cannot trigger on opponent's turn."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        tamer = runner.place_on_field(1, [EX6_074])
        effect = _find_onplay_effect(tamer.top_card)

        # Switch to opponent's turn
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        from engine_py_legacy.engine.data.card_database import CardDatabase
        gatomon = CardDatabase().create_card_source(GATOMON_HOLY_BEAST, game.player1)
        ctx = self._build_ctx(game, tamer, gatomon, game.player1)

        assert not effect.can_use_condition(ctx), (
            "[Your Turn] should prevent activation on opponent's turn"
        )

    def test_tamer_not_on_field_fails_condition(self, debug_runner):
        """If Mirei is not on the field, condition fails."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        # Place Mirei in hand (not on field)
        runner.inject_card(1, EX6_074, "hand")
        tamer_card = game.player1.hand_cards[-1]
        effect = _find_onplay_effect(tamer_card)

        from engine_py_legacy.engine.data.card_database import CardDatabase
        gatomon = CardDatabase().create_card_source(GATOMON_HOLY_BEAST, game.player1)
        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': None,
            'card': tamer_card,
            'turn_player': game.turn_player,
            'opponent_player': game.opponent_player,
            'event_player': game.player1,
            'played_card': gatomon,
        }
        assert not effect.can_use_condition(ctx), (
            "Effect should NOT trigger when Mirei is not on the field"
        )

    def test_tamer_suspended_fails_condition(self, debug_runner):
        """If Mirei is already suspended, condition fails (cost is suspend)."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        tamer = runner.place_on_field(1, [EX6_074], is_suspended=True)
        effect = _find_onplay_effect(tamer.top_card)

        from engine_py_legacy.engine.data.card_database import CardDatabase
        gatomon = CardDatabase().create_card_source(GATOMON_HOLY_BEAST, game.player1)
        ctx = self._build_ctx(game, tamer, gatomon, game.player1)

        assert not effect.can_use_condition(ctx), (
            "Suspended Mirei should NOT be able to activate (suspend is the cost)"
        )


@pytest.mark.behavioral
class TestEX6074Clause1Process:
    """Process callback for Clause 1 — suspend, memory, digivolve-from-trash."""

    def test_process_suspends_tamer_and_gains_memory(self, debug_runner):
        """Process must: (1) suspend this Tamer, (2) gain 1 memory."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        tamer = runner.place_on_field(1, [EX6_074])
        assert not tamer.is_suspended

        # Place a Yellow Lv.4 Digimon as a valid digivolve target
        runner.place_on_field(1, [GATOMON_HOLY_BEAST])

        # Put Angewomon in trash
        runner.inject_card(1, ANGEWOMON, "trash")

        effect = _find_onplay_effect(tamer.top_card)
        memory_before = game.memory

        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': tamer,
            'card': tamer.top_card,
            'turn_player': game.turn_player,
            'opponent_player': game.opponent_player,
            'event_player': game.player1,
            'played_card': tamer.top_card,  # Not used by process
        }
        effect.on_process_callback(ctx)

        assert tamer.is_suspended, (
            "Tamer should be suspended after process runs (cost payment)"
        )
        assert game.memory == memory_before + 1, (
            f"Memory should increase by 1: was {memory_before}, now {game.memory}"
        )

    def test_process_creates_target_selection(self, debug_runner):
        """Process should request selection of which own Digimon to digivolve
        (player agency, NOT auto-select)."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        tamer = runner.place_on_field(1, [EX6_074])

        # Place TWO valid digivolve targets so a real selection is needed
        runner.place_on_field(1, [GATOMON_HOLY_BEAST])
        runner.place_on_field(1, [GATOMON_HOLY_BEAST])

        # Put Angewomon in trash so the chain isn't cut short by no-target
        runner.inject_card(1, ANGEWOMON, "trash")

        effect = _find_onplay_effect(tamer.top_card)
        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': tamer,
            'card': tamer.top_card,
            'turn_player': game.turn_player,
            'opponent_player': game.opponent_player,
            'event_player': game.player1,
            'played_card': tamer.top_card,
        }
        effect.on_process_callback(ctx)

        # Should have a pending selection (target Digimon) — NOT auto-pick
        assert game.pending_selection is not None, (
            "Process should create a pending selection for the digivolve target "
            "(player picks which Digimon). Current script auto-selects, which is a bug."
        )

    def test_process_uses_include_trash(self, debug_runner):
        """Process should call effect_digivolve_from_hand with include_trash=True.

        Verified by: placing Angewomon in TRASH (not hand) and checking that
        the game surfaces a valid digivolve selection.
        """
        runner = debug_runner(initial_memory=5)
        game = runner.game
        tamer = runner.place_on_field(1, [EX6_074])
        target = runner.place_on_field(1, [GATOMON_HOLY_BEAST])

        # Angewomon ONLY in trash, not in hand
        runner.inject_card(1, ANGEWOMON, "trash")

        # Monkey-patch effect_digivolve_from_hand to capture the call
        captured = {}
        original = game.effect_digivolve_from_hand
        def spy(*args, **kwargs):
            captured['args'] = args
            captured['kwargs'] = kwargs
            return original(*args, **kwargs)
        game.effect_digivolve_from_hand = spy

        effect = _find_onplay_effect(tamer.top_card)
        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': tamer,
            'card': tamer.top_card,
            'turn_player': game.turn_player,
            'opponent_player': game.opponent_player,
            'event_player': game.player1,
            'played_card': tamer.top_card,
        }
        effect.on_process_callback(ctx)

        # A pending selection should exist (target Digimon). Pick the target
        # explicitly so the inner callback fires (auto_resolve would pick
        # "decline" at action id 62, skipping the digivolve chain).
        assert game.pending_selection is not None, (
            "Expected a target-selection after process"
        )
        from engine_py_legacy.engine.game.constants import SEL_MY_FIELD_START
        # Pick the first available own-field target (the Gatomon)
        target_action = SEL_MY_FIELD_START + 1  # idx 1 = Gatomon (0 is tamer)
        game.decode_action(target_action, 0)

        assert 'kwargs' in captured, (
            "effect_digivolve_from_hand should have been called by the process"
        )
        kwargs = captured['kwargs']
        assert kwargs.get('include_trash') is True, (
            f"Should call effect_digivolve_from_hand with include_trash=True; "
            f"got kwargs={kwargs}"
        )
        assert kwargs.get('cost_reduction', 0) == 1, (
            f"Should pass cost_reduction=1; got {kwargs.get('cost_reduction')}"
        )
        assert kwargs.get('is_optional') is True, (
            "Digivolve sub-effect should be optional (card says 'may')"
        )

    def test_filter_matches_angewomon(self, debug_runner):
        """The filter passed to effect_digivolve_from_hand should match
        Angewomon cards by name."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        tamer = runner.place_on_field(1, [EX6_074])
        runner.place_on_field(1, [GATOMON_HOLY_BEAST])
        runner.inject_card(1, ANGEWOMON, "trash")

        captured = {}
        original = game.effect_digivolve_from_hand
        def spy(player, permanent, filter_fn, *args, **kwargs):
            captured['filter_fn'] = filter_fn
            return original(player, permanent, filter_fn, *args, **kwargs)
        game.effect_digivolve_from_hand = spy

        effect = _find_onplay_effect(tamer.top_card)
        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': tamer,
            'card': tamer.top_card,
            'turn_player': game.turn_player,
            'opponent_player': game.opponent_player,
            'event_player': game.player1,
            'played_card': tamer.top_card,
        }
        effect.on_process_callback(ctx)

        # Pick a target to actually invoke the inner effect_digivolve_from_hand
        from engine_py_legacy.engine.game.constants import SEL_MY_FIELD_START
        game.decode_action(SEL_MY_FIELD_START + 1, 0)  # Gatomon at idx 1

        filter_fn = captured.get('filter_fn')
        assert filter_fn is not None, "Filter function should have been captured"

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        angewomon = db.create_card_source(ANGEWOMON, game.player1)
        ladydevimon = db.create_card_source(LADYDEVIMON, game.player1)
        agumon = db.create_card_source(AGUMON_GENERIC, game.player1)

        assert filter_fn(angewomon), "Filter should match Angewomon"
        assert filter_fn(ladydevimon), "Filter should match LadyDevimon"
        assert not filter_fn(agumon), "Filter should NOT match Agumon"


# ======================================================================
# Clause 2: [End of Your Turn] [Once Per Turn] DNA digivolve
# ======================================================================


@pytest.mark.behavioral
class TestEX6074Clause2DNADigivolve:
    """Tests for Clause 2 — end-of-turn DNA digivolve (OPT)."""

    def test_eot_effect_exists(self, debug_runner):
        """Should have an OnEndTurn effect."""
        runner = debug_runner(initial_memory=5)
        tamer = runner.place_on_field(1, [EX6_074])
        effect = _find_eot_effect(tamer.top_card)
        assert effect is not None, (
            "EX6-074 should have an OnEndTurn effect (Clause 2)"
        )

    def test_eot_effect_is_optional(self, debug_runner):
        """'may DNA digivolve' --> optional."""
        runner = debug_runner(initial_memory=5)
        tamer = runner.place_on_field(1, [EX6_074])
        effect = _find_eot_effect(tamer.top_card)
        assert effect is not None
        assert effect.is_optional, "EoT DNA digivolve should be optional"

    def test_eot_effect_is_opt(self, debug_runner):
        """[Once Per Turn] --> set_max_count_per_turn(1) + hash_string set."""
        runner = debug_runner(initial_memory=5)
        tamer = runner.place_on_field(1, [EX6_074])
        effect = _find_eot_effect(tamer.top_card)
        assert effect is not None
        assert effect.max_count_per_turn == 1, (
            f"OPT should set max_count_per_turn=1; got {effect.max_count_per_turn}"
        )
        hs = getattr(effect, 'hash_string', None) or getattr(effect, '_hash_string', None)
        assert hs, (
            "Effect should have a hash_string set for OPT persistence across copies"
        )

    def test_eot_condition_passes_with_tamer_on_field_on_my_turn(self, debug_runner):
        """Condition should pass when Mirei is on field and it's my turn."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        tamer = runner.place_on_field(1, [EX6_074])
        effect = _find_eot_effect(tamer.top_card)

        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': tamer,
            'card': tamer.top_card,
            'turn_player': game.turn_player,
            'opponent_player': game.opponent_player,
        }
        assert effect.can_use_condition(ctx), (
            "EoT condition should pass when Mirei is on field on own turn"
        )

    def test_eot_condition_fails_when_not_on_field(self, debug_runner):
        """If Mirei is not on field, EoT condition should fail."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        runner.inject_card(1, EX6_074, "hand")
        tamer_card = game.player1.hand_cards[-1]
        effect = _find_eot_effect(tamer_card)

        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': None,
            'card': tamer_card,
            'turn_player': game.turn_player,
            'opponent_player': game.opponent_player,
        }
        assert not effect.can_use_condition(ctx), (
            "EoT condition should fail when Mirei is not on field"
        )

    def test_eot_process_calls_dna_digivolve(self, debug_runner):
        """Process should call effect_dna_digivolve_from_hand."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        tamer = runner.place_on_field(1, [EX6_074])

        captured = {}
        original = game.effect_dna_digivolve_from_hand
        def spy(player, filter_fn, *args, **kwargs):
            captured['player'] = player
            captured['filter_fn'] = filter_fn
            captured['kwargs'] = kwargs
            return original(player, filter_fn, *args, **kwargs)
        game.effect_dna_digivolve_from_hand = spy

        effect = _find_eot_effect(tamer.top_card)
        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': tamer,
            'card': tamer.top_card,
            'turn_player': game.turn_player,
            'opponent_player': game.opponent_player,
        }
        effect.on_process_callback(ctx)

        assert 'filter_fn' in captured, (
            "Process should call effect_dna_digivolve_from_hand"
        )
        assert captured['player'] is game.player1
        assert captured['kwargs'].get('is_optional') is True, (
            "DNA digivolve sub-effect should be optional"
        )


# ======================================================================
# Clause 3: [Security] Play this card without paying the cost
# ======================================================================


@pytest.mark.behavioral
class TestEX6074Clause3Security:
    """Tests for Clause 3 — security play effect."""

    def test_security_effect_exists(self, debug_runner):
        """Should have a security effect."""
        runner = debug_runner(initial_memory=5)
        runner.inject_card(1, EX6_074, "hand")
        tamer_card = runner.game.player1.hand_cards[-1]
        effect = _find_security_effect(tamer_card)
        assert effect is not None, "EX6-074 should have a security effect"

    def test_security_effect_plays_card(self, debug_runner):
        """Security effect should play the card onto the field."""
        runner = debug_runner(initial_memory=5)
        game = runner.game
        runner.inject_card(1, EX6_074, "hand")
        tamer_card = game.player1.hand_cards[-1]

        effect = _find_security_effect(tamer_card)
        field_before = len(game.player1.battle_area)

        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': None,
            'card': tamer_card,
        })

        assert len(game.player1.battle_area) == field_before + 1, (
            "Security effect should play Mirei onto the field"
        )

    def test_security_effect_is_free(self, debug_runner):
        """Security play should not cost memory."""
        runner = debug_runner(initial_memory=0)
        game = runner.game
        runner.inject_card(1, EX6_074, "hand")
        tamer_card = game.player1.hand_cards[-1]
        effect = _find_security_effect(tamer_card)

        memory_before = game.memory
        effect.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': None,
            'card': tamer_card,
        })
        assert game.memory == memory_before, (
            f"Security play should not change memory: was {memory_before}, "
            f"now {game.memory}"
        )
