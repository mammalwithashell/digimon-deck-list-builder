"""Behavioral tests for P-187 Mastemon (Lv.6 Purple/Yellow Angel, DNA digivolve).

Card text:
  [When Digivolving] <Recovery +1 (Deck)> (Place the top card of your deck on
  top of your security stack.) Then, if DNA digivolving, by placing 1 other
  Digimon or Tamer as the top or bottom security card, trash your opponent's
  top security card.
  [When Digivolving] [When Attacking] [Once Per Turn] By trashing your top
  security card, you may play 1 purple or yellow Digimon card with 6000 DP
  or less from your hand or trash without paying the cost.

DNA Digivolve: Purple Lv.5 + Yellow Lv.5 : Cost 0.

C# reference: DCGO/Assets/Scripts/CardEffect/P/Purple/P_187.cs
"""

import pytest
from digimon_gym.engine.data.enums import EffectTiming, GamePhase


def _resolve_selections(runner, max_steps: int = 20) -> None:
    """Auto-resolve pending selections, preferring non-decline actions.

    Unlike runner.auto_resolve(), this picks the FIRST non-62 legal action
    when one exists (62 = decline/pass). Use this when tests want to verify
    the effect of actually choosing a target.
    """
    for _ in range(max_steps):
        if runner.game.game_over:
            return
        if runner.game.current_phase in (
            GamePhase.Main, GamePhase.Breeding, GamePhase.End
        ):
            return
        legal = runner.action_mask()
        if not legal:
            return
        non_pass = [a for a in legal if a != 62]
        pick = non_pass[0] if non_pass else legal[0]
        runner.execute(pick)


# Test card IDs
P187 = "P-187"            # Mastemon Lv.6 Purple/Yellow DP:13000 Cost:13
SKULLMERAMON = "BT3-085"  # Purple Lv.5 DP:6000
MAGNAANGEMON = "BT1-060"  # Yellow Lv.5 DP:6000
PUMPKINMON = "BT2-076"    # Purple Lv.5 DP:6000
AGUMON = "BT1-010"        # Red Lv.3 (Digimon, low DP, red so not playable by OPT)
TSUKAIMON = "BT1-045"     # Yellow Lv.3 DP:3000 (valid OPT target)
KUDAMON = "BT1-046"       # Yellow Lv.3 DP:1000 (valid OPT target)
APEMON = "BT6-038"        # Yellow Lv.4 DP:8000 (INVALID OPT target — DP > 6000)
KARI = "BT2-087"          # Yellow Tamer (non-digimon target for place-to-security)


@pytest.mark.behavioral
class TestP187Effects:
    """P-187 Mastemon effect structure and registration."""

    def test_has_when_digivolving_recovery_effect(self, debug_runner):
        """Should have a When Digivolving effect for Recovery + DNA placement."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, P187, "hand")
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving]
        assert len(when_digi) >= 1, "Should have at least one When Digivolving effect"

    def test_has_when_attacking_opt_effect(self, debug_runner):
        """Should have a When Attacking OPT effect to trash security & play digimon."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, P187, "hand")
        effects = card.effect_list(None)
        on_attack = [e for e in effects if getattr(e, 'is_on_attack', False)]
        assert len(on_attack) >= 1, (
            "Should have a When Attacking effect"
        )

    def test_opt_effects_share_hash_string(self, debug_runner):
        """Both [When Digivolving] and [When Attacking] branches of the OPT clause
        must share the SAME hash string so [Once Per Turn] is enforced across both."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, P187, "hand")
        effects = card.effect_list(None)
        play_effects = [e for e in effects
                        if e.effect_name and "Trash" in e.effect_name
                        and "play" in e.effect_name.lower()]
        assert len(play_effects) >= 2, (
            f"Should have 2 effects (WhenDigi + WhenAttacking) for OPT, "
            f"got {len(play_effects)}"
        )
        hashes = {e.hash_string for e in play_effects if e.hash_string}
        assert len(hashes) == 1, (
            f"OPT effects should share the same hash string. Got: {hashes}"
        )

    def test_opt_effect_max_count_one(self, debug_runner):
        """OPT effects must have max_count_per_turn = 1."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, P187, "hand")
        effects = card.effect_list(None)
        opt_effects = [e for e in effects
                       if e.effect_name and "Trash" in e.effect_name
                       and "play" in e.effect_name.lower()]
        for e in opt_effects:
            assert e.max_count_per_turn == 1, (
                f"Effect '{e.effect_name}' should be OPT (max_count_per_turn=1), "
                f"got {e.max_count_per_turn}"
            )

    def test_opt_effect_is_optional(self, debug_runner):
        """OPT effects (By trashing security, you MAY play) should be optional."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, P187, "hand")
        effects = card.effect_list(None)
        opt_effects = [e for e in effects
                       if e.effect_name and "Trash" in e.effect_name
                       and "play" in e.effect_name.lower()]
        for e in opt_effects:
            assert e.is_optional, (
                f"Effect '{e.effect_name}' should be optional ('you MAY play')"
            )


@pytest.mark.behavioral
class TestP187RecoveryOnDigivolve:
    """[When Digivolving] <Recovery +1 (Deck)>."""

    def test_recovery_increases_security(self, debug_runner):
        """When Digivolving, Recovery +1 should add a security card from the top of deck."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [P187])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving][0]

        # Ensure there's at least 1 library card
        player = runner.game.player1
        lib_before = len(player.library_cards)
        sec_before = len(player.security_cards)
        assert lib_before >= 1, "Test setup: need at least 1 library card"

        ctx = {
            'player': player,
            'permanent': perm,
            'game': runner.game,
            'is_dna_digivolve': False,
        }
        when_digi.on_process_callback(ctx)

        sec_after = len(player.security_cards)
        lib_after = len(player.library_cards)
        assert sec_after == sec_before + 1, (
            f"Security should gain 1 card. Before={sec_before}, after={sec_after}")
        assert lib_after == lib_before - 1, (
            f"Library should lose 1 card. Before={lib_before}, after={lib_after}")

    def test_recovery_does_not_require_dna(self, debug_runner):
        """Recovery fires even without DNA digivolving."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [P187])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving][0]

        sec_before = len(runner.game.player1.security_cards)

        ctx = {
            'player': runner.game.player1,
            'permanent': perm,
            'game': runner.game,
            'is_dna_digivolve': False,
        }
        when_digi.on_process_callback(ctx)
        runner.auto_resolve()

        sec_after = len(runner.game.player1.security_cards)
        assert sec_after == sec_before + 1, (
            "Recovery +1 must fire even when NOT DNA digivolving")


@pytest.mark.behavioral
class TestP187DnaPlaceSecurity:
    """[When Digivolving] DNA-only: place 1 other Digimon/Tamer as security, trash opp top."""

    def test_non_dna_does_not_trigger_placement(self, debug_runner):
        """Without DNA digivolving, the placement cost/effect should NOT fire."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [P187])
        other = runner.place_on_field(1, [AGUMON])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving][0]

        opp_sec_before = len(runner.game.player2.security_cards)
        p1_battle_before = len(runner.game.player1.battle_area)

        ctx = {
            'player': runner.game.player1,
            'permanent': perm,
            'game': runner.game,
            'is_dna_digivolve': False,
        }
        when_digi.on_process_callback(ctx)
        runner.auto_resolve()

        # Opponent's security should be untouched
        assert len(runner.game.player2.security_cards) == opp_sec_before, (
            "Opp security should not be trashed without DNA")
        # Own battle area unchanged (no placement)
        assert len(runner.game.player1.battle_area) == p1_battle_before, (
            "Own battle area should not lose a permanent without DNA")

    def test_dna_offers_placement_selection(self, debug_runner):
        """With DNA digivolving, should enter a selection phase to pick a permanent."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [P187])
        other = runner.place_on_field(1, [AGUMON])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving][0]

        ctx = {
            'player': runner.game.player1,
            'permanent': perm,
            'game': runner.game,
            'is_dna_digivolve': True,
        }
        when_digi.on_process_callback(ctx)

        # Should have a pending selection for the target permanent
        assert runner.game.pending_selection is not None, (
            "DNA placement should request a selection for target permanent")

    def test_dna_placement_excludes_self(self, debug_runner):
        """The 'other' Digimon/Tamer — P-187 itself must not be a valid target."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [P187])
        other = runner.place_on_field(1, [AGUMON])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving][0]

        ctx = {
            'player': runner.game.player1,
            'permanent': perm,
            'game': runner.game,
            'is_dna_digivolve': True,
        }
        when_digi.on_process_callback(ctx)

        sel = runner.game.pending_selection
        assert sel is not None
        # Build set of permanents corresponding to valid_indices
        from digimon_gym.engine.game.constants import SEL_MY_FIELD_START
        my_battle = runner.game.player1.battle_area
        targets = []
        for idx in sel.valid_indices:
            off = idx - SEL_MY_FIELD_START
            if 0 <= off < len(my_battle):
                targets.append(my_battle[off])
        assert perm not in targets, (
            "P-187 itself should NOT be a valid target for the 'other' placement")
        assert other in targets, (
            "Other own Digimon should be a valid placement target")

    def test_dna_placement_accepts_tamer(self, debug_runner):
        """A Tamer permanent should be a valid placement target (Digimon OR Tamer)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [P187])
        tamer = runner.place_on_field(1, [KARI])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving][0]

        ctx = {
            'player': runner.game.player1,
            'permanent': perm,
            'game': runner.game,
            'is_dna_digivolve': True,
        }
        when_digi.on_process_callback(ctx)

        sel = runner.game.pending_selection
        assert sel is not None
        from digimon_gym.engine.game.constants import SEL_MY_FIELD_START
        my_battle = runner.game.player1.battle_area
        targets = []
        for idx in sel.valid_indices:
            off = idx - SEL_MY_FIELD_START
            if 0 <= off < len(my_battle):
                targets.append(my_battle[off])
        assert tamer in targets, (
            "A Tamer should be a valid placement target (Digimon or Tamer)")

    def test_dna_placement_is_optional(self, debug_runner):
        """Placement is 'by placing 1 other' — player can decline (no trash cost paid)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [P187])
        other = runner.place_on_field(1, [AGUMON])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving][0]

        ctx = {
            'player': runner.game.player1,
            'permanent': perm,
            'game': runner.game,
            'is_dna_digivolve': True,
        }
        when_digi.on_process_callback(ctx)

        sel = runner.game.pending_selection
        assert sel is not None
        assert sel.is_optional, (
            "'By placing' is a cost — the whole effect is optional (can decline)")

    def test_dna_placement_trashes_opp_top_security(self, debug_runner):
        """After successful placement, opponent's top security card is trashed."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [P187])
        other = runner.place_on_field(1, [AGUMON])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving][0]

        opp_sec_before = len(runner.game.player2.security_cards)
        assert opp_sec_before >= 1, "Test setup: opponent must have security"

        ctx = {
            'player': runner.game.player1,
            'permanent': perm,
            'game': runner.game,
            'is_dna_digivolve': True,
        }
        when_digi.on_process_callback(ctx)
        # Resolve selections: target permanent, then top/bottom branch choice
        _resolve_selections(runner)

        opp_sec_after = len(runner.game.player2.security_cards)
        assert opp_sec_after == opp_sec_before - 1, (
            f"Opponent's top security card should be trashed after placement. "
            f"Before={opp_sec_before}, after={opp_sec_after}")

    def test_dna_placement_moves_perm_to_own_security(self, debug_runner):
        """The placed permanent leaves the battle area and its top card joins own security."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [P187])
        other = runner.place_on_field(1, [AGUMON])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving][0]

        own_sec_before = len(runner.game.player1.security_cards)
        own_battle_before = len(runner.game.player1.battle_area)

        ctx = {
            'player': runner.game.player1,
            'permanent': perm,
            'game': runner.game,
            'is_dna_digivolve': True,
        }
        when_digi.on_process_callback(ctx)
        _resolve_selections(runner)

        # Placed perm should no longer be in battle area
        assert other not in runner.game.player1.battle_area, (
            "Selected permanent should leave the battle area after placement")
        # Own security should have gained 1 card (from recovery) plus 1 (from placement)
        own_sec_after = len(runner.game.player1.security_cards)
        assert own_sec_after == own_sec_before + 2, (
            f"Own security should gain 2 cards (Recovery +1 and placement +1). "
            f"Before={own_sec_before}, after={own_sec_after}")

    def test_dna_placement_skipped_if_no_valid_target(self, debug_runner):
        """If P-187 is the only permanent, there's no 'other' to place → effect does nothing
        beyond the initial Recovery +1 step."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [P187])
        card = perm.top_card
        effects = card.effect_list(None)
        when_digi = [e for e in effects if e.is_when_digivolving][0]

        opp_sec_before = len(runner.game.player2.security_cards)

        ctx = {
            'player': runner.game.player1,
            'permanent': perm,
            'game': runner.game,
            'is_dna_digivolve': True,
        }
        when_digi.on_process_callback(ctx)
        runner.auto_resolve()

        # Opponent security untouched (no placement → no trash)
        assert len(runner.game.player2.security_cards) == opp_sec_before, (
            "With no valid 'other' target, opponent security should not be trashed")


@pytest.mark.behavioral
class TestP187PlayDigimonOpt:
    """[When Digivolving] [When Attacking] [Once Per Turn] Trash own top security,
    play 1 purple/yellow Digimon <= 6000 DP from hand or trash."""

    def test_opt_condition_requires_field_presence(self, debug_runner):
        """Condition should fail if P-187 is not on the field (card in hand)."""
        runner = debug_runner(initial_memory=5)
        # Don't place; just create a hand card
        card = runner.inject_card(1, P187, "hand")
        effects = card.effect_list(None)
        opt_effects = [e for e in effects
                       if e.effect_name and "Trash" in e.effect_name
                       and "play" in e.effect_name.lower()]
        for e in opt_effects:
            assert not e.can_use_condition({}), (
                f"Effect '{e.effect_name}' should fail condition when P-187 "
                f"is not on the field")

    def test_opt_condition_requires_security(self, debug_runner):
        """If own security is empty, the OPT effect can't activate (cost can't be paid)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [P187])
        card = perm.top_card
        effects = card.effect_list(None)
        opt_effects = [e for e in effects
                       if e.effect_name and "Trash" in e.effect_name
                       and "play" in e.effect_name.lower()]

        # Clear P1 security
        runner.game.player1.security_cards.clear()

        for e in opt_effects:
            assert not e.can_use_condition({}), (
                f"Effect '{e.effect_name}' should fail when own security empty")

    def test_opt_trashes_own_top_security(self, debug_runner):
        """The cost trashes own top security card."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [P187])
        card = perm.top_card
        effects = card.effect_list(None)
        opt_effects = [e for e in effects
                       if e.effect_name and "Trash" in e.effect_name
                       and "play" in e.effect_name.lower()]
        assert opt_effects, "Should have OPT effect"

        # Put a playable Digimon in hand
        runner.inject_card(1, TSUKAIMON, "hand")

        player = runner.game.player1
        sec_before = len(player.security_cards)
        trash_before = len(player.trash_cards)

        ctx = {'player': player, 'permanent': perm, 'game': runner.game}
        opt_effects[0].on_process_callback(ctx)
        # Resolve the play-selection phase
        runner.auto_resolve()

        sec_after = len(player.security_cards)
        assert sec_after == sec_before - 1, (
            f"Top security should be trashed. Before={sec_before}, after={sec_after}")

    def test_opt_plays_valid_purple_digimon_from_hand(self, debug_runner):
        """Plays 1 purple Digimon <= 6000 DP from hand."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [P187])
        card = perm.top_card
        effects = card.effect_list(None)
        opt_effects = [e for e in effects
                       if e.effect_name and "Trash" in e.effect_name
                       and "play" in e.effect_name.lower()]

        # Put a valid Purple Digimon (SkullMeramon Lv.5 Purple 6000 DP) in hand
        runner.inject_card(1, SKULLMERAMON, "hand")

        player = runner.game.player1
        battle_before = len(player.battle_area)

        ctx = {'player': player, 'permanent': perm, 'game': runner.game}
        opt_effects[0].on_process_callback(ctx)
        _resolve_selections(runner)

        battle_after = len(player.battle_area)
        assert battle_after == battle_before + 1, (
            f"Should play 1 Digimon. Before={battle_before}, after={battle_after}")

    def test_opt_plays_valid_yellow_digimon_from_trash(self, debug_runner):
        """Plays 1 yellow Digimon <= 6000 DP from trash."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [P187])
        card = perm.top_card
        effects = card.effect_list(None)
        opt_effects = [e for e in effects
                       if e.effect_name and "Trash" in e.effect_name
                       and "play" in e.effect_name.lower()]

        # Put MagnaAngemon (Yellow Lv.5 6000 DP) in trash
        runner.inject_card(1, MAGNAANGEMON, "trash")
        runner.clear_zone(1, "hand")  # Ensure no hand interference

        player = runner.game.player1
        battle_before = len(player.battle_area)

        ctx = {'player': player, 'permanent': perm, 'game': runner.game}
        opt_effects[0].on_process_callback(ctx)
        _resolve_selections(runner)

        battle_after = len(player.battle_area)
        assert battle_after == battle_before + 1, (
            f"Should play MagnaAngemon from trash. Before={battle_before}, after={battle_after}")

    def test_opt_rejects_wrong_color(self, debug_runner):
        """A Red Digimon should NOT be a valid target (must be Purple or Yellow)."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [P187])
        card = perm.top_card
        effects = card.effect_list(None)
        opt_effects = [e for e in effects
                       if e.effect_name and "Trash" in e.effect_name
                       and "play" in e.effect_name.lower()]

        runner.clear_zone(1, "hand")
        runner.inject_card(1, AGUMON, "hand")  # Red Lv.3 2000 DP — wrong color

        player = runner.game.player1
        battle_before = len(player.battle_area)

        ctx = {'player': player, 'permanent': perm, 'game': runner.game}
        opt_effects[0].on_process_callback(ctx)
        runner.auto_resolve()

        battle_after = len(player.battle_area)
        assert battle_after == battle_before, (
            f"Red Digimon should NOT be playable by OPT effect. "
            f"Before={battle_before}, after={battle_after}")

    def test_opt_rejects_high_dp(self, debug_runner):
        """A Digimon with > 6000 DP should NOT be a valid target."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [P187])
        card = perm.top_card
        effects = card.effect_list(None)
        opt_effects = [e for e in effects
                       if e.effect_name and "Trash" in e.effect_name
                       and "play" in e.effect_name.lower()]

        runner.clear_zone(1, "hand")
        runner.inject_card(1, APEMON, "hand")  # Yellow Lv.4 8000 DP — too big

        player = runner.game.player1
        battle_before = len(player.battle_area)

        ctx = {'player': player, 'permanent': perm, 'game': runner.game}
        opt_effects[0].on_process_callback(ctx)
        runner.auto_resolve()

        battle_after = len(player.battle_area)
        assert battle_after == battle_before, (
            f"8000 DP Digimon should NOT be playable. "
            f"Before={battle_before}, after={battle_after}")

    def test_opt_accepts_exactly_6000_dp(self, debug_runner):
        """Exactly 6000 DP is the boundary — should be playable."""
        runner = debug_runner(initial_memory=5)
        perm = runner.place_on_field(1, [P187])
        card = perm.top_card
        effects = card.effect_list(None)
        opt_effects = [e for e in effects
                       if e.effect_name and "Trash" in e.effect_name
                       and "play" in e.effect_name.lower()]

        runner.clear_zone(1, "hand")
        runner.inject_card(1, PUMPKINMON, "hand")  # Purple Lv.5 exactly 6000 DP

        player = runner.game.player1
        battle_before = len(player.battle_area)

        ctx = {'player': player, 'permanent': perm, 'game': runner.game}
        opt_effects[0].on_process_callback(ctx)
        _resolve_selections(runner)

        battle_after = len(player.battle_area)
        assert battle_after == battle_before + 1, (
            f"Pumpkinmon (exactly 6000 DP) should be playable. "
            f"Before={battle_before}, after={battle_after}")


@pytest.mark.behavioral
class TestP187ErrorChecklist:
    """16-item faithfulness checklist items relevant to P-187."""

    def test_when_attacking_uses_on_use_attack(self, debug_runner):
        """Item #2: [When Attacking] must use OnUseAttack timing (= attacker self)."""
        runner = debug_runner(initial_memory=5)
        card = runner.inject_card(1, P187, "hand")
        effects = card.effect_list(None)
        # Find the attacking-trigger branch of the OPT
        on_attack = [e for e in effects if getattr(e, 'is_on_attack', False)]
        assert on_attack, "Should have an is_on_attack effect"
        for e in on_attack:
            assert e.timing == EffectTiming.OnUseAttack, (
                f"When Attacking effect must use OnUseAttack (28), "
                f"got {e.timing}")

    def test_battle_area_not_field_cards(self):
        """Item #16: script must not reference player.field_cards."""
        import digimon_gym.engine.data.scripts.p.p_187 as mod
        import inspect
        src = inspect.getsource(mod)
        assert "field_cards" not in src, (
            "Script must use player.battle_area, not field_cards")

    def test_importable(self):
        """Script must import cleanly."""
        from digimon_gym.engine.data.scripts.p.p_187 import P_187  # noqa: F401
