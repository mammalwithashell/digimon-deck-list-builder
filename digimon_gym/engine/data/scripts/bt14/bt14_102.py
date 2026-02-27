from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_102(CardScript):
    """BT14-102 Angemon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [When Attacking] By deleting this Digimon, activate 1 of the effects below:
        # - Place 1 of your opponent's Digimon with the [Virus] trait at the bottom of their security stack.
        # - 1 of your opponent's Digimon gets -5000 DP for the turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-102 Delete this Digimon to select one effect")
        effect1.set_effect_description("[When Attacking] By deleting this Digimon, activate 1 of the effects below: - Place 1 of your opponent's Digimon with the [Virus] trait at the bottom of their security stack. - 1 of your opponent's Digimon gets -5000 DP for the turn.")
        effect1.is_optional = True
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            return card is not None and card.permanent_of_this_card() is not None

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            # Cost: delete this Digimon.
            if hasattr(perm, 'delete'):
                perm.delete()
            elif hasattr(player, 'delete_permanent'):
                player.delete_permanent(perm)

            enemy = player.enemy if player else None
            enemy_digimon = [p for p in (enemy.battle_area if enemy else []) if getattr(p, 'is_digimon', False)]

            def has_virus_trait(p):
                attrs = []
                src = getattr(p, 'card', None) or getattr(p, 'card_source', None)
                if src is not None:
                    attrs = getattr(src, 'attribute_eng', None) or []
                return 'Virus' in attrs

            virus_targets = [p for p in enemy_digimon if has_virus_trait(p)]

            # Choose exactly one effect. Deterministic priority:
            # if a valid Virus target exists, use bottom-security mode; otherwise use -5000 DP mode.
            if virus_targets:
                target = min(virus_targets, key=lambda p: getattr(p, 'dp', 0) or 0)
                player.put_permanent_to_security(target)
                return

            if enemy_digimon:
                target = min(enemy_digimon, key=lambda p: getattr(p, 'dp', 0) or 0)
                if hasattr(target, 'change_dp'):
                    target.change_dp(-5000)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [On Deletion] Place this card at the bottom of your security stack.
        # Then, if you have a Tamer in play, you may hatch a Digi-Egg in your breeding area.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-102 Place this card at bottom of security and optional hatch")
        effect2.set_effect_description("[On Deletion] Place this card at the bottom of your security stack. Then, if you have a Tamer, you may hatch in your breeding area.")
        effect2.is_on_deletion = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if not player:
                return

            if perm is not None:
                player.put_permanent_to_security(perm)

            has_tamer = any(getattr(p, 'is_tamer', False) for p in getattr(player, 'battle_area', []))
            if has_tamer and hasattr(player, 'hatch'):
                player.hatch()

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Inherited [On Deletion] Place 1 yellow Digimon with the [Vaccine] trait
        # from your hand at the bottom of your security stack.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT14-102 Inherited: place yellow Vaccine Digimon from hand to security")
        effect3.set_effect_description("[On Deletion] Place 1 yellow card with the [Vaccine] trait from your hand at the bottom of your security stack.")
        effect3.is_inherited_effect = True
        effect3.is_optional = True
        effect3.is_on_deletion = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if not player:
                return

            hand = list(getattr(player, 'hand', []))

            def is_yellow_vaccine_digimon(c):
                if c is None:
                    return False
                is_digimon = getattr(c, 'card_kind', None) == 0 or getattr(c, 'is_digimon', False)
                colors = getattr(c, 'card_colors', None) or []
                attrs = getattr(c, 'attribute_eng', None) or []
                return is_digimon and (2 in colors) and ('Vaccine' in attrs)

            candidates = [c for c in hand if is_yellow_vaccine_digimon(c)]
            if not candidates:
                return

            chosen = candidates[0]
            if hasattr(player, 'hand_remove_card'):
                player.hand_remove_card(chosen)
            elif hasattr(player, 'remove_card_from_hand'):
                player.remove_card_from_hand(chosen)
            elif chosen in getattr(player, 'hand', []):
                player.hand.remove(chosen)

            if hasattr(player, 'security_add_card_bottom'):
                player.security_add_card_bottom(chosen)
            elif hasattr(player, 'add_card_to_security_bottom'):
                player.add_card_to_security_bottom(chosen)
            elif hasattr(player, 'security_stack'):
                player.security_stack.append(chosen)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
