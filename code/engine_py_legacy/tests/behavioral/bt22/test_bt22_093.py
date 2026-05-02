"""Behavioral tests for BT22-093 Ami Aiba (Tamer White, Cost 4, Traits: CS).

Card text:
  [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
  [Your Turn] When any of your Digimon digivolve into a Digimon with the [CS]
    trait, if it has a digivolution card with the same level as the digivolved
    Digimon, by suspending this Tamer, that Digimon may digivolve into a
    Digimon card with the [CS] trait in the hand without paying the cost.
  [Security] Play this card without paying the cost.
"""

import pytest

from engine_py_legacy.engine.core.permanent import Permanent
from engine_py_legacy.engine.data.card_database import CardDatabase
from engine_py_legacy.engine.data.card_registry import CardRegistry
from engine_py_legacy.engine.data.enums import EffectTiming


# ── Card IDs ────────────────────────────────────────────────────────────
AMI = "BT22-093"                 # White Tamer, cost 4, [CS]

# CS Digimons
AGUMON_CS = "BT22-008"           # Lv.3 Red [Reptile, CS], cost 3
VEEDRAMON_CS = "BT22-022"        # Lv.4 Blue [Dragonkin, CS], cost 5 (from cards.json)
AEROVEEDRAMON_CS = "BT22-023"    # Lv.5 Blue [CS], cost 7
GABUMON_CS = "BT22-017"          # Lv.3 Blue [CS], cost 3

# Non-CS Digimons
NON_CS_LV3 = "ST1-03"            # Lv.3 Red Agumon, [Reptile] — NO CS trait
NON_CS_LV4 = "ST1-04"            # Lv.4 Red Greymon — NO CS trait


# ── Helpers ─────────────────────────────────────────────────────────────

def _build_stack_permanent(game, player, card_ids):
    """Build a multi-card permanent directly (bottom-to-top order).

    Similar to runner.place_on_field but exposes the Permanent for mutation.
    """
    CardRegistry.ensure_initialized()
    db = CardDatabase()
    sources = [db.create_card_source(cid, player) for cid in card_ids]
    perm = Permanent(sources)
    perm._owner_game = game
    perm.turn_played = -1
    player.battle_area.append(perm)
    return perm


def _get_observer_effect(card_id):
    """Retrieve the digivolve-observer effect from the BT22-093 script.

    Uses a detached CardSource — effects that check
    `card.permanent_of_this_card()` will see None. Use
    `_observer_effect_from_permanent` when you need live permanent context.
    """
    CardRegistry.ensure_initialized()
    db = CardDatabase()
    script = db.get_script(card_id)
    from engine_py_legacy.engine.core.card_source import CardSource
    cs = CardSource()
    effects = script.get_card_effects(cs)
    for e in effects:
        if getattr(e, '_is_digivolve_observer', False):
            return e
    return None


def _observer_effect_from_permanent(perm):
    """Retrieve the live digivolve-observer effect from a placed permanent."""
    for source in perm.card_sources:
        for effect in source.effect_list(EffectTiming.NoTiming):
            if getattr(effect, '_is_digivolve_observer', False):
                effect.set_effect_source_permanent(perm)
                return effect
    return None


# ── C1: [Start of Your Main Phase] memory gain if opponent has a Digimon ──

@pytest.mark.behavioral
class TestBT22093StartOfMainPhase:
    """C1 — [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory."""

    def test_memory_gain_when_opponent_has_digimon(self, debug_runner):
        """Start of main phase triggers +1 memory when opponent has a Digimon."""
        runner = debug_runner(initial_memory=0)
        game = runner.game

        # Place Ami on our field and an opponent Digimon
        runner.place_on_field(1, [AMI])
        runner.place_on_field(2, [AGUMON_CS])

        mem_before = game.memory
        # Fire start-of-main-phase effects manually
        game.execute_effects(EffectTiming.OnStartMainPhase, {})
        mem_after = game.memory

        assert mem_after == mem_before + 1, (
            f"Memory should increase by 1 when opponent has a Digimon. "
            f"Before={mem_before}, after={mem_after}"
        )

    def test_no_memory_gain_when_opponent_has_no_digimon(self, debug_runner):
        """Start of main phase does NOT add memory if opponent has no Digimon."""
        runner = debug_runner(initial_memory=0)
        game = runner.game

        runner.place_on_field(1, [AMI])
        runner.clear_zone(2, "field")  # ensure opponent has nothing
        # Opponent has only tamers? Use a tamer (non-Digimon) to verify
        # "has a Digimon" check is strict.

        mem_before = game.memory
        game.execute_effects(EffectTiming.OnStartMainPhase, {})
        mem_after = game.memory

        assert mem_after == mem_before, (
            f"Memory should NOT change when opponent has no Digimon. "
            f"Before={mem_before}, after={mem_after}"
        )

    def test_no_memory_gain_when_opponent_has_only_tamer(self, debug_runner):
        """Tamers do not count as Digimon — effect must NOT fire."""
        runner = debug_runner(initial_memory=0)
        game = runner.game

        runner.place_on_field(1, [AMI])
        runner.clear_zone(2, "field")
        runner.place_on_field(2, ["BT22-094"])  # opponent Tamer (Yuugo Kamishiro)

        mem_before = game.memory
        game.execute_effects(EffectTiming.OnStartMainPhase, {})
        mem_after = game.memory

        assert mem_after == mem_before, (
            f"Memory must not change if opponent only has a Tamer. "
            f"Before={mem_before}, after={mem_after}"
        )

    def test_no_memory_gain_on_opponent_turn(self, debug_runner):
        """The effect is 'Start of Your Main Phase' — not opponent turn."""
        runner = debug_runner(initial_memory=0)
        game = runner.game

        runner.place_on_field(1, [AMI])
        runner.place_on_field(2, [AGUMON_CS])

        # Simulate it being the opponent's turn
        game.player1.is_my_turn = False
        game.player2.is_my_turn = True

        mem_before = game.memory
        game.execute_effects(EffectTiming.OnStartMainPhase, {})
        mem_after = game.memory

        assert mem_after == mem_before, (
            "Memory must not change when it is not our turn"
        )

        # Restore
        game.player1.is_my_turn = True
        game.player2.is_my_turn = False


# ── C2: [Your Turn] OnDigivolve observer — CS trait + same-level + suspend + chain digivolve ──

@pytest.mark.behavioral
class TestBT22093OnDigivolveObserver:
    """C2 — OnDigivolve observer with same-level condition and chain digivolve."""

    def test_observer_effect_is_optional(self):
        """Observer effect should be marked optional (player may decline)."""
        effect = _get_observer_effect(AMI)
        assert effect is not None, "Script must expose a digivolve-observer effect"
        assert effect.is_optional, (
            "BT22-093 observer effect should be marked is_optional=True"
        )

    def test_observer_does_not_use_is_when_digivolving(self):
        """Must NOT set is_when_digivolving=True (would dead-end the observer)."""
        effect = _get_observer_effect(AMI)
        assert effect is not None
        assert not effect.is_when_digivolving, (
            "BT22-093 observer must NOT have is_when_digivolving=True "
            "(would never fire since the Tamer != digivolved_perm; see BT11-094 discovery)"
        )

    def test_observer_condition_requires_cs_top_card(self, debug_runner):
        """Condition must fail when the digivolved top card lacks [CS]."""
        runner = debug_runner(initial_memory=0)
        game = runner.game
        player = game.player1

        # Place Ami tamer
        ami_perm = runner.place_on_field(1, [AMI])
        # Place a NON-CS digimon stack with same-level source underneath
        # Stack: Lv4 NON-CS, Lv4 NON-CS top. Even with same-level, no CS → no fire.
        digi_perm = _build_stack_permanent(game, player, [NON_CS_LV4, NON_CS_LV4])

        effect = _observer_effect_from_permanent(ami_perm)
        assert effect is not None
        ctx = {
            'game': game, 'player': player, 'permanent': ami_perm,
            'card': ami_perm.top_card,
            'digivolved_permanent': digi_perm,
        }
        assert effect.can_use_condition(ctx) is False, (
            "Condition must reject permanents without [CS] trait on top card"
        )

    def test_observer_condition_requires_same_level_in_stack(self, debug_runner):
        """Condition must fail when no stack source shares the top's level."""
        runner = debug_runner(initial_memory=0)
        game = runner.game
        player = game.player1

        ami_perm = runner.place_on_field(1, [AMI])
        # Stack: Lv3 CS, Lv4 CS top — different levels
        digi_perm = _build_stack_permanent(game, player, [AGUMON_CS, VEEDRAMON_CS])

        effect = _observer_effect_from_permanent(ami_perm)
        assert effect is not None
        ctx = {
            'game': game, 'player': player, 'permanent': ami_perm,
            'card': ami_perm.top_card,
            'digivolved_permanent': digi_perm,
        }
        assert effect.can_use_condition(ctx) is False, (
            "Condition must reject stacks where no underlying card matches "
            "the top card's level"
        )

    def test_observer_condition_passes_with_cs_and_same_level(self, debug_runner):
        """Condition passes when top is [CS] and a stack source shares its level."""
        runner = debug_runner(initial_memory=0)
        game = runner.game
        player = game.player1

        ami_perm = runner.place_on_field(1, [AMI])
        # Stack bottom-to-top: Lv4 CS (VEEDRAMON_CS), Lv4 CS top (another VEEDRAMON_CS)
        digi_perm = _build_stack_permanent(game, player, [VEEDRAMON_CS, VEEDRAMON_CS])

        effect = _observer_effect_from_permanent(ami_perm)
        assert effect is not None
        ctx = {
            'game': game, 'player': player, 'permanent': ami_perm,
            'card': ami_perm.top_card,
            'digivolved_permanent': digi_perm,
        }
        assert effect.can_use_condition(ctx) is True, (
            "Condition must pass when top is [CS] and a stack card shares its level"
        )

    def test_observer_condition_rejects_when_tamer_suspended(self, debug_runner):
        """Suspending-cost: cannot fire if Ami is already suspended."""
        runner = debug_runner(initial_memory=0)
        game = runner.game
        player = game.player1

        ami_perm = runner.place_on_field(1, [AMI], is_suspended=True)
        digi_perm = _build_stack_permanent(game, player, [VEEDRAMON_CS, VEEDRAMON_CS])

        effect = _observer_effect_from_permanent(ami_perm)
        assert effect is not None
        ctx = {
            'game': game, 'player': player, 'permanent': ami_perm,
            'card': ami_perm.top_card,
            'digivolved_permanent': digi_perm,
        }
        assert effect.can_use_condition(ctx) is False, (
            "Cannot pay suspend cost when tamer is already suspended"
        )

    def test_observer_condition_rejects_when_not_your_turn(self, debug_runner):
        """[Your Turn] — must not fire on opponent's turn."""
        runner = debug_runner(initial_memory=0)
        game = runner.game
        player = game.player1

        ami_perm = runner.place_on_field(1, [AMI])
        digi_perm = _build_stack_permanent(game, player, [VEEDRAMON_CS, VEEDRAMON_CS])

        # Flip turn ownership
        player.is_my_turn = False
        game.player2.is_my_turn = True

        effect = _observer_effect_from_permanent(ami_perm)
        assert effect is not None
        ctx = {
            'game': game, 'player': player, 'permanent': ami_perm,
            'card': ami_perm.top_card,
            'digivolved_permanent': digi_perm,
        }
        assert effect.can_use_condition(ctx) is False, (
            "[Your Turn] effect must not fire on opponent's turn"
        )

        # Restore
        player.is_my_turn = True
        game.player2.is_my_turn = False

    def test_observer_process_suspends_tamer_and_offers_digivolve(self, debug_runner):
        """Firing the observer should suspend Ami and prompt digivolve-from-hand."""
        runner = debug_runner(initial_memory=0)
        game = runner.game
        player = game.player1

        ami_perm = runner.place_on_field(1, [AMI])
        # Eligible stack: Lv4 CS under Lv4 CS top
        digi_perm = _build_stack_permanent(game, player, [VEEDRAMON_CS, VEEDRAMON_CS])

        # Provide a CS Digimon in hand to digivolve into (any CS Digimon works)
        runner.inject_card(1, AEROVEEDRAMON_CS, "hand")

        stack_before = len(digi_perm.card_sources)
        assert not ami_perm.is_suspended

        # Fire the observer directly
        game._fire_digivolve_observers(digi_perm)

        # Auto-resolve the digivolve selection
        runner.auto_resolve()

        # Ami must have been suspended (cost paid)
        assert ami_perm.is_suspended, (
            "Ami must be suspended after the observer fires (suspend cost paid)"
        )
        # Stack should have grown by one (AeroVeedramon added on top via digivolve-from-hand)
        assert len(digi_perm.card_sources) == stack_before + 1, (
            f"Stack should grow by one (chain digivolve). "
            f"Before={stack_before}, after={len(digi_perm.card_sources)}"
        )
        assert digi_perm.top_card.c_entity_base.card_id == AEROVEEDRAMON_CS, (
            "Top card should now be the chained-digivolve CS Digimon"
        )

    def test_observer_chained_digivolve_is_free(self, debug_runner):
        """Chain digivolve via Ami's effect should be free (no memory cost)."""
        runner = debug_runner(initial_memory=0)
        game = runner.game
        player = game.player1

        ami_perm = runner.place_on_field(1, [AMI])
        digi_perm = _build_stack_permanent(game, player, [VEEDRAMON_CS, VEEDRAMON_CS])
        runner.inject_card(1, AEROVEEDRAMON_CS, "hand")  # cost 7 normally

        mem_before = game.memory

        game._fire_digivolve_observers(digi_perm)
        runner.auto_resolve()

        mem_after = game.memory
        # Engine `player.lose_memory(cost)` with cost_override=0 → no change
        assert mem_after == mem_before, (
            f"Chain digivolve must be free. Memory before={mem_before}, after={mem_after}"
        )

    def test_observer_filter_rejects_non_cs_hand_target(self, debug_runner):
        """When only non-CS digivolve targets are in hand, nothing should digivolve."""
        runner = debug_runner(initial_memory=0)
        game = runner.game
        player = game.player1

        ami_perm = runner.place_on_field(1, [AMI])
        digi_perm = _build_stack_permanent(game, player, [VEEDRAMON_CS, VEEDRAMON_CS])
        # Clear hand, inject only a NON-CS Digimon — Ami's filter should reject it
        runner.clear_zone(1, "hand")
        runner.inject_card(1, NON_CS_LV4, "hand")

        stack_before = len(digi_perm.card_sources)

        game._fire_digivolve_observers(digi_perm)
        runner.auto_resolve()

        # Stack should NOT grow — no legal target was available
        assert len(digi_perm.card_sources) == stack_before, (
            "Stack must not grow when hand contains only non-CS targets"
        )


# ── C3: [Security] Play this card without paying the cost ──

@pytest.mark.behavioral
class TestBT22093SecurityPlay:
    """C3 — [Security] Play this card without paying the cost."""

    def test_security_effect_has_process_callback(self):
        """The security effect must have on_process_callback wired up.

        A missing callback is the BT22 set-wide bug: the security effect is
        listed but silently does nothing.
        """
        CardRegistry.ensure_initialized()
        db = CardDatabase()
        script = db.get_script(AMI)
        from engine_py_legacy.engine.core.card_source import CardSource
        cs = CardSource()
        effects = script.get_card_effects(cs)
        security_effects = [e for e in effects if getattr(e, 'is_security_effect', False)]
        assert len(security_effects) == 1, (
            "BT22-093 should expose exactly one security effect"
        )
        assert security_effects[0].on_process_callback is not None, (
            "[Security] effect must have an on_process_callback (play from security)"
        )

    def test_security_plays_tamer_when_revealed(self, debug_runner):
        """When BT22-093 is revealed in security, it should enter the battle area."""
        runner = debug_runner(initial_memory=5)

        # Put Ami on defender's (P2) security top
        runner.inject_card(2, AMI, "security_top")
        # Attacker for P1
        runner.place_on_field(1, ["ST1-02"], turn_played=-1)

        runner.set_phase("Main")
        attack = runner.find_action("Attack player with")
        assert attack is not None, f"Should be able to attack. Actions: {runner.actions()}"
        runner.execute(attack)
        runner.auto_resolve()

        snap = runner.snapshot()
        ami_on_p2 = any(s.card_id == AMI for s in snap.p2_field)
        assert ami_on_p2, (
            "Ami Aiba should be played to P2 battle area via [Security] effect"
        )
