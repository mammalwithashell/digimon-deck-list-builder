from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_056(CardScript):
    """BT13-056 Leopardmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving][Once Per Turn] You may play 1 green or [Royal Knight] trait Digimon card from your hand for the cost. When it would be played by this effect, reduce the play cost by 4.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT13-056 Play 1 Digimon from hand")
        effect0.set_effect_description("[When Digivolving][Once Per Turn] You may play 1 green or [Royal Knight] trait Digimon card from your hand for the cost. When it would be played by this effect, reduce the play cost by 4.")
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("PlayDigimon_BT13_056")
        effect0.is_when_digivolving = True
        effect0.cost_reduction = 4

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Cost -4, Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not ('Green' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                if not (any('Royal Knight' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            # Cost reduction by 4 — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDeclaration
        # [Main][Once Per Turn] You may play 1 green or [Royal Knight] trait Digimon card from your hand for the cost. When it would be played by this effect, reduce the play cost by 4.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDeclaration)
        effect1._is_field_main = True
        effect1.set_effect_name("BT13-056 Play 1 Digimon from hand")
        effect1.set_effect_description("[Main][Once Per Turn] You may play 1 green or [Royal Knight] trait Digimon card from your hand for the cost. When it would be played by this effect, reduce the play cost by 4.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("PlayDigimon_BT13_056")
        effect1.cost_reduction = 4

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Cost -4, Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not ('Green' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                if not (any('Royal Knight' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            # Cost reduction by 4 — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [All Turns] When you play another Digimon, all of your green and [Royal Knight] trait Digimon gain <Blocker> until the end of your opponent's turn.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT13-056 Your Digimons gain Blocker")
        effect2.set_effect_description("[All Turns] When you play another Digimon, all of your green and [Royal Knight] trait Digimon gain <Blocker> until the end of your opponent's turn.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("GetEffects_BT13_056")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
