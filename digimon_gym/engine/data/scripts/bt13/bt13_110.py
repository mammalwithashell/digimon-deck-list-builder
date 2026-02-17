from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_110(CardScript):
    """BT13-110 Royal Knights of the Purge"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] <Draw 1>. You may place 1 Digimon card from your hand as the bottom digivolution card of 1 of your [King Drasil_7D6] in the breeding area. Then, place this card in the battle area.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-110 Draw 1, Trash From Hand")
        effect0.set_effect_description("[Main] <Draw 1>. You may place 1 Digimon card from your hand as the bottom digivolution card of 1 of your [King Drasil_7D6] in the breeding area. Then, place this card in the battle area.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 1, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)
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

        # Factory effect: delay
        # Delay
        effect1 = ICardEffect()
        effect1.set_effect_name("BT13-110 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Royal Knight' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDeclaration
        # [Main] <Delay> - Play 1 [Royal Knight] trait card from the digivolution cards of your Digimon in the breeding area without paying the cost. [On Play] effects on Digimon played by this effect don't activate, and they gain <Rush> for the turn.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT13-110 Play 1 Digimon from breeding area")
        effect2.set_effect_description("[Main] <Delay> - Play 1 [Royal Knight] trait card from the digivolution cards of your Digimon in the breeding area without paying the cost. [On Play] effects on Digimon played by this effect don't activate, and they gain <Rush> for the turn.")
        effect2._is_rush = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Card, Gain Keyword Rush"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not (any('Royal Knight' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            if perm:
                perm.grant_keyword('_is_rush')

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
