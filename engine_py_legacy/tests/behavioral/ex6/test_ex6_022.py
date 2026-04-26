"""Behavioral tests for EX6-022 Angewomon (Lv.5 Yellow Archangel, DP 6000, Cost 7).

Card text:
<Barrier>
[On Play] [When Digivolving] If you have a [Mirei Mikagura], 1 of your opponent's
    Digimon gains <Security A. -2> until the end of their turn. If you don't
    have a [Mirei Mikagura], you may play 1 [Mirei Mikagura] from your hand
    without paying the cost.

Inherited Effect:
[All Turns] While this Digimon has the [Angel] or [Three Great Angels] trait,
    it gains <Alliance>.
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming


@pytest.mark.behavioral
class TestEX6022Angewomon:
    """Tests for EX6-022 Angewomon."""

    # ── Barrier ──────────────────────────────────────────────────────

    def test_barrier_keyword_present(self, debug_runner):
        """<Barrier> keyword should be present on the permanent via factory effect."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["EX6-022"])

        # Check keyword via has_keyword
        assert perm.has_keyword('_is_barrier'), \
            "EX6-022 should have <Barrier> keyword on the permanent"

    def test_barrier_factory_effect_exists(self, debug_runner):
        """A factory effect with _is_barrier = True should exist on the card."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["EX6-022"])
        card = perm.top_card
        effects = card.effect_list(None)

        barrier_effects = [e for e in effects if getattr(e, '_is_barrier', False)]
        assert len(barrier_effects) >= 1, \
            "Should have a factory effect with _is_barrier = True"

    # ── On Play / When Digivolving Main Effect ────────────────────────

    def test_on_play_effect_registered(self, debug_runner):
        """[On Play] effect with OnEnterFieldAnyone timing and is_on_play should exist."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["EX6-022"])
        card = perm.top_card
        effects = card.effect_list(None)

        on_play = [e for e in effects
                   if e.timing == EffectTiming.OnEnterFieldAnyone
                   and e.is_on_play]
        assert len(on_play) == 1, \
            f"Should have exactly 1 [On Play] effect, got {len(on_play)}"

    def test_when_digivolving_effect_registered(self, debug_runner):
        """[When Digivolving] effect with OnEnterFieldAnyone timing and is_when_digivolving should exist."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["EX6-022"])
        card = perm.top_card
        effects = card.effect_list(None)

        wd = [e for e in effects
              if e.timing == EffectTiming.OnEnterFieldAnyone
              and e.is_when_digivolving]
        assert len(wd) == 1, \
            f"Should have exactly 1 [When Digivolving] effect, got {len(wd)}"

    def test_on_play_and_when_digivolving_are_separate_instances(self, debug_runner):
        """[On Play] and [When Digivolving] must be separate ICardEffect instances."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["EX6-022"])
        card = perm.top_card
        effects = card.effect_list(None)

        on_play = [e for e in effects
                   if e.timing == EffectTiming.OnEnterFieldAnyone
                   and e.is_on_play]
        wd = [e for e in effects
              if e.timing == EffectTiming.OnEnterFieldAnyone
              and e.is_when_digivolving]
        assert len(on_play) == 1 and len(wd) == 1
        assert on_play[0] is not wd[0], \
            "[On Play] and [When Digivolving] must be separate ICardEffect instances"

    # ── Main Effect: SA -2 branch (with Mirei on field) ──────────────

    def test_sa_minus_2_with_mirei_registers_modifier(self, debug_runner):
        """With Mirei Mikagura on field, process selects opponent Digimon and applies SA-2."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        # P1: Place Angewomon + Mirei Mikagura tamer
        perm_ange = runner.place_on_field(1, ["EX6-022"])
        runner.place_on_field(1, ["BT22-089"])  # Mirei Mikagura

        # P2: Place a Digimon as target
        target = runner.place_on_field(2, ["ST1-03"])
        sa_before = target.security_attack_modifier()

        game = runner.game
        player = game.player1
        card = perm_ange.top_card

        effects = card.effect_list(None)
        on_play = [e for e in effects
                   if e.timing == EffectTiming.OnEnterFieldAnyone
                   and e.is_on_play][0]

        # Capture selection + simulate user picking the first valid target
        selected = []

        def mock_select(player, callback, filter_fn=None, is_optional=False, prompt=""):
            candidates = [p for p in player.enemy.battle_area
                          if (filter_fn is None or filter_fn(p))]
            selected.append(('called', is_optional, len(candidates)))
            if candidates:
                callback(candidates[0])

        game.effect_select_opponent_permanent = mock_select

        on_play.on_process_callback({
            'player': player,
            'game': game,
            'permanent': perm_ange,
        })

        assert len(selected) == 1, "Should request opponent selection exactly once"
        _, is_optional, count = selected[0]
        assert is_optional is False, "Selection should be mandatory when a Mirei is on field"
        assert count >= 1, "Should have at least one valid target"

        # Verify SA-2 was applied to the target Digimon
        sa_after = target.security_attack_modifier()
        assert sa_after - sa_before == -2, (
            f"Target SA modifier should be reduced by 2. "
            f"Before={sa_before}, After={sa_after}")

    def test_sa_modifier_expires_end_of_opponent_turn(self, debug_runner):
        """SA-2 modifier should clear on clear_expiry('end_of_opponent_turn')."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        perm_ange = runner.place_on_field(1, ["EX6-022"])
        runner.place_on_field(1, ["BT22-089"])  # Mirei Mikagura
        target = runner.place_on_field(2, ["ST1-03"])
        sa_before = target.security_attack_modifier()

        game = runner.game
        player = game.player1
        card = perm_ange.top_card

        effects = card.effect_list(None)
        on_play = [e for e in effects
                   if e.timing == EffectTiming.OnEnterFieldAnyone
                   and e.is_on_play][0]

        def auto_select(player, callback, filter_fn=None, is_optional=False, prompt=""):
            candidates = [p for p in player.enemy.battle_area
                          if (filter_fn is None or filter_fn(p))]
            if candidates:
                callback(candidates[0])
        game.effect_select_opponent_permanent = auto_select

        on_play.on_process_callback({
            'player': player,
            'game': game,
            'permanent': perm_ange,
        })

        # After process, SA should be -2
        assert target.security_attack_modifier() - sa_before == -2

        # Clear end_of_opponent_turn expiry — SA should reset
        game.modifiers.clear_expiry('end_of_opponent_turn')
        assert target.security_attack_modifier() == sa_before, (
            "SA modifier should expire on 'end_of_opponent_turn' clear")

    def test_sa_modifier_target_filter_is_digimon(self, debug_runner):
        """Selection filter for SA-2 should only match Digimon (not tamers/options/eggs)."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        perm_ange = runner.place_on_field(1, ["EX6-022"])
        runner.place_on_field(1, ["BT22-089"])  # Mirei

        # Opponent field: a Digimon + a Tamer
        target_digi = runner.place_on_field(2, ["ST1-03"])
        target_tamer = runner.place_on_field(2, ["BT22-089"])

        game = runner.game
        player = game.player1
        card = perm_ange.top_card

        effects = card.effect_list(None)
        on_play = [e for e in effects
                   if e.timing == EffectTiming.OnEnterFieldAnyone
                   and e.is_on_play][0]

        captured_filters = []

        def mock_select(player, callback, filter_fn=None, is_optional=False, prompt=""):
            captured_filters.append(filter_fn)
        game.effect_select_opponent_permanent = mock_select

        on_play.on_process_callback({
            'player': player,
            'game': game,
            'permanent': perm_ange,
        })

        assert len(captured_filters) == 1
        f = captured_filters[0]
        assert f is not None, "Must pass a filter_fn"
        assert f(target_digi) is True, "Digimon target should pass filter"
        assert f(target_tamer) is False, "Tamer should NOT pass Digimon filter"

    # ── Main Effect: Play Mirei branch (without Mirei on field) ──────

    def test_play_mirei_when_no_mirei_on_field(self, debug_runner):
        """Without Mirei on field, effect should call effect_play_from_zone for 'hand'."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        perm_ange = runner.place_on_field(1, ["EX6-022"])
        # No Mirei on field, but put one in hand
        runner.inject_card(1, "BT22-089", "hand")

        game = runner.game
        player = game.player1
        card = perm_ange.top_card

        effects = card.effect_list(None)
        on_play = [e for e in effects
                   if e.timing == EffectTiming.OnEnterFieldAnyone
                   and e.is_on_play][0]

        calls = []

        def mock_play_from_zone(player, zone, filter_fn, free=True,
                                manual_reduction=0, is_optional=True,
                                prompt="", on_played=None):
            calls.append({
                'zone': zone,
                'free': free,
                'is_optional': is_optional,
                'filter_fn': filter_fn,
            })
        game.effect_play_from_zone = mock_play_from_zone

        on_play.on_process_callback({
            'player': player,
            'game': game,
            'permanent': perm_ange,
        })

        assert len(calls) == 1, \
            "Should call effect_play_from_zone exactly once when no Mirei on field"
        call = calls[0]
        assert call['zone'] == 'hand', "Zone must be 'hand' per card text"
        assert call['free'] is True, "Must play without paying cost (free=True)"
        assert call['is_optional'] is True, \
            "Must be optional ('you may play') — C# canNoSelect=true"

        # Filter should match Mirei Mikagura
        mirei_hand = [c for c in player.hand_cards
                      if c.c_entity_base and c.c_entity_base.card_id == "BT22-089"]
        assert mirei_hand, "Mirei Mikagura should be in hand"
        assert call['filter_fn'](mirei_hand[0]) is True, \
            "Filter must accept Mirei Mikagura"

    def test_play_mirei_filter_rejects_other_tamers(self, debug_runner):
        """Play filter should reject non-Mirei Mikagura cards."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        perm_ange = runner.place_on_field(1, ["EX6-022"])
        # Inject a non-Mirei tamer AND a digimon into hand
        runner.inject_card(1, "BT22-089", "hand")  # Mirei (should pass)
        runner.inject_card(1, "ST1-03", "hand")    # Agumon (should fail)

        game = runner.game
        player = game.player1
        card = perm_ange.top_card

        effects = card.effect_list(None)
        on_play = [e for e in effects
                   if e.timing == EffectTiming.OnEnterFieldAnyone
                   and e.is_on_play][0]

        captured = []

        def mock_play_from_zone(player, zone, filter_fn, free=True,
                                manual_reduction=0, is_optional=True,
                                prompt="", on_played=None):
            captured.append(filter_fn)
        game.effect_play_from_zone = mock_play_from_zone

        on_play.on_process_callback({
            'player': player,
            'game': game,
            'permanent': perm_ange,
        })

        assert len(captured) == 1
        filter_fn = captured[0]

        mirei = None
        agumon = None
        for c in player.hand_cards:
            if c.c_entity_base and c.c_entity_base.card_id == "BT22-089":
                mirei = c
            if c.c_entity_base and c.c_entity_base.card_id == "ST1-03":
                agumon = c
        assert mirei and agumon
        assert filter_fn(mirei) is True
        assert filter_fn(agumon) is False

    def test_no_sa_modifier_when_no_mirei(self, debug_runner):
        """Without Mirei on field, SA-2 branch must NOT fire."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        perm_ange = runner.place_on_field(1, ["EX6-022"])
        runner.inject_card(1, "BT22-089", "hand")

        target = runner.place_on_field(2, ["ST1-03"])

        game = runner.game
        player = game.player1
        card = perm_ange.top_card

        effects = card.effect_list(None)
        on_play = [e for e in effects
                   if e.timing == EffectTiming.OnEnterFieldAnyone
                   and e.is_on_play][0]

        # Stub out play_from_zone so the process returns cleanly
        def noop_play_from_zone(*args, **kwargs):
            pass
        game.effect_play_from_zone = noop_play_from_zone

        # Selection should NOT be called in this branch
        select_calls = []

        def mock_select(*args, **kwargs):
            select_calls.append(1)
        game.effect_select_opponent_permanent = mock_select

        on_play.on_process_callback({
            'player': player,
            'game': game,
            'permanent': perm_ange,
        })

        assert len(select_calls) == 0, \
            "Should NOT call effect_select_opponent_permanent in no-Mirei branch"

        # Sanity: no SA modifier applied to target
        assert target.security_attack_modifier() == 0, \
            "No SA modifier should be applied in no-Mirei branch"

    def test_condition_requires_on_field(self, debug_runner):
        """can_use_condition should fail when the card is not on field."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        runner.inject_card(1, "EX6-022", "hand")

        game = runner.game
        player = game.player1
        hand_card = next(c for c in player.hand_cards
                         if c.c_entity_base and c.c_entity_base.card_id == "EX6-022")

        effects = hand_card.effect_list(None)
        on_play = [e for e in effects
                   if e.timing == EffectTiming.OnEnterFieldAnyone
                   and e.is_on_play][0]

        ctx = {'player': player, 'game': game, 'permanent': None}
        assert on_play.can_use_condition(ctx) is False, \
            "Effect condition should fail when card is not on field"

    # ── Alliance Inherited Effect ─────────────────────────────────────

    def test_alliance_inherited_effect_flags(self, debug_runner):
        """Alliance effect must be inherited, _is_alliance, with conditional activation."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["EX6-022"])
        card = perm.top_card

        # Get effects (including inherited)
        effects = card.effect_list(None)
        alliance_effects = [e for e in effects if getattr(e, '_is_alliance', False)]
        assert len(alliance_effects) == 1, \
            f"Should have exactly 1 alliance effect, got {len(alliance_effects)}"
        eff = alliance_effects[0]
        assert eff.is_inherited_effect is True, \
            "Alliance must be an inherited effect"

    def test_alliance_not_active_when_top_is_archangel_only(self, debug_runner):
        """When top card is EX6-022 itself (Archangel trait only, NOT Angel),
        Alliance should NOT be granted via has_keyword.

        Bug regression: 'Angel' in 'Archangel' substring match would incorrectly
        grant Alliance. Must use exact trait list match on TopCard only.
        """
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        # Stack: EX6-022 over BT1-053 Darcmon (Angel trait).
        # Top card is EX6-022 itself. Per card text, "this Digimon" refers
        # to the host whose top_card trait determines if Alliance is granted.
        # EX6-022's traits = ["Archangel", "Vaccine"] — NO Angel, NO Three Great Angels.
        perm = runner.place_on_field(1, ["EX6-022"])

        # Verify top card traits don't include Angel or Three Great Angels
        traits = perm.top_card.card_traits
        assert "Archangel" in traits
        assert "Angel" not in traits, \
            "EX6-022 top_card traits list should NOT contain 'Angel' (only 'Archangel')"
        assert "Three Great Angels" not in traits

        # Alliance should NOT be active because top_card has Archangel, not Angel
        assert perm.has_keyword('_is_alliance') is False, \
            "Alliance must NOT be granted when top_card trait is Archangel (not Angel)"

    def test_alliance_active_when_angel_is_on_top(self, debug_runner):
        """When an Angel-trait Digimon is the top card (e.g. digivolved on top of
        EX6-022 in stack), Alliance is granted to the host permanent."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        # Stack: EX6-022 at bottom, Darcmon (Angel, Lv.4) on top
        # Since we can't legally digivolve Darcmon (Champion) onto EX6-022 (Mega)
        # via rules, place_on_field allows us to inject any stack order for testing.
        perm = runner.place_on_field(1, ["EX6-022", "BT1-053"])

        # Top is Darcmon
        assert perm.top_card.c_entity_base.card_id == "BT1-053"
        top_traits = perm.top_card.card_traits
        assert "Angel" in top_traits, \
            "Darcmon (BT1-053) should have 'Angel' in card_traits"

        # Alliance should now be active (inherited from EX6-022 below)
        assert perm.has_keyword('_is_alliance') is True, \
            "Alliance must be granted when top_card has Angel trait"

    def test_alliance_active_when_three_great_angels_on_top(self, debug_runner):
        """Alliance is also granted when top card has Three Great Angels trait."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        # Stack: EX6-022 at bottom, Seraphimon (Three Great Angels, Lv.6) on top
        perm = runner.place_on_field(1, ["EX6-022", "BT1-063"])

        assert perm.top_card.c_entity_base.card_id == "BT1-063"
        top_traits = perm.top_card.card_traits
        assert "Three Great Angels" in top_traits, \
            "Seraphimon (BT1-063) should have 'Three Great Angels' trait"

        assert perm.has_keyword('_is_alliance') is True, \
            "Alliance must be granted when top_card has 'Three Great Angels' trait"

    def test_alliance_not_active_when_below_leaves_field(self, debug_runner):
        """Standalone EX6-022 on field (no stack above) — Alliance not active
        because EX6-022's own top_card has Archangel, not Angel.

        Also validates that the inherited effect is not checked when card is
        not on any permanent.
        """
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        perm = runner.place_on_field(1, ["EX6-022"])

        # EX6-022 alone — top_card = EX6-022 = Archangel, NOT Angel
        assert perm.has_keyword('_is_alliance') is False

    def test_alliance_not_active_when_stacked_below_non_angel(self, debug_runner):
        """If a non-Angel Digimon is on top, Alliance is NOT granted."""
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        # Stack: EX6-022 at bottom, Agumon (Reptile trait, no Angel) on top
        perm = runner.place_on_field(1, ["EX6-022", "ST1-03"])

        top_traits = perm.top_card.card_traits
        assert "Angel" not in top_traits
        assert "Three Great Angels" not in top_traits

        assert perm.has_keyword('_is_alliance') is False, \
            "Alliance must NOT be granted when top_card has no Angel/Three Great Angels"

    def test_alliance_checks_only_top_card_not_stack(self, debug_runner):
        """Alliance condition must only check the TOP card's traits, not all stack sources.

        C# reference: `card.PermanentOfThisCard().TopCard.CardTraits.Contains("Angel")`
        — it checks TopCard only, not the entire stack.

        Regression test for a bug where the condition scanned all card_sources
        and would incorrectly grant Alliance if any stack card had the trait,
        even when the top card did not.
        """
        runner = debug_runner(initial_memory=5)
        runner.set_phase("Main")

        # Stack: EX6-022 (bottom) + Darcmon/Angel (middle) + Agumon/Reptile (top)
        # "This Digimon" refers to the top card (Agumon / Reptile).
        # C# checks TopCard.CardTraits only — Agumon does NOT have Angel, so
        # Alliance must NOT be granted.
        perm = runner.place_on_field(1, ["EX6-022", "BT1-053", "ST1-03"])

        # Verify top is Agumon with Reptile trait
        assert perm.top_card.c_entity_base.card_id == "ST1-03"
        top_traits = perm.top_card.card_traits
        assert "Angel" not in top_traits
        assert "Three Great Angels" not in top_traits

        # Darcmon (Angel) IS present in the stack — but at index 1, not on top.
        darcmon_traits = perm.card_sources[1].card_traits
        assert "Angel" in darcmon_traits, \
            "Sanity: Darcmon in stack should have Angel trait"

        # Alliance MUST NOT be granted (top card isn't Angel — C# checks top only)
        assert perm.has_keyword('_is_alliance') is False, (
            "Alliance must only consider TopCard traits. "
            "Bug: scanning all card_sources granted Alliance when a below-top "
            "card had Angel trait.")
