"""Behavioral tests for BT22-043 Terriermon (Digimon, Green/Yellow, Lv.3).

Card text:
    [Your Turn] [Once Per Turn] When effects place Digimon cards with the
        [CS] trait in this Digimon's digivolution cards, if you have 1 or
        fewer Tamers, you may play 1 Tamer card with the [CS] trait from
        your hand without paying the cost.
    Inherited: [Main] [Once Per Turn] By placing this [CS] trait Digimon's
        top stacked card as its bottom digivolution card, <Draw 1>.
    Alt digivolve: Lv.2 w/[CS] trait for cost 0.
"""

import pytest

from engine_py_legacy.engine.data.enums import EffectTiming


TERRIERMON = "BT22-043"
CS_LV2_EGG = "BT22-004"     # Wanyamon Lv.2 CS egg (for alt-digi base)
CS_LV3 = "BT22-008"         # Agumon Lv.3 CS (used as "added card" for C1)
CS_LV4 = "BT22-010"         # Meramon Lv.4 CS (to host C2 when stacked)
CS_LV5 = "BT22-023"         # AeroVeedramon Lv.5 CS
CS_TAMER_3 = "BT22-089"     # Mirei Mikagura (cost 3 CS Tamer)
CS_TAMER_4 = "BT22-083"     # Yuuko Kamishiro (cost 4 CS Tamer)
PLAIN_TAMER = "ST19-14"     # Arisa Kinosaki (LIBERATOR Tamer, NOT CS)
PLAIN_DIGIMON = "ST1-03"    # Agumon (Red, no CS)


def _get_effects(perm):
    effects = []
    for source in perm.card_sources:
        effects.extend(source.effect_list(EffectTiming.NoTiming))
    return effects


def _find_effect(perm, *, hash_string=None, predicate=None):
    for eff in _get_effects(perm):
        if hash_string and eff.hash_string == hash_string:
            return eff
        if predicate and predicate(eff):
            return eff
    return None


def _find_c1(perm):
    return _find_effect(perm, hash_string="PlayTamer_BT22_043")


def _find_c2(perm):
    return _find_effect(perm, hash_string="ReturnDigivolutionCards_BT22_043")


@pytest.fixture
def make_runner(debug_runner):
    """Build a runner with a dummy deck containing Terriermon + support."""
    def _build(**kwargs):
        deck1 = (
            [TERRIERMON] * 4
            + [CS_LV3] * 8
            + [CS_LV4] * 8
            + [CS_TAMER_3] * 4
            + [CS_TAMER_4] * 4
            + [PLAIN_TAMER] * 4
            + [PLAIN_DIGIMON] * 18
        )
        deck2 = [PLAIN_DIGIMON] * 50
        return debug_runner(deck1=deck1, deck2=deck2, initial_memory=5, **kwargs)
    return _build


@pytest.mark.behavioral
class TestBT22043AltDigivolve:
    """Alt digivolve requirement: Lv.2 w/[CS] trait for cost 0."""

    def test_alt_digi_effect_present(self, make_runner):
        runner = make_runner()
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(TERRIERMON, runner.game.player1)
        effects = cs.effect_list(EffectTiming.NoTiming)
        alt_effects = [e for e in effects if getattr(e, '_alt_digi_cost', None) is not None]
        assert len(alt_effects) == 1, (
            f"Expected exactly 1 alt-digi effect, got {len(alt_effects)}"
        )
        alt = alt_effects[0]
        assert alt._alt_digi_cost == 0
        assert alt._alt_digi_level == 2
        assert getattr(alt, '_alt_digi_trait', None) == "CS", (
            "Alt-digi should require [CS] trait on base"
        )

    def test_alt_digi_matches_lv2_cs_base(self, make_runner):
        """Validator should accept a Lv.2 [CS] base as a valid alt-digi source."""
        runner = make_runner()
        from engine_py_legacy.engine.data.card_database import CardDatabase
        from engine_py_legacy.engine.validation.digivolve_validator import _check_alt_digivolve

        db = CardDatabase()
        terriermon = db.create_card_source(TERRIERMON, runner.game.player1)
        base_perm = runner.place_on_field(1, [CS_LV2_EGG])
        assert base_perm.top_card.level == 2
        assert 'CS' in (base_perm.top_card.card_traits or [])
        assert _check_alt_digivolve(terriermon, base_perm), (
            "Alt-digi should match Lv.2 CS base"
        )


@pytest.mark.behavioral
class TestBT22043C1_PlayCsTamer:
    """C1 — [YT][OPT] OnAddDigivolutionCards CS Digimon → play CS Tamer free."""

    # ── Effect shape ────────────────────────────────────────────────

    def test_c1_effect_shape(self, make_runner):
        runner = make_runner()
        perm = runner.place_on_field(1, [TERRIERMON])
        eff = _find_c1(perm)
        assert eff is not None, "Should find C1 effect on Terriermon"
        assert eff.timing == EffectTiming.OnAddDigivolutionCards, (
            f"C1 should trigger on OnAddDigivolutionCards, got {eff.timing}"
        )
        assert eff.is_optional, "C1 is a 'may play' effect, must be optional"
        assert eff.max_count_per_turn == 1, "C1 is OPT, max_count_per_turn == 1"
        assert not eff.is_inherited_effect, (
            "C1 is on top-text (non-inherited) — inherited flag must be False"
        )

    # ── Condition ──────────────────────────────────────────────────

    def test_c1_condition_passes_with_cs_digimon_added(self, make_runner):
        """Condition should pass when a CS Digimon is the added_card and this
        Terriermon is the event_permanent."""
        runner = make_runner()
        perm = runner.place_on_field(1, [TERRIERMON])
        runner.inject_card(1, CS_TAMER_3, "hand")  # need a CS Tamer in hand

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        added = db.create_card_source(CS_LV4, runner.game.player1)
        assert added.is_digimon and 'CS' in added.card_traits

        eff = _find_c1(perm)
        ctx = {
            'game': runner.game,
            'player': runner.game.player1,
            'permanent': perm,
            'event_permanent': perm,
            'added_card': added,
        }
        assert eff.can_use_condition(ctx) is True

    def test_c1_condition_rejects_non_cs_added_digimon(self, make_runner):
        runner = make_runner()
        perm = runner.place_on_field(1, [TERRIERMON])
        runner.inject_card(1, CS_TAMER_3, "hand")

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        added = db.create_card_source(PLAIN_DIGIMON, runner.game.player1)
        assert 'CS' not in (added.card_traits or [])

        eff = _find_c1(perm)
        ctx = {
            'game': runner.game,
            'player': runner.game.player1,
            'permanent': perm,
            'event_permanent': perm,
            'added_card': added,
        }
        assert eff.can_use_condition(ctx) is False

    def test_c1_condition_rejects_other_permanent_event(self, make_runner):
        """If the event_permanent is a different permanent, C1 should NOT fire."""
        runner = make_runner()
        terrier_perm = runner.place_on_field(1, [TERRIERMON])
        other_perm = runner.place_on_field(1, [CS_LV4])
        runner.inject_card(1, CS_TAMER_3, "hand")

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        added = db.create_card_source(CS_LV3, runner.game.player1)

        eff = _find_c1(terrier_perm)
        ctx = {
            'game': runner.game,
            'player': runner.game.player1,
            'permanent': terrier_perm,
            'event_permanent': other_perm,  # NOT Terriermon's perm
            'added_card': added,
        }
        assert eff.can_use_condition(ctx) is False, (
            "C1 must only fire when Terriermon's own perm is the event_permanent"
        )

    def test_c1_condition_rejects_opponent_turn(self, make_runner):
        runner = make_runner()
        perm = runner.place_on_field(1, [TERRIERMON])
        runner.inject_card(1, CS_TAMER_3, "hand")

        # Flip is_my_turn so player1 (Terriermon's owner) is NOT turn player
        runner.game.player1.is_my_turn = False
        runner.game.player2.is_my_turn = True

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        added = db.create_card_source(CS_LV3, runner.game.player1)

        eff = _find_c1(perm)
        ctx = {
            'game': runner.game,
            'player': runner.game.player1,
            'permanent': perm,
            'event_permanent': perm,
            'added_card': added,
        }
        assert eff.can_use_condition(ctx) is False, (
            "C1 is [Your Turn] only"
        )

    def test_c1_condition_rejects_with_2_tamers_in_play(self, make_runner):
        runner = make_runner()
        perm = runner.place_on_field(1, [TERRIERMON])
        runner.place_on_field(1, [PLAIN_TAMER])
        runner.place_on_field(1, [CS_TAMER_3])
        runner.inject_card(1, CS_TAMER_4, "hand")

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        added = db.create_card_source(CS_LV3, runner.game.player1)

        eff = _find_c1(perm)
        ctx = {
            'game': runner.game,
            'player': runner.game.player1,
            'permanent': perm,
            'event_permanent': perm,
            'added_card': added,
        }
        assert eff.can_use_condition(ctx) is False, (
            "C1 is gated on 'you have 1 or fewer Tamers'"
        )

    def test_c1_condition_ok_with_1_tamer_in_play(self, make_runner):
        runner = make_runner()
        perm = runner.place_on_field(1, [TERRIERMON])
        runner.place_on_field(1, [CS_TAMER_3])
        runner.inject_card(1, CS_TAMER_4, "hand")

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        added = db.create_card_source(CS_LV3, runner.game.player1)

        eff = _find_c1(perm)
        ctx = {
            'game': runner.game,
            'player': runner.game.player1,
            'permanent': perm,
            'event_permanent': perm,
            'added_card': added,
        }
        assert eff.can_use_condition(ctx) is True

    def test_c1_condition_requires_cs_tamer_in_hand(self, make_runner):
        """If hand has only non-CS Tamers, condition should fail (no valid play)."""
        runner = make_runner()
        perm = runner.place_on_field(1, [TERRIERMON])
        runner.inject_card(1, PLAIN_TAMER, "hand")  # NOT CS

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        added = db.create_card_source(CS_LV3, runner.game.player1)

        eff = _find_c1(perm)
        ctx = {
            'game': runner.game,
            'player': runner.game.player1,
            'permanent': perm,
            'event_permanent': perm,
            'added_card': added,
        }
        assert eff.can_use_condition(ctx) is False, (
            "C1 should fail if no CS Tamer is in hand to play"
        )

    def test_c1_condition_fails_with_non_digimon_added_card(self, make_runner):
        runner = make_runner()
        perm = runner.place_on_field(1, [TERRIERMON])
        runner.inject_card(1, CS_TAMER_3, "hand")

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        added = db.create_card_source(CS_TAMER_3, runner.game.player1)  # Tamer, not Digimon
        assert added.is_tamer and not added.is_digimon

        eff = _find_c1(perm)
        ctx = {
            'game': runner.game,
            'player': runner.game.player1,
            'permanent': perm,
            'event_permanent': perm,
            'added_card': added,
        }
        assert eff.can_use_condition(ctx) is False, (
            "C1 requires the ADDED card to be a Digimon"
        )

    # ── Process ────────────────────────────────────────────────────

    def test_c1_process_plays_cs_tamer_free(self, make_runner):
        """Process should play the CS Tamer from hand without paying cost."""
        runner = make_runner()
        perm = runner.place_on_field(1, [TERRIERMON])
        runner.inject_card(1, CS_TAMER_3, "hand")

        game = runner.game
        game.player1.memory = 0  # no memory — must be free
        memory_before = game.player1.memory
        eff = _find_c1(perm)

        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'event_permanent': perm,
        }
        eff.on_process_callback(ctx)
        runner.auto_resolve(max_steps=10)

        # CS Tamer should be on field
        field_ids = [p.top_card.card_id for p in game.player1.battle_area
                     if p.top_card]
        assert CS_TAMER_3 in field_ids, (
            f"CS Tamer should have been played to field. Got {field_ids}"
        )
        # Memory should not have been spent (free play)
        assert game.player1.memory == memory_before, (
            f"Memory should be unchanged (free play). Before={memory_before}, "
            f"after={game.player1.memory}"
        )

    def test_c1_process_ignored_when_tamer_count_exceeds_one(self, make_runner):
        """Even if process is invoked with too many Tamers, it should not play."""
        runner = make_runner()
        perm = runner.place_on_field(1, [TERRIERMON])
        runner.place_on_field(1, [PLAIN_TAMER])
        runner.place_on_field(1, [PLAIN_TAMER])
        runner.inject_card(1, CS_TAMER_3, "hand")

        game = runner.game
        eff = _find_c1(perm)

        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'event_permanent': perm,
        }
        eff.on_process_callback(ctx)
        runner.auto_resolve(max_steps=5)

        field_ids = [p.top_card.card_id for p in game.player1.battle_area
                     if p.top_card]
        assert CS_TAMER_3 not in field_ids, (
            "CS Tamer must NOT be played when Tamers > 1"
        )

    def test_c1_process_does_not_play_non_cs_tamer(self, make_runner):
        runner = make_runner()
        perm = runner.place_on_field(1, [TERRIERMON])
        runner.inject_card(1, PLAIN_TAMER, "hand")  # non-CS Tamer in hand

        game = runner.game
        eff = _find_c1(perm)

        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': perm,
            'event_permanent': perm,
        }
        eff.on_process_callback(ctx)
        runner.auto_resolve(max_steps=5)

        field_ids = [p.top_card.card_id for p in game.player1.battle_area
                     if p.top_card]
        assert PLAIN_TAMER not in field_ids, (
            "Non-CS Tamer must not be selectable for C1"
        )


@pytest.mark.behavioral
class TestBT22043C2_RestackDraw:
    """C2 — Inherited [Main][OPT] by-cost re-stack; <Draw 1>."""

    # ── Effect shape ────────────────────────────────────────────────

    def test_c2_effect_shape(self, make_runner):
        runner = make_runner()
        # Place Terriermon as an under-card so the inherited effect is visible
        perm = runner.place_on_field(1, [TERRIERMON, CS_LV4])
        eff = _find_c2(perm)
        assert eff is not None, "Should find C2 (inherited) effect"
        assert eff.is_inherited_effect, "C2 must be an inherited effect"
        assert getattr(eff, '_is_field_main', False), (
            "C2 is [Main] activatable from field — must set _is_field_main"
        )
        assert eff.max_count_per_turn == 1, "C2 is Once Per Turn"

    # ── Condition ──────────────────────────────────────────────────

    def test_c2_condition_fails_without_digivolution_cards(self, make_runner):
        """Lone Terriermon has no under-cards, so C2 can't move anything."""
        runner = make_runner()
        perm = runner.place_on_field(1, [TERRIERMON])
        eff = _find_c2(perm)
        assert eff is not None
        ctx = {'game': runner.game, 'player': runner.game.player1, 'permanent': perm}
        assert eff.can_use_condition(ctx) is False, (
            "Single-card stack cannot pay the re-stack cost"
        )

    def test_c2_condition_fails_when_top_is_not_cs(self, make_runner):
        """If host permanent's top isn't a CS Digimon, condition fails."""
        runner = make_runner()
        perm = runner.place_on_field(1, [TERRIERMON, PLAIN_DIGIMON])
        eff = _find_c2(perm)
        assert eff is not None
        assert 'CS' not in (perm.top_card.card_traits or [])
        ctx = {'game': runner.game, 'player': runner.game.player1, 'permanent': perm}
        assert eff.can_use_condition(ctx) is False, (
            "C2 requires the top card to have [CS] trait"
        )

    def test_c2_condition_ok_when_top_is_cs_digimon_with_sources(self, make_runner):
        runner = make_runner()
        perm = runner.place_on_field(1, [TERRIERMON, CS_LV4])
        eff = _find_c2(perm)
        assert eff is not None
        assert 'CS' in (perm.top_card.card_traits or [])
        ctx = {'game': runner.game, 'player': runner.game.player1, 'permanent': perm}
        assert eff.can_use_condition(ctx) is True

    # ── Process: cost + draw ────────────────────────────────────────

    def test_c2_process_moves_top_to_bottom(self, make_runner):
        """The cost must move the current TOP card (card_sources[-1]) to index 0."""
        runner = make_runner()
        perm = runner.place_on_field(1, [TERRIERMON, CS_LV4])
        assert len(perm.card_sources) == 2
        original_top = perm.card_sources[-1]  # CS_LV4
        original_under = perm.card_sources[0]  # TERRIERMON

        eff = _find_c2(perm)
        ctx = {
            'game': runner.game,
            'player': runner.game.player1,
            'permanent': perm,
        }
        eff.on_process_callback(ctx)

        # After pay-cost: old top is now at the bottom (index 0);
        # old under is now the new top (index -1).
        assert perm.card_sources[0] is original_top, (
            f"Old top card should be at the bottom after cost; "
            f"got {perm.card_sources[0].card_id}"
        )
        assert perm.card_sources[-1] is original_under, (
            f"Old under-card should be the new top; "
            f"got {perm.card_sources[-1].card_id}"
        )
        assert len(perm.card_sources) == 2, "Stack length unchanged"

    def test_c2_process_draws_1(self, make_runner):
        runner = make_runner()
        perm = runner.place_on_field(1, [TERRIERMON, CS_LV4])

        game = runner.game
        hand_before = len(game.player1.hand_cards)
        eff = _find_c2(perm)

        eff.on_process_callback({
            'game': game, 'player': game.player1, 'permanent': perm,
        })

        hand_after = len(game.player1.hand_cards)
        assert hand_after == hand_before + 1, (
            f"Should draw exactly 1. Before={hand_before}, after={hand_after}"
        )

    def test_c2_process_noop_if_no_sources(self, make_runner):
        """Guard against running cost with only 1 card in stack."""
        runner = make_runner()
        perm = runner.place_on_field(1, [TERRIERMON])  # non-CS top
        # Artificially run the callback — the guard should prevent anything.
        game = runner.game
        eff = _find_c2(perm)

        hand_before = len(game.player1.hand_cards)
        eff.on_process_callback({
            'game': game, 'player': game.player1, 'permanent': perm,
        })
        assert len(perm.card_sources) == 1, "Stack must not change"
        assert len(game.player1.hand_cards) == hand_before, (
            "Draw should not happen if cost cannot be paid"
        )

    def test_c2_opt_blocks_second_activation_same_turn(self, make_runner):
        runner = make_runner()
        perm = runner.place_on_field(1, [TERRIERMON, CS_LV4, CS_LV5])
        eff = _find_c2(perm)
        assert eff.can_activate_this_turn()
        eff.record_activation()
        assert not eff.can_activate_this_turn(), (
            "OPT must block second activation in same turn"
        )

    def test_c2_uses_context_permanent_not_effect_source_permanent(
        self, make_runner
    ):
        """C2 condition must read permanent from ctx, since field-main
        activation does not set effect_source_permanent."""
        runner = make_runner()
        # Effect lives on Terriermon under CS_LV4
        perm = runner.place_on_field(1, [TERRIERMON, CS_LV4])
        eff = _find_c2(perm)
        # Ensure effect_source_permanent is NOT set (simulates no prior collect)
        eff.effect_source_permanent = None
        ctx = {'game': runner.game, 'player': runner.game.player1, 'permanent': perm}
        assert eff.can_use_condition(ctx) is True, (
            "Condition must use context.permanent, not effect_source_permanent"
        )
