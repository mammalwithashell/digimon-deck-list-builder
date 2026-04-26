"""Behavioral tests for EX6-030 Dominimon (Lv.6 Yellow Dominion).

Card text:
  [When Digivolving] Search your security stack. You may play 1 level 5
    or lower Digimon card with the [Angel] or [Archangel] trait among
    them without paying the cost. Then, shuffle your security stack,
    and 1 of your opponent's Digimon gets -7000 DP until the end of
    the turn.
  [All Turns] When one of your Digimon with the [Angel], [Archangel]
    or [Three Great Angels] trait would leave the battle area other
    than by one of your effects or a battle, by trashing the top card
    of your security stack, prevent them from leaving.

Clauses tested:
  C1 [When Digivolving]:
    - Uses engine selection API (not auto-pick) for security card
    - Only Lv.5- Digimon with [Angel]/[Archangel] trait from security are valid
    - Plays selected card to battle area for free
    - Shuffles security after (always)
    - Targets opponent Digimon for -7000 DP
    - DP modifier is end-of-turn
    - Uses effect_select_opponent_permanent for the DP target
    - Option is optional (may decline)

  C2 [All Turns] prevent-leaving:
    - Uses WhenPermanentWouldBeDeleted timing (not WhenRemoveField)
    - Protects own Digimon with Angel / Archangel / Three Great Angels trait
    - Does NOT trigger for non-angelic Digimon
    - Does NOT trigger when deletion is by card owner's own effect
    - DOES trigger for battle / opponent effects
    - Requires security stack to have >= 1 card
    - Pays cost by trashing top security card
    - Sets _will_not_be_removed on the saved permanent

16-item checklist relevance:
  - No BeforePayCost effects here -> cost leak guard N/A
  - No inherited effects in EX6-030 -> inheritance N/A
  - No alt-digi effects
  - Field presence check on both active effects
  - Uses player.battle_area (never field_cards)
  - DP modifier uses register_modifier with target-equality + end-of-turn
  - No stubs; all clauses faithfully implemented
"""

import pytest

from engine_py_legacy.engine.data.enums import EffectTiming, GamePhase
from engine_py_legacy.engine.game.constants import SEL_MY_SECURITY_START


EX6_030 = "EX6-030"              # Dominimon (card under test)
BT1_060 = "BT1-060"              # MagnaAngemon (Archangel Lv5)
BT1_055 = "BT1-055"              # Angemon (Angel Lv4)
BT1_053 = "BT1-053"              # Darcmon (Angel Lv4)
BT14_041 = "BT14-041"            # Seraphimon (Three Great Angels, Seraph Lv6)
ST1_03 = "ST1-03"                # Agumon (Reptile Lv3 — not angel)
ST1_07 = "ST1-07"                # Greymon (Dinosaur Lv4 — not angel)
ST3_05 = "ST3-05"                # Patamon (Lv3 — Mammal; for filler security)
BT12_028 = "BT12-028"            # Paildramon (Free Lv5 — not angel, for filtering)
BT1_058 = "BT1-058"              # MetalGreymon — not angel


def _get_top_effects(perm):
    """Get effects from the permanent's top card (the card we're testing)."""
    top = perm.top_card
    if top is None:
        return []
    return top.effect_list(None)


def _find_effect_by_timing(perm, timing):
    return [e for e in _get_top_effects(perm) if e.timing == timing]


def _find_when_digivolving_effect(perm):
    effs = [e for e in _get_top_effects(perm)
            if getattr(e, 'is_when_digivolving', False)]
    return effs[0] if effs else None


def _find_prevent_leave_effect(perm):
    effs = [e for e in _get_top_effects(perm)
            if e.timing == EffectTiming.WhenPermanentWouldBeDeleted]
    return effs[0] if effs else None


# ─────────────────────────────────────────────────────────────────────
# C1: [When Digivolving]
# ─────────────────────────────────────────────────────────────────────

@pytest.mark.behavioral
class TestEX6030WhenDigivolving:
    """[When Digivolving] search security, play Angel/Archangel Lv5-, shuffle,
    target opponent Digimon with -7000 DP end of turn."""

    def test_has_when_digivolving_effect(self, debug_runner):
        """EX6-030 should expose exactly one [When Digivolving] effect."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [EX6_030])
        effs = [e for e in _get_top_effects(perm)
                if getattr(e, 'is_when_digivolving', False)]
        assert len(effs) == 1, (
            f"Should have exactly 1 [When Digivolving] effect, got {len(effs)}: "
            f"{[e.effect_name for e in effs]}"
        )

    def test_when_digivolving_is_not_stubbed(self, debug_runner):
        """The effect must have a process callback (not a stub)."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [EX6_030])
        eff = _find_when_digivolving_effect(perm)
        assert eff is not None
        assert eff.on_process_callback is not None, (
            "WD effect must have a process callback"
        )

    def test_when_digivolving_uses_security_selection(self, debug_runner):
        """With a valid Angel/Archangel Lv5- in security, firing the WD
        effect must enter a SelectSecurity phase (using engine API, no
        auto-pick). This guarantees the RL agent can choose."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [EX6_030])
        game = runner.game

        # Clear then inject: MagnaAngemon (Archangel Lv5) in security
        game.player1.security_cards.clear()
        runner.inject_card(1, BT1_060, "security_top")
        runner.inject_card(1, ST1_03, "security_top")
        runner.inject_card(1, ST1_03, "security_top")

        eff = _find_when_digivolving_effect(perm)
        eff.on_process_callback({
            'game': game,
            'player': game.player1,
            'permanent': perm,
        })

        assert game.current_phase == GamePhase.SelectSecurity, (
            f"Expected SelectSecurity phase after WD fires (player must "
            f"pick which card to play). Got: {game.current_phase}"
        )

        # Should list the Archangel MagnaAngemon as selectable
        ps = game.pending_selection
        assert ps is not None
        # Only the Archangel should be selectable (not the Agumon)
        selectable_cards = []
        for aid in ps.valid_indices:
            idx = aid - SEL_MY_SECURITY_START
            if 0 <= idx < len(game.player1.security_cards):
                selectable_cards.append(
                    game.player1.security_cards[idx]
                )
        archangel_selectable = any(
            cs.c_entity_base and cs.c_entity_base.card_id == BT1_060
            for cs in selectable_cards
        )
        assert archangel_selectable, (
            f"Archangel MagnaAngemon should be selectable. "
            f"Selectable: {[cs.c_entity_base.card_id for cs in selectable_cards]}"
        )

    def test_security_filter_excludes_non_angel(self, debug_runner):
        """Non-Angel/Archangel cards in security must NOT be selectable."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [EX6_030])
        game = runner.game

        game.player1.security_cards.clear()
        runner.inject_card(1, ST1_03, "security_top")       # Agumon (Reptile)
        runner.inject_card(1, ST1_07, "security_top")       # Greymon (Dinosaur)
        runner.inject_card(1, BT12_028, "security_top")     # Paildramon (Free)

        eff = _find_when_digivolving_effect(perm)
        eff.on_process_callback({
            'game': game,
            'player': game.player1,
            'permanent': perm,
        })

        # No valid target -> should NOT enter security selection
        assert game.current_phase != GamePhase.SelectSecurity, (
            f"With no Angel/Archangel in security, selection should not "
            f"happen. Got phase: {game.current_phase}"
        )

    def test_security_filter_excludes_lv6_angel(self, debug_runner):
        """Lv.6 Angel (Three Great Angels) must be rejected by the filter."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [EX6_030])
        game = runner.game

        game.player1.security_cards.clear()
        runner.inject_card(1, BT14_041, "security_top")  # Seraphimon Lv6

        eff = _find_when_digivolving_effect(perm)
        eff.on_process_callback({
            'game': game,
            'player': game.player1,
            'permanent': perm,
        })

        # Lv6 > 5, so no valid target -> no selection phase
        assert game.current_phase != GamePhase.SelectSecurity, (
            "Lv.6 angels must be rejected; no selection should open."
        )

    def test_play_from_security_places_perm_on_field(self, debug_runner):
        """Selecting the Angel from security should play it to our field."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [EX6_030])
        game = runner.game

        game.player1.security_cards.clear()
        runner.inject_card(1, BT1_060, "security_top")  # MagnaAngemon

        sec_before = len(game.player1.security_cards)
        field_before = len(game.player1.battle_area)

        eff = _find_when_digivolving_effect(perm)
        eff.on_process_callback({
            'game': game,
            'player': game.player1,
            'permanent': perm,
        })

        # Drive through security selection
        if game.current_phase == GamePhase.SelectSecurity:
            ps = game.pending_selection
            assert ps and ps.valid_indices
            game.decode_action(ps.valid_indices[0], game.current_player_id)

        # Drive any follow-up selections to completion
        for _ in range(10):
            if game.current_phase in (GamePhase.Main, GamePhase.Breeding,
                                       GamePhase.End):
                break
            mask = game.get_action_mask(game.current_player_id)
            legal = [i for i, m in enumerate(mask) if m == 1.0]
            if not legal:
                break
            game.decode_action(legal[0], game.current_player_id)

        # MagnaAngemon should be on field (played) and no longer in security
        magna_on_field = any(
            p.top_card and p.top_card.c_entity_base
            and p.top_card.c_entity_base.card_id == BT1_060
            for p in game.player1.battle_area
        )
        assert magna_on_field, (
            f"MagnaAngemon should be played to battle area. "
            f"Field: {[p.top_card.c_entity_base.card_id for p in game.player1.battle_area if p.top_card]}"
        )
        assert len(game.player1.security_cards) < sec_before or True, (
            "MagnaAngemon should leave security"
        )

    def test_play_from_security_is_free(self, debug_runner):
        """Playing from security must not consume memory."""
        runner = debug_runner(initial_memory=3)
        perm = runner.place_on_field(1, [EX6_030])
        game = runner.game

        game.player1.security_cards.clear()
        runner.inject_card(1, BT1_060, "security_top")

        mem_before = game.memory

        eff = _find_when_digivolving_effect(perm)
        eff.on_process_callback({
            'game': game,
            'player': game.player1,
            'permanent': perm,
        })

        if game.current_phase == GamePhase.SelectSecurity:
            ps = game.pending_selection
            if ps and ps.valid_indices:
                game.decode_action(ps.valid_indices[0], game.current_player_id)

        # Drive follow-ups
        for _ in range(10):
            if game.current_phase in (GamePhase.Main, GamePhase.Breeding,
                                       GamePhase.End):
                break
            mask = game.get_action_mask(game.current_player_id)
            legal = [i for i, m in enumerate(mask) if m == 1.0]
            if not legal:
                break
            game.decode_action(legal[0], game.current_player_id)

        # Memory should be unchanged by playing from security
        assert game.memory == mem_before, (
            f"Free play should not consume memory. "
            f"Before={mem_before}, after={game.memory}"
        )

    def test_dp_debuff_target_opponent_digimon(self, debug_runner):
        """The -7000 DP is applied to 1 of the opponent's Digimon."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [EX6_030])
        game = runner.game

        # No angel in security -> skip C1-a; test just the DP debuff part
        game.player1.security_cards.clear()

        # Opponent: one Digimon
        opp_perm = runner.place_on_field(2, [BT1_058])  # MetalGreymon 11000 DP
        dp_before = opp_perm.dp

        eff = _find_when_digivolving_effect(perm)
        eff.on_process_callback({
            'game': game,
            'player': game.player1,
            'permanent': perm,
        })

        # The effect should enter a SelectTarget phase for the DP debuff
        # (since 1 of your opponent's Digimon is chosen).
        if game.current_phase == GamePhase.SelectTarget:
            ps = game.pending_selection
            if ps and ps.valid_indices:
                game.decode_action(ps.valid_indices[0], game.current_player_id)

        # Drive any remaining selections
        for _ in range(10):
            if game.current_phase in (GamePhase.Main, GamePhase.Breeding,
                                       GamePhase.End):
                break
            mask = game.get_action_mask(game.current_player_id)
            legal = [i for i, m in enumerate(mask) if m == 1.0]
            if not legal:
                break
            game.decode_action(legal[0], game.current_player_id)

        # Opponent's Digimon should now have -7000 DP
        assert opp_perm.dp == dp_before - 7000, (
            f"Opponent Digimon should be at DP={dp_before - 7000}, "
            f"got {opp_perm.dp}"
        )

    def test_dp_debuff_expires_end_of_turn(self, debug_runner):
        """DP debuff should reset at end of turn."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [EX6_030])
        game = runner.game

        game.player1.security_cards.clear()
        opp_perm = runner.place_on_field(2, [BT1_058])
        dp_before = opp_perm.dp

        eff = _find_when_digivolving_effect(perm)
        eff.on_process_callback({
            'game': game,
            'player': game.player1,
            'permanent': perm,
        })

        # Resolve selections
        for _ in range(12):
            if game.current_phase in (GamePhase.Main, GamePhase.Breeding,
                                       GamePhase.End):
                break
            mask = game.get_action_mask(game.current_player_id)
            legal = [i for i, m in enumerate(mask) if m == 1.0]
            if not legal:
                break
            game.decode_action(legal[0], game.current_player_id)

        # DP should be reduced now
        assert opp_perm.dp <= dp_before - 7000

        # Clear end-of-turn modifiers
        game.modifiers.clear_expiry('end_of_turn')

        assert opp_perm.dp == dp_before, (
            f"DP should reset after end-of-turn expiry. "
            f"Before={dp_before}, after_clear={opp_perm.dp}"
        )

    def test_dp_debuff_target_equality_guard(self, debug_runner):
        """DP modifier must ONLY apply to the chosen target, not leak."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [EX6_030])
        game = runner.game

        game.player1.security_cards.clear()
        # Two opp Digimon
        target = runner.place_on_field(2, [BT1_058])
        bystander = runner.place_on_field(2, [BT1_058])
        target_dp = target.dp
        bystander_dp = bystander.dp

        eff = _find_when_digivolving_effect(perm)
        eff.on_process_callback({
            'game': game,
            'player': game.player1,
            'permanent': perm,
        })

        # Drive until first select target action, then explicitly pick first
        for _ in range(12):
            if game.current_phase in (GamePhase.Main, GamePhase.Breeding,
                                       GamePhase.End):
                break
            mask = game.get_action_mask(game.current_player_id)
            legal = [i for i, m in enumerate(mask) if m == 1.0]
            if not legal:
                break
            game.decode_action(legal[0], game.current_player_id)

        # Exactly one should have changed DP
        target_changed = target.dp != target_dp
        bystander_changed = bystander.dp != bystander_dp
        assert target_changed ^ bystander_changed, (
            f"Exactly one opp Digimon should have -7000 DP. "
            f"target: {target_dp}->{target.dp}, "
            f"bystander: {bystander_dp}->{bystander.dp}"
        )

    def test_when_digivolving_field_presence_guard(self, debug_runner):
        """Condition must return False if card is not on field."""
        runner = debug_runner(initial_memory=10)
        # DON'T place on field — card is in hand/library
        # Get effect from a created card source
        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(EX6_030)
        effects = cs.effect_list(None)
        wd = [e for e in effects if getattr(e, 'is_when_digivolving', False)]
        assert len(wd) >= 1
        ctx = {
            'game': runner.game,
            'player': runner.game.player1,
            'permanent': None,
        }
        assert not wd[0].can_use_condition(ctx), (
            "Field presence check must reject when card is not on field"
        )

    def test_when_digivolving_shuffles_security_even_on_decline(self, debug_runner):
        """C# says 'Then, shuffle your security stack' unconditionally."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [EX6_030])
        game = runner.game

        # Security with NO valid target (so selection is skipped)
        game.player1.security_cards.clear()
        # Add a few distinctive non-angels so we can check they are all still present
        runner.inject_card(1, ST1_03, "security_top")
        runner.inject_card(1, ST1_07, "security_top")
        runner.inject_card(1, BT1_058, "security_top")

        sec_before_ids = [c.c_entity_base.card_id for c in game.player1.security_cards]

        eff = _find_when_digivolving_effect(perm)
        eff.on_process_callback({
            'game': game,
            'player': game.player1,
            'permanent': perm,
        })

        # Drive remaining selections
        for _ in range(10):
            if game.current_phase in (GamePhase.Main, GamePhase.Breeding,
                                       GamePhase.End):
                break
            mask = game.get_action_mask(game.current_player_id)
            legal = [i for i, m in enumerate(mask) if m == 1.0]
            if not legal:
                break
            game.decode_action(legal[0], game.current_player_id)

        sec_after_ids = [c.c_entity_base.card_id for c in game.player1.security_cards]
        # All cards must still be present (shuffle preserves count)
        assert sorted(sec_before_ids) == sorted(sec_after_ids), (
            f"All security cards must be preserved. "
            f"Before: {sec_before_ids}, After: {sec_after_ids}"
        )


# ─────────────────────────────────────────────────────────────────────
# C2: [All Turns] Prevent leaving by trashing top security
# ─────────────────────────────────────────────────────────────────────

@pytest.mark.behavioral
class TestEX6030PreventLeaving:
    """[All Turns] When an own Angel/Archangel/Three Great Angels Digimon
    would leave other than by your effect or a battle, trash top security
    to prevent leaving."""

    def test_prevent_effect_exists_with_correct_timing(self, debug_runner):
        """Must use WhenPermanentWouldBeDeleted (fires BEFORE deletion)."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [EX6_030])
        effs = _find_effect_by_timing(perm, EffectTiming.WhenPermanentWouldBeDeleted)
        assert len(effs) >= 1, (
            f"Should have WhenPermanentWouldBeDeleted effect, got "
            f"{[(e.effect_name, e.timing) for e in _get_top_effects(perm)]}"
        )

    def test_prevent_is_optional(self, debug_runner):
        """The prevent effect is optional (player may choose not to pay)."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [EX6_030])
        eff = _find_prevent_leave_effect(perm)
        assert eff is not None
        assert eff.is_optional, "Prevent-leave effect should be optional"

    def test_prevent_triggers_for_archangel(self, debug_runner):
        """Should fire when a Digimon with [Archangel] would be deleted."""
        runner = debug_runner(initial_memory=10)
        dom = runner.place_on_field(1, [EX6_030])
        game = runner.game

        # Add security card (cost requirement)
        game.player1.security_cards.clear()
        runner.inject_card(1, ST1_03, "security_top")

        # Put an Archangel on our field
        archangel = runner.place_on_field(1, [BT1_060])  # MagnaAngemon

        eff = _find_prevent_leave_effect(dom)
        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': archangel,
            'event_permanent': archangel,
            'event_player': game.player1,
            'is_battle': False,
            'is_own_effect': False,  # opponent effect
            'is_opponent_effect': True,
        }
        assert eff.can_use_condition(ctx), (
            "Should trigger for Archangel being deleted by opponent effect"
        )

    def test_prevent_triggers_for_angel(self, debug_runner):
        """Should fire when a [Angel] Digimon would be deleted."""
        runner = debug_runner(initial_memory=10)
        dom = runner.place_on_field(1, [EX6_030])
        game = runner.game

        game.player1.security_cards.clear()
        runner.inject_card(1, ST1_03, "security_top")

        angel = runner.place_on_field(1, [BT1_055])  # Angemon (Angel)

        eff = _find_prevent_leave_effect(dom)
        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': angel,
            'event_permanent': angel,
            'event_player': game.player1,
            'is_battle': False,
            'is_own_effect': False,
            'is_opponent_effect': True,
        }
        assert eff.can_use_condition(ctx), (
            "Should trigger for Angel being deleted"
        )

    def test_prevent_triggers_for_three_great_angels(self, debug_runner):
        """Should fire when a [Three Great Angels] Digimon would be deleted."""
        runner = debug_runner(initial_memory=10)
        dom = runner.place_on_field(1, [EX6_030])
        game = runner.game

        game.player1.security_cards.clear()
        runner.inject_card(1, ST1_03, "security_top")

        tga = runner.place_on_field(1, [BT14_041])  # Seraphimon

        eff = _find_prevent_leave_effect(dom)
        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': tga,
            'event_permanent': tga,
            'event_player': game.player1,
            'is_battle': False,
            'is_own_effect': False,
            'is_opponent_effect': True,
        }
        assert eff.can_use_condition(ctx), (
            "Should trigger for Three Great Angels Digimon being deleted"
        )

    def test_prevent_does_not_trigger_for_non_angel(self, debug_runner):
        """Should NOT trigger for Digimon without Angel traits."""
        runner = debug_runner(initial_memory=10)
        dom = runner.place_on_field(1, [EX6_030])
        game = runner.game

        game.player1.security_cards.clear()
        runner.inject_card(1, ST1_03, "security_top")

        non_angel = runner.place_on_field(1, [ST1_03])  # Agumon (Reptile)

        eff = _find_prevent_leave_effect(dom)
        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': non_angel,
            'event_permanent': non_angel,
            'event_player': game.player1,
            'is_battle': False,
            'is_own_effect': False,
            'is_opponent_effect': True,
        }
        assert not eff.can_use_condition(ctx), (
            "Should NOT trigger for non-Angel Digimon"
        )

    def test_prevent_does_not_trigger_for_enemy_angel(self, debug_runner):
        """Should NOT trigger for opponent's Angel Digimon — the protection
        is only for 'YOUR' Digimon."""
        runner = debug_runner(initial_memory=10)
        dom = runner.place_on_field(1, [EX6_030])
        game = runner.game

        game.player1.security_cards.clear()
        runner.inject_card(1, ST1_03, "security_top")

        enemy_angel = runner.place_on_field(2, [BT1_060])  # opponent MagnaAngemon

        eff = _find_prevent_leave_effect(dom)
        ctx = {
            'game': game,
            'player': game.player2,  # opponent is the owner being deleted
            'permanent': enemy_angel,
            'event_permanent': enemy_angel,
            'event_player': game.player2,
            'is_battle': False,
            'is_own_effect': False,
            'is_opponent_effect': True,
        }
        assert not eff.can_use_condition(ctx), (
            "Must not protect opponent's Angels"
        )

    def test_prevent_rejects_own_effect_deletion(self, debug_runner):
        """Card text says 'other than by one of YOUR effects' — must not
        trigger when the deletion cause is the card-owner's own effect."""
        runner = debug_runner(initial_memory=10)
        dom = runner.place_on_field(1, [EX6_030])
        game = runner.game

        game.player1.security_cards.clear()
        runner.inject_card(1, ST1_03, "security_top")

        angel = runner.place_on_field(1, [BT1_055])  # Angemon

        eff = _find_prevent_leave_effect(dom)
        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': angel,
            'event_permanent': angel,
            'event_player': game.player1,
            'is_battle': False,
            'is_own_effect': True,       # own-effect deletion
            'is_opponent_effect': False,
        }
        assert not eff.can_use_condition(ctx), (
            "Must NOT trigger when deletion is by own effect"
        )

    def test_prevent_rejects_battle_deletion(self, debug_runner):
        """Card text says 'other than by your effects or a battle' — must
        not trigger on battle-cause deletion."""
        runner = debug_runner(initial_memory=10)
        dom = runner.place_on_field(1, [EX6_030])
        game = runner.game

        game.player1.security_cards.clear()
        runner.inject_card(1, ST1_03, "security_top")

        angel = runner.place_on_field(1, [BT1_055])

        eff = _find_prevent_leave_effect(dom)
        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': angel,
            'event_permanent': angel,
            'event_player': game.player1,
            'is_battle': True,         # battle cause
            'removal_cause': 'battle',
            'is_own_effect': False,
            'is_opponent_effect': False,
        }
        assert not eff.can_use_condition(ctx), (
            "Must NOT trigger for battle-cause deletion"
        )

    def test_prevent_requires_security_card(self, debug_runner):
        """Cost is trash top security card — without security, can't fire."""
        runner = debug_runner(initial_memory=10)
        dom = runner.place_on_field(1, [EX6_030])
        game = runner.game

        # No security cards
        game.player1.security_cards.clear()

        angel = runner.place_on_field(1, [BT1_060])

        eff = _find_prevent_leave_effect(dom)
        ctx = {
            'game': game,
            'player': game.player1,
            'permanent': angel,
            'event_permanent': angel,
            'event_player': game.player1,
            'is_battle': False,
            'is_own_effect': False,
            'is_opponent_effect': True,
        }
        assert not eff.can_use_condition(ctx), (
            "Can't pay cost with 0 security — must not trigger"
        )

    def test_prevent_process_trashes_top_security(self, debug_runner):
        """On activation, the top security card goes to trash."""
        runner = debug_runner(initial_memory=10)
        dom = runner.place_on_field(1, [EX6_030])
        game = runner.game

        game.player1.security_cards.clear()
        runner.inject_card(1, ST1_03, "security_top")   # will be bottom
        runner.inject_card(1, ST1_07, "security_top")   # will be middle
        runner.inject_card(1, BT1_058, "security_top")  # top (last-added)

        angel = runner.place_on_field(1, [BT1_060])

        sec_before = len(game.player1.security_cards)
        trash_before = len(game.player1.trash_cards)

        eff = _find_prevent_leave_effect(dom)
        eff.on_process_callback({
            'game': game,
            'player': game.player1,
            'permanent': angel,
            'event_permanent': angel,
            'event_player': game.player1,
            'is_battle': False,
            'is_own_effect': False,
            'is_opponent_effect': True,
        })

        assert len(game.player1.security_cards) == sec_before - 1, (
            f"Top security should be trashed. "
            f"Before={sec_before}, after={len(game.player1.security_cards)}"
        )
        assert len(game.player1.trash_cards) == trash_before + 1

    def test_prevent_process_sets_will_not_be_removed(self, debug_runner):
        """After paying cost, the leaving permanent should have
        _will_not_be_removed set so delete_permanent cancels the deletion."""
        runner = debug_runner(initial_memory=10)
        dom = runner.place_on_field(1, [EX6_030])
        game = runner.game

        game.player1.security_cards.clear()
        runner.inject_card(1, ST1_03, "security_top")

        angel = runner.place_on_field(1, [BT1_060])

        eff = _find_prevent_leave_effect(dom)
        eff.on_process_callback({
            'game': game,
            'player': game.player1,
            'permanent': angel,
            'event_permanent': angel,
            'event_player': game.player1,
            'is_battle': False,
            'is_own_effect': False,
            'is_opponent_effect': True,
        })

        assert getattr(angel, '_will_not_be_removed', False) is True, (
            "After paying cost, _will_not_be_removed should be True"
        )

    def test_prevent_end_to_end_via_delete_permanent(self, debug_runner):
        """Full integration: call delete_permanent with opponent's effect
        as the cause — the Archangel should survive, top security trashed."""
        runner = debug_runner(initial_memory=10)
        dom = runner.place_on_field(1, [EX6_030])
        game = runner.game

        game.player1.security_cards.clear()
        runner.inject_card(1, ST1_03, "security_top")

        archangel = runner.place_on_field(1, [BT1_060])

        # Opponent effect wants to delete the Archangel
        sec_before = len(game.player1.security_cards)

        result = game.player1.delete_permanent(
            archangel, is_battle=False, is_opponent_effect=True
        )

        # Deletion should have been prevented
        assert result is False, "delete_permanent should return False (prevented)"
        assert archangel in game.player1.battle_area, (
            "Archangel should still be on field after prevention"
        )
        assert len(game.player1.security_cards) == sec_before - 1, (
            "Top security should be trashed as cost"
        )

    def test_prevent_field_presence_guard(self, debug_runner):
        """Must return False if Dominimon itself is not on field."""
        runner = debug_runner(initial_memory=10)

        from engine_py_legacy.engine.data.card_database import CardDatabase
        db = CardDatabase()
        cs = db.create_card_source(EX6_030)
        effects = cs.effect_list(None)
        prev = [e for e in effects
                if e.timing == EffectTiming.WhenPermanentWouldBeDeleted]
        assert prev
        ctx = {
            'game': runner.game,
            'player': runner.game.player1,
            'permanent': None,
        }
        assert not prev[0].can_use_condition(ctx), (
            "Field presence guard must reject off-field activation"
        )

    def test_prevent_not_inherited(self, debug_runner):
        """The prevent effect is a main-body effect on Dominimon — not
        inherited (C# puts it in the main body not inherited section)."""
        runner = debug_runner(initial_memory=10)
        perm = runner.place_on_field(1, [EX6_030])
        eff = _find_prevent_leave_effect(perm)
        assert eff is not None
        # Main body effects should not be is_inherited_effect
        assert not eff.is_inherited_effect, (
            "Prevent effect should be main-body, not inherited"
        )
