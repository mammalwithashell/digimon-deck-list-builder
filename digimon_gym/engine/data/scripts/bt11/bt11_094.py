from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_094(CardScript):
    """BT11-094 Mirei Mikagura"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartTurn
        # [Start of Your Turn] Gain 1 memory.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartTurn)
        effect0.set_effect_name("BT11-094 Memory +1")
        effect0.set_effect_description("[Start of Your Turn] Gain 1 memory.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn] When one of your Digimon digivolves into [Angewomon] or [LadyDevimon], if you have 1 or fewer Digimon in play, by suspending this Tamer, you may play 1 [Angewomon] or [LadyDevimon] with a different name than the Digimon you digivolved into from your hand without paying the cost.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.WhenDigivolving)
        effect1.set_effect_name("BT11-094 Play 1 [Angewomon] or [LadyDevimon]")
        effect1.set_effect_description("[Your Turn] When one of your Digimon digivolves into [Angewomon] or [LadyDevimon], if you have 1 or fewer Digimon in play, by suspending this Tamer, you may play 1 [Angewomon] or [LadyDevimon] with a different name than the Digimon you digivolved into from your hand without paying the cost.")
        effect1.is_optional = True
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if sum(1 for p in card.owner.battle_area if p.is_digimon) > 1:
                return False
            digivolved_perm = context.get('digivolved_permanent')
            if not digivolved_perm or digivolved_perm.owner is not card.owner:
                return False
            top_card = getattr(digivolved_perm, 'top_card', None)
            if not top_card:
                return False
            return (
                top_card.contains_card_name('Angewomon')
                or top_card.contains_card_name('LadyDevimon')
            )

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend, Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            if perm:
                perm.suspend()
            digivolved_perm = ctx.get('digivolved_permanent')
            digivolved_card = digivolved_perm.top_card if digivolved_perm else None
            def play_filter(c):
                if not (
                    c.contains_card_name('Angewomon')
                    or c.contains_card_name('LadyDevimon')
                ):
                    return False
                if digivolved_card and any(
                    name in getattr(digivolved_card, 'card_names', [])
                    for name in getattr(c, 'card_names', [])
                ):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT11-094 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
