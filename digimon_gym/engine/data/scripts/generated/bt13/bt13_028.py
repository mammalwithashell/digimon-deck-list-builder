from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_028(CardScript):
    """BT13-028 Thetismon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDeclaration
        # [Hand][Main] If you have [Kiyoshiro Higashimitarai], by placing 1 [TeslaJellymon] from your hand as 1 of your [Jellymon]'s bottom digivolution card, that Digimon digivolves into this card for a digivolution cost of 3, ignoring digivolution requirements.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-028 Your 1 [Jellymon] digivolves into this card")
        effect0.set_effect_description("[Hand][Main] If you have [Kiyoshiro Higashimitarai], by placing 1 [TeslaJellymon] from your hand as 1 of your [Jellymon]'s bottom digivolution card, that Digimon digivolves into this card for a digivolution cost of 3, ignoring digivolution requirements.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
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

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEndAttack
        # [End of Attack][Once Per Turn] By returning 3 cards with [Jellymon] in their text from your trash at the bottom of the deck in any order, unsuspend this Digimon.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT13-028 Return cards from trash to the bottom of deck to unsuspend this Digimon")
        effect1.set_effect_description("[End of Attack][Once Per Turn] By returning 3 cards with [Jellymon] in their text from your trash at the bottom of the deck in any order, unsuspend this Digimon.")
        effect1.is_inherited_effect = True
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Unsuspend_BT13_028")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Jellymon' in text):
                    return False
            else:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Unsuspend, Return To Deck"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=True)
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_return(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.return_permanent_to_deck_bottom(target_perm)
            game.effect_select_opponent_permanent(
                player, on_return, filter_fn=target_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
