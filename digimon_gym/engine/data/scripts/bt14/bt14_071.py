from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_071(CardScript):
    """BT14-071 Loogamon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] By placing 1 [Eiji Nagasumi] from your hand or trash as this Digimon's bottom digivolution card, gain 1 memory.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-071 Place cards under this Digimon's digivolution cards to gain Memory +1")
        effect0.set_effect_description("[Start of Your Main Phase] By placing 1 [Eiji Nagasumi] from your hand or trash as this Digimon's bottom digivolution card, gain 1 memory.")
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False

            player = context.get('player')
            if not player:
                return False

            hand = getattr(player, 'hand', []) or []
            trash = getattr(player, 'trash', []) or []

            def is_eiji_nagasumi(c: Any) -> bool:
                return getattr(c, 'card_id', '') == 'BT14-087' or getattr(c, 'card_name_eng', '') == 'Eiji Nagasumi'

            return any(is_eiji_nagasumi(c) for c in hand) or any(is_eiji_nagasumi(c) for c in trash)

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Place 1 [Eiji Nagasumi] from hand/trash as bottom digivolution card, then gain 1 memory."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if not player or not perm:
                return

            hand = getattr(player, 'hand', []) or []
            trash = getattr(player, 'trash', []) or []

            def is_eiji_nagasumi(c: Any) -> bool:
                return getattr(c, 'card_id', '') == 'BT14-087' or getattr(c, 'card_name_eng', '') == 'Eiji Nagasumi'

            source_zone = None
            target = None

            for c in hand:
                if is_eiji_nagasumi(c):
                    source_zone = hand
                    target = c
                    break

            if target is None:
                for c in trash:
                    if is_eiji_nagasumi(c):
                        source_zone = trash
                        target = c
                        break

            if target is None:
                return

            try:
                source_zone.remove(target)
            except ValueError:
                return

            evo_cards = getattr(perm, 'digivolution_cards', None)
            if evo_cards is None:
                evo_cards = []
                setattr(perm, 'digivolution_cards', evo_cards)

            evo_cards.insert(0, target)
            player.add_memory(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn][Once Per Turn] When you play a card with the [Dark Animal] or [SoC] trait, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-071 Memory +1")
        effect1.set_effect_description("[Your Turn][Once Per Turn] When you play a card with the [Dark Animal] or [SoC] trait, gain 1 memory.")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Memory+1_BT14_071")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False

            played_card = context.get('played_card') or context.get('card')
            if not played_card:
                return False

            traits = getattr(played_card, 'type_eng', None) or []
            return ('Dark Animal' in traits) or ('SoC' in traits)

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            if player:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
