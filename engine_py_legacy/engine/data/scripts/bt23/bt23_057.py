from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any

from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, GamePhase

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_057(CardScript):
    """BT23-057 Gankoomon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-057 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5
        effect0._alt_digi_trait = "CS"

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.BeforePayCost)
        effect1.set_effect_name("BT23-057 play cost -5")
        effect1.set_effect_description(
            "When this card would be played, by returning 3 cards with [Huckmon], [Sistermon] or [Jesmon] in their names from your trash to the top or bottom of the deck, reduce the play cost by 5."
        )
        effect1.set_hash_string("PlayCost-5_BT23_057")
        effect1.cost_reduction = 5

        def condition1(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            qualifying = [c for c in owner.trash_cards
                          if any('Huckmon' in n or 'Sistermon' in n or 'Jesmon' in n
                                 for n in getattr(c, 'card_names', []))]
            return len(qualifying) >= 3

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Return 3 Huckmon/Sistermon/Jesmon cards from trash to top or bottom of deck."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def name_filter(c):
                names = getattr(c, 'card_names', [])
                return any('Huckmon' in n or 'Sistermon' in n or 'Jesmon' in n
                           for n in names)

            remaining = [0]  # count of cards still to select
            remaining[0] = 3

            def select_next_trash_card():
                if remaining[0] <= 0:
                    return
                from digimon_gym.engine.game.constants import SEL_TRASH_START
                valid = [SEL_TRASH_START + i for i, c in enumerate(player.trash_cards) if name_filter(c)]
                if not valid:
                    return

                def on_trash_selected(action_id: int):
                    from digimon_gym.engine.game.constants import SEL_TRASH_START
                    idx = action_id - SEL_TRASH_START
                    if not (0 <= idx < len(player.trash_cards)):
                        return
                    chosen = player.trash_cards[idx]
                    player.trash_cards.remove(chosen)
                    remaining[0] -= 1
                    # Let agent choose top or bottom of deck for each card
                    game.effect_choose_deck_placement(
                        player, chosen,
                        callback=select_next_trash_card,
                    )

                game.request_selection(
                    GamePhase.SelectTrash, player, on_trash_selected, valid,
                    is_optional=False,
                    prompt=f"Select a card with [Huckmon], [Sistermon], or [Jesmon] in its name from your trash to return to the deck ({remaining[0]} remaining)."
                )

            select_next_trash_card()

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        def make_token_delete_effect(name: str, when_digivolving: bool) -> ICardEffect:
            effect = ICardEffect()
            effect.set_timing(EffectTiming.OnEnterFieldAnyone)
            effect.set_effect_name(name)
            effect.set_effect_description(
                "You may play 1 [Hinukamuy] Token. Then, delete 1 of your opponent's Digimon within the effect's play-cost cap."
            )
            effect.is_optional = True  # "You may" — declining skips both token play and delete
            if when_digivolving:
                effect.is_when_digivolving = True
            else:
                effect.is_on_play = True

            def condition(context: Dict[str, Any]) -> bool:
                return bool(card and card.permanent_of_this_card() is not None)

            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get("player")
                game = ctx.get("game")
                owner_perm = card.permanent_of_this_card() if card else None
                if not (player and game and owner_perm):
                    return

                game.effect_play_token(player, "hinukamuy")
                max_play_cost = 6 + (3 * sum(1 for p in player.battle_area if p is not owner_perm and p.is_digimon))

                def target_filter(p):
                    if not p.is_digimon or not p.top_card:
                        return False
                    if not p.top_card.has_play_cost:
                        return False
                    return p.top_card.get_cost_itself <= max_play_cost

                def on_delete(target_perm):
                    enemy = player.enemy if player else None
                    if enemy:
                        enemy.delete_permanent(target_perm)

                game.effect_select_opponent_permanent(player, on_delete, filter_fn=target_filter, is_optional=False)

            effect.set_on_process_callback(process)
            return effect

        effects.append(make_token_delete_effect("BT23-057 play Hinukamuy Token", when_digivolving=False))
        effects.append(make_token_delete_effect("BT23-057 play Hinukamuy Token", when_digivolving=True))

        return effects
