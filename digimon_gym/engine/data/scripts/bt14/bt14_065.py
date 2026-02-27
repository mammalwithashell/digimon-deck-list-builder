from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_065(CardScript):
    """BT14-065 Vademon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Your opponent reveals the top 3 cards of their deck. <De-Digivolve 1> 1 of your opponent's Digimon for each Digimon card among them. Return the revealed cards to the top or bottom of the deck.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-065 Reveal the top 3 cards of opponent's deck")
        effect0.set_effect_description("[On Play] Your opponent reveals the top 3 cards of their deck. <De-Digivolve 1> 1 of your opponent's Digimon for each Digimon card among them. Return the revealed cards to the top or bottom of the deck.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game and player.enemy):
                return

            enemy = player.enemy
            state = {'digimon_count': 0}

            def reveal_filter(c):
                return True

            def on_revealed(selected, remaining):
                revealed = []
                if selected is not None:
                    if isinstance(selected, list):
                        revealed.extend(selected)
                    else:
                        revealed.append(selected)
                if remaining:
                    revealed.extend(remaining)

                cnt = 0
                for rc in revealed:
                    kind = getattr(rc, 'card_kind', None)
                    if kind == 0:
                        cnt += 1
                state['digimon_count'] = cnt

                for rc in reversed(revealed):
                    enemy.library_cards.append(rc)

            game.effect_reveal_and_select(enemy, 3, reveal_filter, on_revealed, is_optional=False)

            count = state.get('digimon_count', 0)
            for _ in range(count):
                def on_de_digivolve(target_perm):
                    target_perm.de_digivolve(1)

                game.effect_select_opponent_permanent(
                    player,
                    on_de_digivolve,
                    filter_fn=lambda p: p.is_digimon and len(getattr(p, 'evolution_cards', [])) > 0,
                    is_optional=True,
                )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Your opponent reveals the top 3 cards of their deck. <De-Digivolve 1> 1 of your opponent's Digimon for each Digimon card among them. Return the revealed cards to the top or bottom of the deck.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-065 Reveal the top 3 cards of opponent's deck")
        effect1.set_effect_description("[When Digivolving] Your opponent reveals the top 3 cards of their deck. <De-Digivolve 1> 1 of your opponent's Digimon for each Digimon card among them. Return the revealed cards to the top or bottom of the deck.")
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game and player.enemy):
                return

            enemy = player.enemy
            state = {'digimon_count': 0}

            def reveal_filter(c):
                return True

            def on_revealed(selected, remaining):
                revealed = []
                if selected is not None:
                    if isinstance(selected, list):
                        revealed.extend(selected)
                    else:
                        revealed.append(selected)
                if remaining:
                    revealed.extend(remaining)

                cnt = 0
                for rc in revealed:
                    kind = getattr(rc, 'card_kind', None)
                    if kind == 0:
                        cnt += 1
                state['digimon_count'] = cnt

                for rc in reversed(revealed):
                    enemy.library_cards.append(rc)

            game.effect_reveal_and_select(enemy, 3, reveal_filter, on_revealed, is_optional=False)

            count = state.get('digimon_count', 0)
            for _ in range(count):
                def on_de_digivolve(target_perm):
                    target_perm.de_digivolve(1)

                game.effect_select_opponent_permanent(
                    player,
                    on_de_digivolve,
                    filter_fn=lambda p: p.is_digimon and len(getattr(p, 'evolution_cards', [])) > 0,
                    is_optional=True,
                )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
