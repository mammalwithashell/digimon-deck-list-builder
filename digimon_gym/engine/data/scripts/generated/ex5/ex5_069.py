from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX5_069(CardScript):
    """EX5-069 Biting Crush"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] By trashing 1 card in your hand, delete 1 of your opponent's level 6 or lower Digimon. If this effect trashed a card with the [Seven Great Demon Lords] trait, place this card in the battle area.
        effect0 = ICardEffect()
        effect0.set_effect_name("EX5-069 By Trashing 1 card, delete level 6 or lower")
        effect0.set_effect_description("[Main] By trashing 1 card in your hand, delete 1 of your opponent's level 6 or lower Digimon. If this effect trashed a card with the [Seven Great Demon Lords] trait, place this card in the battle area.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Delete, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if p.level is None or p.level > 6:
                    return False
                if not (any('Seven Great Demon Lords' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('SevenGreatDemonLords' in t for t in (getattr(p.top_card, 'card_traits', []) or []))):
                    return False
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)
            if not (player and game):
                return
            def hand_filter(c):
                if not (any('Seven Great Demon Lords' in _t or 'SevenGreatDemonLords' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                if getattr(c, 'level', None) is None or c.level > 6:
                    return False
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
        effect1.set_effect_name("EX5-069 Delay")
        effect1.set_effect_description("Delay")
        effect1.is_on_play = True
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [All Turns] When an effect plays an opponent's Digimon, <Delay> (After this card is placed, by trashing it the next turn or later, activate the effect below.) - You may play 1 [Leviamon] from your trash without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX5-069 Play 1 [Leviamon] from trash")
        effect2.set_effect_description("[All Turns] When an effect plays an opponent's Digimon, <Delay> (After this card is placed, by trashing it the next turn or later, activate the effect below.) - You may play 1 [Leviamon] from your trash without paying the cost.")
        effect2.is_optional = True
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
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
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: security_play
        # Security: Play this card
        effect3 = ICardEffect()
        effect3.set_effect_name("EX5-069 Security: Play this card")
        effect3.set_effect_description("Security: Play this card")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
