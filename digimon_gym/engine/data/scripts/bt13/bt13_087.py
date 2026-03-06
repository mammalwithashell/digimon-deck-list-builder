from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_087(CardScript):
    """BT13-087 Dynasmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def lucemon_rk_filter(c):
            """Filter for cards with [Lucemon] in name or [Royal Knight] trait."""
            return (
                any('Lucemon' in n for n in getattr(c, 'card_names', []))
                or any('Royal Knight' in t for t in (getattr(c, 'card_traits', []) or []))
            )

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Reveal the top 4 cards of your deck. Add 2 cards with [Lucemon] in their
        # names or the [Royal Knight] trait among them to the hand. Trash the rest.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT13-087 Reveal top 4, add 2 Lucemon/Royal Knight to hand")
        effect0.set_effect_description("[On Play] Reveal the top 4 cards of your deck. Add 2 cards with [Lucemon] in their names or the [Royal Knight] trait among them to the hand. Trash the rest.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Reveal top 4, pick up to 2 Lucemon/Royal Knight to hand, trash rest."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            game.effect_reveal_and_select_multi(
                player, 4,
                passes=[
                    (lucemon_rk_filter, 'hand'),
                    (lucemon_rk_filter, 'hand'),
                ],
                remaining_placement='trash',
                is_optional=True,
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Same reveal effect as On Play.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT13-087 Reveal top 4, add 2 Lucemon/Royal Knight to hand")
        effect1.set_effect_description("[When Digivolving] Reveal the top 4 cards of your deck. Add 2 cards with [Lucemon] in their names or the [Royal Knight] trait among them to the hand. Trash the rest.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Reveal top 4, pick up to 2 Lucemon/Royal Knight to hand, trash rest."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            game.effect_reveal_and_select_multi(
                player, 4,
                passes=[
                    (lucemon_rk_filter, 'hand'),
                    (lucemon_rk_filter, 'hand'),
                ],
                remaining_placement='trash',
                is_optional=True,
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn] When you play another Digimon with [Lucemon] in its name or the
        # [Royal Knight] trait, delete all of your opponent's level 4 or lower Digimon.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT13-087 Delete all opponent Lv.4 or lower Digimon")
        effect2.set_effect_description("[Your Turn] When you play another Digimon with [Lucemon] in its name or the [Royal Knight] trait, delete all of your opponent's level 4 or lower Digimon.")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Must be your turn
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Check that the played Digimon is "another" (not this Dynasmon)
            # and has Lucemon in name or Royal Knight trait
            played_perm = context.get('played_permanent')
            played_card_src = context.get('played_card')
            check_target = played_perm or played_card_src
            if check_target is None:
                return False
            # "another" — not this card
            this_perm = card.permanent_of_this_card()
            if played_perm and this_perm and played_perm is this_perm:
                return False
            # Must be a Digimon with Lucemon name or Royal Knight trait
            tc = getattr(check_target, 'top_card', check_target)
            names = getattr(tc, 'card_names', [])
            traits = getattr(tc, 'card_traits', []) or []
            if not (any('Lucemon' in n for n in names) or any('Royal Knight' in t for t in traits)):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Delete all opponent's level 4 or lower Digimon."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return
            to_delete = [
                p for p in list(enemy.battle_area)
                if p.is_digimon
                and p.top_card
                and p.top_card.level is not None
                and p.top_card.level <= 4
            ]
            for target in to_delete:
                enemy.delete_permanent(target)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
