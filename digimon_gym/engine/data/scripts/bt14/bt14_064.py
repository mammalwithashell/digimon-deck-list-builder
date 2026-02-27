from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_064(CardScript):
    """BT14-064 Cargodramon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _has_trait(c, trait_name: str) -> bool:
            trait_name = trait_name.lower()
            for attr in ('type_eng', 'types', 'traits'):
                vals = getattr(c, attr, None)
                if vals and any(trait_name == str(v).lower() for v in vals):
                    return True
            return trait_name in str(getattr(c, 'card_text', '')).lower()

        def _is_commandramon(c) -> bool:
            name = str(getattr(c, 'card_name_eng', '') or getattr(c, 'name', '')).lower()
            return 'commandramon' in name

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Reveal the top 3 cards of your deck. You may play 1 card with the [D-Brigade] or [DigiPolice] trait and a play cost of 4 or less among them without paying the cost. Trash the rest.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-064 Reveal the top 3 cards of deck")
        effect0.set_effect_description("[On Play] Reveal the top 3 cards of your deck. You may play 1 card with the [D-Brigade] or [DigiPolice] trait and a play cost of 4 or less among them without paying the cost. Trash the rest.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def reveal_filter(c):
                if not getattr(c, 'has_play_cost', False):
                    return False
                if getattr(c, 'get_cost_itself', 0) > 4:
                    return False
                return _has_trait(c, 'D-Brigade') or _has_trait(c, 'DigiPolice')

            def on_revealed(selected, remaining):
                if selected is not None:
                    player.hand_cards.append(selected)
                    def play_filter(x):
                        return x is selected
                    game.effect_play_from_zone(player, 'hand', play_filter, free=True, is_optional=False)
                for c in remaining:
                    player.trash_cards.append(c)

            game.effect_reveal_and_select(player, 3, reveal_filter, on_revealed, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Reveal the top 3 cards of your deck. You may play 1 card with the [D-Brigade] or [DigiPolice] trait and a play cost of 4 or less among them without paying the cost. Trash the rest.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-064 Reveal the top 3 cards of deck")
        effect1.set_effect_description("[When Digivolving] Reveal the top 3 cards of your deck. You may play 1 card with the [D-Brigade] or [DigiPolice] trait and a play cost of 4 or less among them without paying the cost. Trash the rest.")
        effect1.is_when_digivolving = True

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

            def reveal_filter(c):
                if not getattr(c, 'has_play_cost', False):
                    return False
                if getattr(c, 'get_cost_itself', 0) > 4:
                    return False
                return _has_trait(c, 'D-Brigade') or _has_trait(c, 'DigiPolice')

            def on_revealed(selected, remaining):
                if selected is not None:
                    player.hand_cards.append(selected)
                    def play_filter(x):
                        return x is selected
                    game.effect_play_from_zone(player, 'hand', play_filter, free=True, is_optional=False)
                for c in remaining:
                    player.trash_cards.append(c)

            game.effect_reveal_and_select(player, 3, reveal_filter, on_revealed, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [All Turns][Once Per Turn] When one of your other Digimon is deleted, reveal the top 3 cards of your deck. You may play 1 [Commandramon] among them without paying the cost. Trash the rest.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-064 Reveal the top 3 cards of deck")
        effect2.set_effect_description("[All Turns][Once Per Turn] When one of your other Digimon is deleted, reveal the top 3 cards of your deck. You may play 1 [Commandramon] among them without paying the cost. Trash the rest.")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Reveal_BT14_064")
        effect2.is_on_deletion = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def reveal_filter(c):
                return _is_commandramon(c)

            def on_revealed(selected, remaining):
                if selected is not None:
                    player.hand_cards.append(selected)
                    def play_filter(x):
                        return x is selected
                    game.effect_play_from_zone(player, 'hand', play_filter, free=True, is_optional=False)
                for c in remaining:
                    player.trash_cards.append(c)

            game.effect_reveal_and_select(player, 3, reveal_filter, on_revealed, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
