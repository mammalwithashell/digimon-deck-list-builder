"""Behavioral tests for the synthetic Security-effect pilots.

These tests pin the Python engine's behavior on the TEST-020/021/022
pilots and double as the authoritative specification that the matching
Rust pilots (in `digimon-engine/src/cards/test_cards.rs` +
`digimon-engine/tests/security_effects.rs`) must satisfy for
cross-engine parity (RUST_PYTHON_PARITY.md §2.5).

Card texts:
  - TEST-020: [Security] Draw 2 cards.
  - TEST-021: [Security] Play this card without paying its cost.
  - TEST-022: [Security] Gain 3 memory.
"""

from __future__ import annotations

import pytest

from digimon_gym.engine.debug.state_injection import clear_zone as _cz

TEST_020 = "TEST-020"
TEST_021 = "TEST-021"
TEST_022 = "TEST-022"

ATTACKER = "ST1-03"  # Red Lv3 Digimon, used purely as a body to source the attack
FILLER_DECK = [ATTACKER] * 50


def _setup(debug_runner, *, security_card: str, initial_memory: int = 5):
    """Common scaffold: attacker on P1 field, one TEST card on top of P2 security."""
    runner = debug_runner(
        deck1=FILLER_DECK, deck2=FILLER_DECK, initial_memory=initial_memory
    )
    game = runner.game

    runner.place_on_field(1, [ATTACKER])
    _cz(game, 2, "security")
    runner.inject_card(2, security_card, "security_top")
    return runner, game


@pytest.mark.behavioral
class TestSecurityPilotTest020:
    """TEST-020: trigger-and-trash (draw 2)."""

    def test_draws_two_cards_from_security(self, debug_runner):
        runner, game = _setup(debug_runner, security_card=TEST_020)

        hand_before = len(game.player2.hand_cards)
        attacker = game.player1.battle_area[0]
        game.player2.security_attack(attacker)
        runner.auto_resolve()

        assert len(game.player2.hand_cards) == hand_before + 2, (
            "[Security] Draw 2 must draw two cards for the defender"
        )

    def test_card_trashes_after_check(self, debug_runner):
        runner, game = _setup(debug_runner, security_card=TEST_020)

        attacker = game.player1.battle_area[0]
        game.player2.security_attack(attacker)
        runner.auto_resolve()

        trash_ids = [c.card_id for c in game.player2.trash_cards]
        assert TEST_020 in trash_ids, (
            "TEST-020 should trash after resolving (no play_from_security call)"
        )
        # And NOT on the field
        field_ids = [
            p.top_card.card_id for p in game.player2.battle_area if p.top_card
        ]
        assert TEST_020 not in field_ids


@pytest.mark.behavioral
class TestSecurityPilotTest021:
    """TEST-021: play-from-security (card stays on defender's field)."""

    def test_plays_self_from_security(self, debug_runner):
        runner, game = _setup(debug_runner, security_card=TEST_021)

        attacker = game.player1.battle_area[0]
        game.player2.security_attack(attacker)
        runner.auto_resolve()

        field_ids = [
            p.top_card.card_id for p in game.player2.battle_area if p.top_card
        ]
        assert TEST_021 in field_ids, (
            "TEST-021's [Security] must put the card on the defender's field"
        )
        trash_ids = [c.card_id for c in game.player2.trash_cards]
        assert TEST_021 not in trash_ids, (
            "TEST-021 must NOT also trash — _security_played should gate the trash"
        )

    def test_bypasses_cost(self, debug_runner):
        """With 0 memory, the free security play should still land."""
        runner, game = _setup(
            debug_runner, security_card=TEST_021, initial_memory=0
        )

        attacker = game.player1.battle_area[0]
        game.player2.security_attack(attacker)
        runner.auto_resolve()

        field_ids = [
            p.top_card.card_id for p in game.player2.battle_area if p.top_card
        ]
        assert TEST_021 in field_ids, (
            "[Security] play-from-security must bypass memory cost"
        )


@pytest.mark.behavioral
class TestSecurityPilotTest022:
    """TEST-022: memory-gain trigger (gain 3 memory)."""

    def test_gains_memory_and_trashes(self, debug_runner):
        runner, game = _setup(debug_runner, security_card=TEST_022)

        memory_before = game.memory
        attacker = game.player1.battle_area[0]
        game.player2.security_attack(attacker)
        runner.auto_resolve()

        # Memory moved in the defender's favor.
        assert game.memory != memory_before, (
            "TEST-022 should shift memory on resolution"
        )
        # Card ends up in trash (no play_from_security call).
        trash_ids = [c.card_id for c in game.player2.trash_cards]
        assert TEST_022 in trash_ids


@pytest.mark.behavioral
class TestPilotScriptRegistration:
    """Sanity checks: ensure the TEST- scripts landed in the script registry
    with the right timing so the Rust/Python parity story is verifiable."""

    @pytest.mark.parametrize("card_id", [TEST_020, TEST_021, TEST_022])
    def test_registers_security_skill_effect(self, card_id):
        from digimon_gym.engine.data.card_database import CardDatabase
        from digimon_gym.engine.data.enums import EffectTiming

        db = CardDatabase()
        cs = db.create_card_source(card_id)
        assert cs is not None, f"{card_id} must be present in CardDatabase"
        effects = cs.effect_list(None)
        security_effects = [
            e
            for e in effects
            if e.timing == EffectTiming.SecuritySkill
            and getattr(e, "is_security_effect", False)
        ]
        assert len(security_effects) == 1, (
            f"{card_id} must expose exactly one [SecuritySkill] effect with "
            f"is_security_effect=True, got {len(security_effects)}"
        )
