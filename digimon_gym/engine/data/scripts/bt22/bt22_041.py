from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_041(CardScript):
    """BT22-041 Kentaurosmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution: Lv.5 from [Chirinmon] (or CS trait Lv.5) for cost 3
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-041 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5
        effect0._alt_digi_name = "Chirinmon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not permanent:
                return False
            top = getattr(permanent, 'top_card', None)
            if top is None:
                return False
            # Chirinmon or CS-trait Lv.5
            if permanent.contains_card_name('Chirinmon'):
                return True
            traits = getattr(top, 'card_traits', []) or []
            if any('CS' in t for t in traits) and getattr(top, 'level', None) == 5:
                return True
            return False

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.BeforePayCost
        # When this card would be played, if there are 6 or fewer total cards in both players'
        # security stacks, reduce the play cost by 6.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.BeforePayCost)
        effect1.set_effect_name("BT22-041 Play Cost -6 when total security <= 6")
        effect1.set_effect_description("When this card would be played, if there are 6 or fewer total cards in both players' security stacks, reduce the play cost by 6.")
        effect1.cost_reduction = 6

        def condition1(context: Dict[str, Any]) -> bool:
            # Only applies when THIS card is being played
            target_card = context.get('card_source') or context.get('card_to_play')
            if target_card is not None and target_card is not card:
                return False
            owner = getattr(card, 'owner', None)
            if not owner or not owner.enemy:
                return False
            total_security = len(owner.security_cards) + len(owner.enemy.security_cards)
            return total_security <= 6

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Cost reduction — handled via cost_reduction property"""
            pass  # descriptive-tagged: cost_reduction

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: blocker
        effect2 = ICardEffect()
        effect2.set_effect_name("BT22-041 Blocker")
        effect2.set_effect_description("Blocker")
        effect2._is_blocker = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: barrier
        effect3 = ICardEffect()
        effect3.set_effect_name("BT22-041 Barrier")
        effect3.set_effect_description("Barrier")
        effect3._is_barrier = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        def _shared_add_yellow_to_security(ctx: Dict[str, Any]):
            """Select 1 yellow card from hand and place it as the top security card."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            def yellow_filter(c):
                colors = getattr(c, 'card_colors', [])
                return 'Yellow' in [col.name for col in colors]
            if not any(yellow_filter(c) for c in player.hand_cards):
                return
            def on_select(selected):
                player.add_to_security_from_hand(selected, to_top=True)
            game.effect_select_hand_card(
                player,
                yellow_filter,
                on_select,
                is_optional=True,
                prompt="Select 1 yellow card from your hand to place as your top security card.",
            )

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may place 1 yellow card from your hand as your top security card.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT22-041 Place 1 yellow card from hand as top security card")
        effect4.set_effect_description("[On Play] You may place 1 yellow card from your hand as your top security card.")
        effect4.is_optional = True
        effect4.is_on_play = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            _shared_add_yellow_to_security(ctx)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may place 1 yellow card from your hand as your top security card.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect5.set_effect_name("BT22-041 Place 1 yellow card from hand as top security card")
        effect5.set_effect_description("[When Digivolving] You may place 1 yellow card from your hand as your top security card.")
        effect5.is_optional = True
        effect5.is_when_digivolving = True

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            _shared_add_yellow_to_security(ctx)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # Timing: EffectTiming.OnTappedAnyone
        # [All Turns] [Once Per Turn] When THIS Digimon suspends, by trashing your top security
        # card, it unsuspends.
        # Cost: trash top security. Effect: unsuspend this permanent.
        effect6 = ICardEffect()
        effect6.set_timing(EffectTiming.OnTappedAnyone)
        effect6.set_effect_name("BT22-041 By trashing top security, unsuspend")
        effect6.set_effect_description("[All Turns] [Once Per Turn] When this Digimon suspends, by trashing your top security card, it unsuspends.")
        effect6.is_optional = True
        effect6.set_max_count_per_turn(1)
        effect6.set_hash_string("BT22_041_Untap")

        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Only fire when THIS permanent suspended
            suspended_perm = context.get('permanent')
            my_perm = card.permanent_of_this_card() if card else None
            if suspended_perm is not None and my_perm is not None and suspended_perm is not my_perm:
                return False
            # Must have security to pay the cost
            owner = card.owner if card else None
            if not owner or not owner.security_cards:
                return False
            return True

        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            """Action: Trash own top security card (cost), then unsuspend this Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Cost: trash the top security card
            if not player.security_cards:
                return
            top_security = player.security_cards[-1]
            player.trash_security_card(top_security)
            # Effect: unsuspend this Digimon
            my_perm = card.permanent_of_this_card() if card else None
            if my_perm and my_perm.is_suspended:
                my_perm.is_suspended = False  # Direct unsuspend bypassing CANNOT_UNSUSPEND check

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        return effects
