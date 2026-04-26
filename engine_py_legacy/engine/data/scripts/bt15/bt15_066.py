from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, CardColor

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_066(CardScript):
    """BT15-066 Machinedramon | Lv.6

    [On Play] [When Attacking] <De-Digivolve 2> 1 of your opponent's Digimon.
    [Your Turn] This Digimon can only digivolve into white Digimon.
    [End of Opponent's Turn] Delete this Digimon. Then, you may play 1 Digimon
        card with the [Dark Masters] trait, other than [Machinedramon], from
        your hand without paying the cost.
    Inherited: <Reboot>
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Shared De-Digivolve 2 logic for On Play / When Attacking ---
        def _de_digivolve_process(ctx: Dict[str, Any]):
            """<De-Digivolve 2> 1 of your opponent's Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(2)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)

            game.effect_select_opponent_permanent(
                player, on_de_digivolve,
                filter_fn=lambda p: p.is_digimon, is_optional=False)

        # [On Play] <De-Digivolve 2> 1 of your opponent's Digimon.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT15-066 De-Digivolve 2")
        effect0.set_effect_description(
            "[On Play] <De-Digivolve 2> 1 of your opponent's Digimon.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effect0.set_on_process_callback(_de_digivolve_process)
        effects.append(effect0)

        # [When Attacking] <De-Digivolve 2> 1 of your opponent's Digimon.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnUseAttack)
        effect1.set_effect_name("BT15-066 De-Digivolve 2")
        effect1.set_effect_description(
            "[When Attacking] <De-Digivolve 2> 1 of your opponent's Digimon.")
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_de_digivolve_process)
        effects.append(effect1)

        # --- [Your Turn] This Digimon can only digivolve into white Digimon ---
        # Register CANNOT_DIGIVOLVE modifier on self when entering field.
        # The modifier condition allows white cards and blocks non-white.
        effect_restrict = ICardEffect()
        effect_restrict.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_restrict.set_effect_name("BT15-066 Can only digivolve into white")
        effect_restrict.set_effect_description(
            "[Your Turn] This Digimon can only digivolve into white Digimon.")
        effect_restrict.is_on_play = True

        def condition_restrict(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect_restrict.set_can_use_condition(condition_restrict)

        def process_restrict(ctx: Dict[str, Any]):
            """Register CANNOT_DIGIVOLVE modifier that only blocks non-white digivolutions."""
            game = ctx.get('game')
            perm = card.permanent_of_this_card() if card else None
            if not (game and perm):
                return
            from ....interfaces.modifiers import ModifierType

            def restrict_condition(target, context):
                """Active (= block digivolve) when digivolving_card is NOT white."""
                digi_card = context.get('digivolving_card') if context else None
                if digi_card is None:
                    return False  # No card info — don't block
                card_colors = getattr(digi_card, 'card_colors', [])
                return CardColor.White not in card_colors

            game.register_modifier(
                perm,
                ModifierType.CANNOT_DIGIVOLVE,
                condition=restrict_condition,
                source_effect=effect_restrict,
                expiry='permanent',
            )
        effect_restrict.set_on_process_callback(process_restrict)
        effects.append(effect_restrict)

        # [End of Opponent's Turn] Delete this Digimon. Then, you may play 1
        # Digimon card with the [Dark Masters] trait, other than [Machinedramon],
        # from your hand without paying the cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEndTurn)
        effect2.set_effect_name("BT15-066 Delete this Digimon and play 1 Digimon from hand")
        effect2.set_effect_description(
            "[End of Opponent's Turn] Delete this Digimon. Then, you may play "
            "1 Digimon card with the [Dark Masters] trait, other than "
            "[Machinedramon], from your hand without paying the cost.")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # End of OPPONENT's turn only
            if card and card.owner and card.owner.is_my_turn:
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Delete THIS Digimon, then play 1 Dark Masters (not Machinedramon) from hand free."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            # Step 1: Delete THIS Digimon
            if perm and perm in player.battle_area:
                player.delete_permanent(perm)
            # Step 2: Play 1 Digimon with Dark Masters trait (not Machinedramon) from hand free
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                if not any('Dark Masters' in t for t in traits):
                    return False
                names = getattr(c, 'card_names', []) or []
                if any('Machinedramon' in n for n in names):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Inherited: <Reboot>
        effect3 = ICardEffect()
        effect3.set_effect_name("BT15-066 Reboot")
        effect3.set_effect_description("Reboot")
        effect3.is_inherited_effect = True
        effect3._is_reboot = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
