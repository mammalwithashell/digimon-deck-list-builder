from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_030(CardScript):
    """BT13-030 UlforceVeedramon | Lv.6

    [On Play] [When Digivolving] For each of your Digimon with the [Royal Knight]
        trait and each of your blue Tamers in play, trash the top 2 digivolution
        cards of 1 of your opponent's Digimon.
    [Your Turn][Once Per Turn] When you play a Digimon with the [Royal Knight]
        trait or a blue Tamer, return 1 of your opponent's Digimon with no
        digivolution cards to its owner's hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _count_rk_and_blue_tamers():
            """Count Royal Knight Digimon + blue Tamers on our field."""
            owner = card.owner if card else None
            if not owner:
                return 0
            count = 0
            for p in owner.battle_area:
                if not p.top_card:
                    continue
                traits = getattr(p.top_card, 'card_traits', []) or []
                is_rk = p.is_digimon and any('Royal Knight' in t for t in traits)
                colors = getattr(p.top_card, 'card_colors', []) or []
                from ....data.enums import CardColor
                is_blue_tamer = p.is_tamer and CardColor.Blue in colors
                if is_rk or is_blue_tamer:
                    count += 1
            return count

        def _trash_digi_cards_process(ctx: Dict[str, Any]):
            """Select 1 opponent Digimon, trash (2 * RK+BlueTamer count) digivolution cards."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            trash_count = 2 * _count_rk_and_blue_tamers()
            if trash_count <= 0:
                return

            def target_filter(p):
                return p.is_digimon and not p.has_no_digivolution_cards

            def on_select(target_perm):
                actual_count = min(trash_count, len(target_perm.card_sources) - 1)
                if actual_count > 0:
                    trashed = target_perm.trash_digivolution_cards(actual_count, from_top=True)
                    if trashed:
                        target_perm.owner.trash_cards.extend(trashed)

            game.effect_select_opponent_permanent(
                player, on_select, filter_fn=target_filter, is_optional=False)

        # --- Effect 0: [On Play] Trash digi-cards ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT13-030 Trash digivolution cards")
        effect0.set_effect_description(
            "[On Play] For each of your Digimon with the [Royal Knight] trait "
            "and each of your blue Tamers, trash the top 2 digivolution cards "
            "of 1 of your opponent's Digimon."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effect0.set_on_process_callback(_trash_digi_cards_process)
        effects.append(effect0)

        # --- Effect 1: [When Digivolving] Trash digi-cards ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT13-030 Trash digivolution cards")
        effect1.set_effect_description(
            "[When Digivolving] For each of your Digimon with the [Royal Knight] "
            "trait and each of your blue Tamers, trash the top 2 digivolution "
            "cards of 1 of your opponent's Digimon."
        )
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_trash_digi_cards_process)
        effects.append(effect1)

        # --- Effect 2: [Your Turn][Once Per Turn] play observer bounce ---
        effect2 = ICardEffect()
        effect2.set_effect_name("BT13-030 Return 1 Digimon with no digivolution cards to hand")
        effect2.set_effect_description(
            "[Your Turn][Once Per Turn] When you play a Digimon with the "
            "[Royal Knight] trait or a blue Tamer, return 1 of your opponent's "
            "Digimon with no digivolution cards to the hand."
        )
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Bounce_BT13_030")
        effect2._is_play_observer = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Check the played permanent is a Royal Knight Digimon or a blue Tamer
            played_perm = context.get('played_permanent')
            if not played_perm:
                return False
            if not played_perm.top_card:
                return False
            traits = getattr(played_perm.top_card, 'card_traits', []) or []
            is_rk = played_perm.is_digimon and any('Royal Knight' in t for t in traits)
            colors = getattr(played_perm.top_card, 'card_colors', []) or []
            from ....data.enums import CardColor
            is_blue_tamer = played_perm.is_tamer and CardColor.Blue in colors
            if not (is_rk or is_blue_tamer):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Bounce opponent's Digimon with no digivolution cards."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            def target_filter(p):
                return p.is_digimon and p.has_no_digivolution_cards

            def on_bounce(target_perm):
                enemy.bounce_permanent_to_hand(target_perm)

            game.effect_select_opponent_permanent(
                player, on_bounce, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
