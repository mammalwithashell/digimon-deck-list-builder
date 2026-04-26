from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....interfaces.modifiers import ModifierType
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_063(CardScript):
    """BT16-063 Shakkoumon | Lv.5

    Partition (Black Lv.4 & Yellow Lv.4)
    [When Digivolving] Until the end of your opponent's turn, this Digimon
    isn't affected by the effects of their Digimon. Then, if DNA digivolving,
    place 1 of your opponent's Digimon with as high or lower a level as
    the number of cards in your or their security stack at the bottom of
    their security stack.
    Inherited: Partition (Black Lv.4 & Yellow Lv.4)
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Jogress/DNA condition (handled by engine)
        effect0 = ICardEffect()
        effect0.set_effect_name("BT16-063 Jogress Condition")
        effect0.set_effect_description("Jogress Condition")

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # [When Digivolving] Immunity + conditional DNA placement
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT16-063 Immunity + DNA placement")
        effect1.set_effect_description(
            "[When Digivolving] Until the end of your opponent's turn, this "
            "Digimon isn't affected by the effects of their Digimon. Then, if "
            "DNA digivolving, place 1 of your opponent's Digimon with level <= "
            "number of cards in your or their security stack at the bottom of "
            "their security stack."
        )
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            # Grant immunity to opponent Digimon effects
            game.register_modifier(
                perm, ModifierType.CANNOT_BE_AFFECTED,
                condition=lambda p, c, fp=perm: p is fp,
                expiry='end_of_opponent_turn',
            )

            # Check if this was a DNA digivolve
            is_dna = ctx.get('is_dna_digivolve', False)
            if not is_dna:
                return

            enemy = player.enemy
            if not enemy:
                return

            # Level threshold: max(own security count, opponent security count)
            max_level = max(len(player.security_cards), len(enemy.security_cards))

            def target_filter(p):
                if not p.is_digimon:
                    return False
                lvl = getattr(p, 'level', None)
                return lvl is not None and lvl <= max_level

            def on_place_security(target_perm):
                enemy2 = player.enemy if player else None
                if enemy2 and target_perm in enemy2.battle_area:
                    enemy2.battle_area.remove(target_perm)
                    # Place at bottom of opponent's security stack
                    for cs in target_perm.card_sources:
                        enemy2.security_cards.append(cs)

            targets = [p for p in enemy.battle_area if target_filter(p)]
            if targets:
                game.effect_select_opponent_permanent(
                    player, on_place_security, filter_fn=target_filter,
                    is_optional=False,
                    prompt="Place 1 opponent Digimon at bottom of their security stack.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Partition (Black Lv.4 & Yellow Lv.4)
        effect2 = ICardEffect()
        effect2.set_effect_name("BT16-063 Partition")
        effect2.set_effect_description("Partition (Black Lv.4 & Yellow Lv.4)")
        effect2._is_partition = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Inherited: Partition
        effect3 = ICardEffect()
        effect3.set_effect_name("BT16-063 Partition (inherited)")
        effect3.set_effect_description("Partition (Black Lv.4 & Yellow Lv.4)")
        effect3.is_inherited_effect = True
        effect3._is_partition = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
