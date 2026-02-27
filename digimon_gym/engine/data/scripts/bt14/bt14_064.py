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

        def _trait_cost_filter(c) -> bool:
            if not getattr(c, 'has_play_cost', False):
                return False
            if getattr(c, 'get_cost_itself', 0) > 4:
                return False
            has_trait = False
            if hasattr(c, 'has_trait'):
                has_trait = c.has_trait('D-Brigade') or c.has_trait('DigiPolice')
            return has_trait

        def _commandramon_filter(c) -> bool:
            if hasattr(c, 'is_name_contains'):
                return c.is_name_contains('Commandramon')
            name = getattr(c, 'card_name_eng', '') or ''
            return 'Commandramon' in name

        # [On Play] / [When Digivolving]
        def _process_reveal_top3_play_trait(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def on_revealed(selected, remaining):
                if selected is not None:
                    player.hand_cards.append(selected)
                    game.effect_play_from_zone(
                        player, 'hand', lambda c: c == selected, free=True, is_optional=False
                    )
                for c in remaining:
                    player.trash_cards.append(c)

            game.effect_reveal_and_select(
                player, 3, _trait_cost_filter, on_revealed, is_optional=True
            )

        effect0 = ICardEffect()
        effect0.set_effect_name('BT14-064 Reveal top 3 and play trait card')
        effect0.set_effect_description('[On Play] Reveal the top 3 cards of your deck. You may play 1 card with the [D-Brigade] or [DigiPolice] trait and a play cost of 4 or less among them without paying the cost. Trash the rest.')
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effect0.set_on_process_callback(_process_reveal_top3_play_trait)
        effects.append(effect0)

        effect1 = ICardEffect()
        effect1.set_effect_name('BT14-064 Reveal top 3 and play trait card')
        effect1.set_effect_description('[When Digivolving] Reveal the top 3 cards of your deck. You may play 1 card with the [D-Brigade] or [DigiPolice] trait and a play cost of 4 or less among them without paying the cost. Trash the rest.')
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_process_reveal_top3_play_trait)
        effects.append(effect1)

        # Inherited: [All Turns][Once Per Turn] When one of your other Digimon is deleted...
        effect2 = ICardEffect()
        effect2.set_effect_name('BT14-064 Inherited reveal top 3 and play Commandramon')
        effect2.set_effect_description('[All Turns][Once Per Turn] When one of your other Digimon is deleted, reveal the top 3 cards of your deck. You may play 1 [Commandramon] among them without paying the cost. Trash the rest.')
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string('Reveal_BT14_064')
        effect2.is_on_deletion = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            deleted_perm = context.get('target_permanent') or context.get('deleted_permanent') or context.get('permanent')
            owner = context.get('player')
            self_perm = card.permanent_of_this_card() if card else None
            if deleted_perm is None or owner is None:
                return False
            if hasattr(deleted_perm, 'owner') and deleted_perm.owner != owner:
                return False
            if self_perm is not None and deleted_perm == self_perm:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def on_revealed(selected, remaining):
                if selected is not None:
                    player.hand_cards.append(selected)
                    game.effect_play_from_zone(
                        player, 'hand', lambda c: c == selected, free=True, is_optional=False
                    )
                for c in remaining:
                    player.trash_cards.append(c)

            game.effect_reveal_and_select(
                player, 3, _commandramon_filter, on_revealed, is_optional=True
            )

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
