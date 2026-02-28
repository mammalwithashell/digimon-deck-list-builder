from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_089(CardScript):
    """P-089 Amphimon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may trash up to 3 blue cards from your hand. For each card trashed by this effect, you may trash 1 card under 1 of your opponent's Digimon or Tamers. Then, 1 of your opponent's Digimon or Tamers without cards under it can't suspend until the end of your opponent's turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("P-089 Trash cards from hand to trash digivolution cards, and opponent's 1 Digimon or Tamer can't suspend")
        effect0.set_effect_description("[When Digivolving] You may trash up to 3 blue cards from your hand. For each card trashed by this effect, you may trash 1 card under 1 of your opponent's Digimon or Tamers. Then, 1 of your opponent's Digimon or Tamers without cards under it can't suspend until the end of your opponent's turn.")
        effect0.is_when_digivolving = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Trash From Hand, Trash Digivolution Cards, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def hand_filter(c):
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)
            # Grant effect immunity via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # [Opponent's Turn] [Once Per Turn] When an opponent's Digimon attacks, end that attack by returning 3 cards with [Jellymon] in their text from your trash to the bottom of your deck.
        effect1 = ICardEffect()
        effect1.set_effect_name("P-089 Return cards from trash to the bottom of deck to end the attack")
        effect1.set_effect_description("[Opponent's Turn] [Once Per Turn] When an opponent's Digimon attacks, end that attack by returning 3 cards with [Jellymon] in their text from your trash to the bottom of your deck.")
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("EndAttack_P_089")
        effect1.is_on_attack = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Jellymon' in text):
                    return False
            else:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Return To Deck"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_return(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.return_permanent_to_deck_bottom(target_perm)
            game.effect_select_opponent_permanent(
                player, on_return, filter_fn=target_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
