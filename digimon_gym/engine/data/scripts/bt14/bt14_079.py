from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_079(CardScript):
    """BT14-079 Soloogarmon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may play 1 level 3 or lower card with the [Dark Animal] or [SoC] trait from your trash without paying the cost. If [Eiji Nagasumi] is in this Digimon's digivolution cards, add 1 to the level this effect may choose.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-079 Play 1 Digimon from trash")
        effect0.set_effect_description("[When Digivolving] You may play 1 level 3 or lower card with the [Dark Animal] or [SoC] trait from your trash without paying the cost. If [Eiji Nagasumi] is in this Digimon's digivolution cards, add 1 to the level this effect may choose.")
        effect0.is_optional = True
        effect0.is_when_digivolving = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            perm = ctx.get('permanent')
            if not (player and game):
                return

            max_level = 3
            evo_cards = []
            if perm is not None:
                evo_cards = getattr(perm, 'evolution_cards', None) or getattr(perm, 'digivolution_cards', None) or []
            for evo in evo_cards:
                if getattr(evo, 'card_id', '') == 'BT14-087' or getattr(evo, 'card_name_eng', '') == 'Eiji Nagasumi':
                    max_level += 1
                    break

            def has_required_trait(c) -> bool:
                traits = getattr(c, 'type_eng', None) or getattr(c, 'types', None) or []
                if isinstance(traits, str):
                    traits = [traits]
                return ('Dark Animal' in traits) or ('SoC' in traits)

            def play_filter(c):
                if getattr(c, 'is_digi_egg', False):
                    return False
                level = getattr(c, 'level', None)
                if level is None:
                    return False
                if level > max_level:
                    return False
                return has_required_trait(c)

            game.effect_play_from_zone(player, 'trash', play_filter, free=True, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] By trashing 1 card in your hand, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-079 Trash 1 card from hand to gain Memory +1")
        effect1.set_effect_description("[When Attacking] By trashing 1 card in your hand, gain 1 memory.")
        effect1.is_optional = True
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            trashed = {'ok': False}

            def hand_filter(c):
                return True

            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
                    trashed['ok'] = True

            game.effect_select_hand_card(player, hand_filter, on_trashed, is_optional=True)
            if trashed['ok']:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn][Once Per Turn] When you play a card with the [Dark Animal] or [SoC] trait, you may unsuspend this Digimon.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-079 Unsuspend this Digimon")
        effect2.set_effect_description("[Your Turn][Once Per Turn] When you play a card with the [Dark Animal] or [SoC] trait, you may unsuspend this Digimon.")
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Unsupend_BT14_079")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            played_card = context.get('card') or context.get('played_card')
            if played_card is None:
                return False
            traits = getattr(played_card, 'type_eng', None) or getattr(played_card, 'types', None) or []
            if isinstance(traits, str):
                traits = [traits]
            return ('Dark Animal' in traits) or ('SoC' in traits)

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            perm = ctx.get('permanent')
            if perm is not None:
                perm.unsuspend()

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
