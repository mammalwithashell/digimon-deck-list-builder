from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_074(CardScript):
    """EX6-074 Mirei Mikagura"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn] When one of your Digimon with the [Holy Beast], [Archangel] or [Fallen Angel] trait is played, by suspending this Tamer, gain 1 memory. Then, 1 of your Digimon may digivolve into [Angewomon] or [LadyDevimon] in your trash with the cost reduced by 1.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX6-074 Memory +1, Digivolve from trash for 1 cost reduction")
        effect0.set_effect_description("[Your Turn] When one of your Digimon with the [Holy Beast], [Archangel] or [Fallen Angel] trait is played, by suspending this Tamer, gain 1 memory. Then, 1 of your Digimon may digivolve into [Angewomon] or [LadyDevimon] in your trash with the cost reduced by 1.")
        effect0.is_optional = True
        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if context.get('event_player') is not card.owner:
                return False
            played_card = context.get('played_card')
            if not played_card:
                return False
            traits = getattr(played_card, 'card_traits', []) or []
            return any(
                'Holy Beast' in trait or 'HolyBeast' in trait
                or 'Archangel' in trait
                or 'Fallen Angel' in trait or 'FallenAngel' in trait
                for trait in traits
            )

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Gain 1 memory, Suspend, Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.suspend()
            if player:
                player.add_memory(1)
            target = None
            if player:
                target = next((p for p in player.battle_area if p.is_digimon), None)
            if not (player and target and game):
                return
            def digi_filter(c):
                return (
                    getattr(c, 'is_digimon', False)
                    and (
                        c.contains_card_name('Angewomon')
                        or c.contains_card_name('LadyDevimon')
                    )
                )
            game.effect_digivolve_from_hand(
                player, target, digi_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] [Once Per Turn] 2 of your Digimon may DNA digivolve into a Digimon card in your hand.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEndTurn)
        effect1.set_effect_name("EX6-074 DNA digivolve 2 Digimon")
        effect1.set_effect_description("[End of Your Turn] [Once Per Turn] 2 of your Digimon may DNA digivolve into a Digimon card in your hand.")
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("DNA_EX6_074")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: DNA Digivolve"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            def dna_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                return True
            game.effect_dna_digivolve_from_hand(
                player, dna_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("EX6-074 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
