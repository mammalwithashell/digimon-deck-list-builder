"""Behavioral tests for EX6-029 Mastemon (Lv.6 Yellow/Purple Angel Ace).

Card text (effect_text):
  [Hand] [Counter] <Blast DNA Digivolve ([Angewomon] + [LadyDevimon])>
      (1 of your specified Digimon and 1 specified card in hand may DNA
       digivolve into this card.)
  [On Play] [When Digivolving] You may play 1 level 5 or lower Digimon
      card with the [Angel], [Archangel] or [Fallen Angel] trait from
      your hand or trash without paying the cost. Then, if DNA digivolving,
      place 1 other Digimon at the bottom of its owner's security stack,
      and trash cards from the top of your opponent's security stack
      until it has 4 left.

Inherited text:
  Ace Overflow <-4> (As this card moves from the field or under a card
      to an area other than those, lose 4 memory.)

C# reference (DCGO/Assets/Scripts/CardEffect/EX6/Yellow/EX6_029.cs):
  - On Counter timing: BlastDNADigivolveEffect with
    BlastDNACondition("Angewomon") + BlastDNACondition("LadyDevimon").
  - NoTiming jogress condition: Yellow Lv.5 + Purple Lv.5 (also
    enforced by metadata dna_costs).
  - OnEnterFieldAnyone (On Play and When Digivolving) share the same
    resolution: select 1 free-play card from hand/trash (Angel traits
    Lv<=5), then if is_dna_digivolve -> select 1 other Digimon on ANY
    battle area (IsPermanentExistsOnBattleArea) and put at bottom of
    its owner's security, then trash opponent's top security until 4.
"""

import pytest

from digimon_gym.engine.data.enums import EffectTiming, GamePhase


def _resolve_with_first_non_decline(runner, max_steps: int = 10):
    """Resolve pending selections by picking the first non-decline action at
    each step. 'Decline / Pass' is action 62 — we skip it when a selection
    action is available, so optional effects actually resolve into play."""
    for _ in range(max_steps):
        if runner.game.game_over:
            break
        if runner.game.current_phase in (
            GamePhase.Main, GamePhase.Breeding, GamePhase.End
        ):
            break
        legal = runner.action_mask()
        if not legal:
            break
        # Prefer any non-decline action (62); only fall back to 62 if that's
        # the only legal option.
        non_decline = [a for a in legal if a != 62]
        choice = non_decline[0] if non_decline else legal[0]
        runner.execute(choice)


# Card IDs used as test fixtures
EX6_029 = "EX6-029"        # Mastemon (Lv.6 Yellow/Purple Angel) - card under test
BT1_055 = "BT1-055"        # Angemon (Lv.4 Angel)
BT2_037 = "BT2-037"        # Angewomon (Lv.5 Archangel, Yellow)
BT3_088 = "BT3-088"        # LadyDevimon (Lv.5 Fallen Angel, Purple)
BT1_009 = "BT1-009"        # Agumon (Lv.3 Reptile) - non-Angel
BT1_020 = "BT1-020"        # Groundramon (Lv.5 Earth Dragon) - non-Angel
ST1_03 = "ST1-03"          # Greymon placeholder for deck filler


def _deck_filler(n: int = 50):
    return [ST1_03] * n


@pytest.mark.behavioral
class TestEX6029EffectStructure:
    """Verify the effect list structure matches the card text and C# reference."""

    def test_card_loads(self, debug_runner):
        """EX6-029 should load and its script should produce effects."""
        runner = debug_runner(
            deck1=[EX6_029] * 4 + _deck_filler(46),
            deck2=_deck_filler(50),
            initial_memory=3,
        )
        cs = runner.inject_card(1, EX6_029, "hand")
        effects = cs.effect_list(None)
        assert len(effects) >= 3, (
            f"EX6-029 must have multiple effects (Blast DNA, On Play, "
            f"When Digivolving). Got {len(effects)}: "
            f"{[e.effect_name for e in effects]}"
        )

    def test_has_blast_dna_digivolve_counter_effect(self, debug_runner):
        """Card text: [Hand] [Counter] <Blast DNA Digivolve
        ([Angewomon] + [LadyDevimon])>."""
        runner = debug_runner(
            deck1=[EX6_029] * 4 + _deck_filler(46),
            deck2=_deck_filler(50),
        )
        cs = runner.inject_card(1, EX6_029, "hand")
        effects = cs.effect_list(None)
        blast_effects = [
            e for e in effects
            if getattr(e, "_is_blast_dna_digivolve", False)
        ]
        assert len(blast_effects) == 1, (
            f"EX6-029 should have exactly 1 Blast DNA Digivolve effect. "
            f"Got {len(blast_effects)}: {[e.effect_name for e in effects]}"
        )
        blast = blast_effects[0]
        assert blast.is_counter_effect, (
            "Blast DNA Digivolve must be marked is_counter_effect (hand counter timing)"
        )

    def test_blast_dna_names_match_angewomon_ladydevimon(self, debug_runner):
        """The Blast DNA names must be Angewomon + LadyDevimon (matches C#
        BlastDNACondition list)."""
        runner = debug_runner(
            deck1=[EX6_029] * 4 + _deck_filler(46),
            deck2=_deck_filler(50),
        )
        cs = runner.inject_card(1, EX6_029, "hand")
        blast = next(
            e for e in cs.effect_list(None)
            if getattr(e, "_is_blast_dna_digivolve", False)
        )
        names = getattr(blast, "_blast_dna_names", None)
        assert names is not None, (
            "Blast DNA effect should expose _blast_dna_names metadata"
        )
        assert "Angewomon" in names and "LadyDevimon" in names, (
            f"Blast DNA names must include Angewomon and LadyDevimon. Got: {names}"
        )

    def test_has_on_play_effect(self, debug_runner):
        """Card text: [On Play] ..."""
        runner = debug_runner(
            deck1=[EX6_029] * 4 + _deck_filler(46),
            deck2=_deck_filler(50),
        )
        cs = runner.inject_card(1, EX6_029, "hand")
        effects = cs.effect_list(None)
        on_play = [e for e in effects if getattr(e, "is_on_play", False)]
        assert len(on_play) == 1, (
            f"EX6-029 should have exactly 1 On Play effect. "
            f"Got {len(on_play)}: {[e.effect_name for e in on_play]}"
        )

    def test_has_when_digivolving_effect(self, debug_runner):
        """Card text: [When Digivolving] ..."""
        runner = debug_runner(
            deck1=[EX6_029] * 4 + _deck_filler(46),
            deck2=_deck_filler(50),
        )
        cs = runner.inject_card(1, EX6_029, "hand")
        effects = cs.effect_list(None)
        wd = [e for e in effects if getattr(e, "is_when_digivolving", False)]
        assert len(wd) == 1, (
            f"EX6-029 should have exactly 1 When Digivolving effect. "
            f"Got {len(wd)}: {[e.effect_name for e in wd]}"
        )

    def test_on_play_effect_is_optional(self, debug_runner):
        """Card text uses 'You may play' => effect must be optional."""
        runner = debug_runner(
            deck1=[EX6_029] * 4 + _deck_filler(46),
            deck2=_deck_filler(50),
        )
        cs = runner.inject_card(1, EX6_029, "hand")
        on_play = next(
            e for e in cs.effect_list(None)
            if getattr(e, "is_on_play", False)
        )
        assert on_play.is_optional, (
            "On Play effect must be optional ('You may play ...')"
        )


@pytest.mark.behavioral
class TestEX6029PlayAngelFromHand:
    """Clause: [On Play]/[When Digivolving] You may play 1 Lv.5 or lower
    Digimon card with the [Angel], [Archangel] or [Fallen Angel] trait
    from your hand or trash without paying the cost."""

    def test_plays_angel_from_hand_no_cost(self, debug_runner):
        """Angemon (Angel trait, Lv.4) in hand should be playable for free
        by Mastemon's On Play effect."""
        runner = debug_runner(
            deck1=[EX6_029] * 4 + _deck_filler(46),
            deck2=_deck_filler(50),
            initial_memory=20,
        )
        runner.clear_zone(1, "hand")
        perm = runner.place_on_field(1, [EX6_029])
        runner.inject_card(1, BT1_055, "hand")
        game = runner.game
        hand_before = len(game.player1.hand_cards)
        game.memory = 0  # post-cost state; effect should still allow free play

        top = perm.top_card
        on_play = next(
            e for e in top.effect_list(None) if getattr(e, "is_on_play", False)
        )
        on_play.on_process_callback({
            "player": game.player1,
            "game": game,
            "permanent": perm,
            "is_dna_digivolve": False,
        })
        _resolve_with_first_non_decline(runner)

        # Angemon should now be on the field
        field_ids = [p.top_card.card_id for p in game.player1.battle_area if p.top_card]
        assert BT1_055 in field_ids, (
            f"Angemon (Angel Lv.4) should be played from hand. Field: {field_ids}"
        )
        # Memory should not have decreased (free play)
        assert game.memory == 0, (
            f"Free play should not cost memory. Memory: {game.memory}"
        )
        # Hand should have shrunk by 1
        assert len(game.player1.hand_cards) == hand_before - 1, (
            f"Hand should have 1 fewer card. "
            f"Before: {hand_before}, After: {len(game.player1.hand_cards)}"
        )

    def test_plays_archangel_angewomon(self, debug_runner):
        """Angewomon (Archangel trait, Lv.5) should qualify."""
        runner = debug_runner(
            deck1=[EX6_029] * 4 + [BT2_037] * 4 + _deck_filler(42),
            deck2=_deck_filler(50),
            initial_memory=20,
        )
        perm = runner.place_on_field(1, [EX6_029])
        runner.inject_card(1, BT2_037, "hand")
        game = runner.game
        game.memory = 0

        top = perm.top_card
        on_play = next(
            e for e in top.effect_list(None) if getattr(e, "is_on_play", False)
        )
        on_play.on_process_callback({
            "player": game.player1,
            "game": game,
            "permanent": perm,
            "is_dna_digivolve": False,
        })
        _resolve_with_first_non_decline(runner)

        field_ids = [p.top_card.card_id for p in game.player1.battle_area if p.top_card]
        assert BT2_037 in field_ids, (
            f"Angewomon (Archangel Lv.5) should be played for free. Field: {field_ids}"
        )

    def test_plays_fallen_angel_ladydevimon(self, debug_runner):
        """LadyDevimon (Fallen Angel trait, Lv.5) should qualify."""
        runner = debug_runner(
            deck1=[EX6_029] * 4 + [BT3_088] * 4 + _deck_filler(42),
            deck2=_deck_filler(50),
            initial_memory=20,
        )
        perm = runner.place_on_field(1, [EX6_029])
        runner.inject_card(1, BT3_088, "hand")
        game = runner.game
        game.memory = 0

        top = perm.top_card
        on_play = next(
            e for e in top.effect_list(None) if getattr(e, "is_on_play", False)
        )
        on_play.on_process_callback({
            "player": game.player1,
            "game": game,
            "permanent": perm,
            "is_dna_digivolve": False,
        })
        _resolve_with_first_non_decline(runner)

        field_ids = [p.top_card.card_id for p in game.player1.battle_area if p.top_card]
        assert BT3_088 in field_ids, (
            f"LadyDevimon (Fallen Angel Lv.5) should be played for free. Field: {field_ids}"
        )

    def test_plays_from_trash(self, debug_runner):
        """Card text explicitly allows 'from your hand or trash'."""
        runner = debug_runner(
            deck1=[EX6_029] * 4 + _deck_filler(46),
            deck2=_deck_filler(50),
            initial_memory=20,
        )
        runner.clear_zone(1, "hand")
        perm = runner.place_on_field(1, [EX6_029])
        runner.inject_card(1, BT1_055, "trash")
        game = runner.game
        game.memory = 0

        top = perm.top_card
        on_play = next(
            e for e in top.effect_list(None) if getattr(e, "is_on_play", False)
        )
        on_play.on_process_callback({
            "player": game.player1,
            "game": game,
            "permanent": perm,
            "is_dna_digivolve": False,
        })
        _resolve_with_first_non_decline(runner)

        field_ids = [p.top_card.card_id for p in game.player1.battle_area if p.top_card]
        assert BT1_055 in field_ids, (
            f"Angemon should be played from trash. Field: {field_ids}"
        )
        trash_ids = [c.card_id for c in game.player1.trash_cards]
        assert BT1_055 not in trash_ids, "Angemon should have left trash"

    def test_does_not_play_non_angel(self, debug_runner):
        """Agumon (no Angel/Archangel/Fallen Angel trait) must not be
        playable by this effect."""
        runner = debug_runner(
            deck1=[EX6_029] * 4 + [BT1_009] * 4 + _deck_filler(42),
            deck2=_deck_filler(50),
            initial_memory=20,
        )
        perm = runner.place_on_field(1, [EX6_029])
        runner.inject_card(1, BT1_009, "hand")
        game = runner.game
        game.memory = 0

        top = perm.top_card
        on_play = next(
            e for e in top.effect_list(None) if getattr(e, "is_on_play", False)
        )
        on_play.on_process_callback({
            "player": game.player1,
            "game": game,
            "permanent": perm,
            "is_dna_digivolve": False,
        })
        _resolve_with_first_non_decline(runner)

        field_ids = [p.top_card.card_id for p in game.player1.battle_area if p.top_card]
        assert BT1_009 not in field_ids, (
            f"Agumon (non-Angel) must NOT be played. Field: {field_ids}"
        )

    def test_does_not_play_lv6_or_higher(self, debug_runner):
        """Card text restricts to 'level 5 or lower Digimon card'. A Lv.6
        Angel (EX6-029 itself, or any Angel Lv.6) must not qualify."""
        runner = debug_runner(
            deck1=[EX6_029] * 8 + _deck_filler(42),
            deck2=_deck_filler(50),
            initial_memory=20,
        )
        perm = runner.place_on_field(1, [EX6_029])
        # Inject another EX6-029 into hand - same card, Lv.6 Angel
        runner.inject_card(1, EX6_029, "hand")
        game = runner.game
        game.memory = 0
        initial_field_count = len(game.player1.battle_area)

        top = perm.top_card
        on_play = next(
            e for e in top.effect_list(None) if getattr(e, "is_on_play", False)
        )
        on_play.on_process_callback({
            "player": game.player1,
            "game": game,
            "permanent": perm,
            "is_dna_digivolve": False,
        })
        _resolve_with_first_non_decline(runner)

        # No new permanent should appear (Lv.6 doesn't qualify; nothing else in hand)
        assert len(game.player1.battle_area) == initial_field_count, (
            f"Lv.6 Angel must not qualify. "
            f"Field grew from {initial_field_count} to {len(game.player1.battle_area)}"
        )


@pytest.mark.behavioral
class TestEX6029DnaDigivolveRider:
    """Clause: '...Then, if DNA digivolving, place 1 other Digimon at the
    bottom of its owner's security stack, and trash cards from the top of
    your opponent's security stack until it has 4 left.'

    Only fires if is_dna_digivolve=True in context, and only via the
    [When Digivolving] path (On Play can't come from DNA, so that path's
    check is a harmless no-op)."""

    def test_no_dna_rider_without_dna_flag(self, debug_runner):
        """When is_dna_digivolve=False, no security placement or trashing."""
        runner = debug_runner(
            deck1=[EX6_029] * 4 + _deck_filler(46),
            deck2=_deck_filler(50),
            initial_memory=10,
        )
        runner.place_on_field(2, [BT1_009])  # opponent Digimon for potential target
        # Ensure opponent has >4 security so we can detect trashing
        while len(runner.game.player2.security_cards) < 6:
            runner.game.player2.security_cards.append(
                runner.game.player2.library_cards.pop(0)
            )
        perm = runner.place_on_field(1, [EX6_029])
        game = runner.game

        sec_before = len(game.player2.security_cards)
        field_before = len(game.player2.battle_area)

        top = perm.top_card
        wd_effect = next(
            e for e in top.effect_list(None) if getattr(e, "is_when_digivolving", False)
        )
        wd_effect.on_process_callback({
            "player": game.player1,
            "game": game,
            "permanent": perm,
            "is_dna_digivolve": False,
        })
        _resolve_with_first_non_decline(runner)

        assert len(game.player2.security_cards) == sec_before, (
            "Opponent security must not be trashed when not DNA digivolving"
        )
        assert len(game.player2.battle_area) == field_before, (
            "No security placement when not DNA digivolving"
        )

    def test_dna_rider_trashes_opponent_security_down_to_4(self, debug_runner):
        """When DNA digivolving, trash from top of opponent's security
        until 4 cards remain."""
        runner = debug_runner(
            deck1=[EX6_029] * 4 + _deck_filler(46),
            deck2=_deck_filler(50),
            initial_memory=10,
        )
        perm = runner.place_on_field(1, [EX6_029])
        game = runner.game

        # Pad opponent's security to 6 cards
        while len(game.player2.security_cards) < 6:
            game.player2.security_cards.append(game.player2.library_cards.pop(0))
        assert len(game.player2.security_cards) == 6

        top = perm.top_card
        wd_effect = next(
            e for e in top.effect_list(None) if getattr(e, "is_when_digivolving", False)
        )
        wd_effect.on_process_callback({
            "player": game.player1,
            "game": game,
            "permanent": perm,
            "is_dna_digivolve": True,
        })
        _resolve_with_first_non_decline(runner)

        assert len(game.player2.security_cards) == 4, (
            f"Opponent security should be trimmed to 4. "
            f"Got {len(game.player2.security_cards)}"
        )

    def test_dna_rider_does_not_add_security_if_already_at_4(self, debug_runner):
        """If opponent has 4 or fewer security cards, no trashing (until
        it has 4 left => nothing to trash)."""
        runner = debug_runner(
            deck1=[EX6_029] * 4 + _deck_filler(46),
            deck2=_deck_filler(50),
            initial_memory=10,
        )
        perm = runner.place_on_field(1, [EX6_029])
        game = runner.game
        # Force opponent security to exactly 4
        while len(game.player2.security_cards) > 4:
            game.player2.library_cards.insert(
                0, game.player2.security_cards.pop(0)
            )
        assert len(game.player2.security_cards) == 4

        top = perm.top_card
        wd_effect = next(
            e for e in top.effect_list(None) if getattr(e, "is_when_digivolving", False)
        )
        wd_effect.on_process_callback({
            "player": game.player1,
            "game": game,
            "permanent": perm,
            "is_dna_digivolve": True,
        })
        _resolve_with_first_non_decline(runner)

        assert len(game.player2.security_cards) == 4, (
            f"Security should stay at 4, not grow or shrink. "
            f"Got {len(game.player2.security_cards)}"
        )

    def test_dna_rider_places_own_digimon_at_bottom_of_own_security(
        self, debug_runner
    ):
        """When DNA digivolving, the player may place 1 OTHER Digimon on
        the battle area at the bottom of its owner's security stack.
        Targeting an own Digimon: place it at bottom of own security."""
        runner = debug_runner(
            deck1=[EX6_029] * 4 + _deck_filler(46),
            deck2=_deck_filler(50),
            initial_memory=10,
        )
        perm = runner.place_on_field(1, [EX6_029])
        # Place a second own Digimon as target
        other = runner.place_on_field(1, [BT1_009])
        game = runner.game

        # Ensure opponent has exactly 4 security so trash step is a no-op
        while len(game.player2.security_cards) > 4:
            game.player2.library_cards.insert(
                0, game.player2.security_cards.pop(0)
            )
        while len(game.player2.security_cards) < 4:
            game.player2.security_cards.append(game.player2.library_cards.pop(0))

        own_sec_before = len(game.player1.security_cards)
        own_field_before = len(game.player1.battle_area)

        top = perm.top_card
        wd_effect = next(
            e for e in top.effect_list(None) if getattr(e, "is_when_digivolving", False)
        )
        wd_effect.on_process_callback({
            "player": game.player1,
            "game": game,
            "permanent": perm,
            "is_dna_digivolve": True,
        })
        _resolve_with_first_non_decline(runner)

        assert len(game.player1.battle_area) == own_field_before - 1, (
            "The selected other Digimon should have left our battle area"
        )
        assert len(game.player1.security_cards) == own_sec_before + 1, (
            "Our security should gain exactly 1 card from the placement"
        )
        # Placed at bottom: in security_cards list, bottom = end (.append)
        assert game.player1.security_cards[-1].card_id == BT1_009, (
            f"Agumon (BT1-009) should be at bottom of security. "
            f"Got {game.player1.security_cards[-1].card_id}"
        )

    def test_dna_rider_can_target_opponent_digimon(self, debug_runner):
        """C# CanSelectPermanentSharedCondition uses
        IsPermanentExistsOnBattleArea (no owner restriction). So the
        player may also choose an opponent's Digimon, and the place-under
        happens on the opponent's security stack."""
        runner = debug_runner(
            deck1=[EX6_029] * 4 + _deck_filler(46),
            deck2=_deck_filler(50),
            initial_memory=10,
        )
        perm = runner.place_on_field(1, [EX6_029])
        game = runner.game

        # Clear own battle area of other digimon so the ONLY choice is
        # opponent's Digimon (engine auto_resolve will pick the first
        # legal target).
        opp_target = runner.place_on_field(2, [BT1_009])

        # Trim opponent security to 3 (so no trash step needed) and
        # freeze it via positioning so we can detect that the placement
        # only moved 1 card (the digimon) to BOTTOM of security.
        while len(game.player2.security_cards) > 3:
            game.player2.library_cards.insert(
                0, game.player2.security_cards.pop(0)
            )
        while len(game.player2.security_cards) < 3:
            game.player2.security_cards.append(game.player2.library_cards.pop(0))

        opp_sec_before = len(game.player2.security_cards)
        opp_field_before = len(game.player2.battle_area)

        top = perm.top_card
        wd_effect = next(
            e for e in top.effect_list(None) if getattr(e, "is_when_digivolving", False)
        )
        wd_effect.on_process_callback({
            "player": game.player1,
            "game": game,
            "permanent": perm,
            "is_dna_digivolve": True,
        })
        _resolve_with_first_non_decline(runner)

        # The opponent Digimon should be gone from field
        assert len(game.player2.battle_area) == opp_field_before - 1, (
            f"Opponent's Agumon should have left the battle area. "
            f"Before: {opp_field_before}, After: {len(game.player2.battle_area)}"
        )
        # The opponent's security stack should have +1 (at the bottom)
        assert len(game.player2.security_cards) == opp_sec_before + 1, (
            f"Opponent's security stack should have grown by 1 (place at "
            f"bottom of its owner's security). Before: {opp_sec_before}, "
            f"After: {len(game.player2.security_cards)}"
        )
        assert game.player2.security_cards[-1].card_id == BT1_009, (
            "Agumon should be at the bottom of opponent's security stack"
        )

    def test_dna_rider_does_not_target_self(self, debug_runner):
        """The text 'place 1 other Digimon' excludes Mastemon itself."""
        runner = debug_runner(
            deck1=[EX6_029] * 4 + _deck_filler(46),
            deck2=_deck_filler(50),
            initial_memory=10,
        )
        perm = runner.place_on_field(1, [EX6_029])
        game = runner.game
        # No other Digimon on either field; opponent security = 4 (no trash)
        while len(game.player2.security_cards) > 4:
            game.player2.library_cards.insert(
                0, game.player2.security_cards.pop(0)
            )
        while len(game.player2.security_cards) < 4:
            game.player2.security_cards.append(game.player2.library_cards.pop(0))

        own_field_before = len(game.player1.battle_area)
        assert own_field_before == 1  # only Mastemon

        top = perm.top_card
        wd_effect = next(
            e for e in top.effect_list(None) if getattr(e, "is_when_digivolving", False)
        )
        wd_effect.on_process_callback({
            "player": game.player1,
            "game": game,
            "permanent": perm,
            "is_dna_digivolve": True,
        })
        _resolve_with_first_non_decline(runner)

        # Mastemon should still be on the field (it can't target itself)
        assert len(game.player1.battle_area) == own_field_before, (
            f"Mastemon must NOT target itself. Field count changed: "
            f"{own_field_before} -> {len(game.player1.battle_area)}"
        )
        assert perm in game.player1.battle_area, "Mastemon should remain"

    def test_dna_rider_does_not_target_tamers(self, debug_runner):
        """C# CanSelectPermanentSharedCondition checks `permanent.IsDigimon`.
        Tamers must NOT be selectable as targets for this effect."""
        runner = debug_runner(
            deck1=[EX6_029] * 4 + _deck_filler(46),
            deck2=_deck_filler(50),
            initial_memory=10,
        )
        perm = runner.place_on_field(1, [EX6_029])
        # Place a Tamer - BT1-085 Tai Kamiya
        tamer_perm = runner.place_on_field(1, ["BT1-085"])
        assert tamer_perm.is_tamer, "BT1-085 should be a Tamer"
        game = runner.game
        # Freeze opponent security at 4
        while len(game.player2.security_cards) > 4:
            game.player2.library_cards.insert(
                0, game.player2.security_cards.pop(0)
            )
        while len(game.player2.security_cards) < 4:
            game.player2.security_cards.append(game.player2.library_cards.pop(0))

        own_field_before = len(game.player1.battle_area)

        top = perm.top_card
        wd_effect = next(
            e for e in top.effect_list(None) if getattr(e, "is_when_digivolving", False)
        )
        wd_effect.on_process_callback({
            "player": game.player1,
            "game": game,
            "permanent": perm,
            "is_dna_digivolve": True,
        })
        _resolve_with_first_non_decline(runner)

        # The Tamer should still be on the field (it's not a valid target)
        assert tamer_perm in game.player1.battle_area, (
            "Tamer must not be selectable as the 'other Digimon' target"
        )
        # Field count unchanged (Mastemon + Tamer)
        assert len(game.player1.battle_area) == own_field_before, (
            "No permanent should have been moved since only a Tamer was "
            "available as an 'other' target"
        )


@pytest.mark.behavioral
class TestEX6029AceOverflow:
    """Clause (inherited): Ace Overflow <-4>.

    Engine handles this natively via cards.json parsing. The script may
    or may not also register a descriptive effect entry. This test only
    verifies the card data has the correct Ace Overflow cost."""

    def test_ace_overflow_cost_is_4(self, debug_runner):
        """EX6-029 should report Ace Overflow <-4> via its entity_base."""
        runner = debug_runner(
            deck1=[EX6_029] * 4 + _deck_filler(46),
            deck2=_deck_filler(50),
        )
        cs = runner.inject_card(1, EX6_029, "hand")
        assert cs.c_entity_base.is_ace, (
            "EX6-029 must be flagged as an Ace card"
        )
        assert cs.c_entity_base.ace_overflow_cost == 4, (
            f"EX6-029 Ace Overflow cost should be 4. "
            f"Got {cs.c_entity_base.ace_overflow_cost}"
        )
