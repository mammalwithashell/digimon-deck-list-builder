from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_094(CardScript):
    """BT20-094 Emperor Dragon of Calamity"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] You may play 1 [Free] trait Digimon card from your trash with the play cost reduced by 5. Then, place this card in the battle area.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-094 Play 1 [Free] trait Digimon card from your trash with the play cost reduced by 5.")
        effect0.set_effect_description("[Main] You may play 1 [Free] trait Digimon card from your trash with the play cost reduced by 5. Then, place this card in the battle area.")
        effect0.cost_reduction = 5

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Cost -5, Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            # Cost reduction handled via cost_reduction property

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnLoseSecurity
        # [All Turns] When your opponent's security stack is removed from, <Delay>.\r\n� You may play 1 [Imperialdramon: Dragon Mode] from any of your [Imperialdramon: Fighter Mode]'s digivolution cards without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-094 You may play 1 [Imperialdramon: Dragon Mode] from any of your [Imperialdramon: Fighter Mode]'s digivolution cards without paying the cost.")
        effect1.set_effect_description("[All Turns] When your opponent's security stack is removed from, <Delay>.\r\n� You may play 1 [Imperialdramon: Dragon Mode] from any of your [Imperialdramon: Fighter Mode]'s digivolution cards without paying the cost.")
        effect1.is_optional = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Imperialdramon: Fighter Mode'))):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.SecuritySkill
        # [Security] You may play 1 level 3 Digimon card with the [Free] trait from your hand or trash without paying the cost. Then, add this card to the hand.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-094 Play Card, Trash From Hand, Add To Hand")
        effect2.set_effect_description("[Security] You may play 1 level 3 Digimon card with the [Free] trait from your hand or trash without paying the cost. Then, add this card to the hand.")
        effect2.is_security_effect = True
        effect2.is_security_effect = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Card, Trash From Hand, Add To Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not (any('Free' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            if not (player and game):
                return
            def hand_filter(c):
                if not (any('Free' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
