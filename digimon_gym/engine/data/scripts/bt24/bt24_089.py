from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_089(CardScript):
    """BT24-089 Unique Emblem: Blazing Conductor"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] You may play 1 [Elizamon] or [Owen Dreadnought] from your hand or trash without paying the cost. Then, place this card in the battle area.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-089 Play 1 [Elizamon]/[Owen Dreadnought] from hand or trash, then place in battle area")
        effect0.set_effect_description("[Main] You may play 1 [Elizamon] or [Owen Dreadnought] from your hand or trash without paying the cost. Then, place this card in the battle area.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Card, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            if not (player and game):
                return
            def hand_filter(c):
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnTappedAnyone
        # [Your Turn] When any of your [Owen Dreadnought] suspend, <Delay> (By trashing this card after the placing turn, activate the effect below.)\r\n・1 of your [Dragonkin] or [Reptile] trait Digimon may digivolve into a [Dragonkin] or [Reptile] and [LIBERATOR] trait Digimon card in the hand with the digivolution cost reduced by 3.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-089 1 of your [Dragonkin] or [Reptile] trait Digimon may digivolve")
        effect1.set_effect_description("[Your Turn] When any of your [Owen Dreadnought] suspend, <Delay> (By trashing this card after the placing turn, activate the effect below.)\r\n・1 of your [Dragonkin] or [Reptile] trait Digimon may digivolve into a [Dragonkin] or [Reptile] and [LIBERATOR] trait Digimon card in the hand with the digivolution cost reduced by 3.")
        effect1.is_optional = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Owen Dreadnought'))):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
