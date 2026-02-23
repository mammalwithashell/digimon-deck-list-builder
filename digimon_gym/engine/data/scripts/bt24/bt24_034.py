from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_034(CardScript):
    """BT24-034 Aegiomon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req (branch 1: [Elecmon] name)
        # [Digivolve] [Elecmon]/Lv.3 w/[TS] trait: Cost 2
        effect0a = ICardEffect()
        effect0a.set_effect_name("BT24-034 Alternate digivolution requirement (Elecmon)")
        effect0a.set_effect_description("Alternate digivolution requirement")
        effect0a._alt_digi_cost = 2
        effect0a._alt_digi_name = "Elecmon"

        def condition0a(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.contains_card_name('Elecmon')):
                return False
            return True
        effect0a.set_can_use_condition(condition0a)
        effects.append(effect0a)

        # Factory effect: alt_digivolve_req (branch 2: Lv.3 w/[TS] trait)
        effect0b = ICardEffect()
        effect0b.set_effect_name("BT24-034 Alternate digivolution requirement (TS)")
        effect0b.set_effect_description("Alternate digivolution requirement")
        effect0b._alt_digi_cost = 2
        effect0b._alt_digi_level = 3
        effect0b._alt_digi_trait = "TS"

        def condition0b(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('TS' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0b.set_can_use_condition(condition0b)
        effects.append(effect0b)

        # Factory effect: barrier
        # Barrier
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-034 Barrier")
        effect1.set_effect_description("Barrier")
        effect1._is_barrier = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Shared process for When Moving / On Play / When Digivolving:
        # By adding your top security card to the hand, you may play 1 [TS]
        # trait Tamer card from your hand without paying the cost.
        # This effect can't play cards with the same name as any of your Tamers.
        def _shared_process(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Must have security to activate
            if not player.security_cards:
                return
            # Add top security card to hand (cost of effect)
            top_sec = player.security_cards.pop(0)
            player.hand_cards.append(top_sec)
            # Collect names of Tamers already on field
            tamer_names = set()
            for p in player.battle_area:
                if p.is_tamer and p.top_card:
                    for name in (getattr(p.top_card, 'card_names', []) or [getattr(p.top_card, 'card_name', '')]):
                        if name:
                            tamer_names.add(name.lower())
            # Filter: [TS] trait Tamer not sharing name with existing Tamers
            def play_filter(c):
                if not getattr(c, 'is_tamer', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                if not any('TS' in t for t in traits):
                    return False
                # Name restriction
                c_names = getattr(c, 'card_names', []) or [getattr(c, 'card_name', '')]
                for n in c_names:
                    if n and n.lower() in tamer_names:
                        return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        # Timing: EffectTiming.OnMove
        # [When Moving]
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-034 Add sec to hand, play TS Tamer")
        effect2.set_effect_description("[When Moving] By adding your top security card to the hand, you may play 1 [TS] trait Tamer card from your hand without paying the cost.")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not (player and player.security_cards):
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_shared_process)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play]
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-034 Add sec to hand, play TS Tamer")
        effect3.set_effect_description("[On Play] By adding your top security card to the hand, you may play 1 [TS] trait Tamer card from your hand without paying the cost.")
        effect3.is_on_play = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not (player and player.security_cards):
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_shared_process)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving]
        effect4 = ICardEffect()
        effect4.set_effect_name("BT24-034 Add sec to hand, play TS Tamer")
        effect4.set_effect_description("[When Digivolving] By adding your top security card to the hand, you may play 1 [TS] trait Tamer card from your hand without paying the cost.")
        effect4.is_when_digivolving = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not (player and player.security_cards):
                return False
            return True

        effect4.set_can_use_condition(condition4)
        effect4.set_on_process_callback(_shared_process)
        effects.append(effect4)

        # Factory effect: barrier (inherited)
        # Barrier
        effect5 = ICardEffect()
        effect5.set_effect_name("BT24-034 Barrier")
        effect5.set_effect_description("Barrier")
        effect5.is_inherited_effect = True
        effect5._is_barrier = True

        def condition5(context: Dict[str, Any]) -> bool:
            return True
        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        return effects
