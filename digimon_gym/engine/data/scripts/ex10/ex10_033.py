from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_033(CardScript):
    """EX10-033 Pyramidimon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: fragment
        # Fragment
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-033 Fragment")
        effect0.set_effect_description("Fragment")
        effect0._is_fragment = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] [Once Per Turn] You may place up to 3 [Mineral] or [Rock] cards from your trash as this Digimon's bottom digivolution cards.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX10-033 Place up to 3 cards from trash as bottom sources")
        effect1.set_effect_description("[When Digivolving] [Once Per Turn] You may place up to 3 [Mineral] or [Rock] cards from your trash as this Digimon's bottom digivolution cards.")
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("WDWA_EX10_033")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if not (player and perm):
                return
            placed = 0
            for source_card in list(player.trash_cards):
                if not getattr(source_card, 'is_digimon', False):
                    continue
                traits = set(getattr(source_card, 'card_traits', []))
                if "Mineral" not in traits and "Rock" not in traits:
                    continue
                player.trash_cards.remove(source_card)
                perm.add_card_source_bottom(source_card)
                placed += 1
                if placed >= 3:
                    break

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking] [Once Per Turn] You may place up to 3 [Mineral] or [Rock] cards from your trash as this Digimon's bottom digivolution cards.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUseAttack)
        effect2.set_effect_name("EX10-033 Place up to 3 cards from trash as bottom sources")
        effect2.set_effect_description("[When Attacking] [Once Per Turn] You may place up to 3 [Mineral] or [Rock] cards from your trash as this Digimon's bottom digivolution cards.")
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("WDWA_EX10_033")
        effect2.is_on_attack = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if not (player and perm):
                return
            placed = 0
            for source_card in list(player.trash_cards):
                if not getattr(source_card, 'is_digimon', False):
                    continue
                traits = set(getattr(source_card, 'card_traits', []))
                if "Mineral" not in traits and "Rock" not in traits:
                    continue
                player.trash_cards.remove(source_card)
                perm.add_card_source_bottom(source_card)
                placed += 1
                if placed >= 3:
                    break

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By trashing up to 3 [Mineral] or [Rock] trait cards from any of your Digimon's digivolution cards, to 1 of your opponent's Digimon, reduce the play cost by 2 until their turn ends for each card trashed.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("EX10-033 By trashing up to 3 sources, reduce 1 digimons play cost by 2 for each card")
        effect3.set_effect_description("[When Digivolving] By trashing up to 3 [Mineral] or [Rock] trait cards from any of your Digimon's digivolution cards, to 1 of your opponent's Digimon, reduce the play cost by 2 until their turn ends for each card trashed.")
        effect3.is_optional = True
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Trash Digivolution Cards"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(3)
                if player:
                    player.trash_cards.extend(trashed)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking] By trashing up to 3 [Mineral] or [Rock] trait cards from any of your Digimon's digivolution cards, to 1 of your opponent's Digimon, reduce the play cost by 2 until their turn ends for each card trashed.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnUseAttack)
        effect4.set_effect_name("EX10-033 By trashing up to 3 sources, reduce 1 digimons play cost by 2 for each card")
        effect4.set_effect_description("[When Attacking] By trashing up to 3 [Mineral] or [Rock] trait cards from any of your Digimon's digivolution cards, to 1 of your opponent's Digimon, reduce the play cost by 2 until their turn ends for each card trashed.")
        effect4.is_optional = True
        effect4.is_on_attack = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Trash Digivolution Cards"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(3)
                if player:
                    player.trash_cards.extend(trashed)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
