from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_102(CardScript):
    """BT23-102 Mastemon | Lv.6

    Card text:
    <Barrier>
    <Partition ([Angewomon] & [LadyDevimon])>
    [When Digivolving] You may play 1 level 5 or lower yellow or purple card
    from your hand or trash without paying the cost. Then, if this Digimon's
    stack has 2 or more same-level cards, trash the top cards of both players'
    security stacks so that they have 3 cards left.
    [All Turns] [Once Per Turn] When security stacks are removed from, you
    may place 1 Digimon as the bottom security card.

    Additional xros_req:
    - [Digivolve] Lv.5 w/[CS] trait: Cost 5 (alt-digi)
    - [DNA Digivolve] Yellow Lv.5 + Purple Lv.5: Cost 0 (engine-parsed from xros_req)
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ─── Alternate digivolution: Lv.5 w/[CS] trait for cost 5 ─────
        # Matches C# AddSelfDigivolutionRequirementStaticEffect:
        # PermanentCondition = TopCard.IsLevel5 && TopCard.HasCSTraits
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-102 Alternate digivolution requirement")
        effect0.set_effect_description("[Digivolve] Lv.5 w/[CS] trait: Cost 5")
        effect0._alt_digi_cost = 5
        effect0._alt_digi_level = 5
        effect0._alt_digi_trait = "CS"

        def condition0(context: Dict[str, Any]) -> bool:
            # Engine digivolve_validator does not call can_use_condition for
            # alt-digi. The _alt_digi_* attributes encode all constraints.
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # ─── DNA Digivolve: Yellow Lv.5 + Purple Lv.5 cost 0 ─────────
        # Engine auto-parses this from c_entity_base.xros_req → dna_costs.
        # This placeholder keeps the effect list complete for documentation.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-102 DNA Digivolve (Yellow Lv.5 + Purple Lv.5)")
        effect1.set_effect_description(
            "[DNA Digivolve] Yellow Lv.5 + Purple Lv.5: Cost 0"
        )

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # ─── Barrier keyword ─────────────────────────────────────────
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-102 Barrier")
        effect2.set_effect_description(
            "<Barrier> (When this Digimon would be deleted in battle, by "
            "trashing the top card of your security stack, prevent that deletion.)"
        )
        effect2._is_barrier = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # ─── Partition ([Angewomon] & [LadyDevimon]) ─────────────────
        # Engine does not currently implement Partition recovery logic.
        # The _is_partition flag is a descriptive marker. The C# reference
        # uses CardEffectFactory.PartitionSelfEffect with
        # partitionConditions = {"Angewomon", "LadyDevimon"}.
        # ENGINE GAP: Partition recovery is a flag-only implementation —
        # consistent with all other partition scripts (BT16-012, BT16-025,
        # BT16-036, BT16-063, BT16-077, BT23-047, etc.).
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-102 Partition")
        effect3.set_effect_description(
            "<Partition ([Angewomon] & [LadyDevimon])>"
        )
        effect3._is_partition = True
        effect3._partition_names = ["Angewomon", "LadyDevimon"]

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # ─── [When Digivolving] effect ───────────────────────────────
        # Part 1: play 1 Lv.5 or lower yellow/purple from hand or trash
        # Part 2: if stack has 2+ same-level cards, trash top security cards of
        #         BOTH players' security stacks so each has 3 cards remaining.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name(
            "BT23-102 Play 1 Lv.5 or lower yellow/purple card; trash securities to 3"
        )
        effect4.set_effect_description(
            "[When Digivolving] You may play 1 level 5 or lower yellow or "
            "purple card from your hand or trash without paying the cost. "
            "Then, if this Digimon's stack has 2 or more same-level cards, "
            "trash the top cards of both players' security stacks so that "
            "they have 3 cards left."
        )
        effect4.is_when_digivolving = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4)

        def _has_two_same_level_in_stack(perm) -> bool:
            """Check whether the stack contains 2+ cards sharing a level.

            Mirrors C#: permanent.StackCards.Filter(x => !x.IsFlipped)
                       .GroupBy(x => x.Level).Any(g => g.Count() >= 2)
            We approximate IsFlipped == False by including all card_sources.
            """
            if perm is None:
                return False
            levels: Dict[int, int] = {}
            for cs in perm.card_sources:
                lv = getattr(cs, 'level', None)
                if lv is None:
                    continue
                levels[lv] = levels.get(lv, 0) + 1
            return any(count >= 2 for count in levels.values())

        def _trash_security_down_to(game, player, floor: int):
            """Trash top security cards of `player` until they have `floor` left."""
            while len(player.security_cards) > floor:
                trashed = player.security_cards.pop(0)  # index 0 = top
                player.face_up_security.discard(trashed)
                player.trash_cards.append(trashed)

        def process4(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            # Part 1: play 1 Lv.5 or lower yellow/purple from hand or trash
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    # C# uses CanPlayAsNewPermanent; simplest faithful
                    # interpretation is Digimon cards (level-bearing cards).
                    return False
                lv = getattr(c, 'level', None)
                if lv is None or lv > 5:
                    return False
                colors = [col.name for col in (getattr(c, 'card_colors', None) or [])]
                return 'Yellow' in colors or 'Purple' in colors

            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter,
                free=True, is_optional=True,
                prompt="Select 1 level 5 or lower yellow/purple card to play (or decline).",
            )

            # Part 2: if stack has 2+ same-level cards, trash top securities of
            # both players down to 3. The C# fires this unconditionally after
            # the (optional) Part-1 selection, iterating over both players.
            if _has_two_same_level_in_stack(perm):
                enemy = player.enemy if player else None
                _trash_security_down_to(game, player, 3)
                if enemy is not None:
                    _trash_security_down_to(game, enemy, 3)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # ─── [All Turns] [Once Per Turn] OnLoseSecurity ──────────────
        # "When security stacks are removed from, you may place 1 Digimon
        # as the bottom security card."
        # Per C# CanTriggerWhenLoseSecurity(hashtable, null) — fires on EITHER
        # player's security loss. The card owner places one of their own
        # Digimon as the bottom of their own security stack. put_permanent_to_security
        # calls self.security_cards.append(card) which places at bottom (index -1)
        # since index 0 is the top of the stack.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnLoseSecurity)
        effect5.set_effect_name("BT23-102 Place 1 Digimon as bottom security")
        effect5.set_effect_description(
            "[All Turns] [Once Per Turn] When security stacks are removed "
            "from, you may place 1 Digimon as the bottom security card."
        )
        effect5.is_optional = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("BT23_102_AT")

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                return p.is_digimon

            def on_put_security(target_perm):
                if player:
                    player.put_permanent_to_security(target_perm)

            game.effect_select_own_permanent(
                player, on_put_security,
                filter_fn=target_filter,
                is_optional=True,
                prompt="Select 1 of your Digimon to place as the bottom security card.",
            )

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
