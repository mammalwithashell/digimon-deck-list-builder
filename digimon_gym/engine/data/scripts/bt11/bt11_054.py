from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_054(CardScript):
    """BT11-054 Panjyamon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []
        card.also_treated_as_names = ['Leomon']

        # Timing: EffectTiming.None
        # Also Treated As
        effect0 = ICardEffect()
        effect0.set_effect_name("BT11-054 Also treated as having [Leomon] in its name")
        effect0.set_effect_description("Also Treated As")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Also Treated As"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Also treated as [Name] — name aliasing not modeled in engine
            pass  # descriptive-tagged: also_treated_as_name

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may play 1 green or blue Tamer card with a play cost of 4 or less from your hand without paying the cost.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT11-054 Play 1 Tamer from hand")
        effect1.set_effect_description("[When Digivolving] You may play 1 green or blue Tamer card with a play cost of 4 or less from your hand without paying the cost.")
        effect1.is_optional = True
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
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
                if not getattr(c, 'is_tamer', False):
                    return False
                if getattr(c, 'get_cost_itself', 0) > 4:
                    return False
                if not ('Blue' in [col.name for col in getattr(c, 'card_colors', [])] or 'Green' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn][Once Per Turn] When you play another Digimon by an effect, 1 of your Digimon gains <Rush> for the turn. (This Digimon may attack the turn it was played.)
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT11-054 Your 1 Digimon gains Rush")
        effect2.set_effect_description("[Your Turn][Once Per Turn] When you play another Digimon by an effect, 1 of your Digimon gains <Rush> for the turn. (This Digimon may attack the turn it was played.)")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Rush_BT11_054")
        effect2.is_on_play = True
        effect2._is_rush = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain Keyword Rush"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_grant(target_perm):
                target_perm.grant_keyword('_is_rush')
            game.effect_select_own_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
