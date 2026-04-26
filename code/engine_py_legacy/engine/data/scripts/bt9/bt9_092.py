from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT9_092(CardScript):
    """BT9-092 Cool Boy | Tamer | White | Cost 2

    [On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card
        with [X Antibody] in its traits and 1 Option card with
        [X Antibody] in its traits among them to your hand. Place the
        rest at the bottom of your deck in any order.

    [Your Turn] When one of your Digimon digivolves into a same-level
        Digimon with [X Antibody] in its traits, you may suspend this
        Tamer to gain 1 memory and <Draw 1>.

    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [On Play] Reveal top 3 for X Antibody Digimon + Option ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT9-092 On Play: reveal top 3 for X Antibody cards")
        effect0.set_effect_description(
            "[On Play] Reveal the top 3 cards of your deck. Add 1 Digimon "
            "card with [X Antibody] in its traits and 1 Option card with "
            "[X Antibody] in its traits among them to your hand. Place the "
            "rest at the bottom of your deck in any order."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            return len(player.library_cards) >= 1
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def _has_x_antibody_trait(c):
                traits = getattr(c, 'card_traits', []) or []
                return any('X Antibody' in t or 'X-Antibody' in t for t in traits)

            def x_antibody_digimon_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                return _has_x_antibody_trait(c)

            def x_antibody_option_filter(c):
                if not getattr(c, 'is_option', False):
                    return False
                return _has_x_antibody_trait(c)

            game.effect_reveal_and_select_multi(
                player, 3,
                [(x_antibody_digimon_filter, 'hand'),
                 (x_antibody_option_filter, 'hand')],
                remaining_placement='deck_bottom',
                is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Your Turn] When Digimon digivolves into same-level X Antibody,
        #     suspend for memory +1 and Draw 1 ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT9-092 Suspend for memory+1 and draw on X Antibody digivolve")
        effect1.set_effect_description(
            "[Your Turn] When one of your Digimon digivolves into a "
            "same-level Digimon with [X Antibody] in its traits, you may "
            "suspend this Tamer to gain 1 memory and <Draw 1>."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            # Must not already be suspended (suspend is the cost)
            if perm.is_suspended:
                return False
            player = card.owner if card else None
            if not player:
                return False
            # Must be our turn
            if not player.is_my_turn:
                return False
            # Trigger: one of our Digimon digivolved (when_digivolving context)
            trigger_perm = context.get('permanent')
            if not trigger_perm:
                return False
            if trigger_perm.owner != player:
                return False
            if not trigger_perm.is_digimon:
                return False
            # The digivolved Digimon must have [X Antibody] trait
            top = trigger_perm.top_card
            top_traits = getattr(top, 'card_traits', []) or []
            has_x = any('X Antibody' in t or 'X-Antibody' in t for t in top_traits)
            if not top or not has_x:
                return False
            # Must be same-level digivolution (level didn't change)
            # Check if the previous top card had the same level
            if len(trigger_perm.card_sources) >= 2:
                prev_card = trigger_perm.card_sources[-2]
                prev_level = getattr(prev_card, 'level', None)
                cur_level = getattr(top, 'level', None)
                if prev_level is None or cur_level is None:
                    return False
                if prev_level != cur_level:
                    return False
            else:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if not player:
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return

            # Cost: suspend this Tamer
            perm.suspend()

            # Gain 1 memory
            player.add_memory(1)

            # Draw 1
            player.draw()

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Security] Play this card without paying the cost ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("BT9-092 Security: Play this card free")
        effect2.set_effect_description(
            "[Security] Play this card without paying the cost."
        )
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
