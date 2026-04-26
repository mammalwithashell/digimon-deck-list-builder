from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX1_021(CardScript):
    """EX1-021 MetalGarurumon | Lv.6 Blue Digimon | Deep Savers

    [When Digivolving] Gain 1 memory for every 4 cards in your hand.
    [When Attacking] If you have 8 or more cards in your hand and a Tamer
        in play, return 1 of your opponent's Digimon that has an [On Deletion]
        effect to the bottom of its owner's deck.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [When Digivolving] Gain 1 memory per 4 hand cards ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX1-021 When Digivolving: Memory per 4 hand cards")
        effect0.set_effect_description(
            "[When Digivolving] Gain 1 memory for every 4 cards in your hand."
        )
        effect0.is_when_digivolving = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            return len(owner.hand_cards) // 4 >= 1
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Gain 1 memory for every 4 cards in hand."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            count = len(player.hand_cards) // 4
            if count > 0:
                player.add_memory(count)
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [When Attacking] Return opponent Digimon with On Deletion ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnTappedAnyone)
        effect1.set_effect_name("EX1-021 When Attacking: Return On Deletion Digimon to deck bottom")
        effect1.set_effect_description(
            "[When Attacking] If you have 8 or more cards in your hand and "
            "a Tamer in play, return 1 of your opponent's Digimon that has an "
            "[On Deletion] effect to the bottom of its owner's deck."
        )

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            ctx_perm = context.get('attacking_permanent') or context.get('permanent')
            if perm and ctx_perm and perm != ctx_perm:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            # Need 8+ cards in hand
            if len(owner.hand_cards) < 8:
                return False
            # Need a Tamer in play
            if not any(p.is_tamer for p in owner.battle_area):
                return False
            # Need at least 1 opponent Digimon with On Deletion effect
            enemy = owner.enemy
            if not enemy:
                return False
            return any(
                p.is_digimon and getattr(p, 'has_on_deletion_effect', False)
                for p in enemy.battle_area
            )
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Return opponent Digimon with [On Deletion] to deck bottom."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            def on_deletion_filter(p):
                return (p.is_digimon
                        and getattr(p, 'has_on_deletion_effect', False))

            def on_select(target_perm):
                # Return to bottom of owner's deck (no trashing digi cards)
                enemy.return_permanent_to_deck_bottom(target_perm)

            game.effect_select_opponent_permanent(
                player, on_select, filter_fn=on_deletion_filter,
                is_optional=False)
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
