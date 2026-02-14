from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_072(CardScript):
    """BT23-072"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDeclaration
        # [Hand][Main] By paying 3 cost and placing this card as the bottom digivolution card of your [King Drasil_7D6] or [Mother Eater] in the breeding area, <Draw 1>.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-072 <Draw 1>")
        effect0.set_effect_description("[Hand][Main] By paying 3 cost and placing this card as the bottom digivolution card of your [King Drasil_7D6] or [Mother Eater] in the breeding area, <Draw 1>.")
        effect0.is_optional = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('King Drasil_7D6') or permanent.contains_card_name('Mother Eater'))):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [All Turns] When any of your Digimon with the [Royal Knight] or [CS] trait are played, by suspending this Digimon, 1 of the played Digimon gains <Rush>, <Raid>, <Reboot> and <Blocker> until your opponent's turn ends.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-072 By suspending this digimon, 1 played digimon gains <Rush>, <Raid>, <Reboot> and <Blocker>")
        effect1.set_effect_description("[All Turns] When any of your Digimon with the [Royal Knight] or [CS] trait are played, by suspending this Digimon, 1 of the played Digimon gains <Rush>, <Raid>, <Reboot> and <Blocker> until your opponent's turn ends.")
        effect1.is_optional = True
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if not (any('Royal Knight' in t for t in (getattr(p.top_card, 'card_traits', []) or []))):
                    return False
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnStartMainPhase
        # [Breeding] [Start of Your Main Phase] If this Digimon has 6 or more digivolution cards, you may play 1 Digimon card with [King Drasil] in its name from its digivolution cards without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-072 Play 1 [King Drasil] from sources")
        effect2.set_effect_description("[Breeding] [Start of Your Main Phase] If this Digimon has 6 or more digivolution cards, you may play 1 Digimon card with [King Drasil] in its name from its digivolution cards without paying the cost.")
        effect2.is_inherited_effect = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and len(permanent.digivolution_cards) >= 6):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not (any('King Drasil' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
