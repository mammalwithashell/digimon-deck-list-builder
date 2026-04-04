from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_095(CardScript):
    """BT24-095 Sonic Shot | Option (Green, Cost 3)

    While you have an [TS] trait Digimon or Tamer on the field, you can ignore
    this card's color requirements.
    [Security] Activate this card's [Main] effects.
    [Main] Suspend 1 of your opponent's Digimon or Tamers. It can't unsuspend
    in their next unsuspend phase. Then, you may link this card to 1 of your
    Digimon on the field without paying the cost.
    [Link] [TS] trait: Cost 3
    [When Attacking] [Once Per Turn] Return 1 of your opponent's suspended
    Digimon to the hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        # --- Color requirement bypass via _match_color_requirement_fn ---
        def _check_color_req():
            owner = card.owner if card else None
            if not owner:
                return True  # enforce if no owner
            for p in owner.battle_area:
                if (p.is_digimon or p.is_tamer) and p.has_trait('TS'):
                    return False  # bypass color requirement
            return True  # enforce color requirement
        card._match_color_requirement_fn = _check_color_req

        effects = []

        # --- Effect 1: Security — activate [Main] effects ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("BT24-095 Security: Activate [Main] effects")
        effect1.set_effect_description("[Security] Activate this card's [Main] effects.")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            pass  # Security activates main — engine handles re-dispatch

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Main] Suspend 1 opponent Digimon or Tamer, can't unsuspend,
        #    then optionally link ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OptionSkill)
        effect2.set_effect_name(
            "BT24-095 Suspend 1 opponent's Digimon or Tamer (can't unsuspend), then link"
        )
        effect2.set_effect_description(
            "[Main] Suspend 1 of your opponent's Digimon or Tamers. It can't unsuspend "
            "in their next unsuspend phase. Then, you may link this card to 1 of your "
            "Digimon on the field without paying the cost."
        )

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                return p.is_digimon or p.is_tamer

            def link_filter(perm):
                return perm.has_trait('TS')

            def on_suspend(target_perm):
                target_perm.suspend()
                # Grant cannot-unsuspend in next unsuspend phase via modifier
                from ....interfaces.modifiers import ModifierType
                game.register_modifier(
                    target_perm, ModifierType.CANNOT_UNSUSPEND,
                    value_fn=lambda: True, expiry='end_of_opponent_turn'
                )
                # Then optionally link this card to 1 of your [TS] trait Digimon
                game.effect_link_to_permanent(player, card, filter_fn=link_filter, is_optional=True)

            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False
            )

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: [Link] [When Attacking] [Once Per Turn]
        #    Return 1 opponent's suspended Digimon to the hand ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnUseAttack)
        effect3.set_effect_name("BT24-095 Bounce 1 opponent's suspended Digimon")
        effect3.set_effect_description(
            "[When Attacking] [Once Per Turn] Return 1 of your opponent's suspended "
            "Digimon to the hand."
        )
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("WA_BT24-095")
        effect3.is_linked_effect = True
        effect3.is_on_attack = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            def target_filter(p):
                return p.is_digimon and p.is_suspended

            def on_bounce(target_perm):
                enemy.bounce_permanent_to_hand(target_perm)

            game.effect_select_opponent_permanent(
                player, on_bounce, filter_fn=target_filter, is_optional=False
            )

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
