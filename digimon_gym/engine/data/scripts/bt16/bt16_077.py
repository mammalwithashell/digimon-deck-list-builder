from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_077(CardScript):
    """BT16-077 Dinobeemon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Jogress Condition
        effect0 = ICardEffect()
        effect0.set_effect_name("BT16-077 Jogress Condition")
        effect0.set_effect_description("Jogress Condition")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: raid
        # Raid
        effect1 = ICardEffect()
        effect1.set_effect_name("BT16-077 Raid")
        effect1.set_effect_description("Raid")
        effect1.is_on_attack = True
        effect1._is_raid = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: partition
        # Partition
        effect2 = ICardEffect()
        effect2.set_effect_name("BT16-077 Partition")
        effect2.set_effect_description("Partition")
        effect2._is_partition = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: partition
        # Partition
        effect3 = ICardEffect()
        effect3.set_effect_name("BT16-077 Partition")
        effect3.set_effect_description("Partition")
        effect3.is_inherited_effect = True
        effect3._is_partition = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] If DNA digivolving, you may play 1 level 5 or lower Digimon with the [Free] trait from your trash without paying the cost. Then, 1 of your Digimon may gain [Rush] for the turn and attack the player.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT16-077 You may play 1 level 5 or lower Digimon with the [Free] trait. Then 1 Digimon can gain [Rush] and attack.")
        effect4.set_effect_description("[When Digivolving] If DNA digivolving, you may play 1 level 5 or lower Digimon with the [Free] trait from your trash without paying the cost. Then, 1 of your Digimon may gain [Rush] for the turn and attack the player.")
        effect4.is_optional = True
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Play Card, Gain Keyword Rush, Force Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if getattr(c, 'level', None) is None or c.level > 5:
                    return False
                if not (any('Free' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)
            # Grant Rush to 1 of your Digimon for the turn
            def rush_filter(p):
                return p.is_digimon
            def on_rush(target_perm):
                target_perm.grant_keyword('_is_rush')
            game.effect_select_own_permanent(
                player, on_rush, filter_fn=rush_filter, is_optional=True)
            # Force attack — target Digimon may attack (requires engine SelectAttack)
            pass  # descriptive-tagged: force_attack

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
