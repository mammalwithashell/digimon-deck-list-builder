from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST14_11(CardScript):
    """ST14-11 Ai & Mako | Tamer

    [On Play] Reveal the top 4 cards of your deck. Add 1 Digimon card with
        the [Evil], [Wizard] or [Demon Lord] trait among them to your hand.
        Place the rest at the bottom of your deck in any order.
    [Your Turn] When one of your Digimon digivolves into a purple Digimon,
        by suspending this Tamer, return 1 card from your hand to the top of
        your deck and gain 1 memory.
    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [On Play] Reveal top 4, add 1 Digimon with Evil/Wizard/Demon Lord trait
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("ST14-11 On Play: reveal top 4, add 1 to hand")
        effect0.set_effect_description(
            "[On Play] Reveal the top 4 cards of your deck. Add 1 Digimon card "
            "with the [Evil], [Wizard] or [Demon Lord] trait among them to your "
            "hand. Place the rest at the bottom of your deck in any order."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def reveal_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any(
                    'Evil' in t or 'Wizard' in t or 'Demon Lord' in t
                    for t in traits
                )

            game.effect_reveal_and_select_multi(
                player, 4, [(reveal_filter, 'hand')],
                remaining_placement='deck_bottom', is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [Your Turn] When one of your Digimon digivolves into a purple Digimon,
        # by suspending this Tamer, return 1 card from hand to top of deck, gain 1 memory
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("ST14-11 When digivolve into purple: suspend, return hand card, +1 memory")
        effect1.set_effect_description(
            "[Your Turn] When one of your Digimon digivolves into a purple Digimon, "
            "by suspending this Tamer, return 1 card from your hand to the top of "
            "your deck and gain 1 memory."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            perm = card.permanent_of_this_card()
            if not perm or perm.is_suspended:
                return False
            # Check if event is a digivolve into purple
            digivolved = context.get('digivolved_permanent')
            if not digivolved:
                return False
            if not digivolved.is_digimon:
                return False
            colors = [col.name for col in (getattr(digivolved.top_card, 'card_colors', []) or [])] if digivolved.top_card else []
            if 'Purple' not in colors:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return
            # Cost: suspend this Tamer
            perm.suspend()
            if not player.hand_cards:
                return

            # Return 1 card from hand to top of deck
            def hand_filter(c):
                return True

            def on_return(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.library_cards.insert(0, selected)
                player.add_memory(1)

            game.effect_select_hand_card(
                player, hand_filter, on_return, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [Security] Play this card without paying the cost
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("ST14-11 Security: play this card")
        effect2.set_effect_description("[Security] Play this card without paying the cost.")
        effect2.is_security_effect = True
        effect2._security_play_self = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player and card:
                if card in player.trash_cards:
                    player.trash_cards.remove(card)
                player.play_card_from_source(card, pay_cost=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
