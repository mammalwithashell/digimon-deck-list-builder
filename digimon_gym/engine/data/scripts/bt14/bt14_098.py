from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_098(CardScript):
    """BT14-098 DCD Bomb"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] <De-Digivolve 1> 1 of your opponent's Digimon.
        # Then, by returning 3 cards with the [D-Brigade] or [DigiPolice] trait from your trash to the top of the deck,
        # delete up to 6 play cost's total worth of your opponent's Digimon.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-098 De-Digivolve then conditional delete")
        effect0.set_effect_description("[Main] <De-Digivolve 1> 1 of your opponent's Digimon. Then, by returning 3 cards with the [D-Brigade] or [DigiPolice] trait from your trash to the top of the deck, delete up to 6 play cost's total worth of your opponent's Digimon.")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # 1) Mandatory: <De-Digivolve 1> 1 opponent Digimon
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy and removed:
                    enemy.trash_cards.extend(removed)

            game.effect_select_opponent_permanent(
                player,
                on_de_digivolve,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False,
            )

            # 2) Optional additional cost/effect gate:
            # return exactly 3 qualifying cards from trash to top of deck, then delete up to 6 total play cost.
            trash = list(getattr(player, 'trash_cards', []) or [])

            def has_required_trait(c) -> bool:
                traits = getattr(c, 'card_traits', []) or []
                return any('D-Brigade' in t for t in traits) or any('DigiPolice' in t for t in traits)

            valid = [c for c in trash if has_required_trait(c)]
            if len(valid) < 3:
                return

            chosen = valid[:3]
            for c in chosen:
                if c in player.trash_cards:
                    player.trash_cards.remove(c)
            # "to the top of the deck"; keep deterministic order from selection list
            for c in reversed(chosen):
                player.deck_cards.append(c)

            remaining = 6
            enemy = player.enemy if player else None
            if not enemy:
                return

            while remaining > 0:
                candidates = [p for p in list(getattr(enemy, 'battle_area', []) or []) if getattr(p, 'is_digimon', False)]
                affordable = [p for p in candidates if (getattr(getattr(p, 'top_card', None), 'play_cost', 0) or 0) <= remaining]
                if not affordable:
                    break

                selected_holder = {'target': None}

                def on_delete(target_perm):
                    selected_holder['target'] = target_perm

                game.effect_select_opponent_permanent(
                    player,
                    on_delete,
                    filter_fn=lambda p, rem=remaining: p.is_digimon and ((getattr(getattr(p, 'top_card', None), 'play_cost', 0) or 0) <= rem),
                    is_optional=True,
                )

                target = selected_holder['target']
                if target is None:
                    break

                cost = getattr(getattr(target, 'top_card', None), 'play_cost', 0) or 0
                if cost > remaining:
                    break

                enemy.delete_permanent(target)
                remaining -= cost

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
