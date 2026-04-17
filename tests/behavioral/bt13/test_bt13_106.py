"""Behavioral tests for BT13-106 Odin's Breath (Option, Yellow, cost 5).

Card text:
  When an effect trashes this card from the security stack, activate this
  card's [Main] effect.
  [Main] 1 of your opponent's Digimon gets -3000 DP until the end of your
  opponent's turn. Then, if there're 6 or fewer total cards in both players'
  security stacks, all of your opponent's Digimon gain <Security A. -1>
  (This Digimon checks 1 fewer security card.) until the end of your
  opponent's turn.

  [Security] Activate this card's [Main] effects.

Faithfulness clauses tested:
  C1. OnDiscardSecurity trigger — fires when an effect trashes THIS card from
      its owner's security stack, and activates the [Main] effect.
  C2a. [Main] DP -3000 to a player-selected opponent Digimon.
  C2b. Conditional SA -1 to ALL opponent Digimon when total security <= 6.
  C2c. The DP modifier is target-specific — does not leak to other permanents.
  C2d. DP modifier expiry tied to turn rotation (engine end_of_opponent_turn).
  C3. [Security] effect activates the same [Main] body during a security check.
  C4. Trigger guard — OnDiscardSecurity does NOT fire when a DIFFERENT card
      is trashed from security.
"""

import pytest

from digimon_gym.engine.data.enums import EffectTiming, GamePhase
from digimon_gym.engine.game.constants import SEL_OPP_FIELD_START


ODINS_BREATH = "BT13-106"
DRACOMON = "ST1-04"       # Lv.3 Red Digimon, 4000 DP (simple target)
BIRDRAMON = "ST1-05"      # Lv.4 Red Digimon, 5000 DP
AGUMON = "ST1-03"         # Lv.3 Red Digimon, 2000 DP (filler)
GARUDAMON = "ST1-08"      # Lv.5 Red Digimon, 7000 DP (big attacker)
PATAMON = "BT14-033"      # Lv.3 Yellow Digimon — used to satisfy Option colour req


# ─── Helpers ────────────────────────────────────────────────────────────────


def _get_odins_breath_effects():
    """Instantiate a BT13-106 CardSource and return its effects list."""
    from digimon_gym.engine.data.card_database import CardDatabase
    db = CardDatabase()
    cs = db.create_card_source(ODINS_BREATH)
    assert cs is not None, "Failed to create BT13-106 card source"
    return cs, cs.effect_list(None)


def _find_opp_perm_action(runner, card_id: str):
    """Find the SEL_OPP_FIELD action for a specific opponent permanent by card id."""
    game = runner.game
    current_player = game.player1 if game.current_player_id == 1 else game.player2
    opp = game.player2 if current_player is game.player1 else game.player1
    for i, perm in enumerate(opp.battle_area):
        if perm.top_card and perm.top_card.c_entity_base:
            if perm.top_card.c_entity_base.card_id == card_id:
                return SEL_OPP_FIELD_START + i
    return None


def _perm_by_id(player, card_id: str):
    for p in player.battle_area:
        if p.top_card and p.top_card.c_entity_base:
            if p.top_card.c_entity_base.card_id == card_id:
                return p
    return None


def _clear_security(player):
    """Drop all security cards without firing OnLoseSecurity."""
    player.security_cards.clear()
    player.face_up_security.clear()


# ─── Static shape tests ─────────────────────────────────────────────────────


@pytest.mark.behavioral
class TestBT13106EffectShape:
    """Verify the card exposes the expected effect timings."""

    def test_has_on_discard_security_trigger(self):
        cs, effects = _get_odins_breath_effects()
        on_discard = [
            e for e in effects
            if e.timing == EffectTiming.OnDiscardSecurity
        ]
        assert len(on_discard) >= 1, (
            "BT13-106 must expose an OnDiscardSecurity trigger to activate its "
            "[Main] when trashed from security."
        )
        # Must have a process callback — condition-only effect would never fire
        assert on_discard[0].on_process_callback is not None, (
            "OnDiscardSecurity effect must have an on_process_callback that "
            "runs the [Main] body (activate Main effect)."
        )

    def test_has_option_skill_main_effect(self):
        cs, effects = _get_odins_breath_effects()
        main_effects = [
            e for e in effects
            if e.timing == EffectTiming.OptionSkill
        ]
        assert len(main_effects) >= 1, "BT13-106 must expose an OptionSkill effect"
        assert main_effects[0].on_process_callback is not None, (
            "OptionSkill effect must have a process callback for the [Main] body."
        )

    def test_has_security_skill_effect_with_process(self):
        """[Security] must be a SecuritySkill-timed effect with an on_process
        callback that runs the [Main] body — NOT just the default cosmetic
        factory ``is_security_effect=True`` flag with no process."""
        cs, effects = _get_odins_breath_effects()
        sec_effects = [
            e for e in effects
            if e.timing == EffectTiming.SecuritySkill
            and getattr(e, 'is_security_effect', False)
        ]
        assert len(sec_effects) >= 1, (
            "BT13-106 must expose a SecuritySkill-timed effect with "
            "is_security_effect=True for its [Security] clause."
        )
        assert sec_effects[0].on_process_callback is not None, (
            "[Security] effect must have an on_process_callback — card text "
            "says 'Activate this card's [Main] effects'."
        )


# ─── C2a: [Main] DP -3000 selection ─────────────────────────────────────────


@pytest.mark.behavioral
class TestBT13106MainDPSelection:
    """[Main] 1 of your opponent's Digimon gets -3000 DP."""

    def test_play_from_hand_enters_dp_selection(self, debug_runner):
        """Playing Odin's Breath from hand with opponent Digimon on field
        must enter a target-selection phase (NOT auto-select)."""
        runner = debug_runner(archetype1="Yellow Vax", initial_memory=10)
        runner.set_phase("Main")
        # Satisfy Option yellow-color requirement (need Yellow Digimon on field)
        runner.place_on_field(1, [PATAMON])
        # Give the opposing player two Digimon to choose between
        runner.place_on_field(2, [DRACOMON])
        runner.place_on_field(2, [BIRDRAMON])
        runner.inject_card(1, ODINS_BREATH, "hand")

        action = runner.find_action("Odin")
        assert action is not None, (
            f"Should be able to play Odin's Breath. Actions: {runner.actions()}"
        )
        runner.execute(action)

        # After playing the option, the game should be in a selection phase
        # for the DP target — NOT auto-resolved to Main.
        assert runner.game.current_phase != GamePhase.Main, (
            "After playing Odin's Breath with 2 opponent Digimon on field, "
            "player must be prompted to choose which Digimon gets -3000 DP "
            "(no auto-selection)."
        )
        # Both opponent Digimon should be selectable
        dracomon_act = _find_opp_perm_action(runner, DRACOMON)
        birdramon_act = _find_opp_perm_action(runner, BIRDRAMON)
        assert dracomon_act is not None, "Dracomon should be a legal DP target"
        assert birdramon_act is not None, "Birdramon should be a legal DP target"

    def test_dp_minus_3000_applies_to_chosen_target(self, debug_runner):
        """The selected target's DP decreases by exactly 3000."""
        runner = debug_runner(archetype1="Yellow Vax", initial_memory=10)
        runner.set_phase("Main")
        runner.place_on_field(1, [PATAMON])    # colour requirement
        runner.place_on_field(2, [BIRDRAMON])  # 5000 DP
        runner.inject_card(1, ODINS_BREATH, "hand")

        action = runner.find_action("Odin")
        assert action is not None
        runner.execute(action)

        bird_before = _perm_by_id(runner.game.player2, BIRDRAMON)
        assert bird_before is not None
        base_dp = bird_before.dp
        assert base_dp == 5000, f"Birdramon should start at 5000 DP, got {base_dp}"

        bird_action = _find_opp_perm_action(runner, BIRDRAMON)
        assert bird_action is not None
        runner.execute(bird_action)
        runner.auto_resolve()

        bird_after = _perm_by_id(runner.game.player2, BIRDRAMON)
        assert bird_after is not None, "Birdramon should still be on field"
        assert bird_after.dp == base_dp - 3000, (
            f"Birdramon DP should drop by 3000 "
            f"(5000 → 2000), got {bird_after.dp}"
        )

    def test_dp_minus_3000_does_not_leak_to_other_permanent(self, debug_runner):
        """The -3000 DP is target-specific — must not leak to other opp perms.

        This is the 'target-equality guard' bug: if the modifier is registered
        without a condition guard, get_int_modifier() applies it to every
        permanent queried.
        """
        runner = debug_runner(archetype1="Yellow Vax", initial_memory=10)
        runner.set_phase("Main")
        runner.place_on_field(1, [PATAMON])    # colour requirement
        runner.place_on_field(2, [BIRDRAMON])  # 5000 DP, will be targeted
        runner.place_on_field(2, [DRACOMON])   # 4000 DP, must NOT be affected
        runner.inject_card(1, ODINS_BREATH, "hand")

        runner.execute(runner.find_action("Odin"))

        # Snapshot baseline DPs before resolving selection
        bird = _perm_by_id(runner.game.player2, BIRDRAMON)
        draco = _perm_by_id(runner.game.player2, DRACOMON)
        assert bird is not None and draco is not None
        bird_base = bird.dp
        draco_base = draco.dp

        # Target Birdramon
        runner.execute(_find_opp_perm_action(runner, BIRDRAMON))
        runner.auto_resolve()

        bird_after = _perm_by_id(runner.game.player2, BIRDRAMON)
        draco_after = _perm_by_id(runner.game.player2, DRACOMON)
        assert bird_after is not None and draco_after is not None
        assert bird_after.dp == bird_base - 3000, (
            f"Birdramon (targeted) should lose 3000 DP: "
            f"expected {bird_base - 3000}, got {bird_after.dp}"
        )
        assert draco_after.dp == draco_base, (
            f"Dracomon (NOT targeted) DP must not change: "
            f"expected {draco_base}, got {draco_after.dp} "
            f"— -3000 DP leaked from Birdramon to Dracomon"
        )

    def test_dp_minus_3000_does_not_leak_to_own_permanent(self, debug_runner):
        """The -3000 DP modifier must NOT leak to the caster's own Digimon
        either (another target-equality leak vector)."""
        runner = debug_runner(archetype1="Yellow Vax", initial_memory=10)
        runner.set_phase("Main")
        runner.place_on_field(1, [PATAMON])    # colour requirement
        runner.place_on_field(2, [BIRDRAMON])
        runner.place_on_field(1, [DRACOMON])   # own-side witness (red, just DP test)
        runner.inject_card(1, ODINS_BREATH, "hand")

        runner.execute(runner.find_action("Odin"))

        own_draco = _perm_by_id(runner.game.player1, DRACOMON)
        assert own_draco is not None
        own_base = own_draco.dp

        runner.execute(_find_opp_perm_action(runner, BIRDRAMON))
        runner.auto_resolve()

        own_draco_after = _perm_by_id(runner.game.player1, DRACOMON)
        assert own_draco_after is not None
        assert own_draco_after.dp == own_base, (
            f"Own Dracomon DP must not change (expected {own_base}, got "
            f"{own_draco_after.dp}) — -3000 DP leaked across battle areas"
        )


# ─── C2b: Conditional Security A. -1 on all opponent Digimon ────────────────


@pytest.mark.behavioral
class TestBT13106ConditionalSecurityAttackMinusOne:
    """Then, if total security <= 6, all opponent Digimon gain Security A. -1."""

    def test_security_attack_minus_1_applies_when_total_6_or_fewer(self, debug_runner):
        """When total security across both players is <= 6, every opponent
        Digimon receives Security A. -1 until the end of opponent's turn."""
        runner = debug_runner(archetype1="Yellow Vax", initial_memory=10)
        runner.set_phase("Main")

        # Reduce security to 3 each (total = 6).
        # Use inject_card with simple Digimon (AGUMON = ST1-03) so that
        # OnEnterFieldAnyone triggers from other zones are inert.
        _clear_security(runner.game.player1)
        _clear_security(runner.game.player2)
        for _ in range(3):
            runner.inject_card(1, AGUMON, "security_top")
            runner.inject_card(2, AGUMON, "security_top")

        assert len(runner.game.player1.security_cards) + \
               len(runner.game.player2.security_cards) == 6

        runner.place_on_field(1, [PATAMON])    # colour requirement
        runner.place_on_field(2, [BIRDRAMON])  # will be the -3000 DP target
        runner.place_on_field(2, [DRACOMON])   # witness for SA -1
        runner.inject_card(1, ODINS_BREATH, "hand")

        runner.execute(runner.find_action("Odin"))
        runner.execute(_find_opp_perm_action(runner, BIRDRAMON))
        runner.auto_resolve()

        # Both opponent Digimon should have SA -1
        bird = _perm_by_id(runner.game.player2, BIRDRAMON)
        draco = _perm_by_id(runner.game.player2, DRACOMON)
        assert bird is not None and draco is not None
        assert bird.security_attack_modifier() == -1, (
            f"Birdramon SA modifier should be -1, got "
            f"{bird.security_attack_modifier()}"
        )
        assert draco.security_attack_modifier() == -1, (
            f"Dracomon SA modifier should be -1, got "
            f"{draco.security_attack_modifier()}"
        )

    def test_security_attack_unchanged_when_total_above_6(self, debug_runner):
        """When total security > 6, the conditional SA -1 clause does NOT fire."""
        runner = debug_runner(archetype1="Yellow Vax", initial_memory=10)
        runner.set_phase("Main")

        # Force total security = 7 (4 + 3) using inert Agumons.
        _clear_security(runner.game.player1)
        _clear_security(runner.game.player2)
        for _ in range(4):
            runner.inject_card(1, AGUMON, "security_top")
        for _ in range(3):
            runner.inject_card(2, AGUMON, "security_top")
        assert len(runner.game.player1.security_cards) + \
               len(runner.game.player2.security_cards) == 7

        runner.place_on_field(1, [PATAMON])    # colour requirement
        runner.place_on_field(2, [BIRDRAMON])
        runner.place_on_field(2, [DRACOMON])
        runner.inject_card(1, ODINS_BREATH, "hand")

        runner.execute(runner.find_action("Odin"))
        runner.execute(_find_opp_perm_action(runner, BIRDRAMON))
        runner.auto_resolve()

        bird = _perm_by_id(runner.game.player2, BIRDRAMON)
        draco = _perm_by_id(runner.game.player2, DRACOMON)
        assert bird is not None and draco is not None
        assert bird.security_attack_modifier() == 0, (
            f"No SA modifier should apply when total security > 6, "
            f"Birdramon SA mod = {bird.security_attack_modifier()}"
        )
        assert draco.security_attack_modifier() == 0, (
            f"No SA modifier should apply when total security > 6, "
            f"Dracomon SA mod = {draco.security_attack_modifier()}"
        )

    def test_sa_minus_1_does_not_leak_to_own_digimon(self, debug_runner):
        """SA -1 must apply only to opponent Digimon — target-equality guard."""
        runner = debug_runner(archetype1="Yellow Vax", initial_memory=10)
        runner.set_phase("Main")

        # Force total security = 6 using inert Agumons
        _clear_security(runner.game.player1)
        _clear_security(runner.game.player2)
        for _ in range(3):
            runner.inject_card(1, AGUMON, "security_top")
            runner.inject_card(2, AGUMON, "security_top")

        runner.place_on_field(1, [PATAMON])    # colour requirement
        runner.place_on_field(2, [BIRDRAMON])  # DP target + SA -1 recipient
        runner.place_on_field(1, [DRACOMON])   # own-side witness for SA -1 leak
        runner.inject_card(1, ODINS_BREATH, "hand")

        runner.execute(runner.find_action("Odin"))
        runner.execute(_find_opp_perm_action(runner, BIRDRAMON))
        runner.auto_resolve()

        own_draco = _perm_by_id(runner.game.player1, DRACOMON)
        assert own_draco is not None
        assert own_draco.security_attack_modifier() == 0, (
            f"Own Dracomon must not receive SA -1: got "
            f"{own_draco.security_attack_modifier()} — SA -1 leaked to own side"
        )


# ─── C2d: DP modifier expiry ────────────────────────────────────────────────


@pytest.mark.behavioral
class TestBT13106DPExpiry:
    """-3000 DP expires (engine convention: end_of_opponent_turn)."""

    def test_dp_modifier_uses_end_of_opponent_turn_expiry(self):
        """The DP modifier registered by the Main effect must have expiry
        'end_of_opponent_turn' (NOT 'permanent' or 'end_of_turn')."""
        from digimon_gym.engine.data.card_database import CardDatabase
        from digimon_gym.engine.game import Game
        from digimon_gym.engine.loggers import EventLogger
        from digimon_gym.engine.interfaces.modifiers import ModifierType

        db = CardDatabase()
        logger = EventLogger()
        game = Game(logger)
        # Minimal setup — place a target digimon and invoke the main process
        # via the OptionSkill effect's callback directly.
        game.player1.library_cards = [db.create_card_source("ST1-03", game.player1)
                                      for _ in range(10)]
        game.player2.library_cards = [db.create_card_source("ST1-03", game.player2)
                                      for _ in range(10)]
        # Give p2 a target digimon on field
        target_cs = db.create_card_source(BIRDRAMON, game.player2)
        from digimon_gym.engine.core.permanent import Permanent
        target_perm = Permanent([target_cs])
        target_perm._owner_game = game
        game.player2.battle_area.append(target_perm)
        game.turn_player = game.player1
        game.opponent_player = game.player2

        odin = db.create_card_source(ODINS_BREATH, game.player1)
        effects = odin.effect_list(None)
        main_effect = next(
            e for e in effects if e.timing == EffectTiming.OptionSkill
        )
        # Directly invoke process with a pre-selected target (bypass selection)
        # by monkey-patching effect_select_opponent_permanent to call the
        # callback immediately with the target.
        original_fn = game.effect_select_opponent_permanent
        def immediate_select(player, callback, filter_fn=None, is_optional=False,
                             prompt=""):
            callback(target_perm)
        game.effect_select_opponent_permanent = immediate_select
        try:
            ctx = {
                "game": game,
                "player": game.player1,
                "permanent": None,
                "card": odin,
                "turn_player": game.turn_player,
                "opponent_player": game.opponent_player,
            }
            main_effect.on_process_callback(ctx)
        finally:
            game.effect_select_opponent_permanent = original_fn

        # Find the registered CHANGE_DP modifier targeting target_perm
        dp_entries = game.modifiers._modifiers.get(ModifierType.CHANGE_DP, [])
        matching = [
            e for e in dp_entries
            if e.is_active(target_perm, {})
        ]
        assert len(matching) >= 1, "A CHANGE_DP modifier should be registered"
        assert any(e.expiry == 'end_of_opponent_turn' for e in matching), (
            f"DP modifier must have expiry 'end_of_opponent_turn', got expiries: "
            f"{[e.expiry for e in matching]}"
        )

    def test_dp_modifier_cleared_on_caster_next_turn(self, debug_runner):
        """The -3000 DP clears at the start of the caster's next turn (engine
        convention for end_of_opponent_turn)."""
        runner = debug_runner(archetype1="Yellow Vax", initial_memory=10)
        runner.set_phase("Main")
        runner.place_on_field(1, [PATAMON])    # colour requirement
        runner.place_on_field(2, [BIRDRAMON])
        runner.inject_card(1, ODINS_BREATH, "hand")

        runner.execute(runner.find_action("Odin"))
        runner.execute(_find_opp_perm_action(runner, BIRDRAMON))
        runner.auto_resolve()

        bird_after_play = _perm_by_id(runner.game.player2, BIRDRAMON)
        assert bird_after_play is not None
        assert bird_after_play.dp == 2000, (
            f"Birdramon should be at 2000 DP after Odin's Breath, "
            f"got {bird_after_play.dp}"
        )

        # Simulate passing turn to opponent then back to us, which should
        # clear end_of_opponent_turn modifiers granted by us.
        game = runner.game
        # Advance: p1 → p2 → p1
        game.turn_player = game.player2
        game.opponent_player = game.player1
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True
        game.phase_start()
        # Now back to p1 — this should trigger the cleanup
        game.turn_player = game.player1
        game.opponent_player = game.player2
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False
        game.phase_start()

        bird_after_rotate = _perm_by_id(runner.game.player2, BIRDRAMON)
        assert bird_after_rotate is not None
        assert bird_after_rotate.dp == 5000, (
            f"Birdramon DP should return to 5000 after caster's next turn "
            f"begins (end_of_opponent_turn expiry), got {bird_after_rotate.dp}"
        )


# ─── C1: OnDiscardSecurity trigger ──────────────────────────────────────────


@pytest.mark.behavioral
class TestBT13106OnDiscardSecurityTrigger:
    """When an effect trashes THIS card from security, activate [Main]."""

    def test_main_activates_when_trashed_from_security(self, debug_runner):
        """Trashing Odin's Breath from security via an effect activates Main."""
        runner = debug_runner(archetype1="Yellow Vax", initial_memory=10)
        runner.set_phase("Main")

        # Put Odin's Breath on top of P1's security (the defender)
        runner.inject_card(1, ODINS_BREATH, "security_top")
        # Give P2 (the attacker — opponent of card owner) a Digimon to target
        runner.place_on_field(2, [BIRDRAMON])

        # Simulate 'an effect' trashing the security card
        sec_card = runner.game.player1.security_cards[0]
        assert sec_card.c_entity_base.card_id == ODINS_BREATH
        runner.game.player1.trash_security_card(sec_card)

        # After the trigger, we should be in a selection phase for the DP target.
        # The selecting player is the card owner (P1). P1 chooses an opponent
        # Digimon (from P2's field).
        if runner.game.current_phase != GamePhase.Main:
            bird_action = _find_opp_perm_action(runner, BIRDRAMON)
            if bird_action is not None and bird_action in runner.action_mask():
                runner.execute(bird_action)
            runner.auto_resolve()

        bird = _perm_by_id(runner.game.player2, BIRDRAMON)
        assert bird is not None, "Birdramon should still be on field"
        # Either the effect ran (DP = 2000) or it silently no-op'd (DP = 5000).
        # We require the effect to have fired.
        assert bird.dp == 2000, (
            f"Birdramon should have -3000 DP after Odin's Breath was trashed "
            f"from security (trigger should activate [Main]), got DP={bird.dp}"
        )

    def test_trigger_does_not_fire_for_different_card(self, debug_runner):
        """Trashing a DIFFERENT card from security must NOT activate
        Odin's Breath's [Main] effect."""
        runner = debug_runner(archetype1="Yellow Vax", initial_memory=10)
        runner.set_phase("Main")

        # Put Odin's Breath in P1's trash (so its OnDiscardSecurity is in the
        # trash zone for scanning), but trash a DIFFERENT card from security.
        runner.inject_card(1, ODINS_BREATH, "trash")
        # Put a different card on top of security
        runner.inject_card(1, AGUMON, "security_top")

        # Target on opponent field
        runner.place_on_field(2, [BIRDRAMON])

        # Trash the non-Odin card from security — should NOT trigger Odin's Main
        decoy = runner.game.player1.security_cards[0]
        assert decoy.c_entity_base.card_id == AGUMON
        runner.game.player1.trash_security_card(decoy)
        runner.auto_resolve()

        bird = _perm_by_id(runner.game.player2, BIRDRAMON)
        assert bird is not None
        assert bird.dp == 5000, (
            f"Birdramon should still be at 5000 DP — Odin's Breath trigger "
            f"must only fire when THIS card is trashed, not when a different "
            f"card is trashed. Got DP={bird.dp}"
        )


# ─── C3: [Security] effect activates [Main] during a security check ────────


@pytest.mark.behavioral
class TestBT13106SecurityEffect:
    """[Security] Activate this card's [Main] effects."""

    def test_security_check_activates_main_effect(self, debug_runner):
        """When Odin's Breath is revealed during a security check, its [Main]
        body runs against the attacker (for defender's benefit)."""
        runner = debug_runner(archetype1="Yellow Vax", initial_memory=10)
        runner.set_phase("Main")

        # P1 is the attacker. P2 defends and has Odin's Breath on top of security.
        _clear_security(runner.game.player2)
        runner.inject_card(2, ODINS_BREATH, "security_top")
        # The defender's Main effect targets attacker's (P1's) Digimon
        runner.place_on_field(1, [BIRDRAMON])   # attacker field – DP target
        runner.place_on_field(1, [GARUDAMON])   # attacker used to swing

        # Make Garudamon attack directly (no blockers, no other targets).
        # Ensure Garudamon is unsuspended so it can attack.
        garu = _perm_by_id(runner.game.player1, GARUDAMON)
        assert garu is not None
        garu.unsuspend()

        # Directly invoke the SecuritySkill flow by trashing the security card
        # with a side-effect that fires SecuritySkill. Since we want the test
        # to be robust without full attack orchestration, call the security
        # effect process directly — mirroring the security_attack() dispatch.
        from digimon_gym.engine.data.enums import EffectTiming
        sec_card = runner.game.player2.security_cards[0]
        sec_effects = [
            e for e in sec_card.effect_list(EffectTiming.SecuritySkill)
            if e.timing == EffectTiming.SecuritySkill
            and getattr(e, 'is_security_effect', False)
        ]
        assert len(sec_effects) >= 1, "Odin's Breath must expose a SecuritySkill effect"

        # Monkey-patch the select to pick Birdramon immediately
        original_fn = runner.game.effect_select_opponent_permanent
        target_bird = _perm_by_id(runner.game.player1, BIRDRAMON)
        assert target_bird is not None
        def immediate_select(player, callback, filter_fn=None, is_optional=False,
                             prompt=""):
            callback(target_bird)
        runner.game.effect_select_opponent_permanent = immediate_select
        try:
            sec_effects[0].on_process_callback({
                "game": runner.game,
                "player": runner.game.player2,
                "permanent": None,
                "card": sec_card,
                "attacker": garu,
                "turn_player": runner.game.turn_player,
                "opponent_player": runner.game.opponent_player,
            })
        finally:
            runner.game.effect_select_opponent_permanent = original_fn

        bird_after = _perm_by_id(runner.game.player1, BIRDRAMON)
        assert bird_after is not None
        assert bird_after.dp == 2000, (
            f"Birdramon (attacker side) should be at 2000 DP after Odin's "
            f"Breath [Security] effect resolved, got DP={bird_after.dp}"
        )
