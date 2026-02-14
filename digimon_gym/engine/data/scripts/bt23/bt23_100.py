from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_100(CardScript):
    """BT23-100"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-100 Ignore color requirements")
        effect0.set_effect_description("Effect")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        # [Main] <Draw 1> Then, place this card in the battle area.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-100 Draw 1, then place in battle area")
        effect1.set_effect_description("[Main] <Draw 1> Then, place this card in the battle area.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDeclaration
        # [Main] <Delay>, You may play 1 Tamer card with the [CS] trait from your hand without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-100 Play 1 tamer with [CS] trait in hand")
        effect2.set_effect_description("[Main] <Delay>, You may play 1 Tamer card with the [CS] trait from your hand without paying the cost.")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
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

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.SecuritySkill
        # [Security] You may play 1 level 3 Digimon card with the [CS] trait from your hand or trash without paying the cost. Then, place this card in the battle area.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-100 Play 1 level 3 [CS] digimon from hand or trash, then place in battle area")
        effect3.set_effect_description("[Security] You may play 1 level 3 Digimon card with the [CS] trait from your hand or trash without paying the cost. Then, place this card in the battle area.")
        effect3.is_security_effect = True
        effect3.is_security_effect = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
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

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
