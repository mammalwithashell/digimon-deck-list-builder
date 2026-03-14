from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT17_047(CardScript):
    """BT17-047 Parrotmon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Security] At the end of the battle, if you don't have Digimon,
        # play this card without paying the cost.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEndBattle)
        effect0.set_effect_name("BT17-047 Security: Play free if no Digimon")
        effect0.set_effect_description(
            "[Security] At the end of the battle, if you don't have Digimon, "
            "play this card without paying the cost."
        )
        effect0.is_security_effect = True

        def condition0(context: Dict[str, Any]) -> bool:
            owner = card.owner if card else None
            if not owner:
                return False
            # If you don't have Digimon in play
            has_digimon = any(p.is_digimon for p in owner.battle_area)
            return not has_digimon

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and card:
                if not any(p.is_digimon for p in player.battle_area):
                    player.play_card_from_source(card, pay_cost=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [On Play] [When Digivolving] Suspend 1 of your opponent's Digimon.
        def _make_suspend_effect(is_when_digivolving: bool) -> ICardEffect:
            effect = ICardEffect()
            effect.set_timing(EffectTiming.OnEnterFieldAnyone)
            if is_when_digivolving:
                effect.is_when_digivolving = True
                effect.set_effect_name("BT17-047 When Digivolving: Suspend 1 opponent Digimon")
            else:
                effect.is_on_play = True
                effect.set_effect_name("BT17-047 On Play: Suspend 1 opponent Digimon")
            effect.set_effect_description(
                "[On Play] [When Digivolving] Suspend 1 of your opponent's Digimon."
            )

            def cond(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                return True

            effect.set_can_use_condition(cond)

            def proc(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return

                def suspend_filter(p):
                    return p.is_digimon and not p.is_suspended

                def on_suspend(target_perm):
                    target_perm.suspend()

                game.effect_select_opponent_permanent(
                    player, on_suspend,
                    filter_fn=suspend_filter,
                    is_optional=False,
                    prompt="Suspend 1 of your opponent's Digimon."
                )

            effect.set_on_process_callback(proc)
            return effect

        effects.append(_make_suspend_effect(is_when_digivolving=False))
        effects.append(_make_suspend_effect(is_when_digivolving=True))

        # Inherited: [All Turns] [Once Per Turn] When this Digimon deletes an
        # opponent's Digimon in battle, you may unsuspend this Digimon.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEndBattle)
        effect3.is_inherited_effect = True
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("BT17_047_InhUnsuspend")
        effect3.set_effect_name("BT17-047 Inherited: Unsuspend when deleting in battle")
        effect3.set_effect_description(
            "[All Turns] [Once Per Turn] When this Digimon deletes an "
            "opponent's Digimon in battle, you may unsuspend this Digimon."
        )

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if perm is None:
                return False
            # Check if this Digimon won the battle (deleted opponent's Digimon)
            battle_result = context.get('battle_result') or context.get('attack_resolution')
            if battle_result is None:
                return False
            # AttackResolution.Survivor = 0 means this Digimon survived
            from ....data.enums import AttackResolution
            return battle_result == AttackResolution.Survivor

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            perm = card.permanent_of_this_card() if card else None
            if perm:
                perm.unsuspend()

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
