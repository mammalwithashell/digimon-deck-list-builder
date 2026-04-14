from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_030(CardScript):
    """BT24-030 Neptunemon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution: Lv.5 with [Aqua] or [Sea Animal] in any trait, or [TS] trait: Cost 3
        # C# checks: targetPermanent.TopCard.HasAquaTraits || targetPermanent.TopCard.HasTSTraits
        # The engine's _alt_digi_trait only supports a single trait string; to support multiple trait
        # alternatives we register two separate alt-digi effects.
        effect0a = ICardEffect()
        effect0a.set_effect_name("BT24-030 Alternate digivolution requirement (TS)")
        effect0a.set_effect_description("Alternate digivolution requirement: Lv.5 with [TS] trait: Cost 3")
        effect0a._alt_digi_cost = 3
        effect0a._alt_digi_level = 5
        effect0a._alt_digi_trait = "TS"

        def condition0a(context: Dict[str, Any]) -> bool:
            return True
        effect0a.set_can_use_condition(condition0a)
        effects.append(effect0a)

        effect0b = ICardEffect()
        effect0b.set_effect_name("BT24-030 Alternate digivolution requirement (Aqua)")
        effect0b.set_effect_description("Alternate digivolution requirement: Lv.5 with [Aqua] in any trait: Cost 3")
        effect0b._alt_digi_cost = 3
        effect0b._alt_digi_level = 5
        effect0b._alt_digi_trait = "Aqua"

        def condition0b(context: Dict[str, Any]) -> bool:
            return True
        effect0b.set_can_use_condition(condition0b)
        effects.append(effect0b)

        effect0c = ICardEffect()
        effect0c.set_effect_name("BT24-030 Alternate digivolution requirement (Sea Animal)")
        effect0c.set_effect_description("Alternate digivolution requirement: Lv.5 with [Sea Animal] in any trait: Cost 3")
        effect0c._alt_digi_cost = 3
        effect0c._alt_digi_level = 5
        effect0c._alt_digi_trait = "Sea Animal"

        def condition0c(context: Dict[str, Any]) -> bool:
            return True
        effect0c.set_can_use_condition(condition0c)
        effects.append(effect0c)

        # Timing: EffectTiming.BeforePayCost
        # When this card would be played, if your opponent has 2 or more Digimon, reduce the play cost by 5.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.BeforePayCost)
        effect1.set_effect_name("BT24-030 Reduce play cost (5)")
        effect1.set_effect_description("When this card would be played, if your opponent has 2 or more Digimon, reduce the play cost by 5.")
        effect1.cost_reduction = 5

        def condition1(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False
            owner = getattr(card, 'owner', None)
            if not owner:
                return False
            enemy = owner.enemy if owner else None
            if not enemy:
                return False
            return len([p for p in enemy.battle_area if p.is_digimon]) >= 2

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Shared logic: Return all of opponent's Digimon with the fewest digivolution cards to bottom of deck.
        # Digivolution card count = card_sources length - 1 (top card is not a digivolution card).
        def _neptunemon_bottom_deck(player, game):
            """Return all opponent's Digimon with fewest digivolution cards to bottom of deck."""
            enemy = player.enemy if player else None
            if not enemy:
                return
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
            if not opp_digimon:
                return
            # Digivolution cards = card_sources minus the top card
            def digi_card_count(p):
                return max(0, len(p.card_sources) - 1)
            min_sources = min(digi_card_count(p) for p in opp_digimon)
            to_remove = [p for p in opp_digimon if digi_card_count(p) == min_sources]
            for target in list(to_remove):
                if target in enemy.battle_area:
                    enemy.return_permanent_to_deck_bottom(target)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Return all of your opponent's Digimon with the fewest digivolution cards to the bottom of the deck.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT24-030 Bottom deck all opponent digimon with lowest digivolution cards")
        effect3.set_effect_description("[On Play] Return all of your opponent's Digimon with the fewest digivolution cards to the bottom of the deck.")
        effect3.is_on_play = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game:
                _neptunemon_bottom_deck(player, game)

        effect3.set_on_process_callback(process3)
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Return all of your opponent's Digimon with the fewest digivolution cards to the bottom of the deck.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT24-030 Bottom deck all opponent digimon with lowest digivolution cards")
        effect4.set_effect_description("[When Digivolving] Return all of your opponent's Digimon with the fewest digivolution cards to the bottom of the deck.")
        effect4.is_when_digivolving = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        def process4(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game:
                _neptunemon_bottom_deck(player, game)

        effect4.set_on_process_callback(process4)
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Timing: EffectTiming.OnTappedAnyone
        # [All Turns] [Once Per Turn] When this Digimon suspends, it may unsuspend.
        # Must check that the suspended permanent is THIS Digimon (Pattern 5).
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnTappedAnyone)
        effect5.set_effect_name("BT24-030 Unsuspend this digimon")
        effect5.set_effect_description("[All Turns] [Once Per Turn] When this Digimon suspends, it may unsuspend.")
        effect5.is_optional = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("BT24_030_AT")

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Only fires when THIS Digimon is the one that was suspended
            owner_perm = card.permanent_of_this_card()
            ctx_perm = context.get('permanent')
            if owner_perm and ctx_perm and ctx_perm is not owner_perm:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Unsuspend this Digimon"""
            owner_perm = card.permanent_of_this_card()
            if owner_perm:
                owner_perm.unsuspend()

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # Timing: EffectTiming.WhenPermanentWouldBeDeleted
        # [All Turns] When any of your Digimon with the [TS] trait or [Aqua] or [Sea Animal] in any of their
        # traits would leave the battle area by your opponent's effects, by suspending this Digimon, they don't leave.
        # Pattern: WhenPermanentWouldBeDeleted + _will_not_be_removed flag (matches BT23-058 Craniamon)
        effect6 = ICardEffect()
        effect6.set_timing(EffectTiming.WhenPermanentWouldBeDeleted)
        effect6.set_effect_name("BT24-030 By suspending this digimon, your [TS]/[Aqua]/[Sea Animal] digimon wont leave the field")
        effect6.set_effect_description("[All Turns] When any of your Digimon with the [TS] trait or [Aqua] or [Sea Animal] in any of their traits would leave the battle area by your opponent's effects, by suspending this Digimon, they don't leave.")
        effect6.is_optional = True

        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Cost: must be able to suspend this Digimon (not already suspended)
            owner_perm = card.permanent_of_this_card()
            if owner_perm and owner_perm.is_suspended:
                return False
            # The leaving permanent (event_permanent from execute_effects context mapping)
            leaving_perm = context.get('event_permanent')
            if leaving_perm is None:
                return False
            # The leaving permanent must belong to this card's owner
            owner = getattr(card, 'owner', None)
            event_player = context.get('event_player')
            if not owner or not event_player or event_player is not owner:
                return False
            # Must be a Digimon with TS, Aqua, or Sea Animal trait
            if not getattr(leaving_perm, 'is_digimon', False):
                return False
            if not (leaving_perm.has_trait('TS') or leaving_perm.has_trait('Aqua') or leaving_perm.has_trait('Sea Animal')):
                return False
            # Must be by opponent's effect
            if not context.get('is_opponent_effect', False):
                return False
            return True

        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            """Action: Suspend THIS Digimon to protect the leaving Digimon."""
            owner_perm = card.permanent_of_this_card()
            if not owner_perm:
                return
            # Cost: suspend this Digimon
            owner_perm.suspend()
            # Prevent the leaving permanent from being removed (DCGO: willBeRemoveField = false)
            leaving_perm = ctx.get('event_permanent')
            if leaving_perm:
                leaving_perm._will_not_be_removed = True

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        return effects
