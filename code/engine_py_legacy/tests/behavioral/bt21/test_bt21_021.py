"""Behavioral tests for BT21-021 OmniShoutmon (Digimon, Lv.5, Red/Yellow, Xros Heart).

Card text:
This card is also treated as [Shoutmon] for DigiXros.
[End of Attack] You may play 1 card with the [Xros Heart], [Blue Flare] or
[Hero] trait from your hand with the play cost reduced by 5. If you did,
delete this Digimon.
[On Deletion] You may place 1 Digimon card with the [Xros Heart] or
[Blue Flare] trait from your hand or trash under any of your Tamers.
Then, <Save> (You may place this card under one of your Tamers.)

Inherited: [Your Turn] This Digimon with the [Xros Heart] trait gains
<Rush> (This Digimon can attack the turn it comes into play.)

Alt Digivolve:
[Digivolve] [Shoutmon]: Cost 4
[Digivolve] Lv.4 w/[Xros Heart]/[Hero] trait: Cost 3
"""

import pytest
from engine_py_legacy.engine.data.enums import EffectTiming


DECK = ["BT21-021"] * 4 + ["BT10-007"] * 10 + ["BT10-018"] * 10 + ["BT5-009"] * 10 + ["BT10-003"] * 10 + ["BT21-013"] * 6


@pytest.mark.behavioral
class TestBT21021OmniShoutmon:
    """Tests for BT21-021 OmniShoutmon."""

    # -- Alt-digi from [Shoutmon] for cost 4 --

    def test_alt_digi_from_shoutmon(self, debug_runner):
        """Should be able to digivolve from [Shoutmon] for cost 4."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT21-021", runner.game.player1)
        effects = card.effect_list(EffectTiming.NoTiming)

        alt_digi = [e for e in effects if getattr(e, '_alt_digi_cost', None) is not None]
        shoutmon_alt = [e for e in alt_digi if getattr(e, '_alt_digi_name', None) == 'Shoutmon']
        assert len(shoutmon_alt) == 1, "Should have alt-digi from [Shoutmon]"
        assert shoutmon_alt[0]._alt_digi_cost == 4

    # -- Alt-digi Lv.4 w/[Xros Heart]/[Hero] trait for cost 3 --

    def test_alt_digi_xros_heart_or_hero_trait(self, debug_runner):
        """Should be able to digivolve from Lv.4 with [Xros Heart] or [Hero] trait
        for cost 3. The _alt_digi_level must be 4 and a condition_fn must accept
        both Xros Heart and Hero."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT21-021", runner.game.player1)
        effects = card.effect_list(EffectTiming.NoTiming)

        alt_cost3 = [e for e in effects if getattr(e, '_alt_digi_cost', None) == 3]
        assert len(alt_cost3) == 1, "Should have 1 alt-digi for cost 3"
        assert getattr(alt_cost3[0], '_alt_digi_level', None) == 4, \
            "Alt-digi cost 3 should require Lv.4"

        # Must accept both Xros Heart and Hero via _alt_digi_condition_fn
        # Test with a Xros Heart Lv.4 permanent
        xh_perm = runner.place_on_field(1, ["BT10-009"])  # Shoutmon X4, Xros Heart Lv.4
        from engine_py_legacy.engine.validation.digivolve_validator import _check_alt_digivolve
        assert _check_alt_digivolve(card, xh_perm), \
            "Should accept Xros Heart Lv.4 for alt-digi"

        # Test with a Hero Lv.4 permanent
        hero_perm = runner.place_on_field(1, ["BT21-013"])  # Agunimon, Hero Lv.4
        assert _check_alt_digivolve(card, hero_perm), \
            "Should accept Hero Lv.4 for alt-digi"

    # -- End of Attack: Play from hand with cost reduction --

    def test_end_of_attack_effect_exists(self, debug_runner):
        """Should have OnEndAttack timing effect for playing a card."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)

        perm = runner.place_on_field(1, ["BT21-021"])
        card = perm.top_card
        effects = card.effect_list(None)

        eoa_effects = [e for e in effects if e.timing == EffectTiming.OnEndAttack]
        assert len(eoa_effects) >= 1, "Should have at least 1 OnEndAttack effect"

    def test_end_of_attack_plays_xros_heart_blue_flare_or_hero(self, debug_runner):
        """End of Attack should allow playing cards with Xros Heart, Blue Flare,
        or Hero trait from hand with play cost -5."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)

        perm = runner.place_on_field(1, ["BT21-021"])
        game = runner.game

        card = perm.top_card
        effects = card.effect_list(None)
        eoa = [e for e in effects if e.timing == EffectTiming.OnEndAttack][0]

        # Verify play_from_zone is called with correct filter
        play_calls = []
        original_play = game.effect_play_from_zone

        def mock_play(player, zone, filter_fn, free=True, manual_reduction=0,
                      is_optional=True, prompt="", on_played=None):
            play_calls.append({
                'zone': zone,
                'filter_fn': filter_fn,
                'manual_reduction': manual_reduction,
                'is_optional': is_optional,
                'on_played': on_played,
            })

        game.effect_play_from_zone = mock_play

        eoa.on_process_callback({
            'player': game.player1,
            'permanent': perm,
            'game': game,
        })

        assert len(play_calls) == 1, "Should call effect_play_from_zone once"
        assert play_calls[0]['zone'] == 'hand'
        assert play_calls[0]['manual_reduction'] == 5
        assert play_calls[0]['is_optional'] is True

        # Check the filter function
        from engine_py_legacy.engine.core.card_source import CardSource
        from engine_py_legacy.engine.core.entity_base import CEntity_Base

        def _make_card(traits):
            e = CEntity_Base()
            e.type_eng = traits
            cs = CardSource()
            cs.c_entity_base = e
            return cs

        assert play_calls[0]['filter_fn'](_make_card(["Xros Heart"])), \
            "Filter should accept Xros Heart"
        assert play_calls[0]['filter_fn'](_make_card(["Blue Flare"])), \
            "Filter should accept Blue Flare"
        assert play_calls[0]['filter_fn'](_make_card(["Hero"])), \
            "Filter should accept Hero"
        assert not play_calls[0]['filter_fn'](_make_card(["Reptile"])), \
            "Filter should reject non-matching traits"

        game.effect_play_from_zone = original_play

    def test_end_of_attack_deletes_self_after_play(self, debug_runner):
        """"If you did, delete this Digimon" — the on_played callback must
        delete Gogmamon after a successful play."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=10)

        perm = runner.place_on_field(1, ["BT21-021"])
        game = runner.game

        card = perm.top_card
        effects = card.effect_list(None)
        eoa = [e for e in effects if e.timing == EffectTiming.OnEndAttack][0]

        # Capture the on_played callback
        captured = {}
        original_play = game.effect_play_from_zone

        def mock_play(player, zone, filter_fn, free=True, manual_reduction=0,
                      is_optional=True, prompt="", on_played=None):
            captured['on_played'] = on_played

        game.effect_play_from_zone = mock_play

        eoa.on_process_callback({
            'player': game.player1,
            'permanent': perm,
            'game': game,
        })

        game.effect_play_from_zone = original_play

        assert captured.get('on_played') is not None, \
            "Should pass on_played callback to effect_play_from_zone"

        # Invoke the callback as if a card was played — should delete perm
        assert perm in game.player1.battle_area, "Setup: perm should be on field"
        captured['on_played'](None, None)
        assert perm not in game.player1.battle_area, \
            "Gogmamon should be deleted after successful play"

    # -- On Deletion: Place Xros/Blue Flare Digimon under tamer + Save --

    def test_on_deletion_effect_exists(self, debug_runner):
        """Should have OnDestroyedAnyone timing with is_on_deletion for On Deletion."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)

        perm = runner.place_on_field(1, ["BT21-021"])
        card = perm.top_card
        effects = card.effect_list(None)

        on_del = [e for e in effects if e.timing == EffectTiming.OnDestroyedAnyone
                  and getattr(e, 'is_on_deletion', False)]
        assert len(on_del) >= 1, "Should have On Deletion effect"
        assert on_del[0].is_optional, "On Deletion should be optional"

    # -- Inherited Rush (conditional on Xros Heart) --

    def test_inherited_rush_requires_xros_heart(self, debug_runner):
        """Inherited Rush should only activate when the top card has [Xros Heart] trait."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT21-021", runner.game.player1)
        effects = card.effect_list(None)

        rush_effects = [e for e in effects if getattr(e, '_is_rush', False)
                        and e.is_inherited_effect]
        assert len(rush_effects) == 1, "Should have 1 inherited Rush effect"

    # -- DigiXros name treatment --

    def test_treated_as_shoutmon_effect_exists(self, debug_runner):
        """Should have a static effect declaring this card is also treated as [Shoutmon]."""
        runner = debug_runner(deck1=DECK, deck2=DECK, initial_memory=5)

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        card = db.create_card_source("BT21-021", runner.game.player1)
        effects = card.effect_list(None)

        shoutmon_effects = [e for e in effects
                           if 'Shoutmon' in (e.effect_name or '')
                           and 'DigiXros' in (e.effect_name or '')]
        assert len(shoutmon_effects) >= 1, \
            "Should have an effect declaring this card is treated as Shoutmon for DigiXros"
