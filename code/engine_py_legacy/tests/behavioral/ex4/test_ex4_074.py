"""Behavioral tests for EX4-074 ShineGreymon: Ruin Mode.

Lv.7, Purple/Yellow, Light Dragon, DP 15000, Cost 14.

Effect text (SOURCE OF TRUTH):
    [When Digivolving] [On Deletion]
        Until the end of your opponent's next turn, all of your opponent's
        Digimon get -5000DP.
    [End of Attack]
        Delete this Digimon and 1 of your opponent's Digimon, and
        <Recovery +1 (Deck)>. Then, if you have a Tamer in play, hatch 1
        Digi-Egg card to an empty space in your breeding area.

Alt-digi: from [ShineGreymon] for cost 4.
"""

import pytest

from engine_py_legacy.engine.data.enums import EffectTiming
from engine_py_legacy.engine.interfaces.modifiers import ModifierType


def _get_effects(perm):
    """Return all effects exposed by the top card of a permanent."""
    return perm.top_card.effect_list(None)


def _get_when_digivolving(perm):
    return [
        e for e in _get_effects(perm)
        if e.timing == EffectTiming.OnEnterFieldAnyone
        and getattr(e, 'is_when_digivolving', False)
    ]


def _get_on_deletion(perm):
    return [
        e for e in _get_effects(perm)
        if e.timing == EffectTiming.OnDestroyedAnyone
        and getattr(e, 'is_on_deletion', False)
    ]


def _get_end_of_attack(perm):
    return [e for e in _get_effects(perm) if e.timing == EffectTiming.OnEndAttack]


@pytest.mark.behavioral
class TestEX4074AltDigivolve:
    """Alt digivolve: from [ShineGreymon] for cost 4."""

    def test_alt_digi_effect_exists(self, debug_runner):
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])

        alt = [e for e in _get_effects(perm) if getattr(e, '_alt_digi_cost', None) is not None]
        assert len(alt) == 1, "Should have exactly one alt-digi effect"
        assert alt[0]._alt_digi_cost == 4
        assert alt[0]._alt_digi_name == "ShineGreymon"

    def test_alt_digi_name_matches_shinegreymon(self, debug_runner):
        """Alt-digi uses `_alt_digi_name='ShineGreymon'` — validated by the
        digivolve_validator against the base permanent, not via can_use_condition.
        """
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])

        alt = [e for e in _get_effects(perm) if getattr(e, '_alt_digi_cost', None) is not None][0]
        # Standard alt-digi pattern: name attr set, no other filters.
        assert alt._alt_digi_name == "ShineGreymon"
        assert getattr(alt, '_alt_digi_level', None) is None
        assert getattr(alt, '_alt_digi_color', None) is None


@pytest.mark.behavioral
class TestEX4074WhenDigivolving:
    """[When Digivolving] all opponent Digimon get -5000 DP until end of opp next turn."""

    def test_when_digivolving_effect_exists(self, debug_runner):
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])
        wd = _get_when_digivolving(perm)
        assert len(wd) >= 1, "Should have [When Digivolving] effect"

    def test_when_digivolving_applies_minus_5000_to_opponent_digimon(self, debug_runner):
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])
        game = runner.game

        opp_perm = runner.place_on_field(2, ["ST1-08"])  # Garudamon 7000 DP
        assert opp_perm.dp == 7000

        wd = _get_when_digivolving(perm)[0]
        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert opp_perm.dp == 2000, f"Expected 7000-5000=2000 DP, got {opp_perm.dp}"

    def test_when_digivolving_applies_to_all_opponent_digimon(self, debug_runner):
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])
        game = runner.game

        opp1 = runner.place_on_field(2, ["ST1-08"])  # 7000
        opp2 = runner.place_on_field(2, ["ST1-09"])  # 7000

        wd = _get_when_digivolving(perm)[0]
        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert opp1.dp == 2000, f"First opp should be -5000 DP, got {opp1.dp}"
        assert opp2.dp == 2000, f"Second opp should be -5000 DP, got {opp2.dp}"

    def test_when_digivolving_does_not_affect_own_digimon(self, debug_runner):
        """Target-equality: -5000 must NOT leak to the controller's own Digimon."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])
        game = runner.game

        own_other = runner.place_on_field(1, ["ST1-08"])  # 7000 Garudamon owned by me
        opp = runner.place_on_field(2, ["ST1-08"])        # 7000 Garudamon owned by opp
        assert own_other.dp == 7000
        assert opp.dp == 7000

        wd = _get_when_digivolving(perm)[0]
        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        # Only opponent's Digimon lose DP; owner's Digimon are untouched.
        assert own_other.dp == 7000, \
            f"Own Digimon must not be affected, got {own_other.dp}"
        assert opp.dp == 2000, \
            f"Opp Digimon should be at 2000, got {opp.dp}"

    def test_when_digivolving_registers_as_modifier_entry(self, debug_runner):
        """Effect must register CHANGE_DP modifiers (not transient change_dp)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])
        game = runner.game
        opp = runner.place_on_field(2, ["ST1-08"])

        wd = _get_when_digivolving(perm)[0]
        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        entries = game.modifiers._modifiers.get(ModifierType.CHANGE_DP, [])
        assert len(entries) >= 1, \
            "Should register at least one CHANGE_DP modifier entry"

    def test_when_digivolving_modifier_expires_end_of_opponent_next_turn(self, debug_runner):
        """Modifier should clear when opp's next turn ends (= start of our next turn)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])
        game = runner.game
        opp = runner.place_on_field(2, ["ST1-08"])  # 7000

        # Ensure turn_player is player1 so granting_player=player1
        assert game.turn_player is game.player1

        wd = _get_when_digivolving(perm)[0]
        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        assert opp.dp == 2000

        # Simulate start of player1's next turn (after opp's turn ends).
        # clear_opponent_turn_expiry(current_turn_player) clears entries
        # whose granting_player is current_turn_player (us).
        game.modifiers.clear_opponent_turn_expiry(game.player1)

        assert opp.dp == 7000, \
            f"Modifier should expire; expected 7000, got {opp.dp}"

    def test_when_digivolving_modifier_survives_intermediate_turn_flips(self, debug_runner):
        """Modifier must NOT clear prematurely when opp's turn starts."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])
        game = runner.game
        opp = runner.place_on_field(2, ["ST1-08"])

        wd = _get_when_digivolving(perm)[0]
        wd.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        assert opp.dp == 2000

        # Opponent's turn begins: clear_opponent_turn_expiry(opponent) —
        # clears entries granted by opponent (none of ours).
        game.modifiers.clear_opponent_turn_expiry(game.player2)
        assert opp.dp == 2000, \
            f"Must not clear on opp's turn start; got {opp.dp}"


@pytest.mark.behavioral
class TestEX4074OnDeletion:
    """[On Deletion] all opponent Digimon get -5000 DP until end of opp next turn."""

    def test_on_deletion_effect_exists(self, debug_runner):
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])
        od = _get_on_deletion(perm)
        assert len(od) >= 1, "Should have [On Deletion] effect"

    def test_on_deletion_applies_minus_5000(self, debug_runner):
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])
        game = runner.game
        opp = runner.place_on_field(2, ["ST1-08"])

        od = _get_on_deletion(perm)[0]
        od.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })

        assert opp.dp == 2000

    def test_on_deletion_modifier_persists_after_source_leaves_field(self, debug_runner):
        """After [On Deletion] fires and EX4-074 is trashed, the debuff must remain."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])
        game = runner.game
        opp = runner.place_on_field(2, ["ST1-08"])

        od = _get_on_deletion(perm)[0]
        od.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
        })
        assert opp.dp == 2000

        # Simulate the post-deletion cleanup: source permanent leaves field.
        if perm in game.player1.battle_area:
            game.player1.battle_area.remove(perm)
        game.cleanup_modifiers_for_permanent(perm)

        # The modifier should still be active because its source_permanent
        # is the OPPONENT digimon (target), not EX4-074.
        assert opp.dp == 2000, \
            f"Debuff must persist after source leaves, got {opp.dp}"

    def test_on_deletion_has_is_on_deletion_flag(self, debug_runner):
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])
        od = _get_on_deletion(perm)[0]
        assert od.is_on_deletion is True


@pytest.mark.behavioral
class TestEX4074EndOfAttack:
    """[End of Attack] delete self + 1 opp, Recovery +1, hatch if tamer."""

    def test_end_of_attack_effect_exists(self, debug_runner):
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])
        ea = _get_end_of_attack(perm)
        assert len(ea) >= 1, "Should have [End of Attack] effect"

    def test_end_of_attack_condition_only_self(self, debug_runner):
        """condition() must return False when a DIFFERENT permanent is attacking."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])
        other = runner.place_on_field(1, ["ST1-07"])

        ea = _get_end_of_attack(perm)[0]
        assert ea.can_use_condition({
            'permanent': perm,
            'attacking_permanent': other,
        }) is False, "Should NOT trigger for another permanent's attack"
        assert ea.can_use_condition({
            'permanent': perm,
            'attacking_permanent': perm,
        }) is True, "Should trigger for this permanent's attack"

    def test_end_of_attack_deletes_self(self, debug_runner):
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])
        game = runner.game
        runner.place_on_field(2, ["ST1-03"])

        ea = _get_end_of_attack(perm)[0]
        ea.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'attacking_permanent': perm,
        })
        runner.auto_resolve()

        assert perm not in game.player1.battle_area, \
            "ShineGreymon: Ruin Mode should be self-deleted"

    def test_end_of_attack_deletes_one_opponent_digimon(self, debug_runner):
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])
        game = runner.game
        opp = runner.place_on_field(2, ["ST1-03"])

        ea = _get_end_of_attack(perm)[0]
        ea.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'attacking_permanent': perm,
        })
        runner.auto_resolve()

        assert opp not in game.player2.battle_area, \
            "Opponent's Digimon should be deleted"

    def test_end_of_attack_only_one_opponent_digimon_deleted(self, debug_runner):
        """Effect targets exactly 1 opp Digimon, not multiple."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])
        game = runner.game
        opp1 = runner.place_on_field(2, ["ST1-03"])
        opp2 = runner.place_on_field(2, ["ST1-07"])

        ea = _get_end_of_attack(perm)[0]
        ea.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'attacking_permanent': perm,
        })
        runner.auto_resolve()

        # Exactly one of the two opp digimon should remain.
        remaining = [p for p in game.player2.battle_area if p is opp1 or p is opp2]
        assert len(remaining) == 1, \
            f"Exactly 1 opp Digimon should be deleted, {len(remaining)} remain"

    def test_end_of_attack_recovery_plus_1(self, debug_runner):
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])
        game = runner.game
        runner.place_on_field(2, ["ST1-03"])

        sec_before = len(game.player1.security_cards)
        deck_before = len(game.player1.library_cards)

        ea = _get_end_of_attack(perm)[0]
        ea.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'attacking_permanent': perm,
        })
        runner.auto_resolve()

        assert len(game.player1.security_cards) == sec_before + 1, \
            f"Security should increase by 1 (was {sec_before}, now {len(game.player1.security_cards)})"
        assert len(game.player1.library_cards) == deck_before - 1, \
            "Deck should lose 1 card for Recovery (Deck)"

    def test_end_of_attack_hatches_with_tamer(self, debug_runner):
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])
        game = runner.game
        runner.place_on_field(1, ["ST1-12"])  # Tai Kamiya tamer
        runner.place_on_field(2, ["ST1-03"])

        # Ensure breeding starts empty, egg deck has cards.
        game.player1.breeding_area = None
        has_eggs = len(game.player1.digitama_library_cards) > 0
        assert has_eggs, "Test requires non-empty egg deck"
        eggs_before = len(game.player1.digitama_library_cards)

        ea = _get_end_of_attack(perm)[0]
        ea.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'attacking_permanent': perm,
        })
        runner.auto_resolve()

        assert game.player1.breeding_area is not None, \
            "Breeding area should contain a hatched Digi-Egg"
        assert len(game.player1.digitama_library_cards) == eggs_before - 1, \
            "Should consume exactly 1 egg from egg deck"

    def test_end_of_attack_no_hatch_without_tamer(self, debug_runner):
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])
        game = runner.game
        # Remove any accidental tamers
        game.player1.battle_area = [p for p in game.player1.battle_area if not p.is_tamer]
        runner.place_on_field(2, ["ST1-03"])

        game.player1.breeding_area = None
        eggs_before = len(game.player1.digitama_library_cards)

        ea = _get_end_of_attack(perm)[0]
        ea.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'attacking_permanent': perm,
        })
        runner.auto_resolve()

        assert game.player1.breeding_area is None, \
            "No tamer → no hatch, breeding must stay empty"
        assert len(game.player1.digitama_library_cards) == eggs_before, \
            "Egg deck should be untouched"

    def test_end_of_attack_no_hatch_if_breeding_occupied(self, debug_runner):
        """Card text says 'hatch 1 Digi-Egg card to an empty space' — if breeding
        is occupied, no hatch should happen (still self-delete and recovery)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])
        game = runner.game
        runner.place_on_field(1, ["ST1-12"])  # Tamer present
        runner.place_on_field(2, ["ST1-03"])

        # Pre-populate breeding area with a hatched egg.
        runner.place_in_breeding(1, ["ST1-01"])
        assert game.player1.breeding_area is not None
        existing_breeding = game.player1.breeding_area
        eggs_before = len(game.player1.digitama_library_cards)

        ea = _get_end_of_attack(perm)[0]
        ea.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'attacking_permanent': perm,
        })
        runner.auto_resolve()

        # Breeding slot must be unchanged (still the pre-existing occupant).
        assert game.player1.breeding_area is existing_breeding, \
            "Must not overwrite occupied breeding slot"
        assert len(game.player1.digitama_library_cards) == eggs_before, \
            "Egg deck untouched when breeding is occupied"

    def test_end_of_attack_still_self_deletes_without_opp_digimon(self, debug_runner):
        """If no opp digimon to delete: still self-delete, still Recovery+1, still hatch."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])
        game = runner.game
        runner.place_on_field(1, ["ST1-12"])  # Tamer
        # No opp digimon

        sec_before = len(game.player1.security_cards)

        ea = _get_end_of_attack(perm)[0]
        ea.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'attacking_permanent': perm,
        })
        runner.auto_resolve()

        assert perm not in game.player1.battle_area, \
            "Self-delete must still occur"
        assert len(game.player1.security_cards) == sec_before + 1, \
            "Recovery +1 must still occur"

    def test_end_of_attack_hatches_even_without_opp_digimon(self, debug_runner):
        """Hatch still happens when no opp digimon and tamer present."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, ["EX4-074"])
        game = runner.game
        runner.place_on_field(1, ["ST1-12"])  # Tamer
        game.player1.breeding_area = None
        eggs_before = len(game.player1.digitama_library_cards)

        ea = _get_end_of_attack(perm)[0]
        ea.on_process_callback({
            'player': game.player1,
            'game': game,
            'permanent': perm,
            'attacking_permanent': perm,
        })
        runner.auto_resolve()

        assert game.player1.breeding_area is not None, \
            "Should hatch even without opp digimon target"
        assert len(game.player1.digitama_library_cards) == eggs_before - 1
