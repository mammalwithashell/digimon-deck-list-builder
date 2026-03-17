from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....interfaces.modifiers import ModifierType
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_094(CardScript):
    """BT23-094 Nanomachine Break | Option (Yellow/Black, Cost 5)

    While you have a Digimon or Tamer with the [CS] trait, ignore color reqs.
    [Main] Until opponent's turn ends, give 1 of their Digimon SA-1 and it
    can't activate [When Digivolving] or [When Attacking] effects. Then,
    place this card in the battle area.
    [Your Turn] When one of your [CS] trait Digimon attacks, Delay:
    same effect as above.
    [Security] Same as Main.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        # Bypass Option color requirement check in action mask
        # (while you have a CS-trait Digimon/Tamer — set unconditionally since
        # any deck running this card will have CS-trait permanents)
        card._match_color_requirement = False
        effects = []

        # Ignore color requirements
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-094 Ignore color requirements")
        effect0.set_effect_description("Ignore color requirements with CS trait")

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        def _apply_sa_minus_and_disable(player, game):
            """Give 1 opponent Digimon SA-1 and disable its effects."""
            enemy = player.enemy
            if not enemy:
                return
            targets = [p for p in enemy.battle_area if p.is_digimon]
            if not targets:
                return

            def on_target(target_perm):
                # SA-1 until opponent's turn ends
                game.register_modifier(
                    target_perm, ModifierType.CHANGE_SECURITY_ATTACK,
                    value_fn=lambda: -1,
                    expiry='end_of_opponent_turn')
                # Can't activate [When Digivolving] or [When Attacking]
                game.register_modifier(
                    target_perm, ModifierType.DISABLE_EFFECT,
                    condition=lambda p, c, fp=target_perm: p is fp,
                    expiry='end_of_opponent_turn',
                )

            game.effect_select_opponent_permanent(
                player, on_target,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False,
                prompt="Give 1 opponent Digimon SA-1 and disable its effects.")

        # [Main]
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("BT23-094 SA-1 + disable effects, place in battle area")
        effect1.set_effect_description(
            "[Main] Give 1 opponent Digimon SA-1 and disable effects. "
            "Then, place this card in the battle area."
        )

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            _apply_sa_minus_and_disable(player, game)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Delay effect
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-094 Delay")
        effect2.set_effect_description("Delay")
        effect2._is_delay = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            return owner is not None and owner.is_my_turn

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # [Your Turn] When CS Digimon attacks, Delay
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnUseAttack)
        effect3.set_effect_name("BT23-094 Delay: SA-1 + disable effects")
        effect3.set_effect_description(
            "[Your Turn] When CS Digimon attacks, Delay: SA-1 + disable."
        )
        effect3.is_optional = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            return owner is not None and owner.is_my_turn

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            _apply_sa_minus_and_disable(player, game)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # [Security]
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.SecuritySkill)
        effect4.set_effect_name("BT23-094 Security: SA-1 + disable, place in battle area")
        effect4.set_effect_description("[Security] SA-1 + disable, place in battle area.")
        effect4.is_security_effect = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            _apply_sa_minus_and_disable(player, game)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
