from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....interfaces.modifiers import ModifierType
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST17_08(CardScript):
    """ST17-08 MegaGargomon | Lv.6

    [Hand] [Counter] Blast Digivolve
    Blocker, Reboot
    [When Digivolving] Suspend 2 of your opponent's Digimon or Tamers.
    Then, 2 of their Digimon or Tamers can't unsuspend or digivolve
    until the end of their turn.
    [When Digivolving] [End of Attack] [Once Per Turn] You may unsuspend
    this Digimon.
    Inherited: Ace Overflow -4
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Blast Digivolve
        effect0 = ICardEffect()
        effect0.set_effect_name("ST17-08 Blast Digivolve")
        effect0.set_effect_description("Blast Digivolve")
        effect0.is_counter_effect = True
        effect0._is_blast_digivolve = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Blocker
        effect1 = ICardEffect()
        effect1.set_effect_name("ST17-08 Blocker")
        effect1.set_effect_description("Blocker")
        effect1._is_blocker = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Reboot
        effect2 = ICardEffect()
        effect2.set_effect_name("ST17-08 Reboot")
        effect2.set_effect_description("Reboot")
        effect2._is_reboot = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # [When Digivolving] Suspend 2 opponent Digimon/Tamers, then lock 2
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("ST17-08 Suspend 2 + lock 2")
        effect3.set_effect_description(
            "[When Digivolving] Suspend 2 of your opponent's Digimon or Tamers. "
            "Then, 2 of their Digimon or Tamers can't unsuspend or digivolve "
            "until the end of their turn."
        )
        effect3.is_when_digivolving = True

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

            # Suspend 2 opponent Digimon/Tamers
            suspended = []
            for perm in enemy.battle_area:
                if len(suspended) >= 2:
                    break
                if (perm.is_digimon or perm.is_tamer) and not perm.is_suspended:
                    perm.suspend()
                    suspended.append(perm)

            # Lock 2 opponent Digimon/Tamers: can't unsuspend or digivolve
            locked = 0
            for perm in enemy.battle_area:
                if locked >= 2:
                    break
                if perm.is_digimon or perm.is_tamer:
                    game.register_modifier(
                        perm, ModifierType.CANNOT_UNSUSPEND,
                        condition=lambda p, c, fp=perm: p is fp,
                        expiry='end_of_opponent_turn',
                    )
                    game.register_modifier(
                        perm, ModifierType.CANNOT_DIGIVOLVE,
                        condition=lambda p, c, fp=perm: p is fp,
                        expiry='end_of_opponent_turn',
                    )
                    locked += 1

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # [When Digivolving] [End of Attack] [Once Per Turn] May unsuspend this Digimon
        # WhenDigivolving part
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("ST17-08 May unsuspend this Digimon")
        effect4.set_effect_description(
            "[When Digivolving] [Once Per Turn] You may unsuspend this Digimon."
        )
        effect4.is_when_digivolving = True
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("UNSUSPEND_ST17_08")

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            perm = ctx.get('permanent')
            if perm and perm.is_suspended:
                perm.unsuspend()

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # End of Attack part
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnEndAttack)
        effect5.set_effect_name("ST17-08 End of Attack: May unsuspend this Digimon")
        effect5.set_effect_description(
            "[End of Attack] [Once Per Turn] You may unsuspend this Digimon."
        )
        effect5.is_optional = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("UNSUSPEND_ST17_08")

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect5.set_can_use_condition(condition5)
        effect5.set_on_process_callback(process4)  # Share same process
        effects.append(effect5)

        # Inherited: Ace Overflow -4
        effect6 = ICardEffect()
        effect6.set_effect_name("ST17-08 Ace Overflow -4")
        effect6.set_effect_description("Ace Overflow -4")
        effect6.is_inherited_effect = True
        effect6._is_ace_overflow = True
        effect6._overflow_amount = -4

        def condition6(context: Dict[str, Any]) -> bool:
            return True
        effect6.set_can_use_condition(condition6)
        effects.append(effect6)

        return effects
