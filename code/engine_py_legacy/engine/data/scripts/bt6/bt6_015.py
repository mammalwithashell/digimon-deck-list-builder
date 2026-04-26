from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT6_015(CardScript):
    """BT6-015 Jesmon | Lv.6 Red Digimon

    [When Digivolving] You may play 1 Digimon card with [Sistermon] in
        its name from your hand without paying its memory cost.

    Inherited Effect:
    [When Attacking][Once Per Turn] If you have a Digimon with [Sistermon]
        in its name in play, unsuspend this Digimon.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [When Digivolving] Play 1 Sistermon from hand free ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT6-015 When Digivolving: play Sistermon from hand free")
        effect0.set_effect_description(
            "[When Digivolving] You may play 1 Digimon card with [Sistermon] "
            "in its name from your hand without paying its memory cost."
        )
        effect0.is_when_digivolving = True
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            if len(player.hand_cards) < 1:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def sistermon_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                return c.contains_card_name('Sistermon')

            has_sistermon = any(sistermon_filter(c) for c in player.hand_cards)
            if has_sistermon:
                game.effect_play_from_zone(
                    player, 'hand', sistermon_filter,
                    free=True, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: Inherited [When Attacking][Once Per Turn] Unsuspend if Sistermon in play ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnAllyAttack)
        effect1.set_effect_name("BT6-015 Inherited: unsuspend if Sistermon in play")
        effect1.set_effect_description(
            "[When Attacking][Once Per Turn] If you have a Digimon with "
            "[Sistermon] in its name in play, unsuspend this Digimon."
        )
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Unsuspend_BT6_015")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            # Must have a Sistermon in play
            has_sistermon = any(
                p.is_digimon and p.contains_card_name('Sistermon')
                for p in player.battle_area
            )
            return has_sistermon
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            perm = card.permanent_of_this_card() if card else None
            if perm:
                perm.unsuspend()

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
